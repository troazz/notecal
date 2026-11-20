//! Updates (PRD A9): compares this build with the latest GitHub release and
//! tells the UI when a newer one exists. How it is installed depends on how
//! notecal was installed:
//! - Windows installer, `.dmg`, AppImage: the updater plugin downloads the
//!   signed update in the background; one click installs it and restarts.
//! - Homebrew: one click runs `brew upgrade --cask notecal`, so brew keeps
//!   track of what is installed.
//! - pacman (built with makepkg) and the `.deb`: the UI shows the command;
//!   installing needs root.

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::commands::SettingsState;

const LATEST_URL: &str = "https://api.github.com/repos/troazz/notecal/releases/latest";
const RELEASES: &str = "https://github.com/troazz/notecal/releases";
const TIMEOUT: Duration = Duration::from_secs(10);
/// After startup, so the check never competes with the first paint.
const FIRST_DELAY: Duration = Duration::from_secs(10);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub const UPDATE_AVAILABLE: &str = "update-available";
/// An install or brew upgrade failed; the payload is the message.
pub const UPDATE_FAILED: &str = "update-failed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateInfo {
    /// The newer version, without the `v`.
    pub version: String,
    /// How this copy was installed, so the UI shows the matching upgrade step.
    pub install: Install,
    /// `Updater` only: the update is downloaded and verified, ready to install.
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Install {
    /// The Homebrew cask: `brew upgrade --cask notecal`.
    Homebrew,
    /// The pacman package built from the repository with `makepkg`.
    Pacman,
    /// The `.deb` (Ubuntu, Mint, Debian): download it, then `apt install` it.
    Deb,
    /// Windows installer, `.dmg` app or AppImage: the updater plugin installs it.
    Updater,
    /// Anything else (a development build, a copied binary): download it.
    Download,
}

/// Where the install method shows. Kept separate from the filesystem so the
/// decision can be tested.
#[derive(Debug, Default)]
struct Evidence {
    macos: bool,
    /// The cask's folder exists (`/opt/homebrew` or `/usr/local` Caskroom).
    homebrew_cask: bool,
    /// The running binary is `/usr/bin/notecal` and pacman's database lists notecal.
    pacman_package: bool,
    /// The running binary is `/usr/bin/notecal` and dpkg lists the notecal package.
    deb_package: bool,
    /// The updater plugin can replace this copy: Windows and macOS builds, or
    /// a Linux AppImage (`$APPIMAGE` is set).
    self_updatable: bool,
}

fn install_method(evidence: &Evidence) -> Install {
    if evidence.macos && evidence.homebrew_cask {
        Install::Homebrew
    } else if !evidence.macos && evidence.pacman_package {
        Install::Pacman
    } else if !evidence.macos && evidence.deb_package {
        Install::Deb
    } else if evidence.self_updatable {
        Install::Updater
    } else {
        Install::Download
    }
}

fn gather_evidence() -> Evidence {
    let homebrew_cask = [
        "/opt/homebrew/Caskroom/notecal",
        "/usr/local/Caskroom/notecal",
    ]
    .iter()
    .any(|dir| Path::new(dir).is_dir());
    let exe = std::env::current_exe().ok();
    let system_binary = exe.as_deref() == Some(Path::new("/usr/bin/notecal"));
    let deb_package = system_binary && Path::new("/var/lib/dpkg/info/notecal.list").is_file();
    let pacman_package = system_binary
        && std::fs::read_dir("/var/lib/pacman/local").is_ok_and(|entries| {
            entries.flatten().any(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                // `notecal-1.0.0-2`, not `notecal-cli-…` from some other package.
                name.strip_prefix("notecal-")
                    .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
            })
        });
    let self_updatable = if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE").is_some()
    } else {
        // Debug builds run from target/, never from an installed app.
        !cfg!(debug_assertions)
    };
    Evidence {
        macos: cfg!(target_os = "macos"),
        homebrew_cask,
        pacman_package,
        deb_package,
        self_updatable,
    }
}

/// The newest release found, if it is newer than this build, and for the
/// updater the downloaded update waiting for the user's click.
#[derive(Default)]
pub struct UpdateState(Mutex<Found>);

#[derive(Default)]
struct Found {
    info: Option<UpdateInfo>,
    download: Option<(Update, Vec<u8>)>,
}

impl UpdateState {
    fn lock(&self) -> std::sync::MutexGuard<'_, Found> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Fetches the signed update through the plugin (its `latest.json`), and
/// downloads it. The plugin checks the signature against the public key in
/// tauri.conf.json before `install` accepts the bytes.
fn download_update(app: &AppHandle) -> Result<Option<(Update, Vec<u8>)>, String> {
    tauri::async_runtime::block_on(async {
        let updater = app.updater().map_err(|e| e.to_string())?;
        let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let bytes = update
            .download(|_, _| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        Ok(Some((update, bytes)))
    })
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// `1.2.3` or `v1.2.3`; anything else (pre-releases included) is not a version
/// to offer.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.strip_prefix('v').unwrap_or(text).split('.');
    let mut next = || parts.next()?.parse::<u64>().ok();
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

/// The release's version, when it is a stable release newer than `current`.
fn newer_release(current: &str, release: &Release) -> Option<String> {
    if release.draft || release.prerelease {
        return None;
    }
    let latest = parse_version(&release.tag_name)?;
    (latest > parse_version(current)?).then(|| format!("{}.{}.{}", latest.0, latest.1, latest.2))
}

fn fetch_latest() -> Result<Release, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    agent
        .get(LATEST_URL)
        // GitHub rejects requests without a User-Agent.
        .header("User-Agent", concat!("notecal/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Could not check for updates: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| format!("Unexpected update response: {e}"))
}

fn check(app: &AppHandle) {
    let enabled = app.state::<SettingsState>().view().settings.check_updates;
    if !enabled {
        return;
    }
    match fetch_latest() {
        Ok(release) => {
            let Some(version) = newer_release(env!("CARGO_PKG_VERSION"), &release) else {
                return;
            };
            let mut info = UpdateInfo {
                version,
                install: install_method(&gather_evidence()),
                ready: false,
            };
            let state = app.state::<UpdateState>();
            if info.install == Install::Updater {
                if state.lock().download.is_some() {
                    return; // already downloaded and announced
                }
                match download_update(app) {
                    Ok(Some(download)) => {
                        info.ready = true;
                        state.lock().download = Some(download);
                    }
                    // A release without updater files (1.0.0): download by hand.
                    Ok(None) => info.install = Install::Download,
                    Err(e) => {
                        eprintln!("notecal: could not download the update: {e}");
                        info.install = Install::Download;
                    }
                }
            }
            state.lock().info = Some(info.clone());
            if let Err(e) = app.emit(UPDATE_AVAILABLE, info) {
                eprintln!("notecal: {e}");
            }
        }
        // Offline is normal; the next check is tomorrow.
        Err(e) => eprintln!("notecal: {e}"),
    }
}

/// Checks after startup, then once a day, on its own thread.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    let spawned = thread::Builder::new()
        .name("update-check".into())
        .spawn(move || {
            thread::sleep(FIRST_DELAY);
            loop {
                check(&app);
                thread::sleep(INTERVAL);
            }
        });
    if let Err(e) = spawned {
        eprintln!("notecal: could not start the update check: {e}");
    }
}

/// For a UI that starts after the event was sent.
#[tauri::command]
pub fn update_status(state: State<'_, UpdateState>) -> Option<UpdateInfo> {
    state.lock().info.clone()
}

/// Installs the downloaded update and restarts into it (Windows closes the
/// app for the installer itself).
#[tauri::command]
pub fn install_update(app: AppHandle, state: State<'_, UpdateState>) -> Result<(), String> {
    let (update, bytes) = state
        .lock()
        .download
        .take()
        .ok_or("No update has been downloaded.")?;
    update
        .install(bytes)
        .map_err(|e| format!("Could not install the update: {e}"))?;
    app.restart()
}

/// `brew upgrade --cask notecal` on its own thread; restarts into the new
/// version when brew succeeds, or reports brew's message.
#[tauri::command]
pub fn upgrade_with_homebrew(app: AppHandle) -> Result<(), String> {
    // GUI apps don't get the shell's PATH, so look where Homebrew installs itself.
    let brew = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .into_iter()
        .find(|path| Path::new(path).is_file())
        .ok_or("Homebrew was not found.")?;
    thread::Builder::new()
        .name("brew-upgrade".into())
        .spawn(move || {
            let result = Command::new(brew)
                .args(["upgrade", "--cask", "notecal"])
                .output();
            match result {
                Ok(output) if output.status.success() => app.restart(),
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let last = stderr
                        .lines()
                        .rev()
                        .find(|l| !l.trim().is_empty())
                        .unwrap_or("");
                    report_failure(&app, format!("brew upgrade failed: {last}"));
                }
                Err(e) => report_failure(&app, format!("Could not run brew: {e}")),
            }
        })
        .map(drop)
        .map_err(|e| format!("Could not start brew: {e}"))
}

fn report_failure(app: &AppHandle, message: String) {
    eprintln!("notecal: {message}");
    if let Err(e) = app.emit(UPDATE_FAILED, message) {
        eprintln!("notecal: {e}");
    }
}

/// Opens the newer release's page in the browser. The URL is built here from
/// the parsed version, so nothing from the network becomes a command argument.
#[tauri::command]
pub fn open_release_page(state: State<'_, UpdateState>) -> Result<(), String> {
    let version = state.lock().info.as_ref().map(|info| info.version.clone());
    let url = match version.as_deref().and_then(parse_version) {
        Some((major, minor, patch)) => format!("{RELEASES}/tag/v{major}.{minor}.{patch}"),
        None => format!("{RELEASES}/latest"),
    };
    open_in_browser(&url)
}

fn open_in_browser(url: &str) -> Result<(), String> {
    let mut command = if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(windows) {
        let mut c = Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    } else {
        Command::new("xdg-open")
    };
    command
        .arg(url)
        .spawn()
        .map(drop)
        .map_err(|e| format!("Could not open {url}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
        }
    }

    #[test]
    fn offers_only_a_newer_stable_release() {
        assert_eq!(
            newer_release("1.0.0", &release("v1.0.1")),
            Some("1.0.1".into())
        );
        assert_eq!(
            newer_release("1.0.0", &release("v1.10.0")),
            Some("1.10.0".into())
        );
        assert_eq!(
            newer_release("1.2.0", &release("v1.10.0")),
            Some("1.10.0".into())
        );
        assert_eq!(newer_release("1.0.0", &release("v1.0.0")), None);
        assert_eq!(newer_release("1.1.0", &release("v1.0.9")), None);
    }

    #[test]
    fn ignores_drafts_pre_releases_and_odd_tags() {
        let mut draft = release("v2.0.0");
        draft.draft = true;
        assert_eq!(newer_release("1.0.0", &draft), None);
        let mut pre = release("v2.0.0");
        pre.prerelease = true;
        assert_eq!(newer_release("1.0.0", &pre), None);
        for tag in ["v2.0.0-rc.1", "v2.0", "v2.0.0.1", "latest", ""] {
            assert_eq!(newer_release("1.0.0", &release(tag)), None, "{tag}");
        }
    }

    #[test]
    fn upgrade_step_follows_the_install_method() {
        let brew = Evidence {
            macos: true,
            homebrew_cask: true,
            ..Evidence::default()
        };
        assert_eq!(install_method(&brew), Install::Homebrew);
        let pacman = Evidence {
            pacman_package: true,
            ..Evidence::default()
        };
        assert_eq!(install_method(&pacman), Install::Pacman);
        let deb = Evidence {
            deb_package: true,
            ..Evidence::default()
        };
        assert_eq!(install_method(&deb), Install::Deb);
        let installer = Evidence {
            self_updatable: true,
            ..Evidence::default()
        };
        assert_eq!(install_method(&installer), Install::Updater);
        // Homebrew and pacman own their files even where the updater could run.
        let brew_app = Evidence {
            macos: true,
            homebrew_cask: true,
            self_updatable: true,
            ..Evidence::default()
        };
        assert_eq!(install_method(&brew_app), Install::Homebrew);
        // Not replaceable by the updater (a development build): download by hand.
        assert_eq!(
            install_method(&Evidence {
                macos: true,
                ..Evidence::default()
            }),
            Install::Download
        );
        assert_eq!(install_method(&Evidence::default()), Install::Download);
    }

    #[test]
    fn reads_githubs_release_json() {
        let json = r#"{"tag_name":"v1.0.1","name":"notecal v1.0.1","draft":false,"prerelease":false,"assets":[]}"#;
        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(newer_release("1.0.0", &release), Some("1.0.1".into()));
    }
}

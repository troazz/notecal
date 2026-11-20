//! Always on top (PRD A4). Tauri's call works on X11, macOS and Windows; on
//! Wayland it does nothing, because the compositor decides. Hyprland can pin a
//! floating window (above others, on every workspace) through `hyprctl`, so
//! there notecal pins its own window. Other Wayland compositors have no way.

use std::process::Command;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::WebviewWindow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PinSupport {
    /// X11, macOS, Windows: `set_always_on_top`.
    Native,
    /// Hyprland: `hyprctl` pin.
    Hyprland,
    /// Another Wayland compositor: not possible from the app.
    None,
}

pub fn support() -> PinSupport {
    if !crate::shell::is_wayland() {
        PinSupport::Native
    } else if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        PinSupport::Hyprland
    } else {
        PinSupport::None
    }
}

/// For the status bar's pin button.
#[tauri::command]
pub fn pin_support() -> PinSupport {
    support()
}

/// Hyprland's config language, for the bind line the onboarding offers (A8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HyprConfig {
    /// `hyprland.lua` (Hyprland 0.55+, Omarchy).
    Lua,
    /// `hyprland.conf`.
    Classic,
}

/// What the onboarding's Keys step needs to know about the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DesktopInfo {
    /// Apps can't register a global shortcut: the compositor binds `notecal --toggle`.
    pub wayland: bool,
    pub hyprland: Option<HyprConfig>,
    pub pin: PinSupport,
}

#[tauri::command]
pub fn desktop_info() -> DesktopInfo {
    let pin = support();
    let hyprland = (pin == PinSupport::Hyprland).then(|| {
        let lua = notecal_env::paths::system_dirs()
            .is_some_and(|dirs| dirs.config.join("hypr/hyprland.lua").is_file());
        if lua {
            HyprConfig::Lua
        } else {
            HyprConfig::Classic
        }
    });
    DesktopInfo {
        wayland: crate::shell::is_wayland(),
        hyprland,
        pin,
    }
}

/// Applies `on` to the window. Hyprland work runs on its own thread: it waits
/// for the window to be mapped, and `hyprctl` must not block the main thread.
pub fn apply(window: &WebviewWindow, on: bool) {
    match support() {
        PinSupport::Native => {
            if let Err(e) = window.set_always_on_top(on) {
                eprintln!("notecal: could not set always-on-top: {e}");
            }
        }
        PinSupport::Hyprland => {
            thread::spawn(move || {
                if let Err(e) = hyprland_pin(std::process::id(), on) {
                    eprintln!("notecal: could not pin the window in Hyprland: {e}");
                }
            });
        }
        PinSupport::None => {}
    }
}

/// Hyprland forgets the pin when the window is hidden, so it is set again on
/// every show. Pin is a toggle there, so it is sent only when the state differs.
fn hyprland_pin(pid: u32, on: bool) -> Result<(), String> {
    // Right after `show` the window may not be mapped yet.
    let mut client = None;
    for _ in 0..10 {
        client = find_client(&hyprctl(&["clients", "-j"])?, pid);
        if client.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let Some(client) = client else {
        return Err("the window was not found in `hyprctl clients`".into());
    };
    if client.pinned == on {
        return Ok(());
    }
    if on && !client.floating {
        return Err(
            "only floating windows can be pinned; add a float window rule \
                    (docs/linux-setup.md)"
                .into(),
        );
    }
    let window = format!("address:{}", client.address);
    // Lua config (Hyprland 0.55+, Omarchy), then the classic syntax.
    let lua = format!("hl.dsp.window.pin({{ window = \"{window}\" }})");
    if hyprctl(&["dispatch", &lua])?.trim() == "ok" {
        return Ok(());
    }
    match hyprctl(&["dispatch", "pin", &window])?.trim() {
        "ok" => Ok(()),
        other => Err(other.to_string()),
    }
}

fn hyprctl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("hyprctl")
        .args(args)
        .output()
        .map_err(|e| format!("hyprctl: {e}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[derive(Debug, PartialEq, Eq)]
struct Client {
    address: String,
    floating: bool,
    pinned: bool,
}

/// Our window in `hyprctl clients -j`: the client whose pid is ours.
fn find_client(json: &str, pid: u32) -> Option<Client> {
    let clients: Vec<serde_json::Value> = serde_json::from_str(json).ok()?;
    clients
        .iter()
        .find(|c| c["pid"].as_u64() == Some(u64::from(pid)))
        .and_then(|c| {
            Some(Client {
                address: c["address"].as_str()?.to_string(),
                floating: c["floating"].as_bool().unwrap_or(false),
                pinned: c["pinned"].as_bool().unwrap_or(false),
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENTS: &str = r#"[
        {"address": "0x1", "pid": 10, "floating": false, "pinned": false, "class": "Mailspring"},
        {"address": "0x2", "pid": 42, "floating": true, "pinned": true, "class": "notecal"}
    ]"#;

    #[test]
    fn finds_the_window_by_pid() {
        assert_eq!(
            find_client(CLIENTS, 42),
            Some(Client {
                address: "0x2".into(),
                floating: true,
                pinned: true
            })
        );
    }

    #[test]
    fn missing_window_or_bad_output_is_none() {
        assert_eq!(find_client(CLIENTS, 7), None);
        assert_eq!(find_client("not json", 42), None);
        assert_eq!(find_client(r#"[{"pid": 42}]"#, 42), None);
    }
}

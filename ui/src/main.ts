import { mount } from "svelte";
import App from "./App.svelte";
import { startTheming } from "./theme";

const target = document.getElementById("app");
if (!target) throw new Error("#app element missing from index.html");

startTheming();

export default mount(App, { target });

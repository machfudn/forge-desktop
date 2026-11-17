import { installGlobalErrorListeners } from "./utils/appLog.svelte";
import { mount } from "svelte";
import "./styles/main.css";
import "./styles/skins.css";
import "./styles/neobrutalism.css";
import "./styles/riso.css";
import "./styles/blueprint.css";
import App from "./App.svelte";

installGlobalErrorListeners();
const target = document.getElementById("app");
if (!target) {
  throw new Error("Elemen root #app tidak ditemukan");
}

const app = mount(App, {
  target,
});

export default app;

import "@fontsource-variable/archivo";
import "@fontsource-variable/jetbrains-mono";
import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";

const app = mount(App, { target: document.getElementById("app")! });
export default app;

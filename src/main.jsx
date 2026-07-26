import { render } from "preact";
import { App } from "./app.jsx";
import { initTheme } from "./lib/theme.js";
import "./styles/tokens.css";
import "./styles/glass.css";

// 在渲染前应用主题，避免闪白
initTheme();

render(<App />, document.getElementById("app"));

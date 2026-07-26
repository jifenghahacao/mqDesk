// 主题管理：读取/保存/应用系统主题
const THEME_KEY = "mqdesk-theme";

export function getStoredTheme() {
  return localStorage.getItem(THEME_KEY) || "light";
}

export function setStoredTheme(theme) {
  localStorage.setItem(THEME_KEY, theme);
  applyTheme(theme);
}

export function applyTheme(theme) {
  if (theme === "dark") {
    document.documentElement.setAttribute("data-theme", "dark");
  } else if (theme === "system") {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    document.documentElement.setAttribute("data-theme", prefersDark ? "dark" : "light");
  } else {
    document.documentElement.removeAttribute("data-theme");
  }
}

export function initTheme() {
  const theme = getStoredTheme();
  applyTheme(theme);

  // 监听系统主题变化（仅当用户选择「跟随系统」时）
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (getStoredTheme() === "system") {
      applyTheme("system");
    }
  });
}

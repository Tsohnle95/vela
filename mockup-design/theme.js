(() => {
  const storageKey = "vela-mockup-theme";
  const direction = new URLSearchParams(window.location.search).get("direction");
  const defaultTheme = direction === "06" ? "dark" : "light";

  const readTheme = () => {
    try {
      const saved = window.localStorage.getItem(storageKey);
      return saved === "dark" || saved === "light" ? saved : defaultTheme;
    } catch {
      return defaultTheme;
    }
  };

  const applyTheme = (theme) => {
    document.documentElement.dataset.theme = theme;
    const toggle = document.querySelector("[data-theme-toggle]");
    toggle?.setAttribute("aria-pressed", String(theme === "dark"));
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", theme === "dark" ? "#20221f" : "#f4f3ef");
  };

  applyTheme(readTheme());

  const connectToggle = () => {
    const toggle = document.querySelector("[data-theme-toggle]");
    if (!toggle) return;

    toggle.setAttribute("aria-pressed", String(document.documentElement.dataset.theme === "dark"));
    toggle.addEventListener("click", () => {
      const theme = document.documentElement.dataset.theme === "dark" ? "light" : "dark";
      applyTheme(theme);
      try {
        window.localStorage.setItem(storageKey, theme);
      } catch {
        // The toggle still works when browser storage is unavailable.
      }
    });
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", connectToggle, { once: true });
  } else {
    connectToggle();
  }
})();

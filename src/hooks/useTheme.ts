import { appearanceVariables } from "../lib/appearances";
import { useLayoutEffect } from "react";
import { useConfigStore } from "../stores/configStore";

/**
 * Applies the current theme (dark/light/system) to the document root element.
 * When "system" is selected, listens for OS theme changes and updates in real time.
 * Persists theme choice to the config store automatically (handled by the setter).
 */
export function useTheme() {
  const theme = useConfigStore((s) => s.theme);

  const appearance = useConfigStore((s) => s.appearance);

  useLayoutEffect(() => {
    const root = document.documentElement;

    let appliedKeys: string[] = [];
    root.dataset.appearance = appearance;
    function applyTheme(mode: "dark" | "light") {
      root.classList.remove("dark", "light");
      root.classList.add(mode);
      for (const key of appliedKeys) root.style.removeProperty(key);
      const vars = appearanceVariables(appearance, mode);
      for (const [key, value] of Object.entries(vars)) root.style.setProperty(key, value);
      appliedKeys = Object.keys(vars);
    }

    if (theme === "system") {
      const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
      // Apply immediately
      applyTheme(mediaQuery.matches ? "dark" : "light");

      // Listen for OS theme changes
      const handler = (e: MediaQueryListEvent) => {
        applyTheme(e.matches ? "dark" : "light");
      };
      mediaQuery.addEventListener("change", handler);

      return () => {
        for (const key of appliedKeys) root.style.removeProperty(key);
        mediaQuery.removeEventListener("change", handler);
      };
    } else {
      applyTheme(theme);
      return () => { for (const key of appliedKeys) root.style.removeProperty(key); };
    }
  }, [theme, appearance]);
}

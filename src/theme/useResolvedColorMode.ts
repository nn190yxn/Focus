import { useEffect, useState } from "react";

import type { AppearancePreference } from "../features/settings/types";
import { resolveColorMode, type ColorMode } from "./theme";

export function useResolvedColorMode(appearance: AppearancePreference): ColorMode {
  const [systemDark, setSystemDark] = useState(() => systemPrefersDark());

  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(query.matches);
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);

  return resolveColorMode(appearance, systemDark);
}

function systemPrefersDark(): boolean {
  return typeof window !== "undefined"
    && typeof window.matchMedia === "function"
    && window.matchMedia("(prefers-color-scheme: dark)").matches;
}

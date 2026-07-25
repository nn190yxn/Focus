import { useEffect, useState } from "react";

import type { LanguagePreference } from "../features/settings/types";

export const supportedLocales = ["zh-CN", "en-US"] as const;
export type SupportedLocale = (typeof supportedLocales)[number];

export function resolveLocale(
  preference: LanguagePreference,
  systemLanguages: readonly string[] = [],
): SupportedLocale {
  if (preference === "zhCn") return "zh-CN";
  if (preference === "en") return "en-US";
  const language = systemLanguages.find(Boolean)?.toLowerCase() ?? "";
  return language.startsWith("zh") ? "zh-CN" : "en-US";
}

export function useResolvedLocale(preference: LanguagePreference): SupportedLocale {
  const [systemLanguages, setSystemLanguages] = useState(readSystemLanguages);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const update = () => setSystemLanguages(readSystemLanguages());
    window.addEventListener("languagechange", update);
    return () => window.removeEventListener("languagechange", update);
  }, []);

  return resolveLocale(preference, systemLanguages);
}

function readSystemLanguages(): string[] {
  if (typeof navigator === "undefined") return [];
  return navigator.languages.length > 0 ? [...navigator.languages] : [navigator.language];
}

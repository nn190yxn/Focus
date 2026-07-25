import { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";

import { messages, type MessageKey } from "./messages";
import type { SupportedLocale } from "./locale";

type MessageParams = Record<string, string | number>;

export interface I18nValue {
  locale: SupportedLocale;
  t: (key: MessageKey, params?: MessageParams) => string;
  formatDate: (value: Date | string, options?: Intl.DateTimeFormatOptions) => string;
  formatTime: (value: Date | string, options?: Intl.DateTimeFormatOptions) => string;
  formatRelativeTime: (value: number, unit: Intl.RelativeTimeFormatUnit) => string;
}

export function createI18n(locale: SupportedLocale): I18nValue {
  return {
    locale,
    t: (key, params) => interpolate(messages[locale][key], params),
    formatDate: (value, options) => new Intl.DateTimeFormat(locale, options).format(toDate(value)),
    formatTime: (value, options) => new Intl.DateTimeFormat(locale, options ?? { hour: "2-digit", minute: "2-digit" }).format(toDate(value)),
    formatRelativeTime: (value, unit) => new Intl.RelativeTimeFormat(locale, { numeric: "auto" }).format(value, unit),
  };
}

const I18nContext = createContext<I18nValue>(createI18n("zh-CN"));

export function I18nProvider({ locale, children }: { locale: SupportedLocale; children: ReactNode }) {
  const value = useMemo(() => createI18n(locale), [locale]);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  return useContext(I18nContext);
}

function interpolate(template: string, params?: MessageParams): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, key: string) => String(params[key] ?? match));
}

function toDate(value: Date | string): Date {
  if (value instanceof Date) return value;
  const localDate = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (localDate) return new Date(Number(localDate[1]), Number(localDate[2]) - 1, Number(localDate[3]), 12);
  return new Date(value);
}

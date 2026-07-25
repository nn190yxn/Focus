export const themeNames = ["mint", "noir", "office", "blush"] as const;
export const colorModes = ["light", "dark"] as const;

export type ThemeName = (typeof themeNames)[number];
export type ColorMode = (typeof colorModes)[number];
export type ThemeSurface = "main" | "widget";

export type SemanticThemeTokens = Readonly<{
  canvas: string;
  surface: string;
  surfaceMuted: string;
  text: string;
  textMuted: string;
  border: string;
  accent: string;
  accentStrong: string;
  accentContrast: string;
  focusRing: string;
  success: string;
  warning: string;
  danger: string;
  decorative: string;
}>;

const lightBase = {
  surface: "oklch(0.995 0.004 184)",
  text: "oklch(0.285 0.025 205)",
  textMuted: "oklch(0.47 0.02 205)",
  border: "oklch(0.89 0.018 184)",
  accentContrast: "oklch(1 0 0)",
  focusRing: "oklch(0.42 0.13 220)",
  success: "oklch(0.43 0.13 151)",
  warning: "oklch(0.43 0.12 77)",
  danger: "oklch(0.49 0.19 28)",
} as const;

const darkBase = {
  surface: "oklch(0.225 0.018 205)",
  text: "oklch(0.94 0.012 184)",
  textMuted: "oklch(0.72 0.018 190)",
  border: "oklch(0.36 0.02 196)",
  accentContrast: "oklch(0.16 0.012 205)",
  focusRing: "oklch(0.82 0.1 210)",
  success: "oklch(0.76 0.12 151)",
  warning: "oklch(0.8 0.12 77)",
  danger: "oklch(0.76 0.15 28)",
} as const;

const palettes: Record<ThemeName, Record<ColorMode, Omit<SemanticThemeTokens, keyof typeof lightBase>>> = {
  mint: {
    light: {
      canvas: "oklch(0.975 0.012 184)",
      surfaceMuted: "oklch(0.955 0.018 184)",
      accent: "oklch(0.67 0.105 177)",
      accentStrong: "oklch(0.46 0.105 177)",
      decorative: "oklch(0.9 0.055 176)",
    },
    dark: {
      canvas: "oklch(0.18 0.022 190)",
      surfaceMuted: "oklch(0.27 0.03 187)",
      accent: "oklch(0.73 0.11 174)",
      accentStrong: "oklch(0.8 0.1 174)",
      decorative: "oklch(0.34 0.07 178)",
    },
  },
  noir: {
    light: {
      canvas: "oklch(0.955 0.01 82)",
      surfaceMuted: "oklch(0.92 0.018 82)",
      accent: "oklch(0.62 0.115 78)",
      accentStrong: "oklch(0.44 0.095 72)",
      decorative: "oklch(0.84 0.07 79)",
    },
    dark: {
      canvas: "oklch(0.15 0.012 72)",
      surfaceMuted: "oklch(0.25 0.022 76)",
      accent: "oklch(0.76 0.12 80)",
      accentStrong: "oklch(0.84 0.105 83)",
      decorative: "oklch(0.31 0.06 77)",
    },
  },
  office: {
    light: {
      canvas: "oklch(0.97 0.013 242)",
      surfaceMuted: "oklch(0.94 0.025 242)",
      accent: "oklch(0.59 0.13 247)",
      accentStrong: "oklch(0.44 0.13 249)",
      decorative: "oklch(0.86 0.07 240)",
    },
    dark: {
      canvas: "oklch(0.17 0.025 248)",
      surfaceMuted: "oklch(0.25 0.04 245)",
      accent: "oklch(0.7 0.13 242)",
      accentStrong: "oklch(0.79 0.11 239)",
      decorative: "oklch(0.31 0.08 244)",
    },
  },
  blush: {
    light: {
      canvas: "oklch(0.975 0.014 354)",
      surfaceMuted: "oklch(0.945 0.03 354)",
      accent: "oklch(0.65 0.13 353)",
      accentStrong: "oklch(0.46 0.14 350)",
      decorative: "oklch(0.88 0.07 352)",
    },
    dark: {
      canvas: "oklch(0.18 0.022 350)",
      surfaceMuted: "oklch(0.27 0.04 350)",
      accent: "oklch(0.72 0.13 350)",
      accentStrong: "oklch(0.8 0.11 352)",
      decorative: "oklch(0.34 0.08 350)",
    },
  },
};

export function resolveThemeTokens(
  _surface: ThemeSurface,
  theme: ThemeName,
  mode: ColorMode,
): SemanticThemeTokens {
  return { ...(mode === "light" ? lightBase : darkBase), ...palettes[theme][mode] };
}

export function resolveColorMode(
  appearance: "system" | ColorMode,
  systemPrefersDark: boolean,
): ColorMode {
  return appearance === "system" ? (systemPrefersDark ? "dark" : "light") : appearance;
}

export function themeStyle(tokens: SemanticThemeTokens): React.CSSProperties {
  return {
    "--color-canvas": tokens.canvas,
    "--color-surface": tokens.surface,
    "--color-surface-muted": tokens.surfaceMuted,
    "--color-text": tokens.text,
    "--color-text-muted": tokens.textMuted,
    "--color-border": tokens.border,
    "--color-accent": tokens.accent,
    "--color-accent-strong": tokens.accentStrong,
    "--color-accent-contrast": tokens.accentContrast,
    "--color-focus-ring": tokens.focusRing,
    "--color-success": tokens.success,
    "--color-warning": tokens.warning,
    "--color-danger": tokens.danger,
    "--color-decorative": tokens.decorative,
  } as React.CSSProperties;
}

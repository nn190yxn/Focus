import { useEffect, useId, useRef } from "react";
import type { ButtonHTMLAttributes, HTMLAttributes, InputHTMLAttributes, KeyboardEvent, ReactNode, SelectHTMLAttributes } from "react";
import { useI18n } from "../i18n/I18nContext";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  tone?: "primary" | "secondary" | "ghost" | "danger";
};

export function Button({ className = "", tone = "secondary", ...props }: ButtonProps) {
  return <button className={`button button--${tone} ${className}`} {...props} />;
}

export function Panel({ className = "", ...props }: HTMLAttributes<HTMLElement>) {
  return <section className={`panel ${className}`} {...props} />;
}

type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  tone?: "neutral" | "accent" | "success" | "warning" | "danger";
};

export function Badge({ className = "", tone = "neutral", ...props }: BadgeProps) {
  return <span className={`badge badge--${tone} ${className}`} {...props} />;
}

export function Progress({ label, value }: { label: string; value: number }) {
  const safeValue = Math.min(100, Math.max(0, value));
  return (
    <div className="progress" aria-label={label} aria-valuenow={safeValue} aria-valuemin={0} aria-valuemax={100} role="progressbar">
      <span style={{ width: `${safeValue}%` }} />
    </div>
  );
}

type DialogProps = {
  open: boolean;
  title: string;
  children: ReactNode;
  onClose: () => void;
};

export function Dialog({ open, title, children, onClose }: DialogProps) {
  const { t } = useI18n();
  const titleId = useId();
  const dialogRef = useRef<HTMLElement>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);
  const wasOpenRef = useRef(false);

  if (open && !wasOpenRef.current && typeof document !== "undefined") {
    restoreFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  }
  wasOpenRef.current = open;

  useEffect(() => {
    if (!open) return;

    const dialog = dialogRef.current;
    const focusable = Array.from(dialog?.querySelectorAll<HTMLElement>(
      "button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [href], [tabindex]:not([tabindex='-1'])",
    ) ?? []);
    const browserAutofocus = dialog?.contains(document.activeElement)
      ? document.activeElement as HTMLElement
      : null;
    const initialFocus = browserAutofocus
      ?? dialog?.querySelector<HTMLElement>("[autofocus]")
      ?? focusable[0];
    (initialFocus ?? dialog)?.focus();

    return () => restoreFocusRef.current?.focus();
  }, [open]);

  function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== "Tab") return;

    const focusable = Array.from(dialogRef.current?.querySelectorAll<HTMLElement>(
      "button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [href], [tabindex]:not([tabindex='-1'])",
    ) ?? []).filter((element) => !element.hidden);
    if (focusable.length === 0) {
      event.preventDefault();
      dialogRef.current?.focus();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && (document.activeElement === first || document.activeElement === dialogRef.current)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  if (!open) return null;
  return (
    <div className="dialog-backdrop" role="presentation" onMouseDown={onClose}>
      <section ref={dialogRef} className="dialog" role="dialog" aria-modal="true" aria-labelledby={titleId} tabIndex={-1} onKeyDown={handleKeyDown} onMouseDown={(event) => event.stopPropagation()}>
        <header>
          <h2 id={titleId}>{title}</h2>
          <Button tone="ghost" aria-label={t("common.closeDialog")} onClick={onClose}>{t("common.close")}</Button>
        </header>
        {children}
      </section>
    </div>
  );
}

export function Toast({ children, tone = "neutral" }: { children: ReactNode; tone?: BadgeProps["tone"] }) {
  return <div className={`toast toast--${tone}`} role={tone === "danger" ? "alert" : "status"}>{children}</div>;
}

type SegmentedOption<T extends string> = { value: T; label: string };

export function SegmentedControl<T extends string>({ label, options, value, onChange }: {
  label: string;
  options: readonly SegmentedOption<T>[];
  value: T;
  onChange: (value: T) => void;
}) {
  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const direction = event.key === "ArrowRight" || event.key === "ArrowDown"
      ? 1
      : event.key === "ArrowLeft" || event.key === "ArrowUp"
        ? -1
        : 0;
    const nextIndex = event.key === "Home"
      ? 0
      : event.key === "End"
        ? options.length - 1
        : direction
          ? (index + direction + options.length) % options.length
          : index;
    if (nextIndex === index && !["Home", "End"].includes(event.key)) return;

    event.preventDefault();
    const nextButton = event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>("[role='radio']")[nextIndex];
    nextButton?.focus();
    onChange(options[nextIndex].value);
  }

  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((option, index) => (
        <button key={option.value} type="button" role="radio" aria-checked={option.value === value} tabIndex={option.value === value ? 0 : -1} onKeyDown={(event) => handleKeyDown(event, index)} onClick={() => onChange(option.value)}>
          {option.label}
        </button>
      ))}
    </div>
  );
}

export function Select({ className = "", error, ...props }: SelectHTMLAttributes<HTMLSelectElement> & { error?: string }) {
  return (
    <label className="field">
      <select className={`select ${className}`} aria-invalid={Boolean(error)} {...props} />
      {error ? <span className="field__error">{error}</span> : null}
    </label>
  );
}

export function DateTimePicker({ label, error, ...props }: InputHTMLAttributes<HTMLInputElement> & { label: string; error?: string }) {
  return (
    <label className="field">
      <span className="field__label">{label}</span>
      <input className="date-time" type="datetime-local" aria-invalid={Boolean(error)} {...props} />
      {error ? <span className="field__error">{error}</span> : null}
    </label>
  );
}

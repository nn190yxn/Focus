import type { SVGProps } from "react";

export type IconName = "today" | "memos" | "projects" | "focus" | "calendar" | "settings" | "plus" | "clock" | "pin";

const paths: Record<IconName, string> = {
  today: "M5 4h14v15H5z M8 2v4 M16 2v4 M5 9h14 M8 13h3 M8 16h6",
  memos: "M6 3h12v18H6z M9 7h6 M9 11h6 M9 15h4",
  projects: "M4 6h6l2 2h8v11H4z M8 12h8 M8 15h6",
  focus: "M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8z M12 2v3 M12 19v3 M2 12h3 M19 12h3",
  calendar: "M5 4h14v16H5z M8 2v4 M16 2v4 M5 9h14 M9 13h2 M14 13h2 M9 16h2",
  settings: "M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8z M12 2v3 M12 19v3 M2 12h3 M19 12h3 M5 5l2 2 M17 17l2 2 M19 5l-2 2 M7 17l-2 2",
  plus: "M12 5v14 M5 12h14",
  clock: "M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18z M12 7v5l3 2",
  pin: "M9 3h6l-1 6 3 3v2H7v-2l3-3z M12 14v7",
};

export function Icon({ name, ...props }: SVGProps<SVGSVGElement> & { name: IconName }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...props}>
      <path d={paths[name]} />
    </svg>
  );
}

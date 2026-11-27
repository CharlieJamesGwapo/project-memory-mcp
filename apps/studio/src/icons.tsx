import type { CSSProperties } from "react";
const paths = {
  arrow: "M5 12h14m-5-5 5 5-5 5",
  refresh:
    "M20 7v5h-5M4 17v-5h5M6.1 7a7 7 0 0 1 11.6-2L20 8M4 16l2.3 3A7 7 0 0 0 18 17",
  note: "M6 3h9l4 4v14H6V3Zm8 0v5h5M9 12h7M9 16h5",
  check: "m5 12 4 4L19 6",
  clock: "M12 8v5l3 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0",
  stack: "m3 7 9-4 9 4-9 4-9-4Zm0 5 9 4 9-4M3 17l9 4 9-4",
  search: "M20 20l-5-5M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0",
  close: "m6 6 12 12M6 18 18 6",
  plus: "M12 5v14M5 12h14",
  copy: "M9 8h11v13H9V8ZM5 16H3V3h11v2",
  download: "M12 3v12m-5-5 5 5 5-5M4 17v4h16v-4",
  folder: "M3 6h7l2 2h9v12H3V6Z",
  overview: "M3 3h7v7H3V3Zm11 0h7v7h-7V3ZM3 14h7v7H3v-7Zm11 0h7v7h-7v-7Z",
  shield: "M12 3 4 6v6c0 5 8 9 8 9s8-4 8-9V6l-8-3Zm-4 9 3 3 5-6",
} as const;
export type IconName = keyof typeof paths;
export function Icon({
  name,
  size = 15,
  style,
}: {
  name: IconName;
  size?: number;
  style?: CSSProperties;
}) {
  return (
    <svg
      aria-hidden="true"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.45"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={style}
    >
      <path d={paths[name]} />
    </svg>
  );
}
export function Brand() {
  return (
    <div className="brand">
      <div className="brand-mark" aria-hidden="true">
        <i />
        <i />
        <i />
        <i />
      </div>
      <div className="brand-name">
        Project Memory<span>Studio</span>
      </div>
    </div>
  );
}

import type { ReactNode, SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;
type GlyphProps = { className?: string };

// Font glyphs share the rail's crisp 16px rasterization; hand-drawn SVG
// strokes at this size land on fractional pixels.
function Codicon({ name, className }: { name: string; className?: string }): ReactNode {
  return <i className={`codicon codicon-${name}${className ? ` ${className}` : ""}`} aria-hidden="true" />;
}

function Icon({ name, children, className, ...rest }: IconProps & { name: string; children: ReactNode }): ReactNode {
  return (
    <svg
      className={`os-icon codicon codicon-${name}${className ? ` ${className}` : ""}`}
      width="1em"
      height="1em"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...rest}
    >
      {children}
    </svg>
  );
}

function LucideIcon({ name, children, className, ...rest }: IconProps & { name: string; children: ReactNode }): ReactNode {
  return (
    <svg
      className={`os-icon lucide-icon lucide-${name}${className ? ` ${className}` : ""}`}
      width="1em"
      height="1em"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...rest}
    >
      {children}
    </svg>
  );
}

// Filled glyphs copied verbatim from the reference app's icon set.
function CodexIcon({ name, children, className, ...rest }: IconProps & { name: string; children: ReactNode }): ReactNode {
  return (
    <svg
      className={`os-icon codex-icon codex-${name}${className ? ` ${className}` : ""}`}
      width="1em"
      height="1em"
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
      {...rest}
    >
      {children}
    </svg>
  );
}

// `magnifying_glass_search.svg` from the reference app's icon assets.
export function IconSearch(props: IconProps): ReactNode {
  return (
    <CodexIcon name="search" {...props}>
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M10.875 4.5C7.35418 4.5 4.5 7.35418 4.5 10.875C4.5 14.3958 7.35418 17.25 10.875 17.25C14.3958 17.25 17.25 14.3958 17.25 10.875C17.25 7.35418 14.3958 4.5 10.875 4.5ZM2.5 10.875C2.5 6.24962 6.24962 2.5 10.875 2.5C15.5004 2.5 19.25 6.24962 19.25 10.875C19.25 12.8273 18.582 14.6236 17.462 16.0478L21.2071 19.7929C21.5976 20.1834 21.5976 20.8166 21.2071 21.2071C20.8166 21.5976 20.1834 21.5976 19.7929 21.2071L16.0478 17.462C14.6236 18.582 12.8273 19.25 10.875 19.25C6.24962 19.25 2.5 15.5004 2.5 10.875Z"
      />
    </CodexIcon>
  );
}

// Lucide geometry (lucide.dev, ISC) matches the reference app's panel controls.
export function IconPanelLeftClose(props: IconProps): ReactNode {
  return (
    <LucideIcon name="panel-left-close" {...props}>
      <rect width="18" height="18" x="3" y="3" rx="2" />
      <path d="M9 3v18" />
      <path d="m16 15-3-3 3-3" />
    </LucideIcon>
  );
}

// Titlebar chrome is drawn on the same 16-unit grid as the codicon rail.
// Lucide's 24-unit geometry would scale by 2/3 at 16px, leaving strokes and
// edges on fractional pixels. Half-unit coordinates keep the 1px strokes
// crisp at 1x and 2x.
const PANEL_FRAME = <rect x="2.5" y="2.5" width="11" height="11" rx="2" />;

// The reference app's sidebar toggle is the plain panel-left glyph (its
// `panel_left.svg`); the dashed variant (`panel_left_dashed.svg`) marks the
// hidden state.
export function IconPanelLeft(props: IconProps): ReactNode {
  return (
    <Icon name="panel-left" strokeWidth={1} {...props}>
      {PANEL_FRAME}
      <path d="M6.5 2.5v11" />
    </Icon>
  );
}

export function IconPanelLeftDashed(props: IconProps): ReactNode {
  return (
    <Icon name="panel-left-dashed" strokeWidth={1} {...props}>
      {PANEL_FRAME}
      <path d="M6.5 3.5v1M6.5 6v1M6.5 9v1M6.5 12v1" />
    </Icon>
  );
}

export function IconPanelLeftOpen(props: IconProps): ReactNode {
  return (
    <LucideIcon name="panel-left-open" {...props}>
      <rect width="18" height="18" x="3" y="3" rx="2" />
      <path d="M9 3v18" />
      <path d="m14 9 3 3-3 3" />
    </LucideIcon>
  );
}

// The right-panel toggle is the same control mirrored, so the two titlebar
// controls read as one pair.
export function IconPanelRight(props: IconProps): ReactNode {
  return (
    <Icon name="panel-right" strokeWidth={1} {...props}>
      {PANEL_FRAME}
      <path d="M9.5 2.5v11" />
    </Icon>
  );
}

export function IconPanelRightDashed(props: IconProps): ReactNode {
  return (
    <Icon name="panel-right-dashed" strokeWidth={1} {...props}>
      {PANEL_FRAME}
      <path d="M9.5 3.5v1M9.5 6v1M9.5 9v1M9.5 12v1" />
    </Icon>
  );
}

export function IconPanelRightClose(props: IconProps): ReactNode {
  return (
    <LucideIcon name="panel-right-close" {...props}>
      <rect width="18" height="18" x="3" y="3" rx="2" />
      <path d="M15 3v18" />
      <path d="m8 9 3 3-3 3" />
    </LucideIcon>
  );
}

export function IconPanelRightOpen(props: IconProps): ReactNode {
  return (
    <LucideIcon name="panel-right-open" {...props}>
      <rect width="18" height="18" x="3" y="3" rx="2" />
      <path d="M15 3v18" />
      <path d="m10 15-3-3 3-3" />
    </LucideIcon>
  );
}

export function IconSquarePen({ className }: GlyphProps): ReactNode {
  return <Codicon name="new-session" className={className} />;
}

export function IconAdd({ className }: GlyphProps): ReactNode {
  return <Codicon name="add" className={className} />;
}

export function IconSquarePlus(props: IconProps): ReactNode {
  return (
    <Icon name="plus-square" strokeWidth={1} {...props}>
      {PANEL_FRAME}
      <path d="M8.5 5.5v5M5.5 8.5h5" />
    </Icon>
  );
}

export function IconAgentMode(props: IconProps): ReactNode {
  return (
    <Icon name="bot" strokeWidth={1} {...props}>
      <rect x="3.5" y="5.5" width="9" height="8" rx="2" />
      <path d="M8.5 5.5V2.5H6.5" />
      <path d="M1.5 7.5v3M14.5 7.5v3" />
      <path d="M6.5 8.5v2M9.5 8.5v2" />
    </Icon>
  );
}

export function IconArrowDown(props: IconProps): ReactNode {
  return (
    <Icon name="arrow-down" {...props}>
      <path d="M8 3v9.6M4.4 9.2L8 12.8l3.6-3.6" />
    </Icon>
  );
}

export function IconArrowLeft(props: IconProps): ReactNode {
  return (
    <Icon name="arrow-left" {...props}>
      <path d="M13 8H3.4M6.8 4.4L3.2 8l3.6 3.6" />
    </Icon>
  );
}

export function IconArrowRight(props: IconProps): ReactNode {
  return (
    <Icon name="arrow-right" {...props}>
      <path d="M3 8h9.6M9.2 4.4L12.8 8l-3.6 3.6" />
    </Icon>
  );
}

export function IconArrowUp(props: IconProps): ReactNode {
  return (
    <Icon name="arrow-up" {...props}>
      <path d="M8 13V3.4M4.4 6.8L8 3.2l3.6 3.6" />
    </Icon>
  );
}

export function IconCheck(props: IconProps): ReactNode {
  return (
    <Icon name="check" {...props}>
      <path d="M3.4 8.6l3 3 6.2-6.8" />
    </Icon>
  );
}

export function IconChevronDown({ className }: GlyphProps): ReactNode {
  return <Codicon name="chevron-down" className={className} />;
}

export function IconChevronRight({ className }: GlyphProps): ReactNode {
  return <Codicon name="chevron-right" className={className} />;
}

export function IconChevronUp({ className }: GlyphProps): ReactNode {
  return <Codicon name="chevron-up" className={className} />;
}

export function IconClose(props: IconProps): ReactNode {
  return (
    <Icon name="close" {...props}>
      <path d="M4.2 4.2l7.6 7.6M11.8 4.2l-7.6 7.6" />
    </Icon>
  );
}

export function IconCloudDownload(props: IconProps): ReactNode {
  return (
    <Icon name="cloud-download" {...props}>
      <path d="M4.9 10.9a2.7 2.7 0 0 1-.4-5.3 3.8 3.8 0 0 1 7.4-.9 2.65 2.65 0 0 1 .9 5.1" />
      <path d="M8 7.6v6M5.9 11.5L8 13.6l2.1-2.1" />
    </Icon>
  );
}

export function IconDashboard(props: IconProps): ReactNode {
  return (
    <Icon name="dashboard" {...props}>
      <path d="M2.6 12.2a5.4 5.4 0 1 1 10.8 0" />
      <path d="M8 12.2l2.5-3.6" />
      <circle cx="8" cy="12.2" r="0.4" fill="currentColor" stroke="none" />
    </Icon>
  );
}

export function IconEdit(props: IconProps): ReactNode {
  return (
    <Icon name="edit" {...props}>
      <path d="M3.2 12.8l.7-2.9 6.7-6.7a1.53 1.53 0 0 1 2.2 2.2l-6.7 6.7-2.9.7z" />
      <path d="M10 4.8l1.2 1.2" />
    </Icon>
  );
}

export function IconEye(props: IconProps): ReactNode {
  return (
    <Icon name="eye" {...props}>
      <path d="M2.2 8s2.3-3.8 5.8-3.8S13.8 8 13.8 8s-2.3 3.8-5.8 3.8S2.2 8 2.2 8z" />
      <circle cx="8" cy="8" r="1.6" />
    </Icon>
  );
}

export function IconEyeClosed(props: IconProps): ReactNode {
  return (
    <Icon name="eye-closed" {...props}>
      <path d="M2.4 8.6c1.6 1.8 3.5 2.7 5.6 2.7s4-.9 5.6-2.7" />
      <path d="M8 11.3v2.1M4.9 10.5l-1.1 1.8M11.1 10.5l1.1 1.8" />
    </Icon>
  );
}

export function IconFile(props: IconProps): ReactNode {
  return (
    <Icon name="file" {...props}>
      <path d="M4.2 1.8h4.6l3.4 3.4v8.2a1.4 1.4 0 0 1-1.4 1.4H4.2a1.4 1.4 0 0 1-1.4-1.4V3.2a1.4 1.4 0 0 1 1.4-1.4z" />
      <path d="M8.8 1.8v3.6h3.4" />
    </Icon>
  );
}

export function IconFolder(props: IconProps): ReactNode {
  return (
    <Icon name="folder" {...props}>
      <path d="M2.2 4.4c0-.9.7-1.6 1.6-1.6h2.4c.47 0 .92.21 1.22.58l.88 1.02h4.28c.9 0 1.6.72 1.6 1.6v5.6c0 .88-.7 1.6-1.6 1.6H3.8c-.9 0-1.6-.72-1.6-1.6V4.4z" />
    </Icon>
  );
}

export function IconFolderOpen(props: IconProps): ReactNode {
  return (
    <Icon name="folder-opened" {...props}>
      <path d="M2.2 6.4V4.4c0-.9.7-1.6 1.6-1.6h2.4c.47 0 .92.21 1.22.58l.88 1.02h4.28c.9 0 1.6.72 1.6 1.6v1" />
      <path d="M2.1 12.1l1.6-3.7c.21-.49.69-.8 1.22-.8h8.06c.83 0 1.4.82 1.13 1.6l-1.03 2.95c-.19.55-.71.92-1.29.92H3.25c-.85 0-1.44-.84-1.15-1.64z" />
    </Icon>
  );
}

export function IconGear({ className }: GlyphProps): ReactNode {
  return <Codicon name="settings-gear" className={className} />;
}

export function IconGitBranch({ className }: GlyphProps): ReactNode {
  return <Codicon name="git-branch" className={className} />;
}

export function IconGlobe(props: IconProps): ReactNode {
  return (
    <Icon name="globe" {...props}>
      <circle cx="8" cy="8" r="5.7" />
      <path d="M2.4 8h11.2" />
      <path d="M8 2.3c1.6 1.6 2.4 3.5 2.4 5.7S9.6 12.1 8 13.7C6.4 12.1 5.6 10.2 5.6 8s.8-4.1 2.4-5.7z" />
    </Icon>
  );
}

export function IconHistory(props: IconProps): ReactNode {
  return (
    <Icon name="history" {...props}>
      <circle cx="8" cy="8" r="5.8" />
      <path d="M8 4.8V8l2.3 1.4" />
    </Icon>
  );
}

export function IconImage(props: IconProps): ReactNode {
  return (
    <Icon name="image" {...props}>
      <rect x="2.2" y="3" width="11.6" height="10" rx="1.8" />
      <circle cx="5.9" cy="6.6" r="0.95" />
      <path d="M4.6 12l2.7-3.3 2.1 2.6 1.65-1.9 1.75 2.6" />
    </Icon>
  );
}

export function IconMic(props: IconProps): ReactNode {
  return (
    <Icon name="mic" {...props}>
      <path d="M8 2.2a2.1 2.1 0 0 1 2.1 2.1v3.2a2.1 2.1 0 0 1-4.2 0V4.3A2.1 2.1 0 0 1 8 2.2z" />
      <path d="M5.2 7.5a2.8 2.8 0 0 0 5.6 0M8 10.3v2.2M6.2 12.5h3.6" />
    </Icon>
  );
}

export function IconMoreVertical(props: IconProps): ReactNode {
  return (
    <Icon name="more-vertical" {...props}>
      <circle cx="8" cy="3" r="0.75" fill="currentColor" />
      <circle cx="8" cy="8" r="0.75" fill="currentColor" />
      <circle cx="8" cy="13" r="0.75" fill="currentColor" />
    </Icon>
  );
}

export function IconMoreHorizontal(props: IconProps): ReactNode {
  return (
    <Icon name="more-horizontal" {...props}>
      <circle cx="3" cy="8" r="0.75" fill="currentColor" />
      <circle cx="8" cy="8" r="0.75" fill="currentColor" />
      <circle cx="13" cy="8" r="0.75" fill="currentColor" />
    </Icon>
  );
}

export function IconRefresh(props: IconProps): ReactNode {
  return (
    <Icon name="refresh" {...props}>
      <path d="M13.2 8A5.2 5.2 0 1 1 11.5 4.2" />
      <path d="M13.4 2.4v2.9h-2.9" />
    </Icon>
  );
}

export function IconRobot(props: IconProps): ReactNode {
  return (
    <Icon name="robot" {...props}>
      <rect x="2.8" y="5.4" width="10.4" height="6.8" rx="1.8" />
      <path d="M8 5.4V3.6" />
      <circle cx="8" cy="2.7" r="0.9" />
      <path d="M5.9 8.1v1.7M10.1 8.1v1.7" />
    </Icon>
  );
}

export function IconServer(props: IconProps): ReactNode {
  return (
    <Icon name="server" {...props}>
      <rect x="2.6" y="2.6" width="10.8" height="4.4" rx="1.2" />
      <rect x="2.6" y="9" width="10.8" height="4.4" rx="1.2" />
      <path d="M5 4.8h.01M5 11.2h.01" />
    </Icon>
  );
}

export function IconShield(props: IconProps): ReactNode {
  return (
    <Icon name="shield" {...props}>
      <path d="M8 1.9l4.6 1.7v3.6c0 3-1.8 5.2-4.6 6.6-2.8-1.4-4.6-3.6-4.6-6.6V3.6L8 1.9z" />
      <path d="M6.3 7.7l1.2 1.2 2.2-2.4" />
    </Icon>
  );
}

export function IconStarFilled(props: IconProps): ReactNode {
  return (
    <Icon name="star-full" {...props}>
      <path
        d="M8 2.2l1.7 3.5 3.9.5-2.9 2.7.7 3.9L8 10.9l-3.4 1.9.7-3.9L2.4 6.2l3.9-.5L8 2.2z"
        fill="currentColor"
        stroke="none"
      />
    </Icon>
  );
}

export function IconSymbolEvent(props: IconProps): ReactNode {
  return (
    <Icon name="symbol-event" {...props}>
      <path d="M8 2.6L13.4 8 8 13.4 2.6 8 8 2.6z" />
    </Icon>
  );
}

export function IconStop(props: IconProps): ReactNode {
  return (
    <Icon name="stop" {...props}>
      <rect x="4.4" y="4.4" width="7.2" height="7.2" rx="1.6" fill="currentColor" stroke="none" />
    </Icon>
  );
}

export function IconTerminal(props: IconProps): ReactNode {
  return (
    <Icon name="terminal" {...props}>
      <rect x="2.2" y="3" width="11.6" height="10" rx="1.8" />
      <path d="M5 6.6L7 8.4l-2 1.8M8.8 10.6h2.8" />
    </Icon>
  );
}

export type TrustedApplicationLocation =
  | { kind: "exact"; value: string }
  | { kind: "origin"; value: string };

type FrameLike = { url: string };
type WebContentsLike = { mainFrame: FrameLike };
type IpcEventLike = { sender: WebContentsLike; senderFrame: FrameLike | null };

export function applicationUrl(isPackaged: boolean, developmentUrl: string | undefined, packagedUrl: string): string {
  if (isPackaged || developmentUrl === undefined) return packagedUrl;
  const url = new URL(developmentUrl);
  const localHosts = new Set(["localhost", "127.0.0.1", "[::1]"]);
  if (url.protocol !== "http:" || !localHosts.has(url.hostname) || url.username || url.password) {
    throw new Error("Unsupported development renderer URL");
  }
  return url.href;
}

export function trustedApplicationLocation(value: string): TrustedApplicationLocation {
  const url = new URL(value);
  if ((url.protocol === "http:" || url.protocol === "https:") && !url.username && !url.password) {
    return { kind: "origin", value: url.origin };
  }
  if (url.protocol === "file:" && !url.username && !url.password) {
    return { kind: "exact", value: url.href };
  }
  throw new Error("Unsupported application URL");
}

export function isTrustedApplicationUrl(value: string, trusted: TrustedApplicationLocation): boolean {
  try {
    const url = new URL(value);
    if (url.username || url.password) return false;
    return trusted.kind === "origin" ? url.origin === trusted.value : url.href === trusted.value;
  } catch {
    return false;
  }
}

export function isAllowedMainFrameNavigation(
  value: string,
  isMainFrame: boolean,
  trusted: TrustedApplicationLocation
): boolean {
  return !isMainFrame || isTrustedApplicationUrl(value, trusted);
}

export function isTrustedIpcSender(
  event: IpcEventLike,
  expected: WebContentsLike | null,
  trusted: TrustedApplicationLocation
): boolean {
  return expected !== null &&
    event.senderFrame !== null &&
    event.sender === expected &&
    event.senderFrame === expected.mainFrame &&
    isTrustedApplicationUrl(event.senderFrame.url, trusted);
}

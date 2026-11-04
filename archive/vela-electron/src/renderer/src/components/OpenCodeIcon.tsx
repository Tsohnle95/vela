// OpenCode V2 icon geometry, MIT, pinned to 03e67171 (styles/opencode/LICENSE).
import type { ReactNode } from "react";

const icons = {
  "square-arrow-top-right": {
    "viewBox": "0 0 20 20",
    "body": "<path d=\"M7.91675 2.9165H2.91675V17.0832H17.0834V12.0832M12.0834 2.9165H17.0834V7.9165M9.58342 10.4165L16.6667 3.33317\" stroke=\"currentColor\" stroke-linecap=\"square\"/>"
  },
  "circle-ban-sign": {
    "viewBox": "0 0 20 20",
    "body": "<path d=\"M15.3675 4.63087L4.55742 15.441M17.9163 9.9987C17.9163 14.371 14.3719 17.9154 9.99967 17.9154C7.81355 17.9154 5.83438 17.0293 4.40175 15.5966C2.96911 14.164 2.08301 12.1848 2.08301 9.9987C2.08301 5.62644 5.62742 2.08203 9.99967 2.08203C12.1858 2.08203 14.165 2.96813 15.5976 4.40077C17.0302 5.8334 17.9163 7.81257 17.9163 9.9987Z\" stroke=\"currentColor\" stroke-linecap=\"round\"/>"
  },
  "chevron-grabber-vertical": {
    "viewBox": "0 0 20 20",
    "body": "<path d=\"M6.66675 12.4998L10.0001 15.8332L13.3334 12.4998M6.66675 7.49984L10.0001 4.1665L13.3334 7.49984\" stroke=\"currentColor\" stroke-linecap=\"square\"/>"
  },
  "copy": {
    "viewBox": "0 0 20 20",
    "body": "<path d=\"M6.2513 6.24935V2.91602H17.0846V13.7493H13.7513M13.7513 6.24935V17.0827H2.91797V6.24935H13.7513Z\" stroke=\"currentColor\" stroke-linecap=\"round\"/>"
  },
  "edit": {
    "viewBox": "0 0 16 16",
    "body": "<path d=\"M13.5555 8.21534V13.5556H2.44434L2.44434 2.4445H7.78462M6.88878 9.11119C6.88878 9.11119 8.96327 9.0367 9.69678 8.3032L14.0301 3.96986C14.5824 3.4176 14.5824 2.52213 14.0301 1.96986C13.4778 1.4176 12.5824 1.4176 12.0301 1.96986L7.69678 6.3032C7.00513 6.99484 6.88878 9.11119 6.88878 9.11119Z\" stroke=\"currentColor\"/>"
  },
  "folder": {
    "viewBox": "0 0 16 16",
    "body": "<path d=\"M2.545 3.364V12.636H13.455V5H8.545L6.909 3.364H2.545Z\" stroke=\"currentColor\" stroke-miterlimit=\"10\" stroke-linecap=\"square\"/>"
  },
  "magnifying-glass": {
    "viewBox": "0 0 16 16",
    "body": "<path d=\"M14 14L10.3454 10.3454M6.88889 11.7778C9.58889 11.7778 11.7778 9.58889 11.7778 6.88889C11.7778 4.18889 9.58889 2 6.88889 2C4.18889 2 2 4.18889 2 6.88889C2 9.58889 4.18889 11.7778 6.88889 11.7778Z\" stroke=\"currentColor\"/>"
  },
  "chevron-down": {
    "viewBox": "0 0 16 16",
    "body": "<path d=\"M5 6.5L8 9.5L11 6.5\" stroke=\"currentColor\"/>"
  },
  "check": {
    "viewBox": "0 0 16 16",
    "body": "<path d=\"M3.53613 8.17857L6.39328 11.75L12.4647 4.25\" stroke=\"currentColor\"/>"
  },
  "close": {
    "viewBox": "0 0 20 20",
    "body": "<path d=\"M14.4446 5.55566L5.55566 14.4446M5.55566 5.55566L14.4446 14.4446\" stroke=\"currentColor\" stroke-linejoin=\"round\"/>"
  },
  "outline-chevron-down": {
    "viewBox": "0 0 16 16",
    "body": "<path d=\"M5 6.5L8 9.5L11 6.5\" stroke=\"currentColor\"/>"
  },
  "expand": {
    "viewBox": "0 0 16 16",
    "body": "<path d=\"M8.25 6.17773V1.17773M11.25 4.17773L8.25 1.17773L5.25 4.17773\" stroke=\"currentColor\"/><path d=\"M8.25 9.17773V14.1777M11.25 11.1777L8.25 14.1777L5.25 11.1777\" stroke=\"currentColor\"/><path d=\"M4.25 7.67773H12.25\" stroke=\"currentColor\"/>"
  },
  "outline-copy": {
    "viewBox": "0 0 16 16",
    "body": "<path d=\"M4.14908 11.0081H1.76282V1.51758H9.1038V2.55588M14.2225 4.99681H6.75397V14.4873H14.2225V4.99681Z\" stroke=\"currentColor\"/>"
  },
  "reset": {
    "viewBox": "0 0 20 20",
    "body": "<path d=\"M5.83333 4.16406L2.5 7.4974L5.83333 10.8307M3.33333 7.4974H17.9167V15.4141H10\" stroke=\"currentColor\" stroke-linecap=\"square\"/>"
  }
} as const;

export function OpenCodeIcon({ name }: { name: keyof typeof icons }): ReactNode {
  const icon = icons[name];
  return <span data-component="icon" data-size="small"><svg data-slot="icon-svg" width="16" height="16" viewBox={icon.viewBox} fill="none" aria-hidden="true" dangerouslySetInnerHTML={{ __html: icon.body }} /></span>;
}

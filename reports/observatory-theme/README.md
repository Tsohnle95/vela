# Observatory appearance verification

Select **Settings → Appearance → Observatory**. The new palette, decorative
pixel wordmark, syntax colours, folder glyphs, observatory backdrop and violet
composer are scoped to this appearance. Both Codex and Classic interfaces work.

## Native visual checks

The production renderer and real preload ran in an isolated native macOS
Electron window, with a local fixture backend supplying a portfolio file and
conversation. Native mouse events resized the docked panel to its actual 280 px
minimum. No runtime prompts or user workspaces were changed.

- [Docked, 325 px](02-docked.png)
- [Minimum, 280 px](03-minimum-280.png)
- [Minimum panel menu](04-minimum-menu.png)
- [Wide Agent Mode](05-agent-mode.png)
- [1920 px window](05b-agent-mode-1920.png)
- [980 × 594 window](05c-agent-mode-980.png)
- [Classic interface](07b-classic.png)
- [Conversation readability](08-conversation.png)
- [Theme switch back to Prism](09-prism-restored.png)

[verification.json](verification.json) records measured panel and composer
bounds. The scene uses cover sizing with no stretching, frames the observatory
in narrow panels and centres the full landscape in wide panels. Assertions
confirmed no horizontal page overflow and the composer stayed inside every
checked panel. The wide composer measured 700 px. The minimum composer measured
259 px. The panel menu remained visible at minimum width. A renderer reload
restored the selected appearance. Switching to Prism hid the scene and wordmark.
The wide artwork and existing application layout are responsive; this is not a
pixel-for-pixel replacement of the reference mockup's window geometry.

## Repository checks

`npm run check` passed with Node **22.23.2**, matching `.node-version` and engines:
173 test files passed, 2 skipped; 1,433 tests passed, 4 skipped. Type checking,
documentation, architecture checks and production compilation passed.

## Artwork source and editing

- Shell reference: `design/experimental-art/smaller-agent-view.png`.
- Scene reference: `design/experimental-art/ChatGPT Image Oct 14, 2026, 05_22_22 PM-7.png`.
- Saved scene: `src/renderer/src/assets/agent-scenes/observatory.png`.
- Pixel wordmark: `src/renderer/src/assets/observatory-wordmark.svg` (code-native SVG).
- Scene edit used the built-in imagegen tool. The original reference is retained.

Final edit prompt:

> Use case: precise-object-edit. Edit target: provided observatory application mockup. Produce the SAME pixel-art observatory, astronaut, purple spiral galaxy, planets, rocks and stars with exactly the same composition, colors and pixel textures. Remove ONLY the UI overlays: remove the entire top titlebar (the first 58 pixels) and fill it naturally with continuing starry sky; remove the bottom floating rounded prompt input/composer and all its text/buttons (rectangle approximately x442..1145 y848..963 in the 1586x992 input), reconstruct that small area naturally with continuing rocks and foreground. Output a clean full-bleed landscape wallpaper at the same aspect ratio. No UI chrome, no text, no buttons, no logos, no frames. Preserve the observatory at left, astronaut lower left center, diagonal galaxy upper right center and planets right exactly as shown. This is asset extraction for matching an existing design, NOT a new interpretation.

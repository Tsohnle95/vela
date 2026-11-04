# Appearance settings layout

The Appearance page uses a compact header, a segmented Codex/Classic interface
choice, and a responsive gallery of all 15 themes. Each theme shows a preview,
name and selected checkmark. Theme descriptions remain available on hover.
The Observatory artwork is visible in its thumbnail for every active palette.

Native macOS Electron checks used the compiled renderer and real preload with
an isolated local backend fixture. Verified light, dark and Observatory palettes,
both interfaces, persistence after reload, the close control and window widths
of 1280, 780 and 600 px. All 15 themes remain available, exactly one is selected,
the close button stays beside the heading, preview syntax lines remain visible,
and no horizontal settings overflow occurs. At 1280 × 900 all themes fit onscreen.

- [Light](01-codex-light.png)
- [Dark](02-codex-dark.png)
- [Observatory](03-observatory.png)
- [Classic interface](04-classic-interface.png)
- [Narrow](05-narrow.png)
- [Small window](07-small.png)
- [Measured results](verification.json)

`npm run check` passed under supported Node 22.23.2: 173 test files passed,
2 skipped; 1,433 tests passed, 4 skipped. Type checking, documentation,
architecture and production compilation passed.

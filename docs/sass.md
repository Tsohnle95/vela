# Workspace Sass compilation

This guide covers user-workspace CSS output. Vela interface styles have a
[separate application-only live-save owner](main.md), independent of this toggle.

`src/main/sass-watch.ts` owns local watchers, compiler lifetimes, preferences,
and CSS disk output. It never installs tools or executes workspace scripts.
Compilation uses Vela's shipped `sass` dependency in `sass-worker.ts`, keeping
CPU-heavy Sass off Electron's main thread.

## Compiler research and selection

Official sources checked on 2026-10-07:

| Option | Cost / redistribution | Maintenance and compatibility | Disk output / Vela fit |
| --- | --- | --- | --- |
| [Dart Sass `sass`](https://sass-lang.com/documentation/js-api/) | Free; [MIT](https://github.com/sass/dart-sass/blob/main/LICENSE). Retain notices, including the Dart SDK notices bundled in the npm package's LICENSE. | Maintained Sass reference implementation; [recent releases](https://github.com/sass/dart-sass/releases). Pure JavaScript supports Node platforms. The installed package requires Node ≥20.19, compatible with Vela's Node 22 toolchain and Electron. | CLI writes CSS; API returns CSS that Vela writes to the selected workspace. Chosen for portable dependency packaging without a native compiler executable. |
| [`sass-embedded`](https://github.com/sass/embedded-host-node) | Free; [MIT host](https://github.com/sass/embedded-host-node/blob/main/LICENSE), plus bundled compiler/runtime notices. | Same Dart Sass language/API; generally faster. Current host requires Node ≥20.19 and ships platform-specific executables for macOS, Windows, Linux and additional listed targets. | Supports the same CSS outputs, but adds native executable packaging and architecture testing. |
| [Live Sass Compiler, glenn2223](https://marketplace.visualstudio.com/items?itemName=glenn2223.live-sass) | Free MIT extension; redistribution retains its notices. Donations are optional. | Maintained fork; [v6.1.5](https://github.com/glenn2223/vscode-live-sass-compiler/releases/tag/v6.1.5) released February 2026. Requires the VS Code extension host/API, not a standalone Node library. | Writes configured CSS for VS Code workspaces. Vela has no VS Code extension host; it detects compatible output settings and uses Sass directly. |

The [installation guide](https://sass-lang.com/install/) and
[JS API documentation](https://sass-lang.com/documentation/js-api/) explain
the portability/speed tradeoff. Vela promotes its already installed, locked
compiler to a production dependency without downloading an update. Versions
are owned by package.json/package-lock.json; app packaging retains the compiler
package and its LICENSE. No paid service, PATH installation, npx fallback, or
workspace dependency installation is involved.

## Ownership and lifecycle

- A canonical directory owns one watcher/compiler and its preference. Agent
  contexts for the same directory share it and receive status tagged with each
  immutable workspace identity. Closing the last context stops it. Selection
  changes never redirect a worker's root.
- On/off preferences live in Electron user data's atomic `sass-watch.json`,
  with a backup. Close/quit releases resources while retaining the on preference
  for reopen. Only opened, valid roots restore; late initialization/results
  cannot resurrect closed contexts.
- Saved source/partial changes debounce into serialized batches which recompile
  discovered entrypoints. Unsaved editor buffers are never compiled. A stalled
  worker times out after 30 seconds; toggle off/on to recreate it.
- Entrypoints are `.scss` and `.sass` files not beginning with `_`. Directories
  listed in `SASS_IGNORES` are excluded and symlinks are not traversed. Syntax
  errors preserve existing CSS and watching recovers after saved corrections.
- Output uses temporary files and atomic rename. Mappings stay within the root;
  symlink outputs/directories and collisions are rejected. Stop cancels queued
  compiler work and waits for in-flight writes before reporting off.

## Output conventions

Precedence: `vela-sass.json` (or a prior product-branded Sass config when the
Vela file is absent), compatible Live Sass workspace settings, literal Sass CLI mappings in
package scripts, then CSS beside its source:
`css/styles.scss` → `css/styles.css`. Directory mappings preserve relative paths.
Workspace scripts and JavaScript build configuration are never executed.
Dynamic mappings, custom JS importers/functions, and post-processing such as
autoprefixer are not run; use an explicit mapping for complex projects.

```json
{
  "entries": {
    "src/sass": "public/css",
    "themes/dark.scss": "public/dark.css"
  },
  "style": "expanded",
  "loadPaths": ["src/sass/includes"],
  "sourceMap": false
}
```

Entries/load paths are workspace-relative. Omitted entries mean all discovered
non-partial files compile beside themselves. Source maps are optional and use
relative source URLs, suitable for static hosting. Static HTML must link to the
generated CSS paths; Vela does not rewrite workspace HTML.

Detected Live Sass settings: formats (style, extensionName, savePath or literal
savePathReplacementPairs), generateMap, includeItems, and excludeList. Root `/css`
and source-relative `~/../css` keep their extension semantics. Literal
`sass input.scss output.css` and `sass source:destination` package commands also
supply style, source-map, and local load-path flags. Unsupported dynamic mappings
use the default; the log names the convention actually selected.

## UI and validation

The bottom-right footer button shows off, starting, on, and error for the
selected workspace. The terminal tray's Sass Output tab shows bounded compile
logs and errors for that same workspace. Logs and state are delivered through
the trusted preload bridge, not OpenCode's response stream.

`src/main/sass-watch.test.ts` uses real Sass and filesystem watching in temporary
roots; `src/renderer/src/components/EditorStatusMenu.sass.test.tsx` covers selection and
state transitions. After building, `npm run test:sass-worker` checks the emitted
worker and bundled compiler without network access. Mirror-app checks cover the
real preload/IPC, footer, output panel and CSS disk writes.

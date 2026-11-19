# Building and installing Vela

This builds the native macOS app and installs it into `~/Applications` as
**Vela Dev**. You do not need to launch it from a terminal after installation.

## One command

From the repository root:

```sh
bash scripts/install-dev-app.sh
```

That command:

- builds the native Rust app,
- packages the debug `.app` bundle,
- installs it at `~/Applications/Vela Dev.app`,
- installs the Vela icon,
- keeps all app state in one folder:
  `~/Library/Application Support/Vela Dev`,
- loads the bundled app, not the build output.

Quit the app with **Cmd+Q** before running the installer again.

## Open it

Double-click **Vela Dev** in `~/Applications`, or drag it to the Dock.

From a terminal:

```sh
open "$HOME/Applications/Vela Dev.app"
```

Opening it this way is safe: the installed bundle uses the isolated Vela Dev
profile and does not touch production Vela/OpenCode data.

## What the app stores

Everything runtime-related lives under:

```text
~/Library/Application Support/Vela Dev/
```

That includes the profile, OpenCode config/data/state, logs, and downloads.
The app bundle itself is:

```text
~/Applications/Vela Dev.app
```

To move the app, move the `.app` bundle. Leave the `Vela Dev` data folder in
place.

## Build without installing

If you only want the packaged bundle:

```sh
cargo build -p vela
bash scripts/package-macos.sh debug
```

The bundle is written to:

```text
../build-junk/vela-target/debug/Vela Native.app
```

Do not double-click that bundle directly; it uses the default production
profile. Install it through `scripts/install-dev-app.sh` for isolated use.

If package tooling reports missing preview resources, run this once:

```sh
bash scripts/setup-preview-toolchain.sh
```

## Verify the build

```sh
bash scripts/verify.sh
```

This runs documentation checks, workspace Clippy, the full test suite, and a
debug app build.

For the release/CI gate:

```sh
bash scripts/verify.sh --release
```

## Troubleshooting

- **“Cargo was not found”**: set `VELA_CARGO` to your Cargo path, for example
  `VELA_CARGO="$HOME/.cargo/bin/cargo" bash scripts/install-dev-app.sh`.
- **“Quit Vela Dev before reinstalling”**: press Cmd+Q in the app, then rerun
  the installer.
- **OpenCode was not found**: install OpenCode or run the installer with
  `VELA_OPENCODE_BINARY=/path/to/opencode`.
- **The app opens the welcome screen**: open a folder/workspace from the app,
  or launch the terminal helper with a workspace:
  `./scripts/dev-app.sh /path/to/workspace`.
- **Logs**: for the installed app, use Console.app and filter for `Vela`. The
  terminal helper `./scripts/dev-app.sh` writes to
  `~/Library/Application Support/Vela Dev/logs/app.log`.

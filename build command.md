# Vela build commands

Run these commands from Terminal on your Mac.

```sh
cd "$HOME/code-repositories/vela"
export PATH="$HOME/.cargo/bin:$PATH"
```

You’ll need the Xcode Command Line Tools, Rust via rustup, Python 3, and
OpenCode V2. Set up the preview tools before packaging for the first time, or
after clearing the build cache.

```sh
bash scripts/setup-preview-toolchain.sh
```

## Build and test a fix

Quit any running Vela Dev or Vela Native window with **Cmd+Q**, then run:

```sh
bash scripts/dev-app.sh
```

This rebuilds the current checkout in debug mode and opens it with the Vela
repository as the workspace. It uses an isolated Vela Dev profile and OpenCode
service. To test in a different workspace, pass its folder to the script.

To install a double-clickable test build in your personal Applications folder:

```sh
bash scripts/install-dev-app.sh && open "$HOME/Applications/Vela Dev.app"
```

Rerun the same command after each fix to replace **Vela Dev**. Its test data
lives in `~/Library/Application Support/Vela Dev`.

For an agent panel or other UI fix, run the UI tests. Use the second command
for the full workspace checks and a debug build.

```sh
cargo test --locked -p vela-ui
bash scripts/verify.sh
```

## Build and install the everyday app

Quit Vela Dev and Vela Native before packaging. Build the optimized app,
package it, and reveal the signed bundle in Finder.

```sh
cargo build --locked --release -p vela &&
  bash scripts/package-macos.sh release &&
  source scripts/build-env.sh &&
  open -R "$VELA_TARGET_DIR/release/Vela Native.app"
```

For full checks before packaging, use `bash scripts/verify.sh --release` in
place of the `cargo build` command. It also builds the release binary.

Copy **Vela Native.app** from Finder into **Applications**. Use
`/Applications/Vela Native.app` for the system Applications folder or
`~/Applications/Vela Native.app` for your personal Applications folder. For
updates, quit the installed copy, then choose **Replace** when Finder asks
about the existing **Vela Native.app**. Open the installed copy from Finder
or run:

```sh
open "/Applications/Vela Native.app"
```

Use `open "$HOME/Applications/Vela Native.app"` if you installed it in your
personal Applications folder. This build uses the regular Vela profile at
`~/Library/Application Support/vela-native` and your regular OpenCode setup.
The bundle is ad hoc signed for local use.

See [Development](docs/development.md) for prerequisites, build output
locations, and additional test commands.

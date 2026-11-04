# In-progress parity evidence

These captures are implementation checkpoints, not accepted reference matches.
The migration is incomplete. No page has passed final visual acceptance.

Reference checkout: `../vela`, HEAD
`f25a025f1d19acc26e20dfd3a0d22a1363155804`. Native starting HEAD:
`e882e09c9813301d33811ebc3f6099e88163da16`.

Both real applications were loaded against the dedicated OpenCode service on
port 49555 and the disposable `/private/tmp/vela-test-workspace` workspace.
The original uses `/private/tmp/vela-parity-original` as its user-data profile.
Native runs now use `/private/tmp/vela-parity-native`, skip legacy import, and
launch through `scripts/run-parity.sh` on the non-main display at 1280×800.
Captures are 2× raster images (2560×1600). Earlier native captures were
1426×884 and have been replaced for the Settings baseline.

## Verified interactions in the running native app

- Composer: multiline Unicode paste, select all, cut/delete, undo and redo;
  real keyboard input and visible selection. Platform IME range handling is
  implemented and unit-tested, but an OS IME session is still unverified.
- Browser: local fixture navigation, URL/title updates, enabled Back state,
  Back navigation, two separately loaded tabs, retained page input after tab
  switching, page paste/select-all/cut, native Undo/Redo. Matching clipboard and
  navigation actions were also exercised in the original application.
- Appearance: selecting Midnight changes the whole native shell and selected
  card. All 15 source palettes are bundled. Every palette/shell combination
  still needs its own screenshot and interaction pass.

The browser tests exposed and fixed focus theft between GPUI and native guest
views. Fixes include first-click handling, Cocoa responder checks, shortcut
propagation, native edit selectors and native undo-manager routing. The
application-owned ephemeral browser partition passed cookie sharing between
two tabs and a full quit/restart isolation check. The fixture showed “yes”
across tabs during one run and “no” when Page B was opened first after restart.
The three `browser-partition-*.png` captures record those states.

## Automated checks at this checkpoint

- `cargo test --workspace --locked --offline`: 59 passed; one explicitly ignored
  performance benchmark. The service and PTY integration tests ran outside the
  sandbox because loopback binding is denied inside it.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`:
  passed.
- Formatting applied with `cargo fmt --all`.

## Visual acceptance

`scripts/compare-screenshots.py` compares unmodified full captures. It refuses
to compare different viewport sizes and reports missing counterparts. It saves
a 50% overlay, an amplified difference image and raw pixel metrics; it does not
resize, crop or mask images to improve results. `comparison/index.html` is the
review gallery. Captures and metrics must be regenerated as implementation
changes; functional results do not imply pixel identity.

Known unfinished areas include editor capabilities and dirty-file lifecycle,
multi-panel/session operations, full transcript rendering/actions, forms,
attachments, catalogs/settings operations, terminals, Git mutations/recovery,
preview/Sass edge cases, mobile pairing, packaging/updating, accessibility and
the complete shell/theme/responsive comparison matrix. The existing older
feature matrix must be reconciled with these fresh results.

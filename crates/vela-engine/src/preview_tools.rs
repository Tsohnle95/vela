//! Resolve the optional Node/Vite/Sass tooling used by preview and validation.
//!
//! Packaged apps read the tools from `Contents/Resources/preview`. Development
//! and test runs use the disposable `build-junk/vela-preview` root configured
//! by `scripts/build-env.sh`; the older in-repo `.tools/preview` path remains a
//! fallback for checkouts that still have it.

use std::path::{Path, PathBuf};

/// Repository root derived from this crate's manifest.
pub(super) fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Bundled app resource directory when running from a packaged macOS app.
pub(super) fn bundled_root() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    executable
        .parent()
        .and_then(Path::parent)
        .map(|path| path.join("Resources/preview"))
}

/// Disposable preview tooling root (`build-junk/vela-preview` by default).
pub(super) fn root() -> PathBuf {
    std::env::var_os("VELA_PREVIEW_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| source_root().join("../build-junk/vela-preview"))
}

/// Node binary installed by `scripts/setup-preview-toolchain.sh`.
pub(super) fn node() -> PathBuf {
    root().join("toolchain/node/bin/node")
}

/// Locked Vite dependency installed under the external preview root.
pub(super) fn vite() -> PathBuf {
    root().join("deps/node_modules/vite/bin/vite.js")
}

/// Legacy in-repo Node binary, kept only as a fallback.
pub(super) fn legacy_node() -> PathBuf {
    source_root().join(".tools/preview/node/bin/node")
}

/// Legacy in-repo Vite entry, kept only as a fallback.
pub(super) fn legacy_vite() -> PathBuf {
    source_root().join("tools/preview/node_modules/vite/bin/vite.js")
}

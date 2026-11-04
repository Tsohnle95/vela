Vite previews use Vite 6.4.3 and Sass 1.102.0, matching the current reference.
The lockfile retains the reference's resolved transitive package versions and
registry integrity hashes for these tools. This directory supplies project
tooling; the application UI and OpenCode transport remain native Rust.

Run `bash scripts/setup-preview-toolchain.sh`. It verifies an official Node
22.23.2 archive and installs these packages into the checkout, with lifecycle
scripts disabled. Nothing is installed globally. The app packager includes the
Node executable, its license, and these packages under `Resources/preview`.
The native engine starts a tool process only when a preview is requested and
owns its readiness, inventory and termination.

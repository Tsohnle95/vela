//! Native terminal support.
//!
//! [`pty`] owns the operating-system PTY and process lifecycle (engine side).
//! [`emulator`] folds PTY bytes into a renderable grid (view side, single
//! threaded). Keeping the two apart means the emulator is unit-testable
//! without spawning a process, and the PTY never needs to know about GPUI.

pub mod emulator;
pub mod input;
pub mod pty;
pub mod size;

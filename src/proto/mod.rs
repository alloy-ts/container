//! Cross-platform guest-agent framing.
//!
//! The host and the guest init share one bidirectional byte stream (AF_VSOCK
//! on Linux, `VZVirtioSocket` on macOS, an AF_HYPERV socket on Windows).
//! Frames are length-prefixed and independent of the transport.
//!
//! ```text
//! offset  size  field
//! 0       4     magic b"CTNR"
//! 4       1     version (1)
//! 5       1     kind
//! 6       2     flags (le)
//! 8       4     id (le) — request id, or guest pid for stdio/exit
//! 12      4     payload length (le)
//! 16      n     payload
//! ```
//!
//! RPC payloads (`Request` / `Response`) are JSON. Stdio payloads are raw
//! bytes. An exit payload is a little-endian `i32` status; `flags` carries the
//! terminating signal number, or 0 when the process called `exit`.

mod frame;
mod messages;

pub(crate) use frame::{Frame, FrameKind, FLAG_EOF};
pub(crate) use messages::{MountSpec, NamespaceSpec, Request, Response};

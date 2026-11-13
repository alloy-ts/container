//! Guest PID 1.
//!
//! The microVM host packs this binary into the initramfs as `/init`. It mounts
//! the virtio-fs root and serves the agent protocol over `AF_VSOCK`; see
//! [`container_native::guest::run_guest_init`].

use std::process::ExitCode;

fn main() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("guest-init: failed to start the runtime: {err}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(container_native::guest::run_guest_init()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("guest-init: {err}");
            ExitCode::FAILURE
        }
    }
}

//! Shortest libkrun boot: built-in init, a directory rootfs, no guest agent.
//!
//! ```text
//! LD_LIBRARY_PATH=/path/to/libkrunfw sudo -E ./target/debug/examples/krun_boot /path/to/rootfs
//! ```
//!
//! `kernelPath` is ignored. libkrun loads the kernel from libkrunfw.

use std::env;
use std::path::PathBuf;
use std::time::Duration;

use container_native::{Container, ContainerOpts, NetworkConfig, RootfsSource};
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| "/tmp/alpine".into());
    let opts = ContainerOpts::builder("alpine")
        .rootfs(RootfsSource::Directory(PathBuf::from(&root)))
        .kernel_path("libkrunfw")
        .vcpus(1)
        .memory_mb(256)
        .args([
            "/bin/busybox",
            "sh",
            "-c",
            "echo hello-from-libkrun; sleep 3",
        ])
        .working_dir("/")
        .network(NetworkConfig::none())
        .build()
        .expect("options");

    eprintln!("krun_create_ctx via Container::create, rootfs={root}");
    let mut container = match Container::create(opts).await {
        Ok(container) => container,
        Err(err) => {
            eprintln!("boot failed: {err}");
            std::process::exit(1);
        }
    };
    eprintln!("create returned; attaching virtio console with start()");
    let mut process = match container.start().await {
        Ok(process) => process,
        Err(err) => {
            eprintln!("start failed: {err}");
            std::process::exit(1);
        }
    };
    let mut buf = vec![0u8; 4096];
    let read = tokio::time::timeout(Duration::from_secs(8), async {
        let stdout = process.stdout.as_mut().expect("stdout");
        stdout.read(&mut buf).await
    })
    .await;
    match read {
        Ok(Ok(0)) => eprintln!("console EOF with no bytes"),
        Ok(Ok(n)) => println!("{}", String::from_utf8_lossy(&buf[..n])),
        Ok(Err(err)) => eprintln!("console read: {err}"),
        Err(_) => eprintln!("console read timed out"),
    }
    container.delete().await.ok();
}

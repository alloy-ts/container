use std::process::Command;

use napi_derive::napi;

/// Container runtimes we will auto-detect, in priority order.
const CANDIDATE_RUNTIMES: &[&str] = &["container", "podman", "docker"];

/// Generate a typed method per real subcommand of a management group.
///
/// Each generated method forwards `<group> <subcommand> [args...]` to the
/// runtime, so the full Apple Container surface is exposed as a discoverable,
/// typed API rather than an opaque stringly-typed call. The macro emits a
/// *separate* `impl Container` block (napi rejects macro invocations inside an
/// existing `#[napi] impl`).
macro_rules! group_subcommands {
  ($group:literal => { $( $rust:ident => $js:literal ),+ $(,)? }) => {
    #[napi]
    impl Container {
      $(
        #[napi(js_name = $js)]
        pub fn $rust(&self, args: Vec<String>) -> napi::Result<String> {
          self.fwd2($group, $js, &args)
        }
      )+
    }
  };
}

/// A programmable handle to an OCI-compatible container runtime.
///
/// The instance does not reimplement container logic — it forwards commands to
/// the underlying external program (whichever runtime was selected), covering
/// the full Apple Container CLI surface (discovered via `container --help`):
///
/// * container lifecycle: create, rm(delete), exec, inspect, kill, ls(list),
///   logs, run, start, stats, stop
/// * `run` / `build` convenience wrappers
/// * management groups with every real subcommand typed: image, volume,
///   network, builder, registry, system — plus the single-letter shortcuts
///   `i`, `r`, `v`, `n`, `s`.
///
/// This is a NAPI-RS *class* (constructed from JS via `new Container(...)`);
/// the resolved runtime identity is exposed through the `runtime` / `version`
/// getters rather than a `#[napi(object)]` struct (a class needs finalization
/// that `#[napi(object)]` does not provide).
#[napi]
pub struct Container {
  /// The resolved runtime binary name, e.g. `"container"` or `"docker"`.
  pub runtime: String,
  /// The detected runtime version (first line of `<runtime> --version`), if known.
  pub version: Option<String>,
}

#[napi]
impl Container {
  /// Create a new container instance.
  ///
  /// * `runtime` — optional explicit runtime name. When omitted or empty, the
  ///   constructor probes the host in the order **container → podman → docker**
  ///   and selects the first one available on `PATH`.
  #[napi(constructor)]
  pub fn new(runtime: Option<String>) -> napi::Result<Self> {
    let selected = match runtime {
      // Explicit runtime: it MUST be available or we fail loudly.
      Some(r) if !r.trim().is_empty() => probe(&r)?.ok_or_else(|| {
        napi::Error::from_reason(format!(
          "requested container runtime `{r}` was not found on this host"
        ))
      })?,
      // No runtime given: auto-detect in priority order.
      _ => CANDIDATE_RUNTIMES
        .iter()
        .find_map(|name| probe(name).ok().flatten())
        .ok_or_else(|| {
          napi::Error::from_reason(
            "no OCI container runtime found on this host (tried: container, podman, docker)"
              .to_string(),
          )
        })?,
    };
    Ok(selected)
  }

  /// The resolved runtime binary name (e.g. `"container"`, `"docker"`).
  #[napi(getter)]
  pub fn runtime(&self) -> String {
    self.runtime.clone()
  }

  /// The detected runtime version, captured at construction time.
  #[napi(getter)]
  pub fn version(&self) -> Option<String> {
    self.version.clone()
  }

  // ---- generic forwarding -------------------------------------------------

  /// Forward an arbitrary command line to the runtime and return its stdout.
  /// A non-zero exit surfaces as a JS error that includes the runtime's stderr.
  ///
  /// NOTE: synchronous — blocks the calling thread. Promote to async for
  /// long-running / streaming commands (`logs -f`, `stats`, interactive `run`).
  #[napi]
  pub fn command(&self, args: Vec<String>) -> napi::Result<String> {
    forward(&self.runtime, &args)
  }

  // ---- container lifecycle (top-level CONTAINER SUBCOMMANDS) --------------
  // Mirror Apple Container's CONTAINER SUBCOMMANDS verbatim. `rm`/`ls` also
  // expose the documented `delete`/`list` aliases where the JS identifier is
  // legal (`delete` is reserved in JS, so `rm` is the canonical method).

  /// `container create ...`
  #[napi]
  pub fn create(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["create"], &args)
  }

  /// `container rm | delete ...` (JS: `container.rm(...)`)
  #[napi(js_name = "rm")]
  pub fn remove(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["rm"], &args)
  }

  /// `container exec ...`
  #[napi]
  pub fn exec(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["exec"], &args)
  }

  /// `container inspect ...`
  #[napi]
  pub fn inspect(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["inspect"], &args)
  }

  /// `container kill ...`
  #[napi]
  pub fn kill(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["kill"], &args)
  }

  /// `container ls | list ...` (JS: `container.ls(...)`)
  #[napi(js_name = "ls")]
  pub fn ls(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["ls"], &args)
  }

  /// `container list ...` — documented alias of `ls`.
  #[napi(js_name = "list")]
  pub fn list(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["ls"], &args)
  }

  /// `container logs ...`
  #[napi]
  pub fn logs(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["logs"], &args)
  }

  /// `container start ...`
  #[napi]
  pub fn start(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["start"], &args)
  }

  /// `container stats ...`
  #[napi]
  pub fn stats(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["stats"], &args)
  }

  /// `container stop ...`
  #[napi]
  pub fn stop(&self, args: Vec<String>) -> napi::Result<String> {
    self.fwd(&["stop"], &args)
  }

  // ---- run / build (convenience) -----------------------------------------

  /// `container run <image> [args...]`
  #[napi]
  pub fn run(&self, image: String, args: Vec<String>) -> napi::Result<String> {
    let mut full = vec!["run".to_string(), image];
    full.extend(args);
    forward(&self.runtime, &full)
  }

  /// `container build <context-dir> [args...]`
  #[napi]
  pub fn build(&self, context: String, args: Vec<String>) -> napi::Result<String> {
    let mut full = vec!["build".to_string(), context];
    full.extend(args);
    forward(&self.runtime, &full)
  }

  // ---- management groups (generic forwarders) ----------------------------
  // Still provided for forward-compatibility / escaping to undocumented
  // subcommands; the typed methods below are the primary, discoverable API.

  /// `container image <subcommand> ...` (delete, inspect, list, load, prune, pull, push, save, tag)
  #[napi]
  pub fn image(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("image", &subcommand, &args)
  }

  /// `container volume <subcommand> ...` (create, delete, list, inspect, prune)
  #[napi]
  pub fn volume(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("volume", &subcommand, &args)
  }

  /// `container network <subcommand> ...` (create, delete, list, inspect, prune)
  #[napi]
  pub fn network(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("network", &subcommand, &args)
  }

  /// `container builder <subcommand> ...` (start, status, stop, delete)
  #[napi]
  pub fn builder(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("builder", &subcommand, &args)
  }

  /// `container registry <subcommand> ...` (login, logout)
  #[napi]
  pub fn registry(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("registry", &subcommand, &args)
  }

  /// `container system <subcommand> ...` (df, dns, kernel, logs, property, start, status, stop, version)
  #[napi]
  pub fn system(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("system", &subcommand, &args)
  }

  // ---- management group single-letter shortcuts --------------------------
  // Apple Container aliases: i=image, r=registry, v=volume, n=network, s=system.

  /// `container i <subcommand> ...` — alias of `image`.
  #[napi(js_name = "i")]
  pub fn image_alias(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("image", &subcommand, &args)
  }

  /// `container r <subcommand> ...` — alias of `registry`.
  #[napi(js_name = "r")]
  pub fn registry_alias(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("registry", &subcommand, &args)
  }

  /// `container v <subcommand> ...` — alias of `volume`.
  #[napi(js_name = "v")]
  pub fn volume_alias(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("volume", &subcommand, &args)
  }

  /// `container n <subcommand> ...` — alias of `network`.
  #[napi(js_name = "n")]
  pub fn network_alias(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("network", &subcommand, &args)
  }

  /// `container s <subcommand> ...` — alias of `system`.
  #[napi(js_name = "s")]
  pub fn system_alias(&self, subcommand: String, args: Vec<String>) -> napi::Result<String> {
    self.fwd2("system", &subcommand, &args)
  }

  // ---- internal helpers --------------------------------------------------

  /// Build `[sub, ...args]` and forward.
  fn fwd(&self, sub: &[&str], args: &[String]) -> napi::Result<String> {
    let mut full: Vec<String> = sub.iter().map(|s| s.to_string()).collect();
    full.extend_from_slice(args);
    forward(&self.runtime, &full)
  }

  /// Build `[group, subcommand, ...args]` and forward.
  fn fwd2(&self, group: &str, subcommand: &str, args: &[String]) -> napi::Result<String> {
    let mut full = vec![group.to_string(), subcommand.to_string()];
    full.extend_from_slice(args);
    forward(&self.runtime, &full)
  }
}

// ---- typed subcommands per group -------------------------------------------
// Generated from `container <group> --help` so the full surface is discoverable
// and typed (not just stringly forwarded via the generic group methods above).
// Emitted as separate `impl Container` blocks because napi rejects macro
// invocations inside an existing `#[napi] impl`.

group_subcommands! {
  "image" => {
    image_delete  => "imageDelete",
    image_inspect => "imageInspect",
    image_list    => "imageList",
    image_load    => "imageLoad",
    image_prune   => "imagePrune",
    image_pull    => "imagePull",
    image_push    => "imagePush",
    image_save    => "imageSave",
    image_tag     => "imageTag",
  }
}

group_subcommands! {
  "volume" => {
    volume_create  => "volumeCreate",
    volume_delete  => "volumeDelete",
    volume_list    => "volumeList",
    volume_inspect => "volumeInspect",
    volume_prune   => "volumePrune",
  }
}

group_subcommands! {
  "network" => {
    network_create  => "networkCreate",
    network_delete  => "networkDelete",
    network_list    => "networkList",
    network_inspect => "networkInspect",
    network_prune   => "networkPrune",
  }
}

group_subcommands! {
  "builder" => {
    builder_start  => "builderStart",
    builder_status => "builderStatus",
    builder_stop   => "builderStop",
    builder_delete => "builderDelete",
  }
}

group_subcommands! {
  "registry" => {
    registry_login  => "registryLogin",
    registry_logout => "registryLogout",
  }
}

group_subcommands! {
  "system" => {
    system_df       => "systemDf",
    system_dns      => "systemDns",
    system_kernel   => "systemKernel",
    system_logs     => "systemLogs",
    system_property => "systemProperty",
    system_start    => "systemStart",
    system_status   => "systemStatus",
    system_stop     => "systemStop",
    system_version  => "systemVersion",
  }
}

/// Probe whether `name` is an available runtime.
///
/// Returns `Ok(Some(Container))` when present (carrying its version),
/// `Ok(None)` when absent, and `Err` only on an unexpected spawn failure
/// (e.g. permission denied), not on a simple "not found".
fn probe(name: &str) -> napi::Result<Option<Container>> {
  match Command::new(name).arg("--version").output() {
    Ok(out) if out.status.success() => {
      let version = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string());
      Ok(Some(Container {
        runtime: name.to_string(),
        version,
      }))
    }
    // Binary missing / not executable -> "not available", not an error.
    Ok(_) => Ok(None),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(napi::Error::from_reason(format!(
      "failed while probing runtime `{name}`: {e}"
    ))),
  }
}

/// Run `<runtime> <args>` and return stdout; on failure surface stderr.
fn forward(runtime: &str, args: &[String]) -> napi::Result<String> {
  let output = Command::new(runtime).args(args).output().map_err(|e| {
    napi::Error::from_reason(format!("failed to spawn runtime `{runtime}`: {e}"))
  })?;

  if !output.status.success() {
    let code = output
      .status
      .code()
      .map(|c| c.to_string())
      .unwrap_or_else(|| "terminated by signal".to_string());
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(napi::Error::from_reason(format!(
      "runtime `{runtime}` exited with status {code}: {stderr}"
    )));
  }

  Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

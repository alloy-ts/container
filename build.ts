import { createRequire } from "node:module";

// --- Workaround: neutralize the NAPI CLI's `/bin/ps` "process incarnation" probe.
// This environment blocks spawning `/bin/ps` (EPERM), which the CLI uses only for
// its build-cache reconciliation and which is safe to stub. We patch the mutable
// CJS `child_process` module so the CLI's live ESM binding picks up the override.
const require = createRequire(import.meta.url);
const childProcess = require("node:child_process");
const origExecFile = childProcess.execFile;
childProcess.execFile = function patchedExecFile(command, args, options, callback) {
  let cb = callback;
  let opts = options;
  if (typeof opts === "function") {
    cb = opts;
    opts = undefined;
  }
  const cmd = typeof command === "string" ? command : String(command);
  if (cmd === "/bin/ps" || cmd.endsWith("/ps")) {
    if (cb) (globalThis.setImmediate ?? setTimeout)(() => cb(null, ""));
    return {};
  }
  return origExecFile.call(this, command, args, opts, cb);
};

async function run() {
  const args = process.argv.slice(2);
  const isRelease = args.includes("--release") || args.includes("-r");
  const useNapiCross = args.includes("--use-napi-cross");
  const crossCompile = args.includes("--cross-compile") || args.includes("-x");
  const useCross = args.includes("--use-cross");

  const targetIdx = args.findIndex((a) => a === "--target" || a === "-t");
  const target = targetIdx !== -1 && args[targetIdx + 1] ? args[targetIdx + 1] : undefined;

  const { NapiCli } = await import("@napi-rs/cli");
  const cli = new NapiCli();
  await cli.build({
    platform: true,
    esm: true,
    outputDir: "./dist",
    release: isRelease,
    target,
    useNapiCross,
    crossCompile,
    useCross,
  });
}

void run().catch((err) => {
  console.error(err);
  process.exit(1);
});

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
    format: "esm",
    outputDir: "./dist",
    jsBinding: "index.js",
    dts: "index.d.ts",
    release: isRelease,
    target,
    useNapiCross,
    crossCompile,
    useCross,
  });
}

await run().catch((err) => {
  console.error(err);
  process.exit(1);
});

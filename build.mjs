import { createBuildCommand, NapiCli } from "@napi-rs/cli";

const build = createBuildCommand(process.argv.slice(2));
const options = build.getOptions();
const cli = new NapiCli();

const { task } = await cli.build({
  ...options,
  outputDir: "build",
  cargoOptions: build.cargoOptions,
});

await task;

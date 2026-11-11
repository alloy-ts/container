// Entry point for the packaged library.
//
// Re-exports the compiled NAPI-RS bindings that `node build.ts` writes into
// `./dist` (the native `.node` binary plus its JS loader / type declarations).
// `vp pack` bundles this file into `dist/main.mjs`; `vp test` imports it
// directly so the native module is loaded without a separate build step.
export * from "../dist/index.js";

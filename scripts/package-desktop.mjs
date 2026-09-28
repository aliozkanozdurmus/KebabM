import { readdirSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { resolve, basename } from "node:path";

// Build first so native dependencies exist before Tauri resolves bundle resources.
const [bundles, signingConfig] = process.argv.slice(2);
if (!bundles) throw new Error("Usage: node scripts/package-desktop.mjs <bundles> [signing-config]");
const configArgs = signingConfig ? ["--config", signingConfig] : [];
function tauri(args) {
  const result = spawnSync(process.execPath, ["scripts/native.mjs", ...args], { stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
tauri(["build", "--no-bundle", ...configArgs]);
if (process.platform === "win32") {
  const output = resolve("src-tauri/target/release");
  const dlls = readdirSync(output).filter(name => name.toLowerCase().endsWith(".dll") && name.toLowerCase() !== "nexq_lib.dll");
  if (!dlls.some(name => name.toLowerCase() === "directml.dll")) {
    throw new Error("ONNX Runtime's DirectML.dll is missing; refusing to create an incomplete installer");
  }
  const config = resolve("src-tauri/runtime.generated.json");
  writeFileSync(config, JSON.stringify({ bundle: {
    resources: Object.fromEntries(dlls.map(name => [resolve(output, name), basename(name)])),
    windows: { bundleVCRuntime: true },
  } }, null, 2));
  configArgs.push("--config", config);
}
tauri(["bundle", "--bundles", bundles, ...configArgs]);

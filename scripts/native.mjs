import { existsSync } from "node:fs";
import { execFileSync, spawnSync } from "node:child_process";
import { join, delimiter } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";

const env = { ...process.env };
// Actions exposes absent secrets as empty strings; Tauri treats their presence as signing intent.
for (const name of ["APPLE_CERTIFICATE", "APPLE_CERTIFICATE_PASSWORD", "APPLE_SIGNING_IDENTITY", "APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID", "TAURI_SIGNING_PRIVATE_KEY", "TAURI_SIGNING_PRIVATE_KEY_PASSWORD"]) {
  if (env[name] === "") delete env[name];
}
// Windows commonly exposes `Path`; duplicate `PATH` keys hide Node/npm from child processes.
const pathKey = Object.keys(env).find(name => name.toLowerCase() === "path") ?? "PATH";
env[pathKey] = [join(homedir(), ".cargo", "bin"), ...(process.platform === "darwin" ? ["/opt/homebrew/bin", "/usr/local/bin"] : []), env[pathKey]].join(delimiter);
if (process.platform === "darwin" && !env.LIBCLANG_PATH) {
  const developer = execFileSync("xcode-select", ["-p"], { encoding: "utf8" }).trim();
  env.LIBCLANG_PATH = ["usr/lib", "Toolchains/XcodeDefault.xctoolchain/usr/lib"].map(p => join(developer, p)).find(p => existsSync(join(p, "libclang.dylib")));
}
if (process.platform === "win32" && !env.LIBCLANG_PATH && existsSync("C:\\Program Files\\LLVM\\bin")) {
  env.LIBCLANG_PATH = "C:\\Program Files\\LLVM\\bin";
}
const cli = fileURLToPath(new URL("../node_modules/@tauri-apps/cli/tauri.js", import.meta.url));
const args = process.argv.slice(2);
if (args[0] === "build" && !args.includes("--bundles")) args.push("--bundles", process.platform === "darwin" ? "app,dmg" : process.platform === "win32" ? "nsis" : "appimage,deb");
const result = spawnSync(process.execPath, [cli, ...args], { stdio: "inherit", env });
if (result.error) console.error(result.error.message);
process.exit(result.status ?? 1);

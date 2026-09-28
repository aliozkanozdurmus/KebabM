import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const name = process.platform === "win32" ? "nexq.exe" : "nexq";
const out = join(root, "release-artifacts");
mkdirSync(out, { recursive: true });
copyFileSync(join(root, "src-tauri", "target", "release", name), join(out, name));
console.log(`Standalone executable copied to ${out}`);

import { copyFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const from = join(root, "src-tauri", "target", "release", "nexq.exe");
const to = join(root, "zaiqoM.exe");

copyFileSync(from, to);
console.log(`zaiqoM.exe updated (${to})`);

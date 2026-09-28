import { readdirSync, mkdirSync, copyFileSync } from "node:fs";
import { join } from "node:path";
const root = "src-tauri/target/release/bundle";
const output = "release-artifacts/installers";
mkdirSync(output, { recursive: true });
let count = 0;
function visit(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory() && !entry.name.endsWith(".app")) visit(path);
    else if (entry.isFile() && /\.(exe|dmg|AppImage|deb)$/.test(entry.name)) {
      copyFileSync(path, join(output, entry.name)); count++;
      console.log(entry.name);
    }
  }
}
visit(root);
if (!count) throw new Error("No installers produced");

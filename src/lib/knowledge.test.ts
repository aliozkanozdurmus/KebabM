/// <reference types="node" />
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";

test("project knowledge commands used by the UI are registered in the native application", () => {
  const frontend = readFileSync(new URL("./knowledge.ts", import.meta.url), "utf8");
  const native = readFileSync(new URL("../../src-tauri/src/lib.rs", import.meta.url), "utf8");
  const handler = native.slice(native.indexOf("tauri::generate_handler!["));
  const commands = [...frontend.matchAll(/invoke(?:<[^;\n]*?>)?\("([a-z_]+)"/g)].map(m => m[1]);
  expect(commands.length).toBeGreaterThan(10);
  for (const command of commands) expect(handler, command).toContain(`project_commands::${command},`);
});

import { writeFileSync } from "node:fs";
const pubkey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim();
const endpoint = process.env.TAURI_UPDATER_ENDPOINT?.trim();
if (!process.env.TAURI_SIGNING_PRIVATE_KEY?.trim() || !pubkey || !endpoint || !endpoint.startsWith("https://")) throw new Error("Signed release requires TAURI_SIGNING_PRIVATE_KEY, TAURI_UPDATER_PUBLIC_KEY and an HTTPS TAURI_UPDATER_ENDPOINT. No updater was enabled.");
writeFileSync("src-tauri/updater.generated.json", JSON.stringify({ bundle: { createUpdaterArtifacts: true }, plugins: { updater: { pubkey, endpoints: [endpoint] } } }, null, 2));
console.log("Signed updater configuration prepared. Build with --config src-tauri/updater.generated.json.");

# Build and acceptance

Use Node 24, stable Rust, CMake, Ninja and LLVM/libclang. Platform prerequisites follow [Tauri documentation](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
npm run check
npx playwright install chromium
npm run test:e2e
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run app
```

The desktop workflow packages macOS ARM64 DMG, Windows x64 NSIS EXE, Linux x64 AppImage/DEB. Release jobs use the tagged revision and publish only after all package jobs pass. SHA-256 sums accompany installers. Unsigned packages are explicitly identified in release notes; updater remains disabled until signing is configured.

Browser tests use controlled native IPC and do not prove actual OS audio, transparency or window behavior. Synthetic credential tests are opt-in and must not use production keys.

For private retrieval evaluation, supply your own dataset and a clean repository at its pinned revision:

```sh
cargo run --locked --manifest-path src-tauri/Cargo.toml --example evaluate_knowledge -- /path/to/repo artifacts/evaluation.json /path/to/private-golden.json
```

Never commit private source excerpts, project evaluation data, credentials or meeting recordings. Keep reports under ignored artifacts/.

Acceptance targets: recall@8 >=90%, source-supported factual claims >=95%, explicit limits in every unknown/conflicting scenario, median first useful answer <=4 seconds and p95 <=8 seconds. These are targets, not measured release claims. Real 60-minute audio/meeting tests and trusted signing remain open; see plan.md.

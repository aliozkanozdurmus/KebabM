# Workspace appearances and identity

Eight app-owned appearances are available under **Settings → Appearance & behavior**:
IBM (original/default), Liquid Glass, Apple, Linear, Notion, Material, GitHub and Terminal.
Each supports light, dark and OS-controlled color modes. Appearance never changes AI,
audio, language, window opacity or source settings. Missing/unknown values resolve to IBM.

`src/lib/appearances.ts` owns palettes and geometry. `useTheme` applies them before paint,
cleans up all overrides on switching back to IBM, and responds to OS changes. The native
config store persists `appearance` and distributes its change event to launcher/overlay.
MCP `set_settings` accepts the same eight identifiers; `theme` remains independent.

Fonts are bundled IBM Plex or native system fonts. Notion uses locally available Georgia
for editorial headings; Terminal uses bundled Plex Mono. No font/CDN requests are needed.
Liquid Glass uses translucent navigation chrome, restrained light and backdrop blur;
answers retain a solid text color. Reduced transparency requests disable backdrop blur.
It is an in-app glass treatment, not a claim to use Apple's native Liquid Glass framework.

Legacy fixed answer/transcript/translation defaults now resolve to semantic colors.
Custom hex colors remain saved and rendered. “Use appearance color” resets that override.
The old saved keys and application/keychain identity remain unchanged.

## References and attribution

Visual palette/type/component references were adapted from
[nexu-io/open-design](https://github.com/nexu-io/open-design/tree/main/design-systems)
on 2026-09-28: `ibm`, `apple`, `linear-app`, `notion`, `material`, `github` (their DESIGN.md files).
The reference project is licensed under Apache-2.0; a copy is in
[licenses/open-design-Apache-2.0.txt](licenses/open-design-Apache-2.0.txt).
Our implementation changes the reference palettes for meeting readability, adds paired
light/dark palettes, uses local font fallbacks, and supplies application-specific controls.
Liquid Glass and Terminal are app-owned interpretations. Names describe visual inspiration;
this application is not affiliated with those companies. No reference company logos are used.

## Logo

The original ZaiqoM mark was generated using the built-in ChatGPT image tool.
Source: `public/brand/zaiqom-mark.png`; [generation prompt](logo-prompt.md).
`BrandMark` serves the launcher, live overlay, setup, startup, about and appearance previews.
The same source generates macOS ICNS, Windows ICO and PNG/tray sizes with Tauri's icon CLI.
Legacy public PNG aliases and favicon.ico also carry the new mark.

## Verification

- Unit contrast checks: text/background and text/surface >= 7:1; secondary text >= 4.5:1;
  primary control text >= 4.5:1 for all 16 palettes. Custom user colors are excluded.
- Browser: eight appearance choices in both modes at 400, 700 and 1440px; keyboard selection,
  persistence/reload, native-style store events, OS changes, IBM cleanup and invalid fallback.
- Browser: 400px live answer in every palette; default rendered text follows the palette;
  existing search/source/error/retry workflows remain covered.
- Visual artifacts are local under `artifacts/appearance-*.png`.
- Browser fixtures exercise production components through mocked native IPC. Native window
  compositing and Windows runtime behavior still require platform acceptance testing.

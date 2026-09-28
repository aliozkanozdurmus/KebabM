# MeetingHelper design decisions

Audience: one engineer preparing for and participating in technical project meetings.
The repeated task is reading a short, sourced answer while listening to the conversation.

IBM preserves the original Carbon surfaces and IBM Plex Sans / Mono as the default.
Selectable Liquid Glass, Apple, Linear, Notion, Material, GitHub and Terminal appearances
share semantic tokens and the same information hierarchy. Color mode is independent.
See docs/design/README.md for palette sources, licensing and verification. Preserve window
controls, compact dividers, meaningful labels and calm contrast. Answer text has the
strongest hierarchy; transcript, translation and technical controls are secondary.

The launcher opens on Today: an uncluttered agenda and recent notes, inspired by Granola’s meeting-first workflow. Projects and All notes are secondary navigation. Project administration and service diagnostics are kept off the default screen. Notebook adds warm paper surfaces with restrained olive accents; existing appearance and color preferences remain available.

Three work areas: project preparation, live help, meeting history. At 700 px the launcher
keeps a compact project sidebar. At 400 px overlay controls wrap. The currently read
answer stays selected when another answer arrives.

Show source revision, local changes, keyword fallback, pending tests, errors and retry
explicitly. No inferred readiness or numerical confidence badges. Keyboard focus remains
visible. Forms have labels; source inspection supports Escape.

Browser verification uses controlled Tauri fixtures at 400, 700 and 1440 px. Native audio,
window transparency, permissions and Zoom routing require a separate acceptance run.

Use the generated KebabM mark through BrandMark. The authoritative source is
public/brand/kebabm-mark.png; native icons derive from it. Do not introduce new logo variants.

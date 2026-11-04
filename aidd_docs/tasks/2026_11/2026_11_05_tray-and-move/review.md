# Review: tray-and-move

- Rounds: 1
- Started: 2026-11-05

## Round 1 · r-d163

- Date: 2026-11-05
- By: blzbk
- Diff: `8414314...1ad9e59`
- Axes: code, functional, relevancy
- Verdict: approve
- Score: 7/10 met, 0 unmet, 3 unverified at runtime, 0 out of the diff

### Criteria

- [x] Phase 1, task 1: the 4 placement scenarios are tested and pass with `cargo test -p overlay` — `crates/overlay/src/placement.rs:146-218`, `cargo test --workspace --exclude agenthud-desktop` green
- [x] Phase 1, task 2: the 4 label combinations are tested, `architecture-tests` still passes — `crates/overlay/src/tray_menu.rs:28-58`, workspace tests green
- [x] Phase 2, task 1: Vitest checks the drag-region attribute per mode, the permission is declared — `apps/desktop/src/App.test.tsx:39-48`, `apps/desktop/src-tauri/capabilities/default.json:6`, `pnpm test` 25/25
- Unverified at runtime: Phase 2, task 2: `cargo xwin clippy` passes and `Point` JSON is tested (`placement.rs:146-160`), static parts met; the position surviving a relaunch is recorded OK in `manual-tests.md` on `40afded`, not on the reviewed head (`40afded..1ad9e59` touches docs only)
- Unverified at runtime: Phase 3, task 1: `cargo xwin clippy` passes; each tray entry acting and relabelling is recorded OK on `40afded` (checks 4-6, 8), not on the reviewed head
- Unverified at runtime: Phase 3, task 2: hidden overlay shown on 2nd launch is recorded OK on `40afded` (check 7), not on the reviewed head
- [x] Phase 3, task 3: memory describes the tray and the remembered position — `aidd_docs/memory/desktop.md:7-10`, `aidd_docs/memory/codebase-map.md:17,19`
- [x] Phase 4, task 1: the exe exists — `target/x86_64-pc-windows-msvc/release/agenthud.exe`, `bundle/nsis/AgentHud_0.1.0_x64-setup.exe`
- [x] Phase 4, task 2: the human gave a verdict for each check — `manual-tests.md:9-18`
- [x] Phase 4, task 3: `manual-tests.md` is written — `aidd_docs/tasks/2026_11/2026_11_05_tray-and-move/manual-tests.md`

### Findings

- `crates/overlay/src/placement.rs:101` : an off-screen window goes to the screen it overlaps most, else the primary; phase-1 task 1.2 says "l'écran le plus proche", and a window fully beside a secondary screen lands on the primary, not the nearest → either pick the nearest screen by distance, or align phase-1.md and the doc comment on "overlap" — 🟢 minor, fit
- `apps/desktop/src-tauri/src/tray.rs:73` : toggle + identical error log + `refresh` duplicates the shortcut handler at `apps/desktop/src-tauri/src/lib.rs:34-37` → one `tray`-side helper (e.g. `toggle_mode(app)`) called from both — 🟢 minor, rot
- `apps/desktop/src-tauri/src/window_position.rs:62` : one sleeping task is spawned per `Moved` event, so a drag spawns hundreds that all wake to do nothing → acceptable at this scale; if it grows, keep one debounced task (e.g. a `tokio::sync::watch` channel) — 🟢 minor, performance
- `apps/desktop/src-tauri/src/window_position.rs:67` : `is_minimized().unwrap_or(false)` swallows the error silently, unlike every other fallible call in the module → log it before defaulting — 🟢 minor, error-handling

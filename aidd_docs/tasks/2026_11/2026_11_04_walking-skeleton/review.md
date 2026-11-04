# Review: walking-skeleton

- Rounds: 1
- Started: 2026-11-04

## Round 1 · r-7c3e

- Date: 2026-11-04
- By: blzbk
- Diff: `main...81e9acf`
- Axes: code, functional, relevancy
- Verdict: approve
- Score: 12/15 met, 0 unmet, 3 unverified at runtime, 0 out of the diff

### Criteria

- [x] P1.1 `cargo -V`, `pnpm -v`, `cargo xwin --version` answer in WSL — cargo 1.99.1, pnpm 10.34.9, cargo-xwin 0.23.1 (run on 81e9acf)
- [x] P1.2 `cargo metadata` lists the crates of `crates/` — 6 planned crates plus `overlay` listed (run on 81e9acf)
- [x] P1.3 `cargo fmt --check`, clippy `-D warnings`, `cargo test` exit 0 — host gates run on 81e9acf, all exit 0
- [x] P1.4 `dependency_direction` passes and fails on a forbidden edge — `crates/architecture-tests/tests/dependency_direction.rs:30`, `:67`, both pass
- [x] P2.1 `pnpm typecheck`, `pnpm lint`, `pnpm test` exit 0 from root — run on 81e9acf, all exit 0
- [x] P2.2 Vitest finds « AgentHud » — `apps/desktop/src/App.test.tsx:15`, 6/6 Vitest pass
- [x] P2.3 `cargo xwin clippy -p agenthud-desktop ... -D warnings` exits 0, cross build yields an `.exe` — run on 81e9acf, `target/x86_64-pc-windows-msvc/release/agenthud.exe`
- [x] P2.4 every command of `coding-assertions.md` exits 0 — all 9 run on 81e9acf, exit 0
- [x] P3.4 Vitest shows « interactif » on the event — `apps/desktop/src/App.test.tsx:27`
- [x] P4.1 exe in `release/`, installer in `bundle/nsis/` — `AgentHud_0.1.0_x64-setup.exe` built on 81e9acf
- [x] P4.2 per-process values supplied by the human — `ram-measurement.md`
- [x] P4.3 `ram-measurement.md` holds the total and a verdict against 100 MB — `ram-measurement.md` (91,6 Mo, ✅)
- Unverified at runtime: P3.1 transparent, on top, borderless, no taskbar entry — `manual-tests.md` records it on 14b2303, not on 81e9acf
- Unverified at runtime: P3.2 clicks go through at startup (toggle unit test passes in `crates/overlay/src/lib.rs:47`) — recorded on 14b2303 only
- Unverified at runtime: P3.3 `Ctrl+Alt+H` toggles and toggles back — recorded on 14b2303 only; the new `overlay_mode` invoke path never ran on Windows

### Findings

- `aidd_docs/tasks/2026_11/2026_11_04_walking-skeleton/phase-3.md:21` : projection and task 2 still name `src-tauri/src/overlay.rs`, phase-1 criterion 2 still says 6 crates; the code now has `crates/overlay` and `overlay_window.rs` → update both phase files to the delivered structure — 🟢 minor, fit
- `crates/architecture-tests/tests/dependency_direction.rs:14` : the new `overlay` crate has no rule, so it could grow a dependency on `application` or an adapter unnoticed; `architecture.md` does not place it either → add an `overlay` arm rejecting any workspace dependency, plus one line in `architecture.md` — 🟢 minor, architecture
- `aidd_docs/memory/vcs.md:17` : commit scopes list each crate but not `overlay` → add `overlay` to the scopes — 🟢 minor, conform
- `apps/desktop/src/features/overlay/useOverlayMode.ts:26` : `invoke` has no rejection handler (unhandled promise rejection if the command is denied or fails), and the `changedSinceLoad` guard at `:27` is untested → add `.catch` that logs and keeps the default, and a Vitest case where the event lands before `invoke` resolves — 🟢 minor, frontend

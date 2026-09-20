# Merge review — 2026-09-20

## Follow-up: macOS merge preparation

User explicitly limited readiness to macOS and requested commits. Fixed CLI
bootstrap, macOS externalBin inclusion, installed filename resolution, and managed
symlink upgrades. Bundle inspection also found the existing audio-routing helper
was not executable and had the same filename mismatch; both are fixed. Applied
repository Rust formatting so the CI formatting gate passes.

Validation: 373 Rust tests passed, 5 hardware tests ignored; cargo clippy -- -D
warnings and cargo fmt --check pass. Earlier frontend check and 125 frontend
tests passed; final production frontend build passes. A release CLI bootstrapped
without a staged sidecar. An unsigned native macOS app bundle built successfully;
both packaged helpers are executable and their usage commands run. Signing,
notarization, Intel builds, and native microphone/loopback capture were not tested.

Fetched origin/main: it remains an ancestor of this branch, with no divergent
main commits. macOS blockers below are resolved. Windows import remains deferred
by explicit user instruction; do not describe this as Windows-ready. Preserve
unrelated main-is-god scratch edits and LOG.md outside these commits.

## Original review (superseded for macOS)


Compared local main e36c84d9e57f149820534ae42c44bb27611896c0 with HEAD
0eafd75 and all working-tree/untracked transcription changes. Local main is an
ancestor; branch has 13 commits beyond it. Remote freshness was not checked.

Verdict: not ready to merge the whole branch. No background build, benchmark,
or dev-server processes were running when inspected. Review workers finished.
Transcription changes remain uncommitted, alongside unrelated existing work.

## Standards review

- P1: unconditional std::os::unix import in platform/cli_link.rs:7 prevents
  Windows compilation. Reproduced using rustc --crate-type lib --edition 2021
  --target x86_64-pc-windows-msvc src-tauri/src/platform/cli_link.rs, error E0433.
- P1: prepare-cli-sidecar.sh builds the Cargo package before staging externalBin;
  tauri_build validates that missing externalBin during this very build. Clean
  Windows CI cannot bootstrap. Verified against locally installed tauri-build.
- P2: platform/mod.rs expects triple-suffixed installed CLI, but Tauri removes
  the suffix when staging. Development fallback can mask the installation bug.

## Spec review

Same three blocker categories, with an additional cause in macOS packaging:
tauri.macos.conf.json overrides externalBin to
only set-default-output, omitting the CLI entirely. No concrete streaming
lost-audio/speaker-alignment blocker found by either review.

## Checks and remaining verification

- Latest implementation: 369 Rust tests passed, 5 hardware tests ignored.
- Final review: cargo clippy -- -D warnings passes; npm run check reports zero
  errors/warnings; npm test passes 125 tests in 23 files; git diff --check passes.
- cargo fmt --check fails across existing files including build.rs and new CLI
  code; CI has a formatting gate. Full output /tmp/merge-format.log.
- No full Windows application build or packaged-app launch performed. Targeted
  compile already confirms a Windows blocker.
- Native microphone/loopback smoke and broader natural speech accuracy remain
  unverified. System audio remains post-Stop; slow Note saving is separate.
- Context-chunking tickets 03–05 remain intentionally staged, not background jobs.

No code was changed during this review. Fix packaging/cross-platform blockers,
resolve the formatting gate, smoke-test capture, then commit the intended files
before merging. Preserve unrelated user changes in the working tree.

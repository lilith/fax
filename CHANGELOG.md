# Changelog

All notable changes in this fork of [pdf-rs/fax](https://github.com/pdf-rs/fax)
are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

Entries cover changes made in the fork; upstream releases are documented in the
upstream repository.

## [Unreleased]

### Changed

- `fax_derive` now builds on `syn` 3.0.4 (was `2.0`), with `quote` 1.0 → 1.0.47
  and `proc-macro2` 1.0 → 1.0.107 written out in full. `syn` 3.0.4 declares
  `rust-version = 1.71`, which is exactly this crate's declared MSRV, so the
  floor is unaffected. **Verified byte-identical:** `bitmaps!` generates the G3/G4
  bitstream lookup tables, so the whole expanded crate was captured before and
  after — `cargo expand --lib` is unchanged at SHA-256
  `7b561eeef0bb120c465786e1344e7a6ce73f7efce6da4d41bd4799bc79caaf17`. The decode
  corpus in `tests/check.rs` (16 files: 6 `.fax`, 9 reference `.pbm`, 1 `.tiff`,
  plus 72 error-path inputs) still round-trips, and the suite is unchanged at
  4 suites / 7 passed / 0 failed.
- `libfuzzer-sys` 0.4.9 → 0.4.13 in the fuzz package.
- After the bump, `syn` 2 survives only through the `tiff` **dev**-dependency
  (`tiff` → `half` → `zerocopy` → `zerocopy-derive`); the published dependency
  graph is `syn` 3 only.

### Added

- CI: a `Fuzz` workflow that compile-gates the `fuzz/` package on every push and
  PR (`cargo check --all-targets` on stable, Linux only). `fuzz/` declares its
  own `[workspace] members = ["."]` so `cargo fuzz` can resolve it standalone,
  which also makes it invisible to every root-level cargo invocation — ci.yml's
  `cargo test --lib`, `cargo clippy --all-targets` and `cargo check` all run
  against the root package, so the two targets added in `7c5f474` were compiled
  by nothing and could rot against a `fax` API change with CI still green. The
  gate uses `cargo check` rather than `cargo fuzz build` because the rot it
  catches is plain type or resolution error, and `cargo fuzz` would need nightly
  plus per-target sanitizer codegen to find the same thing. Both targets did
  still compile when the gate was added. Verified with `cargo metadata` that the
  fuzz graph resolves from crates.io plus in-repo path deps alone, so the job
  needs no sibling checkouts.
- `.workongoing` is gitignored — it is a local agent-coordination marker and
  must never be committed.

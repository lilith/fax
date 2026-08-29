# Changelog

All notable changes in this fork of [pdf-rs/fax](https://github.com/pdf-rs/fax)
are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

Entries cover changes made in the fork; upstream releases are documented in the
upstream repository.

## [Unreleased]

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

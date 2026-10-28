# Release Checklist

* Update this crate's dependencies if needed. From the repository root, install
  [cargo-edit](https://crates.io/crates/cargo-edit) and run `cargo upgrade -p slog-redis`.
* Update `Cargo.toml` with the new version number.
* Search the repository for the old version number and update those occurrences
  (this crate's README, CHANGELOG, and the `slog-redis` pin in the unFTP
  `Cargo.toml`).
* From this directory run `make pr-prep`, ensuring everything is green.
* Prepare release notes for the GitHub release / crates.io (see `CHANGELOG.md`).
* Make a new commit:
  > Release slog-redis version x.y.x
* Make a pull request for this but don't merge.
* Wait till CI is green, then from this directory run `make publish`.
* Merge the PR via the command line by merging master into the branch and pushing it.
* Create a git tag using format `slog-redis-v{version}` e.g.
  > slog-redis-v0.2.0
  A GitHub *Release* from that tag is fine: the unFTP workflow only publishes
  binaries and Docker images for tags that start with `v` (e.g. `v0.15.2`).
  Tag-only is enough to mark the crate release.

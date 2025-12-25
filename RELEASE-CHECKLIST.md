# Release Checklist

* Update the Rust version in the Makefile and the Github actions file
* Update the Rust version in packaging/docker/*.ci
* Update the alpine version in the packaging/docker/alpine.Dockerfile.ci image
* Update minor versions dependencies. Install [cargo-edit](https://crates.io/crates/cargo-edit) and run `cargo upgrade`.
  Cargo-edit also covers all the crates in the workspace
* Update Cargo.toml with the new version number (also check for libunftp version references)
* Search for the old version number to find references to it in documentation and update those occurrences.
* Run `make pr-prep`, ensuring everything is green
* Prepare release notes for the Github release page
* Make a new commit indicating the crate name and version number e.g.
  > Release unftp version x.y.x

  or

  > Release slog-redis version x.y.x
* Make a pull request for this but don't merge.
* Wait till MR pipelines are OK then run `make publish`
* Merge the MR via the command line by merging master into the branch and pushing it.
* Create the release in Github using tag format \[{component}-\]{version} e.g.
  > v0.15.2
  or
  > slog-redis-v0.1.2
* Wait for the Github Actions pipeline to finish. You should see all artifacts in the release page.
* Build and push the docker containers
* Check if any documentation in the docs directory needs changes.
* Publish the docs site unftp.rs by running `make site`.
* Notify the Telegram channel.

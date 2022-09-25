# Release Checklist

* Update minor versions dependencies
* Update Cargo.toml with the new version number
* Search for the old version number to find references to it in documentation and update those occurrences.
* Run `make pr-prep`, ensuring everything is green
* Prepare release notes for the Github release page
* Make a new commit (don't push) indicating the crate name and version number e.g.    
    > Release unftp version x.y.x

    or

    > Release slog-redis version x.y.x
* Run `make publish`
* Push to Github
* Create the release in Github using tag format \[{component}-\]{version} e.g.
  > v0.12.12
  or
  > slog-redis-v0.1.2
* Create the artifacts: `make release-artifacts` and add to Github
* Build and push the docker containers
* Publish the docs site unftp.rs by running `make site`
* Notify the Telegram channel.

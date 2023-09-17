# Release Checklist

* Update the Rust version in the Makefile and the Github actions file
* Update minor versions dependencies
* Update Cargo.toml with the new version number
* Search for the old version number to find references to it in documentation and update those occurrences.
* Run `make pr-prep`, ensuring everything is green
* Prepare release notes for the Github release page
* Make a new commit indicating the crate name and version number e.g.    
    > Release unftp version x.y.x

    or

    > Release slog-redis version x.y.x
* Make a pull request for this but don't merge.
* Wait till MR pipelins are OK then run `make publish`
* Merge the MR via the command line by merging marster into the branch and pushing it.
* Create the release in Github using tag format \[{component}-\]{version} e.g.
  > v0.14.4
  or
  > slog-redis-v0.1.2
* Wait for the Github Actions pipeline to finish. You should see all artifacts in the release page.
* Build and push the docker containers
* Publish the docs site unftp.rs by running `make site`
* Notify the Telegram channel.

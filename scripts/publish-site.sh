#!/bin/sh
# Build the Doctave site and push its generated files to the netlify branch.
# Run from an up-to-date master with: make site
# Requires Doctave and permission to push to origin/netlify. Netlify deploys
# that branch to unftp.rs; this script does not call the Netlify API.
set -eu

repo_root=$(CDPATH='' cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

if [ "$(git branch --show-current)" != master ]; then
    echo "Error: publish the site from master after merging documentation changes" >&2
    exit 1
fi
if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Error: commit or discard tracked changes before publishing" >&2
    exit 1
fi

site_dir="$repo_root/site"
if [ -e "$site_dir" ] || [ -L "$site_dir" ]; then
    echo "Error: site directory already exists; move it before publishing" >&2
    exit 1
fi

# Publish only the current master. Fetch first so a stale source checkout or
# missing deployment branch fails before Doctave creates any files.
git fetch origin master
if [ "$(git rev-parse HEAD)" != "$(git rev-parse FETCH_HEAD)" ]; then
    echo "Error: update master from origin before publishing" >&2
    exit 1
fi
git fetch origin netlify
doctave build --release
if [ ! -d "$site_dir" ]; then
    echo "Error: site directory not found after build" >&2
    exit 1
fi

# Keep the source checkout intact. The temporary worktree starts at the
# fetched deployment commit, so a concurrent remote update rejects our push.
site_worktree=$(mktemp -d)
cleanup() {
    status=$?
    trap - 0
    if ! git worktree remove --force "$site_worktree" >/dev/null 2>&1; then
        rmdir "$site_worktree" 2>/dev/null ||
            echo "Warning: remove temporary worktree $site_worktree manually" >&2
    fi
    exit "$status"
}
trap cleanup 0

git worktree add --detach "$site_worktree" FETCH_HEAD
git -C "$site_worktree" rm -r -q -- .
cp -R "$site_dir/." "$site_worktree/"
git -C "$site_worktree" add -A

if git -C "$site_worktree" diff --cached --quiet; then
    echo "Documentation site is already up to date"
else
    git -C "$site_worktree" commit -m "Update documentation site"
    git -C "$site_worktree" push origin HEAD:refs/heads/netlify
fi

# Retain the generated site on failure for inspection or a retry.
rm -r "$site_dir"

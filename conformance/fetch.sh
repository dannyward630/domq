#!/bin/sh
# Fetch the HTML tree-construction tests (the html5lib .dat files, now
# maintained in web-platform-tests) at the commit pinned in WPT_COMMIT, into
# conformance/wpt/. Only that directory is downloaded (a sparse, shallow
# fetch), not the whole WPT repository.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
commit=$(tr -d '[:space:]' < "$here/WPT_COMMIT")
dest="$here/wpt"
if [ -f "$dest/.commit" ] && [ "$(cat "$dest/.commit")" = "$commit" ]; then
    echo "conformance/wpt is already at $commit"
    exit 0
fi
rm -rf "$dest"
git init -q "$dest"
cd "$dest"
git remote add origin https://github.com/web-platform-tests/wpt.git
git sparse-checkout set --no-cone /html/syntax/parsing/resources/*.dat /LICENSE.md
git fetch -q --depth 1 --filter=blob:none origin "$commit"
git checkout -q FETCH_HEAD
echo "$commit" > .commit
echo "fetched $(ls html/syntax/parsing/resources/*.dat | wc -l | tr -d ' ') .dat files at $commit"

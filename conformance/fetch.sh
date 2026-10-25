#!/bin/sh
# Fetch the test data the conformance tools run on, each at a pinned commit,
# with sparse, shallow fetches (only the directories needed):
#
# - conformance/wpt/: the HTML tree-construction tests (the html5lib .dat
#   files, maintained in web-platform-tests), at WPT_COMMIT;
# - conformance/readability/: Mozilla Readability's labelled test pages
#   (source.html, expected.html, expected-metadata.json), at
#   READABILITY_COMMIT.
set -eu
here=$(cd "$(dirname "$0")" && pwd)

# fetch DEST REPO COMMIT_FILE PATTERN...
fetch() {
    dest="$here/$1"; repo=$2; commit=$(tr -d '[:space:]' < "$here/$3"); shift 3
    if [ -f "$dest/.commit" ] && [ "$(cat "$dest/.commit")" = "$commit" ]; then
        echo "conformance/$(basename "$dest") is already at $commit"
        return
    fi
    rm -rf "$dest"
    git init -q "$dest"
    (
        cd "$dest"
        git remote add origin "$repo"
        git sparse-checkout set --no-cone "$@"
        git fetch -q --depth 1 --filter=blob:none origin "$commit"
        git checkout -q FETCH_HEAD
        echo "$commit" > .commit
    )
    echo "fetched conformance/$(basename "$dest") at $commit"
}

fetch wpt https://github.com/web-platform-tests/wpt.git WPT_COMMIT \
    '/html/syntax/parsing/resources/*.dat' /LICENSE.md
fetch readability https://github.com/mozilla/readability.git READABILITY_COMMIT \
    '/test/test-pages/' /LICENSE.md

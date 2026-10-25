#!/bin/sh
# Fetch the test data the conformance tools run on, each at a pinned commit,
# with sparse, shallow fetches (only the directories needed):
#
# - conformance/wpt/: the HTML tree-construction tests (the html5lib .dat
#   files, maintained in web-platform-tests), at WPT_COMMIT;
# - conformance/readability/: Mozilla Readability's labelled test pages
#   (source.html, expected.html, expected-metadata.json), at
#   READABILITY_COMMIT;
# - conformance/aeb/: the Zyte / ScrapingHub article extraction benchmark
#   (181 gzipped pages and ground-truth.json, MIT), at AEB_COMMIT;
# - conformance/wcxb/: the Web Content eXtraction Benchmark (2,008 gzipped
#   pages in 7 page types, CC-BY-4.0), at WCXB_COMMIT.
#
# `sh fetch.sh wpt readability` fetches only those; no arguments fetches all.
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

want() {
    [ $# -eq 0 ] && return 0
    for w in $WANT; do [ "$w" = "$1" ] && return 0; done
    return 1
}
WANT="$*"

if [ -z "$WANT" ] || want wpt; then
    fetch wpt https://github.com/web-platform-tests/wpt.git WPT_COMMIT \
        '/html/syntax/parsing/resources/*.dat' /LICENSE.md
fi
if [ -z "$WANT" ] || want readability; then
    fetch readability https://github.com/mozilla/readability.git READABILITY_COMMIT \
        '/test/test-pages/' /LICENSE.md
fi
if [ -z "$WANT" ] || want aeb; then
    fetch aeb https://github.com/scrapinghub/article-extraction-benchmark.git AEB_COMMIT \
        '/html/' /ground-truth.json /LICENSE /evaluate.py
fi
if [ -z "$WANT" ] || want wcxb; then
    fetch wcxb https://github.com/Murrough-Foley/web-content-extraction-benchmark.git WCXB_COMMIT \
        '/dev/' '/test/' /LICENSE /evaluate.py /metadata.json
fi

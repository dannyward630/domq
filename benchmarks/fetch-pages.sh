#!/bin/sh
# Fetch the five real pages astral-html benchmarks on (iniconfig's docs, a
# Bootstrap dashboard, a Rust Book chapter, PEP 8 and the PEP index, with
# their licences) at the commit its published results come from, into
# benchmarks/pages/. Used by `cargo bench --bench real_pages`.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
commit=7ea072add99590685672de232f2a3637baceeba2
dest="$here/pages"
if [ -f "$dest/.commit" ] && [ "$(cat "$dest/.commit")" = "$commit" ]; then
    echo "benchmarks/pages is already at $commit"
    exit 0
fi
rm -rf "$dest"
git init -q "$dest"
cd "$dest"
git remote add origin https://github.com/astral-sh/astral-html.git
git sparse-checkout set --no-cone /benchmarks/fixtures/
git fetch -q --depth 1 --filter=blob:none origin "$commit"
git checkout -q FETCH_HEAD
echo "$commit" > .commit
echo "fetched benchmarks/pages at $commit"

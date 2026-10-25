#!/bin/sh
# Regenerate book/src/cli.md from the CLI's own help text. A test in
# cli/tests/cli.rs fails when the page and `domq --help` disagree.
set -eu
cd "$(dirname "$0")/.."
cargo build -q -p domq-cli
domq=./target/debug/domq
{
    sed -n '1,/^```text$/p' book/src/cli.md | sed '$d'
    echo '```text'
    "$domq" --help
    echo '```'
    echo
    echo '## `domq lint`'
    echo
    echo 'See also [Lint HTML in CI](cookbook/lint.md).'
    echo
    echo '```text'
    "$domq" lint --help
    echo '```'
    echo
    echo '## `domq mcp`'
    echo
    echo 'See also [AI agents (MCP)](mcp.md).'
    echo
    echo '```text'
    "$domq" mcp --help
    echo '```'
} > book/src/cli.md.new
mv book/src/cli.md.new book/src/cli.md
echo "wrote book/src/cli.md"

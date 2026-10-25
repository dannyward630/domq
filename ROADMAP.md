# Roadmap

This is a direction, not a promise: order and scope will move with what people
actually report and ask for. It is organised around **the people who reach for
a tiny HTML parser** and the job each of them is trying to finish, not around
parser internals. Issues and PRs that fit the principles below are very welcome.

**Where we are:** 0.4.0 shipped on 2026-10-22. The project is `domq`
everywhere now, and declarative extraction (`#[derive(FromHtml)]` in Rust,
dataclasses in Python) sits on top of what 0.3.0 brought: selectors, readable
text and Markdown, editing, source spans, streaming events, reader mode, RAG
chunking, data helpers, a CLI with a linter, and packages for crates.io, npm,
PyPI and Homebrew. `domq mcp` is merged and ships in the next release. Most of
0.5 is built and in review. See [what works where](#what-works-where),
[what shipped](#shipped) and the [changelog](./CHANGELOG.md).

## Who it's for

1. **Rust developers pulling data out of HTML** `rust` — scrapers, crawlers,
   feed readers, link checkers, test assertions on rendered pages. They want
   the answer in one line, fast compiles, and no 40-crate tree for a two-line
   job.
2. **Shell users** `shell` — `curl … | domq 'a.title' -a href`. `htmlq` (last
   release 2022) and `pup` (2016) are unmaintained; `domq` is a maintained
   replacement for both, flag-compatible with `htmlq`.
3. **LLM, RAG and agent pipelines** `ai` — turn a page into clean Markdown or
   chunks for a prompt or an index, at the edge, on a server or as an agent's
   MCP tool, without a headless browser.
4. **JavaScript on the edge** `js` — Workers, Deno, Bun, the browser: no DOM
   needed, and a ~41 KB gzipped wasm module.
5. **Python scraping and data code** `py` — where most scraping and RAG code
   lives; a fast, never-panicking parser with selectors and Markdown.
6. **People who ship HTML** `ship` — docs sites, static-site generators, email
   templates: `domq lint` in CI catches missing alt text, empty links,
   duplicate ids and broken markup with `file:line:col`.

Every item below should make at least one of these people's day better, and is
tagged with who it's for. If it doesn't help anyone, it probably doesn't
belong.

## What works where

What each surface can do today, and the release that's planned to fill each
gap (— means no plan yet).

| | Rust | CLI | MCP | JavaScript | Python |
|---|---|---|---|---|---|
| CSS selectors | ✅ | ✅ | ✅ | ✅ | ✅ |
| readable text, Markdown | ✅ | ✅ `-t` `-m` | ✅ | ✅ | ✅ |
| reader mode (main content) | ✅ | ✅ `--main` | ✅ | 0.6 | 0.6 |
| RAG chunks | ✅ | — | ✅ | 0.6 | 0.6 |
| metadata (OpenGraph, JSON-LD, …) | ✅ | ✅ `--meta` | ✅ | 0.6 | 0.6 |
| links and resources | ✅ | ✅ `--links` | ✅ | 0.6 | 0.6 |
| tables, CSV | ✅ | ✅ `--csv` | ✅ | 0.6 | 0.6 |
| declarative extraction | ✅ derive | ✅ `-e` | ✅ | — | ✅ dataclasses |
| encoding detection from bytes | — | ✅ | — | — | 0.6 |
| editing | ✅ | `-r` only | — | 0.8 | 0.8 |
| streaming events | ✅ | 0.8 `--stream` | — | 0.8 | 0.8 |
| accessibility and markup lint | — | ✅ | ✅ | — | — |
| parse errors with positions | ✅ | via lint | via lint | ✅ | ✅ |

## Principles

Every addition is weighed against what defines the project:

1. **Tiny** — small code, especially in WASM.
2. **Zero-dependency by default** — new dependencies must be optional, behind a
   feature or in a separate crate.
3. **Forgiving** — never panic, never hang; recover from real-world mess.
4. **Honest** — no over-claiming. Numbers in the README are reproducible, and
   what we don't do is written down.

If a feature can't keep all four, it goes behind a feature flag, into a
companion crate, or onto the non-goals list.

## Budgets

Numbers we hold ourselves to (✅ = enforced in CI):

| budget | limit | today |
|---|---|---|
| runtime dependencies, default build | **0** ✅ | 0 |
| gzipped wasm, all bindings | **< 45 KiB** ✅ | ~40.8 KB (~42.3 KB with the 0.5 parser work) |
| panics or hangs on any input (HTML, selectors, edits) | **none** ✅ proptest + fuzz | none known |
| stack use on deeply nested input | **bounded** ✅ 100k-deep tests | bounded |
| super-linear time on adversarial input | **none** — every fix gets a timing test | none known |
| clean debug build of the library | **< 1 s** (M-series) | 0.55 s |
| allocations per parse | **O(1)**, not O(nodes) | ~20 |
| MSRV | **Rust 1.88** ✅ (raised only in minor releases, to versions ≥ ~6 months old) | 1.88 |

The wasm budget leaves little room for bringing reader mode, chunks and the
data helpers to JavaScript (0.6). Rather than raise it, those may ship as a
second, optional wasm module, so the core stays small for people who only
select and convert.

## Non-goals

Deliberately out of scope. Reach for the right tool instead:

- **Full WHATWG tree construction** (adoption agency, automatic
  `<html>`/`<head>`/`<body>`, foster parenting). For browser-identical trees,
  use [`html5ever`](https://crates.io/crates/html5ever). We *measure* how far
  from the spec we are (see 0.5) rather than pretend.
- **Security sanitization.** A forgiving, non-spec parser disagrees with
  browsers on edge cases, which is exactly what mutation-XSS exploits. Use
  [`ammonia`](https://crates.io/crates/ammonia).
- **Rendering.** No layout, no CSS cascade, no `:hover`, no JavaScript. If a
  page builds its content with script, you need a browser.
- **Networking.** Neither the library, the CLI nor the MCP server fetches
  URLs; pipe from `curl`, or pair `domq mcp` with a fetch tool. The MCP server
  reads only the HTML it is given and, once `--allow-read` lands (0.4.x),
  files under a directory you name.
- **Speed at any cost.** We stay among the fastest owned-tree parsers, but we
  won't trade safety, correctness or size for a benchmark. If raw speed over a
  borrowed tree is your only metric, use [`tl`](https://crates.io/crates/tl).

---

## How releases are ordered

**By how many people an item unblocks, then by size.** Releases are themed
around one group of users each, so each release finishes a job for someone
instead of being half of several. Small items that don't need a release of
their own live in the [anytime lane](#anytime--small-and-welcome); they land
in whichever release is next when someone picks them up, and most are good
first issues. Every item is tagged with its size and with who it's for:

- **S**: small, a contained change in one area.
- **M**: medium, touches several areas or a binding.
- **L**: large, new infrastructure, a new crate, or a research-style project.

## Ongoing — Launch feedback

The first months after the announcement decide whether this becomes a tool
people rely on.

- [ ] **S** Triage every report within 48 hours; small HTML repros become corpus
      fixtures and regression tests.
- [ ] **S** Patch releases for any panic, hang, wrong match or wrong output, in
      lock-step across crates.io, npm, PyPI and Homebrew.
- [ ] **S** Turn recurring questions into README cookbook entries and doc
      examples.
- [ ] **S** Keep 5–10 `good first issue`s labelled from the
      [anytime lane](#anytime--small-and-welcome).

## 0.4.x — "Agents that don't pay for markup"

`domq mcp` saves an agent from reading raw markup, but only if the agent
doesn't have to *write* that markup first. Today every tool takes the page as
an `html` argument, so the agent re-types the whole page as output tokens:
tens of thousands for an ordinary article, which is slower and costlier than
reading the HTML. This release makes the MCP server cheap to use on real pages.

- [x] **L** `ai` `domq mcp`: an MCP server over stdio exposing select, text,
      Markdown, chunks, links, metadata, tables, extract and lint on HTML the
      agent provides (still no fetching). Speaks MCP 2026-07-28 and the
      handshake-based revisions back to 2024-11-05. *Merged; ships in the
      next release.*
- [ ] **M** `ai` `domq mcp --allow-read DIR`: every tool accepts a `path`
      instead of `html`, restricted to files under DIR (no `..` or symlink
      escapes), off by default. Documented recipe: a fetch tool that saves the
      page to disk, then domq on the file.
- [ ] **S** `ai` Bounded responses on every tool: `max_chars` and a cursor for
      `chunks`, `select`, `links` and `tables`, with `truncated` reported the
      same way everywhere, so one big page can't flood the context window.
- [ ] **S** `ai` `domq mcp` in the official MCP Registry (a `server.json`
      published from the release workflow) and the MCP client directories.
- [ ] **M** `ai` `shell` Install with tools people already have: the native
      `domq` binary in a PyPI package and in npm packages with per-platform
      binaries (the ruff / biome model; names to be confirmed), so
      `uvx … mcp` and `npx -y … mcp` work in MCP client configs and shell users
      get a one-line install on every OS.

## 0.5 — "Trust, measured"

Earn trust with published numbers, not adjectives, including against the tools
people would otherwise pick. Much of this is built on the `feat/conformance`,
`feat/spec-recovery`, `feat/reader-benchmark` and `docs/book` branches and is
waiting to merge.

- [ ] **L** Conformance report: run the HTML tree-construction suite (the
      html5lib tests, now maintained in web-platform-tests), publish pass rates
      per area with the by-design gaps explained, and track them in CI.
      *Built, in review: 1,959 tests, 0 panics, 55.5% same structure.*
- [ ] **L** Spec-aligned recovery where it's cheap, chosen by what the
      conformance report says matters, within the size budget.
      *Built, in review: scoped implicit closes (`<li><a>One<li>`,
      `<td><b>x<td>`, `<p><b>x<div>`), character references without `;`,
      browser comment recovery, raw-text elements and `<plaintext>`; same
      structure as a browser 55.5% → 61.8% for +1.5 KB gzipped wasm.*
- [ ] **L** Reopen formatting elements (a bounded "adoption agency lite"):
      `<p><b>x<p>y` keeps `y` bold, `<b><i>x</b>y</i>` keeps `y` italic. The
      biggest remaining area in the conformance report.
- [ ] **L** `ai` Reader-mode benchmark: score `main_content()` and
      `metadata()` against Mozilla Readability's labelled test pages, publish
      precision and recall, and tune the heuristic with it.
      *Built, in review: 91.9% → 95.2% F1.*
- [ ] **M** `ai` Metadata against the same pages: titles without the trailing
      site name, bylines from the page text ("By Jane Doe"), more published
      date sources.
- [ ] **M** `ai` Reader mode and Markdown next to what RAG pipelines use today
      — trafilatura, Readability.js, markdownify, turndown — on the same
      labelled pages, with the numbers published either way.
- [ ] **M** `py` `js` Benchmarks where people choose: Python against
      BeautifulSoup (html.parser and lxml), lxml and selectolax; JavaScript
      against cheerio, parse5, node-html-parser and linkedom. Speed and size,
      reproducible like the Rust benchmarks.
- [ ] **S** `ai` An output-stability policy for `to_text`, `to_markdown` and
      chunks: their output changes only in minor releases, under an "Output
      changes" heading in the changelog, so re-embedding an index is planned
      rather than a surprise.
- [ ] **L** A docs site next to the playground: cookbook, CLI reference, and
      migration guides from htmlq, pup, scraper, BeautifulSoup and cheerio.
      *Built, in review (pup still to write).*
- [ ] **M** Apply to OSS-Fuzz once eligible, and run ClusterFuzzLite on every
      PR and nightly in the meantime; crashers become regression tests.
- [ ] **S** More budgets: CLI binary size, wheel size, and a regression
      threshold on the weekly benchmark instead of a report only.

## 0.6 — "Where people are: Python, JavaScript and RAG"

Reader mode, chunks and the data helpers are what personas 3–5 come for, and
today they exist only in Rust, the CLI and MCP. Bring them to the bindings
before anything else.

- [ ] **M** `py` Python: `main_content()`, `metadata()`, `links()` /
      `resources()`, `to_markdown_chunks()`, `table_rows()` and CSV,
      `resolve_url`, and reusable `Selector` objects.
- [ ] **S** `py` `domq.parse(bytes)` with the CLI's encoding detection (BOM,
      `<meta charset>`, UTF-8, Windows-1252): `requests`' `r.content` and
      Scrapy's `response.body` are bytes.
- [ ] **M** `js` JavaScript: `mainContent()`, `metadata()`, `links()`,
      `toMarkdownChunks()` and `tables()`, inside the size budget — as a
      second, optional wasm module if the core can't hold them.
- [ ] **S** `ai` Token-aware chunking: `to_markdown_chunks_by(max, |s| tokens(s))`
      with your own tokenizer, optional overlap, and stable chunk ids for
      incremental re-indexing; in every binding.
- [ ] **S** `ai` Output options for LLMs: links as numbered footnotes or
      stripped, image alt text inline or dropped, tables kept as HTML when
      spans don't fit Markdown; for `to_text` and `to_markdown`.
- [ ] **M** `py` `ai` Integrations: a LangChain document transformer and a
      LlamaIndex reader (in a companion package, or as documented recipes),
      and tables into pandas.
- [ ] **S** `py` More wheels: musllinux x86_64 / aarch64 (Alpine images),
      Windows arm64, free-threaded CPython 3.14t; and conda-forge.
- [ ] **S** `js` Publish to JSR for Deno; Cloudflare Workers and Deno
      examples, and when to use domq rather than Workers' `HTMLRewriter`.
- [ ] **M** Playground "copy as Rust / JS / Python / CLI" for the current
      selector, so it doubles as a code generator.
- [ ] **M** More package managers: Scoop / winget (Windows), Nix, AUR, and a
      tiny Docker image.

## 0.7 — "Lint for teams"

Everything a team needs to put `domq lint` in CI and keep it there, in one
release.

- [ ] **S** `ship` `--allow` / `--deny` per rule, a summary line, and
      `--format sarif` for GitHub code scanning.
- [ ] **S** `ship` Inline suppressions (`<!-- domq-disable-next-line img-alt -->`)
      and a config file, so a team can adopt the linter on an existing site.
- [ ] **M** `ship` More rules people expect: `html-lang`, `missing-title`,
      `form-label`, `iframe-title`, nested interactive elements (an `<a>` in a
      `<button>`), and in-page anchors that point at no `id`.
- [ ] **M** `ship` `rust` Local link checking: relative `href` / `src` that
      point at files that don't exist, for static sites (no network).
- [ ] **M** `ship` A reusable GitHub Action for `domq lint` that annotates pull
      requests.
- [ ] **S** `ship` A pre-commit hook for `domq lint`.

## 0.8 — "Huge documents and full trees"

For inputs too big to hold as a tree, for comparing documents in tests, and
for the rest of the binding parity.

- [ ] **L** `rust` `shell` Streaming selectors: run a `Selector` over
      `domq::events` for multi-gigabyte dumps without building a tree; CLI
      `--stream`.
- [ ] **L** `rust` `shell` `domq diff a.html b.html`: a structural diff
      (node-level, whitespace-insensitive) for snapshot tests of rendered pages.
- [ ] **M** `py` Python: editing API and `events()`.
- [ ] **M** `js` JavaScript: editing on `Document` and `events()` in the wasm
      package, inside the size budget.
- [ ] **M** `rust` Optional `<tbody>`/`<colgroup>` insertion: a builder option
      for browser-identical table structure.
- [ ] **M** `rust` Context-aware fragments: `append_html` and
      `parse_fragment_in("tr", …)` parse in the parent's context (a `<td>`
      inside a `<tr>`).

## Anytime — small and welcome

Small, high-value polish that doesn't need a release of its own. Most items are
good first issues; each lands in whichever release is next.

- [ ] **S** `shell` Several inputs in one run (`-f` repeatable), with
      grep-style `FILE:` prefixes and a `file` field in `--jsonl`.
- [ ] **S** `shell` `--exit-status`: exit 1 when nothing matches, for
      `if domq --exit-status …; then`. The default stays htmlq-compatible.
- [ ] **S** `shell` Shell completions (bash, zsh, fish, PowerShell) and a man
      page, generated from `--help` and shipped in the release archives and
      the Homebrew formula.
- [ ] **S** `shell` A warning on stderr when `<meta charset>` names an encoding
      the CLI can't decode (Shift_JIS, GBK, …), instead of silent mojibake.
- [ ] **S** `rust` `Selection` helpers, chainable and allocation-light:
      `dom.select("a").attrs("href")`, `.texts()`, `.first()`, `.len()`.
- [ ] **S** `rust` Reusable parser: `Dom::parse_into(&mut self, html)` reuses
      the arenas, so a crawler parsing millions of pages stops allocating per
      page.
- [ ] **S** `rust` Cookbook: fetching with `reqwest` / `ureq`, and parsing
      pages in parallel with rayon.
- [ ] **S** A counting-allocator test, so the allocations-per-parse budget is
      enforced in CI.
- [ ] **S** Release automation: one command bumps every version in lock-step
      (crate, CLI, wasm, Python) and drafts the changelog; the release workflow
      updates the Homebrew tap itself (token scoped to the tap repo).
- [ ] **S** Supply chain: signed build provenance (GitHub artifact attestations)
      and an SBOM attached to every release.
- [ ] **M** Playground: reader-mode, chunks, metadata and lint tabs, within the
      wasm size budget.

## 1.0 — "Stable"

1.0 ships when all of these hold, not on a date:

- [ ] The public API is reviewed against the Rust API Guidelines, with
      `#[non_exhaustive]` where growth is expected and no known breaking
      changes pending.
- [x] A written MSRV policy (see [Budgets](#budgets)).
- [ ] Conformance and reader-mode numbers are published and tracked.
- [ ] Months of continuous fuzzing with no open crashers.
- [ ] Used by real projects (a "used by" list in the README).
- [ ] SemVer covers everything people script against: the crate API, CLI
      flags, output formats and exit codes, the MCP tools and their schemas,
      and the JS and Python APIs.
- [ ] The output-stability policy (0.5) has held for at least two minor
      releases.
- [ ] A deprecation policy: anything removed is deprecated, with a warning,
      for at least one minor release first.
- [ ] A written support policy for Python, Node and Deno versions.
- [ ] A security response target in [SECURITY.md](./SECURITY.md).

---

## Distribution

Being easy to *get* matters as much as features.

**Done:** crates.io (`domq`, `domq-cli`, `domq-derive`), `cargo binstall`,
release binaries (Linux x86_64/arm64, macOS, Windows) with checksums, WASI build,
Homebrew tap, npm (trusted publishing, provenance), PyPI (trusted publishing,
abi3 wheels), and the playground.

**Planned:** the MCP Registry and `uvx` / `npx`-launchable binaries (0.4.x);
the docs site (0.5); musllinux, Windows arm64 and free-threaded wheels,
conda-forge, JSR, Scoop / winget / Nix / AUR / Docker (0.6); release
automation and the Homebrew tap auto-update (anytime); homebrew-core once the
project meets its notability bar.

## Community

- [ ] `good first issue`s kept stocked; first response on issues within 48
      hours.
- [ ] A "used by" section and credit for every contributor in release notes.
- [ ] GitHub Discussions for questions and show-and-tell, so issues stay
      actionable.

## Exploring

Ideas we like but haven't committed to. Prototypes and use-cases welcome.

- **More encodings in the CLI** (Shift_JIS, GBK, EUC-KR, ISO-8859-x) through
  an optional `encoding_rs` feature, used in the release binaries.
- **WASI component (WIT)** so any component runtime can call the parser.
- **More bindings** (C ABI, Ruby, PHP, Go via wasm) if there's demand.
- **Language-server-lite** — lint diagnostics in editors, using source spans;
  a natural follow-up to lint for teams (0.7).
- **`domq fmt`** — an HTML formatter built on the pretty-printer and spans.
- **XPath subset** — only with strong demand; CSS covers most real queries.
- **Incremental re-parse** for editors.

## Considered and declined

- **Borrowed-tree mode** — measured in 0.3: string copying is 10–15% of parse
  time on dense markup and nothing on prose, and it would need a lifetime on
  `Dom` or a second API. Numbers in [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md).
- **Fetching inside `domq mcp`** — agents already have fetch tools, and
  fetching brings TLS, redirects, robots.txt and server-side request forgery
  into a project that has none of them. A fetch tool that saves to disk plus
  `--allow-read` gives the same savings. We'll revisit if that pairing proves
  impractical.
- **Fetching URLs**, **sanitization**, **full WHATWG** — see non-goals.

---

## Shipped

### 0.4.0 — 2026-10-22 · "Declarative scraping"

Details in the [changelog](./CHANGELOG.md#040--2026-10-22).

- [x] **One name everywhere:** the crate `domq`, the CLI `domq-cli` (binary
      `domq`), the npm package `domq-wasm`, the PyPI package and import
      `domq`, and `brew install iamsaquib8/tap/domq`.
- [x] `#[derive(FromHtml)]` in the optional companion crate `domq-derive`
      (the core stays zero-dependency): declarative extraction into structs
      with `select`, `attr`, `html`, `markdown`, `default`, `with`, `Option<T>`,
      `Vec<T>`, nested and generic structs, compile-time diagnostics, and
      errors that name the field path and selector that failed.
- [x] Python `domq.extract` / `extract_all`: the same extraction into
      dataclasses with `Annotated[T, Select(...)]`, with the same error paths
      and messages as Rust.
- [x] MSRV 1.88, Python 3.11+; `to_text()` / `to_markdown()` called *on* a
      hidden element return its content.

### 0.3.0 — 2026-10-21 · "One-line scraping"

Everything below shipped in 0.3.0 (what was planned as phases 0.3–0.7, plus
distribution and two rounds of review fixes). Details in the
[changelog](./CHANGELOG.md).

<details>
<summary>Everything in 0.3.0</summary>

**Selectors** (the `query` feature, now on by default)

- [x] `+` / `~` sibling combinators.
- [x] Attribute operators `^=` `$=` `*=` `~=` `|=` with the `i` / `s` flags;
      HTML's case-insensitive enumerated attributes (`type`, `method`, …).
- [x] Structural pseudo-classes: `:first-child`, `:last-child`, `:only-child`,
      `:nth-child(An+B [of S])`, `:nth-last-child`, `:first-of-type`,
      `:last-of-type`, `:only-of-type`, `:nth-of-type`, `:nth-last-of-type`,
      `:empty`, `:root`, `:scope`.
- [x] Logical pseudo-classes: `:not()`, `:is()`, `:where()`, `:has()` with
      relative selectors.
- [x] `:checked`, `:disabled`, `:enabled`, CSS escapes (`.md\:flex`), and the
      non-standard `:contains("text")`, clearly labelled.
- [x] `Selector::parse` / `FromStr` with positioned errors; `select` accepts
      `&str` or a compiled `&Selector`.
- [x] Scoped queries: `NodeRef::select`, `select_first`, `matches`, `closest`;
      `Dom::select_first`.
- [x] Linear-time matching: per-query memo for `:nth-*`, sibling and ancestor
      searches; bounded memo memory; scoped queries pay only for what they walk.

**Getting content out**

- [x] `to_text()`: readable text (blocks become lines, whitespace collapsed,
      `<head>`/`<script>`/`<style>`/`<title>` dropped).
- [x] `to_markdown()`: zero-dependency CommonMark + GFM (headings, emphasis,
      links, images, nested lists, `<li value>`/`<ol reversed>`, code blocks
      with languages, quotes, tables).
- [x] `inner_html()`, `outer_html()`, `next_element_sibling()`,
      `prev_element_sibling()`.
- [x] Windows-1252 remapping for numeric references `&#128;`–`&#159;`.

**Edit and point**

- [x] Editing: `remove`, `set_attr`, `remove_attr`, `set_text`, `rename`,
      `create_element` / `create_text`, `append_child`, `insert_before`,
      `wrap` / `unwrap`, `append_html` / `prepend_html` /
      `insert_html_before` / `insert_html_after`. No cycles; document order is
      kept after every edit; `is_detached()`.
- [x] Source spans: `NodeRef::source_range()` and `domq::line_col()`.
- [x] Top-level nodes linked as siblings (`li + li` works on fragments).
- [x] More WHATWG optional-tag rules: the full `<p>`-closing list, headings,
      unclosed `<head>`, ruby, `<hr>` in `<select>`, captions / `colgroup` / `col`.
- [x] `entities-full` feature: all 2,125 WHATWG named references (default: the
      178 most common).

**Fast and lean**

- [x] One attribute arena per document; 52-byte nodes; no per-tag allocations
      (~20 allocations per parse); unused arena capacity released after parsing.
- [x] Word-at-a-time byte scanning; optional `simd` feature (`memchr`).
- [x] Parsing 25–37% faster on markup and 69% on prose, ahead of `tl` on large
      documents.
- [x] `domq::events()`: streaming parse events with the same recovery as
      the tree builder.
- [x] Benchmarks against `tl`, `scraper` and `html5ever`, a memory report, and a
      weekly benchmark workflow.
- [x] Differential fuzzing against html5ever on the supported subset.
- [x] Borrowed-tree mode evaluated and declined (see above).

**Structure and data**

- [x] `Dom::main_content()`: deterministic reader-mode heuristic.
- [x] `to_markdown_chunks()`: heading-aware RAG chunks with heading paths.
- [x] `Dom::links()` / `resources()` / `base_url()`, honouring `<base href>`
      (but not inside `<template>`).
- [x] `Dom::metadata()`: title, description, canonical, lang, OpenGraph,
      Twitter cards, JSON-LD and feeds.
- [x] `NodeRef::table_rows()` with colspan/rowspan expanded; `domq::to_csv`.
- [x] `domq::resolve_url` / `resolve_srcset`: RFC 3986, HTML `srcset` rules,
      and browsers' backslash handling.

**CLI: `domq`** (htmlq-compatible: `-a -t -p -w -r -b -B -f -o`)

- [x] Output: `--markdown`, `--json`, `--jsonl`, `--csv`, `--count`, `--first`,
      pretty HTML; conflicting formats are a usage error.
- [x] `-e NAME=SELECTOR[@attr][[]]` structured extraction, scoped per match.
- [x] `--main` reader mode, `--links`, `--meta`, `--remove-nodes`, `--base`.
- [x] Encoding detection (BOM, `<meta charset>`, UTF-8, Windows-1252) and
      `--encoding`.
- [x] `domq lint`: alt text, empty links and buttons, duplicate ids, heading
      order and parse errors, as `file:line:col` or JSON, with CI exit codes.

**JavaScript, Python and the browser**

- [x] The npm package (`dom-tree-wasm` in 0.3.0, `domq-wasm` from 0.4.0): `parse`, `select`, `selectFirst`, `toText`,
      `toMarkdown`, `toHtml`, `errors`, and a parse-once `Document`; full
      TypeScript types; works in Node with no setup; ~41 KB gzipped.
- [x] Playground: <https://iamsaquib8.github.io/domq/>.
- [x] `domq` on PyPI (`import domq`): abi3 wheels for 5 platforms,
      typed stubs, the GIL released on large inputs.

**Distribution**

- [x] crates.io: `domq` and `domq-cli`.
- [x] Release binaries for Linux x86_64/arm64, macOS Intel/Apple Silicon and
      Windows, with `SHA256SUMS`; `cargo binstall domq-cli`.
- [x] WASI build (`domq.wasm`, runs under wasmtime).
- [x] Homebrew: `brew install iamsaquib8/tap/domq`.
- [x] npm and PyPI publishing from CI with trusted publishing (no stored
      tokens; npm with provenance).

**Quality and hardening**

- [x] `Dom::errors()` no longer flags valid HTML (a bare `&`, optional end
      tags); positions point at the problem.
- [x] Two independent reviews. All findings fixed: quadratic paths (tag-name
      hash collisions, scoped queries, `append_html`, chunk paths, nested tables,
      CLI `--links`/`-e`), markup injection through edited content (raw-text
      escaping, name validation), and URL/`<base>` edge cases.
- [x] Never-panic proptests for selectors, output, editing and source spans;
      fuzz targets for `parse`, `tokenize`, `select`, `markdown` and
      `differential`; 100k-deep nesting tests.
- [x] CI: tests (stable/beta), MSRV 1.85, `no_std`, clippy, fuzz smoke, wasm
      size gate, zero-dependency gate, Python, WASI, Pages, release.
- [x] Project: README redesign, `CODE_OF_CONDUCT.md`, `SECURITY.md`,
      `docs/ARCHITECTURE.md`, issue/PR templates, dependabot, launch kit.

</details>

### 0.2 — 2026-06

Never-panicking, error-tolerant parser; arena tree with O(1) traversal; a CSS
subset; zero-dependency JSON; `to_html()`; `no_std + alloc`; WASM bindings;
property tests, snapshot corpus, fuzzing and CI.

## Have an idea?

Open an issue describing the use case, ideally naming which of the six groups
above it helps. The bar is "does it keep the project tiny, zero-dependency,
forgiving and honest?" If yes, let's talk.

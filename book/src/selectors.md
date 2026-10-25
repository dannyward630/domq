# Selectors

domq implements the CSS selectors that matter for querying documents, in
every binding (Rust, CLI, JavaScript, Python, MCP). Anything it does not
support is an **error with a position**, never a selector that silently
matches nothing.

## Supported

| Kind | Syntax |
|---|---|
| Type, universal | `div`, `*` |
| Id, class | `#main`, `.card`, `.md\:flex` (CSS escapes) |
| Attribute | `[href]`, `[lang=en]`, `[class~=a]`, `[lang\|=en]`, `[href^=https]`, `[src$=".png"]`, `[title*=intro]`, with ` i` / ` s` flags |
| Lists | `h1, h2` |
| Combinators | descendant (`article p`), child (`ul > li`), next sibling (`h2 + p`), subsequent sibling (`h2 ~ p`) |
| Structure | `:first-child`, `:last-child`, `:only-child`, `:nth-child(An+B [of S])`, `:nth-last-child()`, `:first-of-type`, `:last-of-type`, `:only-of-type`, `:nth-of-type()`, `:nth-last-of-type()`, `:empty`, `:root`, `:scope` |
| Logic | `:not()`, `:is()`, `:where()` |
| Relational | `:has()`, including relative selectors: `:has(> img)`, `:has(+ p)` |
| Forms | `:checked`, `:disabled`, `:enabled` |
| Text (jQuery) | `:contains("text")`: a case-sensitive substring of the element's text content |

Tag, attribute and pseudo-class names are ASCII case-insensitive. Ids,
classes and attribute values are case-sensitive, except the values of HTML's
enumerated attributes (`type`, `method`, `dir`, `rel`, …), which compare
case-insensitively as in browsers unless the selector says ` s`.

Sibling combinators and positional pseudo-classes count elements only.
Top-level nodes are siblings of each other, so `li + li` works on a fragment
like `<li>a</li><li>b</li>`, and each top-level element is `:root`.

## Not supported

Pseudo-elements (`::before`), interactive and browser states (`:hover`,
`:focus`, `:visited`, `:target`), namespaces (`svg|rect`), and `:has()`
nested inside `:has()`.

```rust
# extern crate domq;
let err = domq::Selector::parse("a:hover").unwrap_err();
assert_eq!(err.position(), 1);
assert_eq!(err.message(), "unsupported pseudo-class ':hover'");
```

## One thing to know about the tree

domq does not insert the `<html>`, `<head>` and `<body>` elements a browser
adds to every page, nor `<tbody>` in tables. A selector written against a
browser's tree, like `html > body > div` or `table > tbody > tr`, needs those
elements in the source. Prefer selectors that do not depend on them
(`body > div` matches when the page has a `<body>` tag; `table tr` always
does).

## Performance

Matching is memoized per query, so `:has()` and long descendant chains stay
fast on large documents. Parse a selector
once with `Selector::parse` (or `"…".parse::<Selector>()`) to reuse it across
documents.

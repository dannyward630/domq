# Quick start

The same task four ways: from a product page, get the title, every product
name with its price, and the page as Markdown.

```html
<html><head><title>Gadgets</title></head><body>
  <nav><a href="/">Home</a></nav>
  <main>
    <h1>Gadgets</h1>
    <div class="product"><h2>Widget</h2><span class="price" data-value="9.5">$9.50</span></div>
    <div class="product"><h2>Gizmo</h2><span class="price" data-value="12">$12</span></div>
  </main>
</body></html>
```

## Rust

```rust
# extern crate domq;
let html = r#"<title>Gadgets</title><main><h1>Gadgets</h1>
  <div class="product"><h2>Widget</h2><span class="price" data-value="9.5">$9.50</span></div>
  <div class="product"><h2>Gizmo</h2><span class="price" data-value="12">$12</span></div></main>"#;

let dom = domq::parse(html);

let title = dom.select_first("title").map(|t| t.to_text());
assert_eq!(title.as_deref(), Some("Gadgets"));

for product in dom.select(".product") {
    let name = product.select_first("h2").map(|h| h.to_text()).unwrap_or_default();
    let price = product.select_first(".price").and_then(|p| p.attr("data-value"));
    println!("{name}: {price:?}");
}

println!("{}", dom.to_markdown());
```

`domq::parse` never fails: any input gives a tree, and what it repaired is
in `dom.errors()`. Selectors are parsed on every call when passed as `&str`;
to reuse one, parse it once with `domq::Selector::parse` and pass `&selector`.

## Command line

```sh
domq -t title < page.html
domq '.product' -e name=h2 -e price=.price@data-value < page.html
# [{"name":"Widget","price":"9.5"},{"name":"Gizmo","price":"12"}]
domq --main --markdown < page.html
```

## JavaScript

```js
import { select, selectFirst, toMarkdown } from "domq-wasm";

selectFirst(html, "title")?.text;            // "Gadgets"
select(html, ".product .price").map((m) => m.attrs["data-value"]); // ["9.5", "12"]
toMarkdown(html, "main");
```

Each function parses the HTML again. To run several queries, parse once with
`new Document(html)` and call the same methods on it.

## Python

```python
import domq

doc = domq.parse(html)
doc.select_first("title").text                      # 'Gadgets'
[(p.select_first("h2").text, p.select_first(".price").attr("data-value"))
 for p in doc.select(".product")]                   # [('Widget', '9.5'), ('Gizmo', '12')]
doc.to_markdown()
```

## Next

- Pull those fields straight into structs or dataclasses: [Extract structured
  data](cookbook/extracting.md).
- Feed pages to a model: [Pages to Markdown for LLMs and
  RAG](cookbook/reader-mode.md).

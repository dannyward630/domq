//! Real pages: `domq` against astral-html, `tl` and `scraper` on the five
//! documents astral-html publishes its numbers for, with its two workloads,
//! so the comparison runs on one machine instead of across READMEs.
//!
//! * `parse-document`: parse into each library's own document and drop it.
//! * `extract-links`: parse, collect an owned `{href, title, rel}` record for
//!   every `<a href>`, drop everything (astral-html's link workload).
//!
//! Run with `sh fetch-pages.sh && cargo bench --bench real_pages` from the
//! `benchmarks/` directory. Results are in `README.md`.

use std::hint::black_box;
use std::path::Path;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};

/// The pages, in astral-html's order.
const PAGES: &[(&str, &str)] = &[
    ("iniconfig", "iniconfig.html"),
    ("bootstrap-dashboard", "bootstrap-dashboard.html"),
    ("rust-book", "rust-book.html"),
    ("pep8", "pep8.html"),
    ("pep-index", "pep-index.html"),
];

#[derive(Debug, PartialEq, Eq)]
struct Link {
    href: String,
    title: Option<String>,
    rel: Option<String>,
}

fn domq_links(input: &str) -> Vec<Link> {
    let dom = domq::parse(input);
    dom.find_by_tag("a")
        .filter_map(|a| {
            Some(Link {
                href: a.attr("href")?.to_owned(),
                title: a.attr("title").map(str::to_owned),
                rel: a.attr("rel").map(str::to_owned),
            })
        })
        .collect()
}

/// The streaming API: links from start events, no tree.
fn domq_event_links(input: &str) -> Vec<Link> {
    domq::events(input)
        .filter_map(|(event, _)| {
            let domq::Event::Start { name, attrs } = event else {
                return None;
            };
            if name != "a" {
                return None;
            }
            let get = |key: &str| {
                attrs
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.to_string())
            };
            Some(Link {
                href: get("href")?,
                title: get("title"),
                rel: get("rel"),
            })
        })
        .collect()
}

fn astral_document_links(input: &str) -> Vec<Link> {
    let document = astral_html::Document::parse(input).expect("astral-html parses the page");
    document
        .elements()
        .filter(|element| element.is("a"))
        .filter_map(|element| {
            let value = |name| element.attribute(name).map(|v| v.value().to_owned());
            Some(Link {
                href: value("href")?,
                title: value("title"),
                rel: value("rel"),
            })
        })
        .collect()
}

fn astral_reader_links(input: &str) -> Vec<Link> {
    astral_html::Reader::new(input)
        .filter_map(|token| {
            let astral_html::Token::StartTag(tag) = token else {
                return None;
            };
            if tag.name != "a" {
                return None;
            }
            let (mut href, mut title, mut rel) = (None, None, None);
            for attribute in tag.attributes {
                let slot = match attribute.name.as_ref() {
                    "href" => &mut href,
                    "title" => &mut title,
                    "rel" => &mut rel,
                    _ => continue,
                };
                if slot.is_none() {
                    *slot = Some(attribute.value.into_owned());
                }
            }
            Some(Link {
                href: href?,
                title,
                rel,
            })
        })
        .collect()
}

fn tl_links(input: &str) -> Vec<Link> {
    let document = tl::parse(input, tl::ParserOptions::default()).expect("tl parses the page");
    document
        .nodes()
        .iter()
        .filter_map(tl::Node::as_tag)
        .filter(|element| element.name().as_bytes().eq_ignore_ascii_case(b"a"))
        .filter_map(|element| {
            let value = |name| {
                element
                    .attributes()
                    .get(name)
                    .map(|v| v.map_or_else(String::new, |v| v.as_utf8_str().into_owned()))
            };
            Some(Link {
                href: value("href")?,
                title: value("title"),
                rel: value("rel"),
            })
        })
        .collect()
}

/// scraper's own entry point (astral-html's harness turns scripting off
/// through html5ever directly; scraper 0.29 depends on a newer html5ever
/// than this crate's `rcdom` benchmark, so that route doesn't type-check
/// here). Scripting only changes how `<noscript>` is parsed.
fn scraper_document(input: &str) -> scraper::Html {
    scraper::Html::parse_document(input)
}

fn scraper_links(input: &str, selector: &scraper::Selector) -> Vec<Link> {
    scraper_document(input)
        .select(selector)
        .map(|element| Link {
            href: element.value().attr("href").unwrap_or("").to_owned(),
            title: element.value().attr("title").map(str::to_owned),
            rel: element.value().attr("rel").map(str::to_owned),
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("pages/benchmarks/fixtures");
    let selector = scraper::Selector::parse("a[href]").unwrap();
    for (name, file) in PAGES {
        let input = std::fs::read_to_string(dir.join(file))
            .unwrap_or_else(|e| panic!("{file}: {e} (run `sh fetch-pages.sh` first)"));
        let input = input.as_str();
        // Every parser must find the same links, or the comparison is unfair.
        let expected = domq_links(input).len();
        for (parser, found) in [
            ("domq/events", domq_event_links(input).len()),
            ("astral-html", astral_document_links(input).len()),
            ("astral-html/reader", astral_reader_links(input).len()),
            ("tl", tl_links(input).len()),
            ("scraper", scraper_links(input, &selector).len()),
        ] {
            if found != expected {
                eprintln!("note: {name}: {parser} finds {found} links, domq {expected}");
            }
        }

        let mut group = c.benchmark_group(format!("parse-document/{name}"));
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.warm_up_time(Duration::from_secs(1));
        group.measurement_time(Duration::from_secs(3));
        group.bench_function("domq", |b| {
            b.iter(|| drop(black_box(domq::parse(black_box(input)))))
        });
        group.bench_function("astral-html", |b| {
            b.iter(|| drop(black_box(astral_html::Document::parse(black_box(input)).unwrap())))
        });
        group.bench_function("tl", |b| {
            b.iter(|| {
                drop(black_box(
                    tl::parse(black_box(input), tl::ParserOptions::default()).unwrap(),
                ))
            })
        });
        group.bench_function("scraper", |b| {
            b.iter(|| drop(black_box(scraper_document(black_box(input)))))
        });
        group.finish();

        let mut group = c.benchmark_group(format!("extract-links/{name}"));
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.warm_up_time(Duration::from_secs(1));
        group.measurement_time(Duration::from_secs(3));
        group.bench_function("domq", |b| b.iter(|| drop(black_box(domq_links(black_box(input))))));
        group.bench_function("domq/events", |b| {
            b.iter(|| drop(black_box(domq_event_links(black_box(input)))))
        });
        group.bench_function("astral-html", |b| {
            b.iter(|| drop(black_box(astral_document_links(black_box(input)))))
        });
        group.bench_function("astral-html/reader", |b| {
            b.iter(|| drop(black_box(astral_reader_links(black_box(input)))))
        });
        group.bench_function("tl", |b| b.iter(|| drop(black_box(tl_links(black_box(input))))));
        group.bench_function("scraper", |b| {
            b.iter(|| drop(black_box(scraper_links(black_box(input), &selector))))
        });
        group.finish();
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);

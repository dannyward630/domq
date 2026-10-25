//! Score reader mode on human-labelled extraction benchmarks.
//!
//! `reader` (the other binary) measures *agreement* with Readability on
//! Readability's own test pages. This one measures *quality* against human
//! labels, on the corpora other extractors publish numbers for, scored the
//! way each benchmark scores:
//!
//! * **AEB**, the Zyte / ScrapingHub article extraction benchmark: 181
//!   article pages, MIT. Each page's article body is compared as bags of
//!   4-word shingles over `\w+` tokens (case kept), normalized per page;
//!   precision and recall are averaged over pages, F1 is computed from the
//!   two means, and a bootstrap (1,000 resamples, fixed seed) gives the
//!   spread. As in its `evaluate.py`; soft hyphens are removed first, as its
//!   runner does for every Rust extractor.
//! * **WCXB**, the Web Content eXtraction Benchmark: 2,008 pages in seven
//!   page types, CC-BY-4.0, in a development split (1,497) and a held-out
//!   test split (511). Each page scores a word-level F1 (bags of lower-cased
//!   `\w+` tokens), averaged over pages, plus the share of its "must
//!   include" and "must exclude" snippets found. As in its `evaluate.py`.
//!
//! The extraction is `Dom::main_content()` as text (`to_text()`), or the
//! whole page's text when it returns `None`, exactly what `domq --main -t`
//! prints. A whole-page row shows what reader mode adds. Tune only against
//! the AEB pages and WCXB's dev split; the test split is for reporting.
//!
//! ```text
//! sh conformance/fetch.sh aeb wcxb
//! cargo run --release --manifest-path conformance/Cargo.toml --bin quality
//! cargo run --release --manifest-path conformance/Cargo.toml --bin quality -- --write docs/reader-quality.md
//! cargo run --release --manifest-path conformance/Cargo.toml --bin quality -- --check docs/reader-quality.md
//! cargo run --release --manifest-path conformance/Cargo.toml --bin quality -- --predictions /tmp/out
//! ```
//!
//! `--predictions DIR` writes `aeb-domq.json` and `wcxb-{dev,test}-domq.json`
//! in the formats the benchmarks' own `evaluate.py` scripts read, to check
//! this reimplementation against them.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;
use std::{env, fs};

use flate2::read::GzDecoder;
use serde_json::Value;

fn main() -> ExitCode {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut write = None;
    let mut check = None;
    let mut predictions = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--write" => write = args.next().map(PathBuf::from),
            "--check" => check = args.next().map(PathBuf::from),
            "--predictions" => predictions = args.next().map(PathBuf::from),
            "-h" | "--help" => {
                println!("usage: quality [--write FILE | --check FILE | --predictions DIR]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("quality: unknown argument '{other}'");
                return ExitCode::from(2);
            }
        }
    }
    let commit = |name: &str| {
        fs::read_to_string(here.join(name))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let aeb = match load_aeb(&here.join("aeb")) {
        Ok(pages) => pages,
        Err(e) => return missing(&e),
    };
    let dev = match load_wcxb(&here.join("wcxb"), "dev") {
        Ok(pages) => pages,
        Err(e) => return missing(&e),
    };
    let test = match load_wcxb(&here.join("wcxb"), "test") {
        Ok(pages) => pages,
        Err(e) => return missing(&e),
    };

    let aeb_run = run(&aeb);
    let dev_run = run(&dev);
    let test_run = run(&test);

    if let Some(dir) = predictions {
        if let Err(e) = write_predictions(&dir, &aeb, &aeb_run, &dev, &dev_run, &test, &test_run) {
            eprintln!("quality: writing predictions: {e}");
            return ExitCode::from(2);
        }
        println!("wrote predictions to {}", dir.display());
        return ExitCode::SUCCESS;
    }

    let report = Report {
        aeb_commit: commit("AEB_COMMIT"),
        wcxb_commit: commit("WCXB_COMMIT"),
        aeb_main: score_aeb(&aeb, |i| &aeb_run.main[i]),
        aeb_whole: score_aeb(&aeb, |i| &aeb_run.whole[i]),
        aeb_fallbacks: aeb_run.fallbacks,
        dev: score_wcxb(&dev, &dev_run),
        test: score_wcxb(&test, &test_run),
        timing: format!(
            "Measured when this report was last written, on the machine that wrote it \
             (extraction only, release build, single thread): AEB {:.1} ms/page, WCXB dev \
             {:.1} ms/page, WCXB test {:.1} ms/page.",
            aeb_run.ms_per_page, dev_run.ms_per_page, test_run.ms_per_page
        ),
    };
    let text = report.markdown();
    if let Some(path) = write {
        if let Err(e) = fs::write(&path, &text) {
            eprintln!("quality: writing {}: {e}", path.display());
            return ExitCode::from(2);
        }
        println!("wrote {}", path.display());
    } else if let Some(path) = check {
        let current = fs::read_to_string(&path).unwrap_or_default();
        if without_timing(&current) != without_timing(&text) {
            eprintln!(
                "quality: {} is out of date. Reader mode's scores changed; regenerate it with\n  \
                 cargo run --release --manifest-path conformance/Cargo.toml --bin quality -- --write {}\n\
                 and review the difference.\n\n--- new report ---\n{text}",
                path.display(),
                path.display()
            );
            return ExitCode::FAILURE;
        }
        println!("{} is up to date", path.display());
    } else {
        print!("{}", report.summary());
    }
    ExitCode::SUCCESS
}

/// The version of the domq crate being measured (from its manifest).
fn domq_version() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"))
        .lines()
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or("?")
}

fn missing(e: &str) -> ExitCode {
    eprintln!("quality: {e}\nRun `sh conformance/fetch.sh aeb wcxb` first.");
    ExitCode::from(2)
}

/// One labelled page.
struct Page {
    id: String,
    html: String,
    /// AEB's `articleBody` or WCXB's `main_content`.
    reference: String,
    /// WCXB only.
    with: Vec<String>,
    without: Vec<String>,
    page_type: String,
}

fn gunzip(path: &Path) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    GzDecoder::new(file)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("decompressing {}: {e}", path.display()))?;
    // Both benchmarks decode their pages as UTF-8.
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parsing {}: {e}", path.display()))
}

fn load_aeb(dir: &Path) -> Result<Vec<Page>, String> {
    let truth = read_json(&dir.join("ground-truth.json"))?;
    let truth = truth
        .as_object()
        .ok_or("ground-truth.json is not an object")?;
    let mut pages = Vec::new();
    for (id, item) in truth {
        let reference = item
            .get("articleBody")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        pages.push(Page {
            id: id.clone(),
            html: gunzip(&dir.join("html").join(format!("{id}.html.gz")))?,
            reference,
            with: Vec::new(),
            without: Vec::new(),
            page_type: "article".to_string(),
        });
    }
    pages.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(pages)
}

fn load_wcxb(dir: &Path, split: &str) -> Result<Vec<Page>, String> {
    let truth_dir = dir.join(split).join("ground-truth");
    let mut files: Vec<PathBuf> = fs::read_dir(&truth_dir)
        .map_err(|e| format!("reading {}: {e}", truth_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let strings = |v: Option<&Value>| -> Vec<String> {
        v.and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut pages = Vec::new();
    for file in files {
        let data = read_json(&file)?;
        let Some(gt) = data.get("ground_truth").filter(|g| g.is_object()) else {
            continue;
        };
        let id = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let page_type = match data.pointer("/_internal/page_type") {
            Some(Value::Object(o)) => o
                .get("primary")
                .and_then(Value::as_str)
                .unwrap_or("article"),
            Some(Value::String(s)) => s.as_str(),
            _ => "article",
        };
        let page_type = if page_type == "category" {
            "collection"
        } else {
            page_type
        };
        pages.push(Page {
            html: gunzip(&dir.join(split).join("html").join(format!("{id}.html.gz")))?,
            reference: gt
                .get("main_content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            with: strings(gt.get("with")),
            without: strings(gt.get("without")),
            page_type: page_type.to_string(),
            id,
        });
    }
    Ok(pages)
}

/// The extracted text of every page.
struct Run {
    /// `main_content()` as text (the whole page when it is `None`).
    main: Vec<String>,
    /// The whole page as text.
    whole: Vec<String>,
    fallbacks: usize,
    ms_per_page: f64,
}

fn run(pages: &[Page]) -> Run {
    let (mut main, mut whole, mut fallbacks) = (Vec::new(), Vec::new(), 0);
    let start = Instant::now();
    for page in pages {
        let dom = domq::parse(&page.html);
        match dom.main_content() {
            Some(node) => main.push(node.to_text()),
            None => {
                fallbacks += 1;
                main.push(dom.to_text());
            }
        }
    }
    let ms_per_page = start.elapsed().as_secs_f64() * 1000.0 / pages.len().max(1) as f64;
    for page in pages {
        whole.push(domq::parse(&page.html).to_text());
    }
    Run {
        main,
        whole,
        fallbacks,
        ms_per_page,
    }
}

/// `\w+` tokens, as Python's `re.findall(r'\w+', …)` finds them.
fn tokens(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .collect()
}

// --- AEB --------------------------------------------------------------------

/// A page's shingle matches, normalized to sum to 1.
type TpFpFn = (f64, f64, f64);

fn shingles(text: &str) -> HashMap<Vec<&str>, usize> {
    let toks = tokens(text);
    let mut bag = HashMap::new();
    // As `_ngrams`: one (shorter) shingle when there are fewer than 4 tokens.
    let n = 4;
    for i in 0..toks.len().saturating_sub(n).saturating_add(1).max(1) {
        let shingle: Vec<&str> = toks.iter().skip(i).take(n).copied().collect();
        if !shingle.is_empty() {
            *bag.entry(shingle).or_insert(0) += 1;
        }
    }
    bag
}

fn shingle_matching(truth: &str, pred: &str) -> TpFpFn {
    let (t, p) = (shingles(truth), shingles(pred));
    let (mut tp, mut fp, mut fn_) = (0.0, 0.0, 0.0);
    for key in t.keys().chain(p.keys().filter(|k| !t.contains_key(*k))) {
        let tc = *t.get(key).unwrap_or(&0) as f64;
        let pc = *p.get(key).unwrap_or(&0) as f64;
        tp += tc.min(pc);
        fp += (pc - tc).max(0.0);
        fn_ += (tc - pc).max(0.0);
    }
    let sum = tp + fp + fn_;
    if sum > 0.0 {
        (tp / sum, fp / sum, fn_ / sum)
    } else {
        (tp, fp, fn_)
    }
}

fn precision(&(tp, fp, fn_): &TpFpFn) -> f64 {
    if fp == 0.0 && fn_ == 0.0 {
        1.0
    } else if tp == 0.0 && fp == 0.0 {
        0.0
    } else {
        tp / (tp + fp)
    }
}

fn recall(&(tp, fp, fn_): &TpFpFn) -> f64 {
    if fp == 0.0 && fn_ == 0.0 {
        1.0
    } else if tp == 0.0 && fn_ == 0.0 {
        0.0
    } else {
        tp / (tp + fn_)
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    if n == 0 { 0.0 } else { sum / n as f64 }
}

/// Precision, recall and F1 over pages, as `metrics_from_tp_fp_fns`.
fn aeb_metrics(matches: &[&TpFpFn]) -> (f64, f64, f64) {
    let p = mean(
        matches
            .iter()
            .filter(|m| m.0 + m.1 > 0.0)
            .map(|m| precision(m)),
    );
    let r = mean(
        matches
            .iter()
            .filter(|m| m.0 + m.2 > 0.0)
            .map(|m| recall(m)),
    );
    let f1 = if p + r > 0.0 {
        2.0 * p * r / (p + r)
    } else {
        0.0
    };
    (p, r, f1)
}

/// A small deterministic generator for the bootstrap (xorshift64*).
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) % n as u64) as usize
    }
}

fn stdev(values: &[f64]) -> f64 {
    let m = mean(values.iter().copied());
    let var =
        values.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / (values.len().max(2) - 1) as f64;
    var.sqrt()
}

struct AebScore {
    precision: f64,
    recall: f64,
    f1: f64,
    f1_std: f64,
    precision_std: f64,
    recall_std: f64,
    accuracy: f64,
}

fn score_aeb<'a>(pages: &[Page], text: impl Fn(usize) -> &'a String) -> AebScore {
    let normalize = |s: &str| s.replace('\u{ad}', "");
    let matches: Vec<TpFpFn> = pages
        .iter()
        .enumerate()
        .map(|(i, p)| shingle_matching(&p.reference, &normalize(text(i))))
        .collect();
    let exact = pages
        .iter()
        .enumerate()
        .filter(|(i, p)| tokens(&p.reference) == tokens(&normalize(text(*i))))
        .count();
    let all: Vec<&TpFpFn> = matches.iter().collect();
    let (precision, recall, f1) = aeb_metrics(&all);
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut ps, mut rs, mut fs) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..1000 {
        let sample: Vec<&TpFpFn> = (0..matches.len())
            .map(|_| &matches[rng.below(matches.len())])
            .collect();
        let (p, r, f) = aeb_metrics(&sample);
        ps.push(p);
        rs.push(r);
        fs.push(f);
    }
    AebScore {
        precision,
        recall,
        f1,
        f1_std: stdev(&fs),
        precision_std: stdev(&ps),
        recall_std: stdev(&rs),
        accuracy: exact as f64 / pages.len().max(1) as f64,
    }
}

// --- WCXB -------------------------------------------------------------------

fn word_prf(pred: &str, reference: &str) -> (f64, f64, f64) {
    let (pred_l, ref_l) = (pred.to_lowercase(), reference.to_lowercase());
    let (p_toks, r_toks) = (tokens(&pred_l), tokens(&ref_l));
    if r_toks.is_empty() {
        return if p_toks.is_empty() {
            (1.0, 1.0, 1.0)
        } else {
            (0.0, 0.0, 0.0)
        };
    }
    if p_toks.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut counts: HashMap<&str, i64> = HashMap::new();
    for t in &r_toks {
        *counts.entry(t).or_insert(0) += 1;
    }
    let mut overlap = 0usize;
    for t in &p_toks {
        if let Some(c) = counts.get_mut(t)
            && *c > 0
        {
            *c -= 1;
            overlap += 1;
        }
    }
    let p = overlap as f64 / p_toks.len() as f64;
    let r = overlap as f64 / r_toks.len() as f64;
    let f1 = if p + r > 0.0 {
        2.0 * p * r / (p + r)
    } else {
        0.0
    };
    (p, r, f1)
}

fn snippet_rate(text: &str, snippets: &[String]) -> f64 {
    if snippets.is_empty() {
        return 1.0;
    }
    let lower = text.to_lowercase();
    snippets
        .iter()
        .filter(|s| lower.contains(&s.to_lowercase()))
        .count() as f64
        / snippets.len() as f64
}

#[derive(Default, Clone, Copy)]
struct WcxbScore {
    pages: usize,
    precision: f64,
    recall: f64,
    f1: f64,
    with: f64,
    without: f64,
    whole_f1: f64,
}

struct WcxbReport {
    overall: WcxbScore,
    by_type: Vec<(String, WcxbScore)>,
    fallbacks: usize,
}

const TYPES: &[&str] = &[
    "article",
    "forum",
    "product",
    "collection",
    "listing",
    "documentation",
    "service",
];

/// One WCXB page's scores.
struct PageScore<'a> {
    page_type: &'a str,
    /// Precision, recall and F1 of the extraction.
    prf: (f64, f64, f64),
    with: f64,
    without: f64,
    /// F1 of the whole page's text.
    whole_f1: f64,
}

fn score_wcxb(pages: &[Page], run: &Run) -> WcxbReport {
    let per_page: Vec<PageScore<'_>> = pages
        .iter()
        .enumerate()
        .map(|(i, p)| PageScore {
            page_type: p.page_type.as_str(),
            prf: word_prf(&run.main[i], &p.reference),
            with: snippet_rate(&run.main[i], &p.with),
            without: snippet_rate(&run.main[i], &p.without),
            whole_f1: word_prf(&run.whole[i], &p.reference).2,
        })
        .collect();
    let summarize = |rows: &[&PageScore<'_>]| WcxbScore {
        pages: rows.len(),
        precision: mean(rows.iter().map(|r| r.prf.0)),
        recall: mean(rows.iter().map(|r| r.prf.1)),
        f1: mean(rows.iter().map(|r| r.prf.2)),
        with: mean(rows.iter().map(|r| r.with)),
        without: mean(rows.iter().map(|r| r.without)),
        whole_f1: mean(rows.iter().map(|r| r.whole_f1)),
    };
    let all: Vec<&PageScore<'_>> = per_page.iter().collect();
    let by_type = TYPES
        .iter()
        .map(|t| {
            let rows: Vec<&PageScore<'_>> = per_page.iter().filter(|r| r.page_type == *t).collect();
            (t.to_string(), summarize(&rows))
        })
        .filter(|(_, s)| s.pages > 0)
        .collect();
    WcxbReport {
        overall: summarize(&all),
        by_type,
        fallbacks: run.fallbacks,
    }
}

// --- Report -------------------------------------------------------------------

struct Report {
    aeb_commit: String,
    wcxb_commit: String,
    aeb_main: AebScore,
    aeb_whole: AebScore,
    aeb_fallbacks: usize,
    dev: WcxbReport,
    test: WcxbReport,
    timing: String,
}

/// Published AEB results (its README at the pinned commit): name, version,
/// F1, precision, recall. Main-content extractors, then whole-page converters.
const AEB_PUBLISHED: &[(&str, &str, f64, f64, f64)] = &[
    ("rs_trafilatura", "9261e08", 0.970, 0.951, 0.990),
    ("go_trafilatura", "ae7ea06", 0.960, 0.940, 0.980),
    ("trafilatura (Python)", "2.0.0", 0.958, 0.938, 0.978),
    ("newspaper4k", "0.9.3.1", 0.949, 0.964, 0.934),
    ("Readability.js", "0.6.0", 0.947, 0.914, 0.982),
    (
        "go_readability (readeck fork)",
        "fb0fbc5",
        0.947,
        0.914,
        0.982,
    ),
    ("readability-lxml", "0.8.4.1", 0.922, 0.913, 0.931),
    ("goose3", "3.1.20", 0.896, 0.940, 0.856),
    ("readable-readability (Rust)", "0.4.0", 0.884, 0.886, 0.881),
    ("readability (Rust)", "0.3.0", 0.873, 0.906, 0.843),
    ("dom_smoothie (Rust)", "0.14.0", 0.865, 0.785, 0.963),
    ("readabilityrs (Rust)", "0.1.2", 0.832, 0.745, 0.943),
    ("llm_readability (Rust)", "0.0.13", 0.829, 0.851, 0.809),
    ("justext", "3.0.2", 0.804, 0.858, 0.756),
    ("html-text, whole page", "0.7.0", 0.665, 0.500, 0.994),
    ("htmd, whole page (Rust)", "0.5.0", 0.184, 0.102, 0.970),
];

/// Published WCXB results (its README at the pinned commit): name, dev F1,
/// dev precision, dev recall, test F1 (`None` when not published).
const WCXB_PUBLISHED: &[(&str, f64, f64, f64, Option<f64>)] = &[
    ("rs-trafilatura", 0.859, 0.863, 0.890, Some(0.893)),
    ("MinerU-HTML (0.6B model)", 0.827, 0.845, 0.840, None),
    ("Trafilatura", 0.791, 0.852, 0.793, Some(0.833)),
    ("dom-smoothie", 0.762, 0.806, 0.768, Some(0.808)),
    ("ReaderLM-v2 (1.5B model)", 0.741, 0.741, 0.790, None),
    ("Newspaper4k", 0.720, 0.838, 0.683, None),
    ("magic-html", 0.719, 0.813, 0.713, None),
    ("jusText", 0.707, 0.771, 0.695, None),
    ("BoilerPy3", 0.687, 0.795, 0.661, None),
    ("Readability", 0.675, 0.685, 0.713, Some(0.726)),
    ("Goose3", 0.652, 0.845, 0.593, None),
];

/// Published WCXB dev F1 by page type: rs-trafilatura, Trafilatura,
/// dom-smoothie, Readability.
const WCXB_TYPES_PUBLISHED: &[(&str, [f64; 4])] = &[
    ("article", [0.932, 0.926, 0.908, 0.825]),
    ("forum", [0.808, 0.585, 0.530, 0.466]),
    ("product", [0.641, 0.567, 0.502, 0.407]),
    ("collection", [0.716, 0.553, 0.504, 0.445]),
    ("listing", [0.707, 0.589, 0.596, 0.496]),
    ("documentation", [0.932, 0.888, 0.868, 0.736]),
    ("service", [0.844, 0.763, 0.714, 0.604]),
];

const TIMING_START: &str = "<!-- timing: not checked by CI -->";
const TIMING_END: &str = "<!-- /timing -->";

fn without_timing(text: &str) -> String {
    match (text.find(TIMING_START), text.find(TIMING_END)) {
        (Some(a), Some(b)) if a < b => format!("{}{}", &text[..a], &text[b..]),
        _ => text.to_string(),
    }
}

impl Report {
    fn summary(&self) -> String {
        format!(
            "AEB (181 articles): F1 {:.3} ± {:.3} (P {:.3}, R {:.3}); whole page F1 {:.3}\n\
             WCXB dev ({} pages): F1 {:.3} (P {:.3}, R {:.3}); whole page F1 {:.3}\n\
             WCXB test ({} pages): F1 {:.3}\n{}\n",
            self.aeb_main.f1,
            self.aeb_main.f1_std,
            self.aeb_main.precision,
            self.aeb_main.recall,
            self.aeb_whole.f1,
            self.dev.overall.pages,
            self.dev.overall.f1,
            self.dev.overall.precision,
            self.dev.overall.recall,
            self.dev.overall.whole_f1,
            self.test.overall.pages,
            self.test.overall.f1,
            self.timing
        )
    }

    fn markdown(&self) -> String {
        let a = &self.aeb_main;
        let w = &self.aeb_whole;
        let short = |c: &str| c.get(..7).unwrap_or(c).to_string();
        let mut out = String::new();
        let _ = write!(
            out,
            "# Reader mode on labelled benchmarks

How well `Dom::main_content()` (`domq --main`, the MCP tools' `main` option)
extracts a page's main content, scored against **human-labelled** ground
truth on the two public benchmarks other extractors publish numbers for, with
each benchmark's own scoring. ([docs/reader-mode.md](reader-mode.md) is
different: it measures *agreement* with Readability on Readability's own test
pages.)

<!-- Generated by conformance/ (`cargo run --release --manifest-path
conformance/Cargo.toml --bin quality -- --write docs/reader-quality.md`). Do
not edit: CI regenerates it and fails when the scores change. -->

domq's rows are measured on the commit this report was generated from
(released versions are noted in the changelog). What is scored: the text of
`main_content()`, or of the whole page when it
finds no container (as `domq --main -t` does). The published rows are copied
from each benchmark's README at the pinned commit; they were produced by the
benchmark maintainers with other tools' versions at the time, so read small
differences with care.

## AEB: article extraction benchmark

[Zyte / ScrapingHub's benchmark](https://github.com/scrapinghub/article-extraction-benchmark/tree/{aeb_commit})
(commit `{aeb_short}`, MIT): **181 article pages**. Precision and recall over
4-word shingles of the article body, averaged over pages; F1 from the two
means; ± is the bootstrap standard deviation (1,000 resamples). Accuracy is
the share of pages extracted word for word.

| Extractor | Version | F1 | Precision | Recall |
|---|---|---:|---:|---:|
| **domq `main_content()`** | this commit | **{f1:.3} ± {f1s:.3}** | {p:.3} ± {ps:.3} | {r:.3} ± {rs:.3} |
| domq, whole page `to_text()` | this commit | {wf1:.3} | {wp:.3} | {wr:.3} |
",
            aeb_commit = self.aeb_commit,
            aeb_short = short(&self.aeb_commit),
            f1 = a.f1,
            f1s = a.f1_std,
            p = a.precision,
            ps = a.precision_std,
            r = a.recall,
            rs = a.recall_std,
            wf1 = w.f1,
            wp = w.precision,
            wr = w.recall,
        );
        for (name, version, f1, p, r) in AEB_PUBLISHED {
            let _ = writeln!(out, "| {name} | {version} | {f1:.3} | {p:.3} | {r:.3} |");
        }
        let _ = write!(
            out,
            "
domq's accuracy (pages word for word): {acc:.3}. `main_content()` found no
container on {fb} of the 181 pages; their whole text was used.

## WCXB: Web Content eXtraction Benchmark

[WCXB](https://github.com/Murrough-Foley/web-content-extraction-benchmark/tree/{wcxb_commit})
(commit `{wcxb_short}`, CC-BY-4.0, [leaderboard](https://webcontentextraction.org)):
**2,008 pages in seven page types**, split into a development set and a
held-out test set. Word-level F1 (bags of lower-cased words) per page,
averaged over pages. \"Must include\" is the share of required snippets found
in the extraction (higher is better), \"must exclude\" the share of
boilerplate snippets found (lower is better). domq's heuristics may be tuned
on the dev set only; the test set is for reporting.

| Extractor | Dev F1 | Dev precision | Dev recall | Test F1 |
|---|---:|---:|---:|---:|
| **domq `main_content()`** | **{df1:.3}** | {dp:.3} | {dr:.3} | **{tf1:.3}** |
| domq, whole page `to_text()` | {dwf1:.3} | | | {twf1:.3} |
",
            acc = a.accuracy,
            fb = self.aeb_fallbacks,
            wcxb_commit = self.wcxb_commit,
            wcxb_short = short(&self.wcxb_commit),
            df1 = self.dev.overall.f1,
            dp = self.dev.overall.precision,
            dr = self.dev.overall.recall,
            tf1 = self.test.overall.f1,
            dwf1 = self.dev.overall.whole_f1,
            twf1 = self.test.overall.whole_f1,
        );
        for (name, f1, p, r, test) in WCXB_PUBLISHED {
            let test = test.map_or_else(|| "—".to_string(), |t| format!("{t:.3}"));
            let _ = writeln!(out, "| {name} | {f1:.3} | {p:.3} | {r:.3} | {test} |");
        }
        let _ = write!(
            out,
            "
domq's snippet checks: must include {dw:.1}% (dev) / {tw:.1}% (test), must
exclude {dwo:.1}% (dev) / {two:.1}% (test). `main_content()` found no container
on {dfb} dev and {tfb} test pages.

### By page type (dev)

| Page type | Pages | domq F1 | domq whole page | rs-trafilatura | Trafilatura | dom-smoothie | Readability |
|---|---:|---:|---:|---:|---:|---:|---:|
",
            dw = self.dev.overall.with * 100.0,
            tw = self.test.overall.with * 100.0,
            dwo = self.dev.overall.without * 100.0,
            two = self.test.overall.without * 100.0,
            dfb = self.dev.fallbacks,
            tfb = self.test.fallbacks,
        );
        for (name, s) in &self.dev.by_type {
            let published = WCXB_TYPES_PUBLISHED
                .iter()
                .find(|(t, _)| t == name)
                .map(|(_, v)| {
                    v.iter()
                        .map(|x| format!("{x:.3}"))
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .unwrap_or_else(|| "— | — | — | —".to_string());
            let _ = writeln!(
                out,
                "| {name} | {} | {:.3} | {:.3} | {published} |",
                s.pages, s.f1, s.whole_f1
            );
        }
        let _ = write!(
            out,
            "
## Speed

{TIMING_START}
{timing}
{TIMING_END}

## Reproduce

```sh
sh conformance/fetch.sh aeb wcxb   # pinned commits, sparse fetch (~210 MB)
cargo run --release --manifest-path conformance/Cargo.toml --bin quality
# Cross-check against the benchmarks' own scripts:
cargo run --release --manifest-path conformance/Cargo.toml --bin quality -- --predictions /tmp/domq
```

The pages are © their publishers; the benchmarks are by Zyte (MIT) and
Murrough Foley (CC-BY-4.0). They are downloaded by `conformance/fetch.sh`, not
stored in this repository.
",
            timing = self.timing,
        );
        out
    }
}

fn write_predictions(
    dir: &Path,
    aeb: &[Page],
    aeb_run: &Run,
    dev: &[Page],
    dev_run: &Run,
    test: &[Page],
    test_run: &Run,
) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let mut output = serde_json::Map::new();
    for (page, text) in aeb.iter().zip(&aeb_run.main) {
        output.insert(
            page.id.clone(),
            serde_json::json!({ "articleBody": text.replace('\u{ad}', "") }),
        );
    }
    let aeb_json = serde_json::json!({ "version": domq_version(), "output": output });
    fs::write(
        dir.join("aeb-domq.json"),
        serde_json::to_string_pretty(&aeb_json)?,
    )?;
    for (name, pages, run) in [("dev", dev, dev_run), ("test", test, test_run)] {
        let map: serde_json::Map<String, Value> = pages
            .iter()
            .zip(&run.main)
            .map(|(p, t)| (p.id.clone(), Value::String(t.clone())))
            .collect();
        fs::write(
            dir.join(format!("wcxb-{name}-domq.json")),
            serde_json::to_string_pretty(&Value::Object(map))?,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shingles_match_evaluate_py() {
        // Fewer than 4 tokens: one short shingle, as `_ngrams` makes.
        assert_eq!(shingles("a b").len(), 1);
        let (tp, fp, fn_) = shingle_matching("one two three four five", "one two three four");
        // truth: [1234, 2345]; pred: [1234] -> tp 1, fn 1, normalized.
        assert!((tp - 0.5).abs() < 1e-12 && fp == 0.0 && (fn_ - 0.5).abs() < 1e-12);
        assert_eq!(precision(&(0.0, 0.0, 0.0)), 1.0);
    }

    #[test]
    fn word_f1_matches_evaluate_py() {
        let (p, r, f) = word_prf("The cat sat", "the cat");
        assert!((p - 2.0 / 3.0).abs() < 1e-12 && (r - 1.0).abs() < 1e-12);
        assert!((f - 0.8).abs() < 1e-12);
        assert_eq!(word_prf("", ""), (1.0, 1.0, 1.0));
        assert_eq!(snippet_rate("Hello World", &["world".into()]), 1.0);
    }

    #[test]
    fn tokens_are_python_word_runs() {
        assert_eq!(tokens("it's a_b, 42!"), ["it", "s", "a_b", "42"]);
    }
}

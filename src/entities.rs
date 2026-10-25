//! HTML character-reference (entity) decoding.
//!
//! By default we decode the ~200 named entities that actually appear in
//! real-world content plus full numeric references (`&#169;`, `&#xA9;`, with
//! the browser's Windows-1252 remapping of `&#128;`–`&#159;`). Unknown named
//! references are left **verbatim** (browser-ish behaviour) and reported via
//! [`ErrorKind::UnknownEntity`](crate::ErrorKind::UnknownEntity). Shipping the
//! full WHATWG table by default would conflict with the crate's "tiny" goal, so
//! it is opt-in: the `entities-full` feature swaps the curated table for all
//! 2,125 `;`-terminated names (generated into `src/entities_full.rs` by
//! `scripts/gen_entities.py`; looked up by binary search).
//!
//! As in browsers, a missing `;` is recovered where WHATWG says so: numeric
//! references end at the last digit (`&#169 2026`), and the 106 legacy names
//! ([`LEGACY`], in both tables) decode without it (`&copy 2026`, `AT&ampT`),
//! except in an attribute value when an alphanumeric or `=` follows
//! (`?a=1&copy=2` stays as written). Each of those is reported as
//! [`ErrorKind::MissingSemicolon`](crate::ErrorKind::MissingSemicolon).

use alloc::borrow::Cow;
use alloc::string::String;

use crate::error::ErrorKind;
use crate::scan;

/// Maximum length we will scan looking for a closing `;`. Real entity names are
/// short; capping avoids treating a far-away `;` (as in `&nbsp ; foo`) as part
/// of a reference and keeps decoding linear.
const MAX_ENTITY_LEN: usize = 32;

/// The Unicode replacement character, used for out-of-range / null numeric refs.
const REPLACEMENT: char = '\u{FFFD}';

/// What decoding reports: the byte offset of an `&` and what was wrong.
pub(crate) type Problem = (usize, ErrorKind);

/// Decode HTML entities in `input`.
///
/// Returns the decoded text (borrowed unchanged when there is nothing to do)
/// and the first [`Problem`], if any. `in_attr` selects the attribute-value
/// rules (see [`decode_into`]).
pub(crate) fn decode(input: &str, in_attr: bool) -> (Cow<'_, str>, Option<Problem>) {
    if scan::find_byte(b'&', input.as_bytes()).is_none() {
        return (Cow::Borrowed(input), None);
    }
    let mut out = String::with_capacity(input.len());
    let unknown = decode_into(input, &mut out, in_attr);
    (Cow::Owned(out), unknown)
}

/// Append `input` to `out` with its HTML entities decoded (the tree builder
/// decodes straight into its string arena, with no temporary). Returns the
/// first [`Problem`]: its offset is the `&`'s in `input` (the parser adds the
/// token's source offset to it).
///
/// Decoding follows WHATWG. A numeric reference may end without its `;`
/// (`&#169 2026`). A named one is the longest known `&name;`, else the
/// longest legacy name ([`LEGACY`]) at the start of the run of letters and
/// digits after the `&`, so `&notit;` is `¬it;` and `AT&ampT` is `AT&T` —
/// but in an attribute value a legacy name followed by an alphanumeric or `=`
/// is plain text (`?a=1&copy=2`, `&copyright=x`). A reference decoded
/// without its `;` is reported as [`ErrorKind::MissingSemicolon`].
///
/// An unknown `&name;` or a malformed `&#…` stays verbatim and is reported as
/// [`ErrorKind::UnknownEntity`]. Any other `&` is plain text, valid HTML, and
/// not reported (`a & b`, `&order=1`, `tom&nearby`, `x &ne y`).
pub(crate) fn decode_into(input: &str, out: &mut String, in_attr: bool) -> Option<Problem> {
    let mut problem = None;
    let mut rest = input;

    while let Some(amp) = scan::find_byte(b'&', rest.as_bytes()) {
        // Everything before the '&' is literal.
        if let Some(literal) = rest.get(..amp) {
            out.push_str(literal);
        }
        // `after` starts just past the '&'.
        let after = match rest.get(amp + 1..) {
            Some(s) => s,
            None => {
                out.push('&');
                rest = "";
                break;
            }
        };

        // `rest` is a suffix of `input` and `amp` indexes it.
        let at = input.len() - rest.len() + amp;
        match decode_one(after, out, in_attr) {
            Some((consumed, terminated)) => {
                if !terminated && problem.is_none() {
                    problem = Some((at, ErrorKind::MissingSemicolon));
                }
                rest = after.get(consumed..).unwrap_or("");
            }
            None => {
                // Not a valid reference: keep the '&' verbatim and continue.
                // Only a reference-shaped `&…` is reported; a bare `&`
                // (`a & b`, `?a=1&b=2`) is valid HTML.
                if problem.is_none() && looks_like_reference(after, in_attr) {
                    problem = Some((at, ErrorKind::UnknownEntity));
                }
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    problem
}

/// The names WHATWG also recognizes without their `;` ("legacy" references,
/// for compatibility with old content): the 106 keys of
/// <https://html.spec.whatwg.org/entities.json> with no trailing `;`, sorted
/// in byte order for [`longest_legacy_prefix`]. Each is also a
/// `;`-terminated name.
const LEGACY: [&str; 106] = [
    "AElig", "AMP", "Aacute", "Acirc", "Agrave", "Aring", "Atilde", "Auml", "COPY", "Ccedil",
    "ETH", "Eacute", "Ecirc", "Egrave", "Euml", "GT", "Iacute", "Icirc", "Igrave", "Iuml", "LT",
    "Ntilde", "Oacute", "Ocirc", "Ograve", "Oslash", "Otilde", "Ouml", "QUOT", "REG", "THORN",
    "Uacute", "Ucirc", "Ugrave", "Uuml", "Yacute", "aacute", "acirc", "acute", "aelig", "agrave",
    "amp", "aring", "atilde", "auml", "brvbar", "ccedil", "cedil", "cent", "copy", "curren", "deg",
    "divide", "eacute", "ecirc", "egrave", "eth", "euml", "frac12", "frac14", "frac34", "gt",
    "iacute", "icirc", "iexcl", "igrave", "iquest", "iuml", "laquo", "lt", "macr", "micro",
    "middot", "nbsp", "not", "ntilde", "oacute", "ocirc", "ograve", "ordf", "ordm", "oslash",
    "otilde", "ouml", "para", "plusmn", "pound", "quot", "raquo", "reg", "sect", "shy", "sup1",
    "sup2", "sup3", "szlig", "thorn", "times", "uacute", "ucirc", "ugrave", "uml", "uuml",
    "yacute", "yen", "yuml",
];

/// The length of the shortest [`LEGACY`] name (`lt`)…
const LEGACY_MIN_LEN: usize = 2;
/// …and of the longest (`middot`, `frac12`, …).
const LEGACY_MAX_LEN: usize = 6;

/// The length of the longest [`LEGACY`] name `run` starts with.
fn longest_legacy_prefix(run: &str) -> Option<usize> {
    let key = run.get(..run.len().min(LEGACY_MAX_LEN))?;
    // Every name that is a prefix of `key` sorts at or before it, and every
    // name sorting between such a prefix and `key` starts with that prefix:
    // so the candidates are the run of names just before `key` that share
    // its first letter.
    let end = LEGACY.partition_point(|name| *name <= key);
    LEGACY
        .get(..end)?
        .iter()
        .rev()
        .take_while(|name| name.as_bytes().first() == key.as_bytes().first())
        .filter(|name| key.starts_with(*name))
        .map(|name| name.len())
        .max()
        .filter(|&len| len >= LEGACY_MIN_LEN)
}

/// Whether the text after an `&` that [`decode_one`] left alone was meant as
/// a reference: a malformed `&#…`, or an unknown `&name;`.
fn looks_like_reference(after: &str, _in_attr: bool) -> bool {
    let bytes = after.as_bytes();
    if bytes.first() == Some(&b'#') {
        return true;
    }
    let n = alphanumeric_run(bytes);
    n > 0 && bytes.get(n) == Some(&b';')
}

/// The length of the run of ASCII letters and digits starting `bytes`, up to
/// [`MAX_ENTITY_LEN`].
fn alphanumeric_run(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .take(MAX_ENTITY_LEN)
        .take_while(|b| b.is_ascii_alphanumeric())
        .count()
}

/// Try to decode a single reference whose text begins right after the `&`
/// (rules on [`decode_into`]). On success appends what it decodes to to `out`
/// and returns how many bytes of `after` it used, and whether it ended with
/// its `;`.
fn decode_one(after: &str, out: &mut String, in_attr: bool) -> Option<(usize, bool)> {
    let bytes = after.as_bytes();
    if bytes.first() == Some(&b'#') {
        let (radix, start) = match bytes.get(1) {
            Some(b'x' | b'X') => (16, 2),
            _ => (10, 1),
        };
        let digits = bytes
            .get(start..)?
            .iter()
            .take_while(|&&b| char::from(b).is_digit(radix))
            .count();
        if digits == 0 {
            return None;
        }
        let end = start + digits;
        out.push(numeric(after.get(start..end)?, radix));
        return Some(match bytes.get(end) {
            Some(b';') => (end + 1, true),
            _ => (end, false),
        });
    }
    let n = alphanumeric_run(bytes);
    let run = after.get(..n)?;
    if n > 0
        && bytes.get(n) == Some(&b';')
        && let Some(value) = named(run)
    {
        out.push_str(value);
        return Some((n + 1, true));
    }
    let len = longest_legacy_prefix(run)?;
    if in_attr
        && bytes
            .get(len)
            .is_some_and(|&b| b == b'=' || b.is_ascii_alphanumeric())
    {
        return None;
    }
    out.push_str(named(run.get(..len)?)?);
    Some((len, false))
}

/// What numeric references `&#x80;`–`&#x9F;` decode to (WHATWG): the C1
/// control range is remapped through Windows-1252, because in real content it
/// almost always is mislabelled cp1252. Five slots are unmapped and decode to
/// themselves.
const WINDOWS_1252: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, // 80–87
    0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x008D, 0x017D, 0x008F, // 88–8F
    0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, // 90–97
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178, // 98–9F
];

/// Decode a numeric reference's digits (after `&#` or `&#x`), all valid in
/// `radix`.
///
/// `0`, surrogates and anything above U+10FFFF (however many digits) decode to
/// U+FFFD; `0x80..=0x9F` go through [`WINDOWS_1252`].
fn numeric(digits: &str, radix: u32) -> char {
    // Saturate rather than overflow: every out-of-range value is U+FFFD.
    let mut code: u32 = 0;
    for c in digits.chars() {
        code = code
            .saturating_mul(radix)
            .saturating_add(c.to_digit(radix).unwrap_or(0))
            .min(0x11_0000);
    }
    if let Some(&mapped) = code
        .checked_sub(0x80)
        .and_then(|i| WINDOWS_1252.get(i as usize))
    {
        code = u32::from(mapped);
    }
    char::from_u32(code)
        .filter(|&c| c != '\0')
        .unwrap_or(REPLACEMENT)
}

/// Look up a named reference (the part between `&` and `;`): the curated
/// table by default, the complete WHATWG one with the `entities-full`
/// feature. `None` means unknown (the caller leaves it verbatim).
#[inline]
fn named(name: &str) -> Option<&'static str> {
    #[cfg(feature = "entities-full")]
    {
        full(name)
    }
    #[cfg(not(feature = "entities-full"))]
    {
        curated(name)
    }
}

/// Binary search of the generated, name-sorted WHATWG table.
#[cfg(feature = "entities-full")]
fn full(name: &str) -> Option<&'static str> {
    use crate::entities_full::ENTITIES;
    let i = ENTITIES.binary_search_by(|&(k, _)| k.cmp(name)).ok()?;
    ENTITIES.get(i).map(|&(_, v)| v)
}

/// Curated named-entity table. Returns the UTF-8 replacement, or `None` if the
/// name is not recognised (caller leaves it verbatim).
#[cfg(any(not(feature = "entities-full"), test))]
fn curated(name: &str) -> Option<&'static str> {
    Some(match name {
        // Core / markup-significant
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => "\u{a0}",
        // Spaces & invisibles
        "ensp" => "\u{2002}",
        "emsp" => "\u{2003}",
        "thinsp" => "\u{2009}",
        "zwnj" => "\u{200c}",
        "zwj" => "\u{200d}",
        "lrm" => "\u{200e}",
        "rlm" => "\u{200f}",
        "shy" => "\u{ad}",
        // Punctuation & typography
        "hellip" => "\u{2026}",
        "mdash" => "\u{2014}",
        "ndash" => "\u{2013}",
        "lsquo" => "\u{2018}",
        "rsquo" => "\u{2019}",
        "sbquo" => "\u{201a}",
        "ldquo" => "\u{201c}",
        "rdquo" => "\u{201d}",
        "bdquo" => "\u{201e}",
        "laquo" => "\u{ab}",
        "raquo" => "\u{bb}",
        "lsaquo" => "\u{2039}",
        "rsaquo" => "\u{203a}",
        "bull" => "\u{2022}",
        "middot" => "\u{b7}",
        "dagger" => "\u{2020}",
        "Dagger" => "\u{2021}",
        "permil" => "\u{2030}",
        "prime" => "\u{2032}",
        "Prime" => "\u{2033}",
        "oline" => "\u{203e}",
        "frasl" => "\u{2044}",
        // Currency & legal
        "cent" => "\u{a2}",
        "pound" => "\u{a3}",
        "curren" => "\u{a4}",
        "yen" => "\u{a5}",
        "euro" => "\u{20ac}",
        "copy" => "\u{a9}",
        "reg" => "\u{ae}",
        "trade" => "\u{2122}",
        // Symbols & misc latin-1
        "sect" => "\u{a7}",
        "para" => "\u{b6}",
        "deg" => "\u{b0}",
        "plusmn" => "\u{b1}",
        "times" => "\u{d7}",
        "divide" => "\u{f7}",
        "frac14" => "\u{bc}",
        "frac12" => "\u{bd}",
        "frac34" => "\u{be}",
        "sup1" => "\u{b9}",
        "sup2" => "\u{b2}",
        "sup3" => "\u{b3}",
        "micro" => "\u{b5}",
        "iexcl" => "\u{a1}",
        "iquest" => "\u{bf}",
        "brvbar" => "\u{a6}",
        "uml" => "\u{a8}",
        "ordf" => "\u{aa}",
        "ordm" => "\u{ba}",
        "not" => "\u{ac}",
        "macr" => "\u{af}",
        "acute" => "\u{b4}",
        "cedil" => "\u{b8}",
        // Arrows & math
        "larr" => "\u{2190}",
        "uarr" => "\u{2191}",
        "rarr" => "\u{2192}",
        "darr" => "\u{2193}",
        "harr" => "\u{2194}",
        "lArr" => "\u{21d0}",
        "rArr" => "\u{21d2}",
        "hArr" => "\u{21d4}",
        "forall" => "\u{2200}",
        "part" => "\u{2202}",
        "exist" => "\u{2203}",
        "empty" => "\u{2205}",
        "nabla" => "\u{2207}",
        "isin" => "\u{2208}",
        "notin" => "\u{2209}",
        "ni" => "\u{220b}",
        "prod" => "\u{220f}",
        "sum" => "\u{2211}",
        "minus" => "\u{2212}",
        "lowast" => "\u{2217}",
        "radic" => "\u{221a}",
        "prop" => "\u{221d}",
        "infin" => "\u{221e}",
        "ang" => "\u{2220}",
        "and" => "\u{2227}",
        "or" => "\u{2228}",
        "cap" => "\u{2229}",
        "cup" => "\u{222a}",
        "int" => "\u{222b}",
        "there4" => "\u{2234}",
        "sim" => "\u{223c}",
        "cong" => "\u{2245}",
        "asymp" => "\u{2248}",
        "ne" => "\u{2260}",
        "equiv" => "\u{2261}",
        "le" => "\u{2264}",
        "ge" => "\u{2265}",
        "sub" => "\u{2282}",
        "sup" => "\u{2283}",
        "sube" => "\u{2286}",
        "supe" => "\u{2287}",
        "oplus" => "\u{2295}",
        "otimes" => "\u{2297}",
        "perp" => "\u{22a5}",
        "sdot" => "\u{22c5}",
        "loz" => "\u{25ca}",
        "spades" => "\u{2660}",
        "clubs" => "\u{2663}",
        "hearts" => "\u{2665}",
        "diams" => "\u{2666}",
        // Common accented latin (lower + upper for the frequent ones)
        "agrave" => "\u{e0}",
        "aacute" => "\u{e1}",
        "acirc" => "\u{e2}",
        "atilde" => "\u{e3}",
        "auml" => "\u{e4}",
        "aring" => "\u{e5}",
        "aelig" => "\u{e6}",
        "ccedil" => "\u{e7}",
        "egrave" => "\u{e8}",
        "eacute" => "\u{e9}",
        "ecirc" => "\u{ea}",
        "euml" => "\u{eb}",
        "igrave" => "\u{ec}",
        "iacute" => "\u{ed}",
        "icirc" => "\u{ee}",
        "iuml" => "\u{ef}",
        "ntilde" => "\u{f1}",
        "ograve" => "\u{f2}",
        "oacute" => "\u{f3}",
        "ocirc" => "\u{f4}",
        "otilde" => "\u{f5}",
        "ouml" => "\u{f6}",
        "oslash" => "\u{f8}",
        "ugrave" => "\u{f9}",
        "uacute" => "\u{fa}",
        "ucirc" => "\u{fb}",
        "uuml" => "\u{fc}",
        "yacute" => "\u{fd}",
        "yuml" => "\u{ff}",
        "szlig" => "\u{df}",
        "eth" => "\u{f0}",
        "thorn" => "\u{fe}",
        "Agrave" => "\u{c0}",
        "Aacute" => "\u{c1}",
        "Acirc" => "\u{c2}",
        "Auml" => "\u{c4}",
        "Aring" => "\u{c5}",
        "AElig" => "\u{c6}",
        "Ccedil" => "\u{c7}",
        "Egrave" => "\u{c8}",
        "Eacute" => "\u{c9}",
        "Ntilde" => "\u{d1}",
        "Ouml" => "\u{d6}",
        "Uuml" => "\u{dc}",
        "Oslash" => "\u{d8}",
        // The rest of the legacy names (decoded even without `;`).
        "AMP" => "&",
        "LT" => "<",
        "GT" => ">",
        "QUOT" => "\"",
        "COPY" => "\u{a9}",
        "REG" => "\u{ae}",
        "Atilde" => "\u{c3}",
        "Igrave" => "\u{cc}",
        "Iacute" => "\u{cd}",
        "Icirc" => "\u{ce}",
        "Iuml" => "\u{cf}",
        "Ecirc" => "\u{ca}",
        "Euml" => "\u{cb}",
        "ETH" => "\u{d0}",
        "Ograve" => "\u{d2}",
        "Oacute" => "\u{d3}",
        "Ocirc" => "\u{d4}",
        "Otilde" => "\u{d5}",
        "Ugrave" => "\u{d9}",
        "Uacute" => "\u{da}",
        "Ucirc" => "\u{db}",
        "Yacute" => "\u{dd}",
        "THORN" => "\u{de}",
        // Greek (the commonly seen ones)
        "alpha" => "\u{3b1}",
        "beta" => "\u{3b2}",
        "gamma" => "\u{3b3}",
        "delta" => "\u{3b4}",
        "epsilon" => "\u{3b5}",
        "theta" => "\u{3b8}",
        "lambda" => "\u{3bb}",
        "mu" => "\u{3bc}",
        "pi" => "\u{3c0}",
        "sigma" => "\u{3c3}",
        "phi" => "\u{3c6}",
        "omega" => "\u{3c9}",
        "Delta" => "\u{394}",
        "Sigma" => "\u{3a3}",
        "Omega" => "\u{3a9}",
        "Pi" => "\u{3a0}",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{LEGACY, LEGACY_MAX_LEN, LEGACY_MIN_LEN, decode, named};
    use crate::error::ErrorKind::{MissingSemicolon, UnknownEntity};
    use alloc::string::String;

    fn text(s: &str) -> String {
        decode(s, false).0.into_owned()
    }

    fn attr(s: &str) -> String {
        decode(s, true).0.into_owned()
    }

    #[test]
    fn every_legacy_name_decodes() {
        for name in LEGACY {
            assert!(named(name).is_some(), "&{name};");
        }
    }

    #[test]
    fn numeric_references_without_semicolon() {
        assert_eq!(text("FOO&#41BAR"), "FOO)BAR");
        assert_eq!(text("FOO&#x41ZOO"), "FOOAZOO");
        assert_eq!(text("FOO&#x41BAR"), "FOO\u{41ba}R"); // hex digits run on
        assert_eq!(text("&#169 2026"), "\u{a9} 2026");
        assert_eq!(text("&#0000000000000000000000000000000065;"), "A");
        assert_eq!(text("&#x; &#z &#"), "&#x; &#z &#");
        assert_eq!(decode("a&#41b", false).1, Some((1, MissingSemicolon)));
    }

    #[test]
    fn legacy_names_without_semicolon() {
        assert_eq!(text("FOO&gtBAR"), "FOO>BAR");
        assert_eq!(text("AT&ampT"), "AT&T");
        assert_eq!(text("I'm &notit; I tell you"), "I'm \u{ac}it; I tell you");
        assert_eq!(text("&notin; &ampersand;"), "\u{2209} &ersand;");
        assert_eq!(text("&copy 2026 &COPY"), "\u{a9} 2026 \u{a9}");
        // Only legacy names work without `;`, and unknown names stay.
        assert_eq!(
            text("&hellip &order=1 tom&nearby"),
            "&hellip &order=1 tom&nearby"
        );
    }

    #[test]
    fn attributes_keep_legacy_names_before_alphanumerics_and_equals() {
        assert_eq!(attr("?a=1&copy=2&ampx"), "?a=1&copy=2&ampx");
        assert_eq!(attr("&copy &amp;x &copy"), "\u{a9} &x \u{a9}");
        assert_eq!(attr("&lt;b&gt;"), "<b>");
    }

    #[test]
    fn legacy_table_is_sorted_for_binary_search() {
        assert!(LEGACY.windows(2).all(|w| w[0] < w[1]));
        let lens = LEGACY.iter().map(|n| n.len());
        assert_eq!(lens.clone().min(), Some(LEGACY_MIN_LEN));
        assert_eq!(lens.max(), Some(LEGACY_MAX_LEN));
        assert!(
            LEGACY
                .iter()
                .all(|n| n.bytes().all(|b| b.is_ascii_alphanumeric()))
        );
    }

    #[test]
    fn the_first_problem_is_located() {
        assert_eq!(decode("a & b", false).1, None);
        assert_eq!(decode("ab &bogus; &x;", false).1, Some((3, UnknownEntity)));
        assert_eq!(
            decode("&amp; é &#xZZ; &nosuch;", false).1,
            Some((9, UnknownEntity))
        );
        assert_eq!(decode("?a=1&b=2&copy", true).1, Some((8, MissingSemicolon)));
        assert_eq!(decode("?a=1&b=2&copy=3", true).1, None);
        assert_eq!(
            decode("x &copy &bogus;", false).1,
            Some((2, MissingSemicolon))
        );
    }
}

#[cfg(all(test, feature = "entities-full"))]
mod full_tests {
    use crate::entities_full::ENTITIES;

    #[test]
    fn legacy_names_are_full_names() {
        for name in super::LEGACY {
            assert!(super::full(name).is_some(), "&{name};");
        }
    }

    #[test]
    fn full_table_is_sorted_for_binary_search() {
        assert!(ENTITIES.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(
            ENTITIES
                .iter()
                .all(|(k, _)| k.len() < super::MAX_ENTITY_LEN)
        );
    }

    #[test]
    fn curated_table_is_a_subset_of_the_full_one() {
        // Turning the feature on must never change how a curated name decodes.
        let mut found = 0;
        for &(name, value) in &ENTITIES {
            if let Some(curated) = super::curated(name) {
                assert_eq!(curated, value, "&{name};");
                found += 1;
            }
        }
        assert_eq!(found, 201, "every curated name is a WHATWG name");
    }
}

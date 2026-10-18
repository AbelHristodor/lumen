//! Parsing and formatting of Bible references such as `John 3:16-18`.

use crate::fold::fold;
use crate::i18n::Language;

use super::names;

/// A book index into the 66 book canon, a 1 based chapter, and an optional
/// inclusive verse range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Reference {
    pub book: usize,
    pub chapter: u32,
    pub verses: Option<(u32, u32)>,
}

/// Canonical name of a book in the given language. Empty for an index past 65.
pub fn book_name(book: usize, lang: Language) -> &'static str {
    names::names(lang).get(book).copied().unwrap_or("")
}

/// Parse a reference typed by a user. Accepts `C`, `C:V`, `C,V`, `C:V-W`,
/// and `C:V–W` after the book name. Book names may be canonical, aliases,
/// unique prefixes, or in any of the three languages.
pub fn parse_reference(input: &str, lang: Language) -> Option<Reference> {
    let text = normalize(input);
    let (book_part, rest) = split_book(&text)?;
    let book = match_book(&book_part, lang)?;
    let (chapter, verses) = parse_chapter_verses(rest)?;
    Some(Reference {
        book,
        chapter,
        verses,
    })
}

/// Print a reference like `John 3`, `John 3:16`, or `John 3:16-18`.
pub fn format_reference(r: &Reference, lang: Language) -> String {
    let name = book_name(r.book, lang);
    match r.verses {
        None => format!("{name} {}", r.chapter),
        Some((a, b)) if a == b => format!("{name} {}:{a}", r.chapter),
        Some((a, b)) => format!("{name} {}:{a}-{b}", r.chapter),
    }
}

/// Fold accents, lower case, drop dots, collapse whitespace, and turn a
/// leading roman numeral book prefix into a digit.
fn normalize(input: &str) -> String {
    let folded = fold(input).replace('.', " ");
    let mut words: Vec<&str> = folded.split_whitespace().collect();
    if let Some(first) = words.first_mut() {
        *first = match *first {
            "i" => "1",
            "ii" => "2",
            "iii" => "3",
            other => other,
        };
    }
    words.join(" ")
}

/// Split `1 sam 2:1` into `("1 sam", "2:1")`. The first digit that is not
/// part of a numbered book prefix starts the chapter.
fn split_book(text: &str) -> Option<(String, &str)> {
    let mut scan_from = 0;
    let mut chars = text.char_indices();
    if let Some((_, c)) = chars.next() {
        if matches!(c, '1' | '2' | '3') {
            match chars.next() {
                Some((i, ' ')) => scan_from = i + 1,
                Some((i, next)) if next.is_alphabetic() => scan_from = i,
                _ => return None,
            }
        }
    }
    let rel = text[scan_from..].find(|c: char| c.is_ascii_digit())?;
    let pos = scan_from + rel;
    let mut book = text[..pos].trim().to_string();
    if book.is_empty() {
        return None;
    }
    // Turn `1sam` into `1 sam` so one alias form covers both spellings.
    if book.len() > 1 && book.as_bytes()[0].is_ascii_digit() && book.as_bytes()[1] != b' ' {
        book.insert(1, ' ');
    }
    Some((book, text[pos..].trim()))
}

/// Exact name, then alias, then unique prefix, first in `lang` then in the
/// other languages.
fn match_book(book: &str, lang: Language) -> Option<usize> {
    let mut order = vec![lang];
    order.extend(Language::all().into_iter().filter(|l| *l != lang));
    for l in order {
        if let Some(i) = match_book_in(book, l) {
            return Some(i);
        }
    }
    None
}

fn match_book_in(book: &str, lang: Language) -> Option<usize> {
    let canon = names::names(lang);
    if let Some(i) = canon.iter().position(|n| fold(n) == book) {
        return Some(i);
    }
    if let Some((_, i)) = names::aliases(lang).iter().find(|(a, _)| *a == book) {
        return Some(*i);
    }
    let mut hits = canon
        .iter()
        .enumerate()
        .filter(|(_, n)| fold(n).starts_with(book))
        .map(|(i, _)| i);
    match (hits.next(), hits.next()) {
        (Some(i), None) => Some(i),
        _ => None,
    }
}

/// Parse `3`, `3:16`, `3,16`, `3:16-18`, or `3:16–18`.
fn parse_chapter_verses(rest: &str) -> Option<(u32, Option<(u32, u32)>)> {
    let compact: String = rest.chars().filter(|c| !c.is_whitespace()).collect();
    let (chapter_s, verse_s) = match compact.split_once([':', ',']) {
        Some((c, v)) => (c, Some(v)),
        None => (compact.as_str(), None),
    };
    let chapter: u32 = chapter_s.parse().ok().filter(|c| *c > 0)?;
    let verses = match verse_s {
        None | Some("") => None,
        Some(v) => {
            let (a, b) = v.split_once(['-', '–']).unwrap_or((v, v));
            let a: u32 = a.parse().ok().filter(|n| *n > 0)?;
            let b: u32 = b.parse().ok().filter(|n| *n > 0)?;
            Some((a.min(b), a.max(b)))
        }
    };
    Some((chapter, verses))
}

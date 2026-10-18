//! Bible loader, localized references, verse search, and verse slides.
//!
//! Bibles are the JSON files of the thiagobodruk/bible repository pinned at
//! [`PIN`]. Each file is a 66 element array of books in canon order.

mod names;
mod reference;

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::fold::fold;
use crate::resolve::SlideContent;

pub use names::{EN, IT, RO};
pub use reference::{book_name, format_reference, parse_reference, Reference};

/// Git commit of thiagobodruk/bible that every download URL points at.
pub const PIN: &str = "0e8fd65a89727f5b73e9f88edb220da8e0d5daba";

const RAW_BASE: &str = "https://raw.githubusercontent.com/thiagobodruk/bible";

/// One book as stored in the repository JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Book {
    pub abbrev: String,
    pub name: String,
    pub chapters: Vec<Vec<String>>,
}

/// A loaded Bible. `code` is the file stem, for example `en_kjv`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bible {
    pub code: String,
    pub books: Vec<Book>,
}

/// One downloadable version from the repository index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub code: String,
    pub language: String,
    pub name: String,
    pub abbreviation: String,
    pub public_domain: Option<bool>,
}

/// URL of the pinned version index.
pub fn index_url() -> String {
    format!("{RAW_BASE}/{PIN}/json/index.json")
}

/// URL of one pinned version file.
pub fn version_url(code: &str) -> String {
    format!("{RAW_BASE}/{PIN}/json/{code}.json")
}

/// Known licensing status of a version code. `None` means unknown.
pub fn public_domain(code: &str) -> Option<bool> {
    const FREE: &[&str] = &[
        "en_kjv", "en_kjva", "en_asv", "en_darby", "en_dra", "en_ylt98", "en_gnv", "en_web",
        "en_bsb", "la_vulg", "cs_bkr", "hu_hunk", "eo_esp", "tl_abtag",
    ];
    const RESTRICTED: &[&str] = &[
        "en_niv",
        "en_esv",
        "en_nlt",
        "en_nasb",
        "en_nkjv",
        "en_csb",
        "en_amp",
        "en_gnt",
        "en_cev",
        "it_irb20",
        "ro_vdc",
        "es_rvr1960",
        "de_sch1951",
    ];
    if FREE.contains(&code) {
        Some(true)
    } else if RESTRICTED.contains(&code) {
        Some(false)
    } else {
        None
    }
}

#[derive(Deserialize)]
struct IndexLanguage {
    language: String,
    versions: Vec<IndexVersion>,
}

#[derive(Deserialize)]
struct IndexVersion {
    name: String,
    abbreviation: String,
}

/// Parse the repository `index.json` into a flat version list.
pub fn parse_index(json: &str) -> Result<Vec<VersionInfo>> {
    let langs: Vec<IndexLanguage> =
        serde_json::from_str(strip_bom(json)).context("bible index json")?;
    let mut out = Vec::new();
    for lang in langs {
        let prefix = language_prefix(&lang.language);
        for v in lang.versions {
            let code = format!("{prefix}_{}", v.abbreviation);
            out.push(VersionInfo {
                public_domain: public_domain(&code),
                code,
                language: lang.language.clone(),
                name: v.name,
                abbreviation: v.abbreviation,
            });
        }
    }
    Ok(out)
}

fn language_prefix(display: &str) -> String {
    const TABLE: &[(&str, &str)] = &[
        ("English", "en"),
        ("Romanian", "ro"),
        ("Italian", "it"),
        ("Portuguese", "pt"),
        ("Spanish", "es"),
        ("German", "de"),
        ("French", "fr"),
    ];
    if let Some((_, p)) = TABLE.iter().find(|(d, _)| *d == display) {
        return p.to_string();
    }
    display
        .chars()
        .take(2)
        .flat_map(char::to_lowercase)
        .collect()
}

fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

/// Load one version file. The code is the file stem.
pub fn load(path: &Path) -> Result<Bible> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let books: Vec<Book> = serde_json::from_str(strip_bom(&text))
        .with_context(|| format!("parse {}", path.display()))?;
    let code = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Bible { code, books })
}

/// Verses for a reference as `(number, text)`. The range is clamped to the
/// chapter. A missing book or chapter yields an empty list.
pub fn verses(bible: &Bible, r: &Reference) -> Vec<(u32, String)> {
    let Some(chapter) = bible
        .books
        .get(r.book)
        .and_then(|b| b.chapters.get(r.chapter.checked_sub(1)? as usize))
    else {
        return Vec::new();
    };
    let len = chapter.len() as u32;
    let (start, end) = match r.verses {
        Some((a, b)) => (a.max(1), b.min(len)),
        None => (1, len),
    };
    if start > end {
        return Vec::new();
    }
    (start..=end)
        .map(|n| (n, chapter[(n - 1) as usize].clone()))
        .collect()
}

/// Case and accent insensitive substring search over every verse.
/// Stops after `limit` hits. Each hit is a single verse reference.
pub fn search(bible: &Bible, query: &str, limit: usize) -> Vec<(Reference, String)> {
    let q = fold(query.trim());
    let mut hits = Vec::new();
    if q.is_empty() || limit == 0 {
        return hits;
    }
    // ponytail: linear scan over every verse, build a folded index if it feels slow
    for (bi, book) in bible.books.iter().enumerate() {
        for (ci, chapter) in book.chapters.iter().enumerate() {
            for (vi, verse) in chapter.iter().enumerate() {
                if fold(verse).contains(&q) {
                    let n = vi as u32 + 1;
                    hits.push((
                        Reference {
                            book: bi,
                            chapter: ci as u32 + 1,
                            verses: Some((n, n)),
                        },
                        verse.clone(),
                    ));
                    if hits.len() >= limit {
                        return hits;
                    }
                }
            }
        }
    }
    hits
}

/// Group verses into slides of `per_slide` verses each. Every slide carries
/// the same reference string.
pub fn verse_slides(verses: &[(u32, String)], reference: &str, per_slide: u8) -> Vec<SlideContent> {
    verses
        .chunks(per_slide.max(1) as usize)
        .map(|chunk| SlideContent::Verses {
            reference: reference.to_string(),
            verses: chunk.to_vec(),
        })
        .collect()
}

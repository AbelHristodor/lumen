use crate::fold::fold;
use crate::model::{Hymn, MatchField, SearchHit};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32String};

/// One searchable field: the folded text plus its UTF-32 encoding, so a
/// query never re-encodes or re-folds a haystack.
struct Field {
    kind: MatchField,
    weight: u32,
    folded: String,
    utf32: Utf32String,
}

impl Field {
    fn new(kind: MatchField, weight: u32, folded: String) -> Self {
        let utf32 = Utf32String::from(folded.as_str());
        Field { kind, weight, folded, utf32 }
    }
}

struct Indexed {
    entry: Hymn,
    label: String,
    fields: Vec<Field>,
}

/// In-memory fuzzy searcher over hymn entries. Owns its matcher so the
/// scoring slab is allocated once, not per keystroke.
pub struct Searcher {
    items: Vec<Indexed>,
    matcher: Matcher,
}

/// Sort key for a hymn number that orders numerically with a letter suffix as
/// the tiebreaker: "664" < "664a" < "664b" < "665". Numbers come before any
/// non-numeric/`None` value. Returns `(numeric, suffix)`; unparseable or `None`
/// sort last via `u32::MAX`.
fn number_sort_key(number: Option<&str>) -> (u32, &str) {
    match number {
        Some(s) => {
            let digits = s.len() - s.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            (s[..digits].parse().unwrap_or(u32::MAX), &s[digits..])
        }
        None => (u32::MAX, ""),
    }
}

/// One result row: "150  Cerul, pământul  · Imnuri".
fn row_label(entry: &Hymn) -> String {
    match &entry.number {
        Some(n) => format!("{n}  {}  · {}", entry.title, entry.library),
        None => format!("{}  · {}", entry.title, entry.library),
    }
}

impl Searcher {
    pub fn new(mut entries: Vec<Hymn>) -> Self {
        entries.sort_by(|a, b| {
            number_sort_key(a.number.as_deref()).cmp(&number_sort_key(b.number.as_deref()))
        });
        let items = entries
            .into_iter()
            .map(|entry| {
                let filename = entry
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let fields = [
                    (MatchField::Number, 4, entry.number.clone().unwrap_or_default()),
                    (MatchField::Title, 3, fold(&entry.title)),
                    (MatchField::Filename, 2, fold(filename)),
                    (MatchField::Body, 1, fold(&entry.plain_slides().join("\n"))),
                ]
                .into_iter()
                .filter(|(_, _, text)| !text.is_empty())
                .map(|(kind, weight, text)| Field::new(kind, weight, text))
                .collect();
                Indexed {
                    label: row_label(&entry),
                    fields,
                    entry,
                }
            })
            .collect();
        Searcher {
            items,
            matcher: Matcher::new(Config::DEFAULT),
        }
    }

    /// Number of indexed hymns.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when no hymns are indexed.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Look up an entry by the `index` carried on a [`SearchHit`].
    pub fn entry(&self, index: usize) -> Option<&Hymn> {
        self.items.get(index).map(|it| &it.entry)
    }

    /// Display label for the entry at `index`, precomputed at construction.
    pub fn label(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(|it| it.label.as_str())
    }

    /// Rank entries against `query`. Empty query returns all entries sorted by
    /// hymn number. Otherwise fuzzy-matches across number, title, filename and
    /// body, keeping the best-scoring field per entry.
    ///
    /// Hits borrow their entry from the searcher (no body cloning per keystroke).
    pub fn search(&mut self, query: &str) -> Vec<SearchHit<'_>> {
        // A query wrapped in double quotes must appear as an exact phrase in
        // the folded body.
        let trimmed = query.trim();
        let (q, exact) = match trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            Some(inner) => (fold(inner), true),
            None => (fold(query), false),
        };
        if q.trim().is_empty() {
            return self
                .items
                .iter()
                .enumerate()
                .map(|(index, it)| SearchHit {
                    index,
                    entry: &it.entry,
                    score: 0,
                    field: MatchField::Number,
                })
                .collect();
        }

        let pattern = Pattern::parse(&q, CaseMatching::Ignore, Normalization::Smart);
        let matcher = &mut self.matcher;
        let mut hits: Vec<SearchHit<'_>> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, it)| {
                if exact
                    && !it.fields.iter().any(|f| f.kind == MatchField::Body && f.folded.contains(&q))
                {
                    return None;
                }
                let best = it
                    .fields
                    .iter()
                    .filter_map(|f| {
                        let raw = pattern.score(f.utf32.slice(..), matcher)?;
                        // Exact/prefix/substring bonuses dominate fuzzy scores so
                        // number "150" always beats a fuzzy hit in some other field.
                        let bonus = if f.folded == q {
                            20_000
                        } else if f.folded.starts_with(&q) {
                            10_000
                        } else if f.folded.contains(&q) {
                            5_000
                        } else {
                            0
                        };
                        Some((f.kind, raw * 10 + bonus + f.weight))
                    })
                    .max_by_key(|(_, score)| *score)?;
                // The hymn whose number is the whole query always leads.
                let number_bonus = if it.entry.number.as_deref() == Some(q.as_str()) {
                    10_000
                } else {
                    0
                };
                Some(SearchHit {
                    index,
                    entry: &it.entry,
                    score: best.1 + number_bonus,
                    field: best.0,
                })
            })
            .collect();
        hits.sort_by_key(|h| std::cmp::Reverse(h.score));
        hits
    }
}

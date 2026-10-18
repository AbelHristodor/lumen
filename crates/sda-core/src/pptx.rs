use crate::model::{Frame, Paragraph, Run, Slide, TextBox};
use crate::theme::{HAlign, Rgba};
use anyhow::{Context, Result};
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;
use zip::ZipArchive;

/// Styled content extracted from a single .pptx, before it becomes a Hymn.
pub struct ParsedPptx {
    pub number: Option<String>,
    pub title: String,
    /// Slides in presentation order.
    pub slides: Vec<Slide>,
}

/// Default slide size in EMU, used when `presentation.xml` is missing.
const DEFAULT_SLIDE_SIZE: (f32, f32) = (9_144_000.0, 6_858_000.0);

/// Extract styled slides from a .pptx file.
///
/// The number is taken from the filename stem. The in slide "Imnul N" text
/// is split across XML runs and is not reliable. The title is the first
/// meaningful line of the first slide, skipping the "Imnul ..." marker and
/// "N/M" counter lines.
pub fn extract(path: &Path) -> Result<ParsedPptx> {
    let number = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(hymn_number_from_stem);

    let file = std::fs::File::open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut zip = ZipArchive::new(file)
        .with_context(|| format!("read zip {}", path.display()))?;

    let slide_size = read_opt(&mut zip, "ppt/presentation.xml")
        .and_then(|xml| parse_slide_size(&xml))
        .unwrap_or(DEFAULT_SLIDE_SIZE);
    let scheme = ColorScheme::load(&mut zip);
    let master_bg = read_opt(&mut zip, "ppt/slideMasters/slideMaster1.xml")
        .and_then(|xml| parse_background(&xml, &scheme));

    // Collect slide XML paths and sort by numeric index in the filename.
    let mut slide_names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .collect();
    slide_names.sort_by_key(|n| slide_index(n));

    let mut slides: Vec<Slide> = Vec::with_capacity(slide_names.len());
    for name in &slide_names {
        let mut xml = String::new();
        zip.by_name(name)?.read_to_string(&mut xml)?;
        let boxes = parse_slide(&xml, &scheme, slide_size)?;

        let rels_name = format!(
            "ppt/slides/_rels/{}.rels",
            name.trim_start_matches("ppt/slides/")
        );
        let rels = read_opt(&mut zip, &rels_name)
            .map(|x| parse_rels(&x))
            .unwrap_or_default();
        let layout_bg = rel_target(&rels, "/slideLayout")
            .and_then(|t| read_opt(&mut zip, &t))
            .and_then(|xml| parse_background(&xml, &scheme));
        let background = parse_background(&xml, &scheme)
            .or(layout_bg)
            .or(master_bg);
        let notes = rel_target(&rels, "/notesSlide")
            .and_then(|t| read_opt(&mut zip, &t))
            .and_then(|xml| notes_text(&xml));

        slides.push(Slide { background, boxes, notes });
    }

    let first_lines: Vec<String> = slides
        .first()
        .map(|s| s.plain_text().lines().map(str::to_string).collect())
        .unwrap_or_default();
    let title = pick_title(&first_lines);
    Ok(ParsedPptx { number, title, slides })
}

/// Read one zip entry as a string. Missing or unreadable entries give `None`.
fn read_opt<R: Read + Seek>(zip: &mut ZipArchive<R>, name: &str) -> Option<String> {
    let mut xml = String::new();
    zip.by_name(name).ok()?.read_to_string(&mut xml).ok()?;
    Some(xml)
}

/// Derive the hymn number from a filename stem, keeping an optional letter
/// suffix: "001" is "1", "150" is "150", "664b" is "664b", "000" is "0".
fn hymn_number_from_stem(stem: &str) -> String {
    let digits: String = stem.chars().take_while(|c| c.is_ascii_digit()).collect();
    let suffix: &str = &stem[digits.len()..];
    let number = digits.trim_start_matches('0');
    // "000" trims to nothing, so fall back to "0".
    let number = if number.is_empty() && !digits.is_empty() {
        "0"
    } else {
        number
    };
    format!("{number}{suffix}")
}

/// Numeric slide index from "ppt/slides/slide12.xml", here 12.
fn slide_index(name: &str) -> u32 {
    name.trim_start_matches("ppt/slides/slide")
        .trim_end_matches(".xml")
        .parse()
        .unwrap_or(u32::MAX)
}

/// Value of the attribute with local name `key`, unescaped.
fn attr(e: &BytesStart, key: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == key)
        .and_then(|a| a.unescape_value().ok())
        .map(|v| v.into_owned())
}

fn attr_f32(e: &BytesStart, key: &[u8]) -> Option<f32> {
    attr(e, key)?.parse().ok()
}

fn attr_u32(e: &BytesStart, key: &[u8]) -> Option<u32> {
    attr(e, key)?.parse().ok()
}

/// Reader that reports self closing tags as a Start followed by an End, so
/// the state machines below only handle Start, End, and Text.
fn reader(xml: &str) -> Reader<&[u8]> {
    let mut r = Reader::from_str(xml);
    r.config_mut().expand_empty_elements = true;
    r
}

/// Scale `c` lightness in HSL space. `lum_mod` and `lum_off` are OOXML
/// percentages where 100000 is 100 percent.
pub fn apply_lum(c: Rgba, lum_mod: Option<u32>, lum_off: Option<u32>) -> Rgba {
    let (h, s, l) = rgb_to_hsl(c);
    let l = (l * lum_mod.map_or(1.0, |m| m as f32 / 100_000.0)
        + lum_off.map_or(0.0, |o| o as f32 / 100_000.0))
    .clamp(0.0, 1.0);
    let (r, g, b) = hsl_to_rgb(h, s, l);
    Rgba::new(r, g, b, c.a)
}

fn rgb_to_hsl(c: Rgba) -> (f32, f32, f32) {
    let r = c.r as f32 / 255.0;
    let g = c.g as f32 / 255.0;
    let b = c.b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let to_byte = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    if s == 0.0 {
        let v = to_byte(l);
        return (v, v, v);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let hue = |mut t: f32| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (
        to_byte(hue(h + 1.0 / 3.0)),
        to_byte(hue(h)),
        to_byte(hue(h - 1.0 / 3.0)),
    )
}

/// Theme colors plus the master color map, built once per file.
#[derive(Default)]
struct ColorScheme {
    /// Theme color name like "dk1" to its color.
    scheme: HashMap<String, Rgba>,
    /// Master mapping like "bg1" to "lt1".
    map: HashMap<String, String>,
}

impl ColorScheme {
    fn load<R: Read + Seek>(zip: &mut ZipArchive<R>) -> Self {
        let scheme = read_opt(zip, "ppt/theme/theme1.xml")
            .map(|xml| parse_clr_scheme(&xml))
            .unwrap_or_default();
        let map = read_opt(zip, "ppt/slideMasters/slideMaster1.xml")
            .map(|xml| parse_clr_map(&xml))
            .unwrap_or_default();
        ColorScheme { scheme, map }
    }

    /// Resolve a scheme color name through the color map, then the theme.
    fn scheme_color(&self, name: &str) -> Option<Rgba> {
        let key = self.map.get(name).map_or(name, String::as_str);
        self.scheme.get(key).copied()
    }
}

/// Parse the first `a:clrScheme` of a theme into name to color.
fn parse_clr_scheme(xml: &str) -> HashMap<String, Rgba> {
    let mut reader = reader(xml);
    let mut out = HashMap::new();
    let mut in_scheme = false;
    let mut current: Option<String> = None;
    let mut buf = Vec::new();
    while let Ok(ev) = reader.read_event_into(&mut buf) {
        match ev {
            Event::Start(e) => match e.local_name().as_ref() {
                b"clrScheme" => in_scheme = true,
                b"srgbClr" if in_scheme => {
                    if let (Some(name), Some(c)) =
                        (&current, attr(&e, b"val").and_then(|v| Rgba::from_hex(&v)))
                    {
                        out.insert(name.clone(), c);
                    }
                }
                b"sysClr" if in_scheme => {
                    if let (Some(name), Some(c)) =
                        (&current, attr(&e, b"lastClr").and_then(|v| Rgba::from_hex(&v)))
                    {
                        out.insert(name.clone(), c);
                    }
                }
                name if in_scheme => {
                    current = Some(String::from_utf8_lossy(name).into_owned());
                }
                _ => {}
            },
            Event::End(e) if e.local_name().as_ref() == b"clrScheme" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

/// Parse the `p:clrMap` attributes of a slide master.
fn parse_clr_map(xml: &str) -> HashMap<String, String> {
    let mut reader = reader(xml);
    let mut buf = Vec::new();
    while let Ok(ev) = reader.read_event_into(&mut buf) {
        match ev {
            Event::Start(e) if e.local_name().as_ref() == b"clrMap" => {
                return e
                    .attributes()
                    .flatten()
                    .filter_map(|a| {
                        let k = String::from_utf8_lossy(a.key.local_name().as_ref()).into_owned();
                        let v = a.unescape_value().ok()?.into_owned();
                        Some((k, v))
                    })
                    .collect();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    HashMap::new()
}

/// Slide size in EMU from `p:sldSz`.
fn parse_slide_size(xml: &str) -> Option<(f32, f32)> {
    let mut reader = reader(xml);
    let mut buf = Vec::new();
    while let Ok(ev) = reader.read_event_into(&mut buf) {
        match ev {
            Event::Start(e) if e.local_name().as_ref() == b"sldSz" => {
                return Some((attr_f32(&e, b"cx")?, attr_f32(&e, b"cy")?));
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// Type and Target pairs of a `.rels` part.
fn parse_rels(xml: &str) -> Vec<(String, String)> {
    let mut reader = reader(xml);
    let mut out = Vec::new();
    let mut buf = Vec::new();
    while let Ok(ev) = reader.read_event_into(&mut buf) {
        match ev {
            Event::Start(e) if e.local_name().as_ref() == b"Relationship" => {
                if let (Some(t), Some(target)) = (attr(&e, b"Type"), attr(&e, b"Target")) {
                    out.push((t, target));
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

/// Zip path of the first relationship whose Type ends with `suffix`. Targets
/// are relative to `ppt/slides/`, so `../x` becomes `ppt/x`.
fn rel_target(rels: &[(String, String)], suffix: &str) -> Option<String> {
    let (_, target) = rels.iter().find(|(t, _)| t.ends_with(suffix))?;
    Some(match target.strip_prefix("../") {
        Some(rest) => format!("ppt/{rest}"),
        None => match target.strip_prefix('/') {
            Some(abs) => abs.to_string(),
            None => format!("ppt/slides/{target}"),
        },
    })
}

/// Color pieces seen inside one `a:solidFill`, resolved once the fill ends.
#[derive(Default)]
struct ColorAcc {
    rgb: Option<Rgba>,
    scheme: Option<String>,
    lum_mod: Option<u32>,
    lum_off: Option<u32>,
}

impl ColorAcc {
    fn on_start(&mut self, e: &BytesStart) {
        match e.local_name().as_ref() {
            b"srgbClr" => self.rgb = attr(e, b"val").and_then(|v| Rgba::from_hex(&v)),
            b"schemeClr" => self.scheme = attr(e, b"val"),
            b"lumMod" => self.lum_mod = attr_u32(e, b"val"),
            b"lumOff" => self.lum_off = attr_u32(e, b"val"),
            _ => {}
        }
    }

    fn resolve(&self, scheme: &ColorScheme) -> Option<Rgba> {
        let base = self
            .rgb
            .or_else(|| scheme.scheme_color(self.scheme.as_deref()?))?;
        if self.lum_mod.is_none() && self.lum_off.is_none() {
            return Some(base);
        }
        Some(apply_lum(base, self.lum_mod, self.lum_off))
    }
}

/// Solid background color of a slide, layout, or master part from `p:bg`.
fn parse_background(xml: &str, scheme: &ColorScheme) -> Option<Rgba> {
    let mut reader = reader(xml);
    let mut buf = Vec::new();
    let mut in_bg = false;
    let mut in_fill = false;
    let mut acc = ColorAcc::default();
    while let Ok(ev) = reader.read_event_into(&mut buf) {
        match ev {
            Event::Start(e) => match e.local_name().as_ref() {
                b"bg" => in_bg = true,
                b"solidFill" if in_bg => in_fill = true,
                _ if in_fill => acc.on_start(&e),
                _ => {}
            },
            Event::End(e) => match e.local_name().as_ref() {
                b"bg" => return acc.resolve(scheme),
                b"solidFill" => in_fill = false,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// Speaker notes: one line per paragraph, chrome and empty lines dropped.
fn notes_text(xml: &str) -> Option<String> {
    let mut reader = reader(xml);
    let mut lines: Vec<String> = Vec::new();
    let mut in_t = false;
    let mut current = String::new();
    let mut buf = Vec::new();
    while let Ok(ev) = reader.read_event_into(&mut buf) {
        match ev {
            Event::Start(e) => match e.local_name().as_ref() {
                b"p" => current.clear(),
                b"t" => in_t = true,
                _ => {}
            },
            Event::End(e) => match e.local_name().as_ref() {
                b"p" => {
                    let line = current.trim();
                    if !line.is_empty() && !is_chrome(line) {
                        lines.push(line.to_string());
                    }
                    current.clear();
                }
                b"t" => in_t = false,
                _ => {}
            },
            Event::Text(t) if in_t => {
                if let Ok(s) = t.unescape() {
                    current.push_str(&s);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Parser state for one slide part.
#[derive(Default)]
struct SlideWalk {
    boxes: Vec<TextBox>,
    text_box: Option<TextBox>,
    para: Option<Paragraph>,
    run: Option<Run>,
    off: Option<(f32, f32)>,
    ext: Option<(f32, f32)>,
    in_xfrm: bool,
    in_rpr: bool,
    in_ln: bool,
    in_fill: bool,
    in_ln_spc: bool,
    in_t: bool,
    color: ColorAcc,
}

fn paragraph_text(p: &Paragraph) -> String {
    p.runs.iter().map(|r| r.text.as_str()).collect::<String>().trim().to_string()
}

/// Parse the text boxes of one slide. Frames are fractions of `slide_size`.
fn parse_slide(xml: &str, scheme: &ColorScheme, slide_size: (f32, f32)) -> Result<Vec<TextBox>> {
    // Text events are not trimmed. Spaces at run edges matter when joining.
    let mut reader = reader(xml);
    let mut w = SlideWalk::default();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => slide_start(&mut w, &e),
            Event::End(e) => slide_end(&mut w, &e, scheme, slide_size),
            Event::Text(t) if w.in_t => {
                if let Some(run) = w.run.as_mut() {
                    run.text.push_str(&t.unescape()?);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(w.boxes)
}

fn slide_start(w: &mut SlideWalk, e: &BytesStart) {
    let name = e.local_name();
    match name.as_ref() {
        b"sp" => {
            w.text_box = Some(TextBox { frame: None, paragraphs: Vec::new() });
            w.off = None;
            w.ext = None;
        }
        b"xfrm" if w.text_box.is_some() => w.in_xfrm = true,
        b"off" if w.in_xfrm => w.off = attr_f32(e, b"x").zip(attr_f32(e, b"y")),
        b"ext" if w.in_xfrm => w.ext = attr_f32(e, b"cx").zip(attr_f32(e, b"cy")),
        b"p" if w.text_box.is_some() => {
            w.para = Some(Paragraph { align: None, line_spacing: None, runs: Vec::new() });
        }
        b"pPr" if w.para.is_some() => {
            let align = match attr(e, b"algn").as_deref() {
                Some("l") => Some(HAlign::Left),
                Some("ctr") => Some(HAlign::Center),
                Some("r") => Some(HAlign::Right),
                _ => None,
            };
            if let Some(p) = w.para.as_mut() {
                p.align = align;
            }
        }
        b"lnSpc" if w.para.is_some() => w.in_ln_spc = true,
        b"spcPct" if w.in_ln_spc => {
            if let (Some(p), Some(v)) = (w.para.as_mut(), attr_f32(e, b"val")) {
                p.line_spacing = Some(v / 100_000.0);
            }
        }
        b"r" | b"fld" if w.para.is_some() => w.run = Some(Run::default()),
        b"rPr" if w.run.is_some() => {
            w.in_rpr = true;
            w.color = ColorAcc::default();
            if let Some(run) = w.run.as_mut() {
                run.size_pt = attr_f32(e, b"sz").map(|s| s / 100.0);
                run.bold = attr(e, b"b").map(|b| b == "1" || b == "true");
                run.italic = attr(e, b"i").map(|i| i == "1" || i == "true");
                run.underline = attr(e, b"u").map(|u| u != "none");
            }
        }
        b"ln" if w.in_rpr => w.in_ln = true,
        b"solidFill" if w.in_rpr && !w.in_ln => w.in_fill = true,
        b"latin" if w.in_rpr => {
            if let Some(run) = w.run.as_mut() {
                run.font = attr(e, b"typeface");
            }
        }
        b"t" if w.run.is_some() => w.in_t = true,
        _ if w.in_fill => w.color.on_start(e),
        _ => {}
    }
}

fn slide_end(w: &mut SlideWalk, e: &BytesEnd, scheme: &ColorScheme, slide_size: (f32, f32)) {
    let name = e.local_name();
    match name.as_ref() {
        b"sp" => {
            if let Some(b) = w.text_box.take() {
                let has_text = b.paragraphs.iter().any(|p| !paragraph_text(p).is_empty());
                let all_chrome = b.paragraphs.iter().all(|p| is_chrome(&paragraph_text(p)));
                if has_text && !all_chrome {
                    w.boxes.push(b);
                }
            }
        }
        b"xfrm" => {
            w.in_xfrm = false;
            if let (Some(b), Some((x, y)), Some((cx, cy))) = (w.text_box.as_mut(), w.off, w.ext) {
                b.frame = Some(Frame {
                    x: x / slide_size.0,
                    y: y / slide_size.1,
                    w: cx / slide_size.0,
                    h: cy / slide_size.1,
                });
            }
        }
        b"p" => {
            if let (Some(b), Some(p)) = (w.text_box.as_mut(), w.para.take()) {
                if !paragraph_text(&p).is_empty() {
                    b.paragraphs.push(p);
                }
            }
        }
        b"lnSpc" => w.in_ln_spc = false,
        b"r" | b"fld" => {
            if let (Some(p), Some(run)) = (w.para.as_mut(), w.run.take()) {
                if !run.text.is_empty() {
                    p.runs.push(run);
                }
            }
        }
        b"rPr" => {
            w.in_rpr = false;
            if let Some(run) = w.run.as_mut() {
                run.color = w.color.resolve(scheme);
            }
        }
        b"ln" => w.in_ln = false,
        b"solidFill" => w.in_fill = false,
        b"t" => w.in_t = false,
        _ => {}
    }
}

/// Pick the title: first line that is not the "Imnul" marker and not a "N/M"
/// counter, falling back to the first line of any kind.
fn pick_title(lines: &[String]) -> String {
    lines
        .iter()
        .find(|l| !is_marker(l))
        .or_else(|| lines.first())
        .cloned()
        .unwrap_or_default()
}

/// True for "Imnul ...", a bare number, or a "N/M" counter like "1/300".
fn is_marker(line: &str) -> bool {
    let l = line.trim();
    l.starts_with("Imnul")
        || l.chars().all(|c| c.is_ascii_digit())
        || (l.contains('/')
            && l.chars().all(|c| c.is_ascii_digit() || c == '/'))
}

/// True for slide chrome that should never appear in the preview or search
/// index: the "IMNURI CREȘTINE 2013" footer present on every slide, plus the
/// markers caught by [`is_marker`]. Lyrics never match these.
fn is_chrome(line: &str) -> bool {
    let l = line.trim();
    let lower = l.to_lowercase();
    lower.starts_with("imnuri creștine")
        || lower.starts_with("imnuri crestine")
        || is_marker(l)
        || is_standalone_number(l)
}

/// True for a line that is only a number with an optional trailing dot, like
/// "1" or "2.", the per slide verse number. A line like "1. Ca un cerb" is
/// kept because lyrics follow the number.
fn is_standalone_number(line: &str) -> bool {
    let l = line.trim().trim_end_matches('.');
    !l.is_empty() && l.chars().all(|c| c.is_ascii_digit())
}

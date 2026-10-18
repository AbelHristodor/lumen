use sda_core::model::Run;
use sda_core::pptx::{self, extract};
use sda_core::theme::Rgba;
use std::path::Path;

/// Plain text of every slide joined by a newline.
fn body(parsed: &pptx::ParsedPptx) -> String {
    parsed.slides.iter().map(|s| s.plain_text()).collect::<Vec<_>>().join("\n")
}

#[test]
fn extracts_number_title_and_body() {
    let path = Path::new("tests/fixtures/001.pptx");
    let parsed = extract(path).expect("should parse");

    // Number comes from the filename stem.
    assert_eq!(parsed.number.as_deref(), Some("1"));
    // Title is the first meaningful line of the title slide.
    assert!(parsed.title.contains("Plecaţi-vă lui Dumnezeu"));
    // Body contains verse text from later slides.
    assert!(body(&parsed).contains("Popoare-oriunde"));
    // The "Imnul" marker and counter lines are not the title.
    assert!(!parsed.title.starts_with("Imnul"));
}

#[test]
fn number_from_three_digit_filename() {
    let parsed = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    assert_eq!(parsed.number.as_deref(), Some("150"));
    assert!(parsed.title.contains("Cerul, pământul"));
}

#[test]
fn number_supports_letter_suffix() {
    // Hymns like 664a / 664b have a trailing letter on the filename stem; the
    // number must be preserved verbatim, not dropped.
    let parsed = extract(Path::new("tests/fixtures/664b.pptx")).unwrap();
    assert_eq!(parsed.number.as_deref(), Some("664b"));
    assert!(parsed.title.contains("aducem"));
}

#[test]
fn title_joins_runs_split_within_a_paragraph() {
    // Hymn 356's title is split across four <a:t> runs in one <a:p>:
    // "Ca un " + "cerb" + " setos de " + "ape". The parser must join runs
    // within a paragraph so the full title survives, not just "Ca un".
    let parsed = extract(Path::new("tests/fixtures/356.pptx")).unwrap();
    assert_eq!(parsed.number.as_deref(), Some("356"));
    assert_eq!(parsed.title, "Ca un cerb setos de ape");
    assert!(!parsed.title.starts_with("Imnul"));
}

#[test]
fn preserves_text_per_slide() {
    let parsed = extract(Path::new("tests/fixtures/001.pptx")).unwrap();
    // Hymn 1 has 5 slides.
    assert_eq!(parsed.slides.len(), 5);
    // The first slide holds the title.
    assert!(parsed.slides[0].plain_text().contains("Plecaţi-vă lui Dumnezeu"));
}

#[test]
fn strips_slide_chrome_footer_counter_and_imnul() {
    // Each slide carries chrome that should not appear in the preview/search:
    // the "IMNURI CREȘTINE 2013" footer, a "X/920" (or "X/300") counter, and
    // on the first slide the "Imnul X" marker.
    let parsed = extract(Path::new("tests/fixtures/001.pptx")).unwrap();
    let all = body(&parsed);
    assert!(!all.contains("IMNURI CREȘTINE"), "footer must be stripped");
    assert!(!all.contains("Imnul"), "'Imnul X' marker must be stripped");
    // No bare "N/M" counter lines remain.
    assert!(
        !parsed.slides.iter().any(|s| s.plain_text().lines().any(is_counter_line)),
        "counter lines like '1/300' must be stripped"
    );
    // Real lyrics survive.
    assert!(all.contains("Popoare-oriunde"));
    // The title is still intact on slide 0.
    assert!(parsed.slides[0].plain_text().contains("Plecaţi-vă lui Dumnezeu"));
}

/// A line that is only digits and slashes, e.g. "1/300" or "150/920".
fn is_counter_line(line: &str) -> bool {
    let l = line.trim();
    !l.is_empty() && l.contains('/') && l.chars().all(|c| c.is_ascii_digit() || c == '/')
}

#[test]
fn strips_standalone_slide_numbers_but_keeps_numbered_lyrics() {
    // Hymn 1's slides carry a standalone slide-number line ("1.", "2.", …).
    let one = extract(Path::new("tests/fixtures/001.pptx")).unwrap();
    for slide in &one.slides {
        for line in slide.plain_text().lines() {
            let l = line.trim();
            let standalone_number = !l.is_empty()
                && l.trim_end_matches('.').chars().all(|c| c.is_ascii_digit())
                && l.trim_end_matches('.').chars().next().is_some();
            assert!(
                !standalone_number,
                "standalone slide number {l:?} must be stripped"
            );
        }
    }
    // Hymn 356's verse lines START with "1. " followed by lyrics — keep those.
    let h356 = extract(Path::new("tests/fixtures/356.pptx")).unwrap();
    let body = body(&h356);
    assert!(
        body.contains("Ca un cerb setos de ape,"),
        "numbered lyric lines must survive"
    );
}

#[test]
fn slide_runs_keep_color_size_and_bold() {
    let p = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    let slide = &p.slides[1];
    let runs: Vec<&Run> = slide.boxes[0].paragraphs[0].runs.iter().collect();
    assert_eq!(runs[0].text, "1");
    assert_eq!(runs[0].color, Some(Rgba::new(0xFF, 0xDD, 0x4B, 255)));
    assert_eq!(runs[0].size_pt, Some(40.0));
    assert_eq!(runs[0].bold, Some(true));
    assert_eq!(runs[0].font.as_deref(), Some("Georgia"));
    let white = runs.iter().find(|r| r.text.contains("Cerul")).unwrap();
    // schemeClr bg1 maps to lt1 which is FFFFFF.
    assert_eq!(white.color, Some(Rgba::new(255, 255, 255, 255)));
    assert_eq!(white.bold, None);
}

#[test]
fn slide_background_comes_from_master() {
    let p = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    // tx1 maps to dk1 which is 000000.
    assert_eq!(p.slides[1].background, Some(Rgba::new(0, 0, 0, 255)));
}

#[test]
fn text_box_frame_is_fraction_of_slide() {
    let p = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    let f = p.slides[1].boxes[0].frame.unwrap();
    assert!((f.x - 838200.0 / 9144000.0).abs() < 1e-4);
    assert!((f.y - 417255.0 / 6858000.0).abs() < 1e-4);
    assert!((f.w - 8305800.0 / 9144000.0).abs() < 1e-4);
}

#[test]
fn chrome_boxes_are_dropped() {
    let p = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    let all = p.slides[1].plain_text();
    assert!(!all.contains("IMNURI"));
    assert!(!all.contains("150/920"));
    assert_eq!(p.slides[1].boxes.len(), 1);
}

#[test]
fn lum_mod_darkens_scheme_color() {
    assert_eq!(
        pptx::apply_lum(Rgba::new(255, 255, 255, 255), Some(50000), None),
        Rgba::new(128, 128, 128, 255)
    );
}

#[test]
fn paragraph_alignment_and_notes_are_read() {
    // A minimal deck with no theme or master: one right aligned run, 1.5 line
    // spacing, and a notes slide linked through the slide rels.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("7.pptx");
    let file = std::fs::File::create(&path).unwrap();
    let mut z = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    let mut put = |name: &str, body: &str| {
        z.start_file(name, opts).unwrap();
        std::io::Write::write_all(&mut z, body.as_bytes()).unwrap();
    };
    put(
        "ppt/presentation.xml",
        r#"<p:presentation xmlns:p="p"><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#,
    );
    put(
        "ppt/slides/slide1.xml",
        r#"<p:sld xmlns:a="a" xmlns:p="p"><p:cSld><p:spTree><p:sp><p:txBody>
        <a:p><a:pPr algn="r"><a:lnSpc><a:spcPct val="150000"/></a:lnSpc></a:pPr>
        <a:r><a:rPr i="1" u="sng"/><a:t>Hello</a:t></a:r></a:p>
        </p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#,
    );
    put(
        "ppt/slides/_rels/slide1.xml.rels",
        r#"<Relationships xmlns="r"><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide1.xml"/></Relationships>"#,
    );
    put(
        "ppt/notesSlides/notesSlide1.xml",
        r#"<p:notes xmlns:a="a" xmlns:p="p"><p:cSld><p:spTree><p:sp><p:txBody>
        <a:p><a:r><a:t>Sing it</a:t></a:r><a:r><a:t> softly</a:t></a:r></a:p>
        <a:p><a:r><a:t>Twice</a:t></a:r></a:p>
        </p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#,
    );
    z.finish().unwrap();

    let p = extract(&path).unwrap();
    assert_eq!(p.number.as_deref(), Some("7"));
    assert_eq!(p.title, "Hello");
    let slide = &p.slides[0];
    assert_eq!(slide.background, None);
    let para = &slide.boxes[0].paragraphs[0];
    assert_eq!(para.align, Some(sda_core::theme::HAlign::Right));
    assert_eq!(para.line_spacing, Some(1.5));
    assert_eq!(slide.boxes[0].frame, None);
    assert_eq!(para.runs[0].italic, Some(true));
    assert_eq!(para.runs[0].underline, Some(true));
    assert_eq!(para.runs[0].color, None);
    assert_eq!(slide.notes.as_deref(), Some("Sing it softly
Twice"));
}

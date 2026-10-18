//! As-is slide rendering. Finds LibreOffice or PowerPoint on this machine
//! and turns a pptx file into one png per slide so the app can show the
//! original design instead of its own text layout.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The external program that converts pptx files.
#[derive(Debug, Clone, PartialEq)]
pub enum Converter {
    /// Path to the soffice binary.
    LibreOffice(PathBuf),
    /// Path to the PowerPoint app bundle or exe.
    PowerPoint(PathBuf),
}

/// Find a converter using the PATH variable and well known install paths.
pub fn detect() -> Option<Converter> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let candidates: Vec<PathBuf> = if cfg!(target_os = "macos") {
        vec![
            "/Applications/LibreOffice.app/Contents/MacOS/soffice".into(),
            "/Applications/Microsoft PowerPoint.app".into(),
        ]
    } else if cfg!(windows) {
        vec![
            r"C:\Program Files\LibreOffice\program\soffice.exe".into(),
            r"C:\Program Files\Microsoft Office\root\Office16\POWERPNT.EXE".into(),
        ]
    } else {
        vec!["/usr/bin/soffice".into()]
    };
    detect_in(&path_var, &candidates)
}

/// Candidates that exist win, then each PATH entry joined with soffice.
pub fn detect_in(path_var: &str, candidates: &[PathBuf]) -> Option<Converter> {
    let soffice = if cfg!(windows) {
        "soffice.exe"
    } else {
        "soffice"
    };
    let from_candidates = candidates.iter().find(|p| p.exists()).map(|p| {
        let s = p.to_string_lossy().to_uppercase();
        if s.ends_with(".APP") || s.contains("POWERPNT") {
            Converter::PowerPoint(p.clone())
        } else {
            Converter::LibreOffice(p.clone())
        }
    });
    from_candidates.or_else(|| {
        std::env::split_paths(path_var)
            .map(|dir| dir.join(soffice))
            .find(|p| p.is_file())
            .map(Converter::LibreOffice)
    })
}

/// Cache folder for one pptx at one output size. Any change to the file
/// path, its mtime, or the size gives a new folder.
pub fn cache_dir_for(root: &Path, pptx: &Path, mtime: i64, size: (u32, u32)) -> PathBuf {
    let mut h = Sha256::new();
    h.update(pptx.to_string_lossy().as_bytes());
    h.update(mtime.to_le_bytes());
    h.update(size.0.to_le_bytes());
    h.update(size.1.to_le_bytes());
    let hex: String = h.finalize()[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    root.join(hex)
}

/// The png files in a finished cache folder, sorted by slide number.
/// None when the folder has no .done marker.
pub fn cached(dir: &Path) -> Option<Vec<PathBuf>> {
    if !dir.join(".done").is_file() {
        return None;
    }
    Some(sorted_pngs(dir))
}

fn sorted_pngs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<(u32, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .filter_map(|p| {
            let n = p.file_stem()?.to_str()?.parse().ok()?;
            Some((n, p))
        })
        .collect();
    v.sort();
    v.into_iter().map(|(_, p)| p).collect()
}

/// Render every slide of `pptx` into `out_dir/<n>.png` at `size` and write
/// a .done marker last. Returns the sorted png paths.
pub fn render(
    conv: &Converter,
    pptx: &Path,
    out_dir: &Path,
    size: (u32, u32),
) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(out_dir)?;
    match conv {
        Converter::LibreOffice(exe) => render_libreoffice(exe, pptx, out_dir, size)?,
        Converter::PowerPoint(_) => render_powerpoint(pptx, out_dir, size)?,
    }
    let pngs = sorted_pngs(out_dir);
    if pngs.is_empty() {
        bail!("converter wrote no slides for {}", pptx.display());
    }
    std::fs::write(out_dir.join(".done"), "")?;
    Ok(pngs)
}

fn run(cmd: &mut Command) -> Result<()> {
    let out = cmd
        .output()
        .with_context(|| format!("spawn {:?}", cmd.get_program()))?;
    if !out.status.success() {
        bail!(
            "{:?} failed: {}",
            cmd.get_program(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn render_libreoffice(exe: &Path, pptx: &Path, out_dir: &Path, size: (u32, u32)) -> Result<()> {
    run(Command::new(exe)
        .args(["--headless", "--convert-to", "pdf", "--outdir"])
        .arg(out_dir)
        .arg(pptx))?;
    let stem = pptx.file_stem().context("pptx has no file name")?;
    let pdf_path = out_dir.join(stem).with_extension("pdf");
    pdf_to_pngs(&pdf_path, out_dir, size)?;
    std::fs::remove_file(pdf_path).ok();
    Ok(())
}

/// Rasterize each page of the PDF to out_dir/<n>.png with width size.0.
fn pdf_to_pngs(pdf_path: &Path, out_dir: &Path, size: (u32, u32)) -> Result<()> {
    let bytes = std::fs::read(pdf_path)?;
    // hayro 0.8 exposes the document as hayro::hayro_syntax::Pdf and a free
    // hayro::render function that returns a vello Pixmap with its own png
    // encoder, so the image crate is not needed on this path.
    let pdf = hayro::hayro_syntax::Pdf::new(bytes).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let cache = hayro::RenderCache::new();
    let interp = hayro::hayro_interpret::InterpreterSettings::default();
    let settings = hayro::RenderSettings::default();
    for (i, page) in pdf.pages().iter().enumerate() {
        let (w, _) = page.render_dimensions();
        let scale = size.0 as f32 / w.max(1.0);
        let pix = hayro::render(
            page,
            &cache,
            &interp,
            &settings,
            &hayro::PixmapSettings {
                x_scale: scale,
                y_scale: scale,
                bg_color: hayro::vello_cpu::color::palette::css::WHITE,
            },
        );
        let png = pix.into_png().map_err(|e| anyhow::anyhow!("{e}"))?;
        std::fs::write(out_dir.join(format!("{}.png", i + 1)), png)?;
    }
    std::fs::remove_file(pdf_path).ok();
    Ok(())
}

fn render_powerpoint(pptx: &Path, out_dir: &Path, size: (u32, u32)) -> Result<()> {
    let pptx = pptx.canonicalize()?;
    if cfg!(target_os = "macos") {
        // Checked on PowerPoint 16.x for Mac. The app is sandboxed, so a
        // save to any folder outside its container pops a Grant File Access
        // dialog and then does nothing. Inside the container no dialog shows.
        // There save as PNG also writes nothing, while save as PDF works.
        // So we save a PDF into the container tmp dir and rasterize it.
        let home = std::env::var("HOME").context("HOME not set")?;
        let scratch = PathBuf::from(home)
            .join("Library/Containers/com.microsoft.Powerpoint/Data/tmp/sda-asis");
        std::fs::create_dir_all(&scratch)?;
        let pdf_path = scratch.join(format!("{}.pdf", std::process::id()));
        let script = format!(
            r#"tell application "Microsoft PowerPoint"
    open POSIX file "{}"
    set p to active presentation
    save p in POSIX file "{}" as save as PDF
    close p saving no
end tell"#,
            esc(&pptx),
            esc(&pdf_path)
        );
        run(Command::new("osascript").arg("-e").arg(script))?;
        pdf_to_pngs(&pdf_path, out_dir, size)?;
        std::fs::remove_file(pdf_path).ok();
    } else {
        let script = format!(
            r#"$pp = New-Object -ComObject PowerPoint.Application
$pres = $pp.Presentations.Open("{}", $true, $false, $false)
$i = 1
foreach ($s in $pres.Slides) {{ $s.Export("{}\$i.png", "PNG", {}, {}); $i++ }}
$pres.Close(); $pp.Quit()"#,
            pptx.display(),
            out_dir.display(),
            size.0,
            size.1
        );
        run(Command::new("powershell").args(["-NoProfile", "-Command", &script]))?;
    }
    Ok(())
}

fn esc(p: &Path) -> String {
    p.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

//! Manual check for the as-is renderer.
//! Usage: cargo run -p sda-core --example asis_probe -- <file.pptx> <out_dir>

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let pptx = std::path::PathBuf::from(args.next().expect("pptx path"));
    let out = std::path::PathBuf::from(args.next().expect("output dir"));
    let conv = sda_core::asis::detect().ok_or_else(|| anyhow::anyhow!("no converter found"))?;
    println!("converter: {conv:?}");
    for p in sda_core::asis::render(&conv, &pptx, &out, (1920, 1080))? {
        println!("{}", p.display());
    }
    Ok(())
}

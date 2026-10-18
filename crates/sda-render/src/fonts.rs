//! Bundled fonts. Inter, Noto Serif, and Lora are OFL. Roboto Slab is
//! Apache 2.0. The license texts live next to the ttf files.

/// Family name and font bytes for every bundled face.
pub fn fonts() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("Inter", include_bytes!("../fonts/Inter-Regular.ttf")),
        ("Inter", include_bytes!("../fonts/Inter-Bold.ttf")),
        (
            "Noto Serif",
            include_bytes!("../fonts/NotoSerif-Regular.ttf"),
        ),
        ("Noto Serif", include_bytes!("../fonts/NotoSerif-Bold.ttf")),
        ("Lora", include_bytes!("../fonts/Lora-Regular.ttf")),
        ("Lora", include_bytes!("../fonts/Lora-Bold.ttf")),
        (
            "Roboto Slab",
            include_bytes!("../fonts/RobotoSlab-Regular.ttf"),
        ),
        (
            "Roboto Slab",
            include_bytes!("../fonts/RobotoSlab-Bold.ttf"),
        ),
    ]
}

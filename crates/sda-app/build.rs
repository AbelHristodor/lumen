fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icon resource failed: {e}");
        }
    }
}

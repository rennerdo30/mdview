//! Build script for mdview
//!
//! On Windows, this embeds the application icon into the executable.

fn main() {
    // Only run on Windows
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "mdview");
        res.set("FileDescription", "A fast, cross-platform GUI markdown viewer");
        res.set("LegalCopyright", "Copyright (c) mdview contributors");
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resources: {}", e);
        }
    }
}

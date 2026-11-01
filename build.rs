// Embeds the application icon in the .exe (Windows only). If the resource compiler is not available the build still succeeds,
// the .exe just has no icon.

#[path = "src/app_icon.rs"]
#[allow(dead_code)]
mod app_icon;

fn main() {
    println!("cargo:rerun-if-changed=src/app_icon.rs");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // PNG at 256x256, wrapped in a one-image .ico
    let size = 256u32;
    let img = image::RgbaImage::from_raw(size, size, app_icon::rgba(size)).unwrap();
    let mut png = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let png = png.into_inner();

    let mut ico = vec![0u8, 0, 1, 0, 1, 0];
    ico.extend([0u8, 0, 0, 0, 1, 0, 32, 0]); // 256x256 (stored as 0), no palette, 1 plane, 32 bpp
    ico.extend((png.len() as u32).to_le_bytes());
    ico.extend(22u32.to_le_bytes());
    ico.extend(png);
    let path = out.join("app.ico");
    std::fs::write(&path, ico).unwrap();

    let mut res = winresource::WindowsResource::new();
    res.set_icon(path.to_str().unwrap());
    res.set("ProductName", "ClassicLauncher");
    res.set("FileDescription", "ClassicLauncher");
    if let Err(e) = res.compile() {
        println!("cargo:warning=could not embed the icon: {e}");
    }
}

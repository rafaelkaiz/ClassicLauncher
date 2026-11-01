//! The application icon: the ClassicLauncher logo (`assets/logo/classiclauncher-icon.svg`, drawn once to a 256 pixel PNG that is
//! embedded here), brought to the size asked for. Used by the window and, through `build.rs`, by the .exe.

const PNG: &[u8] = include_bytes!("../assets/logo/classiclauncher-icon-256.png");

/// `size` x `size` RGBA pixels (straight alpha).
pub fn rgba(size: u32) -> Vec<u8> {
    let img = image::load_from_memory_with_format(PNG, image::ImageFormat::Png).expect("the embedded icon").to_rgba8();
    if img.width() == size && img.height() == size {
        return img.into_raw();
    }
    image::imageops::resize(&img, size, size, image::imageops::FilterType::Lanczos3).into_raw()
}

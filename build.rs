//! Embeds the application icon (`assets/icon.png`) into the Windows executable resources
//! so it appears in Explorer, the taskbar, and the Properties dialog.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.png");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_icon();
    }
}

fn embed_windows_icon() {
    use image::ExtendedColorType;
    use image::codecs::ico::{IcoEncoder, IcoFrame};

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");

    let source = image::open(std::path::Path::new(&manifest_dir).join("assets/icon.png"))
        .expect("failed to open assets/icon.png")
        .to_rgba8();

    const SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];
    let mut frames = Vec::with_capacity(SIZES.len());
    for &size in &SIZES {
        let resized =
            image::imageops::resize(&source, size, size, image::imageops::FilterType::Lanczos3);
        frames.push(
            IcoFrame::as_png(&resized, size, size, ExtendedColorType::Rgba8)
                .unwrap_or_else(|err| panic!("failed to encode icon frame {size}x{size}: {err}")),
        );
    }

    let ico_path = std::path::Path::new(&out_dir).join("icon.ico");
    let file =
        std::fs::File::create(&ico_path).expect("failed to create .ico in build output directory");

    IcoEncoder::new(file)
        .encode_images(&frames)
        .expect("failed to assemble .ico from frames");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico_path.to_str().expect("icon path is not valid UTF-8"));
    res.compile()
        .expect("failed to embed icon into executable (rc.exe/llvm-rc must be in PATH)");
}

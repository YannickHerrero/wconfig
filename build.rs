fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ico_path = out_dir.join("wconfig.ico");

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for &size in &[16u32, 32, 48, 64, 128, 256] {
        let rgba = generate_icon_rgba(size);
        let image = ico::IconImage::from_rgba_data(size, size, rgba);
        icon_dir.add_entry(ico::IconDirEntry::encode(&image).expect("encode ico entry"));
    }
    let file = std::fs::File::create(&ico_path).expect("create ico");
    icon_dir.write(file).expect("write ico");

    let mut res = winres::WindowsResource::new();
    res.set_icon(ico_path.to_str().expect("ico path utf-8"));

    if std::env::consts::OS != "windows" && target.ends_with("windows-gnu") {
        res.set_windres_path("x86_64-w64-mingw32-windres");
        res.set_ar_path("x86_64-w64-mingw32-ar");
    }

    res.compile().expect("compile windows resource");

    println!("cargo:rerun-if-changed=build.rs");
}

fn generate_icon_rgba(size: u32) -> Vec<u8> {
    // wconfig icon: same Paper palette as wmenu, square inside circle to differentiate.
    const ACCENT: [u8; 4] = [0xB5, 0x59, 0x3A, 0xFF];
    const PAPER: [u8; 4] = [0xF4, 0xEB, 0xD9, 0xFF];
    const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];

    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    let cx = (s - 1.0) / 2.0;
    let cy = (s - 1.0) / 2.0;
    let outer_r = s * 0.48;
    let inner_half = s * 0.20;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            let i = ((y * size + x) * 4) as usize;
            let in_square = dx.abs() <= inner_half && dy.abs() <= inner_half;
            let pixel = if in_square {
                PAPER
            } else if d <= outer_r {
                ACCENT
            } else {
                TRANSPARENT
            };
            rgba[i..i + 4].copy_from_slice(&pixel);
        }
    }
    rgba
}

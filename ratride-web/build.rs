use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let md_path = env::var("RATRIDE_SLIDE_FILE").expect("RATRIDE_SLIDE_FILE must be set");
    let md_dir = Path::new(&md_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let md_content = fs::read_to_string(&md_path).expect("failed to read slide file");

    println!("cargo:rerun-if-changed={}", md_path);

    // Scan for ![...](path) image references
    let mut image_paths = Vec::new();
    for line in md_content.lines() {
        let mut rest = line;
        while let Some(start) = rest.find("![") {
            rest = &rest[start + 2..];
            if let Some(bracket) = rest.find("](") {
                rest = &rest[bracket + 2..];
                if let Some(paren) = rest.find(')') {
                    let path = rest[..paren].trim();
                    if !path.is_empty() {
                        image_paths.push(path.to_string());
                    }
                    rest = &rest[paren + 1..];
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // Generate embedded_images.rs
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("embedded_images.rs");

    let mut code = String::new();
    code.push_str("fn get_embedded_image(path: &str) -> Option<&'static str> {\n");
    code.push_str("    match path {\n");

    for img_path in &image_paths {
        if img_path.starts_with("http://") || img_path.starts_with("https://") {
            code.push_str(&format!(
                "        {p:?} => Some({p:?}),\n",
                p = img_path
            ));
        } else {
            let resolved = md_dir.join(img_path);
            if let Ok(canonical) = resolved.canonicalize() {
                println!("cargo:rerun-if-changed={}", canonical.display());
                let bytes = fs::read(&canonical)
                    .unwrap_or_else(|e| panic!("failed to read image {}: {e}", canonical.display()));
                let mime = match canonical
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "svg" => "image/svg+xml",
                    "webp" => "image/webp",
                    _ => "application/octet-stream",
                };
                let b64 = {
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD.encode(&bytes)
                };
                let data_uri = format!("data:{mime};base64,{b64}");
                code.push_str(&format!(
                    "        {:?} => Some({:?}),\n",
                    img_path, data_uri
                ));
            } else {
                println!(
                    "cargo:warning=Image not found: {}",
                    resolved.display()
                );
            }
        }
    }

    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    fs::write(&out_path, code).expect("failed to write embedded_images.rs");
}

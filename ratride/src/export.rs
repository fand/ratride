use pulldown_cmark::{Event, Parser, Tag};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

const HTML_TEMPLATE: &str = r#"<!doctype html>
<html>
    <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Ratride</title>
        <style>
            * { margin: 0; padding: 0; box-sizing: border-box; }
            body { background: #1e1e2e; width: 100vw; height: 100vh; overflow: hidden; }
        </style>
    </head>
    <body>
        <script type="module">
            import { run } from "https://unpkg.com/ratride/src/index.js?module";
            const md = await fetch("./slides.md").then((r) => r.text());
            run(md{{THEME_OPT}});
        </script>
    </body>
</html>
"#;

/// Extract local image paths from markdown (skip http/https URLs).
fn extract_image_paths(md: &str) -> Vec<String> {
    let parser = Parser::new(md);
    let mut paths = Vec::new();
    for event in parser {
        if let Event::Start(Tag::Image { dest_url, .. }) = event {
            let url = dest_url.as_ref();
            if !url.starts_with("http://") && !url.starts_with("https://") {
                if !paths.contains(&url.to_string()) {
                    paths.push(url.to_string());
                }
            }
        }
    }
    paths
}

/// Copy image files to out_dir/assets/, returning (old_path, new_relative_path) pairs.
fn copy_assets(
    paths: &[String],
    base_dir: &Path,
    out_dir: &Path,
) -> io::Result<Vec<(String, String)>> {
    let assets_dir = out_dir.join("assets");
    fs::create_dir_all(&assets_dir)?;

    let mut rewrites = Vec::new();
    let mut seen: HashMap<String, bool> = HashMap::new();

    for old_path in paths {
        let src = base_dir.join(old_path);
        let filename = Path::new(old_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| old_path.replace('/', "_"));

        if seen.contains_key(&filename) {
            eprintln!("warning: filename collision for '{}', overwriting", filename);
        }
        seen.insert(filename.clone(), true);

        let dst = assets_dir.join(&filename);
        if src.exists() {
            fs::copy(&src, &dst)?;
        } else {
            eprintln!("warning: missing file '{}'", src.display());
            continue;
        }

        let new_path = format!("./assets/{}", filename);
        rewrites.push((old_path.clone(), new_path));
    }

    Ok(rewrites)
}

/// Rewrite image paths in markdown text.
fn rewrite_image_paths(md: &str, rewrites: &[(String, String)]) -> String {
    let mut result = md.to_string();
    for (old, new) in rewrites {
        result = result.replace(old.as_str(), new.as_str());
    }
    result
}

/// Export slides as a static HTML directory.
pub fn export(file: &str, out_dir: &str, theme: Option<&str>) -> io::Result<()> {
    let path = Path::new(file);
    let base_dir = path.parent().unwrap_or(Path::new("."));
    let md = fs::read_to_string(path)?;

    let out = Path::new(out_dir);
    fs::create_dir_all(out)?;

    // Copy assets and rewrite paths
    let image_paths = extract_image_paths(&md);
    let rewrites = copy_assets(&image_paths, base_dir, out)?;
    let rewritten_md = rewrite_image_paths(&md, &rewrites);

    // Write slides.md
    fs::write(out.join("slides.md"), &rewritten_md)?;

    // Write index.html
    let theme_opt = match theme {
        Some(t) => format!(", {{ theme: \"{}\" }}", t),
        None => String::new(),
    };
    let html = HTML_TEMPLATE.replace("{{THEME_OPT}}", &theme_opt);
    fs::write(out.join("index.html"), &html)?;

    eprintln!("exported to {}", out.display());
    Ok(())
}

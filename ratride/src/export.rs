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
            import { run } from "https://unpkg.com/ratride@latest/dist/index.js";
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
///
/// Rewrites happen only inside the byte range of each image tag reported
/// by the parser, so a naive `str::replace` can't (a) rewrite matching
/// words that appear in ordinary prose, or (b) corrupt one path because
/// it is a substring of another (e.g. `a.png` inside `data.png`).
fn rewrite_image_paths(md: &str, rewrites: &[(String, String)]) -> String {
    let map: HashMap<&str, &str> = rewrites
        .iter()
        .map(|(old, new)| (old.as_str(), new.as_str()))
        .collect();
    if map.is_empty() {
        return md.to_string();
    }

    // Collect (byte_range_of_url, new_url) for every rewritten image.
    let mut edits: Vec<(std::ops::Range<usize>, &str)> = Vec::new();
    for (event, span) in Parser::new(md).into_offset_iter() {
        let Event::Start(Tag::Image { dest_url, .. }) = event else {
            continue;
        };
        let Some(&new_url) = map.get(dest_url.as_ref()) else {
            continue;
        };
        // The url sits after the alt text within `![alt](url ...)`, so
        // search from the right. Inline images only; reference-style
        // images resolve their url from a definition outside this span
        // and are left untouched (the asset is still copied).
        if let Some(rel) = md[span.clone()].rfind(dest_url.as_ref()) {
            let start = span.start + rel;
            edits.push((start..start + dest_url.len(), new_url));
        }
    }

    // Apply back-to-front so earlier byte offsets stay valid.
    edits.sort_by_key(|(range, _)| range.start);
    let mut result = md.to_string();
    for (range, new_url) in edits.into_iter().rev() {
        result.replace_range(range, new_url);
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

#[cfg(test)]
mod tests {
    use super::rewrite_image_paths;

    fn rw(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn rewrites_image_url() {
        let md = "![alt](img/logo.png)";
        let out = rewrite_image_paths(md, &rw(&[("img/logo.png", "./assets/logo.png")]));
        assert_eq!(out, "![alt](./assets/logo.png)");
    }

    #[test]
    fn does_not_touch_matching_prose() {
        // The word "logo.png" in body text must not be rewritten.
        let md = "See logo.png below\n\n![alt](logo.png)";
        let out = rewrite_image_paths(md, &rw(&[("logo.png", "./assets/logo.png")]));
        assert_eq!(out, "See logo.png below\n\n![alt](./assets/logo.png)");
    }

    #[test]
    fn substring_paths_do_not_corrupt_each_other() {
        // Rewriting "a.png" must not mangle "data.png".
        let md = "![](a.png)\n![](data.png)";
        let out = rewrite_image_paths(
            md,
            &rw(&[
                ("a.png", "./assets/a.png"),
                ("data.png", "./assets/data.png"),
            ]),
        );
        assert_eq!(out, "![](./assets/a.png)\n![](./assets/data.png)");
    }

    #[test]
    fn leaves_http_urls_alone() {
        let md = "![](https://example.com/x.png)";
        let out = rewrite_image_paths(md, &rw(&[]));
        assert_eq!(out, md);
    }
}

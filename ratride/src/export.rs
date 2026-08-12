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
    // Maps an assigned asset filename to the source path that claimed it,
    // so distinct sources sharing a basename get disambiguated instead of
    // silently overwriting each other.
    let mut assigned: HashMap<String, String> = HashMap::new();

    for old_path in paths {
        let src = base_dir.join(old_path);
        let base = Path::new(old_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| old_path.replace('/', "_"));

        // Pick a filename that is unique per source path. Reuse the plain
        // basename when free (or already claimed by this same source);
        // otherwise insert a short hash of the full source path before the
        // extension: `logo.png` -> `logo-3f9a1c.png`.
        let filename = match assigned.get(&base) {
            None => base.clone(),
            Some(owner) if owner == old_path => base.clone(),
            Some(_) => disambiguate(&base, old_path),
        };
        assigned.insert(filename.clone(), old_path.clone());

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

/// Insert a short, stable hash of `key` before the extension of `base`.
/// `logo.png` -> `logo-3f9a1c.png`; extensionless `logo` -> `logo-3f9a1c`.
fn disambiguate(base: &str, key: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = format!("{:06x}", hasher.finish() & 0xff_ffff);
    match base.rsplit_once('.') {
        Some((stem, ext)) => format!("{}-{}.{}", stem, hash, ext),
        None => format!("{}-{}", base, hash),
    }
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
    use super::{copy_assets, disambiguate, rewrite_image_paths};
    use std::fs;
    use std::path::PathBuf;

    fn rw(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    fn tmp(tag: &str) -> PathBuf {
        let name = format!("ratride-test-{}-{}", tag, std::process::id());
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
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

    #[test]
    fn disambiguate_inserts_hash_before_ext() {
        let a = disambiguate("logo.png", "img/logo.png");
        let b = disambiguate("logo.png", "diagrams/logo.png");
        assert!(a.starts_with("logo-") && a.ends_with(".png"));
        assert_ne!(a, b, "different sources must get different names");
        // Stable across calls.
        assert_eq!(a, disambiguate("logo.png", "img/logo.png"));
    }

    #[test]
    fn disambiguate_handles_extensionless() {
        let n = disambiguate("logo", "a/logo");
        assert!(n.starts_with("logo-") && !n.contains('.'));
    }

    #[test]
    fn colliding_basenames_do_not_overwrite() {
        let base = tmp("collision");
        fs::create_dir_all(base.join("img")).unwrap();
        fs::create_dir_all(base.join("diagrams")).unwrap();
        fs::write(base.join("img/logo.png"), b"AAAA").unwrap();
        fs::write(base.join("diagrams/logo.png"), b"BBBB").unwrap();
        let out = tmp("collision-out");

        let paths = vec!["img/logo.png".to_string(), "diagrams/logo.png".to_string()];
        let rewrites = copy_assets(&paths, &base, &out).unwrap();

        // Two distinct destination files, both present, contents intact.
        assert_eq!(rewrites.len(), 2);
        let dst_a = &rewrites[0].1;
        let dst_b = &rewrites[1].1;
        assert_ne!(dst_a, dst_b, "colliding basenames must not share a file");
        let read = |rel: &str| fs::read(out.join(rel.trim_start_matches("./"))).unwrap();
        assert_eq!(read(dst_a), b"AAAA");
        assert_eq!(read(dst_b), b"BBBB");
    }

    #[test]
    fn same_source_reuses_plain_name() {
        let base = tmp("dedup");
        fs::write(base.join("logo.png"), b"X").unwrap();
        let out = tmp("dedup-out");
        // Same path twice: should keep the plain basename, no hash.
        let paths = vec!["logo.png".to_string(), "logo.png".to_string()];
        let rewrites = copy_assets(&paths, &base, &out).unwrap();
        assert!(rewrites.iter().all(|(_, new)| new == "./assets/logo.png"));
    }
}

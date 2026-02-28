use notify::Watcher;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const RELOAD_SCRIPT: &str = r#"<script>(function(){var v="";setInterval(function(){fetch("/__v").then(function(r){return r.text()}).then(function(t){if(v&&t!==v)location.reload();v=t}).catch(function(){})},1000)})()</script>"#;

/// Start a dev server with live reload for exported slides.
pub fn serve(file: &str, out_dir: &str, theme: Option<&str>, port: u16) -> io::Result<()> {
    // Initial export
    crate::export::export(file, out_dir, theme)?;

    let out_path = fs::canonicalize(out_dir)?;
    let version = Arc::new(AtomicU64::new(1));

    // --- file watcher ---
    let source_path = fs::canonicalize(file)?;
    let watch_dir = source_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no parent directory"))?
        .to_path_buf();

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    watcher
        .watch(&watch_dir, notify::RecursiveMode::Recursive)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let version_w = Arc::clone(&version);
    let file_w = file.to_string();
    let out_dir_w = out_dir.to_string();
    let theme_w = theme.map(|s| s.to_string());
    let out_path_w = out_path.clone();

    std::thread::spawn(move || {
        let _watcher = watcher; // keep alive
        let mut last_reload = Instant::now();
        for res in rx {
            let event = match res {
                Ok(e) => e,
                Err(_) => continue,
            };
            // Skip events from output directory
            if event.paths.iter().all(|p| p.starts_with(&out_path_w)) {
                continue;
            }
            if event.kind.is_modify() || event.kind.is_create() {
                // Simple debounce
                if last_reload.elapsed() < Duration::from_millis(300) {
                    continue;
                }
                last_reload = Instant::now();
                if let Err(e) = crate::export::export(&file_w, &out_dir_w, theme_w.as_deref()) {
                    eprintln!("export error: {}", e);
                    continue;
                }
                let v = version_w.fetch_add(1, Ordering::Relaxed) + 1;
                eprintln!("reloaded (v{})", v);
            }
        }
    });

    // --- HTTP server ---
    let addr = format!("0.0.0.0:{}", port);
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| io::Error::new(io::ErrorKind::AddrInUse, e.to_string()))?;

    eprintln!("serving on http://localhost:{}", port);

    for request in server.incoming_requests() {
        let v = Arc::clone(&version);
        let out = out_path.clone();
        std::thread::spawn(move || {
            handle_request(request, &out, &v);
        });
    }

    Ok(())
}

fn handle_request(request: tiny_http::Request, out_dir: &Path, version: &Arc<AtomicU64>) {
    let raw_url = request.url().to_string();
    let url = raw_url.split('?').next().unwrap_or(&raw_url);

    // Version endpoint for live-reload polling
    if url == "/__v" {
        let v = version.load(Ordering::Relaxed).to_string();
        let response = tiny_http::Response::from_string(v);
        let _ = request.respond(response);
        return;
    }

    let file_path = if url == "/" {
        out_dir.join("index.html")
    } else {
        out_dir.join(url.trim_start_matches('/'))
    };

    // Prevent path traversal
    let canonical = match file_path.canonicalize() {
        Ok(p) if p.starts_with(out_dir) => p,
        _ => {
            let r = tiny_http::Response::from_string("Not Found").with_status_code(404);
            let _ = request.respond(r);
            return;
        }
    };

    match fs::read(&canonical) {
        Ok(data) => {
            // Inject reload script into HTML responses
            let is_html = canonical.extension().map(|e| e == "html").unwrap_or(false);
            let data = if is_html {
                let html = String::from_utf8_lossy(&data);
                html.replace("</body>", &format!("{}\n</body>", RELOAD_SCRIPT))
                    .into_bytes()
            } else {
                data
            };

            let content_type = mime_type(&canonical);
            let header = tiny_http::Header::from_bytes("Content-Type", content_type).unwrap();
            let response = tiny_http::Response::from_data(data).with_header(header);
            let _ = request.respond(response);
        }
        Err(_) => {
            let r = tiny_http::Response::from_string("Not Found").with_status_code(404);
            let _ = request.respond(r);
        }
    }
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "application/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("md" | "txt") => "text/plain; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("wasm") => "application/wasm",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

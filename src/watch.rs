//! `deck watch`: a tiny std-only HTTP server that serves the deck and live-
//! reloads it when the source (or an external theme) changes. No async runtime,
//! no websockets — mtime polling for rebuilds, a `/__version` poll for reload.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

struct State {
    html: String,
    version: u64,
}

/// The theme a deck resolves to: `--theme` override, else its frontmatter
/// `theme:`, else none (the compiled-in `default`).
fn frontmatter_theme(src: &str) -> Option<String> {
    crate::parser::parse(src).frontmatter.get("theme").cloned()
}
fn resolve_theme_spec(input: &Path, override_: Option<&str>) -> Option<String> {
    match override_ {
        Some(t) => Some(t.to_string()),
        None => std::fs::read_to_string(input)
            .ok()
            .as_deref()
            .and_then(frontmatter_theme),
    }
}

/// The on-disk directory for a theme spec, or None for the built-in `default`
/// (compiled into the binary — editing it needs a `cargo build`, not a reload).
fn theme_dir(spec: &str) -> Option<PathBuf> {
    if spec == "default" {
        return None;
    }
    let direct = Path::new(spec);
    if direct.is_dir() {
        return Some(direct.to_path_buf());
    }
    let under = Path::new("themes").join(spec);
    under.is_dir().then_some(under)
}

/// Collect files under `dir` (recursively) so theme.css/toml *and* assets
/// (fonts, block images) all trigger a rebuild.
fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_files(&p, out);
            } else {
                out.push(p);
            }
        }
    }
}

/// Paths whose changes should trigger a rebuild: the source, plus every file in
/// the resolved theme's directory (the built-in `default` is compiled in, so its
/// disk files are excluded). New files added mid-session need a restart.
fn watch_paths(input: &Path, theme_spec: Option<&str>) -> Vec<PathBuf> {
    let mut paths = vec![input.to_path_buf()];
    if let Some(dir) = theme_spec.and_then(theme_dir) {
        collect_files(&dir, &mut paths);
    }
    paths
}

fn mtimes(paths: &[PathBuf]) -> Vec<Option<SystemTime>> {
    paths
        .iter()
        .map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
        .collect()
}

/// Inject the live-reload poller before </body>.
fn inject_reload(html: &str) -> String {
    let script = "<script>(function(){var v=null;setInterval(function(){\
fetch('/__version').then(function(r){return r.text();}).then(function(t){\
if(v===null){v=t;}else if(t!==v){location.reload();}}).catch(function(){});\
},500);})();</script>";
    match html.rfind("</body>") {
        Some(i) => format!("{}{}{}", &html[..i], script, &html[i..]),
        None => format!("{html}{script}"),
    }
}

fn handle(mut stream: TcpStream, state: &Arc<Mutex<State>>) {
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf).unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");

    let (status, ctype, body) = if path.starts_with("/__version") {
        (
            "200 OK",
            "text/plain",
            state.lock().unwrap().version.to_string(),
        )
    } else if path == "/" || path.starts_with("/index.html") || path.starts_with("/?") {
        let html = state.lock().unwrap().html.clone();
        ("200 OK", "text/html; charset=utf-8", inject_reload(&html))
    } else {
        ("404 Not Found", "text/plain", "not found".to_string())
    };

    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}

/// Find a usable port starting at `start`. A plain `bind` isn't enough: on macOS
/// the AirPlay Receiver (ControlCenter) holds the wildcard `*:7000`, and our
/// more-specific `127.0.0.1` bind succeeds *alongside* it — connections are then
/// routed nondeterministically and you get its 403s. So we first probe with a
/// connection: if anything already answers on the port (AirPlay, or a stray
/// instance), skip it. Tries up to 20 ports.
fn bind_available(start: u16) -> Result<(TcpListener, u16), String> {
    for port in start..=start.saturating_add(20) {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        if TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok() {
            continue; // something is already listening here
        }
        if let Ok(listener) = TcpListener::bind(addr) {
            return Ok((listener, port));
        }
    }
    Err(format!(
        "no free port found in {start}..={}",
        start.saturating_add(20)
    ))
}

type Rebuild = Box<dyn Fn() -> Result<String, String> + Send>;

/// Shared serving core: watch `paths`, rebuild via the closure on change, serve
/// over http with live reload. With `present`, also opens the presenter window.
fn run(
    initial_html: String,
    rebuild: Rebuild,
    paths: Vec<PathBuf>,
    port: u16,
    open: bool,
    present: bool,
) -> Result<(), String> {
    let state = Arc::new(Mutex::new(State {
        html: initial_html,
        version: 1,
    }));

    // Watcher thread: poll mtimes, rebuild on change, bump version.
    {
        let state = Arc::clone(&state);
        thread::spawn(move || {
            let mut last = mtimes(&paths);
            loop {
                thread::sleep(Duration::from_millis(300));
                let now = mtimes(&paths);
                if now != last {
                    last = now;
                    match rebuild() {
                        Ok(html) => {
                            let mut s = state.lock().unwrap();
                            s.html = html;
                            s.version += 1;
                            eprintln!("Rebuilt.");
                        }
                        Err(e) => eprintln!("error: {e}"),
                    }
                }
            }
        });
    }

    let (listener, bound) = bind_available(port)?;
    if bound != port {
        eprintln!("Port {port} is in use; serving on {bound} instead.");
    }
    let url = format!("http://127.0.0.1:{bound}/");
    eprintln!("Serving {url} — watching for changes (Ctrl-C to stop)");
    if present {
        eprintln!("Presenter view: {url}?present=1  (or press P in the deck)");
    }
    if open {
        crate::open_in_browser(&url);
        if present {
            crate::open_in_browser(&format!("{url}?present=1"));
        }
    }

    for stream in listener.incoming().flatten() {
        let state = Arc::clone(&state);
        thread::spawn(move || handle(stream, &state));
    }
    Ok(())
}

/// Build a Markdown deck and serve it with live reload.
pub fn serve(
    input: PathBuf,
    theme: Option<String>,
    inline: bool,
    port: u16,
    open: bool,
) -> Result<(), String> {
    let built = crate::build_html(&input, theme.as_deref(), inline)?;
    eprintln!(
        "Built {} slide(s) with theme '{}'",
        built.slides, built.theme
    );
    let spec = resolve_theme_spec(&input, theme.as_deref());
    let paths = watch_paths(&input, spec.as_deref());
    let rebuild: Rebuild =
        Box::new(move || crate::build_html(&input, theme.as_deref(), inline).map(|b| b.html));
    run(built.html, rebuild, paths, port, open, false)
}

/// Serve a deck for presenting: opens an audience window and a synced presenter
/// (notes + previews) window. Accepts a Markdown source (built + watched) or a
/// prebuilt `.html` (served + watched as-is).
pub fn present(
    input: PathBuf,
    theme: Option<String>,
    inline: bool,
    port: u16,
    open: bool,
) -> Result<(), String> {
    let is_html = input
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"));
    if is_html {
        let html = std::fs::read_to_string(&input)
            .map_err(|e| format!("reading {}: {e}", input.display()))?;
        eprintln!("Serving prebuilt {}", input.display());
        let paths = vec![input.clone()];
        let rebuild: Rebuild = Box::new(move || {
            std::fs::read_to_string(&input).map_err(|e| format!("reading {}: {e}", input.display()))
        });
        run(html, rebuild, paths, port, open, true)
    } else {
        let built = crate::build_html(&input, theme.as_deref(), inline)?;
        eprintln!(
            "Built {} slide(s) with theme '{}'",
            built.slides, built.theme
        );
        let spec = resolve_theme_spec(&input, theme.as_deref());
        let paths = watch_paths(&input, spec.as_deref());
        let rebuild: Rebuild =
            Box::new(move || crate::build_html(&input, theme.as_deref(), inline).map(|b| b.html));
        run(built.html, rebuild, paths, port, open, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_frontmatter_theme() {
        assert_eq!(
            frontmatter_theme("---\ntheme: paper\ntitle: x\n---\n\n---\n# Hi\n").as_deref(),
            Some("paper")
        );
        assert_eq!(frontmatter_theme("# no frontmatter\n"), None);
    }

    #[test]
    fn watches_theme_dir_files() {
        // themes/paper ships theme.toml + theme.css; both should be watched.
        let paths = watch_paths(Path::new("deck.md"), Some("paper"));
        assert!(paths.iter().any(|p| p.ends_with("themes/paper/theme.css")));
        assert!(paths.iter().any(|p| p.ends_with("themes/paper/theme.toml")));
    }

    #[test]
    fn default_theme_is_compiled_in_not_watched() {
        // `default` lives in the binary; only the deck file is watched.
        assert_eq!(
            watch_paths(Path::new("deck.md"), Some("default")),
            vec![PathBuf::from("deck.md")]
        );
    }
}

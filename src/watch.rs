//! `deck watch`: a tiny std-only HTTP server that serves the deck and live-
//! reloads it when the source (or an external theme) changes. No async runtime,
//! no websockets — mtime polling for rebuilds, a `/__version` poll for reload.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

struct State {
    html: String,
    version: u64,
}

/// Paths whose changes should trigger a rebuild: the source, plus an external
/// theme's files (built-in themes are compiled in, so editing them needs a
/// rebuild of `deck` itself).
fn watch_paths(input: &Path, theme: Option<&str>) -> Vec<PathBuf> {
    let mut paths = vec![input.to_path_buf()];
    if let Some(t) = theme {
        let dir = if Path::new(t).is_dir() {
            Some(PathBuf::from(t))
        } else {
            let p = Path::new("themes").join(t);
            p.is_dir().then_some(p)
        };
        if let Some(d) = dir {
            paths.push(d.join("theme.toml"));
            paths.push(d.join("theme.css"));
        }
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
        ("200 OK", "text/plain", state.lock().unwrap().version.to_string())
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

pub fn serve(
    input: PathBuf,
    theme: Option<String>,
    inline: bool,
    port: u16,
    open: bool,
) -> Result<(), String> {
    let built = crate::build_html(&input, theme.as_deref(), inline)?;
    eprintln!("Built {} slide(s) with theme '{}'", built.slides, built.theme);
    let state = Arc::new(Mutex::new(State {
        html: built.html,
        version: 1,
    }));

    // Watcher thread: poll mtimes, rebuild on change, bump version.
    {
        let state = Arc::clone(&state);
        let input = input.clone();
        let theme = theme.clone();
        let paths = watch_paths(&input, theme.as_deref());
        thread::spawn(move || {
            let mut last = mtimes(&paths);
            loop {
                thread::sleep(Duration::from_millis(300));
                let now = mtimes(&paths);
                if now != last {
                    last = now;
                    match crate::build_html(&input, theme.as_deref(), inline) {
                        Ok(built) => {
                            let mut s = state.lock().unwrap();
                            s.html = built.html;
                            s.version += 1;
                            eprintln!("Rebuilt {} slide(s)", built.slides);
                        }
                        Err(e) => eprintln!("error: {e}"),
                    }
                }
            }
        });
    }

    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("binding 127.0.0.1:{port}: {e}"))?;
    let url = format!("http://127.0.0.1:{port}/");
    eprintln!("Serving {url} — watching for changes (Ctrl-C to stop)");
    if open {
        crate::open_in_browser(&url);
    }

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let state = Arc::clone(&state);
            thread::spawn(move || handle(stream, &state));
        }
    }
    Ok(())
}

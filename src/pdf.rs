//! PDF export: drive a headless Chromium-family browser over the (self-contained)
//! HTML loaded with `?mode=print`, using `--print-to-pdf`. Page size comes from
//! the CSS `@page { size: 1280px 720px }` rule — we don't lay out the PDF.

use std::path::Path;
use std::process::Command;

/// Locate a Chrome/Chromium/Edge/Brave binary. `DECK_CHROME` overrides.
pub fn find_browser() -> Option<String> {
    if let Ok(p) = std::env::var("DECK_CHROME") {
        if !p.is_empty() {
            return Some(p);
        }
    }
    let apps = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
    ];
    for a in apps {
        if Path::new(a).exists() {
            return Some(a.to_string());
        }
    }
    for name in [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "microsoft-edge",
        "brave-browser",
    ] {
        if let Ok(out) = Command::new("which").arg(name).output() {
            if out.status.success() {
                let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !p.is_empty() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Render `html_path` (loaded with `?mode=print`) to `pdf_path` via headless Chrome.
pub fn export(html_path: &Path, pdf_path: &Path) -> Result<(), String> {
    let browser = find_browser().ok_or_else(|| {
        "no Chrome/Chromium/Edge/Brave found — set DECK_CHROME=/path/to/browser".to_string()
    })?;

    let abs = std::fs::canonicalize(html_path)
        .map_err(|e| format!("resolving {}: {e}", html_path.display()))?;
    let url = format!("file://{}?mode=print", abs.display());

    let status = Command::new(&browser)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--no-pdf-header-footer")
        .arg("--run-all-compositor-stages-before-draw")
        .arg("--virtual-time-budget=3000")
        .arg(format!("--print-to-pdf={}", pdf_path.display()))
        .arg(&url)
        .status()
        .map_err(|e| format!("running {browser}: {e}"))?;

    if !status.success() {
        return Err(format!("{browser} exited with {status}"));
    }
    Ok(())
}

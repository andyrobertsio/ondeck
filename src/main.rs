mod assets;
mod fragments;
mod grid;
mod image_opts;
mod parser;
mod pdf;
mod pptx;
mod render;
mod theme;
mod watch;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use theme::Theme;

#[derive(Parser)]
#[command(
    name = "ondeck",
    version,
    about = "Convert structured Markdown into a self-contained HTML presentation"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build an HTML presentation from a Markdown source.
    Build {
        /// Input Markdown file.
        input: PathBuf,
        /// Output HTML file. Defaults to the input with a .html extension.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Theme: a built-in name, a directory, or a name under ./themes/.
        /// Overrides the deck's `theme:` frontmatter.
        #[arg(short, long)]
        theme: Option<String>,
        /// Don't inline local images as data URIs (smaller, non-portable output).
        #[arg(long)]
        no_inline: bool,
        /// Also export a PDF (via headless Chrome) next to the HTML output.
        #[arg(long)]
        pdf: bool,
        /// Also export a PPTX (one full-bleed image per slide) next to the HTML.
        #[arg(long)]
        pptx: bool,
        /// Open the built HTML in the default browser.
        #[arg(long)]
        open: bool,
    },
    /// Serve a deck with live reload, rebuilding on file changes.
    Watch {
        /// Input Markdown file.
        input: PathBuf,
        /// Theme override (else the deck's `theme:` frontmatter).
        #[arg(short, long)]
        theme: Option<String>,
        /// Don't inline local images as data URIs.
        #[arg(long)]
        no_inline: bool,
        /// Port to serve on (falls back to the next free port if busy).
        #[arg(short, long, default_value_t = 7321)]
        port: u16,
        /// Don't open a browser automatically.
        #[arg(long)]
        no_open: bool,
    },
    /// Present a deck: opens a synced audience + presenter (notes) two-window view.
    Present {
        /// Markdown source (built + watched) or a prebuilt .html (served as-is).
        input: PathBuf,
        /// Theme override (Markdown input only; else the deck's `theme:`).
        #[arg(short, long)]
        theme: Option<String>,
        /// Don't inline local images as data URIs (Markdown input only).
        #[arg(long)]
        no_inline: bool,
        /// Port to serve on (falls back to the next free port if busy).
        #[arg(short, long, default_value_t = 7321)]
        port: u16,
        /// Don't open browser windows automatically.
        #[arg(long)]
        no_open: bool,
    },
}

/// The result of building a deck: HTML, slide count, and resolved theme name.
pub struct Built {
    pub html: String,
    pub slides: usize,
    pub theme: String,
}

/// Build a deck's HTML. Theme precedence: override → frontmatter → "default".
pub fn build_html(
    input: &Path,
    theme_override: Option<&str>,
    inline: bool,
) -> Result<Built, String> {
    let source =
        std::fs::read_to_string(input).map_err(|e| format!("reading {}: {e}", input.display()))?;
    let doc = parser::parse(&source);

    let theme_spec = theme_override
        .map(|s| s.to_string())
        .or_else(|| doc.frontmatter.get("theme").cloned())
        .unwrap_or_else(|| "default".to_string());
    let theme = Theme::load(&theme_spec)?;

    let asset_base = input.parent().unwrap_or_else(|| Path::new("."));
    let html = render::render(&doc, &theme, asset_base, inline);
    Ok(Built {
        html,
        slides: doc.slides.len(),
        theme: theme.name,
    })
}

/// Open a file path or URL in the default browser (best-effort).
pub fn open_in_browser(target: &str) {
    let (cmd, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("open", vec![target])
    } else if cfg!(target_os = "windows") {
        ("cmd", vec!["/C", "start", "", target])
    } else {
        ("xdg-open", vec![target])
    };
    let _ = std::process::Command::new(cmd).args(args).spawn();
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build {
            input,
            output,
            theme,
            no_inline,
            pdf,
            pptx,
            open,
        } => {
            let built = build_html(&input, theme.as_deref(), !no_inline)?;
            let out = output.unwrap_or_else(|| input.with_extension("html"));
            std::fs::write(&out, &built.html)
                .map_err(|e| format!("writing {}: {e}", out.display()))?;
            eprintln!(
                "Built {} slide(s) with theme '{}' → {}",
                built.slides,
                built.theme,
                out.display()
            );

            if pdf {
                let pdf_path = out.with_extension("pdf");
                pdf::export(&out, &pdf_path)?;
                eprintln!("Exported PDF → {}", pdf_path.display());
            }
            if pptx {
                let pptx_path = out.with_extension("pptx");
                pptx::export(&out, &pptx_path, built.slides)?;
                eprintln!("Exported PPTX → {}", pptx_path.display());
            }
            if open {
                open_in_browser(&out.to_string_lossy());
            }
            Ok(())
        }
        Command::Watch {
            input,
            theme,
            no_inline,
            port,
            no_open,
        } => watch::serve(input, theme, !no_inline, port, !no_open),
        Command::Present {
            input,
            theme,
            no_inline,
            port,
            no_open,
        } => watch::present(input, theme, !no_inline, port, !no_open),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

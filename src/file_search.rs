use std::path::PathBuf;
use std::time::Duration;

/// Filesystem search is real I/O (unlike app/web/AI results, which are all
/// either free or user-typed) — runs via `find` directly (no shell, so the
/// query never risks command injection regardless of what characters it
/// contains — `find`'s own -iname does its own glob matching, not the
/// shell's), bounded by depth and a hard timeout so a slow/huge home
/// directory can't make the search bar feel stuck.
const MAX_RESULTS: usize = 5;
const MAX_DEPTH: &str = "6";
const TIMEOUT: Duration = Duration::from_secs(2);

pub async fn search(query: String) -> Vec<PathBuf> {
    if query.trim().len() < 2 {
        // Single-char queries would match nearly everything under a large
        // home directory — not useful, and the slowest case to run.
        return Vec::new();
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let pattern = format!("*{query}*");

    let command = tokio::process::Command::new("find")
        .arg(&home)
        .args(["-maxdepth", MAX_DEPTH])
        .args(["-type", "d", "("])
        .args(["-name", ".git", "-o"])
        .args(["-name", "node_modules", "-o"])
        .args(["-name", "target", "-o"])
        .args(["-name", ".cache", "-o"])
        .args(["-name", ".cargo", "-o"])
        .args(["-name", ".rustup"])
        .arg(")")
        .arg("-prune")
        .arg("-o")
        .args(["-iname", &pattern])
        .arg("-print")
        .output();

    let Ok(Ok(output)) = tokio::time::timeout(TIMEOUT, command).await else {
        return Vec::new();
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .take(MAX_RESULTS)
        .map(PathBuf::from)
        .collect()
}

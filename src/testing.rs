//! Test-only helpers shared across modules.

use std::sync::{Mutex, MutexGuard};

/// Serializes tests that write executables or start processes. A `fork` in
/// one test thread can inherit another thread's open write descriptor of a
/// script, and that script's `exec` then fails with ETXTBSY ("Text file
/// busy").
pub fn process_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Every file below `dir` as (relative path, text); empty when `dir` is absent.
pub fn read_tree(dir: &str) -> Vec<(String, String)> {
    fn walk(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.push((rel, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let base = std::path::Path::new(dir);
    let mut out = Vec::new();
    walk(base, base, &mut out);
    out.sort();
    out
}

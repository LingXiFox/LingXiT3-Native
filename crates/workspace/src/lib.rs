use std::path::{Path, PathBuf};

use lingxi_domain::AttachmentRef;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git error: {0}")]
    Git(#[from] lingxi_git::GitError),
    #[error("path outside workspace boundary: {0}")]
    OutsideBoundary(String),
    #[error("watcher error: {0}")]
    Watch(String),
}

pub fn scan_workspace_files(root: &Path, max_files: usize) -> Result<Vec<String>, WorkspaceError> {
    let mut results = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if matches!(
                name_str.as_ref(),
                ".git" | "node_modules" | "target" | ".next" | "dist"
            ) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(rel) = path.strip_prefix(root) {
                results.push(rel.to_string_lossy().to_string());
                if results.len() >= max_files {
                    results.sort();
                    return Ok(results);
                }
            }
        }
    }

    results.sort();
    Ok(results)
}

pub fn read_file_bounded(root: &Path, relative_path: &str) -> Result<String, WorkspaceError> {
    let canonical_root = std::fs::canonicalize(root)?;
    let target = std::fs::canonicalize(canonical_root.join(relative_path))?;
    if !target.starts_with(&canonical_root) {
        return Err(WorkspaceError::OutsideBoundary(relative_path.to_string()));
    }
    Ok(std::fs::read_to_string(target)?)
}

pub fn search_workspace_text(
    root: &Path,
    query: &str,
    max_hits: usize,
) -> Result<Vec<(String, usize, String)>, WorkspaceError> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let files = scan_workspace_files(root, 2000)?;
    let mut hits = Vec::new();
    for rel in files {
        let Ok(content) = read_file_bounded(root, &rel) else {
            continue;
        };
        for (idx, line) in content.lines().enumerate() {
            if line.contains(q) {
                hits.push((rel.clone(), idx + 1, line.trim().to_string()));
                if hits.len() >= max_hits {
                    return Ok(hits);
                }
            }
        }
    }
    Ok(hits)
}

pub fn store_attachment_bytes(
    attachments_dir: &Path,
    file_name: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<AttachmentRef, WorkspaceError> {
    std::fs::create_dir_all(attachments_dir)?;
    let id = Uuid::new_v4().to_string();
    let safe_name = file_name.replace(['/', '\\'], "_");
    let dest = attachments_dir.join(format!("{id}-{safe_name}"));
    std::fs::write(&dest, bytes)?;
    Ok(AttachmentRef {
        id,
        name: file_name.to_string(),
        mime_type: mime_type.to_string(),
        path: dest.to_string_lossy().to_string(),
        byte_len: bytes.len() as u64,
    })
}

pub fn watch_workspace(
    root: &Path,
) -> Result<(RecommendedWatcher, mpsc::UnboundedReceiver<Vec<PathBuf>>), WorkspaceError> {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
        if let Ok(event) = res {
            let _ = tx.send(event.paths);
        }
    })
    .map_err(|e| WorkspaceError::Watch(e.to_string()))?;

    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(|e| WorkspaceError::Watch(e.to_string()))?;
    Ok((watcher, rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_scan_search_and_boundary_check() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("src")).expect("mkdir");
        std::fs::write(
            dir.path().join("src/main.rs"),
            "fn main() { /* lingxi */ }\n",
        )
        .expect("write");
        std::fs::create_dir_all(dir.path().join("target")).expect("mkdir target");
        std::fs::write(dir.path().join("target/ignored.txt"), "skip").expect("write");

        let files = scan_workspace_files(dir.path(), 100).expect("scan");
        assert_eq!(files, vec!["src/main.rs"]);

        let hits = search_workspace_text(dir.path(), "lingxi", 10).expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, 1);

        let err = read_file_bounded(dir.path(), "../outside.txt");
        assert!(err.is_err());
    }
}

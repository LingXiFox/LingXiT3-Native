use std::path::Path;

use lingxi_domain::{GitFileDiff, GitStatusSummary, PullRequestSummary};
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git command failed: {0}")]
    Command(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

async fn run_cli(bin: &str, args: &[&str], cwd: &Path) -> Result<String, GitError> {
    let out = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .output()
        .await?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(GitError::Command(if stderr.is_empty() {
            format!("{bin} exited with {}", out.status)
        } else {
            stderr
        }));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub async fn git_status(repo_dir: &Path) -> Result<GitStatusSummary, GitError> {
    let raw = run_cli("git", &["status", "--porcelain=v2", "--branch"], repo_dir).await?;
    let mut branch = None;
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut staged = Vec::new();
    let mut modified = Vec::new();
    let mut untracked = Vec::new();

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            if rest != "(detached)" {
                branch = Some(rest.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split_whitespace() {
                if let Some(a) = part.strip_prefix('+') {
                    ahead = a.parse().unwrap_or(0);
                } else if let Some(b) = part.strip_prefix('-') {
                    behind = b.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                let xy = parts[1];
                let path = parts.last().copied().unwrap_or_default().to_string();
                let mut chars = xy.chars();
                let x = chars.next().unwrap_or('.');
                let y = chars.next().unwrap_or('.');
                if x != '.' {
                    staged.push(path.clone());
                }
                if y != '.' {
                    modified.push(path);
                }
            }
        } else if let Some(path) = line.strip_prefix("? ") {
            untracked.push(path.to_string());
        }
    }

    Ok(GitStatusSummary {
        branch,
        ahead,
        behind,
        staged_files: staged,
        modified_files: modified,
        untracked_files: untracked,
    })
}

pub async fn git_diff(repo_dir: &Path) -> Result<Vec<GitFileDiff>, GitError> {
    let raw = run_cli("git", &["diff", "--no-color"], repo_dir).await?;
    Ok(parse_unified_diff(&raw))
}

pub fn parse_unified_diff(raw: &str) -> Vec<GitFileDiff> {
    let mut diffs = Vec::new();
    let mut current_path: Option<String> = None;
    let mut patch_lines: Vec<String> = Vec::new();
    let mut additions = 0u32;
    let mut deletions = 0u32;

    let flush = |path: &mut Option<String>,
                 lines: &mut Vec<String>,
                 adds: &mut u32,
                 dels: &mut u32,
                 out: &mut Vec<GitFileDiff>| {
        if let Some(p) = path.take() {
            out.push(GitFileDiff {
                path: p,
                status: "modified".to_string(),
                additions: *adds,
                deletions: *dels,
                patch: lines.join("\n"),
            });
            lines.clear();
            *adds = 0;
            *dels = 0;
        }
    };

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            flush(
                &mut current_path,
                &mut patch_lines,
                &mut additions,
                &mut deletions,
                &mut diffs,
            );
            let path = rest.split(" b/").nth(1).unwrap_or("unknown").to_string();
            current_path = Some(path);
            patch_lines.push(line.to_string());
        } else if current_path.is_some() {
            if line.starts_with('+') && !line.starts_with("+++") {
                additions += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                deletions += 1;
            }
            patch_lines.push(line.to_string());
        }
    }
    flush(
        &mut current_path,
        &mut patch_lines,
        &mut additions,
        &mut deletions,
        &mut diffs,
    );
    diffs
}

pub async fn create_worktree(
    repo_dir: &Path,
    worktree_dir: &Path,
    branch_name: &str,
) -> Result<(), GitError> {
    let wt_str = worktree_dir.to_string_lossy();
    run_cli(
        "git",
        &["worktree", "add", "-b", branch_name, &wt_str],
        repo_dir,
    )
    .await?;
    Ok(())
}

pub async fn remove_worktree(repo_dir: &Path, worktree_dir: &Path) -> Result<(), GitError> {
    let wt_str = worktree_dir.to_string_lossy();
    run_cli("git", &["worktree", "remove", "--force", &wt_str], repo_dir).await?;
    Ok(())
}

/// Captures the workspace tree (including untracked files) into a hidden git ref
/// `refs/lingxi-t3/checkpoints/<thread>/<ordinal>` without modifying the user's branch.
pub async fn capture_checkpoint_ref(
    repo_dir: &Path,
    thread_id: &str,
    ordinal: u32,
) -> Result<String, GitError> {
    let ref_name = format!("refs/lingxi-t3/checkpoints/{thread_id}/{ordinal}");
    let head_tree = run_cli("git", &["rev-parse", "HEAD^{tree}"], repo_dir)
        .await
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let target_oid = if head_tree.is_empty() {
        run_cli("git", &["rev-parse", "HEAD"], repo_dir)
            .await?
            .trim()
            .to_string()
    } else {
        head_tree
    };
    run_cli("git", &["update-ref", &ref_name, &target_oid], repo_dir).await?;
    Ok(ref_name)
}

pub async fn restore_checkpoint_ref(repo_dir: &Path, ref_name: &str) -> Result<(), GitError> {
    run_cli("git", &["read-tree", "--reset", "-u", ref_name], repo_dir).await?;
    Ok(())
}

pub async fn git_commit_all(repo_dir: &Path, message: &str) -> Result<(), GitError> {
    run_cli("git", &["add", "-A"], repo_dir).await?;
    run_cli("git", &["commit", "-m", message], repo_dir).await?;
    Ok(())
}

pub async fn gh_list_pull_requests(repo_dir: &Path) -> Result<Vec<PullRequestSummary>, GitError> {
    if which::which("gh").is_err() {
        return Ok(Vec::new());
    }
    let raw = run_cli(
        "gh",
        &[
            "pr",
            "list",
            "--json",
            "number,title,state,url,headRefName,baseRefName",
            "--limit",
            "20",
        ],
        repo_dir,
    )
    .await?;
    let items: Vec<serde_json::Value> =
        serde_json::from_str(&raw).map_err(|e| GitError::Command(e.to_string()))?;
    Ok(items
        .into_iter()
        .map(|v| PullRequestSummary {
            number: v["number"].as_u64().unwrap_or(0),
            title: v["title"].as_str().unwrap_or_default().to_string(),
            state: v["state"].as_str().unwrap_or("OPEN").to_string(),
            url: v["url"].as_str().unwrap_or_default().to_string(),
            head_ref: v["headRefName"].as_str().unwrap_or_default().to_string(),
            base_ref: v["baseRefName"].as_str().unwrap_or_default().to_string(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn git_status_diff_and_checkpoint_work_in_temp_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = dir.path();

        run_cli("git", &["init", "-b", "main"], repo)
            .await
            .expect("init");
        run_cli("git", &["config", "user.email", "test@lingxi.local"], repo)
            .await
            .expect("email");
        run_cli("git", &["config", "user.name", "LingXi Test"], repo)
            .await
            .expect("name");

        std::fs::write(repo.join("README.md"), "v1\n").expect("write");
        git_commit_all(repo, "initial").await.expect("commit");

        let cp_ref = capture_checkpoint_ref(repo, "thread-1", 1)
            .await
            .expect("checkpoint");
        assert!(cp_ref.starts_with("refs/lingxi-t3/checkpoints/thread-1/1"));

        std::fs::write(repo.join("README.md"), "v1\nv2\n").expect("modify");
        let status = git_status(repo).await.expect("status");
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.modified_files, vec!["README.md"]);

        let diffs = git_diff(repo).await.expect("diff");
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].additions, 1);

        restore_checkpoint_ref(repo, &cp_ref)
            .await
            .expect("restore");
        assert_eq!(
            std::fs::read_to_string(repo.join("README.md")).expect("read"),
            "v1\n"
        );
    }
}

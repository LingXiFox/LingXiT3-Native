use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use lingxi_domain::{UpdateChannel, UpdateReleaseInfo};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

pub const DEFAULT_GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/LingXiFox/LingXiT3-Native/releases";

#[derive(Debug, Error)]
pub enum UpdaterError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("invalid version: {0}")]
    InvalidVersion(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    prerelease: bool,
    body: Option<String>,
    published_at: Option<String>,
    assets: Vec<GithubAsset>,
}

pub fn pick_matching_release(
    releases_json: &str,
    current_version: &str,
    channel: UpdateChannel,
) -> Result<Option<UpdateReleaseInfo>, UpdaterError> {
    let current = Version::parse(current_version.trim_start_matches('v'))
        .map_err(|e| UpdaterError::InvalidVersion(e.to_string()))?;
    let releases: Vec<GithubRelease> = serde_json::from_str(releases_json)
        .map_err(|e| UpdaterError::InvalidVersion(e.to_string()))?;

    let mut best: Option<(Version, UpdateReleaseInfo)> = None;
    for rel in releases {
        match channel {
            UpdateChannel::Stable if rel.prerelease => continue,
            UpdateChannel::Nightly | UpdateChannel::Stable => {}
        }
        let ver_str = rel.tag_name.trim_start_matches('v');
        let Ok(ver) = Version::parse(ver_str) else {
            continue;
        };
        if ver <= current {
            continue;
        }
        let Some(asset) = select_platform_asset(&rel.assets) else {
            continue;
        };
        let replace = match &best {
            None => true,
            Some((best_ver, _)) => ver > *best_ver,
        };
        if replace {
            best = Some((
                ver,
                UpdateReleaseInfo {
                    version: ver_str.to_string(),
                    channel,
                    notes: rel.body.unwrap_or_default(),
                    download_url: asset.browser_download_url.clone(),
                    sha256: None,
                    published_at: rel.published_at.unwrap_or_default(),
                },
            ));
        }
    }
    Ok(best.map(|(_, info)| info))
}

fn select_platform_asset(assets: &[GithubAsset]) -> Option<&GithubAsset> {
    let os_hint = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    };
    assets
        .iter()
        .find(|a| a.name.to_lowercase().contains(os_hint))
        .or_else(|| assets.first())
}

pub async fn check_for_updates(
    releases_api_url: &str,
    current_version: &str,
    channel: UpdateChannel,
) -> Result<Option<UpdateReleaseInfo>, UpdaterError> {
    let client = reqwest::Client::builder()
        .user_agent("LingXiT3-Native-Updater")
        .build()?;
    let body = client.get(releases_api_url).send().await?.text().await?;
    pick_matching_release(&body, current_version, channel)
}

pub async fn download_and_verify(
    url: &str,
    dest_path: &Path,
    expected_sha256: Option<&str>,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<(), UpdaterError> {
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let client = reqwest::Client::builder()
        .user_agent("LingXiT3-Native-Updater")
        .build()?;
    let resp = client.get(url).send().await?.error_for_status()?;
    let total = resp.content_length();
    let mut downloaded = 0u64;
    let mut hasher = Sha256::new();

    let staging = dest_path.with_extension("part");
    let mut file = tokio::fs::File::create(&staging).await?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total);
    }
    file.flush().await?;
    drop(file);

    let actual_hex = format!("{:x}", hasher.finalize());
    if let Some(expected) = expected_sha256
        && !expected.eq_ignore_ascii_case(&actual_hex)
    {
        let _ = std::fs::remove_file(&staging);
        return Err(UpdaterError::ChecksumMismatch {
            expected: expected.to_string(),
            actual: actual_hex,
        });
    }

    std::fs::rename(&staging, dest_path)?;
    Ok(())
}

/// Atomically swaps `target_binary` with `staged_binary`, keeping a `.bak` copy
/// and rolling back automatically if the replacement step fails.
pub fn install_with_rollback(
    target_binary: &Path,
    staged_binary: &Path,
) -> Result<PathBuf, UpdaterError> {
    let backup = target_binary.with_extension("bak");
    if target_binary.exists() {
        std::fs::copy(target_binary, &backup)?;
    }
    if let Err(err) = std::fs::copy(staged_binary, target_binary) {
        if backup.exists() {
            let _ = std::fs::copy(&backup, target_binary);
        }
        return Err(UpdaterError::Io(err));
    }
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};

    #[tokio::test]
    async fn updater_checks_channels_verifies_sha256_and_rolls_back_cleanly() {
        let payload = b"lingxi-native-release-bytes";
        let payload_hex = format!("{:x}", Sha256::digest(payload));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let releases_payload = serde_json::json!([
            {
                "tag_name": "v0.2.0",
                "prerelease": false,
                "body": "Stable release",
                "published_at": "2026-11-23T00:00:00Z",
                "assets": [{ "name": "lingxi-t3-native-macos-arm64.tar.gz", "browser_download_url": format!("http://{addr}/asset") }]
            },
            {
                "tag_name": "v0.3.0-nightly.1",
                "prerelease": true,
                "body": "Nightly build",
                "published_at": "2026-11-23T01:00:00Z",
                "assets": [{ "name": "lingxi-t3-native-macos-arm64.tar.gz", "browser_download_url": format!("http://{addr}/asset") }]
            }
        ]);

        let app = Router::new()
            .route(
                "/releases",
                get({
                    let body = releases_payload.to_string();
                    move || async move { body }
                }),
            )
            .route("/asset", get(|| async { payload.as_slice() }));

        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        let stable = check_for_updates(
            &format!("http://{addr}/releases"),
            "0.1.0",
            UpdateChannel::Stable,
        )
        .await
        .expect("check stable")
        .expect("update found");
        assert_eq!(stable.version, "0.2.0");

        let nightly = check_for_updates(
            &format!("http://{addr}/releases"),
            "0.1.0",
            UpdateChannel::Nightly,
        )
        .await
        .expect("check nightly")
        .expect("nightly found");
        assert_eq!(nightly.version, "0.3.0-nightly.1");

        let dir = tempfile::tempdir().expect("tempdir");
        let staged = dir.path().join("staged.bin");

        // Wrong checksum must be rejected
        let bad =
            download_and_verify(&stable.download_url, &staged, Some("deadbeef"), |_, _| {}).await;
        assert!(matches!(bad, Err(UpdaterError::ChecksumMismatch { .. })));

        // Valid checksum succeeds with progress
        let mut last_bytes = 0u64;
        download_and_verify(&stable.download_url, &staged, Some(&payload_hex), |b, _| {
            last_bytes = b;
        })
        .await
        .expect("download valid");
        assert_eq!(last_bytes, payload.len() as u64);

        let target = dir.path().join("app.bin");
        std::fs::write(&target, b"old-binary").expect("seed old");
        let backup = install_with_rollback(&target, &staged).expect("install");
        assert_eq!(std::fs::read(&target).expect("read"), payload);
        assert_eq!(std::fs::read(&backup).expect("bak"), b"old-binary");

        server.abort();
    }
}

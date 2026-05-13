//! Integration tests for syncing to a device with an active `DeviceProfile`.
//!
//! These tests exercise the full path from `sync_to_device` through
//! `remap_pc_files_with_profile` and the `device_template` engine. They
//! verify that:
//!
//! - Files inside the `Podcasts/` managed root are renamed by the template.
//! - Files outside the managed root (e.g. `Playlists/`) are forwarded
//!   verbatim.
//! - The local downloads tree is never modified by a profile-driven sync.
//! - Filename collisions after templating are disambiguated.
//! - Passing `None` as the profile preserves today's verbatim behavior.

use chrono::Utc;
use podcast_tui::config::{DeviceProfile, DownloadConfig};
use podcast_tui::download::DownloadManager;
use podcast_tui::podcast::{Episode, Podcast};
use podcast_tui::storage::{JsonStorage, Storage};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;

/// Build a test podcast with `n` episodes whose `local_path` points at
/// real files inside `downloads_dir`. Returns the podcast plus the list
/// of created file paths so callers can assert the local tree post-sync.
async fn make_podcast_with_episodes(
    storage: &Arc<JsonStorage>,
    downloads_dir: &std::path::Path,
    podcast_title: &str,
    episode_titles: &[&str],
) -> (Podcast, Vec<std::path::PathBuf>) {
    let podcast = Podcast::new(
        podcast_title.to_string(),
        format!("https://example.com/{}.rss", podcast_title),
    );
    storage.save_podcast(&podcast).await.expect("save podcast");

    let podcast_dir = downloads_dir.join(podcast_title);
    fs::create_dir_all(&podcast_dir).await.unwrap();

    let mut local_files = Vec::new();
    for (i, title) in episode_titles.iter().enumerate() {
        let file_name = format!("{:02} - {}.mp3", i + 1, title);
        let local_path = podcast_dir.join(&file_name);
        fs::write(&local_path, format!("audio for {}", title).as_bytes())
            .await
            .unwrap();

        let mut ep = Episode::new(
            podcast.id.clone(),
            title.to_string(),
            format!("https://example.com/{}-{}.mp3", podcast_title, i),
            Utc::now(),
        );
        ep.episode_number = Some((i + 1) as u32);
        ep.local_path = Some(local_path.clone());

        storage
            .save_episode(&podcast.id, &ep)
            .await
            .expect("save episode");
        local_files.push(local_path);
    }

    (podcast, local_files)
}

fn make_profile(template: &str, ascii_only: bool, max: usize) -> DeviceProfile {
    DeviceProfile {
        name: "test-profile".to_string(),
        match_path_contains: None,
        filename_template: template.to_string(),
        max_filename_length: max,
        ascii_only,
        preserve_structure: true,
    }
}

fn make_flat_profile(template: &str, ascii_only: bool, max: usize) -> DeviceProfile {
    DeviceProfile {
        name: "flat-test-profile".to_string(),
        match_path_contains: None,
        filename_template: template.to_string(),
        max_filename_length: max,
        ascii_only,
        preserve_structure: false,
    }
}

#[tokio::test]
async fn test_sync_with_profile_rewrites_filenames() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let downloads_dir = temp.path().join("downloads");
    let device_dir = temp.path().join("device");
    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    storage.initialize().await.unwrap();

    let (_p1, _) = make_podcast_with_episodes(
        &storage,
        &downloads_dir,
        "Hello World Show",
        &["First", "Second", "Third"],
    )
    .await;
    let (_p2, _) =
        make_podcast_with_episodes(&storage, &downloads_dir, "Other Pod", &["A", "B", "C"]).await;

    let manager = DownloadManager::new(
        storage.clone(),
        downloads_dir.clone(),
        DownloadConfig::default(),
    )
    .unwrap();

    let profile = make_profile("{podcast_short}/{track:03} - {title}.{ext}", true, 64);

    let report = manager
        .sync_to_device(
            device_dir.clone(),
            None,
            false,
            false,
            false,
            None,
            Some(profile),
        )
        .await
        .expect("sync should succeed");

    assert_eq!(report.errors.len(), 0, "no errors expected");
    assert_eq!(report.files_copied.len(), 6, "expected 6 files copied");

    // Sample assertions: files land under Podcasts/<podcast_short>/...
    let podcasts_root = device_dir.join("Podcasts");
    assert!(podcasts_root.exists(), "Podcasts/ root should exist");

    // Walk and collect every file under Podcasts/.
    let mut found = Vec::new();
    let mut stack = vec![podcasts_root.clone()];
    while let Some(dir) = stack.pop() {
        let mut rd = fs::read_dir(&dir).await.unwrap();
        while let Some(entry) = rd.next_entry().await.unwrap() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                found.push(p.strip_prefix(&podcasts_root).unwrap().to_path_buf());
            }
        }
    }
    assert_eq!(found.len(), 6, "expected 6 device files, got {:?}", found);

    // Every rendered name should be a non-empty .mp3 file. Exact spacing
    // depends on the sanitizer (ascii_only collapses some whitespace).
    for f in &found {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        assert!(name.ends_with(".mp3"), "unexpected file name: {}", name);
        assert!(!name.is_empty(), "file name should not be empty");
    }
}

#[tokio::test]
async fn test_sync_with_profile_preserves_local_downloads() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let downloads_dir = temp.path().join("downloads");
    let device_dir = temp.path().join("device");
    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    storage.initialize().await.unwrap();
    let (_p, local_files) =
        make_podcast_with_episodes(&storage, &downloads_dir, "Snapshot Pod", &["alpha", "beta"])
            .await;

    let manager = DownloadManager::new(
        storage.clone(),
        downloads_dir.clone(),
        DownloadConfig::default(),
    )
    .unwrap();
    let profile = make_profile("{podcast_short}/{title}.{ext}", false, 128);

    // Snapshot file contents before sync.
    let mut before = Vec::new();
    for f in &local_files {
        before.push((f.clone(), fs::read(f).await.unwrap()));
    }

    manager
        .sync_to_device(
            device_dir.clone(),
            None,
            false,
            false,
            false,
            None,
            Some(profile),
        )
        .await
        .expect("sync should succeed");

    // Verify the local files are byte-identical and still in place.
    for (f, original) in &before {
        assert!(f.exists(), "local file should still exist: {}", f.display());
        let after = fs::read(f).await.unwrap();
        assert_eq!(
            &after,
            original,
            "local file modified by sync: {}",
            f.display()
        );
    }
}

#[tokio::test]
async fn test_sync_without_profile_byte_identical_to_today() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let downloads_dir = temp.path().join("downloads");
    let device_dir = temp.path().join("device");
    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    // No storage entries — exercises the fall-through path: when the
    // remap helper cannot find an episode for a file it must still copy
    // the file under its original name. With profile=None, this code
    // path is skipped entirely.
    let podcast_dir = downloads_dir.join("Verbatim Pod");
    fs::create_dir_all(&podcast_dir).await.unwrap();
    let f1 = podcast_dir.join("01 - intro.mp3");
    fs::write(&f1, b"intro audio").await.unwrap();

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir));
    storage.initialize().await.unwrap();
    let manager = DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

    let report = manager
        .sync_to_device(device_dir.clone(), None, false, false, false, None, None)
        .await
        .expect("sync should succeed");

    assert_eq!(report.errors.len(), 0);
    assert_eq!(report.files_copied.len(), 1);
    assert!(
        device_dir
            .join("Podcasts")
            .join("Verbatim Pod")
            .join("01 - intro.mp3")
            .exists(),
        "expected verbatim copy under Podcasts/Verbatim Pod/"
    );
}

#[tokio::test]
async fn test_sync_with_profile_disambiguates_collisions() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let downloads_dir = temp.path().join("downloads");
    let device_dir = temp.path().join("device");
    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    storage.initialize().await.unwrap();

    // Two episodes with the same rendered name (template uses only {title}
    // with no track number → guaranteed collision).
    let _ = make_podcast_with_episodes(
        &storage,
        &downloads_dir,
        "Collide Pod",
        &["Episode", "Episode"],
    )
    .await;

    let manager = DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();
    let profile = make_profile("{title}.{ext}", false, 128);

    let report = manager
        .sync_to_device(
            device_dir.clone(),
            None,
            false,
            false,
            false,
            None,
            Some(profile),
        )
        .await
        .expect("sync should succeed");

    assert_eq!(report.errors.len(), 0);
    // After disambiguation, both files should land on the device — one
    // under the rendered name, one with the episode-id suffix.
    assert_eq!(
        report.files_copied.len(),
        2,
        "both colliding files should be copied: {:?}",
        report.files_copied
    );
}

/// A malformed filename template must surface as a hard `SyncError`
/// rather than silently falling back to verbatim names. The
/// `DeviceProfile` config schema explicitly promises this behavior, and
/// PR #219 review feedback called out the previous silent fallback.
#[tokio::test]
async fn test_sync_with_profile_malformed_template_returns_error() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let downloads_dir = temp.path().join("downloads");
    let device_dir = temp.path().join("device");
    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    storage.initialize().await.unwrap();

    let _ = make_podcast_with_episodes(&storage, &downloads_dir, "Pod", &["E1"]).await;

    let manager = DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();
    // {bogus_token} does not exist in the engine — must error.
    let profile = make_profile("{bogus_token}.{ext}", false, 128);

    let result = manager
        .sync_to_device(
            device_dir.clone(),
            None,
            false,
            false,
            false,
            None,
            Some(profile),
        )
        .await;

    assert!(
        result.is_err(),
        "malformed template should produce a SyncError, got: {:?}",
        result
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("template") || err_msg.contains("Template"),
        "error message should mention template, got: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// Flat-layout (preserve_structure: false) tests — issue #221
// ---------------------------------------------------------------------------

/// When `preserve_structure: false` is set on the active profile, podcast
/// files must land at the device root (no `Podcasts/` subdir) and any `/`
/// in the rendered template output must be flattened to `_`.
#[tokio::test]
async fn test_sync_flat_layout_writes_to_device_root() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let downloads_dir = temp.path().join("downloads");
    let device_dir = temp.path().join("device");
    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    storage.initialize().await.unwrap();

    let _ = make_podcast_with_episodes(&storage, &downloads_dir, "Flat Pod", &["First", "Second"])
        .await;

    let manager = DownloadManager::new(
        storage.clone(),
        downloads_dir.clone(),
        DownloadConfig::default(),
    )
    .unwrap();

    // Template includes a `/` separator — must be flattened to `_` in flat mode.
    let profile = make_flat_profile("{podcast_short}/{track:03} - {title}.{ext}", true, 64);

    let report = manager
        .sync_to_device(
            device_dir.clone(),
            None,
            false,
            false,
            false,
            None,
            Some(profile),
        )
        .await
        .expect("sync should succeed");

    assert_eq!(report.errors.len(), 0, "no errors expected");
    assert_eq!(report.files_copied.len(), 2, "expected 2 files copied");

    // No Podcasts/ subdir should be created.
    assert!(
        !device_dir.join("Podcasts").exists(),
        "Podcasts/ should NOT exist in flat mode"
    );

    // Walk the device root non-recursively and assert files land there.
    let mut found = Vec::new();
    let mut entries = fs::read_dir(&device_dir).await.unwrap();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        let p = entry.path();
        if p.is_file() {
            found.push(p.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    assert_eq!(
        found.len(),
        2,
        "expected 2 files at device root, got {:?}",
        found
    );

    // Each filename must be flat (no path separators) and contain the
    // flattened separator (`_`) where the template had `/`.
    for name in &found {
        assert!(name.ends_with(".mp3"), "unexpected file name: {}", name);
        assert!(
            !name.contains('/') && !name.contains('\\'),
            "filename should be flat, got: {}",
            name
        );
        assert!(
            name.contains('_'),
            "expected flattened separator in name: {}",
            name
        );
    }
}

/// In flat mode, `delete_orphans: true` must skip podcast orphan deletion
/// entirely (because podcast files share the device root with arbitrary
/// user files) and surface a warning in the report.
#[tokio::test]
async fn test_sync_flat_layout_skips_orphan_deletion_with_warning() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let downloads_dir = temp.path().join("downloads");
    let device_dir = temp.path().join("device");
    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    storage.initialize().await.unwrap();

    let _ = make_podcast_with_episodes(&storage, &downloads_dir, "Pod", &["alpha"]).await;

    // Pre-existing user file at the device root that the sync MUST NOT touch.
    let user_file = device_dir.join("vacation_photo.jpg");
    fs::write(&user_file, b"my photo bytes").await.unwrap();

    // Pre-existing "stale" audio file at the root that COULD plausibly be a
    // previous flat-mode sync output, but the app cannot tell — must also
    // be left alone in this conservative-first-cut implementation.
    let stale_audio = device_dir.join("Old Pod_Old Episode.mp3");
    fs::write(&stale_audio, b"stale audio").await.unwrap();

    let manager = DownloadManager::new(
        storage.clone(),
        downloads_dir.clone(),
        DownloadConfig::default(),
    )
    .unwrap();
    let profile = make_flat_profile("{podcast_short}_{title}.{ext}", false, 128);

    let report = manager
        .sync_to_device(
            device_dir.clone(),
            None,
            true, // delete_orphans on
            false,
            false,
            None,
            Some(profile),
        )
        .await
        .expect("sync should succeed");

    assert_eq!(report.errors.len(), 0, "no errors expected");
    assert_eq!(
        report.files_deleted.len(),
        0,
        "no orphan deletion should occur in flat mode, got: {:?}",
        report.files_deleted
    );
    assert!(
        !report.warnings.is_empty(),
        "expected a flat-mode warning to be surfaced"
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.to_lowercase().contains("orphan") && w.to_lowercase().contains("flat")),
        "warning text should explain the skipped orphan deletion: {:?}",
        report.warnings
    );

    // Both pre-existing files must still be on disk.
    assert!(
        user_file.exists(),
        "user's unrelated file at device root must not be deleted"
    );
    assert!(
        stale_audio.exists(),
        "stale audio at device root must not be deleted in flat mode"
    );
}

/// In flat mode, syncing a second time must detect already-present files
/// (by name + size) at the device root and mark them as skipped — not
/// re-copy them every run.
#[tokio::test]
async fn test_sync_flat_layout_skips_already_synced_files() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let downloads_dir = temp.path().join("downloads");
    let device_dir = temp.path().join("device");
    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    storage.initialize().await.unwrap();

    let _ = make_podcast_with_episodes(&storage, &downloads_dir, "Pod", &["one", "two"]).await;

    let manager = DownloadManager::new(
        storage.clone(),
        downloads_dir.clone(),
        DownloadConfig::default(),
    )
    .unwrap();
    let profile = make_flat_profile("{podcast_short}_{title}.{ext}", false, 128);

    // First sync — everything new.
    let r1 = manager
        .sync_to_device(
            device_dir.clone(),
            None,
            false,
            false,
            false,
            None,
            Some(profile.clone()),
        )
        .await
        .expect("first sync");
    assert_eq!(r1.files_copied.len(), 2);
    assert_eq!(r1.files_skipped.len(), 0);

    // Second sync with the same inputs — all files should be skipped.
    let r2 = manager
        .sync_to_device(
            device_dir.clone(),
            None,
            false,
            false,
            false,
            None,
            Some(profile),
        )
        .await
        .expect("second sync");
    assert_eq!(
        r2.files_copied.len(),
        0,
        "no files should re-copy on second sync, got: {:?}",
        r2.files_copied
    );
    assert_eq!(
        r2.files_skipped.len(),
        2,
        "both files should be skipped, got: {:?}",
        r2.files_skipped
    );
}

/// In flat mode, playlist files (under `Playlists/`) must KEEP their
/// directory structure and orphan deletion must still reconcile them.
/// Flat layout only flattens the podcast tree.
#[tokio::test]
async fn test_sync_flat_layout_preserves_playlists_structure() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let downloads_dir = temp.path().join("downloads");
    let playlists_dir = temp.path().join("playlists_data");
    let device_dir = temp.path().join("device");
    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    // Build a minimal playlist on disk: playlists_data/<name>/audio/<file>
    let playlist_audio = playlists_dir.join("My List").join("audio");
    fs::create_dir_all(&playlist_audio).await.unwrap();
    fs::write(playlist_audio.join("001-track.mp3"), b"playlist audio")
        .await
        .unwrap();

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    storage.initialize().await.unwrap();

    let manager = DownloadManager::new(
        storage.clone(),
        downloads_dir.clone(),
        DownloadConfig::default(),
    )
    .unwrap();
    let profile = make_flat_profile("{podcast_short}_{title}.{ext}", false, 128);

    let report = manager
        .sync_to_device(
            device_dir.clone(),
            Some(playlists_dir.clone()),
            true,
            false,
            false,
            None,
            Some(profile),
        )
        .await
        .expect("sync should succeed");

    assert_eq!(report.errors.len(), 0);

    // Playlist file must land under Playlists/My List/... (sync strips the
    // intermediate `audio/` directory; see `scan_playlists_for_sync`).
    let playlist_target = device_dir
        .join("Playlists")
        .join("My List")
        .join("001-track.mp3");
    assert!(
        playlist_target.exists(),
        "playlist file should preserve structure under Playlists/, expected: {}",
        playlist_target.display()
    );

    // Flat-mode warning must still be surfaced because delete_orphans was on.
    assert!(
        !report.warnings.is_empty(),
        "flat-mode warning should be surfaced even when only playlists are present"
    );
}

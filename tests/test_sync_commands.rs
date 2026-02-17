// Integration tests for sync commands
//
// These tests verify that sync operations (copy and dry-run) work correctly
// at the DownloadManager level, which is the async handler invoked by UI commands.

use podcast_tui::config::Config;
use podcast_tui::download::DownloadManager;
use podcast_tui::storage::JsonStorage;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_sync_to_device_copies_files() {
    // Create temporary directories
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("data");
    let downloads_dir = temp_dir.path().join("downloads");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    // Create a test audio file in downloads
    let podcast_dir = downloads_dir.join("Test Podcast");
    fs::create_dir_all(&podcast_dir).await.unwrap();
    let test_file = podcast_dir.join("episode1.mp3");
    fs::write(&test_file, b"test audio content").await.unwrap();

    // Create config with test directories
    let mut config = Config::default();
    config.storage.data_directory = Some(data_dir.to_string_lossy().to_string());
    config.downloads.directory = downloads_dir.to_string_lossy().to_string();

    // Initialize components
    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    let download_manager = Arc::new(
        DownloadManager::new(
            storage.clone(),
            downloads_dir.clone(),
            config.downloads.clone(),
        )
        .unwrap(),
    );

    // Test sync directly (simulating what the command would trigger)
    let device_path = device_dir.clone();
    let result = download_manager
        .sync_to_device(device_path.clone(), None, false, false)
        .await;

    assert!(result.is_ok(), "Sync should succeed");
    let report = result.unwrap();
    assert_eq!(report.files_copied.len(), 1, "Should copy 1 file");
    assert_eq!(report.errors.len(), 0, "Should have no errors");

    // Verify file was copied
    let device_podcast_dir = device_dir.join("Podcasts").join("Test Podcast");
    assert!(
        device_podcast_dir.exists(),
        "Podcast directory should exist on device"
    );
    assert!(
        device_podcast_dir.join("episode1.mp3").exists(),
        "Episode file should exist on device"
    );
}

#[tokio::test]
async fn test_sync_dry_run() {
    // Create temporary directories
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("data");
    let downloads_dir = temp_dir.path().join("downloads");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&data_dir).await.unwrap();
    fs::create_dir_all(&downloads_dir).await.unwrap();
    fs::create_dir_all(&device_dir).await.unwrap();

    // Create a test audio file in downloads
    let podcast_dir = downloads_dir.join("Test Podcast");
    fs::create_dir_all(&podcast_dir).await.unwrap();
    let test_file = podcast_dir.join("episode1.mp3");
    fs::write(&test_file, b"test audio content").await.unwrap();

    // Create config with test directories
    let mut config = Config::default();
    config.storage.data_directory = Some(data_dir.to_string_lossy().to_string());
    config.downloads.directory = downloads_dir.to_string_lossy().to_string();

    // Initialize components
    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    let download_manager = Arc::new(
        DownloadManager::new(
            storage.clone(),
            downloads_dir.clone(),
            config.downloads.clone(),
        )
        .unwrap(),
    );

    // Test dry run sync
    let device_path = device_dir.clone();
    let result = download_manager
        .sync_to_device(device_path.clone(), None, false, true)
        .await;

    assert!(result.is_ok(), "Dry run sync should succeed");
    let report = result.unwrap();
    assert_eq!(report.files_copied.len(), 1, "Should report 1 file to copy");
    assert_eq!(report.errors.len(), 0, "Should have no errors");

    // Verify file was NOT copied (dry run)
    let device_podcast_dir = device_dir.join("Podcasts").join("Test Podcast");
    assert!(
        !device_podcast_dir.exists(),
        "Podcast directory should NOT exist on device after dry run"
    );
}

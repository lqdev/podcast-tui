//! Integration test for the unsubscribe feature that deletes downloaded episodes
//!
//! This test verifies that when unsubscribing from a podcast,
//! all downloaded episodes are automatically deleted.

use anyhow::Result;
use podcast_tui::{
    config::DownloadConfig,
    download::DownloadManager,
    podcast::{subscription::SubscriptionManager, Episode, EpisodeStatus, Podcast},
    storage::{EpisodeId, JsonStorage, PodcastId, Storage},
};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_unsubscribe_deletes_downloaded_episodes() -> Result<()> {
    // Setup test environment
    let temp_dir = TempDir::new()?;
    let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
    storage.initialize().await?;

    let download_manager = Arc::new(DownloadManager::new(
        storage.clone(),
        temp_dir.path().join("downloads"),
        DownloadConfig::default(),
    )?);

    let subscription_manager = Arc::new(SubscriptionManager::with_download_manager(
        storage.clone(),
        download_manager.clone(),
    ));

    // Create a test podcast
    let podcast_id = PodcastId::new();
    let mut podcast = Podcast::new(
        "Test Podcast".to_string(),
        "https://example.com/feed.xml".to_string(),
    );
    podcast.id = podcast_id.clone();

    // Create test episodes
    let episode1_id = EpisodeId::new();
    let episode2_id = EpisodeId::new();

    let mut episode1 = Episode::new(
        podcast_id.clone(),
        "Episode 1".to_string(),
        "https://example.com/episode1.mp3".to_string(),
        chrono::Utc::now(),
    );
    episode1.id = episode1_id.clone();
    episode1.status = EpisodeStatus::Downloaded;

    let mut episode2 = Episode::new(
        podcast_id.clone(),
        "Episode 2".to_string(),
        "https://example.com/episode2.mp3".to_string(),
        chrono::Utc::now(),
    );
    episode2.id = episode2_id.clone();
    episode2.status = EpisodeStatus::New; // Not downloaded

    // Add episodes to podcast
    podcast.add_episode(episode1_id.clone());
    podcast.add_episode(episode2_id.clone());

    // Save everything to storage
    storage.save_podcast(&podcast).await?;
    storage.save_episode(&podcast_id, &episode1).await?;
    storage.save_episode(&podcast_id, &episode2).await?;

    // Create fake downloaded file in the correct folder structure
    // The DownloadManager uses the podcast title as folder name with default config
    let podcast_folder_name = "Test Podcast"; // Based on sanitize_filename logic
    let podcast_download_dir = temp_dir.path().join("downloads").join(podcast_folder_name);
    tokio::fs::create_dir_all(&podcast_download_dir).await?;

    let episode_file_path = podcast_download_dir.join("episode1.mp3");
    tokio::fs::write(&episode_file_path, b"fake audio data").await?;

    // Update episode1 to point to the correct file path
    episode1.local_path = Some(episode_file_path.clone());
    storage.save_episode(&podcast_id, &episode1).await?;

    // Verify initial state
    let loaded_episode1 = storage.load_episode(&podcast_id, &episode1_id).await?;
    assert_eq!(loaded_episode1.status, EpisodeStatus::Downloaded);
    assert!(loaded_episode1.local_path.as_ref().unwrap().exists());

    let loaded_episode2 = storage.load_episode(&podcast_id, &episode2_id).await?;
    assert_eq!(loaded_episode2.status, EpisodeStatus::New);

    // Unsubscribe from the podcast
    subscription_manager.unsubscribe(&podcast_id).await?;

    // Verify podcast is deleted
    assert!(!storage.podcast_exists(&podcast_id).await?);

    // Verify episodes are deleted from storage (should fail to load)
    assert!(storage
        .load_episode(&podcast_id, &episode1_id)
        .await
        .is_err());
    assert!(storage
        .load_episode(&podcast_id, &episode2_id)
        .await
        .is_err());

    // Verify downloaded file is deleted
    assert!(!episode_file_path.exists());

    // Verify empty directory is cleaned up
    assert!(!podcast_download_dir.exists());

    Ok(())
}

#[tokio::test]
async fn test_unsubscribe_handles_missing_download_manager() -> Result<()> {
    // Setup test environment without download manager
    let temp_dir = TempDir::new()?;
    let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
    storage.initialize().await?;

    // Create subscription manager without download manager
    let subscription_manager = Arc::new(SubscriptionManager::new(storage.clone()));

    // Create a test podcast
    let podcast_id = PodcastId::new();
    let mut podcast = Podcast::new(
        "Test Podcast".to_string(),
        "https://example.com/feed.xml".to_string(),
    );
    podcast.id = podcast_id.clone();

    // Save podcast to storage
    storage.save_podcast(&podcast).await?;

    // Verify podcast exists
    assert!(storage.podcast_exists(&podcast_id).await?);

    // Unsubscribe should work even without download manager
    subscription_manager.unsubscribe(&podcast_id).await?;

    // Verify podcast is deleted
    assert!(!storage.podcast_exists(&podcast_id).await?);

    Ok(())
}

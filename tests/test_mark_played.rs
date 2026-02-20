use podcast_tui::podcast::{Episode, EpisodeStatus, Podcast};
use podcast_tui::storage::{JsonStorage, PodcastId, Storage};
use std::sync::Arc;
use tempfile::TempDir;

async fn setup_storage() -> (TempDir, Arc<JsonStorage>) {
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let data_dir = tmp.path().join("data");
    tokio::fs::create_dir_all(&data_dir)
        .await
        .expect("Failed to create data dir");
    let storage = Arc::new(JsonStorage::with_data_dir(data_dir));
    storage
        .initialize()
        .await
        .expect("Failed to initialize storage");
    (tmp, storage)
}

async fn create_podcast_and_episode(storage: &Arc<JsonStorage>) -> (PodcastId, Episode) {
    let mut podcast = Podcast::new(
        "Test Podcast".to_string(),
        "https://example.com/feed.xml".to_string(),
    );
    let podcast_id = podcast.id.clone();

    let episode = Episode::new(
        podcast_id.clone(),
        "Test Episode".to_string(),
        "https://example.com/ep1.mp3".to_string(),
        chrono::Utc::now(),
    );
    let episode_id = episode.id.clone();

    podcast.add_episode(episode_id);
    storage.save_podcast(&podcast).await.unwrap();
    storage.save_episode(&podcast_id, &episode).await.unwrap();

    (podcast_id, episode)
}

#[tokio::test]
async fn test_mark_played_persists_to_storage() {
    // Arrange
    let (_tmp, storage) = setup_storage().await;
    let (podcast_id, episode) = create_podcast_and_episode(&storage).await;
    let episode_id = episode.id.clone();
    assert!(!episode.is_played());

    // Act: load, mark played, save
    let mut loaded = storage
        .load_episode(&podcast_id, &episode_id)
        .await
        .expect("should load episode");
    loaded.mark_played();
    storage
        .save_episode(&podcast_id, &loaded)
        .await
        .expect("should save episode");

    // Assert: reload and verify
    let reloaded = storage
        .load_episode(&podcast_id, &episode_id)
        .await
        .expect("should reload episode");
    assert!(reloaded.is_played());
    assert_eq!(reloaded.status, EpisodeStatus::Played);
}

#[tokio::test]
async fn test_mark_unplayed_persists_to_storage() {
    // Arrange
    let (_tmp, storage) = setup_storage().await;
    let (podcast_id, mut episode) = create_podcast_and_episode(&storage).await;
    let episode_id = episode.id.clone();

    // Start as played
    episode.mark_played();
    storage
        .save_episode(&podcast_id, &episode)
        .await
        .expect("should save");

    // Act: load, mark unplayed, save
    let mut loaded = storage
        .load_episode(&podcast_id, &episode_id)
        .await
        .expect("should load episode");
    loaded.mark_unplayed();
    storage
        .save_episode(&podcast_id, &loaded)
        .await
        .expect("should save episode");

    // Assert: reload and verify
    let reloaded = storage
        .load_episode(&podcast_id, &episode_id)
        .await
        .expect("should reload episode");
    assert!(!reloaded.is_played());
    assert_ne!(reloaded.status, EpisodeStatus::Played);
}

#[tokio::test]
async fn test_mark_already_played_episode_as_played_is_noop() {
    // Arrange
    let (_tmp, storage) = setup_storage().await;
    let (podcast_id, mut episode) = create_podcast_and_episode(&storage).await;
    let episode_id = episode.id.clone();

    // Mark played once and record play_count
    episode.mark_played();
    storage
        .save_episode(&podcast_id, &episode)
        .await
        .expect("should save");
    let first_save = storage
        .load_episode(&podcast_id, &episode_id)
        .await
        .expect("should load");
    let play_count_after_first = first_save.play_count;

    // Act: mark played again — should be a no-op (count unchanged)
    let mut loaded = storage
        .load_episode(&podcast_id, &episode_id)
        .await
        .expect("should load episode");
    loaded.mark_played();
    storage
        .save_episode(&podcast_id, &loaded)
        .await
        .expect("should save");

    // Assert: play_count unchanged (mark_played is idempotent)
    let reloaded = storage
        .load_episode(&podcast_id, &episode_id)
        .await
        .expect("should reload");
    assert!(reloaded.is_played());
    assert_eq!(reloaded.play_count, play_count_after_first);
}

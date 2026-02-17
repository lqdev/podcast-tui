use podcast_tui::config::Config;
use podcast_tui::download::DownloadManager;
use podcast_tui::playlist::{
    auto_generator::TodayGenerator, manager::PlaylistManager, RefreshPolicy,
};
use podcast_tui::podcast::{Episode, EpisodeStatus, Podcast};
use podcast_tui::storage::{EpisodeId, JsonStorage, PodcastId, Storage};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;

struct TestSetup {
    _tmp: TempDir,
    storage: Arc<JsonStorage>,
    download_manager: Arc<DownloadManager<JsonStorage>>,
    playlist_manager: PlaylistManager,
    today_generator: TodayGenerator,
    data_dir: std::path::PathBuf,
    downloads_dir: std::path::PathBuf,
}

async fn setup() -> TestSetup {
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let data_dir = tmp.path().join("data");
    let downloads_dir = tmp.path().join("downloads");
    fs::create_dir_all(&data_dir)
        .await
        .expect("Failed to create data dir");
    fs::create_dir_all(&downloads_dir)
        .await
        .expect("Failed to create downloads dir");

    let storage = Arc::new(JsonStorage::with_data_dir(data_dir.clone()));
    storage
        .initialize()
        .await
        .expect("Failed to initialize storage");

    let mut config = Config::default();
    config.downloads.directory = downloads_dir.to_string_lossy().to_string();
    let download_manager = Arc::new(
        DownloadManager::new(
            storage.clone(),
            downloads_dir.clone(),
            config.downloads.clone(),
        )
        .expect("Failed to create download manager"),
    );

    let playlists_dir = data_dir.join("Playlists");
    let playlist_manager = PlaylistManager::new(
        storage.clone(),
        download_manager.clone(),
        playlists_dir.clone(),
    );
    let today_generator =
        TodayGenerator::new(storage.clone(), download_manager.clone(), playlists_dir);

    TestSetup {
        _tmp: tmp,
        storage,
        download_manager,
        playlist_manager,
        today_generator,
        data_dir,
        downloads_dir,
    }
}

async fn seed_downloaded_episode(
    setup: &TestSetup,
    title: &str,
    published: chrono::DateTime<chrono::Utc>,
) -> (PodcastId, EpisodeId) {
    let podcast_id = PodcastId::new();
    let mut podcast = Podcast::new(
        "Integration Podcast".to_string(),
        "https://example.com/feed.xml".to_string(),
    );
    podcast.id = podcast_id.clone();
    setup
        .storage
        .save_podcast(&podcast)
        .await
        .expect("Failed to save podcast");

    let episode_id = EpisodeId::new();
    let mut episode = Episode::new(
        podcast_id.clone(),
        title.to_string(),
        "https://example.com/audio.mp3".to_string(),
        published,
    );
    episode.id = episode_id.clone();
    episode.status = EpisodeStatus::Downloaded;
    let file_path = setup.downloads_dir.join(format!("{title}.mp3"));
    fs::write(&file_path, b"integration-audio")
        .await
        .expect("Failed to create audio file");
    episode.local_path = Some(file_path);

    setup
        .storage
        .save_episode(&podcast_id, &episode)
        .await
        .expect("Failed to save episode");

    (podcast_id, episode_id)
}

#[tokio::test]
async fn test_create_playlist_and_add_episode() {
    let setup = setup().await;
    let (podcast_id, episode_id) =
        seed_downloaded_episode(&setup, "integration-episode", chrono::Utc::now()).await;

    let playlist = setup
        .playlist_manager
        .create_playlist("Morning Commute", None)
        .await
        .expect("Failed to create playlist");
    setup
        .playlist_manager
        .add_episode_to_playlist(&playlist.id, &podcast_id, &episode_id)
        .await
        .expect("Failed to add episode to playlist");

    let reloaded = setup
        .playlist_manager
        .get_playlist(&playlist.id)
        .await
        .expect("Failed to reload playlist");
    assert_eq!(reloaded.episodes.len(), 1);
    assert_eq!(reloaded.episodes[0].order, 1);

    let audio_dir = setup
        .data_dir
        .join("Playlists")
        .join("Morning Commute")
        .join("audio");
    assert!(audio_dir.exists());
    assert!(audio_dir.join("001-integration-episode.mp3").exists());
}

#[tokio::test]
async fn test_today_playlist_refresh() {
    let setup = setup().await;
    let (_recent_podcast, recent_episode) = seed_downloaded_episode(
        &setup,
        "recent-episode",
        chrono::Utc::now() - chrono::Duration::hours(2),
    )
    .await;
    let (_old_podcast, old_episode) = seed_downloaded_episode(
        &setup,
        "old-episode",
        chrono::Utc::now() - chrono::Duration::hours(30),
    )
    .await;

    let result = setup
        .today_generator
        .ensure_today_playlist_exists(RefreshPolicy::Daily)
        .await
        .expect("Failed to ensure today playlist");
    assert_eq!(result.name, "Today");

    let refresh = setup
        .today_generator
        .refresh()
        .await
        .expect("Failed to refresh today playlist");
    assert!(refresh
        .playlist
        .episodes
        .iter()
        .any(|entry| entry.episode_id == recent_episode));
    assert!(!refresh
        .playlist
        .episodes
        .iter()
        .any(|entry| entry.episode_id == old_episode));
}

#[tokio::test]
async fn test_sync_with_playlists_creates_sibling_device_dirs() {
    let setup = setup().await;
    let device_dir = setup.data_dir.join("device");
    fs::create_dir_all(&device_dir)
        .await
        .expect("Failed to create device dir");

    let podcast_dir = setup.downloads_dir.join("Test Podcast");
    fs::create_dir_all(&podcast_dir)
        .await
        .expect("Failed to create podcast dir");
    fs::write(podcast_dir.join("episode1.mp3"), b"podcast-file")
        .await
        .expect("Failed to write podcast file");

    let playlist_audio = setup
        .data_dir
        .join("Playlists")
        .join("Morning Commute")
        .join("audio");
    fs::create_dir_all(&playlist_audio)
        .await
        .expect("Failed to create playlist audio dir");
    fs::write(playlist_audio.join("001-track.mp3"), b"playlist-file")
        .await
        .expect("Failed to write playlist file");

    let report = setup
        .download_manager
        .sync_to_device(
            device_dir.clone(),
            Some(setup.data_dir.join("Playlists")),
            false,
            false,
        )
        .await
        .expect("Sync should succeed");

    assert_eq!(report.files_copied.len(), 2);
    assert!(device_dir
        .join("Podcasts")
        .join("Test Podcast")
        .join("episode1.mp3")
        .exists());
    assert!(device_dir
        .join("Playlists")
        .join("Morning Commute")
        .join("001-track.mp3")
        .exists());
}

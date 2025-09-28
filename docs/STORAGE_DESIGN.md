# Storage Abstraction Design

The storage layer is designed around traits that allow for easy swapping of implementations.

## Core Storage Traits

```rust
// Core storage trait that all implementations must satisfy
pub trait Storage: Send + Sync {
    type Error;
    
    // Podcast operations
    async fn save_podcast(&self, podcast: &Podcast) -> Result<(), Self::Error>;
    async fn load_podcast(&self, id: &str) -> Result<Option<Podcast>, Self::Error>;
    async fn list_podcasts(&self) -> Result<Vec<Podcast>, Self::Error>;
    async fn delete_podcast(&self, id: &str) -> Result<(), Self::Error>;
    
    // Episode operations
    async fn save_episode(&self, episode: &Episode) -> Result<(), Self::Error>;
    async fn load_episode(&self, id: &str) -> Result<Option<Episode>, Self::Error>;
    async fn list_episodes(&self, podcast_id: &str) -> Result<Vec<Episode>, Self::Error>;
    async fn delete_episode(&self, id: &str) -> Result<(), Self::Error>;
    
    // Playlist operations
    async fn save_playlist(&self, playlist: &Playlist) -> Result<(), Self::Error>;
    async fn load_playlist(&self, id: &str) -> Result<Option<Playlist>, Self::Error>;
    async fn list_playlists(&self) -> Result<Vec<Playlist>, Self::Error>;
    async fn delete_playlist(&self, id: &str) -> Result<(), Self::Error>;
    
    // Statistics operations
    async fn save_stats(&self, stats: &Statistics) -> Result<(), Self::Error>;
    async fn load_stats(&self) -> Result<Option<Statistics>, Self::Error>;
    
    // Backup/restore operations
    async fn backup(&self, path: &Path) -> Result<(), Self::Error>;
    async fn restore(&self, path: &Path) -> Result<(), Self::Error>;
}
```

## JSON Implementation

The JSON implementation stores data in organized files:

```
data/
├── podcasts/
│   ├── podcast_1.json
│   ├── podcast_2.json
│   └── ...
├── episodes/
│   ├── podcast_1/
│   │   ├── episode_1.json
│   │   ├── episode_2.json
│   │   └── ...
│   └── ...
├── playlists/
│   ├── playlist_1.json
│   └── ...
├── stats.json
└── config.json
```

This design allows for:
- Easy manual editing of data files
- Simple backup (copy directory)
- Future implementations (SQLite, remote storage, etc.)
- Clean separation of concerns
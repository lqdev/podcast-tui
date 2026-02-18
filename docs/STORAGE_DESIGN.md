# Storage Abstraction Design

The storage layer is designed around traits that allow for easy swapping of implementations.

## Core Storage Traits

The storage trait covers:

- Podcast CRUD operations
- Episode CRUD and per-podcast episode listing
- Playlist CRUD/list/existence operations
- Statistics persistence
- Backup/restore workflows

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
│   ├── My Playlist/
│   │   ├── playlist.json
│   │   └── audio/
│   │       ├── 001-episode.mp3
│   │       └── ...
│   └── ...
├── stats.json
└── config.json
```

This design allows for:
- Easy manual editing of data files
- Simple backup (copy directory)
- Future implementations (SQLite, remote storage, etc.)
- Clean separation of concerns

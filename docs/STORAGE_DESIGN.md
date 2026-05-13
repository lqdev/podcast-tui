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

## Cache Performance (issue #206)

The in-memory cache (#204) and persistent `cache_index.json` (#205) were
benchmarked against a synthetic fixture of 30 podcasts × 200 episodes
(6,000 total episodes). Results from `cargo run --release --example bench_storage_load`:

| config              | initialize | first traversal | subsequent traversal |
|---------------------|-----------:|----------------:|---------------------:|
| no cache            |       0 ms |          481 ms |               489 ms |
| cache (cold build)  |     456 ms |            0 ms |                 0 ms |
| cache (warm)        |      13 ms |            0 ms |                 0 ms |

**Warm-cache speedup vs no-cache traversal: ~946× — well above the 10×
target set by epic #202.**

### Reading the table

- **no cache** uses `JsonStorage::with_cache(false)`. Every `list_podcasts` /
  `load_episodes` call hits disk. This is the pre-#204 baseline.
- **cache (cold build)** is the first launch after install: no
  `cache_index.json` yet on disk. `initialize()` builds the in-memory
  snapshot from the per-podcast / per-episode JSON files, so the build cost
  shows up here (456 ms in the table). All subsequent reads are in-memory
  hits (0 ms).
- **cache (warm)** is every subsequent launch: `initialize()` deserialises
  `cache_index.json` in one shot (~13 ms for 6,000 episodes); reads are pure
  in-memory hits.

### Tuning notes

- **Flush interval**: the background flush task ticks every
  `CACHE_FLUSH_INTERVAL` (5 s). The 13 ms warm-init time on a 6,000-episode
  fixture leaves no pressure to lower it; the dirty-bit gate already skips
  the write when nothing changed.
- **Snapshot shape**: a single `cache_index.json` is plenty. Splitting per
  podcast would only matter if warm init crossed the ~200 ms threshold,
  which the benchmark shows is more than an order of magnitude away.
- **Compression**: not worth it. The dataset is small and the I/O time is
  already negligible compared to the JSON parse cost.

### Reproducing

```bash
cargo run --release --example bench_storage_load
# Custom fixture size:
BENCH_PODCASTS=10 BENCH_EPISODES=50 cargo run --release --example bench_storage_load
```

The benchmark seeds a fresh `TempDir` each run, so it is safe to execute
without touching real user data.

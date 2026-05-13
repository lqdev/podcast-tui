//! Storage cache benchmark — issue #206.
//!
//! Generates a synthetic fixture (default 30 podcasts × 200 episodes), then
//! measures three configurations against the same on-disk dataset:
//!
//!   1. **No cache** — `JsonStorage::with_cache(false)`. Every read hits disk.
//!   2. **Cold cache** — `cache_enabled = true`, no `cache_index.json` present.
//!      First access pays the build cost; subsequent reads are O(1) memory hits.
//!   3. **Warm cache** — `cache_enabled = true`, `cache_index.json` already
//!      flushed. `initialize()` deserialises the snapshot in one shot.
//!
//! Override the fixture size with `BENCH_PODCASTS` and `BENCH_EPISODES` env vars.
//!
//! Run with:
//!
//! ```bash
//! cargo run --release --example bench_storage_load
//! BENCH_PODCASTS=10 BENCH_EPISODES=50 cargo run --release --example bench_storage_load
//! ```

use std::path::Path;
use std::time::{Duration, Instant};

use chrono::Utc;
use podcast_tui::podcast::models::{Episode, Podcast};
use podcast_tui::storage::{json::JsonStorage, traits::Storage};
use tempfile::TempDir;

const DEFAULT_PODCASTS: usize = 30;
const DEFAULT_EPISODES: usize = 200;

#[derive(Debug, Clone, Copy)]
struct Sample {
    initialize: Duration,
    first_traversal: Duration,
    subsequent_traversal: Duration,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let n_podcasts = env_usize("BENCH_PODCASTS", DEFAULT_PODCASTS);
    let n_episodes = env_usize("BENCH_EPISODES", DEFAULT_EPISODES);
    let total_episodes = n_podcasts * n_episodes;

    println!("fixture: {n_podcasts} podcasts × {n_episodes} episodes = {total_episodes} total\n");

    let tmp = TempDir::new()?;
    let data_dir = tmp.path().to_path_buf();

    // Build the fixture once. All three configs share the same on-disk layout.
    let fixture_built = Instant::now();
    seed_fixture(&data_dir, n_podcasts, n_episodes).await?;
    println!(
        "fixture seeded in {} ms\n",
        fixture_built.elapsed().as_millis()
    );

    // ── 1. No cache ─────────────────────────────────────────────────────
    let no_cache = measure(&data_dir, false, n_podcasts).await?;

    // ── 2. Cold cache build ─────────────────────────────────────────────
    // Ensure no stale cache index from a previous run is on disk. seed_fixture
    // does not write one, but this guards future refactors.
    let cache_index = data_dir.join("cache_index.json");
    if cache_index.exists() {
        std::fs::remove_file(&cache_index)?;
    }
    let cold_cache = measure(&data_dir, true, n_podcasts).await?;

    // The cold-cache run flushed cache_index.json on shutdown. Re-using it
    // gives us the warm measurement.
    assert!(
        cache_index.exists(),
        "expected cache_index.json after cold-cache run; cache flush may be broken"
    );

    // ── 3. Warm cache (persistent index already on disk) ────────────────
    let warm_cache = measure(&data_dir, true, n_podcasts).await?;

    print_table(&no_cache, &cold_cache, &warm_cache);
    print_speedup(&no_cache, &warm_cache);

    Ok(())
}

/// Run a single measurement against an existing on-disk dataset.
///
/// Each measurement uses a fresh `JsonStorage` instance so the in-memory
/// cache state is the one we asked for, not whatever the previous run left
/// behind.
async fn measure(
    data_dir: &Path,
    cache_enabled: bool,
    n_podcasts: usize,
) -> anyhow::Result<Sample> {
    let storage = JsonStorage::with_data_dir(data_dir.to_path_buf()).with_cache(cache_enabled);

    let t0 = Instant::now();
    storage.initialize().await?;
    let initialize = t0.elapsed();

    let t1 = Instant::now();
    traverse(&storage).await?;
    let first_traversal = t1.elapsed();

    let t2 = Instant::now();
    traverse(&storage).await?;
    let subsequent_traversal = t2.elapsed();

    // Force the cache to write its persistent index before we drop the
    // instance, otherwise the warm-cache run would race the background
    // flush task that has not yet ticked.
    if cache_enabled {
        storage.flush_cache_blocking().await?;
    }

    // Sanity check: the traversal saw what we seeded. A silent zero-result
    // bug would invalidate every number in the table.
    let listed = storage.list_podcasts().await?;
    assert_eq!(
        listed.len(),
        n_podcasts,
        "list_podcasts returned {} entries, expected {}",
        listed.len(),
        n_podcasts
    );

    Ok(Sample {
        initialize,
        first_traversal,
        subsequent_traversal,
    })
}

/// One full WhatsNew-style sweep: list every podcast, then load every
/// podcast's episode collection.
async fn traverse(storage: &JsonStorage) -> anyhow::Result<usize> {
    let podcast_ids = storage.list_podcasts().await?;
    let mut count = 0usize;
    for id in &podcast_ids {
        let episodes = storage.load_episodes(id).await?;
        count += episodes.len();
    }
    Ok(count)
}

/// Generate `n_podcasts` podcasts × `n_episodes` episodes on disk by going
/// through the cache-disabled path so the seeded layout is byte-identical
/// to a real subscription store.
async fn seed_fixture(data_dir: &Path, n_podcasts: usize, n_episodes: usize) -> anyhow::Result<()> {
    let storage = JsonStorage::with_data_dir(data_dir.to_path_buf()).with_cache(false);
    storage.initialize().await?;

    for p in 0..n_podcasts {
        let mut podcast = Podcast::new(format!("Podcast {p}"), format!("https://example.com/{p}"));
        podcast.description = Some(format!("Synthetic podcast {p} for benchmarking"));
        podcast.author = Some("bench".to_string());

        let now = Utc::now();
        let episodes: Vec<Episode> = (0..n_episodes)
            .map(|e| {
                let mut ep = Episode::new(
                    podcast.id.clone(),
                    format!("Episode {e}"),
                    format!("https://example.com/{p}/{e}.mp3"),
                    now,
                );
                ep.description = Some(format!("Body of episode {e}"));
                ep.episode_number = Some(e as u32);
                ep
            })
            .collect();

        storage.save_podcast(&podcast).await?;
        storage.save_episodes(&podcast.id, &episodes).await?;
    }

    Ok(())
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn print_table(no_cache: &Sample, cold: &Sample, warm: &Sample) {
    println!("results (lower is better):\n");
    println!(
        "{:<22} | {:>12} | {:>16} | {:>22}",
        "config", "initialize", "first traversal", "subsequent traversal"
    );
    println!("{}", "-".repeat(82));
    print_row("no cache", no_cache);
    print_row("cache (cold build)", cold);
    print_row("cache (warm)", warm);
    println!();
}

fn print_row(label: &str, s: &Sample) {
    println!(
        "{:<22} | {:>9} ms | {:>13} ms | {:>19} ms",
        label,
        s.initialize.as_millis(),
        s.first_traversal.as_millis(),
        s.subsequent_traversal.as_millis(),
    );
}

fn print_speedup(no_cache: &Sample, warm: &Sample) {
    let baseline_ns = no_cache.first_traversal.as_nanos().max(1);
    let warm_ns = warm.subsequent_traversal.as_nanos().max(1);
    let speedup = baseline_ns as f64 / warm_ns as f64;
    println!(
        "warm-cache speedup vs cold no-cache traversal: {speedup:.1}× \
         (target ≥10×)"
    );
}

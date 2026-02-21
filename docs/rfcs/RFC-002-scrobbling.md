# RFC-002: ListenBrainz-Compatible Scrobbling

**Status**: Draft
**Date**: 2026-02-21
**Author**: @lqdev

## Summary

Add podcast listen scrobbling to podcast-tui via a self-hosted, ListenBrainz-compatible server (`podcast-scrobbler`) and a resilient client module in the TUI. The server is a separate C# .NET 10 project; the TUI integration is an additive `src/scrobbling/` module that is non-breaking and can be merged before playback is implemented.

## Motivation

Podcast apps generally lack listen history tracking comparable to what music listeners get from Last.fm/ListenBrainz. podcast-tui already tracks `play_count` and `last_played_position` on `Episode`, but:
- There is no timestamp of *when* listens occurred
- There is no history log (only the last position, not a log of all plays)
- There is no way to sync listen history across devices
- There is no aggregation (top podcasts, weekly stats, etc.)

By targeting the ListenBrainz API spec, we get protocol compatibility with existing tools (Jellyfin, Navidrome, GPodder-compatible apps) and the ability to self-host with full data ownership.

## Detailed Design

### Part 1: Scrobble Server (separate repo: `lqdev/podcast-scrobbler`)

**Tech Stack:**
- C# .NET 10, ASP.NET Core minimal Web API
- DuckDB (via DuckDB.NET) — embedded analytical database, zero external dependencies
- Optional Bearer token auth via `SCROBBLER_TOKEN` env var
- Docker image deployable to Azure Container Apps, VPS, or local

**Core Endpoints (ListenBrainz-compatible):**

| Method | Route | Auth | Description |
|--------|-------|------|-------------|
| `POST` | `/1/submit-listens` | Yes* | Submit listens (`playing_now`, `single`, or `import`) |
| `GET` | `/1/user/{username}/listens` | No | Paginated listen history (`?count=25&max_ts=...&min_ts=...`) |
| `GET` | `/1/user/{username}/listen-count` | No | Total listen count for user |
| `GET` | `/1/user/{username}/playing-now` | No | Current in-progress episode (ephemeral) |
| `GET` | `/1/validate-token` | Yes | Returns `{ code, message, valid, user_name }` per LB spec |

*Auth only required when `SCROBBLER_TOKEN` env var is set.

**Infrastructure Endpoints:**

| Method | Route | Description |
|--------|-------|-------------|
| `GET` | `/health` | Liveness check (always 200) |
| `GET` | `/ready` | Readiness check (200 if DuckDB accessible) |

**Aggregation Endpoints (podcast-aware extensions):**

| Method | Route | Description |
|--------|-------|-------------|
| `GET` | `/1/user/{username}/stats/podcasts` | Top podcasts by listen count + total time |
| `GET` | `/1/user/{username}/stats/weekly` | Listens and total listen time per week |
| `GET` | `/1/user/{username}/stats/recent-podcasts` | Recently active podcasts (deduplicated) |
| `GET` | `/1/user/{username}/stats/summary` | Lifetime stats: total listens, total time, unique podcasts |

**Database Schema (DuckDB):**

```sql
-- Core listen history (append-only, only 'single' and 'import' types)
-- playing_now events are NEVER stored here (ephemeral per LB spec)
CREATE TABLE listens (
    id            UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    username      VARCHAR NOT NULL,
    listened_at   BIGINT NOT NULL,       -- unix timestamp (seconds)
    artist_name   VARCHAR NOT NULL,      -- podcast name
    track_name    VARCHAR NOT NULL,      -- episode title
    additional_info JSON,               -- podcast GUID, feed URL, duration_ms, etc.
    created_at    TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_listens_user_time ON listens (username, listened_at DESC);

-- Persisted playing_now (for durability across server restarts)
CREATE TABLE playing_now (
    username      VARCHAR PRIMARY KEY,
    updated_at    TIMESTAMPTZ DEFAULT now(),
    artist_name   VARCHAR NOT NULL,
    track_name    VARCHAR NOT NULL,
    additional_info JSON
);
```

> **Design Decision #1**: `playing_now` is ephemeral per the ListenBrainz spec — it is NOT stored in the `listens` table. Only `single` and `import` listen types create permanent history records.

> **Design Decision #2**: `PlayingNowStore` uses a hybrid in-memory + persisted pattern: `ConcurrentDictionary<string, PlayingNow>` for fast reads, with UPSERT to DuckDB on every update for restart durability.

> **Design Decision #3**: DuckDB uses single-writer concurrency. The server registers `DuckDbContext` as a singleton in DI to avoid write contention.

**`additional_info` Podcast Fields:**
```json
{
  "media_player": "podcast-tui",
  "podcast_feed_url": "https://...",
  "episode_guid": "abc-123",
  "duration_ms": 3600000,
  "position_ms": 2700000,
  "percent_complete": 75.0
}
```

**Auth Design:**
- `SCROBBLER_TOKEN` env var: empty/unset = auth disabled (trusted network mode)
- Header format: `Authorization: Token <token>` (ListenBrainz convention, NOT `Bearer`)
- When set: required on write endpoints, optional on read endpoints (configurable via `SCROBBLER_REQUIRE_AUTH_FOR_READS`)

**Deployment:**
- Dockerfile with .NET 10 runtime base image
- DuckDB file volume-mounted at `/data/scrobbles.db`
- Port via `PORT` env var (default 5000)
- `docker-compose.yml` for VPS, `containerapp.yaml` for Azure Container Apps

### Part 2: podcast-tui Client Integration

#### Config Changes (`src/config.rs`)

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScrobblingConfig {
    pub enabled: bool,
    pub endpoint: Option<String>,       // e.g., Some("http://my-server:5000")
    pub token: Option<String>,          // None = no auth (matches server's optional auth)
    pub username: String,
    pub min_listen_percent: u8,         // default: 25 (podcast-appropriate)
    pub min_listen_seconds: u32,        // default: 300 (5 min floor)
    pub submit_playing_now: bool,       // default: true
    pub timeout_secs: u64,             // default: 5 (must never block playback)
    pub max_retry_queue_size: usize,   // default: 500
    pub retry_queue_ttl_days: u32,     // default: 30
}
```

> **Design Decision #4**: Dual scrobble threshold — `min_listen_percent: 25` AND `min_listen_seconds: 300`, both must be met. The ListenBrainz music rule ("half the track or 4 minutes, whichever is lower") doesn't work for podcasts: 4 minutes of a 90-minute episode is only 4.4%. The dual threshold prevents premature scrobbling of long episodes.

> **Design Decision #5**: `endpoint` and `token` use `Option<String>` (not `String`) to match existing podcast-tui patterns (`sync_device_path: Option<String>`, `external_player: Option<String>`). `None` = not configured / no auth.

#### Constants (`src/constants.rs`)

```rust
pub mod scrobbling {
    pub const DEFAULT_MIN_LISTEN_PERCENT: u8 = 25;
    pub const DEFAULT_MIN_LISTEN_SECONDS: u32 = 300;  // 5 minutes
    pub const SCROBBLE_TIMEOUT_SECS: u64 = 5;
    pub const MAX_RETRY_QUEUE_SIZE: usize = 500;
    pub const RETRY_QUEUE_TTL_DAYS: u32 = 30;
    pub const CIRCUIT_BREAKER_FAILURE_THRESHOLD: u32 = 5;
    pub const CIRCUIT_BREAKER_RESET_SECS: u64 = 60;
    pub const DRAIN_INTERVAL_BASE_SECS: u64 = 30;
    pub const DRAIN_INTERVAL_MAX_SECS: u64 = 300;
}
```

#### New Module: `src/scrobbling/`

```
src/scrobbling/
├── mod.rs              # Scrobbler trait, ScrobbleEvent, ScrobblerError, re-exports
├── client.rs           # ListenBrainzClient (reqwest, short timeout)
├── models.rs           # SubmitListensRequest, TrackMetadata, AdditionalInfo (serde)
├── noop.rs             # NoopScrobbler (when disabled or misconfigured)
├── queue.rs            # PersistentRetryQueue — pending_scrobbles.json in data_dir
└── circuit_breaker.rs  # CircuitBreaker — open/closed/half-open states
```

**Scrobbler Trait:**
```rust
#[async_trait]
pub trait Scrobbler: Send + Sync {
    async fn playing_now(&self, event: &ScrobbleEvent) -> Result<(), ScrobblerError>;
    async fn scrobble(&self, event: &ScrobbleEvent) -> Result<(), ScrobblerError>;
    async fn flush_pending(&self) -> Result<usize, ScrobblerError>;
    fn pending_count(&self) -> usize;
    fn circuit_state(&self) -> CircuitState;
}
```

**ScrobbleEvent (maps from existing Episode model):**
```rust
pub struct ScrobbleEvent {
    pub podcast_title: String,        // from Podcast.title
    pub episode_title: String,        // from Episode.title
    pub feed_url: Option<String>,     // from Podcast.feed_url
    pub episode_guid: Option<String>, // from Episode.guid
    pub duration_ms: Option<u64>,     // from Episode.duration (seconds → ms)
    pub position_ms: u64,             // from Episode.last_played_position (seconds → ms)
    pub listened_at: i64,             // unix timestamp (seconds)
}
```

**Resiliency Patterns (the scrobbler must NEVER block or affect playback):**

1. **Fire-and-forget dispatch**: via `tokio::spawn` (matching existing `trigger_async_download`, `trigger_async_refresh` patterns)
2. **Persistent retry queue**: JSON array in `pending_scrobbles.json`, atomic write (`.tmp` then rename), FIFO eviction at 500 cap, 30-day TTL
3. **Circuit breaker**: open after 5 consecutive failures, half-open after 60s, re-opens on half-open failure
4. **Background drain task**: `tokio::spawn`ed at startup, exponential backoff (30s → 300s max), respects circuit breaker
5. **Short timeout**: 5s on all HTTP calls (configurable)

> **Design Decision #6**: Retry queue uses standard JSON array format (not JSONL) for consistency with existing data files (`sync_targets.json`, `sync_history.json`). The queue is small (max 500 events) so read-parse-modify-write is acceptable.

#### Playback Integration (future, blocked on audio implementation)

```rust
// On play start
tokio::spawn(scrobbler.playing_now(event));

// When position >= min_listen_seconds AND percent >= min_listen_percent
tokio::spawn(scrobbler.scrobble(event));

// On app shutdown
scrobbler.flush_pending().await;
```

### Implementation Steps

1. Add `ScrobblingConfig` struct to `src/config.rs` + constants to `src/constants.rs`
2. Add `src/scrobbling/` module (trait, client, models, noop, queue, circuit_breaker)
3. Wire scrobbler into playback lifecycle (blocked on audio playback implementation)

## Alternatives Considered

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| Custom REST API (non-LB-compatible) | Full control over schema | No ecosystem compatibility; must build clients from scratch | Rejected — LB spec is simple enough and gives free compatibility |
| ListenBrainz public service | Zero server maintenance | Data on third-party servers; no podcast-specific extensions; no self-hosting | Rejected — user wants self-hosted with full control |
| Local-only SQLite (no server) | Dead simple, no network | No cross-device sync; no web-accessible stats | Rejected — server enables multi-device + future dashboard |
| Last.fm/AudioScrobbler protocol | Wide adoption | MD5-signed requests, XML-based, closed source, can't self-host | Rejected — ListenBrainz is the modern open replacement |
| Rust for server | Consistent ecosystem | User preference for C# | Rejected — user chose C# |

## Design Decisions

> **Design Decision #1**: `playing_now` is ephemeral — NOT stored in permanent listen history (per LB spec).

> **Design Decision #2**: Hybrid in-memory + DuckDB persistence for `playing_now` (fast reads + restart durability).

> **Design Decision #3**: DuckDB singleton writer connection to avoid single-writer contention.

> **Design Decision #4**: Dual scrobble threshold (percent AND seconds) for podcast-appropriate scrobbling.

> **Design Decision #5**: `Option<String>` for optional config fields (endpoint, token) per codebase conventions.

> **Design Decision #6**: JSON array format for retry queue file (consistency with existing data files).

## Open Questions

- [x] Should `playing_now` be ephemeral or persisted? (Resolved: hybrid — in-memory for speed, DuckDB for durability)
- [x] What scrobble threshold for podcasts? (Resolved: dual threshold — 25% AND 5 minutes, both must be met)
- [ ] Should the server expose a deletion endpoint (`POST /1/delete-listen` per LB spec)?
- [ ] Should the TUI show a scrobble status indicator in the status bar?
- [ ] Should the `:scrobble-import` command synthesize timestamps for historical `play_count` data?

## References

- [ListenBrainz API Documentation](https://listenbrainz.readthedocs.io/en/latest/users/api/core.html)
- [ListenBrainz JSON Format](https://listenbrainz.readthedocs.io/en/latest/users/json.html)
- podcast-tui Episode model: `src/podcast/models.rs`
- podcast-tui Config: `src/config.rs`
- podcast-tui Constants: `src/constants.rs`

---

*Last Updated: February 2026 | Version: v1.6.0 | Maintainer: [@lqdev](https://github.com/lqdev)*

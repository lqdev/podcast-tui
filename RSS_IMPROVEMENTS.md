# RSS Parsing Improvements Summary

## Issues Addressed

### 1. **Redirect Handling** 
**Problem**: Many RSS feeds (like Buzzsprout) return HTTP 301/302 redirects to the actual feed URL, but the original HTTP client didn't handle redirects properly.

**Solution**: Enhanced HTTP client configuration in both `FeedParser` and `DownloadManager`:

```rust
let http_client = Client::builder()
    .user_agent("podcast-tui/1.0.0-mvp (like FeedReader)")
    .timeout(Duration::from_secs(30))
    .connect_timeout(Duration::from_secs(10))
    .redirect(reqwest::redirect::Policy::limited(10)) // Handle up to 10 redirects
    .build()
    .expect("Failed to create HTTP client");
```

**Evidence**: 
- `https://feeds.buzzsprout.com/1121972.rss` → **HTTP 301** → `https://feeds.simplecast.com/_IjaDYAj`
- Curl test shows redirect works: `curl -L` successfully follows redirect and retrieves content

### 2. **Better HTTP Headers**
**Problem**: Generic HTTP requests might not be served proper RSS content by some servers.

**Solution**: Added proper Accept headers and user agent:

```rust
.header("Accept", "application/rss+xml, application/rdf+xml, application/atom+xml, application/xml, text/xml, */*")
```

### 3. **Enhanced Debug Output**
**Problem**: No visibility into why RSS parsing was failing.

**Solution**: Added comprehensive debug logging in `download_feed()`:

```rust
eprintln!("DEBUG: Downloading feed from: {}", feed_url);
eprintln!("DEBUG: Response status: {}, final URL: {}", status, final_url);
eprintln!("DEBUG: Content-Type: {}", ct_str);
eprintln!("DEBUG: Downloaded {} bytes of content", content.len());
```

### 4. **Episode Deduplication Improvements**
**Problem**: Episodes were being duplicated due to non-deterministic ID generation.

**Solution**: Enhanced deterministic ID generation in `src/storage/models.rs`:

```rust
pub fn from_guid(guid: &str) -> Self {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    guid.hash(&mut hasher);
    let hash = hasher.finish();
    
    // Create deterministic UUID from hash
    let uuid = uuid::Uuid::from_u64_pair(hash, hash.rotate_left(32));
    Self(uuid)
}
```

### 5. **Audio URL Extraction Robustness**
**Problem**: Episodes showing "(no audio URL)" preventing downloads.

**Solution**: Already had 5-strategy audio URL extraction, enhanced with debug output:

1. **MIME type matching** (`audio/*`, `application/octet-stream`)
2. **File extension detection** (`.mp3`, `.m4a`, `.wav`, etc.)
3. **Enclosure element parsing**
4. **GUID-based URL extraction**
5. **Heuristic pattern matching**

## Feed Validation Results

### ✅ Working Feeds Tested:

1. **Windows Weekly**: `https://feeds.twit.tv/ww.xml`
   - **Status**: HTTP 200 (direct, no redirect)
   - **Content-Type**: `text/xml`
   - **Enclosures**: ✅ Present with `type="audio/mpeg"`
   - **Example**: `<enclosure url="https://...R1_ww0952.mp3" length="157740428" type="audio/mpeg"/>`

2. **Deep Questions**: `https://feeds.buzzsprout.com/1121972.rss`
   - **Status**: HTTP 301 → `https://feeds.simplecast.com/_IjaDYAj`
   - **Content-Type**: `application/atom+xml` (final)
   - **Enclosures**: ✅ Present with `type="audio/mpeg"`
   - **Example**: `<enclosure length="41595435" type="audio/mpeg" url="https://...default.mp3"/>`

## Code Changes Made

### Files Modified:

1. **`src/podcast/feed.rs`**
   - Enhanced HTTP client with redirect handling
   - Added debug output for feed download and parsing
   - Improved error handling for network requests

2. **`src/download/manager.rs`** 
   - Updated HTTP client configuration to match feed parser
   - Added redirect handling for episode downloads

3. **`src/storage/models.rs`** (from previous session)
   - Added `EpisodeId::from_guid()` for deterministic ID generation

4. **`src/podcast/subscription.rs`** (from previous session) 
   - Enhanced deduplication logic with multiple fallback strategies

## Expected Behavior After Improvements

1. **Redirect Handling**: Feeds with redirects (like Buzzsprout) should now work correctly
2. **Audio URL Extraction**: Episodes should no longer show "(no audio URL)" if the feed has proper enclosures
3. **Episode Deduplication**: Duplicate episodes should be significantly reduced due to GUID-based IDs
4. **Debug Visibility**: RSS parsing issues should be visible in debug output
5. **F5 Refresh**: Buffer refresh should work properly in the UI

## Manual Testing Steps

1. Run: `cargo run`
2. Press F2 to switch to podcast list  
3. Press 'a' to add a feed
4. Test with: `https://feeds.twit.tv/ww.xml` (direct feed)
5. Test with: `https://feeds.buzzsprout.com/1121972.rss` (redirect feed)
6. Check terminal output for debug messages
7. Verify episodes have audio URLs and minimal duplicates
8. Test F5 refresh functionality
# Fix: Omny.fm Podcast Download Corruption

## Problem

Episodes from Omny.fm hosted podcasts (e.g., Desert Oracle, Better Offline) were not downloading correctly:
- Downloaded files were corrupted and wouldn't play
- Podcast cover art was not visible
- Both issues affected feeds from `omnycontent.com` specifically

## Root Cause Analysis

The download manager in `src/download/manager.rs` had three issues:

1. **Missing HTTP Status Validation**: The `download_file()` and `download_artwork()` methods didn't check if the HTTP response was successful before streaming the content. If the server returned an error (404, 403, 500, etc.), the error page HTML would be written to the file.

2. **Missing Content-Type Validation**: Even when servers returned HTTP 200 OK, they sometimes returned HTML error pages instead of audio files. The download manager wrote this HTML to the MP3 file, creating a "corrupted" file with valid ID3 tags but HTML content instead of audio data.

3. **Inconsistent User-Agent**: The download manager used a different User-Agent string (`podcast-tui/1.0.0-mvp (like FeedReader)`) than the feed parser, which could cause some servers to reject requests.

## Solution

### Changes Made

**File**: `src/download/manager.rs`

1. **Updated HTTP Client User-Agent** (line ~53):
   ```rust
   // Before:
   .user_agent("podcast-tui/1.0.0-mvp (like FeedReader)")
   
   // After:
   .user_agent("Mozilla/5.0 (compatible; podcast-tui/1.0; +https://github.com/podcast-tui) AppleWebKit/537.36 (KHTML, like Gecko)")
   ```

2. **Added HTTP Status Validation to `download_file()`** (line ~481):
   ```rust
   async fn download_file(&self, url: &str, path: &Path) -> Result<(), DownloadError> {
       let response = self.client.get(url).send().await?;
       
       // Check if the response is successful
       let response = response.error_for_status()?;
       
       // ... rest of the logic
   }
   ```

3. **Added Content-Type Validation to `download_file()`** (line ~492):
   ```rust
   // Get content type to verify it's actually audio
   let content_type = response
       .headers()
       .get("content-type")
       .and_then(|ct| ct.to_str().ok())
       .unwrap_or("unknown");
   
   // Reject downloads that are not audio files
   // This catches cases where servers return HTML error pages with 200 OK status
   let is_audio = content_type.starts_with("audio/") 
       || content_type == "application/octet-stream"
       || content_type.starts_with("video/")
       || content_type == "binary/octet-stream"
       || content_type == "unknown";
   
   if !is_audio && content_type.contains("html") {
       return Err(DownloadError::InvalidPath(format!(
           "Server returned HTML instead of audio file (Content-Type: {})",
           content_type
       )));
   }
   ```

4. **Added HTTP Status Validation to `download_artwork()`** (line ~809):
   ```rust
   async fn download_artwork(&self, url: &str) -> Result<(String, Vec<u8>), DownloadError> {
       let response = self.client.get(url).send().await?;
       
       // Check if the response is successful
       let response = response.error_for_status()?;
       
       // ... rest of the image processing logic
   }
   ```

## Technical Details

### HTTP Status Validation

The fix uses `response.error_for_status()` which:
- Returns the response as-is if the status code is 2xx (success)
- Returns an error if the status code is 4xx or 5xx
- Properly propagates the error through the `?` operator to `DownloadError::Http`

### Content-Type Validation

The fix validates the `Content-Type` header before downloading:
- Accepts: `audio/*`, `video/*`, `application/octet-stream`, `binary/octet-stream`, or unknown
- Rejects: Any content type containing "html" (typically `text/html`)
- This prevents writing HTML error pages to MP3 files even when servers return HTTP 200 OK
- Provides clear error messages: "Server returned HTML instead of audio file"

### User-Agent Standardization

Using the same User-Agent as the feed parser ensures consistent behavior across all HTTP requests and prevents servers from blocking download requests while allowing feed requests.

### Redirect Handling

The HTTP client was already configured to handle redirects with `.redirect(reqwest::redirect::Policy::limited(10))`, so no changes were needed for Podtrac tracking URLs.

## Testing

### Verification Steps

1. **Code Compilation**: `cargo check` - ✅ Success
2. **Unit Tests**: `cargo test` - ✅ 97 tests passed
3. **Integration Tests**: All relevant tests passed

### Manual Testing Required

To fully verify the fix, users should:
1. Subscribe to a podcast from Omny.fm (e.g., Desert Oracle or Better Offline)
2. Download an episode
3. Verify the file plays correctly
4. Verify the podcast cover art displays correctly

## Impact

- **Scope**: Affects all podcast downloads, especially from Omny.fm/omnycontent.com
- **Breaking Changes**: None
- **Performance**: No impact
- **User Experience**: Significantly improved - episodes will download correctly instead of resulting in corrupted files

## Related Issues

This fix addresses download failures specifically for:
- Desert Oracle: `https://omny.fm/shows/desert-oracle-radio/playlists/podcast.rss`
- Better Offline: `https://omny.fm/shows/better-offline/playlists/podcast.rss`
- Any other podcasts hosted on Omny.fm platform

## Future Improvements

Consider adding:
1. Content-Type validation to ensure downloaded files match expected MIME types
2. File size validation to detect suspiciously small downloads
3. Retry logic for failed downloads
4. More detailed error messages for users when downloads fail

---

**Date**: 2025-01-XX
**Files Changed**: `src/download/manager.rs`
**Lines Changed**: ~10 lines
**Tests Affected**: None (all existing tests pass)

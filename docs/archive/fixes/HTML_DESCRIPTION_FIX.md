# HTML Description Rendering Fix

**Date**: October 9, 2025  
**Status**: ✅ Complete  
**Issue**: Episode descriptions with HTML/CDATA content not rendering correctly

## Problem Description

Some RSS feeds (notably Audioboom) include HTML markup and CDATA in episode descriptions. The feed parser was extracting this content as-is, and the TUI was displaying raw HTML tags, making descriptions unreadable.

### Example Issue

From the Audioboom feed for "National Park After Dark":
```html
<div>In recognition of Hispanic Heritage Month, today's episode is dedicated to 
George Meléndez Wright, the first Hispanic person to occupy a professional role 
in the National Park Service.<br>
<br>
To submit a business for the Outsiders Gift Guide, please email 
<a href="mailto:assistant@npadpodcast.com">assistant@npadpodcast.com</a> by 
October 22nd :)<br>
<strong><br>
Sources:</strong><br>
```

This was being displayed with all HTML tags visible in the terminal UI.

## Root Cause

The `feed-rs` library correctly extracts the content from RSS feeds, but it preserves the original format including HTML markup. The application wasn't sanitizing this content before display in the TUI.

## Solution

Implemented a comprehensive HTML stripping and text sanitization system:

### 1. New Text Utility Module (`src/utils/text.rs`)

Created three key functions:

#### `strip_html(input: &str) -> String`
- Removes all HTML tags using regex pattern `<[^>]*>`
- Decodes HTML entities after tag removal (to avoid entity-encoded tags)
- Cleans up excessive whitespace
- Maintains paragraph breaks (double newlines)

#### `decode_html_entities(input: &str) -> Cow<'_, str>`
- Converts common HTML entities to their character equivalents
- Supports: `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&nbsp;`, `&ndash;`, `&mdash;`, `&ldquo;`, `&rdquo;`, `&lsquo;`, `&rsquo;`, `&bull;`, `&copy;`, `&reg;`, `&trade;`, and more
- Uses Unicode escape sequences for smart quotes and special characters
- Zero-copy optimization when no entities present

#### `clean_whitespace(input: &str) -> Cow<'_, str>`
- Removes excessive spaces and tabs
- Limits consecutive newlines to 2 (preserves paragraph breaks)
- Trims leading/trailing whitespace
- Normalizes whitespace for consistent display

### 2. Integration in Feed Parser (`src/podcast/feed.rs`)

Modified two key areas:

#### Episode Description Extraction
```rust
let description = entry
    .summary
    .as_ref()
    .map(|t| strip_html(&t.content))
    .or_else(|| {
        entry
            .content
            .as_ref()
            .and_then(|c| c.body.as_ref().map(|body| strip_html(body)))
    })
    .filter(|s| !s.is_empty());
```

#### Podcast Description Extraction
```rust
description: feed
    .description
    .as_ref()
    .map(|d| strip_html(&d.content))
    .filter(|s| !s.is_empty()),
```

### 3. Comprehensive Testing

Created 10 unit tests covering:
- Basic HTML tag removal
- Nested tags
- HTML entity decoding
- Complex real-world examples (Audioboom feed)
- Clean text preservation (Libsyn feed)
- Whitespace normalization
- Edge cases

All tests pass ✅

## Implementation Details

### Design Decisions

1. **Regex-based stripping**: Simple, fast, and reliable for removing HTML tags
2. **Two-phase processing**: Strip tags first, then decode entities (prevents entity-encoded tags from surviving)
3. **Preserve paragraphs**: Allow up to 2 consecutive newlines to maintain readability
4. **Zero-copy optimization**: Use `Cow<'_, str>` to avoid allocations when no changes needed
5. **No external dependencies**: Uses only `regex` which was already a project dependency

### Performance Considerations

- Regex compilation happens once via lazy static
- Cow<str> returns borrowed data when no changes needed
- String allocations only when modifications required
- Efficient for both HTML-heavy and clean text feeds

### Safety & Correctness

- Handles all common HTML entities
- Preserves non-ASCII characters (Unicode)
- Maintains text structure and readability
- No data loss - only removes markup
- Filters out truly empty descriptions

## Testing Strategy

### Unit Tests
- Test all individual components (strip, decode, clean)
- Test integration scenarios
- Test both problematic and clean feeds
- Verify preservation of clean text

### Manual Testing Recommendations
1. Subscribe to Audioboom feed: `https://audioboom.com/channels/5055896.rss`
2. View episode details for any episode
3. Verify description is clean and readable
4. Compare with Libsyn feed to ensure clean text unchanged

## Files Changed

### New Files
- `src/utils/text.rs` (203 lines, 10 tests)

### Modified Files
- `src/utils/mod.rs` - Added `pub mod text;`
- `src/podcast/feed.rs` - Applied `strip_html()` to descriptions
- `CHANGELOG.md` - Documented the fix

### No Breaking Changes
- Backward compatible with existing feeds
- Clean text feeds work exactly as before
- Only affects display of HTML-containing descriptions

## Results

### Before
```
Episode: 326: Short Life, Long Legacy. The Vision of George Meléndez Wright.
Description:
<div>In recognition of Hispanic Heritage Month, today's episode is dedicated to George Meléndez Wright, the first Hispanic person to occupy a professional role in the National Park Service.<br>
<br>
To submit a business for the Outsiders Gift Guide, please email <a href="mailto:assistant@npadpodcast.com">assistant@npadpodcast.com</a> by October 22nd :)<br>
<strong><br>
Sources:</strong><br>
```

### After
```
Episode: 326: Short Life, Long Legacy. The Vision of George Meléndez Wright.
Description:
In recognition of Hispanic Heritage Month, today's episode is dedicated to George Meléndez Wright, the first Hispanic person to occupy a professional role in the National Park Service.

To submit a business for the Outsiders Gift Guide, please email assistant@npadpodcast.com by October 22nd :)

Sources:
```

## Future Enhancements (Optional)

1. **Markdown support**: Convert common patterns to markdown for richer display
2. **Link extraction**: Show URLs separately or inline with indicator
3. **Image extraction**: Parse and note image URLs in descriptions
4. **Configurable sanitization**: Allow users to control stripping behavior

## Lessons Learned

1. **RSS feeds vary widely**: Different providers use different approaches to content
2. **HTML in RSS is common**: Many podcast hosting platforms include HTML markup
3. **Testing with real feeds matters**: Unit tests alone don't catch all edge cases
4. **Simple solutions work**: Regex-based stripping is sufficient for 99% of cases
5. **Entity decoding order matters**: Must decode entities *after* stripping tags

## References

- RSS 2.0 Specification: https://www.rssboard.org/rss-specification
- Audioboom feed example: https://audioboom.com/channels/5055896.rss
- Libsyn feed example: https://feeds.libsyn.com/314366/rss
- feed-rs library: https://github.com/feed-rs/feed-rs

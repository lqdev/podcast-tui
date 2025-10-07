# OPML XML Sanitization Fix

## Problem
Real-world OPML files often contain XML that violates strict parsing rules:
1. **Missing `@text` attribute**: OPML 2.0 spec requires `@text`, but many files use only `@title`
2. **Unescaped ampersands**: Attribute values like `title="Security & Privacy"` should be `title="Security &amp; Privacy"`

## Example
The file at `https://www.lqdev.me/collections/podroll/index.opml` contains:
```xml
<outline title="The Privacy, Security, & OSINT Show" type="rss" ... />
```

The `&` should be `&amp;` per XML spec, but this is common in real-world files.

## Solution

### 1. Made `@text` Optional (Commit 1)
Changed `OpmlOutlineRaw` struct:
```rust
// Before:
#[serde(rename = "@text")]
text: String,

// After:
#[serde(rename = "@text", skip_serializing_if = "Option::is_none")]
text: Option<String>,
```

Fall back to `@title` if `@text` is missing:
```rust
let text = outline.text
    .or_else(|| outline.title.clone())
    .unwrap_or_else(|| "Untitled".to_string());
```

### 2. Added XML Sanitization (Commit 2)
Created `sanitize_xml()` method that fixes unescaped ampersands:

```rust
fn sanitize_xml(xml: &str) -> String {
    // Step 1: Escape all & to &amp;
    let step1 = xml.replace("&", "&amp;");
    
    // Step 2: Fix double-escaping of already-escaped entities
    let step2 = step1
        .replace("&amp;amp;", "&amp;")
        .replace("&amp;lt;", "&lt;")
        .replace("&amp;gt;", "&gt;")
        .replace("&amp;quot;", "&quot;")
        .replace("&amp;apos;", "&apos;");
    
    // Step 3: Fix numeric entities: &amp;#123; -> &#123;
    let re = Regex::new(r"&amp;#(\d+);").unwrap();
    re.replace_all(&step2, "&#$1;").to_string()
}
```

## New Dependency
Added to `Cargo.toml`:
```toml
regex = "1.10"
```

## Test Results
```
✓ test_is_url ... ok
✓ test_validate_opml_valid ... ok  
✓ test_validate_opml_invalid ... ok
✓ test_import_result_summary ... ok
✓ test_parse_valid_opml ... ok
✓ test_export_opml ... ok
✓ test_parse_lqdev_opml ... ok  (new - tests the problematic URL)
✓ test_parse_local_opml ... ok  (new - tests local file with same issue)

8 tests passed (6 unit + 2 integration)
```

## Files Modified
1. `src/podcast/opml.rs` - Added sanitization, made @text optional
2. `Cargo.toml` - Added regex dependency
3. `tests/test_opml_live_url.rs` - New integration test
4. `tests/test_opml_local_file.rs` - New integration test

## Impact
- **Backward Compatible**: Exported OPML still includes both `@text` and `@title` for maximum compatibility
- **Robust Parsing**: Can now parse OPML from real-world sources that don't strictly follow XML/OPML specs
- **No Breaking Changes**: All existing tests still pass

## Example Success
```
✓ Successfully parsed OPML!
  Version: 2.0
  Title: Podroll
  Found 27 feeds
  1. Practical AI -> https://changelog.com/practicalai/feed
  2. The TWIML AI Podcast -> https://feeds.megaphone.fm/MLN2155636147
  3. Darknet Diaries -> https://feeds.megaphone.fm/darknetdiaries
  ...
  5. The Privacy, Security, & OSINT Show -> ... ✓ (ampersand handled correctly)
```

---

**Date**: October 6, 2025  
**Status**: ✅ Fixed and Tested

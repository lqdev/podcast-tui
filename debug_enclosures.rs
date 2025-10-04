use feed_rs::parser;

fn main() {
    let rss_sample = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Podcast</title>
    <item>
      <title>Test Episode</title>
      <enclosure url="https://example.com/episode.mp3" type="audio/mpeg" length="12345" />
      <link>https://example.com/episode-page</link>
    </item>
  </channel>
</rss>"#;

    match parser::parse(rss_sample.as_bytes()) {
        Ok(feed) => {
            for entry in &feed.entries {
                println!("Entry: {}", entry.title.as_ref().map(|t| &t.content).unwrap_or("No title"));
                println!("  Links: {}", entry.links.len());
                for (i, link) in entry.links.iter().enumerate() {
                    println!("    Link {}: href='{}', media_type='{:?}', rel='{:?}'", 
                             i, link.href, link.media_type, link.rel);
                }
                
                // Check if feed-rs exposes enclosures
                println!("  Extensions: {:?}", entry.extensions.keys().collect::<Vec<_>>());
            }
        }
        Err(e) => println!("Parse error: {}", e),
    }
}
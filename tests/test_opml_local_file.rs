#[cfg(test)]
mod test_opml_local_file {
    use podcast_tui::podcast::OpmlParser;
    use std::path::Path;

    #[tokio::test]
    async fn test_parse_local_opml() {
        // Use the sample fixture checked into the repo. Build the path via
        // `Path::join` so we get the right separator on every platform
        // (the nix sandbox is Linux, but the test should also be correct on
        // Windows / macOS dev machines).
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("sample-subscriptions.opml");
        let path_str = path.to_string_lossy();

        let parser = OpmlParser::new();

        let result = parser.parse(&path_str).await;

        match &result {
            Ok(document) => {
                println!("✓ Successfully parsed local OPML!");
                println!("  Version: {}", document.version);
                if let Some(head) = &document.head {
                    if let Some(title) = &head.title {
                        println!("  Title: {}", title);
                    }
                }
                println!("  Found {} feeds", document.outlines.len());
                for (i, outline) in document.outlines.iter().take(3).enumerate() {
                    println!(
                        "  {}. {} -> {}",
                        i + 1,
                        outline.text,
                        outline.feed_url().unwrap_or("no URL")
                    );
                }
            }
            Err(e) => {
                eprintln!("✗ Failed to parse OPML: {}", e);
            }
        }

        let doc = result.expect("Should successfully parse local OPML file");
        assert_eq!(doc.outlines.len(), 6, "Sample OPML should contain 6 feeds");
        assert_eq!(
            doc.head.as_ref().and_then(|h| h.title.as_deref()),
            Some("Sample Podcast Subscriptions"),
        );
    }
}

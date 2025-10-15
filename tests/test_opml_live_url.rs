#[cfg(test)]
mod test_opml_live_url {
    use podcast_tui::podcast::OpmlParser;

    #[tokio::test]
    async fn test_parse_lqdev_opml() {
        let url = "https://www.lqdev.me/collections/podroll/index.opml";

        let parser = OpmlParser::new();

        let result = parser.parse(url).await;

        match &result {
            Ok(document) => {
                println!("✓ Successfully parsed OPML!");
                println!("  Version: {}", document.version);
                if let Some(head) = &document.head {
                    if let Some(title) = &head.title {
                        println!("  Title: {}", title);
                    }
                }
                println!("  Found {} feeds", document.outlines.len());
            }
            Err(e) => {
                eprintln!("✗ Failed to parse OPML: {}", e);
            }
        }

        assert!(result.is_ok(), "Should successfully parse OPML from URL");
    }
}

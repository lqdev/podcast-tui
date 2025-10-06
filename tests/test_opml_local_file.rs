#[cfg(test)]
mod test_opml_local_file {
    use podcast_tui::podcast::OpmlParser;

    #[tokio::test]
    async fn test_parse_local_opml() {
        let path = "test.opml";
        
        let parser = OpmlParser::new();
        
        let result = parser.parse(path).await;
        
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
                    println!("  {}. {} -> {}", i+1, outline.text, outline.feed_url().unwrap_or("no URL"));
                }
            }
            Err(e) => {
                eprintln!("✗ Failed to parse OPML: {}", e);
            }
        }
        
        assert!(result.is_ok(), "Should successfully parse local OPML file");
    }
}

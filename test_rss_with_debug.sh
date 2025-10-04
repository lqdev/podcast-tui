#!/bin/bash

# Test RSS parsing with debug output
echo "Testing RSS parsing with improved redirect handling..."

# Create a simple Rust test program to test our RSS parser
cat > /tmp/test_rss.rs << 'EOF'
use std::time::Duration;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test with a known working feed
    let test_urls = vec![
        "https://feeds.twit.tv/ww.xml",
        "https://feeds.buzzsprout.com/1121972.rss",
        "https://rss.cnn.com/rss/edition.rss", // CNN RSS (should work)
    ];
    
    let client = Client::builder()
        .user_agent("podcast-tui/1.0.0-mvp (like FeedReader)")
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    for url in test_urls {
        println!("\n=== Testing: {} ===", url);
        
        match client.get(url)
            .header("Accept", "application/rss+xml, application/rdf+xml, application/atom+xml, application/xml, text/xml, */*")
            .send()
            .await 
        {
            Ok(response) => {
                let status = response.status();
                let final_url = response.url().clone();
                println!("Status: {}, Final URL: {}", status, final_url);
                
                if let Some(content_type) = response.headers().get("content-type") {
                    if let Ok(ct_str) = content_type.to_str() {
                        println!("Content-Type: {}", ct_str);
                    }
                }
                
                if status.is_success() {
                    match response.text().await {
                        Ok(content) => {
                            println!("Content length: {} bytes", content.len());
                            
                            // Check if it looks like XML/RSS
                            if content.trim_start().starts_with("<?xml") || content.contains("<rss") || content.contains("<feed") {
                                println!("✓ Looks like valid XML/RSS content");
                                
                                // Try to parse with feed-rs
                                match feed_rs::parser::parse(content.as_bytes()) {
                                    Ok(feed) => {
                                        println!("✓ Successfully parsed with feed-rs");
                                        println!("  Title: {:?}", feed.title.as_ref().map(|t| &t.content));
                                        println!("  Episodes: {}", feed.entries.len());
                                        
                                        // Check first episode for audio links
                                        if let Some(entry) = feed.entries.first() {
                                            println!("  First episode: {:?}", entry.title.as_ref().map(|t| &t.content));
                                            println!("  Links: {}", entry.links.len());
                                            
                                            for (i, link) in entry.links.iter().enumerate() {
                                                println!("    Link {}: href='{}', media_type='{:?}'", i, link.href, link.media_type);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("✗ Failed to parse with feed-rs: {}", e);
                                    }
                                }
                            } else {
                                println!("✗ Content doesn't look like XML/RSS");
                                let preview: String = content.lines().take(3).collect::<Vec<_>>().join("\n");
                                println!("Preview: {}", preview);
                            }
                        }
                        Err(e) => {
                            println!("✗ Failed to read response body: {}", e);
                        }
                    }
                } else {
                    println!("✗ HTTP error: {}", status);
                }
            }
            Err(e) => {
                println!("✗ Request failed: {}", e);
            }
        }
    }
    
    Ok(())
}
EOF

# Compile and run the test
echo "Compiling test program..."
rustc --edition 2021 -L target/debug/deps /tmp/test_rss.rs -o /tmp/test_rss \
    --extern reqwest=target/debug/deps/libreqwest-*.rlib \
    --extern tokio=target/debug/deps/libtokio-*.rlib \
    --extern feed_rs=target/debug/deps/libfeed_rs-*.rlib

if [ $? -eq 0 ]; then
    echo "Running RSS test..."
    /tmp/test_rss
else
    echo "Compilation failed. Let's try a simpler approach with curl..."
    
    echo "=== Testing with curl ==="
    for url in "https://feeds.twit.tv/ww.xml" "https://feeds.buzzsprout.com/1121972.rss"; do
        echo
        echo "Testing: $url"
        curl -L -s -I "$url" | head -10
        echo "Content preview:"
        curl -L -s "$url" | head -20
        echo
    done
fi
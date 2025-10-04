#!/bin/bash

# Quick test script to debug RSS parsing

echo "=== RSS Parsing Debug Test ==="

# Create a simple test program to parse one feed
cat > /tmp/debug_rss.rs << 'EOF'
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <feed_url>", args[0]);
        return Ok(());
    }
    
    let feed_url = &args[1];
    println!("Parsing feed: {}", feed_url);
    
    // Use reqwest to get the feed
    let client = reqwest::Client::new();
    let content = client.get(feed_url).send().await?.text().await?;
    
    println!("Feed content length: {} bytes", content.len());
    
    // Parse with feed-rs
    let feed = feed_rs::parser::parse(content.as_bytes())?;
    
    println!("Feed title: {}", feed.title.map(|t| t.content).unwrap_or("Unknown".to_string()));
    println!("Number of entries: {}", feed.entries.len());
    
    // Check first few entries
    for (i, entry) in feed.entries.iter().take(3).enumerate() {
        println!("\n--- Entry {} ---", i + 1);
        println!("Title: {}", entry.title.as_ref().map(|t| &t.content).unwrap_or("No title"));
        println!("ID/GUID: {}", entry.id);
        println!("Links: {}", entry.links.len());
        
        for (j, link) in entry.links.iter().enumerate() {
            println!("  Link {}: {} (type: {:?}, rel: {:?})", 
                j + 1, 
                link.href, 
                link.media_type, 
                link.rel
            );
        }
        
        // Try to find audio
        let audio_link = entry.links.iter().find(|link| {
            link.media_type.as_ref().map(|mt| mt.starts_with("audio/")).unwrap_or(false)
        });
        
        if let Some(audio) = audio_link {
            println!("  -> Found audio: {}", audio.href);
        } else {
            println!("  -> No audio link found with audio MIME type");
            
            // Try file extension check
            let ext_link = entry.links.iter().find(|link| {
                let href = &link.href.to_lowercase();
                let url_path = href.split('?').next().unwrap_or(href);
                url_path.ends_with(".mp3") || url_path.ends_with(".m4a")
            });
            
            if let Some(ext_audio) = ext_link {
                println!("  -> Found audio by extension: {}", ext_audio.href);
            } else {
                println!("  -> No audio found by extension either");
            }
        }
    }
    
    Ok(())
}
EOF

echo "Compiling debug RSS parser..."
cd /tmp
cargo init --name debug_rss . >/dev/null 2>&1
echo 'tokio = { version = "1", features = ["full"] }' >> Cargo.toml
echo 'reqwest = { version = "0.11", features = ["json"] }' >> Cargo.toml
echo 'feed-rs = "1.3"' >> Cargo.toml
mv debug_rss.rs src/main.rs

echo "Testing with Deep Questions feed..."
cargo run --quiet "https://feeds.simplecast.com/l3Apgf3I" 2>/dev/null | head -50

echo ""
echo "Testing with Windows Weekly feed..."
cargo run --quiet "https://feeds.twit.tv/ww.xml" 2>/dev/null | head -50

echo ""
echo "=== Debug completed ==="
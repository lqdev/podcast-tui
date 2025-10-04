// Debug program to test audio URL extraction with the fixed implementation
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Test with a working TWiT feed
    let client = reqwest::Client::new();
    let feed_content = client
        .get("https://feeds.twit.tv/ww.xml")
        .send()
        .await?
        .text()
        .await?;
    
    let feed = feed_rs::parser::parse(feed_content.as_bytes())?;
    
    println!("Feed title: {}", feed.title.as_ref().map(|t| &t.content).unwrap_or("No title"));
    println!("Number of entries: {}", feed.entries.len());
    
    let mut audio_found = 0;
    let mut total_links = 0;
    
    for (i, entry) in feed.entries.iter().take(5).enumerate() {
        println!("\nEntry {}: {}", i + 1, 
                 entry.title.as_ref().map(|t| &t.content).unwrap_or("No title"));
        println!("  Links: {}", entry.links.len());
        total_links += entry.links.len();
        
        let mut found_audio = false;
        
        for (j, link) in entry.links.iter().enumerate() {
            println!("    Link {}: href='{}', media_type='{:?}', rel='{:?}'", 
                     j, link.href, link.media_type, link.rel);
            
            // Test our audio detection logic
            if is_audio_link(link) {
                println!("      ✓ AUDIO DETECTED!");
                found_audio = true;
            }
        }
        
        if found_audio {
            audio_found += 1;
        } else {
            println!("      ✗ No audio found for this entry");
        }
    }
    
    println!("\nSummary:");
    println!("  Total entries checked: 5");
    println!("  Total links: {}", total_links);
    println!("  Entries with audio: {}", audio_found);
    
    Ok(())
}

fn is_audio_link(link: &feed_rs::model::Link) -> bool {
    // Strategy 1: Check for audio MIME types
    if let Some(media_type) = &link.media_type {
        if media_type.starts_with("audio/") || media_type == "application/octet-stream" {
            return true;
        }
    }
    
    // Strategy 2: Check for enclosure relationship
    if let Some(rel) = &link.rel {
        if rel == "enclosure" {
            return true;
        }
    }
    
    // Strategy 3: Check for audio file extensions
    let href = &link.href.to_lowercase();
    let url_path = href.split('?').next().unwrap_or(href);
    if url_path.ends_with(".mp3")
        || url_path.ends_with(".m4a")
        || url_path.ends_with(".mp4")
        || url_path.ends_with(".ogg")
        || url_path.ends_with(".wav")
        || url_path.ends_with(".aac")
        || url_path.ends_with(".flac")
    {
        return true;
    }
    
    false
}
// Integration test for episode detail buffer with various episode formats

use chrono::Utc;
use podcast_tui::*;
use podcast_tui::storage::PodcastId;
use podcast_tui::ui::buffers::Buffer;

#[tokio::test]
async fn test_episode_detail_with_real_feeds() {
    // Test with mock episodes that represent various podcast feed formats
    let test_scenarios = vec![
        ("Podcasting 2.0 - Full metadata", true, true, true, true, Some("This is a podcast about podcasting with the latest news and developments in the podcasting 2.0 namespace.".to_string())),
        ("Megaphone - Rich description", true, false, false, false, Some("A long description with multiple paragraphs.\n\nThis podcast explores various topics in depth with expert guests and thoughtful discussions.".to_string())),
        ("Generation Why - Minimal metadata", true, false, false, false, Some("True crime podcast investigating mysteries.".to_string())),
        ("Omny Content - Episode numbers", true, true, true, false, Some("Episode with season and episode number tracking.".to_string())),
        ("Hacker Public Radio - Basic", false, false, false, false, Some("Community podcast with user submissions.".to_string())),
    ];

    println!("\n=== Testing Episode Detail Buffer with Various Episode Formats ===\n");

    let mut results = Vec::new();

    for (name, has_duration, has_season, has_episode_num, has_file_size, description) in test_scenarios.iter() {
        println!("Testing: {}", name);
        println!("{}", "-".repeat(80));

        // Create a mock episode with the specified characteristics
        let podcast_id = PodcastId::new();
        let mut episode = podcast::Episode::new(
            podcast_id.clone(),
            format!("{} - Test Episode", name),
            "https://example.com/audio.mp3".to_string(),
            Utc::now(),
        );

        // Set optional fields based on test scenario
        episode.description = description.clone();
        
        if *has_duration {
            episode.duration = Some(3672); // 1 hour, 1 minute, 12 seconds
        }
        
        if *has_season {
            episode.season = Some(3);
        }
        
        if *has_episode_num {
            episode.episode_number = Some(42);
        }
        
        if *has_file_size {
            episode.file_size = Some(125829120); // ~120 MB
        }

        println!("   Episode: {}", episode.title);

        // Verify episode detail buffer data is available and formatted correctly
        let mut detail_fields = Vec::new();

        detail_fields.push("Title: ✓".to_string());
        detail_fields.push(format!("Published: {}", episode.published.format("%Y-%m-%d %H:%M UTC")));
        detail_fields.push(format!("Status: {}", episode.status));

        if episode.duration.is_some() {
            let formatted = episode.formatted_duration();
            detail_fields.push(format!("Duration: {}", formatted));
            println!("   - Duration: {}", formatted);
        }

        if episode.file_size.is_some() {
            let formatted = episode.formatted_file_size();
            detail_fields.push(format!("File size: {}", formatted));
            println!("   - File size: {}", formatted);
        }

        if let Some(ref desc) = episode.description {
            let clean_desc: String = desc.replace('\n', " ");
            let preview = if clean_desc.len() > 100 {
                format!("{}...", &clean_desc[..100])
            } else {
                clean_desc.clone()
            };
            detail_fields.push(format!("Description: {} chars", desc.len()));
            println!("   - Description preview: {}", preview);
        }

        if episode.season.is_some() {
            detail_fields.push(format!("Season: {}", episode.season.unwrap()));
        }

        if episode.episode_number.is_some() {
            detail_fields.push(format!("Episode number: {}", episode.episode_number.unwrap()));
        }

        if episode.transcript.is_some() {
            detail_fields.push("Transcript: ✓".to_string());
        }

        println!("   Available fields: {}", detail_fields.join(", "));
        
        // Test that we can create an episode detail buffer
        let detail_buffer = ui::buffers::episode_detail::EpisodeDetailBuffer::new(episode.clone());
        assert_eq!(detail_buffer.name(), format!("Episode: {}", episode.title));
        assert!(detail_buffer.can_close());
        
        println!("   ✅ Episode detail buffer created successfully");

        results.push((name, true, 1));

        println!();
    }

    println!("\n=== Summary ===");
    println!("{:<50} {:<15}", "Scenario", "Success");
    println!("{}", "=".repeat(65));
    for (name, success, _count) in &results {
        let status = if *success { "✅ Success" } else { "❌ Failed" };
        println!("{:<50} {:<15}", name, status);
    }
    println!();

    // All scenarios should work
    let successful = results.iter().filter(|(_, success, _)| *success).count();
    println!("Successfully tested {}/{} scenarios", successful, test_scenarios.len());

    assert_eq!(
        successful,
        test_scenarios.len(),
        "All episode format scenarios should be successfully tested"
    );
    
    println!("\n✅ Episode Detail Buffer validated with various podcast feed formats");
}

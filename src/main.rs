use anyhow::Result;
use clap::{Arg, Command};
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
    cursor,
};
use podcast_tui::{app::App, config::Config, InitStatus};
use std::io::{stdout, Write};
use tokio::sync::mpsc;

/// Render the full cassette tape splash screen (call once at startup)
fn render_splash_screen_initial() -> Result<()> {
    let mut stdout = stdout();
    
    // Clear the entire screen and move to top
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    
    // Define colors
    let orange = Color::Rgb { r: 255, g: 140, b: 0 };
    let brown = Color::Rgb { r: 139, g: 90, b: 43 };
    let dark_brown = Color::Rgb { r: 101, g: 67, b: 33 };
    let beige = Color::Rgb { r: 245, g: 222, b: 179 };
    let black = Color::Black;
    let gray = Color::Rgb { r: 100, g: 100, b: 100 };
    
    println!("\n\n");
    
    // Top of cassette
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("              ╔════════════════════════════════════════════════════════╗");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("              ║                                                        ║");
    
    // Label area with text
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(black))?;
    print!("                                                ");
    execute!(stdout, ResetColor, SetBackgroundColor(Color::Black), SetForegroundColor(dark_brown))?;
    print!("    ║");
    execute!(stdout, ResetColor)?;
    println!();
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(black))?;
    print!("                                                ");
    execute!(stdout, ResetColor, SetBackgroundColor(Color::Black), SetForegroundColor(dark_brown))?;
    print!("    ║");
    println!();
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(Color::Rgb { r: 0, g: 100, b: 200 }))?;
    print!("              ╔═════════════════╗               ");
    execute!(stdout, ResetColor, SetBackgroundColor(Color::Black), SetForegroundColor(dark_brown))?;
    print!("    ║");
    execute!(stdout, ResetColor)?;
    println!();
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(Color::Rgb { r: 0, g: 100, b: 200 }))?;
    print!("              ║  PODCAST  TUI   ║               ");
    execute!(stdout, ResetColor, SetBackgroundColor(Color::Black), SetForegroundColor(dark_brown))?;
    print!("    ║");
    execute!(stdout, ResetColor)?;
    println!();
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(Color::Rgb { r: 0, g: 100, b: 200 }))?;
    print!("              ╚═════════════════╝               ");
    execute!(stdout, ResetColor, SetBackgroundColor(Color::Black), SetForegroundColor(dark_brown))?;
    print!("    ║");
    execute!(stdout, ResetColor)?;
    println!();
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(gray))?;
    print!("             Terminal Podcast Manager           ");
    execute!(stdout, ResetColor, SetBackgroundColor(Color::Black), SetForegroundColor(dark_brown))?;
    print!("    ║");
    execute!(stdout, ResetColor)?;
    println!();
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(black))?;
    print!("                                                ");
    execute!(stdout, ResetColor, SetBackgroundColor(Color::Black), SetForegroundColor(dark_brown))?;
    print!("    ║");
    execute!(stdout, ResetColor)?;
    println!();
    
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("              ║                                                        ║");
    
    // Cassette reels and tape
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║      "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("╔═══════╗");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("══════════════════════════");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("╔═══════╗");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║      "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("███████");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("███████");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║      "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(beige))?;
    print!("   ");
    execute!(stdout, ResetColor, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(beige))?;
    print!("   ");
    execute!(stdout, ResetColor, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║      "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(beige))?;
    print!(" ");
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(black))?;
    print!("█");
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(beige))?;
    print!(" ");
    execute!(stdout, ResetColor, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(beige))?;
    print!(" ");
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(black))?;
    print!("█");
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(beige))?;
    print!(" ");
    execute!(stdout, ResetColor, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║      "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(beige))?;
    print!("   ");
    execute!(stdout, ResetColor, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(beige))?;
    print!("███");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║      "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("███████");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("███████");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║      "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("╚═══════╝");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("══════════════════════════");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("╚═══════╝");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("              ║                                                        ║");
    
    // Bottom screws
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║      "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("  ⊕         ⊕         ⊕         ⊕         ⊕ ");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("              ╚════════════════════════════════════════════════════════╝");
    
    // Version
    println!();
    execute!(stdout, SetForegroundColor(Color::Cyan))?;
    println!("                                 v1.0.0-mvp");
    
    // Initial status message area (line 29)
    execute!(stdout, SetForegroundColor(Color::Yellow))?;
    println!("\n                      Initializing...");
    
    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    
    Ok(())
}

/// Update only the status message at the bottom of the splash screen
fn update_splash_status(status: &str) -> Result<()> {
    let mut stdout = stdout();
    
    // Move cursor to the status line (line 29, column 0)
    execute!(stdout, cursor::MoveTo(0, 29))?;
    
    // Clear the line and write new status
    execute!(stdout, Clear(ClearType::CurrentLine))?;
    execute!(stdout, SetForegroundColor(Color::Yellow))?;
    
    // Center the status message
    let padded_status = format!("{:^80}", status);
    print!("{}", padded_status);
    
    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("podcast-tui")
        .version("1.0.0-mvp")
        .about("A cross-platform terminal user interface for podcast management")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file"),
        )
        .get_matches();

    // Create channel for initialization status updates
    let (status_tx, mut status_rx) = mpsc::unbounded_channel::<InitStatus>();

    // Clear screen and hide cursor for splash screen
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), cursor::Hide)?;
    
    // Render the cassette tape once
    render_splash_screen_initial()?;

    // Spawn a task to monitor status updates and update the display
    let status_monitor = tokio::spawn(async move {
        while let Some(status) = status_rx.recv().await {
            if let Err(e) = update_splash_status(status.message()) {
                eprintln!("Failed to update splash status: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    });

    // Load configuration
    update_splash_status(InitStatus::LoadingConfig.message())?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    let config_path = matches.get_one::<String>("config");
    let config = Config::load_or_default(config_path)?;

    // Initialize app with status updates
    let mut app = App::new_with_progress(config, status_tx.clone()).await?;
    
    // Send final status
    status_tx.send(InitStatus::Complete).ok();
    drop(status_tx); // Close the channel
    
    // Wait for status monitor to finish
    let _ = status_monitor.await;
    
    // Show final status briefly
    update_splash_status(InitStatus::Complete.message())?;
    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

    // Clear splash screen and show cursor
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0), cursor::Show)?;

    // Run the application
    app.run().await?;

    Ok(())
}

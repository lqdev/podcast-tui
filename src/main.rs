use anyhow::Result;
use clap::{Arg, Command};
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    cursor,
};
use podcast_tui::{app::App, config::Config};
use std::io::{stdout, Write};

/// Display a colorful cassette tape splash screen
fn show_splash_screen() -> Result<()> {
    let mut stdout = stdout();
    
    // Clear screen and hide cursor
    execute!(stdout, Clear(ClearType::All), cursor::Hide)?;
    execute!(stdout, cursor::MoveTo(0, 0))?;
    
    // Define colors matching the cassette tape aesthetic
    let orange = Color::Rgb { r: 255, g: 140, b: 0 };
    let brown = Color::Rgb { r: 139, g: 90, b: 43 };
    let dark_brown = Color::Rgb { r: 101, g: 67, b: 33 };
    let beige = Color::Rgb { r: 245, g: 222, b: 179 };
    let white = Color::White;
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
    print!("                                                    ");
    execute!(stdout, ResetColor, SetForegroundColor(dark_brown))?;
    println!("║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(black))?;
    print!("                                                    ");
    execute!(stdout, ResetColor, SetForegroundColor(dark_brown))?;
    println!("║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(Color::Rgb { r: 0, g: 100, b: 200 }))?;
    print!("              ╔═════════════════╗                 ");
    execute!(stdout, ResetColor, SetForegroundColor(dark_brown))?;
    println!("║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(Color::Rgb { r: 0, g: 100, b: 200 }))?;
    print!("              ║  PODCAST  TUI   ║                 ");
    execute!(stdout, ResetColor, SetForegroundColor(dark_brown))?;
    println!("║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(Color::Rgb { r: 0, g: 100, b: 200 }))?;
    print!("              ╚═════════════════╝                 ");
    execute!(stdout, ResetColor, SetForegroundColor(dark_brown))?;
    println!("║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(gray))?;
    print!("             Terminal Podcast Manager            ");
    execute!(stdout, ResetColor, SetForegroundColor(dark_brown))?;
    println!("║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║    "))?;
    execute!(stdout, SetBackgroundColor(beige), SetForegroundColor(black))?;
    print!("                                                    ");
    execute!(stdout, ResetColor, SetForegroundColor(dark_brown))?;
    println!("║");
    
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("              ║                                                        ║");
    
    // Cassette reels and tape
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║       "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("╔═══════╗");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("═══════════════════════");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("╔═══════╗");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║       "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("███████");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("███████");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║       "))?;
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
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
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
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║       "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(beige))?;
    print!("█");
    execute!(stdout, SetForegroundColor(black))?;
    print!("█");
    execute!(stdout, SetForegroundColor(beige))?;
    print!("█");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(beige))?;
    print!("█");
    execute!(stdout, SetForegroundColor(black))?;
    print!("█");
    execute!(stdout, SetForegroundColor(beige))?;
    print!("█");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("██");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║       "))?;
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
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
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
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║       "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("███████");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(brown))?;
    print!("███████");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("║");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║       "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("╚═══════╝");
    execute!(stdout, SetForegroundColor(orange))?;
    print!("═══════════════════════");
    execute!(stdout, SetForegroundColor(gray))?;
    print!("╚═══════╝");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("      ║");
    
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("              ║                                                        ║");
    
    // Bottom screws
    execute!(stdout, SetForegroundColor(dark_brown), Print("              ║     "))?;
    execute!(stdout, SetForegroundColor(gray))?;
    print!("⊕          ⊕          ⊕          ⊕          ⊕");
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("     ║");
    
    execute!(stdout, SetForegroundColor(dark_brown))?;
    println!("              ╚════════════════════════════════════════════════════════╝");
    
    // Version and loading message
    println!();
    execute!(stdout, SetForegroundColor(Color::Cyan))?;
    println!("                                 v1.0.0-mvp");
    execute!(stdout, SetForegroundColor(white))?;
    println!("\n                          Loading your podcasts...");
    execute!(stdout, ResetColor)?;
    
    stdout.flush()?;
    
    // Brief pause to show the splash screen
    std::thread::sleep(std::time::Duration::from_millis(1200));
    
    // Clear screen before starting the app
    execute!(stdout, Clear(ClearType::All))?;
    execute!(stdout, cursor::MoveTo(0, 0))?;
    execute!(stdout, cursor::Show)?;
    
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

    // Show the cassette tape splash screen
    if let Err(e) = show_splash_screen() {
        eprintln!("Failed to show splash screen: {}", e);
    }

    let config_path = matches.get_one::<String>("config");
    let config = Config::load_or_default(config_path)?;

    let mut app = App::new(config).await?;
    app.run().await?;

    Ok(())
}

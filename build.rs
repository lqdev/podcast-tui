fn main() {
    // Only compile Windows resources on Windows
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/podcast-tui.ico");
        res.set("FileDescription", "Podcast TUI - Terminal Podcast Manager");
        res.set("ProductName", "Podcast TUI");
        res.set("LegalCopyright", "Copyright (c) 2024");
        
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}

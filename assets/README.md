# Podcast TUI Assets

This directory contains application assets including icons and platform-specific resources.

## Icons

The application icon combines a cassette tape (representing audio/podcasts) with an RSS feed symbol (representing subscription/syndication). This visual metaphor represents the core functionality of Podcast TUI.

### Icon Files

Located in `icons/`:

- **podcast-tui.svg** - Scalable vector source (256x256 viewBox)
- **podcast-tui.ico** - Windows icon file (multi-resolution)
- **podcast-tui-{size}.png** - PNG exports at various sizes:
  - 16x16 - Taskbar/small icons
  - 32x32 - Standard application icon
  - 48x48 - Large application icon
  - 64x64 - High-DPI small icon
  - 128x128 - High-DPI medium icon
  - 256x256 - High-DPI large icon / app list

### Platform Integration

#### Windows

The icon is automatically embedded in the Windows executable during build via `build.rs`:
- Uses `winres` crate to compile Windows resources
- Icon appears in Task Manager, taskbar, file explorer
- No installation steps required

#### Linux

Icon integration requires installation to system directories:

```bash
# Install icon and desktop entry
./scripts/install-icon-linux.sh
```

This script:
1. Copies PNG icons to `~/.local/share/icons/hicolor/{size}/apps/`
2. Copies SVG icon to `~/.local/share/icons/hicolor/scalable/apps/`
3. Installs desktop entry to `~/.local/share/applications/`
4. Updates icon and desktop caches if available

The icon will appear in:
- Application launchers (GNOME, KDE, XFCE, etc.)
- File managers when browsing to the executable
- Alt+Tab task switchers (desktop environment dependent)

### Icon Design

The icon features:
- **Cassette tape**: Brown/tan colors, represents audio content and retro podcast aesthetic
- **Two reels**: Classic cassette design with magnetic tape between them
- **Label area**: Contains "PODCAST TUI" text
- **RSS symbol**: Orange circular badge in top-right, representing feed/subscription
- **Colors**: Warm browns and tans with an orange RSS accent

The design is intentionally simple and recognizable at small sizes while maintaining detail at larger sizes.

### Regenerating Icons

If you need to modify the icon:

1. Edit `icons/podcast-tui.svg`
2. Regenerate PNG files:
   ```bash
   cd assets/icons
   ./regenerate-icons.sh
   ```
3. Rebuild the application to embed the new Windows icon

### License

The icon is part of the Podcast TUI project and is licensed under the same MIT license as the application.

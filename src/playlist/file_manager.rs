use crate::playlist::PlaylistEpisode;
use crate::utils::validation::sanitize_playlist_name;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PlaylistFileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Source file not found: {0}")]
    SourceNotFound(String),
    #[error("Playlist directory error: {0}")]
    DirectoryError(String),
}

pub struct PlaylistFileManager {
    playlists_dir: PathBuf,
}

impl PlaylistFileManager {
    pub fn new(playlists_dir: PathBuf) -> Self {
        Self { playlists_dir }
    }

    pub async fn copy_episode_to_playlist(
        &self,
        source_path: &Path,
        playlist_name: &str,
        order: usize,
    ) -> Result<String, PlaylistFileError> {
        if !source_path.exists() {
            return Err(PlaylistFileError::SourceNotFound(
                source_path.display().to_string(),
            ));
        }

        let audio_dir = self.playlist_audio_dir(playlist_name);
        fs::create_dir_all(&audio_dir).await?;

        let extension = source_path
            .extension()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("mp3");
        let stem = source_path
            .file_stem()
            .map(|s| s.to_string_lossy().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Untitled".to_string());
        let filename = format!("{:03}-{}.{}", order, stem, extension);
        let target_path = audio_dir.join(&filename);

        fs::copy(source_path, &target_path).await?;
        Ok(filename)
    }

    pub async fn remove_episode_file(
        &self,
        playlist_name: &str,
        filename: &str,
    ) -> Result<(), PlaylistFileError> {
        let file_path = self.playlist_audio_dir(playlist_name).join(filename);
        if file_path.exists() {
            fs::remove_file(file_path).await?;
        }
        Ok(())
    }

    pub async fn rename_files_for_reorder(
        &self,
        playlist_name: &str,
        episodes: &[PlaylistEpisode],
    ) -> Result<Vec<String>, PlaylistFileError> {
        let audio_dir = self.playlist_audio_dir(playlist_name);
        if !audio_dir.exists() {
            return Ok(Vec::new());
        }

        let mut temp_renames = Vec::new();
        let mut final_renames = Vec::new();
        let mut updated_names = Vec::new();

        for (idx, episode) in episodes.iter().enumerate() {
            let Some(old_name) = &episode.filename else {
                continue;
            };

            let old_path = audio_dir.join(old_name);
            if !old_path.exists() {
                return Err(PlaylistFileError::SourceNotFound(
                    old_path.display().to_string(),
                ));
            }

            let rest = old_name
                .split_once('-')
                .map(|(_, tail)| tail.to_string())
                .unwrap_or_else(|| old_name.clone());
            let new_name = format!("{:03}-{}", idx + 1, rest);
            let new_path = audio_dir.join(&new_name);
            updated_names.push(new_name.clone());

            if old_path == new_path {
                continue;
            }

            let temp_name = format!("{}.tmp-reorder-{}", old_name, idx);
            let temp_path = audio_dir.join(temp_name);
            temp_renames.push((old_path, temp_path.clone()));
            final_renames.push((temp_path, new_path));
        }

        for (old_path, temp_path) in temp_renames {
            fs::rename(old_path, temp_path).await?;
        }
        for (temp_path, new_path) in final_renames {
            fs::rename(temp_path, new_path).await?;
        }

        Ok(updated_names)
    }

    pub async fn cleanup_orphaned_files(
        &self,
        playlist_name: &str,
        valid_filenames: &[String],
    ) -> Result<usize, PlaylistFileError> {
        let audio_dir = self.playlist_audio_dir(playlist_name);
        if !audio_dir.exists() {
            return Ok(0);
        }

        let valid: HashSet<&str> = valid_filenames.iter().map(String::as_str).collect();
        let mut entries = fs::read_dir(&audio_dir).await?;
        let mut deleted = 0usize;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };

            if !valid.contains(name) {
                fs::remove_file(path).await?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }

    pub async fn get_playlist_size(&self, playlist_name: &str) -> Result<u64, PlaylistFileError> {
        let audio_dir = self.playlist_audio_dir(playlist_name);
        if !audio_dir.exists() {
            return Ok(0);
        }

        let mut entries = fs::read_dir(&audio_dir).await?;
        let mut total_size = 0u64;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            total_size += fs::metadata(path).await?.len();
        }

        Ok(total_size)
    }

    pub fn playlist_audio_dir(&self, playlist_name: &str) -> PathBuf {
        self.playlists_dir
            .join(sanitize_playlist_name(playlist_name))
            .join("audio")
    }

    /// Write (or overwrite) the `.m3u` playlist manifest for `playlist_name`.
    ///
    /// The manifest lives at `<playlists_dir>/<sanitized-name>/<sanitized-name>.m3u`,
    /// alongside the `audio/` directory, and references each copied episode by the
    /// relative path `audio/<filename>`. Episodes without a copied file (`filename`
    /// is `None`) are skipped. The file uses the `.m3u` extension (broadest player
    /// recognition) with UTF-8 content so Unicode titles round-trip. The write is
    /// atomic (temp file + rename) to avoid leaving a half-written manifest behind.
    pub async fn write_m3u(
        &self,
        playlist_name: &str,
        episodes: &[PlaylistEpisode],
    ) -> Result<(), PlaylistFileError> {
        let sanitized = sanitize_playlist_name(playlist_name);
        let playlist_dir = self.playlists_dir.join(&sanitized);
        fs::create_dir_all(&playlist_dir).await?;

        let mut content = String::from("#EXTM3U\n");
        for episode in episodes {
            let Some(filename) = &episode.filename else {
                continue;
            };

            let title = episode
                .episode_title
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string())
                .unwrap_or_else(|| {
                    Path::new(filename)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| filename.clone())
                });
            // Collapse any newlines so a single #EXTINF line is never broken.
            let title = title.replace(['\r', '\n'], " ");

            content.push_str(&format!("#EXTINF:-1,{title}\n"));
            content.push_str(&format!("audio/{filename}\n"));
        }

        let m3u_path = playlist_dir.join(format!("{sanitized}.m3u"));
        let temp_path = playlist_dir.join(format!(".{sanitized}.{}.m3u.tmp", Uuid::new_v4()));
        fs::write(&temp_path, content.as_bytes()).await?;
        if let Err(e) = fs::rename(&temp_path, &m3u_path).await {
            let _ = fs::remove_file(&temp_path).await;
            return Err(PlaylistFileError::Io(e));
        }
        Ok(())
    }

    /// Transform a local `.m3u` manifest into the layout used on a synced device.
    ///
    /// The local manifest references audio via the relative path `audio/<file>`
    /// (the files live in an `audio/` subfolder). Device sync flattens each
    /// playlist so the audio files land directly under `Playlists/<name>/<file>`
    /// with no `audio/` subfolder. This rewrites every path line by stripping a
    /// leading `audio/` (or `audio\`) segment, leaving `#EXTM3U`/`#EXTINF`
    /// directive lines and any non-`audio/` paths untouched. The line structure
    /// and order are preserved verbatim so titles round-trip.
    pub fn transform_m3u_for_device(content: &str) -> String {
        let mut out = String::with_capacity(content.len());
        for line in content.split_inclusive('\n') {
            let (body, eol) = match line.strip_suffix('\n') {
                Some(b) => (b, "\n"),
                None => (line, ""),
            };
            let trimmed = body.trim_start();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                out.push_str(body);
            } else {
                let stripped = body
                    .strip_prefix("audio/")
                    .or_else(|| body.strip_prefix("audio\\"))
                    .unwrap_or(body);
                out.push_str(stripped);
            }
            out.push_str(eol);
        }
        out
    }

    pub async fn delete_playlist_directory(
        &self,
        playlist_name: &str,
    ) -> Result<(), PlaylistFileError> {
        let playlist_dir = self
            .playlists_dir
            .join(sanitize_playlist_name(playlist_name));
        if playlist_dir.exists() {
            fs::remove_dir_all(playlist_dir).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::PlaylistEpisode;
    use crate::storage::{EpisodeId, PodcastId};
    use chrono::Utc;
    use tempfile::TempDir;

    fn test_episode(filename: Option<String>, order: usize) -> PlaylistEpisode {
        PlaylistEpisode {
            podcast_id: PodcastId::new(),
            episode_id: EpisodeId::new(),
            episode_title: None,
            added_at: Utc::now(),
            order,
            file_synced: true,
            filename,
        }
    }

    #[tokio::test]
    async fn test_copy_episode_creates_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let playlists_dir = temp.path().join("Playlists");
        let source = temp.path().join("source.mp3");
        fs::write(&source, b"abc")
            .await
            .expect("Failed to write source");

        let manager = PlaylistFileManager::new(playlists_dir.clone());
        let filename = manager
            .copy_episode_to_playlist(&source, "Morning Commute", 1)
            .await
            .expect("Failed to copy episode");
        let copied = manager.playlist_audio_dir("Morning Commute").join(filename);
        assert!(copied.exists());
    }

    #[tokio::test]
    async fn test_remove_episode_deletes_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let manager = PlaylistFileManager::new(temp.path().join("Playlists"));
        let audio_dir = manager.playlist_audio_dir("Delete Test");
        fs::create_dir_all(&audio_dir)
            .await
            .expect("Failed to create audio dir");
        let file = audio_dir.join("001-test.mp3");
        fs::write(&file, b"abc")
            .await
            .expect("Failed to write test file");

        manager
            .remove_episode_file("Delete Test", "001-test.mp3")
            .await
            .expect("Failed to remove file");
        assert!(!file.exists());
    }

    #[tokio::test]
    async fn test_rename_for_reorder() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let manager = PlaylistFileManager::new(temp.path().join("Playlists"));
        let audio_dir = manager.playlist_audio_dir("Reorder Test");
        fs::create_dir_all(&audio_dir)
            .await
            .expect("Failed to create audio dir");

        fs::write(audio_dir.join("001-first.mp3"), b"a")
            .await
            .expect("Failed to write first file");
        fs::write(audio_dir.join("002-second.mp3"), b"b")
            .await
            .expect("Failed to write second file");

        let episodes = vec![
            test_episode(Some("002-second.mp3".to_string()), 0),
            test_episode(Some("001-first.mp3".to_string()), 1),
        ];

        let updated = manager
            .rename_files_for_reorder("Reorder Test", &episodes)
            .await
            .expect("Failed to reorder files");

        assert_eq!(updated[0], "001-second.mp3");
        assert_eq!(updated[1], "002-first.mp3");
        assert!(audio_dir.join("001-second.mp3").exists());
        assert!(audio_dir.join("002-first.mp3").exists());
    }

    #[tokio::test]
    async fn test_cleanup_orphaned_files() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let manager = PlaylistFileManager::new(temp.path().join("Playlists"));
        let audio_dir = manager.playlist_audio_dir("Cleanup Test");
        fs::create_dir_all(&audio_dir)
            .await
            .expect("Failed to create audio dir");

        fs::write(audio_dir.join("001-keep.mp3"), b"a")
            .await
            .expect("Failed to write keep file");
        fs::write(audio_dir.join("002-delete.mp3"), b"b")
            .await
            .expect("Failed to write delete file");

        let deleted = manager
            .cleanup_orphaned_files("Cleanup Test", &["001-keep.mp3".to_string()])
            .await
            .expect("Failed to cleanup files");

        assert_eq!(deleted, 1);
        assert!(audio_dir.join("001-keep.mp3").exists());
        assert!(!audio_dir.join("002-delete.mp3").exists());
    }

    #[tokio::test]
    async fn test_get_playlist_size() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let manager = PlaylistFileManager::new(temp.path().join("Playlists"));
        let audio_dir = manager.playlist_audio_dir("Size Test");
        fs::create_dir_all(&audio_dir)
            .await
            .expect("Failed to create audio dir");

        fs::write(audio_dir.join("001-one.mp3"), b"12345")
            .await
            .expect("Failed to write first file");
        fs::write(audio_dir.join("002-two.mp3"), b"1234")
            .await
            .expect("Failed to write second file");

        let size = manager
            .get_playlist_size("Size Test")
            .await
            .expect("Failed to get playlist size");
        assert_eq!(size, 9);
    }

    fn titled_episode(
        filename: Option<String>,
        title: Option<&str>,
        order: usize,
    ) -> PlaylistEpisode {
        PlaylistEpisode {
            episode_title: title.map(str::to_string),
            ..test_episode(filename, order)
        }
    }

    #[tokio::test]
    async fn test_write_m3u_content_order_and_relative_paths() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let manager = PlaylistFileManager::new(temp.path().join("Playlists"));

        let episodes = vec![
            titled_episode(Some("001-intro.mp3".to_string()), Some("Intro"), 1),
            titled_episode(Some("002-deep-dive.mp3".to_string()), Some("Deep Dive"), 2),
        ];

        manager
            .write_m3u("Morning Commute", &episodes)
            .await
            .expect("Failed to write m3u");

        let m3u_path = temp
            .path()
            .join("Playlists")
            .join("Morning Commute")
            .join("Morning Commute.m3u");
        assert!(m3u_path.exists(), ".m3u file should exist");

        let content = fs::read_to_string(&m3u_path)
            .await
            .expect("Failed to read m3u");
        let expected = "#EXTM3U\n\
            #EXTINF:-1,Intro\n\
            audio/001-intro.mp3\n\
            #EXTINF:-1,Deep Dive\n\
            audio/002-deep-dive.mp3\n";
        assert_eq!(content, expected);
    }

    #[tokio::test]
    async fn test_write_m3u_skips_episodes_without_filename() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let manager = PlaylistFileManager::new(temp.path().join("Playlists"));

        let episodes = vec![
            titled_episode(Some("001-have.mp3".to_string()), Some("Have File"), 1),
            titled_episode(None, Some("No File Yet"), 2),
        ];

        manager
            .write_m3u("Skip Test", &episodes)
            .await
            .expect("Failed to write m3u");

        let content = fs::read_to_string(
            temp.path()
                .join("Playlists")
                .join("Skip Test")
                .join("Skip Test.m3u"),
        )
        .await
        .expect("Failed to read m3u");

        assert!(content.contains("audio/001-have.mp3"));
        assert!(!content.contains("No File Yet"));
    }

    #[tokio::test]
    async fn test_write_m3u_unicode_title_roundtrips() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let manager = PlaylistFileManager::new(temp.path().join("Playlists"));

        let episodes = vec![titled_episode(
            Some("001-episode.mp3".to_string()),
            Some("Café ☕ — naïve"),
            1,
        )];

        manager
            .write_m3u("Unicode Test", &episodes)
            .await
            .expect("Failed to write m3u");

        let content = fs::read_to_string(
            temp.path()
                .join("Playlists")
                .join("Unicode Test")
                .join("Unicode Test.m3u"),
        )
        .await
        .expect("Failed to read m3u");
        assert!(content.contains("#EXTINF:-1,Café ☕ — naïve"));
    }

    #[tokio::test]
    async fn test_write_m3u_empty_playlist_writes_header_only() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let manager = PlaylistFileManager::new(temp.path().join("Playlists"));

        manager
            .write_m3u("Empty Test", &[])
            .await
            .expect("Failed to write m3u");

        let content = fs::read_to_string(
            temp.path()
                .join("Playlists")
                .join("Empty Test")
                .join("Empty Test.m3u"),
        )
        .await
        .expect("Failed to read m3u");
        assert_eq!(content, "#EXTM3U\n");
    }

    #[tokio::test]
    async fn test_write_m3u_falls_back_to_filename_stem_when_no_title() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let manager = PlaylistFileManager::new(temp.path().join("Playlists"));

        let episodes = vec![titled_episode(
            Some("003-fallback.mp3".to_string()),
            None,
            3,
        )];

        manager
            .write_m3u("Fallback Test", &episodes)
            .await
            .expect("Failed to write m3u");

        let content = fs::read_to_string(
            temp.path()
                .join("Playlists")
                .join("Fallback Test")
                .join("Fallback Test.m3u"),
        )
        .await
        .expect("Failed to read m3u");
        assert!(content.contains("#EXTINF:-1,003-fallback"));
    }

    #[test]
    fn test_transform_m3u_strips_audio_prefix() {
        let local = "#EXTM3U\n#EXTINF:-1,Episode One\naudio/001-episode.mp3\n";
        let device = PlaylistFileManager::transform_m3u_for_device(local);
        assert_eq!(device, "#EXTM3U\n#EXTINF:-1,Episode One\n001-episode.mp3\n");
    }

    #[test]
    fn test_transform_m3u_preserves_directives_and_order() {
        let local = "#EXTM3U\n\
            #EXTINF:-1,One\naudio/001-a.mp3\n\
            #EXTINF:-1,Two\naudio/002-b.mp3\n";
        let device = PlaylistFileManager::transform_m3u_for_device(local);
        assert_eq!(
            device,
            "#EXTM3U\n\
             #EXTINF:-1,One\n001-a.mp3\n\
             #EXTINF:-1,Two\n002-b.mp3\n"
        );
    }

    #[test]
    fn test_transform_m3u_header_only_unchanged() {
        assert_eq!(
            PlaylistFileManager::transform_m3u_for_device("#EXTM3U\n"),
            "#EXTM3U\n"
        );
    }

    #[test]
    fn test_transform_m3u_leaves_non_audio_paths_untouched() {
        // A path line without the `audio/` prefix is passed through verbatim.
        let local = "#EXTM3U\n#EXTINF:-1,X\n001-already-flat.mp3\n";
        assert_eq!(PlaylistFileManager::transform_m3u_for_device(local), local);
    }

    #[test]
    fn test_transform_m3u_strips_backslash_prefix() {
        let local = "#EXTM3U\naudio\\001-episode.mp3\n";
        assert_eq!(
            PlaylistFileManager::transform_m3u_for_device(local),
            "#EXTM3U\n001-episode.mp3\n"
        );
    }
}

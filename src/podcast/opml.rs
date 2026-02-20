//! OPML import/export support for podcast subscriptions
//!
//! This module provides functionality to import podcast subscriptions from OPML files
//! (Outline Processor Markup Language) and export current subscriptions to OPML format.
//!
//! # Features
//!
//! - Import from local OPML files or URLs
//! - Non-destructive import (skips duplicates)
//! - Sequential processing with progress callbacks
//! - Detailed error logging and reporting
//! - Export to configurable location with timestamped filenames
//! - OPML 2.0 standard compliance
//!
//! # Examples
//!
//! ```no_run
//! use podcast_tui::podcast::opml::{OpmlParser, OpmlExporter};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Parse an OPML file
//! let parser = OpmlParser::new();
//! let document = parser.parse("~/podcasts.opml").await?;
//!
//! // Export podcasts to OPML
//! let exporter = OpmlExporter::new();
//! // exporter.export(&podcasts, &output_path).await?;
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use chrono::Utc;
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

use crate::podcast::Podcast;

/// OPML parser for importing podcast subscriptions
pub struct OpmlParser {
    client: Client,
}

impl OpmlParser {
    /// Create new OPML parser with HTTP client
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; podcast-tui/1.0)")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Parse OPML from file path or URL
    ///
    /// Accepts both local file paths and HTTP(S) URLs.
    /// Returns parsed OPML document or validation error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> anyhow::Result<()> {
    /// use podcast_tui::podcast::opml::OpmlParser;
    ///
    /// let parser = OpmlParser::new();
    ///
    /// // From file
    /// let doc = parser.parse("~/podcasts.opml").await?;
    ///
    /// // From URL
    /// let doc = parser.parse("https://example.com/subscriptions.opml").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn parse(&self, source: &str) -> Result<OpmlDocument, OpmlError> {
        // Expand tilde and get absolute path
        let expanded_source = shellexpand::tilde(source).to_string();

        // Determine if source is URL or file path
        let xml_content = if Self::is_url(&expanded_source) {
            self.download_opml(&expanded_source).await?
        } else {
            tokio::fs::read_to_string(&expanded_source)
                .await
                .map_err(OpmlError::FileRead)?
        };

        // Validate OPML structure before parsing
        Self::validate_opml(&xml_content)?;

        // Sanitize XML to handle unescaped ampersands (common in real-world OPML files)
        let sanitized_xml = Self::sanitize_xml(&xml_content);

        // Parse the OPML document
        let opml: OpmlRoot = from_str(&sanitized_xml)
            .map_err(|e| OpmlError::ParseError(format!("XML parsing failed: {}", e)))?;

        // Convert to our document structure
        let document = OpmlDocument {
            version: opml.version,
            head: opml.head.map(|h| OpmlHead {
                title: h.title,
                date_created: h.date_created,
            }),
            outlines: Self::flatten_outlines(opml.body.outlines),
        };

        // Check if we have any feeds
        if document.outlines.is_empty() {
            return Err(OpmlError::NoFeeds);
        }

        Ok(document)
    }

    /// Sanitize XML to handle common real-world issues
    ///
    /// Many OPML files have unescaped ampersands in attribute values (e.g., title="Security & Privacy")
    /// This method fixes those issues to make the XML parseable.
    fn sanitize_xml(xml: &str) -> String {
        // Replace all & with &amp;, then fix the ones that were already escaped
        let step1 = xml.replace("&", "&amp;");
        // Fix double-escaping: &amp;amp; -> &amp;, &amp;lt; -> &lt;, etc.
        let step2 = step1
            .replace("&amp;amp;", "&amp;")
            .replace("&amp;lt;", "&lt;")
            .replace("&amp;gt;", "&gt;")
            .replace("&amp;quot;", "&quot;")
            .replace("&amp;apos;", "&apos;");
        // Fix numeric entities: &amp;#123; -> &#123;
        let re = Regex::new(r"&amp;#(\d+);").unwrap();
        re.replace_all(&step2, "&#$1;").to_string()
    }

    /// Validate OPML structure before attempting to parse
    fn validate_opml(xml: &str) -> Result<(), OpmlError> {
        // Basic XML validation
        if !xml.trim_start().starts_with("<?xml") && !xml.trim_start().starts_with("<opml") {
            return Err(OpmlError::ValidationError(
                "Not a valid XML document".to_string(),
            ));
        }

        // Check for required OPML elements
        if !xml.contains("<opml") {
            return Err(OpmlError::ValidationError(
                "Missing <opml> root element".to_string(),
            ));
        }

        if !xml.contains("<body") {
            return Err(OpmlError::ValidationError(
                "Missing <body> element".to_string(),
            ));
        }

        Ok(())
    }

    /// Download OPML from URL
    async fn download_opml(&self, url: &str) -> Result<String, OpmlError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(OpmlError::NetworkError)?;

        if !response.status().is_success() {
            return Err(OpmlError::NetworkError(
                response.error_for_status().unwrap_err(),
            ));
        }

        let content = response.text().await.map_err(OpmlError::NetworkError)?;

        Ok(content)
    }

    /// Check if source is a URL
    fn is_url(source: &str) -> bool {
        source.starts_with("http://") || source.starts_with("https://")
    }

    /// Flatten nested outlines (we only support flat structure for MVP)
    fn flatten_outlines(outlines: Vec<OpmlOutlineRaw>) -> Vec<OpmlOutline> {
        let mut result = Vec::new();

        for outline in outlines {
            // Only include outlines with feed URLs
            if outline.xml_url.is_some() || outline.url.is_some() {
                // OPML 2.0 spec requires @text, but real-world files often use @title instead
                // Use @text if present, otherwise fall back to @title, or use "Untitled" as last resort
                let text = outline
                    .text
                    .or_else(|| outline.title.clone())
                    .unwrap_or_else(|| "Untitled".to_string());

                result.push(OpmlOutline {
                    text,
                    title: outline.title,
                    xml_url: outline.xml_url.clone(),
                    url: outline.url.clone(),
                    description: outline.description,
                    outline_type: outline.outline_type,
                });
            }

            // Recursively process children (flatten them)
            if let Some(children) = outline.outlines {
                result.extend(Self::flatten_outlines(children));
            }
        }

        result
    }
}

impl Default for OpmlParser {
    fn default() -> Self {
        Self::new()
    }
}

/// OPML exporter for podcast subscriptions
pub struct OpmlExporter;

impl OpmlExporter {
    /// Create new OPML exporter
    pub fn new() -> Self {
        Self
    }

    /// Export podcasts to OPML file
    ///
    /// Generates a valid OPML 2.0 document and writes it to the specified path.
    /// Uses atomic write pattern (temp file + rename) for data safety.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> anyhow::Result<()> {
    /// use podcast_tui::podcast::opml::OpmlExporter;
    /// use std::path::Path;
    ///
    /// let exporter = OpmlExporter::new();
    /// let podcasts = vec![]; // Your podcasts
    /// exporter.export(&podcasts, Path::new("~/podcasts-export.opml")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn export(&self, podcasts: &[Podcast], path: &Path) -> Result<(), OpmlError> {
        // Generate OPML XML
        let opml_xml = self.generate_opml(podcasts)?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| OpmlError::DirectoryCreation(e.to_string()))?;
        }

        // Write atomically (temp file + rename)
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, opml_xml)
            .await
            .map_err(OpmlError::FileRead)?;

        tokio::fs::rename(&temp_path, path)
            .await
            .map_err(OpmlError::FileRead)?;

        Ok(())
    }

    /// Generate OPML XML from podcast list
    fn generate_opml(&self, podcasts: &[Podcast]) -> Result<String, OpmlError> {
        let outlines: Vec<OpmlOutlineRaw> = podcasts
            .iter()
            .map(|p| OpmlOutlineRaw {
                outline_type: Some("rss".to_string()),
                // For export, include both @text and @title for maximum compatibility
                text: Some(p.title.clone()),
                title: Some(p.title.clone()),
                xml_url: Some(p.url.clone()),
                url: None,
                description: p.description.clone(),
                outlines: None,
            })
            .collect();

        let opml = OpmlRoot {
            version: "2.0".to_string(),
            head: Some(OpmlHeadRaw {
                title: Some("Podcast Subscriptions".to_string()),
                date_created: Some(Utc::now().to_rfc2822()),
            }),
            body: OpmlBodyRaw { outlines },
        };

        // Serialize to XML with proper formatting
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        let opml_xml = to_string(&opml)
            .map_err(|e| OpmlError::ParseError(format!("Failed to generate XML: {}", e)))?;
        xml.push_str(&opml_xml);

        Ok(xml)
    }
}

impl Default for OpmlExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Parsed OPML document
#[derive(Debug, Clone)]
pub struct OpmlDocument {
    pub version: String,
    pub head: Option<OpmlHead>,
    pub outlines: Vec<OpmlOutline>,
}

/// OPML head metadata
#[derive(Debug, Clone)]
pub struct OpmlHead {
    pub title: Option<String>,
    pub date_created: Option<String>,
}

/// OPML outline element (feed)
#[derive(Debug, Clone)]
pub struct OpmlOutline {
    pub text: String,
    pub title: Option<String>,
    pub xml_url: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub outline_type: Option<String>,
}

impl OpmlOutline {
    /// Get the feed URL (prefer xmlUrl over url)
    pub fn feed_url(&self) -> Option<&str> {
        self.xml_url.as_deref().or(self.url.as_deref())
    }
}

/// Result of OPML import operation
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub total_feeds: usize,
    pub imported: usize,
    pub skipped: usize,
    pub failed: Vec<FailedImport>,
}

impl ImportResult {
    /// Create a new import result
    pub fn new(total_feeds: usize) -> Self {
        Self {
            total_feeds,
            imported: 0,
            skipped: 0,
            failed: Vec::new(),
        }
    }

    /// Format summary for display
    pub fn summary(&self) -> String {
        format!(
            "Total: {}, Imported: {}, Skipped: {}, Failed: {}",
            self.total_feeds,
            self.imported,
            self.skipped,
            self.failed.len()
        )
    }

    /// Check if any imports failed
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }

    /// Get detailed failure report
    pub fn failure_report(&self) -> String {
        if self.failed.is_empty() {
            return String::new();
        }

        let mut report = String::from("Failed imports:\n");
        for (i, failure) in self.failed.iter().enumerate() {
            report.push_str(&format!(
                "{}. {} ({}): {}\n",
                i + 1,
                failure.title.as_deref().unwrap_or("Unknown"),
                failure.url,
                failure.error
            ));
        }
        report
    }
}

/// Failed import details
#[derive(Debug, Clone)]
pub struct FailedImport {
    pub url: String,
    pub title: Option<String>,
    pub error: String,
}

/// OPML operation errors
#[derive(Debug, thiserror::Error)]
pub enum OpmlError {
    #[error("Failed to read OPML file: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Failed to download OPML: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid OPML format: {0}")]
    InvalidFormat(String),

    #[error("Failed to parse OPML XML: {0}")]
    ParseError(String),

    #[error("OPML validation failed: {0}")]
    ValidationError(String),

    #[error("No feeds found in OPML file")]
    NoFeeds,

    #[error("Failed to create directory: {0}")]
    DirectoryCreation(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

// Internal structures for XML serialization/deserialization

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "opml")]
struct OpmlRoot {
    #[serde(rename = "@version")]
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<OpmlHeadRaw>,
    body: OpmlBodyRaw,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpmlHeadRaw {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(rename = "dateCreated", skip_serializing_if = "Option::is_none")]
    date_created: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpmlBodyRaw {
    #[serde(rename = "outline", default)]
    outlines: Vec<OpmlOutlineRaw>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpmlOutlineRaw {
    #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
    outline_type: Option<String>,
    // Note: OPML 2.0 spec requires @text, but many real-world files use @title instead
    // Make @text optional and fall back to @title if not present
    #[serde(rename = "@text", skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(rename = "@title", skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(rename = "@xmlUrl", skip_serializing_if = "Option::is_none")]
    xml_url: Option<String>,
    #[serde(rename = "@url", skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(rename = "@description", skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(rename = "outline", default, skip_serializing_if = "Option::is_none")]
    outlines: Option<Vec<OpmlOutlineRaw>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_url() {
        assert!(OpmlParser::is_url("http://example.com/feed.opml"));
        assert!(OpmlParser::is_url("https://example.com/feed.opml"));
        assert!(!OpmlParser::is_url("/path/to/file.opml"));
        assert!(!OpmlParser::is_url("~/podcasts.opml"));
    }

    #[test]
    fn test_validate_opml_valid() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head>
    <title>Test</title>
  </head>
  <body>
    <outline text="Test" />
  </body>
</opml>"#;
        assert!(OpmlParser::validate_opml(xml).is_ok());
    }

    #[test]
    fn test_validate_opml_invalid() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss>Invalid</rss>"#;
        assert!(OpmlParser::validate_opml(xml).is_err());
    }

    #[tokio::test]
    async fn test_parse_valid_opml() {
        let opml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head>
    <title>My Podcasts</title>
  </head>
  <body>
    <outline type="rss" text="Test Podcast" xmlUrl="https://example.com/feed.xml"/>
  </body>
</opml>"#;

        // Create a temp file
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().join("test.opml");
        tokio::fs::write(&temp_path, opml_content).await.unwrap();

        let parser = OpmlParser::new();
        let result = parser.parse(temp_path.to_str().unwrap()).await;

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.version, "2.0");
        assert_eq!(doc.outlines.len(), 1);
        assert_eq!(doc.outlines[0].text, "Test Podcast");
    }

    #[test]
    fn test_import_result_summary() {
        let mut result = ImportResult::new(10);
        result.imported = 7;
        result.skipped = 2;
        result.failed.push(FailedImport {
            url: "https://example.com/feed.xml".to_string(),
            title: Some("Failed Podcast".to_string()),
            error: "Network error".to_string(),
        });

        let summary = result.summary();
        assert!(summary.contains("Total: 10"));
        assert!(summary.contains("Imported: 7"));
        assert!(summary.contains("Skipped: 2"));
        assert!(summary.contains("Failed: 1"));
    }

    #[tokio::test]
    async fn test_export_opml() {
        use crate::podcast::Podcast;
        use crate::storage::PodcastId;

        let podcasts = vec![Podcast {
            id: PodcastId::from_url("https://example.com/feed1.xml"),
            title: "Test Podcast 1".to_string(),
            url: "https://example.com/feed1.xml".to_string(),
            description: Some("A test podcast".to_string()),
            author: Some("Test Author".to_string()),
            image_url: None,
            language: Some("en".to_string()),
            categories: vec![],
            explicit: false,
            last_updated: Utc::now(),
            episodes: vec![],
        }];

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().join("export.opml");

        let exporter = OpmlExporter::new();
        let result = exporter.export(&podcasts, &temp_path).await;

        assert!(result.is_ok());
        assert!(temp_path.exists());

        // Verify the content
        let content = tokio::fs::read_to_string(&temp_path).await.unwrap();
        assert!(content.contains("<?xml"));
        assert!(content.contains("<opml"));
        assert!(content.contains("Test Podcast 1"));
        assert!(content.contains("https://example.com/feed1.xml"));
    }
}

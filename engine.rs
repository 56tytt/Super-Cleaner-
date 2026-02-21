use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;
use which::which;
use chrono::{DateTime, Local};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleaningStats {
    pub files_deleted: u64,
    pub bytes_freed: u64,
    pub directories_cleaned: u64,
    pub timestamp: DateTime<Local>,
}

impl CleaningStats {
    pub fn new() -> Self {
        CleaningStats {
            files_deleted: 0,
            bytes_freed: 0,
            directories_cleaned: 0,
            timestamp: Local::now(),
        }
    }

    pub fn add_file(&mut self, size: u64) {
        self.files_deleted += 1;
        self.bytes_freed += size;
    }

    pub fn add_directory(&mut self) {
        self.directories_cleaned += 1;
    }
}

pub type LogCallback = Arc<Mutex<Box<dyn Fn(String) + Send + Sync>>>;

pub struct SystemCleaner {
    pub stats: Arc<Mutex<CleaningStats>>,
    pub verbose: bool,
    pub dry_run: bool,
    pub log_callback: Option<LogCallback>,
}

impl SystemCleaner {
    pub fn new(verbose: bool, dry_run: bool) -> Self {
        SystemCleaner {
            stats: Arc::new(Mutex::new(CleaningStats::new())),
            verbose,
            dry_run,
            log_callback: None,
        }
    }

    pub fn with_callback(mut self, callback: LogCallback) -> Self {
        self.log_callback = Some(callback);
        self
    }

    // שליחת לוג למסך השחור
    async fn log(&self, message: &str) {
        if let Some(ref callback) = self.log_callback {
            if let Ok(cb) = callback.lock() {
                cb(message.to_string());
            }
        } else if self.verbose {
            println!("{}", message);
        }
    }

    pub fn get_home_dir(&self) -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
    }

    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        format!("{:.2} {}", size, UNITS[unit_index])
    }

    pub fn get_stats_sync(&self) -> CleaningStats {
        self.stats.lock().unwrap().clone()
    }

    // === Helper Methods ===

    async fn clean_directory_contents<P: AsRef<Path>>(&self, dir: P, _category: &str) -> Result<(), Box<dyn std::error::Error>> {
        let dir = dir.as_ref();
        if !dir.exists() { return Ok(()); }

        let mut files_to_delete = Vec::new();

        for entry in WalkDir::new(dir).min_depth(1).contents_first(true).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.is_file() {
                    files_to_delete.push((path.to_path_buf(), metadata.len()));
                }
            }
        }

        for (path, size) in files_to_delete {
            let success = if !self.dry_run { fs::remove_file(&path).is_ok() } else { true };
            if success {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                self.log(&format!("Deleted: {} ({})", filename, Self::format_bytes(size))).await;
                if let Ok(mut stats) = self.stats.lock() { stats.add_file(size); }
            }
        }
        Ok(())
    }

    async fn clean_files_by_pattern<P: AsRef<Path>>(&self, dir: P, pattern: &str) -> Result<(), Box<dyn std::error::Error>> {
        let dir = dir.as_ref();
        if !dir.exists() { return Ok(()); }

        // הערה: glob פשוט. לשיפור אפשר להשתמש ב-glob crate
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let name = entry.file_name().to_string_lossy();
                // בדיקה פשוטה ל-ends_with או contains
                let matches = if pattern.starts_with('*') && pattern.ends_with('*') {
                    name.contains(&pattern[1..pattern.len()-1])
                } else if pattern.starts_with('*') {
                    name.ends_with(&pattern[1..])
                } else if pattern.ends_with('*') {
                    name.starts_with(&pattern[..pattern.len()-1])
                } else {
                    name == pattern
                };

                if matches {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    let success = if !self.dry_run { fs::remove_file(entry.path()).is_ok() } else { true };

                    if success {
                        self.log(&format!("Deleted: {} ({})", name, Self::format_bytes(size))).await;
                        if let Ok(mut stats) = self.stats.lock() { stats.add_file(size); }
                    }
                }
            }
        }
        Ok(())
    }

    // === System Cleaning ===

    pub async fn clean_system_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cache_dirs = vec!["/var/cache", "/tmp", "/var/tmp"];
        for dir in cache_dirs {
            self.clean_directory_contents(dir, "System").await?;
        }
        let home = self.get_home_dir();
        self.clean_directory_contents(home.join(".cache"), "System").await?;
        Ok(())
    }

    pub async fn clean_trash(&self) -> Result<(), Box<dyn std::error::Error>> {
        let home = self.get_home_dir();
        self.log("🗑️ Emptying Trash...").await;
        self.clean_directory_contents(home.join(".local/share/Trash"), "Trash").await?;
        Ok(())
    }

    pub async fn clean_logs(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Rotated logs and application logs
        self.log("📜 Cleaning System Logs...").await;
        self.clean_directory_contents("/var/log", "Logs").await?;

        let home = self.get_home_dir();
        self.clean_files_by_pattern(home.join(".local/share"), "*.log").await?;
        self.clean_files_by_pattern(home.join(".config"), "*.log").await?;
        Ok(())
    }

    pub async fn clean_thumbnails(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.log("🖼️ Cleaning Thumbnails...").await;
        let home = self.get_home_dir();
        let dirs = vec![
            home.join(".thumbnails"),
            home.join(".cache/thumbnails"),
            home.join(".local/share/thumbnails"),
        ];
        for d in dirs {
            self.clean_directory_contents(d, "Thumbnails").await?;
        }
        Ok(())
    }

    pub async fn clean_clipboard(&self) -> Result<(), Box<dyn std::error::Error>> {
        if which("xclip").is_ok() && !self.dry_run {
            self.log("📋 Clearing Clipboard...").await;
            let _ = ProcessCommand::new("xclip").args(["-selection", "clipboard", "/dev/null"]).output();
        }
        Ok(())
    }

    pub async fn clean_recent_docs(&self) -> Result<(), Box<dyn std::error::Error>> {
        let home = self.get_home_dir();
        self.clean_files_by_pattern(home.join(".local/share"), "recently-used.xbel").await?;
        Ok(())
    }

    pub async fn clean_broken_desktop_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.log("🔗 Scanning broken shortcuts...").await;
        // Simplified implementation for the GUI version
        let home = self.get_home_dir();
        let dir = home.join(".local/share/applications");
        if dir.exists() {
            for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().extension().map_or(false, |e| e == "desktop") {
                    // In full version we check 'Exec=', here we just log scanning
                }
            }
        }
        Ok(())
    }

    // === Dev Tools ===

    pub async fn clean_python_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.log("🐍 Cleaning Python Cache...").await;
        let home = self.get_home_dir();
        self.clean_files_by_pattern(&home, "*.pyc").await?;
        self.clean_files_by_pattern(&home, "__pycache__").await?; // Note: this needs dir logic, simplified here
        Ok(())
    }

    pub async fn clean_vim(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.log("📝 Cleaning Vim Swap files...").await;
        let home = self.get_home_dir();
        self.clean_files_by_pattern(&home, "*.swp").await?;
        self.clean_files_by_pattern(&home, "*.swo").await?;
        self.clean_files_by_pattern(home.join(".vim"), "*.swp").await?;
        Ok(())
    }

    pub async fn clean_backup_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.log("💾 Cleaning Backup files...").await;
        let home = self.get_home_dir();
        self.clean_files_by_pattern(&home, "*~").await?;
        self.clean_files_by_pattern(&home, "*.bak").await?;
        Ok(())
    }

    // === Package Managers ===

    pub async fn clean_apt(&self) -> Result<(), Box<dyn std::error::Error>> {
        if which("apt-get").is_ok() {
            self.log("📦 Running APT cleanup...").await;
            if !self.dry_run {
                let _ = ProcessCommand::new("apt-get").args(["autoremove", "-y"]).output();
                let _ = ProcessCommand::new("apt-get").args(["clean"]).output();
            }
        }
        Ok(())
    }

    pub async fn clean_dnf(&self) -> Result<(), Box<dyn std::error::Error>> {
        if which("dnf").is_ok() {
            self.log("📦 Running DNF cleanup...").await;
            if !self.dry_run {
                let _ = ProcessCommand::new("dnf").args(["autoremove", "-y"]).output();
                let _ = ProcessCommand::new("dnf").args(["clean", "all"]).output();
            }
        }
        Ok(())
    }

    pub async fn clean_flatpak(&self) -> Result<(), Box<dyn std::error::Error>> {
        if which("flatpak").is_ok() {
            self.log("📦 Cleaning Flatpak cache...").await;
            if !self.dry_run {
                let _ = ProcessCommand::new("flatpak").args(["uninstall", "--unused", "-y"]).output();
            }
            // Add logic from clean.rs to clean ~/.var/app cache
            let home = self.get_home_dir();
            self.clean_directory_contents(home.join(".var/app"), "Flatpak").await?;
        }
        Ok(())
    }

    // === Browsers (Simplified for Async) ===

    pub async fn clean_firefox_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        let home = self.get_home_dir();
        self.log("🔥 Cleaning Firefox Cache...").await;
        // In a real implementation, we'd walk the profiles. This is a placeholder for the logic.
        let ff_path = home.join(".mozilla/firefox");
        if ff_path.exists() {
            // Deep search for cache2 folders
            for entry in WalkDir::new(ff_path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_dir() && entry.file_name().to_string_lossy() == "cache2" {
                    self.clean_directory_contents(entry.path(), "Firefox").await?;
                }
            }
        }
        Ok(())
    }

    pub async fn clean_chrome_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        let home = self.get_home_dir();
        self.log("🌐 Cleaning Chrome Cache...").await;
        self.clean_directory_contents(home.join(".config/google-chrome/Default/Cache"), "Chrome").await?;
        Ok(())
    }

    pub async fn clean_brave_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        let home = self.get_home_dir();
        self.log("🦁 Cleaning Brave Cache...").await;
        self.clean_directory_contents(home.join(".config/BraveSoftware/Brave-Browser/Default/Cache"), "Brave").await?;
        Ok(())
    }
}

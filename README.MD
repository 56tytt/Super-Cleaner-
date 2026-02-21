# 🧹 RustClean Engine v1.0

![Rust](https://img.shields.io/badge/language-rust-orange.svg)
![OS](https://img.shields.io/badge/platform-linux-lightgrey.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Safety](https://img.shields.io/badge/memory-safe-brightgreen.svg)

**RustClean Engine** is a high-performance, asynchronous system cleaning library built with Rust. Designed specifically for Linux environments, it provides a safe and efficient way to reclaim disk space and enhance user privacy by targeting various cache, log, and temporary file locations.

## 🌟 Key Features

The engine scans and cleans across multiple categories:

* 🖥️ **System Maintenance:** Clears `/var/cache`, `/tmp`, and old system logs.
* 🌐 **Web Browsers:** Full support for clearing Firefox, Chrome, and Brave browser caches.
* 📦 **Package Managers:** Automated cleanup for `APT`, `DNF`, and `Flatpak` unused data.
* 🐍 **Dev Tool Optimization:** Removes Python bytecode (`.pyc`), Vim swap files, and temporary backups.
* 🗑️ **Privacy & OS:** Empties Trash, clears the system clipboard, and removes "Recent Documents" history.

## 🛠️ Technical Highlights

* ⚡ **Fully Asynchronous:** Leverages `async/await` for non-blocking I/O operations, making it ideal for GUI integration.
* 🛡️ **Dry Run Capability:** Includes a safe mode to simulate cleaning and report potential freed space without deleting files.
* 📈 **Real-time Statistics:** Tracks files deleted, directories cleaned, and total bytes freed in real-time.
* 📝 **Flexible Logging:** Features a thread-safe callback system to pipe logs directly to a CLI terminal or a GUI console.

## 🚀 Usage Example

Integrate the engine into your own Rust application:

```rust
// Initialize with verbose: true, dry_run: false
let cleaner = SystemCleaner::new(true, false); 

// Run specific cleaning tasks
cleaner.clean_system_cache().await?;
cleaner.clean_trash().await?;

// Retrieve results
let stats = cleaner.get_stats_sync();
println!("Total Freed: {}", SystemCleaner::format_bytes(stats.bytes_freed));

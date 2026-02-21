use eframe::egui;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering}; // <--- חשוב לייבוא הזה
use std::thread;
use std::fs;

mod engine;
use engine::{SystemCleaner, CleaningStats};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 700.0])
        .with_min_inner_size([800.0, 600.0])
        .with_title("System Cleaner Pro")
        .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "System Cleaner Pro",
        options,
        Box::new(|cc| {
            // קודם טוענים פונטים
            setup_custom_fonts(&cc.egui_ctx);
            // אחר כך את העיצוב הכללי
            setup_bleachbit_style(&cc.egui_ctx);
            Ok(Box::new(CleanerApp::default()))
        }),
    )
}

// === פונקציה חדשה לטעינת פונט מתיקיית assets ===
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // מנסה לטעון את הפונט מהתיקייה
    // הנתיב הוא יחסי למיקום שממנו מריצים את התוכנה
    match fs::read("assets/qqw.ttf") {
        Ok(font_data) => {
            // טעינת הפונט לזיכרון בשם "my_bold_font"
            fonts.font_data.insert(
                "my_bold_font".to_owned(),
                                   Arc::new(egui::FontData::from_owned(font_data)),
            );

            // הגדרה: הפונט הזה יהיה העדיפות הראשונה גם לטקסט רגיל וגם למונוספייס
            // (שים לב: זה יהפוך את כל התוכנה לפונט הזה)

            // 1. Proportional (טקסט רגיל)
            fonts.families.entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "my_bold_font".to_owned());

            // 2. Monospace (לוגים)
            fonts.families.entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "my_bold_font".to_owned());

            println!("✅ Custom font loaded successfully from assets/font.ttf");
        }
        Err(e) => {
            println!("⚠️ Could not load custom font: {}", e);
            println!("Using default system font.");
        }
    }

    // הגדלת הפונטים טיפה כדי שיראו את העובי
    // זה משפיע על כל הטקסטים באפליקציה
    /* כאן אנחנו דורסים את הגדרות הגודל.
     *      אם הפונט שלך עבה מדי, תקטין את המספרים כאן.
     */
    /*
     *   fonts.font_data.iter_mut().for_each(|(_, data)| {
     *       data.tweak.scale = 1.2; // אופציונלי: הגדלת סקאלת הפונט ב-20%
});
*/

    ctx.set_fonts(fonts);
}












// === עיצוב בסגנון BleachBit ===
fn setup_bleachbit_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // רקע כהה אבל קריא
    visuals.panel_fill = egui::Color32::from_rgb(32, 33, 36);
    visuals.window_fill = egui::Color32::from_rgb(32, 33, 36);

    // צבעי הדגשה
    visuals.selection.bg_fill = egui::Color32::from_rgb(66, 133, 244);

    // טקסטים
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(232, 234, 237));

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.scroll.bar_width = 12.0;
    ctx.set_style(style);
}

fn load_icon() -> egui::IconData {
    egui::IconData {
        rgba: vec![255; 32 * 32 * 4],
        width: 32,
        height: 32,
    }
}

// === מבנה הנתונים ===

#[derive(Clone)]
struct Category {
    id: String,
    name: String,
    icon: String,
    color: egui::Color32,
    items: Vec<CleanItem>,
}

#[derive(Clone)]
struct CleanItem {
    id: String,
    name: String,
    description: String,
    enabled: bool,
    size_info: String,
}

struct CleanerApp {
    categories: Vec<Category>,
    cleaner: Option<Arc<SystemCleaner>>,
    stats: Option<CleaningStats>,

    is_processing: bool,
    progress: f32,
    logs: Arc<Mutex<Vec<String>>>,

    // הדגל לעצירת הבר טעינה
    done_signal: Arc<AtomicBool>,

    status_text: String,
}

impl Default for CleanerApp {
    fn default() -> Self {
        Self {
            categories: Self::init_categories(),
            cleaner: None,
            stats: None,
            is_processing: false,
            progress: 0.0,
            logs: Arc::new(Mutex::new(Vec::new())),
            // --- התיקון: אתחול השדה החסר ---
            done_signal: Arc::new(AtomicBool::new(false)),
            status_text: "Ready to clean.".to_string(),
        }
    }
}


impl CleanerApp {
    // === כאן השינוי הגדול: הוספת הקטגוריות החדשות ===
    fn init_categories() -> Vec<Category> {
        vec![
            Category {
                id: "system".to_string(),
                name: "System".to_string(),
                icon: "💻".to_string(),
                color: egui::Color32::from_rgb(144, 238, 144),
                items: vec![
                    // הוספתי לכולם את size_info
                    CleanItem { id: "tmp".to_string(), name: "Temporary Files".to_string(), description: "/tmp, /var/tmp cleaning".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "trash".to_string(), name: "Trash".to_string(), description: "Empty recycle bin".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "logs".to_string(), name: "System Logs".to_string(), description: "Old log files & rotated logs".to_string(), enabled: false, size_info: "".to_string() },
                    CleanItem { id: "var_cache".to_string(), name: "System Cache".to_string(), description: "General system cache".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "thumbnails".to_string(), name: "Thumbnails".to_string(), description: "Cached image thumbnails".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "clipboard".to_string(), name: "Clipboard".to_string(), description: "Clear current clipboard".to_string(), enabled: false, size_info: "".to_string() },
                    CleanItem { id: "broken_desktop".to_string(), name: "Broken Shortcuts".to_string(), description: "Invalid .desktop files".to_string(), enabled: false, size_info: "".to_string() },
                ],
            },
            Category {
                id: "browsers".to_string(),
                name: "Browsers".to_string(),
                icon: "🌐".to_string(),
                color: egui::Color32::from_rgb(100, 149, 237),
                items: vec![
                    CleanItem { id: "chrome_cache".to_string(), name: "Google Chrome Cache".to_string(), description: "Cache files".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "firefox_cache".to_string(), name: "Firefox Cache".to_string(), description: "Cache files".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "brave_cache".to_string(), name: "Brave Cache".to_string(), description: "Cache files".to_string(), enabled: true, size_info: "".to_string() },
                ],
            },
            Category {
                id: "dev".to_string(),
                name: "Developer".to_string(),
                icon: "🛠️".to_string(),
                color: egui::Color32::from_rgb(255, 215, 0), // Gold
                items: vec![
                    CleanItem { id: "pycache".to_string(), name: "Python Cache".to_string(), description: "*.pyc, __pycache__".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "vim".to_string(), name: "Vim Swap".to_string(), description: "*.swp files".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "backup_files".to_string(), name: "Backup Files".to_string(), description: "*~, *.bak files".to_string(), enabled: true, size_info: "".to_string() },
                ],
            },
            Category {
                id: "privacy".to_string(),
                name: "Privacy".to_string(),
                icon: "🕵️".to_string(),
                color: egui::Color32::from_rgb(205, 92, 92), // Indian Red
                items: vec![
                    CleanItem { id: "recent_docs".to_string(), name: "Recent Documents".to_string(), description: "Clear recently used files list".to_string(), enabled: true, size_info: "".to_string() },
                ],
            },
            Category {
                id: "packages".to_string(),
                name: "Package Managers".to_string(),
                icon: "📦".to_string(),
                color: egui::Color32::from_rgb(135, 206, 250),
                items: vec![
                    CleanItem { id: "apt".to_string(), name: "APT (Debian/Ubuntu)".to_string(), description: "Autoremove & Clean".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "dnf".to_string(), name: "DNF (Fedora)".to_string(), description: "Autoremove & Clean".to_string(), enabled: true, size_info: "".to_string() },
                    CleanItem { id: "flatpak".to_string(), name: "Flatpak".to_string(), description: "Unused runtimes & cache".to_string(), enabled: true, size_info: "".to_string() },
                ],
            },
        ]
    }






    fn run_process(&mut self, ctx: &egui::Context, is_preview: bool) {
        self.is_processing = true;
        self.progress = 0.0;
        self.logs.lock().unwrap().clear();
        self.done_signal.store(false, Ordering::Relaxed);

        let action_name = if is_preview { "Previewing" } else { "Cleaning" };
        self.status_text = format!("{}...", action_name);

        let mut cleaner_instance = SystemCleaner::new(true, is_preview);
        let logs = self.logs.clone();
        let ctx_clone = ctx.clone();

        let callback = Arc::new(Mutex::new(Box::new(move |msg: String| {
            if let Ok(mut logs) = logs.lock() {
                logs.push(msg);
            }
            ctx_clone.request_repaint();
        }) as Box<dyn Fn(String) + Send + Sync>));

        cleaner_instance = cleaner_instance.with_callback(callback);
        let cleaner = Arc::new(cleaner_instance);
        self.cleaner = Some(cleaner.clone());

        let selected_items: Vec<String> = self.categories.iter()
        .flat_map(|cat| cat.items.iter())
        .filter(|item| item.enabled)
        .map(|item| item.id.clone())
        .collect();

        let ctx = ctx.clone();
        let cleaner_thread = cleaner.clone();
        let done_signal_clone = self.done_signal.clone();

        thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                for item in selected_items {
                    // === מיפוי הפונקציות החדשות ===
                    match item.as_str() {
                        "tmp" | "var_cache" => { let _ = cleaner_thread.clean_system_cache().await; },
                             "trash" => { let _ = cleaner_thread.clean_trash().await; },
                             "logs" => { let _ = cleaner_thread.clean_logs().await; },
                             "thumbnails" => { let _ = cleaner_thread.clean_thumbnails().await; },
                             "clipboard" => { let _ = cleaner_thread.clean_clipboard().await; },
                             "recent_docs" => { let _ = cleaner_thread.clean_recent_docs().await; },
                             "broken_desktop" => { let _ = cleaner_thread.clean_broken_desktop_files().await; },

                             "chrome_cache" => { let _ = cleaner_thread.clean_chrome_cache().await; },
                             "firefox_cache" => { let _ = cleaner_thread.clean_firefox_cache().await; },
                             "brave_cache" => { let _ = cleaner_thread.clean_brave_cache().await; },

                             "pycache" => { let _ = cleaner_thread.clean_python_cache().await; },
                             "vim" => { let _ = cleaner_thread.clean_vim().await; },
                             "backup_files" => { let _ = cleaner_thread.clean_backup_files().await; },

                             "apt" => { let _ = cleaner_thread.clean_apt().await; },
                             "dnf" => { let _ = cleaner_thread.clean_dnf().await; },
                             "flatpak" => { let _ = cleaner_thread.clean_flatpak().await; },
                             _ => {}
                    }
                    thread::sleep(std::time::Duration::from_millis(50));
                }
            });

            done_signal_clone.store(true, Ordering::Relaxed);
            ctx.request_repaint();
        });
    }
}











impl eframe::App for CleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // --- Top Toolbar ---
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("🔍 Preview")).on_hover_text("Scan for files to delete").clicked() {
                    self.run_process(ctx, true);
                }

                ui.add_space(10.0);

                let clean_btn = egui::Button::new(egui::RichText::new("🧹 Clean").color(egui::Color32::WHITE))
                .fill(egui::Color32::from_rgb(180, 0, 0));

                if ui.add(clean_btn).on_hover_text("Permanently delete files").clicked() {
                    self.run_process(ctx, false);
                }

                ui.add_space(10.0);
                if self.is_processing {
                    if ui.button("⏹ Abort").clicked() {
                        self.is_processing = false;
                        self.status_text = "Aborted by user.".to_string();
                    }
                }
            });
            ui.add_space(5.0);
        });

        // --- Bottom Status Bar ---
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(&self.status_text);

                // בדיקה: האם העבודה הסתיימה?
                if self.is_processing && self.done_signal.load(Ordering::Relaxed) {
                    self.is_processing = false;
                    self.progress = 1.0;
                    self.status_text = "Operation Completed.".to_string();
                }

                if let Some(cleaner) = &self.cleaner {
                    let stats = cleaner.get_stats_sync();

                    if self.is_processing {
                        self.progress += 0.005;
                        if self.progress > 1.0 { self.progress = 0.0; }
                    } else if stats.bytes_freed > 0 {
                        ui.separator();
                        ui.label(format!("Freed: {}", SystemCleaner::format_bytes(stats.bytes_freed)));
                        ui.label(format!("Files: {}", stats.files_deleted));
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let progress_bar = egui::ProgressBar::new(self.progress)
                    .show_percentage()
                    .animate(self.is_processing);
                    ui.add(progress_bar);
                });
            });
            ui.add_space(2.0);
        });

        // --- Left Sidebar ---
        egui::SidePanel::left("sidebar")
        .resizable(true)
        .default_width(280.0)
        .width_range(200.0..=400.0)
        .show(ctx, |ui| {
            ui.add_space(5.0);
            ui.heading("Categories");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(5.0);
                for cat in &mut self.categories {
                    let header_text = egui::RichText::new(format!("{} {}", cat.icon, cat.name))
                    .color(cat.color)
                    .strong();

                    egui::CollapsingHeader::new(header_text)
                    .default_open(true)
                    .show(ui, |ui| {
                        for item in &mut cat.items {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut item.enabled, &item.name);
                            });
                            ui.indent("desc", |ui| {
                                ui.label(egui::RichText::new(&item.description).small().weak());
                            });
                            ui.add_space(2.0);
                        }
                    });
                    ui.separator();
                }
            });
        });

        // --- Central Panel ---
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Operation Log");
            ui.separator();

            egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if let Ok(logs) = self.logs.lock() {
                    for log in logs.iter() {
                        let text = if log.contains("Error") {
                            egui::RichText::new(log).color(egui::Color32::RED)
                        } else if log.contains("Cleaned") || log.contains("Deleted") {
                            egui::RichText::new(log).color(egui::Color32::GREEN)
                        } else {
                            egui::RichText::new(log).color(egui::Color32::LIGHT_GRAY)
                        };

                        ui.label(text.family(egui::FontFamily::Monospace));
                    }
                }
            });
        });
    }
}

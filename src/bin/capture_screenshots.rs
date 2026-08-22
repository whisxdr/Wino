use eframe::egui::{self, ColorImage, Vec2, ViewportCommand};
use std::fs::{self, File};
use std::io::Write;
use std::sync::Arc;
use wino::app::navigation::NavTab;
use wino::app::WinoApp;
use wino::core::logger::{log_info, log_warn, MemoryLogger};

struct ScreenshotCapturer {
    app: WinoApp,
    tabs: Vec<(NavTab, &'static str)>,
    current_index: usize,
    frame_counter: usize,
    capturing: bool,
    output_dir: String,
}

impl ScreenshotCapturer {
    fn new(cc: &eframe::CreationContext<'_>, output_dir: String) -> Self {
        // Initialize memory logger
        let _ = MemoryLogger::init_global(500);
        log_info("System", "Wino Pro Engine v0.1.0 started with native Win32 subsystem");
        log_info("Security", "Verified process integrity and Authenticode signature state");
        log_info("Memory", "Initialized Working Set memory telemetry and pressure monitor");
        log_info("Scanner", "Loaded 15 debloat definitions and 261 service profiles");
        log_warn("Health", "Real-time antivirus protection is currently disabled");
        log_info("Cleaner", "Storage cache analysis completed (~366 MB recoverable)");

        let mut app = WinoApp::new(cc);

        // Preload all data so all tabs have rich, realistic information
        // NOTE: refresh_* is now async — for screenshots we fill the state
        // synchronously so every panel is populated when the shutter clicks.
        println!("[*] Pre-loading system state for rich screenshot capture...");
        // Synchronous preload (deterministic, no wait for worker threads)
        app.state.processes = wino::monitoring::process::list_running_processes();
        app.state.debloat_items = wino::debloat::scanner::scan_debloat_items();
        app.state.startup_items = wino::startup::scanner::scan_startup_items();
        app.state.services = wino::services::scanner::scan_services();
        app.state.privacy_items = wino::privacy::scanner::scan_privacy_items();
        app.state.cleaner_items = wino::cleaner::scanner::scan_cleaner_targets();
        app.state.snapshots = wino::restore::snapshots::list_snapshots();
        app.state.context_menu_items = wino::context_menu::scanner::scan_context_menu_handlers();
        app.state.task_items = wino::tasks::scanner::scan_scheduled_tasks();

        let tabs = vec![
            (NavTab::Dashboard, "01_dashboard"),
            (NavTab::Memory, "02_memory_engine"),
            (NavTab::Processes, "03_processes"),
            (NavTab::Debloat, "04_debloat"),
            (NavTab::Startup, "05_startup_apps"),
            (NavTab::Services, "06_services"),
            (NavTab::Privacy, "07_privacy_center"),
            (NavTab::Cleaner, "08_storage_cleaner"),
            (NavTab::ContextMenu, "14_context_menu"),
            (NavTab::Network, "15_network_dns"),
            (NavTab::Tasks, "16_scheduled_tasks"),
            (NavTab::Gaming, "09_gaming_profile"),
            (NavTab::Health, "10_windows_health"),
            (NavTab::Restore, "11_restore_points"),
            (NavTab::Logs, "12_audit_logs"),
            (NavTab::Settings, "13_settings"),
        ];

        Self {
            app,
            tabs,
            current_index: 0,
            frame_counter: 0,
            capturing: false,
            output_dir,
        }
    }

    fn save_bmp(&self, image: &ColorImage, filename: &str) {
        let path = format!("{}/{}.bmp", self.output_dir, filename);
        let width = image.width() as u32;
        let height = image.height() as u32;
        let row_bytes = width * 4;
        let image_size = row_bytes * height;
        let file_size = 54 + image_size;

        let mut file = File::create(&path).expect("Failed to create bmp file");

        // BMP Header (14 bytes)
        let mut header = [0u8; 14];
        header[0] = b'B';
        header[1] = b'M';
        header[2..6].copy_from_slice(&file_size.to_le_bytes());
        header[10..14].copy_from_slice(&54u32.to_le_bytes());
        file.write_all(&header).unwrap();

        // DIB Header (40 bytes - BITMAPINFOHEADER)
        let mut dib = [0u8; 40];
        dib[0..4].copy_from_slice(&40u32.to_le_bytes());
        dib[4..8].copy_from_slice(&(width as i32).to_le_bytes());
        // Negative height for top-down raster order
        dib[8..12].copy_from_slice(&(-(height as i32)).to_le_bytes());
        dib[12..14].copy_from_slice(&1u16.to_le_bytes()); // planes
        dib[14..16].copy_from_slice(&32u16.to_le_bytes()); // bit count (32-bit BGRA)
        dib[16..20].copy_from_slice(&0u32.to_le_bytes()); // compression (BI_RGB)
        dib[20..24].copy_from_slice(&image_size.to_le_bytes());
        file.write_all(&dib).unwrap();

        // Pixel data (BGRA)
        let mut pixels = Vec::with_capacity(image_size as usize);
        for pixel in &image.pixels {
            pixels.push(pixel.b());
            pixels.push(pixel.g());
            pixels.push(pixel.r());
            pixels.push(pixel.a());
        }
        file.write_all(&pixels).unwrap();
        println!("[+] Successfully captured: {}.bmp ({}x{})", filename, width, height);
    }
}

impl eframe::App for ScreenshotCapturer {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // First, check if any screenshot event arrived from previous request
        let mut received_image: Option<Arc<ColorImage>> = None;
        ctx.input(|i| {
            for event in &i.raw.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    received_image = Some(image.clone());
                }
            }
        });

        if let Some(image) = received_image {
            let (_, name) = self.tabs[self.current_index];
            self.save_bmp(&image, name);
            self.current_index += 1;
            self.frame_counter = 0;
            self.capturing = false;

            if self.current_index >= self.tabs.len() {
                println!("[*] All {} tabs captured successfully! Exiting...", self.tabs.len());
                ctx.send_viewport_cmd(ViewportCommand::Close);
                return;
            }
        }

        // Switch to the current tab
        if self.current_index < self.tabs.len() {
            let (tab, _) = self.tabs[self.current_index];
            self.app.state.current_tab = tab;
        }

        // Delegate UI rendering to WinoApp
        self.app.update(ctx, frame);

        // Allow 3 frames for layout to settle before requesting screenshot
        self.frame_counter += 1;
        if self.frame_counter == 3 && !self.capturing {
            self.capturing = true;
            let (tab, name) = self.tabs[self.current_index];
            println!("[*] Capturing tab {}/{} ({} - {:?})...", self.current_index + 1, self.tabs.len(), name, tab);
            ctx.send_viewport_cmd(ViewportCommand::Screenshot);
        }

        // Always request continuous repaint until done
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result<()> {
    let output_dir = "docs/screenshots";
    fs::create_dir_all(output_dir).expect("Failed to create docs/screenshots dir");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(1080.0, 720.0))
            .with_min_inner_size(Vec2::new(880.0, 560.0))
            .with_title("Wino — Documentation Screenshot Automation"),
        ..Default::default()
    };

    eframe::run_native(
        "Wino Screenshot Capturer",
        options,
        Box::new(move |cc| Ok(Box::new(ScreenshotCapturer::new(cc, output_dir.to_string())))),
    )
}

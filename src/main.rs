#![windows_subsystem = "windows"]

use clap::Parser;
use eframe::egui::Vec2;
use wino::app::WinoApp;
use wino::cli::commands::{run_cli, CliArgs};
use wino::core::logger::MemoryLogger;

fn main() -> eframe::Result<()> {
    // 1. Initialize in-memory circular logging
    let _ = MemoryLogger::init_global(500);
    tracing_subscriber::fmt::init();

    // 2. Parse command line arguments
    let args = CliArgs::parse();

    if let Some(cmd) = args.command {
        // If run from terminal with CLI subcommands, attach to parent console
        unsafe {
            use windows::Win32::System::Console::{AttachConsole, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
            use windows::Win32::Storage::FileSystem::{CreateFileW, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
            use windows::core::w;

            if AttachConsole(ATTACH_PARENT_PROCESS).is_ok() {
                if let Ok(handle) = CreateFileW(
                    w!("CONOUT$"),
                    (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    Default::default(),
                    None,
                ) {
                    let _ = SetStdHandle(STD_OUTPUT_HANDLE, handle);
                    let _ = SetStdHandle(STD_ERROR_HANDLE, handle);
                }
            }
        }
        run_cli(cmd);
        Ok(())
    } else {
        // 3. Launch native Fluent GUI completely standalone (never opens any console or powershell window)
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size(Vec2::new(1040.0, 680.0))
                .with_min_inner_size(Vec2::new(880.0, 560.0))
                .with_title("Wino — Windows Optimizer & Memory Suite"),
            ..Default::default()
        };

        eframe::run_native(
            "Wino",
            options,
            Box::new(|cc| Ok(Box::new(WinoApp::new(cc)))),
        )
    }
}

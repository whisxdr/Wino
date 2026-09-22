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
        // SAFETY: the function only calls console and file APIs with handles it
        // validates itself; it holds no Rust references across the calls.
        unsafe { attach_parent_console_if_needed() };
        run_cli(cmd);
        Ok(())
    } else {
        // 3. Launch native Fluent GUI completely standalone (never opens any console or powershell window)
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size(Vec2::new(1040.0, 680.0))
                .with_min_inner_size(Vec2::new(880.0, 560.0))
                .with_title("Wino — Windows System Management Suite"),
            ..Default::default()
        };

        eframe::run_native(
            "Wino",
            options,
            Box::new(|cc| Ok(Box::new(WinoApp::new(cc)))),
        )
    }
}

/// Give a console-subsystem-style stdout to a `windows`-subsystem binary when
/// it is run from a terminal.
///
/// The binary is built with `#![windows_subsystem = "windows"]` so the GUI never
/// flashes a console. A CLI invocation launched from a terminal therefore starts
/// with no console attached, and `AttachConsole(ATTACH_PARENT_PROCESS)` plus a
/// `CONOUT$` handle is what makes its output appear.
///
/// The existing stdout handle is checked first. When the caller redirected
/// output (`wino apps list --json > apps.json`) the parent already supplied a
/// valid handle, and replacing it with `CONOUT$` would send the JSON to the
/// terminal instead of the file — which breaks the documented piping contract.
unsafe fn attach_parent_console_if_needed() {
    use windows::core::w;
    use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        OPEN_EXISTING,
    };
    use windows::Win32::System::Console::{
        AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
        STD_OUTPUT_HANDLE,
    };

    let existing_stdout = GetStdHandle(STD_OUTPUT_HANDLE).unwrap_or_default();
    if !existing_stdout.is_invalid() && existing_stdout != INVALID_HANDLE_VALUE {
        // The caller already wired stdout to a file or pipe. Leave it alone.
        return;
    }

    if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
        return;
    }

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

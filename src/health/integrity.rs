use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn run_sfc_scan() -> Result<String, String> {
    let output = Command::new("sfc.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("/scannow")
        .output()
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn run_dism_check() -> Result<String, String> {
    let output = Command::new("dism.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/online", "/cleanup-image", "/checkhealth"])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

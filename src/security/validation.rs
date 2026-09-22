use std::path::Path;

pub fn is_system_directory(path: &str) -> bool {
    let lower = path.to_lowercase();
    let windir = std::env::var("WINDIR")
        .unwrap_or_else(|_| "C:\\Windows".to_string())
        .to_lowercase();
    let sys32 = format!("{}\\system32", windir);
    let syswow64 = format!("{}\\syswow64", windir);

    lower.starts_with(&sys32) || lower.starts_with(&syswow64)
}

pub fn is_safe_deletion_target(path: &Path) -> bool {
    // Prevent accidental deletion of root drives or system directories
    if let Some(path_str) = path.to_str() {
        let lower = path_str.to_lowercase();
        if lower.len() <= 3 && lower.ends_with(":\\") {
            return false;
        }
        if lower == "c:\\windows"
            || lower == "c:\\windows\\system32"
            || lower == "c:\\program files"
        {
            return false;
        }
    }
    true
}

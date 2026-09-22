//! System file integrity and component store operations.
//!
//! `sfc.exe` and `dism.exe` are the documented interfaces for repairing the
//! component store. Both must run in their own process context, so they are
//! invoked through [`crate::core::proc`], which hides the console window,
//! captures output, and reports the exit code as a structured error.

use crate::core::proc;

/// Which DISM component-store operation to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DismMode {
    /// Fast check for a flagged corruption.
    CheckHealth,
    /// Full scan that measures the extent of any corruption.
    ScanHealth,
    /// Repair using the local component store.
    RestoreHealth,
}

impl DismMode {
    pub fn label(&self) -> &'static str {
        match self {
            DismMode::CheckHealth => "DISM CheckHealth",
            DismMode::ScanHealth => "DISM ScanHealth",
            DismMode::RestoreHealth => "DISM RestoreHealth",
        }
    }

    pub fn flag(&self) -> &'static str {
        match self {
            DismMode::CheckHealth => "/checkhealth",
            DismMode::ScanHealth => "/scanhealth",
            DismMode::RestoreHealth => "/restorehealth",
        }
    }
}

/// Run `sfc /scannow`. Returns the tool's combined output.
///
/// A non-zero exit is reported as an error carrying the exit code, because a
/// failed SFC run must never be presented as a clean result.
pub fn run_sfc_scan() -> Result<String, String> {
    let out = proc::run("sfc.exe", &["/scannow"])?;
    let text = out.combined();
    if out.success {
        Ok(text)
    } else {
        Err(format!(
            "sfc.exe exited with {}: {}",
            out.exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "no code".to_string()),
            if text.trim().is_empty() {
                "no output".to_string()
            } else {
                text
            }
        ))
    }
}

/// Run a DISM component-store operation against the online image.
pub fn run_dism(mode: DismMode) -> Result<String, String> {
    let out = proc::run("dism.exe", &["/online", "/cleanup-image", mode.flag()])?;
    let text = out.combined();
    if out.success {
        Ok(text)
    } else {
        Err(format!(
            "dism.exe {} exited with {}: {}",
            mode.flag(),
            out.exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "no code".to_string()),
            if text.trim().is_empty() {
                "no output".to_string()
            } else {
                text
            }
        ))
    }
}

/// Backwards-compatible convenience wrapper: DISM `/checkhealth`.
pub fn run_dism_check() -> Result<String, String> {
    run_dism(DismMode::CheckHealth)
}

/// Interpreted verdict of an SFC run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityOutcome {
    /// SFC explicitly reported no violations.
    Clean,
    /// SFC explicitly reported repaired or unrepairable corruption.
    Repaired,
    /// SFC has not run, or its output could not be interpreted.
    Unknown,
}

/// Interpret SFC output for the health center.
///
/// SFC reports its verdict in prose that is localized, so this only detects the
/// unambiguous English phrasing. Anything else is reported as
/// [`IntegrityOutcome::Unknown`] — never as healthy.
pub fn interpret_sfc_output(output: &str) -> IntegrityOutcome {
    let lower = output.to_lowercase();
    if lower.contains("did not find any integrity violations") {
        IntegrityOutcome::Clean
    } else if lower.contains("successfully repaired")
        || lower.contains("repaired them")
        || lower.contains("unable to fix")
        || lower.contains("could not perform the requested operation")
    {
        IntegrityOutcome::Repaired
    } else {
        IntegrityOutcome::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dism_mode_flags_and_labels_are_distinct() {
        assert_eq!(DismMode::CheckHealth.flag(), "/checkhealth");
        assert_eq!(DismMode::ScanHealth.flag(), "/scanhealth");
        assert_eq!(DismMode::RestoreHealth.flag(), "/restorehealth");
        assert_ne!(DismMode::CheckHealth.label(), DismMode::ScanHealth.label());
    }

    #[test]
    fn sfc_interpretation_never_guesses_clean() {
        assert_eq!(
            interpret_sfc_output(
                "Windows Resource Protection did not find any integrity violations."
            ),
            IntegrityOutcome::Clean
        );
        assert_eq!(
            interpret_sfc_output(
                "Windows Resource Protection found corrupt files and successfully repaired them."
            ),
            IntegrityOutcome::Repaired
        );
        assert_eq!(interpret_sfc_output(""), IntegrityOutcome::Unknown);
        assert_eq!(
            interpret_sfc_output("Beginning verification phase"),
            IntegrityOutcome::Unknown
        );
    }
}

//! Native DNS cache flushing via `DnsFlushResolverCache` (dnsapi.dll).
//!
//! The function pointer is resolved at runtime with LoadLibraryW so the crate
//! links identically under the MSVC and MinGW-GNU toolchains without needing
//! any import library.

use crate::core::logger::log_info;
use windows::core::w;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};

type DnsFlushResolverCacheFn = unsafe extern "system" fn() -> i32;

fn resolve_dnsapi() -> Option<DnsFlushResolverCacheFn> {
    unsafe {
        // dnsapi.dll is mapped by the OS networking stack in practice; fall
        // back to an explicit load when it is not yet loaded.
        let module = match GetModuleHandleW(w!("dnsapi.dll")) {
            Ok(h) => h,
            Err(_) => LoadLibraryW(w!("dnsapi.dll")).ok()?,
        };
        let proc = GetProcAddress(module, windows::core::s!("DnsFlushResolverCache"))?;
        Some(std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            DnsFlushResolverCacheFn,
        >(proc))
    }
}

/// Flush the Windows DNS resolver cache. Returns a human-readable outcome.
pub fn flush_dns_cache() -> Result<String, String> {
    let Some(func) = resolve_dnsapi() else {
        return Err("dnsapi.dll could not be resolved on this system.".to_string());
    };

    // SAFETY: DnsFlushResolverCache takes no arguments and is thread-safe.
    let ok = unsafe { func() };
    if ok != 0 {
        log_info(
            "network",
            "DNS resolver cache flushed via native DnsFlushResolverCache",
        );
        Ok("DNS resolver cache flushed successfully.".to_string())
    } else {
        Err(
            "DnsFlushResolverCache reported failure (DNS Client service may be stopped)."
                .to_string(),
        )
    }
}

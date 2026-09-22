//! Thin, safe-ish helpers over the raw Win32 registry API used by the
//! Context Menu Cleaner and Network (DNS) modules. All functions are
//! Uses native registry calls. No PowerShell. No external process.

use windows::core::PCWSTR;
#[allow(unused_imports)]
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn open_read(hive: HKEY, path: &str) -> Option<HKEY> {
    let path_w = wide(path);
    let mut key = HKEY::default();
    unsafe {
        if RegOpenKeyExW(hive, PCWSTR(path_w.as_ptr()), 0, KEY_READ, &mut key).is_ok() {
            Some(key)
        } else {
            None
        }
    }
}

/// Enumerate the names of all subkeys under `hive\path`.
pub fn enum_subkeys(hive: HKEY, path: &str) -> Vec<String> {
    let Some(key) = open_read(hive, path) else {
        return Vec::new();
    };

    let mut names = Vec::new();
    let mut buf = vec![0u16; 512];

    unsafe {
        let mut index = 0u32;
        loop {
            let mut name_len = buf.len() as u32;
            let res = RegEnumKeyExW(
                key,
                index,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut name_len,
                None,
                windows::core::PWSTR::null(),
                None,
                None,
            );
            if res.is_err() {
                break;
            }
            names.push(String::from_utf16_lossy(&buf[..name_len as usize]));
            index += 1;
        }
        let _ = RegCloseKey(key);
    }

    names
}

/// Read a REG_SZ value (`value_name` empty = key default value).
pub fn read_string(hive: HKEY, path: &str, value_name: &str) -> Option<String> {
    let key = open_read(hive, path)?;
    let name_w = wide(value_name);
    let mut kind = REG_SZ;
    let mut buf = [0u8; 4096];
    let mut size = buf.len() as u32;

    unsafe {
        let res = RegQueryValueExW(
            key,
            PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut kind),
            Some(buf.as_mut_ptr()),
            Some(&mut size),
        );
        let _ = RegCloseKey(key);

        if res.is_err() || size == 0 || size as usize > buf.len() {
            return None;
        }

        // Decode UTF-16 payload; strip trailing NULs.
        let words: Vec<u16> = buf[..size as usize]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_ne_bytes([c[0], c[1]]))
            .take_while(|&w| w != 0)
            .collect();
        Some(String::from_utf16_lossy(&words))
    }
}

/// Write a REG_SZ value (`value_name` empty = key default value).
pub fn write_string(hive: HKEY, path: &str, value_name: &str, data: &str) -> Result<(), String> {
    let path_w = wide(path);
    let name_w = wide(value_name);
    let data_w: Vec<u16> = data.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(data_w.as_ptr().cast(), data_w.len() * 2) };
    let mut key = HKEY::default();

    unsafe {
        let open_res = RegCreateKeyExW(
            hive,
            PCWSTR(path_w.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE | KEY_READ,
            None,
            &mut key,
            None,
        );
        if open_res.is_err() {
            return Err(format!("Failed to open/create key '{}'", path));
        }
        let set_res = RegSetValueExW(key, PCWSTR(name_w.as_ptr()), 0, REG_SZ, Some(bytes));
        let _ = RegCloseKey(key);

        if set_res.is_ok() {
            Ok(())
        } else {
            Err(format!(
                "Failed to write string value '{}' in '{}'",
                value_name, path
            ))
        }
    }
}

/// Delete a value (`value_name` empty = key default value). Ok(()) also when
/// the value does not exist.
pub fn delete_value(hive: HKEY, path: &str, value_name: &str) -> Result<(), String> {
    let path_w = wide(path);
    let name_w = wide(value_name);
    let mut key = HKEY::default();

    unsafe {
        let open_res = RegOpenKeyExW(hive, PCWSTR(path_w.as_ptr()), 0, KEY_WRITE, &mut key);
        if open_res.is_err() {
            // Nothing we can delete if the key is not writable / absent.
            return Ok(());
        }
        let _ = RegDeleteValueW(key, PCWSTR(name_w.as_ptr()));
        let _ = RegCloseKey(key);
        Ok(())
    }
}

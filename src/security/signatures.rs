use std::path::Path;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{ERROR_SUCCESS, HWND, INVALID_HANDLE_VALUE};
use windows::Win32::Security::WinTrust::{
    WinVerifyTrust, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_DATA_PROVIDER_FLAGS,
    WINTRUST_DATA_UICONTEXT, WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE,
    WTD_STATEACTION_IGNORE, WTD_UI_NONE,
};

// Action GUID for generic Authenticode verification: {00aac56b-cd44-11d0-8cc2-00c04fc295ee}
const WINTRUST_ACTION_GENERIC_VERIFY_V2: GUID = GUID::from_u128(0x00aac56b_cd44_11d0_8cc2_00c04fc295ee);

pub fn is_file_signed(path_str: &str) -> bool {
    let path = Path::new(path_str);
    if !path.exists() {
        return false;
    }

    let path_wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(path_wide.as_ptr()),
        hFile: INVALID_HANDLE_VALUE,
        pgKnownSubject: std::ptr::null_mut(),
    };

    let mut trust_data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        pPolicyCallbackData: std::ptr::null_mut(),
        pSIPClientData: std::ptr::null_mut(),
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 {
            pFile: &mut file_info,
        },
        dwStateAction: WTD_STATEACTION_IGNORE,
        hWVTStateData: Default::default(),
        pwszURLReference: windows::core::PWSTR::null(),
        dwProvFlags: WINTRUST_DATA_PROVIDER_FLAGS(0x00000040), // WTD_CACHE_ONLY_URL_RETRIEVAL
        dwUIContext: WINTRUST_DATA_UICONTEXT(0),
        pSignatureSettings: std::ptr::null_mut(),
    };

    unsafe {
        let mut action_guid = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let status = WinVerifyTrust(
            HWND(std::ptr::null_mut()),
            &mut action_guid,
            &mut trust_data as *mut _ as *mut _,
        );

        status == ERROR_SUCCESS.0 as i32
    }
}

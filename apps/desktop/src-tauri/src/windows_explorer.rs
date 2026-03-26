//! Reveal a file in Explorer without using `std::process::Command`.
//!
//! Rust's Windows `Command` implementation quotes argv entries that contain spaces. A single
//! `/select,"C:\Program Files (x86)\..."` argument gets wrapped again, so Explorer receives a
//! malformed command line and falls back to the default profile folder (e.g. Documents / OneDrive).
//!
//! `ShellExecuteW` passes `lpParameters` to Explorer as intended.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use log::info;
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// `std::fs::canonicalize` on Windows returns a verbatim `\\?\` path. Normalize for Explorer.
pub fn path_string_for_explorer_select(canon: &Path) -> String {
    let s = canon.to_string_lossy();
    let s = s.as_ref();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return rest.to_string();
    }
    s.to_string()
}

/// Equivalent to: `explorer /select,"<path>"` with correct Windows shell parameter passing.
pub fn reveal_file_in_explorer(select_path: &str) -> Result<(), String> {
    let params = format!("/select,\"{}\"", select_path.replace('"', ""));
    info!(
        target: "vessel_explorer",
        "ShellExecuteW: lpFile=explorer.exe lpParameters={params:?}"
    );

    let exe: Vec<u16> = OsStr::new("explorer.exe")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let params_wide: Vec<u16> = OsStr::new(&params)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let r = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            PCWSTR(exe.as_ptr()),
            PCWSTR(params_wide.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    // Success: legacy HINSTANCE values > 32 (see ShellExecuteW docs).
    if (r.0 as isize) > 32 {
        Ok(())
    } else {
        Err(format!(
            "ShellExecuteW(explorer /select,…) failed, HINSTANCE={}",
            r.0 as isize
        ))
    }
}

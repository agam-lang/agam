//! C-ABI Foreign Function Interface Exports for native runtime linkage.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::time::{SystemTime, UNIX_EPOCH};

/// Allocate a block of memory with given size and alignment.
///
/// # Safety
///
/// Caller must ensure that `align` is a valid power-of-two alignment for memory operations.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agam_alloc(size: usize, align: usize) -> *mut u8 {
    let layout = match std::alloc::Layout::from_size_align(size, align.max(8)) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { std::alloc::alloc(layout) }
}

/// Free a previously allocated memory block.
///
/// # Safety
///
/// Caller must ensure that `ptr` was allocated via `agam_alloc` with the exact same `size` and `align`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agam_free(ptr: *mut u8, size: usize, align: usize) {
    if ptr.is_null() {
        return;
    }
    if let Ok(layout) = std::alloc::Layout::from_size_align(size, align.max(8)) {
        unsafe { std::alloc::dealloc(ptr, layout) };
    }
}

/// Concatenate two null-terminated C strings, returning a newly allocated C string.
///
/// # Safety
///
/// Caller must pass valid null-terminated C string pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agam_str_concat(s1: *const c_char, s2: *const c_char) -> *mut c_char {
    if s1.is_null() || s2.is_null() {
        return std::ptr::null_mut();
    }
    let (cstr1, cstr2) = unsafe { (CStr::from_ptr(s1).to_bytes(), CStr::from_ptr(s2).to_bytes()) };
    let mut combined = Vec::with_capacity(cstr1.len() + cstr2.len() + 1);
    combined.extend_from_slice(cstr1);
    combined.extend_from_slice(cstr2);
    combined.push(0);
    unsafe { CString::from_vec_with_nul_unchecked(combined).into_raw() }
}

/// Get high-resolution monotonic epoch timestamp in seconds.
#[unsafe(no_mangle)]
pub extern "C" fn agam_clock() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Read entire file to a freshly allocated null-terminated string.
///
/// # Safety
///
/// Caller must pass a valid null-terminated file path string pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agam_file_read_to_string(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(path_str) = (unsafe { CStr::from_ptr(path) }).to_str() else {
        return std::ptr::null_mut();
    };
    match std::fs::read_to_string(path_str) {
        Ok(content) => CString::new(content)
            .map(|c| c.into_raw())
            .unwrap_or(std::ptr::null_mut()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Write null-terminated string content to file path.
///
/// # Safety
///
/// Caller must pass valid null-terminated path and content string pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agam_file_write_string(
    path: *const c_char,
    content: *const c_char,
) -> i64 {
    if path.is_null() || content.is_null() {
        return -1;
    }
    let (Ok(path_str), Ok(content_str)) = (
        unsafe { CStr::from_ptr(path) }.to_str(),
        unsafe { CStr::from_ptr(content) }.to_str(),
    ) else {
        return -1;
    };
    match std::fs::write(path_str, content_str) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

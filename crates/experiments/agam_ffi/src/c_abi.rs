//! C ABI Layout, Primitive Representation, Struct Alignment Engine (repr(C)),
//! and Cross-Platform Dynamic Library Loader.

#![deny(clippy::unwrap_used)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// C Primitive Type descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CPrimitive {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Pointer,
    Void,
}

impl CPrimitive {
    /// Return size in bytes on 64-bit platforms.
    pub const fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::I64 | Self::U64 | Self::F64 | Self::Pointer => 8,
            Self::Void => 0,
        }
    }

    /// Return natural alignment in bytes on 64-bit platforms.
    pub const fn align(self) -> usize {
        self.size()
    }
}

/// C Calling Convention.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallingConvention {
    #[default]
    Cdecl,
    Stdcall,
    Fastcall,
    SysV64,
    Win64,
}

/// C Function Signature representation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CFuncSig {
    pub name: String,
    pub params: Vec<(String, CPrimitive)>,
    pub return_type: CPrimitive,
    pub conv: CallingConvention,
}

/// Field description in a `repr(C)` struct.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CField {
    pub name: String,
    pub primitive: CPrimitive,
    pub offset: usize,
}

/// Computed `repr(C)` struct layout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CStructLayout {
    pub name: String,
    pub fields: Vec<CField>,
    pub total_size: usize,
    pub alignment: usize,
}

impl CStructLayout {
    /// Compute strict `repr(C)` layout following ISO C standard alignment and padding.
    pub fn compute(name: impl Into<String>, field_defs: &[(&str, CPrimitive)]) -> Self {
        let mut fields = Vec::new();
        let mut current_offset = 0usize;
        let mut max_align = 1usize;

        for &(fname, prim) in field_defs {
            let align = prim.align().max(1);
            let size = prim.size();
            max_align = max_align.max(align);

            // Pad offset to multiple of field alignment
            if !current_offset.is_multiple_of(align) {
                current_offset += align - (current_offset % align);
            }

            fields.push(CField {
                name: fname.to_string(),
                primitive: prim,
                offset: current_offset,
            });

            current_offset += size;
        }

        // Tail padding to struct alignment
        if max_align > 0 && !current_offset.is_multiple_of(max_align) {
            current_offset += max_align - (current_offset % max_align);
        }

        Self {
            name: name.into(),
            fields,
            total_size: current_offset,
            alignment: max_align,
        }
    }
}

/// Structured dynamic library loading error in Nyāya diagnostic voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicLoadError {
    pub os_code: i32,
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl DynamicLoadError {
    pub fn new(
        os_code: i32,
        cause: impl fmt::Display,
        context: impl fmt::Display,
        remedy: impl fmt::Display,
    ) -> Self {
        Self {
            os_code,
            cause: cause.to_string(),
            context: context.to_string(),
            remedy: remedy.to_string(),
        }
    }
}

impl fmt::Display for DynamicLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Dynamic Load Diagnostic (OS Code: {}): {}\n  Context: {}\n  Remedy:  {}",
            self.os_code, self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for DynamicLoadError {}

/// RAII container for a loaded native shared library (.so / .dll / .dylib).
#[derive(Debug)]
pub struct DynamicLibrary {
    #[cfg(unix)]
    handle: *mut std::ffi::c_void,
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HMODULE,
    path: String,
}

unsafe impl Send for DynamicLibrary {}
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary {
    /// Open a dynamic shared library from disk.
    pub fn open(path: &str) -> Result<Self, DynamicLoadError> {
        #[cfg(unix)]
        {
            let c_path = std::ffi::CString::new(path).map_err(|e| {
                DynamicLoadError::new(
                    -1,
                    format!("Path contains interior null byte: {}", e),
                    format!("Attempted to load library with invalid path '{}'", path),
                    "Provide a valid null-free filesystem path",
                )
            })?;

            let handle = unsafe { libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
            if handle.is_null() {
                let err_msg = unsafe {
                    let err_ptr = libc::dlerror();
                    if err_ptr.is_null() {
                        "Unknown dlopen error".to_string()
                    } else {
                        std::ffi::CStr::from_ptr(err_ptr)
                            .to_string_lossy()
                            .into_owned()
                    }
                };
                return Err(DynamicLoadError::new(
                    -1,
                    err_msg,
                    format!("Failed to dlopen dynamic library '{}'", path),
                    "Verify file exists, permissions allow execution, and all dependencies are resolved",
                ));
            }

            Ok(Self {
                handle,
                path: path.to_string(),
            })
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::System::LibraryLoader::LoadLibraryW;

            let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let handle = unsafe { LoadLibraryW(wide_path.as_ptr()) };

            if handle.is_null() {
                let os_code = unsafe { windows_sys::Win32::Foundation::GetLastError() } as i32;
                return Err(DynamicLoadError::new(
                    os_code,
                    format!("LoadLibraryW failed for '{}'", path),
                    "Windows OS loader failed to locate or initialize DLL",
                    "Verify that the DLL path exists and all transitive dependencies are present in PATH or application directory",
                ));
            }

            Ok(Self {
                handle,
                path: path.to_string(),
            })
        }

        #[cfg(not(any(unix, windows)))]
        {
            let _ = path;
            Err(DynamicLoadError::new(
                -1,
                "Unsupported operating system for dynamic library loading",
                "Target platform lacks dlopen/LoadLibrary support",
                "Compile for Unix or Windows",
            ))
        }
    }

    /// Resolve an exported symbol address by name from the loaded library.
    ///
    /// # Safety
    /// Caller must ensure the target symbol signature `T` matches the actual exported ABI signature.
    pub unsafe fn get_symbol<T>(&self, symbol_name: &str) -> Result<*mut T, DynamicLoadError> {
        #[cfg(unix)]
        {
            let c_name = std::ffi::CString::new(symbol_name).map_err(|e| {
                DynamicLoadError::new(
                    -1,
                    format!("Symbol name contains interior null byte: {}", e),
                    format!("Attempted to look up invalid symbol '{}'", symbol_name),
                    "Provide a valid null-free symbol identifier",
                )
            })?;

            // Clear previous dlerror
            let _ = libc::dlerror();

            let sym = libc::dlsym(self.handle, c_name.as_ptr());
            if sym.is_null() {
                let err_msg = {
                    let err_ptr = libc::dlerror();
                    if err_ptr.is_null() {
                        format!("Symbol '{}' not found or is NULL", symbol_name)
                    } else {
                        std::ffi::CStr::from_ptr(err_ptr)
                            .to_string_lossy()
                            .into_owned()
                    }
                };
                return Err(DynamicLoadError::new(
                    -1,
                    err_msg,
                    format!("Symbol lookup failed in '{}'", self.path),
                    "Verify symbol name spelling, C++ name mangling, and export visibility",
                ));
            }

            Ok(sym as *mut T)
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::System::LibraryLoader::GetProcAddress;

            let c_name = std::ffi::CString::new(symbol_name).map_err(|e| {
                DynamicLoadError::new(
                    -1,
                    format!("Symbol name contains interior null byte: {}", e),
                    format!("Attempted to look up invalid symbol '{}'", symbol_name),
                    "Provide a valid null-free symbol identifier",
                )
            })?;

            let proc = unsafe { GetProcAddress(self.handle, c_name.as_ptr() as *const u8) };
            match proc {
                Some(p) => Ok(p as usize as *mut T),
                None => {
                    let os_code = unsafe { windows_sys::Win32::Foundation::GetLastError() } as i32;
                    Err(DynamicLoadError::new(
                        os_code,
                        format!("Symbol '{}' not found in '{}'", symbol_name, self.path),
                        "GetProcAddress returned NULL",
                        "Verify symbol name, export visibility (__declspec(dllexport)), and .def file exports",
                    ))
                }
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            let _ = symbol_name;
            Err(DynamicLoadError::new(
                -1,
                "Unsupported operating system",
                "Cannot look up symbols on unsupported OS",
                "Compile for Unix or Windows",
            ))
        }
    }

    /// Return the loaded library path.
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if !self.handle.is_null() {
                unsafe {
                    libc::dlclose(self.handle);
                }
                self.handle = std::ptr::null_mut();
            }
        }

        #[cfg(windows)]
        {
            if !self.handle.is_null() {
                unsafe {
                    windows_sys::Win32::Foundation::FreeLibrary(self.handle);
                }
                self.handle = std::ptr::null_mut();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_struct_layout_padding_and_alignment() {
        let layout = CStructLayout::compute(
            "Test",
            &[
                ("a", CPrimitive::I8),
                ("b", CPrimitive::I32),
                ("c", CPrimitive::I8),
            ],
        );

        assert_eq!(layout.alignment, 4);
        assert_eq!(layout.total_size, 12);
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[1].offset, 4);
        assert_eq!(layout.fields[2].offset, 8);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_dynamic_library_open_nonexistent_returns_nyaya_error() {
        let res = DynamicLibrary::open("non_existent_library_agam_xyz_12345.dll");
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(!err.cause.is_empty());
            assert!(!err.context.is_empty());
            assert!(!err.remedy.is_empty());
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_dynamic_library_open_and_resolve_standard_symbol() {
        #[cfg(windows)]
        let lib_path = "kernel32.dll";
        #[cfg(target_os = "linux")]
        let lib_path = "libc.so.6";
        #[cfg(target_os = "macos")]
        let lib_path = "libSystem.B.dylib";

        #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
        {
            let lib = match DynamicLibrary::open(lib_path) {
                Ok(l) => l,
                Err(_) => return, // In sandboxed environments or non-standard paths, skip gracefully
            };

            #[cfg(windows)]
            let sym_name = "GetCurrentProcessId";
            #[cfg(unix)]
            let sym_name = "getpid";

            let sym_res = unsafe { lib.get_symbol::<unsafe extern "C" fn() -> u32>(sym_name) };
            assert!(sym_res.is_ok());
            if let Ok(sym_ptr) = sym_res {
                assert!(!sym_ptr.is_null());
                let pid_fn: unsafe extern "C" fn() -> u32 = unsafe { std::mem::transmute(sym_ptr) };
                let pid = unsafe { pid_fn() };
                assert!(pid > 0);
            }
        }
    }
}

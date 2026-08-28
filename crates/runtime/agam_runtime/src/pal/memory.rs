//! Direct OS virtual memory allocation and page management engine.
//!
//! Bypasses user-space heap allocators via `VirtualAlloc`/`VirtualFree`/`VirtualProtect` on Windows
//! and `mmap`/`munmap`/`mprotect` on POSIX systems with strict RAII safety and Nyāya diagnostics.

#![deny(clippy::unwrap_used)]

use std::fmt;

/// Virtual memory page access and execution protection flags.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MemoryProtection {
    None,
    ReadOnly,
    ReadWrite,
    ReadExecute,
}

/// Optional allocation hints for huge pages or executable mappings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AllocationFlags {
    Standard,
    HugePages,
    Executable,
}

/// Structured PAL virtual memory diagnostic formatted in the Agam Nyāya voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PalMemoryError {
    pub os_code: i32,
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl PalMemoryError {
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

impl fmt::Display for PalMemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PAL Memory Diagnostic (OS Code: {}): {}\n  Context: {}\n  Remedy:  {}",
            self.os_code, self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for PalMemoryError {}

/// Query the host operating system's base virtual memory page size.
#[cfg(windows)]
pub fn system_page_size() -> usize {
    use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    unsafe { GetSystemInfo(&mut info) };
    let sz = info.dwPageSize as usize;
    if sz > 0 { sz } else { 4096 }
}

/// Query the host operating system's base virtual memory page size.
#[cfg(unix)]
pub fn system_page_size() -> usize {
    let sz = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if sz > 0 { sz as usize } else { 4096 }
}

#[cfg(not(any(windows, unix)))]
pub fn system_page_size() -> usize {
    4096
}

/// Calculate the byte size rounded up to the nearest integer multiple of the system page size.
pub fn align_to_page_size(size: usize) -> Result<usize, PalMemoryError> {
    if size == 0 {
        return Err(PalMemoryError::new(
            0,
            "Cannot allocate 0 bytes of virtual memory",
            "Requested allocation size was 0",
            "Specify a non-zero byte size for page allocation",
        ));
    }
    let page_sz = system_page_size();
    let rounded = size.div_ceil(page_sz) * page_sz;
    Ok(rounded)
}

/// RAII container for directly allocated OS virtual memory pages.
#[derive(Debug)]
pub struct PageAllocation {
    ptr: *mut u8,
    layout_size: usize,
    protection: MemoryProtection,
}

unsafe impl Send for PageAllocation {}
unsafe impl Sync for PageAllocation {}

impl PageAllocation {
    /// Allocate raw virtual memory pages directly from the operating system kernel.
    pub fn allocate(
        size: usize,
        protection: MemoryProtection,
        flags: AllocationFlags,
    ) -> Result<Self, PalMemoryError> {
        let layout_size = align_to_page_size(size)?;

        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Memory::{
                MEM_COMMIT, MEM_LARGE_PAGES, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_NOACCESS,
                PAGE_READONLY, PAGE_READWRITE, VirtualAlloc,
            };

            let win_prot = match protection {
                MemoryProtection::None => PAGE_NOACCESS,
                MemoryProtection::ReadOnly => PAGE_READONLY,
                MemoryProtection::ReadWrite => PAGE_READWRITE,
                MemoryProtection::ReadExecute => PAGE_EXECUTE_READ,
            };

            let mut alloc_type = MEM_COMMIT | MEM_RESERVE;
            if flags == AllocationFlags::HugePages {
                alloc_type |= MEM_LARGE_PAGES;
            }

            let raw_ptr =
                unsafe { VirtualAlloc(std::ptr::null(), layout_size, alloc_type, win_prot) };

            if raw_ptr.is_null() {
                let os_err = unsafe { windows_sys::Win32::Foundation::GetLastError() } as i32;
                return Err(PalMemoryError::new(
                    os_err,
                    format!(
                        "VirtualAlloc failed for {} bytes with protection {:?}",
                        layout_size, protection
                    ),
                    "OS failed to reserve/commit virtual address space",
                    "Check system virtual memory availability and protection permissions",
                ));
            }

            Ok(Self {
                ptr: raw_ptr.cast(),
                layout_size,
                protection,
            })
        }

        #[cfg(unix)]
        {
            let unix_prot = match protection {
                MemoryProtection::None => libc::PROT_NONE,
                MemoryProtection::ReadOnly => libc::PROT_READ,
                MemoryProtection::ReadWrite => libc::PROT_READ | libc::PROT_WRITE,
                MemoryProtection::ReadExecute => libc::PROT_READ | libc::PROT_EXEC,
            };

            let map_flags = libc::MAP_ANONYMOUS | libc::MAP_PRIVATE;
            let raw_ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    layout_size,
                    unix_prot,
                    map_flags,
                    -1,
                    0,
                )
            };

            if raw_ptr == libc::MAP_FAILED || raw_ptr.is_null() {
                let os_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                return Err(PalMemoryError::new(
                    os_err,
                    format!(
                        "mmap failed for {} bytes with protection {:?}",
                        layout_size, protection
                    ),
                    "POSIX kernel failed to allocate anonymous memory pages",
                    "Check process memory limits (rlimit) and address space exhaustion",
                ));
            }

            #[cfg(target_os = "linux")]
            if flags == AllocationFlags::HugePages {
                unsafe {
                    libc::madvise(raw_ptr, layout_size, libc::MADV_HUGEPAGE);
                }
            }

            Ok(Self {
                ptr: raw_ptr.cast(),
                layout_size,
                protection,
            })
        }

        #[cfg(not(any(windows, unix)))]
        {
            let _ = (layout_size, protection, flags);
            Err(PalMemoryError::new(
                -1,
                "Unsupported operating system for direct PAL page allocation",
                "Target platform lacks direct virtual memory driver bindings",
                "Compile for Windows (x86_64/aarch64) or POSIX Unix (Linux/macOS/BSD)",
            ))
        }
    }

    /// Change the page protection flags of this existing virtual memory mapping.
    pub fn set_protection(
        &mut self,
        new_protection: MemoryProtection,
    ) -> Result<(), PalMemoryError> {
        if self.ptr.is_null() || self.layout_size == 0 {
            return Err(PalMemoryError::new(
                0,
                "Cannot change protection on null page allocation",
                "Allocation pointer was null or layout size was zero",
                "Ensure allocation is active before mutating protection flags",
            ));
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Memory::{
                PAGE_EXECUTE_READ, PAGE_NOACCESS, PAGE_PROTECTION_FLAGS, PAGE_READONLY,
                PAGE_READWRITE, VirtualProtect,
            };

            let win_prot = match new_protection {
                MemoryProtection::None => PAGE_NOACCESS,
                MemoryProtection::ReadOnly => PAGE_READONLY,
                MemoryProtection::ReadWrite => PAGE_READWRITE,
                MemoryProtection::ReadExecute => PAGE_EXECUTE_READ,
            };

            let mut old_prot: PAGE_PROTECTION_FLAGS = 0;
            let res = unsafe {
                VirtualProtect(self.ptr.cast(), self.layout_size, win_prot, &mut old_prot)
            };

            if res == 0 {
                let os_err = unsafe { windows_sys::Win32::Foundation::GetLastError() } as i32;
                return Err(PalMemoryError::new(
                    os_err,
                    format!("VirtualProtect failed changing to {:?}", new_protection),
                    "OS rejected page protection transition",
                    "Verify page address validity and execution policy constraints",
                ));
            }

            self.protection = new_protection;
            Ok(())
        }

        #[cfg(unix)]
        {
            let unix_prot = match new_protection {
                MemoryProtection::None => libc::PROT_NONE,
                MemoryProtection::ReadOnly => libc::PROT_READ,
                MemoryProtection::ReadWrite => libc::PROT_READ | libc::PROT_WRITE,
                MemoryProtection::ReadExecute => libc::PROT_READ | libc::PROT_EXEC,
            };

            let res = unsafe { libc::mprotect(self.ptr.cast(), self.layout_size, unix_prot) };
            if res != 0 {
                let os_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                return Err(PalMemoryError::new(
                    os_err,
                    format!("mprotect failed changing to {:?}", new_protection),
                    "POSIX kernel rejected memory protection modification",
                    "Verify address alignment and security permission constraints",
                ));
            }

            self.protection = new_protection;
            Ok(())
        }

        #[cfg(not(any(windows, unix)))]
        {
            let _ = new_protection;
            Err(PalMemoryError::new(
                -1,
                "Unsupported operating system",
                "Cannot change protection on unsupported OS",
                "Compile for Windows or POSIX Unix",
            ))
        }
    }

    /// Return the aligned byte size allocated by the operating system kernel.
    pub fn len(&self) -> usize {
        self.layout_size
    }

    /// Check if the allocation size is zero.
    pub fn is_empty(&self) -> bool {
        self.layout_size == 0
    }

    /// Return the active memory protection mode of this allocation.
    pub fn protection(&self) -> MemoryProtection {
        self.protection
    }

    /// Return the base raw pointer to the mapped virtual address range.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Return the mutable base raw pointer to the mapped virtual address range.
    ///
    /// # Safety
    /// Caller must ensure that concurrent writes do not violate Rust aliasing guarantees
    /// and that memory access is restricted to the allocated page bounds.
    pub unsafe fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    /// Provide a safe byte slice view over the allocated memory pages.
    pub fn as_slice(&self) -> &[u8] {
        if self.ptr.is_null() || self.layout_size == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, self.layout_size) }
        }
    }

    /// Provide a safe mutable byte slice view over the allocated memory pages.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        if self.ptr.is_null() || self.layout_size == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.ptr, self.layout_size) }
        }
    }
}

impl Drop for PageAllocation {
    fn drop(&mut self) {
        if self.ptr.is_null() || self.layout_size == 0 {
            return;
        }

        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::Memory::{MEM_RELEASE, VirtualFree};
            VirtualFree(self.ptr.cast(), 0, MEM_RELEASE);
        }

        #[cfg(unix)]
        unsafe {
            libc::munmap(self.ptr.cast(), self.layout_size);
        }

        self.ptr = std::ptr::null_mut();
        self.layout_size = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_allocation_write_and_drop() {
        let alloc_res =
            PageAllocation::allocate(4096, MemoryProtection::ReadWrite, AllocationFlags::Standard);
        assert!(alloc_res.is_ok());
        if let Ok(mut alloc) = alloc_res {
            assert!(alloc.len() >= 4096);
            assert_eq!(alloc.protection(), MemoryProtection::ReadWrite);

            let slice = alloc.as_mut_slice();
            slice[0] = 0xDE;
            slice[1] = 0xAD;
            slice[2] = 0xBE;
            slice[3] = 0xEF;

            assert_eq!(&alloc.as_slice()[0..4], &[0xDE, 0xAD, 0xBE, 0xEF]);
        }
    }

    #[test]
    fn test_page_size_alignment_rounding() {
        let alloc_res =
            PageAllocation::allocate(100, MemoryProtection::ReadWrite, AllocationFlags::Standard);
        assert!(alloc_res.is_ok());
        if let Ok(alloc) = alloc_res {
            let page_sz = system_page_size();
            assert_eq!(alloc.len(), page_sz);
        }
    }

    #[test]
    fn test_zero_size_allocation_fails() {
        let alloc_res =
            PageAllocation::allocate(0, MemoryProtection::ReadWrite, AllocationFlags::Standard);
        assert!(alloc_res.is_err());
        if let Err(e) = alloc_res {
            assert!(e.to_string().contains("PAL Memory Diagnostic"));
        }
    }

    #[test]
    fn test_protection_transition() {
        let alloc_res =
            PageAllocation::allocate(4096, MemoryProtection::ReadWrite, AllocationFlags::Standard);
        assert!(alloc_res.is_ok());
        if let Ok(mut alloc) = alloc_res {
            let slice = alloc.as_mut_slice();
            slice[0] = 42;

            let prot_res = alloc.set_protection(MemoryProtection::ReadOnly);
            assert!(prot_res.is_ok());
            assert_eq!(alloc.protection(), MemoryProtection::ReadOnly);
            assert_eq!(alloc.as_slice()[0], 42);
        }
    }
}

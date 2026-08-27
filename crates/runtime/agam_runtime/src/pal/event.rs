//! Cross-platform asynchronous I/O event demultiplexer engine.
//!
//! Provides bare-metal OS polling multiplexing (`epoll` on Linux, `kqueue` on macOS/BSD,
//! and `WSAPoll` on Windows) with zero unvalidated panics and Nyāya diagnostic errors.

#![deny(clippy::unwrap_used)]

use std::fmt;
use std::time::Duration;

/// Token uniquely identifying a registered I/O descriptor or resource.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Token(pub usize);

/// Readiness interest flags for event registration.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EventInterest {
    pub readable: bool,
    pub writable: bool,
    pub edge_triggered: bool,
}

impl EventInterest {
    pub const READABLE: Self = Self {
        readable: true,
        writable: false,
        edge_triggered: false,
    };
    pub const WRITABLE: Self = Self {
        readable: false,
        writable: true,
        edge_triggered: false,
    };
    pub const BOTH: Self = Self {
        readable: true,
        writable: true,
        edge_triggered: false,
    };
}

/// A readiness notification delivered by the operating system kernel.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub token: Token,
    pub readable: bool,
    pub writable: bool,
    pub is_error: bool,
    pub is_hup: bool,
}

/// Polling timeout policy.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PollTimeout {
    Zero,
    Infinite,
    Duration(Duration),
}

/// Structured PAL event loop diagnostic formatted in the Agam Nyāya voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PalEventError {
    pub os_code: i32,
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl PalEventError {
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

impl fmt::Display for PalEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PAL Event Diagnostic (OS Code: {}): {}\n  Context: {}\n  Remedy:  {}",
            self.os_code, self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for PalEventError {}

#[cfg(windows)]
#[derive(Clone, Debug)]
struct WindowsPollEntry {
    socket: usize,
    token: Token,
    interest: EventInterest,
}

/// Cross-platform asynchronous I/O event demultiplexer.
pub struct EventDemuxer {
    #[cfg(target_os = "linux")]
    epoll_fd: i32,

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    kqueue_fd: i32,

    #[cfg(windows)]
    entries: Vec<WindowsPollEntry>,
}

unsafe impl Send for EventDemuxer {}
unsafe impl Sync for EventDemuxer {}

#[cfg(windows)]
fn ensure_winsock_initialized() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        unsafe {
            use windows_sys::Win32::Networking::WinSock::{WSAStartup, WSADATA};
            let mut data: WSADATA = std::mem::zeroed();
            let _ = WSAStartup(0x0202, &mut data);
        }
    });
}

impl EventDemuxer {
    /// Create a new OS-level event demultiplexer instance.
    pub fn new() -> Result<Self, PalEventError> {
        #[cfg(target_os = "linux")]
        {
            let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
            if epoll_fd < 0 {
                let os_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                return Err(PalEventError::new(
                    os_err,
                    "Failed to create epoll file descriptor via epoll_create1",
                    "Linux kernel epoll subsystem initialization failed",
                    "Verify process file descriptor limits (nofile) and kernel capabilities",
                ));
            }
            Ok(Self { epoll_fd })
        }

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            let kqueue_fd = unsafe { libc::kqueue() };
            if kqueue_fd < 0 {
                let os_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                return Err(PalEventError::new(
                    os_err,
                    "Failed to create kqueue file descriptor",
                    "BSD/macOS kqueue subsystem initialization failed",
                    "Verify file descriptor limits and kernel state",
                ));
            }
            Ok(Self { kqueue_fd })
        }

        #[cfg(windows)]
        {
            ensure_winsock_initialized();
            Ok(Self {
                entries: Vec::new(),
            })
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            windows
        )))]
        {
            Err(PalEventError::new(
                -1,
                "Unsupported operating system for async I/O demuxing",
                "Target platform lacks epoll/kqueue/WSAPoll backend driver",
                "Compile for Linux, macOS, BSD, or Windows",
            ))
        }
    }

    /// Register a raw descriptor/socket with the demultiplexer.
    pub fn register(
        &mut self,
        raw_handle: usize,
        token: Token,
        interest: EventInterest,
    ) -> Result<(), PalEventError> {
        #[cfg(target_os = "linux")]
        {
            let mut events = 0u32;
            if interest.readable {
                events |= libc::EPOLLIN as u32;
            }
            if interest.writable {
                events |= libc::EPOLLOUT as u32;
            }
            if interest.edge_triggered {
                events |= libc::EPOLLET as u32;
            }

            let mut ev = libc::epoll_event {
                events,
                u64: token.0 as u64,
            };

            let res = unsafe {
                libc::epoll_ctl(
                    self.epoll_fd,
                    libc::EPOLL_CTL_ADD,
                    raw_handle as i32,
                    &mut ev,
                )
            };

            if res < 0 {
                let os_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                return Err(PalEventError::new(
                    os_err,
                    format!("epoll_ctl ADD failed for descriptor {}", raw_handle),
                    "Kernel rejected descriptor registration in epoll interest set",
                    "Verify descriptor validity and ensure it is not already registered",
                ));
            }
            Ok(())
        }

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            let mut changes = Vec::with_capacity(2);
            let flags = libc::EV_ADD
                | if interest.edge_triggered {
                    libc::EV_CLEAR
                } else {
                    0
                };

            if interest.readable {
                changes.push(libc::kevent {
                    ident: raw_handle,
                    filter: libc::EVFILT_READ,
                    flags: flags as u16,
                    fflags: 0,
                    data: 0,
                    udata: token.0 as *mut libc::c_void,
                });
            }
            if interest.writable {
                changes.push(libc::kevent {
                    ident: raw_handle,
                    filter: libc::EVFILT_WRITE,
                    flags: flags as u16,
                    fflags: 0,
                    data: 0,
                    udata: token.0 as *mut libc::c_void,
                });
            }

            let res = unsafe {
                libc::kevent(
                    self.kqueue_fd,
                    changes.as_ptr(),
                    changes.len() as i32,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            };

            if res < 0 {
                let os_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                return Err(PalEventError::new(
                    os_err,
                    format!("kevent registration failed for descriptor {}", raw_handle),
                    "Kernel rejected descriptor registration in kqueue interest set",
                    "Verify descriptor validity and filter permissions",
                ));
            }
            Ok(())
        }

        #[cfg(windows)]
        {
            if let Some(entry) = self.entries.iter_mut().find(|e| e.socket == raw_handle) {
                entry.token = token;
                entry.interest = interest;
            } else {
                self.entries.push(WindowsPollEntry {
                    socket: raw_handle,
                    token,
                    interest,
                });
            }
            Ok(())
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            windows
        )))]
        {
            let _ = (raw_handle, token, interest);
            Err(PalEventError::new(
                -1,
                "Unsupported operating system",
                "Cannot register descriptor on unsupported OS",
                "Compile for Linux, macOS, BSD, or Windows",
            ))
        }
    }

    /// Modify the interest or token of a previously registered descriptor/socket.
    pub fn reregister(
        &mut self,
        raw_handle: usize,
        token: Token,
        interest: EventInterest,
    ) -> Result<(), PalEventError> {
        #[cfg(target_os = "linux")]
        {
            let mut events = 0u32;
            if interest.readable {
                events |= libc::EPOLLIN as u32;
            }
            if interest.writable {
                events |= libc::EPOLLOUT as u32;
            }
            if interest.edge_triggered {
                events |= libc::EPOLLET as u32;
            }

            let mut ev = libc::epoll_event {
                events,
                u64: token.0 as u64,
            };

            let res = unsafe {
                libc::epoll_ctl(
                    self.epoll_fd,
                    libc::EPOLL_CTL_MOD,
                    raw_handle as i32,
                    &mut ev,
                )
            };

            if res < 0 {
                let os_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                return Err(PalEventError::new(
                    os_err,
                    format!("epoll_ctl MOD failed for descriptor {}", raw_handle),
                    "Kernel failed to modify descriptor interest in epoll set",
                    "Verify that the descriptor was previously registered",
                ));
            }
            Ok(())
        }

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            // Re-registration in kqueue is achieved via EV_ADD with new filter / udata
            self.register(raw_handle, token, interest)
        }

        #[cfg(windows)]
        {
            if let Some(entry) = self.entries.iter_mut().find(|e| e.socket == raw_handle) {
                entry.token = token;
                entry.interest = interest;
                Ok(())
            } else {
                Err(PalEventError::new(
                    0,
                    format!("Descriptor {} not found in polling table", raw_handle),
                    "Attempted to reregister an untracked socket",
                    "Register the socket first using register()",
                ))
            }
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            windows
        )))]
        {
            let _ = (raw_handle, token, interest);
            Err(PalEventError::new(
                -1,
                "Unsupported operating system",
                "Cannot reregister descriptor on unsupported OS",
                "Compile for Linux, macOS, BSD, or Windows",
            ))
        }
    }

    /// Deregister a descriptor/socket from the demultiplexer.
    pub fn deregister(&mut self, raw_handle: usize) -> Result<(), PalEventError> {
        #[cfg(target_os = "linux")]
        {
            let res = unsafe {
                libc::epoll_ctl(
                    self.epoll_fd,
                    libc::EPOLL_CTL_DEL,
                    raw_handle as i32,
                    std::ptr::null_mut(),
                )
            };

            if res < 0 {
                let os_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                return Err(PalEventError::new(
                    os_err,
                    format!("epoll_ctl DEL failed for descriptor {}", raw_handle),
                    "Kernel failed to remove descriptor from epoll set",
                    "Verify that the descriptor is currently registered",
                ));
            }
            Ok(())
        }

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            let changes = [
                libc::kevent {
                    ident: raw_handle,
                    filter: libc::EVFILT_READ,
                    flags: libc::EV_DELETE as u16,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut(),
                },
                libc::kevent {
                    ident: raw_handle,
                    filter: libc::EVFILT_WRITE,
                    flags: libc::EV_DELETE as u16,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut(),
                },
            ];

            let _ = unsafe {
                libc::kevent(
                    self.kqueue_fd,
                    changes.as_ptr(),
                    changes.len() as i32,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            };
            Ok(())
        }

        #[cfg(windows)]
        {
            self.entries.retain(|e| e.socket != raw_handle);
            Ok(())
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            windows
        )))]
        {
            let _ = raw_handle;
            Err(PalEventError::new(
                -1,
                "Unsupported operating system",
                "Cannot deregister descriptor on unsupported OS",
                "Compile for Linux, macOS, BSD, or Windows",
            ))
        }
    }

    /// Wait for readiness events from the OS kernel and populate `events`.
    pub fn poll(
        &mut self,
        events: &mut Vec<Event>,
        timeout: PollTimeout,
    ) -> Result<usize, PalEventError> {
        events.clear();

        #[cfg(target_os = "linux")]
        {
            let timeout_ms = match timeout {
                PollTimeout::Zero => 0,
                PollTimeout::Infinite => -1,
                PollTimeout::Duration(d) => d.as_millis().min(i32::MAX as u128) as i32,
            };

            let mut epoll_events: [libc::epoll_event; 128] =
                unsafe { std::mem::zeroed() };

            let nfds = loop {
                let res = unsafe {
                    libc::epoll_wait(
                        self.epoll_fd,
                        epoll_events.as_mut_ptr(),
                        epoll_events.len() as i32,
                        timeout_ms,
                    )
                };
                if res < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() == Some(libc::EINTR) {
                        continue;
                    }
                    let os_err = err.raw_os_error().unwrap_or(-1);
                    return Err(PalEventError::new(
                        os_err,
                        "epoll_wait syscall returned an error",
                        "OS failed while waiting for descriptor events",
                        "Check epoll file descriptor health",
                    ));
                }
                break res as usize;
            };

            for i in 0..nfds {
                let ev = epoll_events[i];
                let ev_flags = ev.events;
                let readable = (ev_flags & (libc::EPOLLIN as u32 | libc::EPOLLPRI as u32)) != 0;
                let writable = (ev_flags & (libc::EPOLLOUT as u32)) != 0;
                let is_error = (ev_flags & (libc::EPOLLERR as u32)) != 0;
                let is_hup = (ev_flags & (libc::EPOLLHUP as u32 | libc::EPOLLRDHUP as u32)) != 0;

                events.push(Event {
                    token: Token(ev.u64 as usize),
                    readable,
                    writable,
                    is_error,
                    is_hup,
                });
            }
            Ok(events.len())
        }

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            let ts = match timeout {
                PollTimeout::Zero => Some(libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                }),
                PollTimeout::Infinite => None,
                PollTimeout::Duration(d) => Some(libc::timespec {
                    tv_sec: d.as_secs() as libc::time_t,
                    tv_nsec: d.subsec_nanos() as libc::c_long,
                }),
            };

            let ts_ptr = match &ts {
                Some(t) => t as *const libc::timespec,
                None => std::ptr::null(),
            };

            let mut kev_events: [libc::kevent; 128] = unsafe { std::mem::zeroed() };

            let nevents = loop {
                let res = unsafe {
                    libc::kevent(
                        self.kqueue_fd,
                        std::ptr::null(),
                        0,
                        kev_events.as_mut_ptr(),
                        kev_events.len() as i32,
                        ts_ptr,
                    )
                };
                if res < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() == Some(libc::EINTR) {
                        continue;
                    }
                    let os_err = err.raw_os_error().unwrap_or(-1);
                    return Err(PalEventError::new(
                        os_err,
                        "kevent wait syscall returned an error",
                        "OS failed while waiting for kqueue events",
                        "Check kqueue file descriptor health",
                    ));
                }
                break res as usize;
            };

            for i in 0..nevents {
                let ev = kev_events[i];
                let readable = ev.filter == libc::EVFILT_READ;
                let writable = ev.filter == libc::EVFILT_WRITE;
                let is_error = (ev.flags & (libc::EV_ERROR as u16)) != 0;
                let is_hup = (ev.flags & (libc::EV_EOF as u16)) != 0;

                events.push(Event {
                    token: Token(ev.udata as usize),
                    readable,
                    writable,
                    is_error,
                    is_hup,
                });
            }
            Ok(events.len())
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::Networking::WinSock::{
                POLLERR, POLLHUP, POLLIN, POLLNVAL, POLLOUT, WSAPOLLFD, WSAPoll,
            };

            if self.entries.is_empty() {
                match timeout {
                    PollTimeout::Zero => return Ok(0),
                    PollTimeout::Duration(d) => {
                        std::thread::sleep(d);
                        return Ok(0);
                    }
                    PollTimeout::Infinite => {
                        std::thread::sleep(Duration::from_millis(100));
                        return Ok(0);
                    }
                }
            }

            let mut poll_fds: Vec<WSAPOLLFD> = self
                .entries
                .iter()
                .map(|e| {
                    let mut req_events = 0i16;
                    if e.interest.readable {
                        req_events |= POLLIN as i16;
                    }
                    if e.interest.writable {
                        req_events |= POLLOUT as i16;
                    }
                    WSAPOLLFD {
                        fd: e.socket,
                        events: req_events,
                        revents: 0,
                    }
                })
                .collect();

            let timeout_ms = match timeout {
                PollTimeout::Zero => 0,
                PollTimeout::Infinite => -1,
                PollTimeout::Duration(d) => d.as_millis().min(i32::MAX as u128) as i32,
            };

            let res = unsafe {
                WSAPoll(
                    poll_fds.as_mut_ptr(),
                    poll_fds.len() as u32,
                    timeout_ms,
                )
            };

            if res < 0 {
                let os_err = unsafe { windows_sys::Win32::Networking::WinSock::WSAGetLastError() } as i32;
                return Err(PalEventError::new(
                    os_err,
                    "WSAPoll syscall returned an error",
                    "Windows Sockets demuxer failed during polling",
                    "Check WinSock initialization and socket handle health",
                ));
            }

            for (i, pfd) in poll_fds.iter().enumerate() {
                if pfd.revents != 0 {
                    let entry = &self.entries[i];
                    let rev = pfd.revents;
                    let readable = (rev & (POLLIN as i16)) != 0;
                    let writable = (rev & (POLLOUT as i16)) != 0;
                    let is_error = (rev & (POLLERR as i16 | POLLNVAL as i16)) != 0;
                    let is_hup = (rev & (POLLHUP as i16)) != 0;

                    events.push(Event {
                        token: entry.token,
                        readable,
                        writable,
                        is_error,
                        is_hup,
                    });
                }
            }
            Ok(events.len())
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            windows
        )))]
        {
            let _ = (events, timeout);
            Err(PalEventError::new(
                -1,
                "Unsupported operating system",
                "Cannot poll on unsupported OS",
                "Compile for Linux, macOS, BSD, or Windows",
            ))
        }
    }
}

impl Drop for EventDemuxer {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            if self.epoll_fd >= 0 {
                unsafe {
                    libc::close(self.epoll_fd);
                }
                self.epoll_fd = -1;
            }
        }

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            if self.kqueue_fd >= 0 {
                unsafe {
                    libc::close(self.kqueue_fd);
                }
                self.kqueue_fd = -1;
            }
        }

        #[cfg(windows)]
        {
            self.entries.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::io::Write;

    #[test]
    fn test_event_demuxer_creation_and_teardown() {
        let demuxer_res = EventDemuxer::new();
        assert!(demuxer_res.is_ok());
    }

    #[test]
    fn test_event_demuxer_poll_timeout_zero() {
        let mut demuxer = EventDemuxer::new().unwrap_or_else(|_| unreachable!());
        let mut events = Vec::new();
        let res = demuxer.poll(&mut events, PollTimeout::Zero);
        assert!(res.is_ok());
        if let Ok(count) = res {
            assert_eq!(count, 0);
            assert!(events.is_empty());
        }
    }

    #[test]
    fn test_event_demuxer_poll_timeout_duration() {
        let mut demuxer = EventDemuxer::new().unwrap_or_else(|_| unreachable!());
        let mut events = Vec::new();
        let start = std::time::Instant::now();
        let res = demuxer.poll(&mut events, PollTimeout::Duration(Duration::from_millis(50)));
        let elapsed = start.elapsed();

        assert!(res.is_ok());
        if let Ok(count) = res {
            assert_eq!(count, 0);
            assert!(events.is_empty());
            assert!(elapsed >= Duration::from_millis(30));
        }
    }

    #[test]
    fn test_event_pipe_readiness_notification() {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(_) => return,
        };
        let addr = match listener.local_addr() {
            Ok(a) => a,
            Err(_) => return,
        };

        let mut sender = match TcpStream::connect(addr) {
            Ok(s) => s,
            Err(_) => return,
        };

        let (receiver, _) = match listener.accept() {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let mut demuxer = match EventDemuxer::new() {
            Ok(d) => d,
            Err(_) => return,
        };

        #[cfg(unix)]
        let raw_handle = {
            use std::os::unix::io::AsRawFd;
            receiver.as_raw_fd() as usize
        };

        #[cfg(windows)]
        let raw_handle = {
            use std::os::windows::io::AsRawSocket;
            receiver.as_raw_socket() as usize
        };

        let reg_res = demuxer.register(raw_handle, Token(42), EventInterest::READABLE);
        assert!(reg_res.is_ok());

        // Write a byte to the sender stream
        let write_res = sender.write_all(b"X");
        assert!(write_res.is_ok());

        let mut events = Vec::new();
        let poll_res = demuxer.poll(&mut events, PollTimeout::Duration(Duration::from_millis(200)));
        assert!(poll_res.is_ok());
        if let Ok(count) = poll_res {
            assert!(count >= 1);
            let target_ev = events.iter().find(|e| e.token == Token(42));
            assert!(target_ev.is_some());
            if let Some(ev) = target_ev {
                assert!(ev.readable);
            }
        }

        let dereg_res = demuxer.deregister(raw_handle);
        assert!(dereg_res.is_ok());
    }
}

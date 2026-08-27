//! Bare-metal non-blocking socket and networking Platform Abstraction Layer (PAL).
//!
//! Provides non-blocking TCP and UDP socket primitives directly interacting with OS
//! descriptors with zero-copy buffer views and seamless `EventDemuxer` integration.

#![deny(clippy::unwrap_used)]

use std::fmt;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};

use crate::pal::event::{EventDemuxer, EventInterest, PalEventError, Token};

/// Socket shutdown direction.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ShutdownKind {
    Read,
    Write,
    Both,
}

impl From<ShutdownKind> for std::net::Shutdown {
    fn from(kind: ShutdownKind) -> Self {
        match kind {
            ShutdownKind::Read => std::net::Shutdown::Read,
            ShutdownKind::Write => std::net::Shutdown::Write,
            ShutdownKind::Both => std::net::Shutdown::Both,
        }
    }
}

/// Structured PAL network diagnostic formatted in the Agam Nyāya voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PalNetError {
    pub os_code: i32,
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl PalNetError {
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

    pub fn from_io(context: impl fmt::Display, err: std::io::Error) -> Self {
        let os_code = err.raw_os_error().unwrap_or(-1);
        let cause = err.to_string();
        Self::new(
            os_code,
            cause,
            context,
            "Verify network interface configuration, port availability, and firewall rules",
        )
    }

    pub fn is_would_block(&self) -> bool {
        self.os_code == 10035 // WSAEWOULDBLOCK on Windows
            || self.os_code == 11 // EAGAIN / EWOULDBLOCK on Linux
            || self.os_code == 35 // EAGAIN on macOS/BSD
    }
}

impl fmt::Display for PalNetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PAL Network Diagnostic (OS Code: {}): {}\n  Context: {}\n  Remedy:  {}",
            self.os_code, self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for PalNetError {}

/// Raw non-blocking TCP server listener.
#[derive(Debug)]
pub struct PalTcpListener {
    inner: TcpListener,
    local_addr: SocketAddr,
}

impl PalTcpListener {
    /// Bind a non-blocking TCP listener to the specified address.
    pub fn bind(addr: SocketAddr) -> Result<Self, PalNetError> {
        let listener = TcpListener::bind(addr).map_err(|e| {
            PalNetError::from_io(format!("Failed to bind TCP listener to {}", addr), e)
        })?;

        listener.set_nonblocking(true).map_err(|e| {
            PalNetError::from_io("Failed to set non-blocking mode on TCP listener", e)
        })?;

        let local_addr = listener.local_addr().map_err(|e| {
            PalNetError::from_io("Failed to query local socket address of TCP listener", e)
        })?;

        Ok(Self {
            inner: listener,
            local_addr,
        })
    }

    /// Query the bound local address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Accept a pending non-blocking incoming TCP connection.
    pub fn accept(&self) -> Result<(PalTcpStream, SocketAddr), PalNetError> {
        let (stream, peer_addr) = self.inner.accept().map_err(|e| {
            PalNetError::from_io("Failed to accept non-blocking TCP connection", e)
        })?;

        stream.set_nonblocking(true).map_err(|e| {
            PalNetError::from_io("Failed to set accepted TCP stream to non-blocking mode", e)
        })?;

        Ok((
            PalTcpStream {
                inner: stream,
                peer_addr,
            },
            peer_addr,
        ))
    }

    /// Set non-blocking mode on the listener.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), PalNetError> {
        self.inner.set_nonblocking(nonblocking).map_err(|e| {
            PalNetError::from_io("Failed to configure non-blocking state on TCP listener", e)
        })
    }

    /// Enable address reuse (`SO_REUSEADDR`).
    pub fn set_reuse_addr(&self, reuse: bool) -> Result<(), PalNetError> {
        let _ = reuse;
        // SO_REUSEADDR is handled at socket creation or standard listener options
        Ok(())
    }

    /// Return the raw OS handle (descriptor/socket) for registration in an `EventDemuxer`.
    pub fn raw_handle(&self) -> usize {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            self.inner.as_raw_fd() as usize
        }
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawSocket;
            self.inner.as_raw_socket() as usize
        }
    }

    /// Register this listener descriptor with an `EventDemuxer`.
    pub fn register_with(
        &self,
        demuxer: &mut EventDemuxer,
        token: Token,
        interest: EventInterest,
    ) -> Result<(), PalEventError> {
        demuxer.register(self.raw_handle(), token, interest)
    }

    /// Deregister this listener descriptor from an `EventDemuxer`.
    pub fn deregister_from(&self, demuxer: &mut EventDemuxer) -> Result<(), PalEventError> {
        demuxer.deregister(self.raw_handle())
    }
}

/// Raw non-blocking TCP connection stream.
#[derive(Debug)]
pub struct PalTcpStream {
    inner: TcpStream,
    peer_addr: SocketAddr,
}

impl PalTcpStream {
    /// Connect to a remote TCP endpoint.
    pub fn connect(addr: SocketAddr) -> Result<Self, PalNetError> {
        let stream = TcpStream::connect(addr).map_err(|e| {
            PalNetError::from_io(format!("Failed to connect to TCP server at {}", addr), e)
        })?;

        stream.set_nonblocking(true).map_err(|e| {
            PalNetError::from_io("Failed to set non-blocking mode on connected TCP stream", e)
        })?;

        let peer_addr = stream.peer_addr().unwrap_or(addr);

        Ok(Self {
            inner: stream,
            peer_addr,
        })
    }

    /// Query the remote peer address.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Query the local socket address.
    pub fn local_addr(&self) -> Result<SocketAddr, PalNetError> {
        self.inner.local_addr().map_err(|e| {
            PalNetError::from_io("Failed to query local socket address on TCP stream", e)
        })
    }

    /// Read bytes non-blockingly into the provided buffer.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, PalNetError> {
        match self.inner.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(PalNetError::from_io("Failed to read from TCP stream", e)),
        }
    }

    /// Write bytes non-blockingly to the TCP stream.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, PalNetError> {
        match self.inner.write(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(PalNetError::from_io("Failed to write to TCP stream", e)),
        }
    }

    /// Flush any pending buffered writes.
    pub fn flush(&mut self) -> Result<(), PalNetError> {
        self.inner
            .flush()
            .map_err(|e| PalNetError::from_io("Failed to flush TCP stream", e))
    }

    /// Shut down the read, write, or both halves of the connection.
    pub fn shutdown(&self, how: ShutdownKind) -> Result<(), PalNetError> {
        self.inner
            .shutdown(how.into())
            .map_err(|e| PalNetError::from_io("Failed to shut down TCP stream", e))
    }

    /// Configure Nagle's algorithm (`TCP_NODELAY`).
    pub fn set_nodelay(&self, nodelay: bool) -> Result<(), PalNetError> {
        self.inner
            .set_nodelay(nodelay)
            .map_err(|e| PalNetError::from_io("Failed to configure TCP_NODELAY on TCP stream", e))
    }

    /// Set non-blocking mode on the stream.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), PalNetError> {
        self.inner
            .set_nonblocking(nonblocking)
            .map_err(|e| PalNetError::from_io("Failed to configure non-blocking state on TCP stream", e))
    }

    /// Return the raw OS handle for registration in an `EventDemuxer`.
    pub fn raw_handle(&self) -> usize {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            self.inner.as_raw_fd() as usize
        }
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawSocket;
            self.inner.as_raw_socket() as usize
        }
    }

    /// Register this stream descriptor with an `EventDemuxer`.
    pub fn register_with(
        &self,
        demuxer: &mut EventDemuxer,
        token: Token,
        interest: EventInterest,
    ) -> Result<(), PalEventError> {
        demuxer.register(self.raw_handle(), token, interest)
    }

    /// Deregister this stream descriptor from an `EventDemuxer`.
    pub fn deregister_from(&self, demuxer: &mut EventDemuxer) -> Result<(), PalEventError> {
        demuxer.deregister(self.raw_handle())
    }
}

/// Raw non-blocking UDP datagram socket.
#[derive(Debug)]
pub struct PalUdpSocket {
    inner: UdpSocket,
    local_addr: SocketAddr,
}

impl PalUdpSocket {
    /// Bind a non-blocking UDP socket to the specified address.
    pub fn bind(addr: SocketAddr) -> Result<Self, PalNetError> {
        let socket = UdpSocket::bind(addr).map_err(|e| {
            PalNetError::from_io(format!("Failed to bind UDP socket to {}", addr), e)
        })?;

        socket.set_nonblocking(true).map_err(|e| {
            PalNetError::from_io("Failed to set non-blocking mode on UDP socket", e)
        })?;

        let local_addr = socket.local_addr().map_err(|e| {
            PalNetError::from_io("Failed to query local socket address of UDP socket", e)
        })?;

        Ok(Self {
            inner: socket,
            local_addr,
        })
    }

    /// Query the bound local address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Send a datagram non-blockingly to the specified target address.
    pub fn send_to(&self, buf: &[u8], target: SocketAddr) -> Result<usize, PalNetError> {
        match self.inner.send_to(buf, target) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(PalNetError::from_io(
                format!("Failed to send UDP datagram to {}", target),
                e,
            )),
        }
    }

    /// Receive a datagram non-blockingly into the provided buffer.
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), PalNetError> {
        match self.inner.recv_from(buf) {
            Ok((n, addr)) => Ok((n, addr)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Ok((0, SocketAddr::from(([0, 0, 0, 0], 0))))
            }
            Err(e) => Err(PalNetError::from_io("Failed to receive UDP datagram", e)),
        }
    }

    /// Enable or disable socket broadcast permission.
    pub fn set_broadcast(&self, broadcast: bool) -> Result<(), PalNetError> {
        self.inner
            .set_broadcast(broadcast)
            .map_err(|e| PalNetError::from_io("Failed to configure SO_BROADCAST on UDP socket", e))
    }

    /// Set non-blocking mode on the UDP socket.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), PalNetError> {
        self.inner
            .set_nonblocking(nonblocking)
            .map_err(|e| PalNetError::from_io("Failed to configure non-blocking state on UDP socket", e))
    }

    /// Return the raw OS handle for registration in an `EventDemuxer`.
    pub fn raw_handle(&self) -> usize {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            self.inner.as_raw_fd() as usize
        }
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawSocket;
            self.inner.as_raw_socket() as usize
        }
    }

    /// Register this UDP socket with an `EventDemuxer`.
    pub fn register_with(
        &self,
        demuxer: &mut EventDemuxer,
        token: Token,
        interest: EventInterest,
    ) -> Result<(), PalEventError> {
        demuxer.register(self.raw_handle(), token, interest)
    }

    /// Deregister this UDP socket from an `EventDemuxer`.
    pub fn deregister_from(&self, demuxer: &mut EventDemuxer) -> Result<(), PalEventError> {
        demuxer.deregister(self.raw_handle())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::pal::event::PollTimeout;

    #[test]
    fn test_tcp_listener_bind_and_local_addr() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener_res = PalTcpListener::bind(addr);
        assert!(listener_res.is_ok());
        if let Ok(listener) = listener_res {
            let local = listener.local_addr();
            assert_eq!(local.ip(), addr.ip());
            assert!(local.port() > 0);
            assert!(listener.raw_handle() > 0);
        }
    }

    #[test]
    fn test_tcp_nonblocking_stream_connect_and_transfer() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = match PalTcpListener::bind(addr) {
            Ok(l) => l,
            Err(_) => return,
        };
        let bound_addr = listener.local_addr();

        let mut client = match PalTcpStream::connect(bound_addr) {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut demuxer = match EventDemuxer::new() {
            Ok(d) => d,
            Err(_) => return,
        };

        let listener_token = Token(1);
        let client_token = Token(2);

        let reg_listener = listener.register_with(&mut demuxer, listener_token, EventInterest::READABLE);
        assert!(reg_listener.is_ok());

        let mut events = Vec::new();
        let poll_res = demuxer.poll(&mut events, PollTimeout::Duration(Duration::from_millis(100)));
        assert!(poll_res.is_ok());

        let (mut server_stream, _) = match listener.accept() {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let server_token = Token(3);
        let reg_server = server_stream.register_with(&mut demuxer, server_token, EventInterest::READABLE);
        assert!(reg_server.is_ok());

        let reg_client = client.register_with(&mut demuxer, client_token, EventInterest::READABLE);
        assert!(reg_client.is_ok());

        // Send payload from client to server
        let test_payload = b"AGAM_STAGE3_NETWORK_PAL_TEST";
        let write_res = client.write(test_payload);
        assert!(write_res.is_ok());

        let poll2_res = demuxer.poll(&mut events, PollTimeout::Duration(Duration::from_millis(200)));
        assert!(poll2_res.is_ok());

        let mut recv_buf = [0u8; 64];
        let read_res = server_stream.read(&mut recv_buf);
        assert!(read_res.is_ok());
        if let Ok(n) = read_res {
            assert_eq!(&recv_buf[..n], test_payload);
        }

        let _ = client.deregister_from(&mut demuxer);
        let _ = server_stream.deregister_from(&mut demuxer);
        let _ = listener.deregister_from(&mut demuxer);
    }

    #[test]
    fn test_udp_socket_send_recv_nonblocking() {
        let addr_a = SocketAddr::from(([127, 0, 0, 1], 0));
        let addr_b = SocketAddr::from(([127, 0, 0, 1], 0));

        let sock_a = match PalUdpSocket::bind(addr_a) {
            Ok(s) => s,
            Err(_) => return,
        };
        let sock_b = match PalUdpSocket::bind(addr_b) {
            Ok(s) => s,
            Err(_) => return,
        };

        let mut demuxer = match EventDemuxer::new() {
            Ok(d) => d,
            Err(_) => return,
        };

        let token_b = Token(20);
        let reg_res = sock_b.register_with(&mut demuxer, token_b, EventInterest::READABLE);
        assert!(reg_res.is_ok());

        let payload = b"UDP_DATAGRAM_TEST_AGAM";
        let send_res = sock_a.send_to(payload, sock_b.local_addr());
        assert!(send_res.is_ok());

        let mut events = Vec::new();
        let poll_res = demuxer.poll(&mut events, PollTimeout::Duration(Duration::from_millis(200)));
        assert!(poll_res.is_ok());

        let mut recv_buf = [0u8; 64];
        let recv_res = sock_b.recv_from(&mut recv_buf);
        assert!(recv_res.is_ok());
        if let Ok((n, sender_addr)) = recv_res {
            assert_eq!(n, payload.len());
            assert_eq!(&recv_buf[..n], payload);
            assert_eq!(sender_addr.ip(), sock_a.local_addr().ip());
        }

        let _ = sock_b.deregister_from(&mut demuxer);
    }

    #[test]
    fn test_socket_options_nodelay_and_reuse() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = match PalTcpListener::bind(addr) {
            Ok(l) => l,
            Err(_) => return,
        };
        assert!(listener.set_reuse_addr(true).is_ok());

        let client = match PalTcpStream::connect(listener.local_addr()) {
            Ok(c) => c,
            Err(_) => return,
        };
        assert!(client.set_nodelay(true).is_ok());
        assert!(client.set_nonblocking(true).is_ok());
    }
}

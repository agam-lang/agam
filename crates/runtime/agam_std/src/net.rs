use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};

/// Structured error for network operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetError {
    pub operation: String,
    pub address: String,
    pub message: String,
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NetError in operation '{}' on address '{}': {}",
            self.operation, self.address, self.message
        )
    }
}

impl std::error::Error for NetError {}

/// Handle table managing open TCP streams and listeners by ID.
pub struct NetworkManager {
    next_id: i64,
    streams: HashMap<i64, TcpStream>,
    listeners: HashMap<i64, TcpListener>,
    udp_sockets: HashMap<i64, UdpSocket>,
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            streams: HashMap::new(),
            listeners: HashMap::new(),
            udp_sockets: HashMap::new(),
        }
    }

    pub fn connect(&mut self, addr: &str) -> Result<i64, NetError> {
        let stream = TcpStream::connect(addr).map_err(|e| NetError {
            operation: "connect".to_string(),
            address: addr.to_string(),
            message: e.to_string(),
        })?;
        let id = self.next_id;
        self.next_id += 1;
        self.streams.insert(id, stream);
        Ok(id)
    }

    pub fn listen(&mut self, addr: &str) -> Result<i64, NetError> {
        let listener = TcpListener::bind(addr).map_err(|e| NetError {
            operation: "listen".to_string(),
            address: addr.to_string(),
            message: e.to_string(),
        })?;
        let id = self.next_id;
        self.next_id += 1;
        self.listeners.insert(id, listener);
        Ok(id)
    }

    pub fn accept(&mut self, listener_id: i64) -> Result<i64, NetError> {
        let listener = self.listeners.get(&listener_id).ok_or_else(|| NetError {
            operation: "accept".to_string(),
            address: format!("listener:{listener_id}"),
            message: "invalid listener id".to_string(),
        })?;
        let (stream, _) = listener.accept().map_err(|e| NetError {
            operation: "accept".to_string(),
            address: format!("listener:{listener_id}"),
            message: e.to_string(),
        })?;
        let id = self.next_id;
        self.next_id += 1;
        self.streams.insert(id, stream);
        Ok(id)
    }

    pub fn send(&mut self, stream_id: i64, data: &[u8]) -> Result<usize, NetError> {
        let stream = self.streams.get_mut(&stream_id).ok_or_else(|| NetError {
            operation: "send".to_string(),
            address: format!("stream:{stream_id}"),
            message: "invalid stream id".to_string(),
        })?;
        stream.write_all(data).map_err(|e| NetError {
            operation: "send".to_string(),
            address: format!("stream:{stream_id}"),
            message: e.to_string(),
        })?;
        Ok(data.len())
    }

    pub fn recv(&mut self, stream_id: i64, max_bytes: usize) -> Result<Vec<u8>, NetError> {
        let stream = self.streams.get_mut(&stream_id).ok_or_else(|| NetError {
            operation: "recv".to_string(),
            address: format!("stream:{stream_id}"),
            message: "invalid stream id".to_string(),
        })?;
        let mut buf = vec![0u8; max_bytes];
        let bytes_read = stream.read(&mut buf).map_err(|e| NetError {
            operation: "recv".to_string(),
            address: format!("stream:{stream_id}"),
            message: e.to_string(),
        })?;
        buf.truncate(bytes_read);
        Ok(buf)
    }

    pub fn close(&mut self, id: i64) -> bool {
        self.streams.remove(&id).is_some()
            || self.listeners.remove(&id).is_some()
            || self.udp_sockets.remove(&id).is_some()
    }
}

use std::sync::{Mutex, OnceLock};

pub fn global_net_manager() -> &'static Mutex<NetworkManager> {
    static INSTANCE: OnceLock<Mutex<NetworkManager>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(NetworkManager::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_manager_lifecycle() {
        let mut mgr = NetworkManager::new();
        let listener_id = mgr.listen("127.0.0.1:0").expect("listen failed");
        assert!(listener_id > 0);
        let closed = mgr.close(listener_id);
        assert!(closed);
    }
}

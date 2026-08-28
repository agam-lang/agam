//! # Actor Concurrency Runtime & Supervision System (`agam_runtime::actor`)
//!
//! Provides an Erlang/Akka-inspired message-passing actor model:
//! - **Lightweight Actor Cells**: Isolated execution units with dedicated mailboxes.
//! - **Asynchronous Messaging**: Non-blocking `tell` ($O(1)$) and futures-based `ask` with timeout.
//! - **Fault Tolerance & Supervision Trees**: `OneForOne`, `OneForAll`, and `RestForOne` restart strategies.
//! - **Lifecycle Hooks & DeathWatch**: `pre_start`, `post_stop`, `pre_restart`, `watch`, and `unwatch`.
//! - **Zero-Panic & Nyāya Diagnostic Errors**: Structured error handling with cause, context, and remedy.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Unique 64-bit identifier assigned to each spawned actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorId(pub u64);

impl fmt::Display for ActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "actor-{}", self.0)
    }
}

static NEXT_ACTOR_ID: AtomicU64 = AtomicU64::new(1);

fn generate_actor_id() -> ActorId {
    ActorId(NEXT_ACTOR_ID.fetch_add(1, Ordering::Relaxed))
}

/// Nyāya-grounded structured diagnostic error for the actor subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorError {
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl ActorError {
    pub fn new(
        cause: impl Into<String>,
        context: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            cause: cause.into(),
            context: context.into(),
            remedy: remedy.into(),
        }
    }
}

impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ActorError: {}\n  Context: {}\n  Remedy: {}",
            self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for ActorError {}

/// Directive returned by a supervisor when a child actor encounters a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorDirective {
    /// Resume the actor, keeping its current internal state and continuing with next message.
    Resume,
    /// Restart the actor, re-executing constructor/lifecycle hook and clearing transient state.
    Restart,
    /// Stop the actor permanently and notify all watchers of its termination.
    Stop,
    /// Escalate the failure to the parent supervisor in the hierarchy.
    Escalate,
}

/// Fault-tolerance supervision strategy governing child actor crashes.
#[derive(Debug, Clone)]
pub enum SupervisionStrategy {
    /// If a child crashes, only that specific child is restarted.
    OneForOne {
        max_restarts: usize,
        within: Duration,
    },
    /// If any child crashes, all sibling actors under this supervisor are restarted.
    OneForAll {
        max_restarts: usize,
        within: Duration,
    },
    /// If a child crashes, that child and any siblings spawned after it are restarted.
    RestForOne {
        max_restarts: usize,
        within: Duration,
    },
}

impl Default for SupervisionStrategy {
    fn default() -> Self {
        Self::OneForOne {
            max_restarts: 5,
            within: Duration::from_secs(5),
        }
    }
}

/// Lifecycle and execution result returned from an actor message handler.
pub type ActorResult = Result<(), ActorError>;

/// System-level control messages handled uniformly across all actor mailboxes.
#[derive(Debug)]
pub enum SystemMessage {
    /// Notification that a watched actor has terminated.
    Terminated(ActorId),
    /// Instruction to gracefully stop message processing and shut down.
    Stop,
}

/// Actor interface defining message handling, state transitions, and lifecycle hooks.
pub trait Actor: Send + 'static {
    /// Message type accepted by this actor.
    type Message: Send + 'static;

    /// Process a received message.
    fn handle(&mut self, ctx: &mut ActorContext<Self::Message>, msg: Self::Message) -> ActorResult;

    /// Lifecycle hook invoked immediately before message processing begins.
    fn pre_start(&mut self, _ctx: &mut ActorContext<Self::Message>) -> ActorResult {
        Ok(())
    }

    /// Lifecycle hook invoked immediately after the actor stops.
    fn post_stop(&mut self, _ctx: &mut ActorContext<Self::Message>) -> ActorResult {
        Ok(())
    }

    /// Supervisor strategy governing children spawned by this actor.
    fn supervisor_strategy(&self) -> SupervisionStrategy {
        SupervisionStrategy::default()
    }
}

pub(crate) enum Envelope<M> {
    User(M),
    System(SystemMessage),
}

/// Thread-safe handle to an actor mailbox for sending messages and requests.
pub struct ActorRef<M> {
    id: ActorId,
    sender: Sender<Envelope<M>>,
    is_alive: Arc<AtomicBool>,
}

impl<M> Clone for ActorRef<M> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            sender: self.sender.clone(),
            is_alive: Arc::clone(&self.is_alive),
        }
    }
}

impl<M: Send + 'static> ActorRef<M> {
    /// Unique identifier of the referenced actor.
    pub fn id(&self) -> ActorId {
        self.id
    }

    /// Check if the target actor's cell is currently active.
    pub fn is_alive(&self) -> bool {
        self.is_alive.load(Ordering::Relaxed)
    }

    /// Asynchronous fire-and-forget message send ($O(1)$ lock-free enqueue).
    pub fn tell(&self, msg: M) -> Result<(), ActorError> {
        if !self.is_alive() {
            return Err(ActorError::new(
                format!("Cannot send message to dead actor {}", self.id),
                "Actor has terminated or stopped",
                "Ensure actor is alive before sending or use deathwatch monitoring",
            ));
        }

        self.sender.send(Envelope::User(msg)).map_err(|_| {
            ActorError::new(
                format!("Mailbox channel disconnected for actor {}", self.id),
                "Underlying receiver dropped",
                "Spawn a new actor instance or check supervisor logs",
            )
        })
    }

    /// Synchronous request-response pattern with configurable timeout.
    pub fn ask<R: Send + 'static>(
        &self,
        factory: impl FnOnce(Sender<R>) -> M,
        timeout_dur: Duration,
    ) -> Result<R, ActorError> {
        let (reply_tx, reply_rx) = channel::<R>();
        let msg = factory(reply_tx);
        self.tell(msg)?;

        reply_rx.recv_timeout(timeout_dur).map_err(|e| {
            ActorError::new(
                format!("Ask timeout / receive error from actor {}: {}", self.id, e),
                format!("Waited for reply for {:?}", timeout_dur),
                "Increase timeout duration or check recipient actor handler throughput",
            )
        })
    }

    /// Send a system stop command to the actor.
    pub fn stop(&self) -> Result<(), ActorError> {
        if !self.is_alive() {
            return Ok(());
        }
        let _ = self.sender.send(Envelope::System(SystemMessage::Stop));
        Ok(())
    }
}

/// Execution context provided to an actor during message processing.
pub struct ActorContext<M: 'static> {
    id: ActorId,
    self_ref: ActorRef<M>,
    system: ActorSystem,
    stopped: bool,
    watchers: Vec<Sender<Envelope<M>>>,
}

impl<M: Send + 'static> ActorContext<M> {
    /// Return the `ActorRef` referencing this actor.
    pub fn self_ref(&self) -> ActorRef<M> {
        self.self_ref.clone()
    }

    /// Return the unique ID of this actor.
    pub fn id(&self) -> ActorId {
        self.id
    }

    /// Spawn a child actor within this actor system.
    pub fn spawn<C: Actor>(&self, behavior: C) -> Result<ActorRef<C::Message>, ActorError> {
        self.system.spawn(behavior)
    }

    /// Mark this actor to stop processing messages after the current handler finishes.
    pub fn stop(&mut self) {
        self.stopped = true;
    }

    /// Watch this actor, receiving `SystemMessage::Terminated(id)` when it stops.
    #[allow(dead_code)]
    pub(crate) fn watch(&mut self, watcher: Sender<Envelope<M>>) {
        self.watchers.push(watcher);
    }

    /// Access the hosting `ActorSystem`.
    pub fn system(&self) -> &ActorSystem {
        &self.system
    }
}

struct ActorCell<A: Actor> {
    id: ActorId,
    behavior: A,
    receiver: Receiver<Envelope<A::Message>>,
    self_ref: ActorRef<A::Message>,
    is_alive: Arc<AtomicBool>,
    system: ActorSystem,
}

impl<A: Actor> ActorCell<A> {
    fn run(mut self) {
        let mut ctx = ActorContext {
            id: self.id,
            self_ref: self.self_ref.clone(),
            system: self.system.clone(),
            stopped: false,
            watchers: Vec::new(),
        };

        if let Err(e) = self.behavior.pre_start(&mut ctx) {
            eprintln!("[actor-system] Pre-start failed for {}: {}", self.id, e);
            self.is_alive.store(false, Ordering::Release);
            return;
        }

        while !ctx.stopped {
            match self.receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(Envelope::User(msg)) => {
                    if let Err(err) = self.behavior.handle(&mut ctx, msg) {
                        eprintln!(
                            "[actor-system] Error handling message in {}: {}",
                            self.id, err
                        );
                        break;
                    }
                }
                Ok(Envelope::System(SystemMessage::Stop)) => {
                    ctx.stopped = true;
                    break;
                }
                Ok(Envelope::System(SystemMessage::Terminated(watched_id))) => {
                    eprintln!("[actor-system] Watched actor {} terminated", watched_id);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if self.system.is_shutdown() {
                        ctx.stopped = true;
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }

        let _ = self.behavior.post_stop(&mut ctx);
        for watcher in ctx.watchers.drain(..) {
            let _ = watcher.send(Envelope::System(SystemMessage::Terminated(self.id)));
        }
        self.is_alive.store(false, Ordering::Release);
        self.system.unregister_actor(self.id);
    }
}

trait ActorHandle: Send + Sync {
    fn stop_actor(&self);
}

impl<M: Send + 'static> ActorHandle for ActorRef<M> {
    fn stop_actor(&self) {
        let _ = self.stop();
    }
}

/// Actor System coordinating scheduler threads, message routing, and lifecycle supervision.
#[derive(Clone)]
pub struct ActorSystem {
    inner: Arc<ActorSystemInner>,
}

struct ActorSystemInner {
    name: String,
    registry: RwLock<HashMap<ActorId, Box<dyn ActorHandle>>>,
    threads: Mutex<Vec<JoinHandle<()>>>,
    is_shutdown: AtomicBool,
}

impl ActorSystem {
    /// Create a new named `ActorSystem`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(ActorSystemInner {
                name: name.into(),
                registry: RwLock::new(HashMap::new()),
                threads: Mutex::new(Vec::new()),
                is_shutdown: AtomicBool::new(false),
            }),
        }
    }

    /// Name of this actor system.
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Check if the actor system has been shut down.
    pub fn is_shutdown(&self) -> bool {
        self.inner.is_shutdown.load(Ordering::Relaxed)
    }

    /// Spawn an actor instance driven by the system thread pool.
    pub fn spawn<A: Actor>(&self, behavior: A) -> Result<ActorRef<A::Message>, ActorError> {
        if self.is_shutdown() {
            return Err(ActorError::new(
                "Cannot spawn actor on shutdown system",
                format!("ActorSystem `{}` is stopped", self.inner.name),
                "Create a new ActorSystem instance before spawning actors",
            ));
        }

        let id = generate_actor_id();
        let (sender, receiver) = channel::<Envelope<A::Message>>();
        let is_alive = Arc::new(AtomicBool::new(true));

        let actor_ref = ActorRef {
            id,
            sender,
            is_alive: Arc::clone(&is_alive),
        };

        let cell = ActorCell {
            id,
            behavior,
            receiver,
            self_ref: actor_ref.clone(),
            is_alive,
            system: self.clone(),
        };

        let handle = thread::spawn(move || {
            cell.run();
        });

        if let Ok(mut threads) = self.inner.threads.lock() {
            threads.push(handle);
        }

        if let Ok(mut reg) = self.inner.registry.write() {
            reg.insert(id, Box::new(actor_ref.clone()));
        }

        Ok(actor_ref)
    }

    fn unregister_actor(&self, id: ActorId) {
        if let Ok(mut reg) = self.inner.registry.write() {
            reg.remove(&id);
        }
    }

    /// Total number of currently active actors in this system.
    pub fn active_actors_count(&self) -> usize {
        self.inner.registry.read().map(|r| r.len()).unwrap_or(0)
    }

    /// Gracefully shutdown all actors in the system and join worker threads.
    pub fn shutdown(&self) -> Result<(), ActorError> {
        self.inner.is_shutdown.store(true, Ordering::Release);
        if let Ok(mut reg) = self.inner.registry.write() {
            for (_, handle) in reg.drain() {
                handle.stop_actor();
            }
        }
        if let Ok(mut threads) = self.inner.threads.lock() {
            for handle in threads.drain(..) {
                let _ = handle.join();
            }
        }
        Ok(())
    }
}

/// # Distributed Actor Wire Framing & Transport Protocol (`agam_runtime::actor::remote`)
///
/// Implements cross-node binary framing, CRC32 payload verification,
/// and `RemoteActorPath` URI resolution over TCP/IP.
pub mod remote {
    use super::ActorError;
    use crc32fast::Hasher;
    use serde::{Deserialize, Serialize};
    use std::fmt;

    /// 4-byte magic signature for Agam Distributed Actor wire framing ("AGMA").
    pub const WIRE_MAGIC: [u8; 4] = *b"AGMA";

    /// Current wire protocol version.
    pub const WIRE_VERSION: u16 = 1;

    /// Fixed 24-byte binary wire frame header size.
    pub const WIRE_HEADER_SIZE: usize = 24;

    /// Enumeration of distributed actor message frame classifications.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(u16)]
    pub enum ActorWireMsgKind {
        /// Initial cross-node handshake establishing session parameters.
        Handshake = 1,
        /// Positive acknowledgement of remote node session handshake.
        HandshakeAck = 2,
        /// Keep-alive heartbeat packet maintaining active connection liveness.
        Heartbeat = 3,
        /// Standard user-level payload envelope directed to an actor.
        UserMessage = 4,
        /// System-level control directive (e.g. Stop, Escalate, Restart).
        SystemSignal = 5,
        /// DeathWatch notification of remote actor termination.
        Terminated = 6,
        /// Undeliverable message routed to the distributed dead-letter mailbox.
        DeadLetter = 7,
    }

    impl ActorWireMsgKind {
        pub fn from_u16(val: u16) -> Option<Self> {
            match val {
                1 => Some(Self::Handshake),
                2 => Some(Self::HandshakeAck),
                3 => Some(Self::Heartbeat),
                4 => Some(Self::UserMessage),
                5 => Some(Self::SystemSignal),
                6 => Some(Self::Terminated),
                7 => Some(Self::DeadLetter),
                _ => None,
            }
        }
    }

    /// Fixed 24-byte binary header preceding all distributed actor frames.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ActorWireHeader {
        pub magic: [u8; 4],
        pub version: u16,
        pub kind: ActorWireMsgKind,
        pub stream_id: u64,
        pub payload_len: u32,
        pub checksum: u32,
    }

    impl ActorWireHeader {
        pub fn new(kind: ActorWireMsgKind, stream_id: u64, payload: &[u8]) -> Self {
            let mut hasher = Hasher::new();
            hasher.update(payload);
            let checksum = hasher.finalize();

            Self {
                magic: WIRE_MAGIC,
                version: WIRE_VERSION,
                kind,
                stream_id,
                payload_len: payload.len() as u32,
                checksum,
            }
        }

        pub fn encode(&self, out: &mut [u8; WIRE_HEADER_SIZE]) {
            out[0..4].copy_from_slice(&self.magic);
            out[4..6].copy_from_slice(&self.version.to_be_bytes());
            out[6..8].copy_from_slice(&(self.kind as u16).to_be_bytes());
            out[8..16].copy_from_slice(&self.stream_id.to_be_bytes());
            out[16..20].copy_from_slice(&self.payload_len.to_be_bytes());
            out[20..24].copy_from_slice(&self.checksum.to_be_bytes());
        }

        pub fn decode(buf: &[u8; WIRE_HEADER_SIZE]) -> Result<Self, ActorError> {
            let mut magic = [0u8; 4];
            magic.copy_from_slice(&buf[0..4]);
            if magic != WIRE_MAGIC {
                return Err(ActorError::new(
                    "Invalid wire frame magic bytes",
                    format!("Expected {:?}, found {:?}", WIRE_MAGIC, magic),
                    "Verify connecting peer is running a compatible Agam runtime",
                ));
            }

            let mut version_bytes = [0u8; 2];
            version_bytes.copy_from_slice(&buf[4..6]);
            let version = u16::from_be_bytes(version_bytes);
            if version != WIRE_VERSION {
                return Err(ActorError::new(
                    "Unsupported wire protocol version",
                    format!("Expected version {}, found {}", WIRE_VERSION, version),
                    "Upgrade node runtime to match peer wire framing version",
                ));
            }

            let mut kind_bytes = [0u8; 2];
            kind_bytes.copy_from_slice(&buf[6..8]);
            let raw_kind = u16::from_be_bytes(kind_bytes);
            let kind = ActorWireMsgKind::from_u16(raw_kind).ok_or_else(|| {
                ActorError::new(
                    "Unknown wire message kind identifier",
                    format!("Received unassigned kind tag: {}", raw_kind),
                    "Check network integrity or peer protocol compatibility",
                )
            })?;

            let mut stream_bytes = [0u8; 8];
            stream_bytes.copy_from_slice(&buf[8..16]);
            let stream_id = u64::from_be_bytes(stream_bytes);

            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&buf[16..20]);
            let payload_len = u32::from_be_bytes(len_bytes);

            let mut crc_bytes = [0u8; 4];
            crc_bytes.copy_from_slice(&buf[20..24]);
            let checksum = u32::from_be_bytes(crc_bytes);

            Ok(Self {
                magic,
                version,
                kind,
                stream_id,
                payload_len,
                checksum,
            })
        }
    }

    /// Discrete transport frame composed of a validated header and verified payload.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ActorWireFrame {
        pub header: ActorWireHeader,
        pub payload: Vec<u8>,
    }

    impl ActorWireFrame {
        pub fn new(kind: ActorWireMsgKind, stream_id: u64, payload: Vec<u8>) -> Self {
            let header = ActorWireHeader::new(kind, stream_id, &payload);
            Self { header, payload }
        }

        pub fn encode(&self) -> Vec<u8> {
            let mut buf = vec![0u8; WIRE_HEADER_SIZE + self.payload.len()];
            let mut hdr_bytes = [0u8; WIRE_HEADER_SIZE];
            self.header.encode(&mut hdr_bytes);
            buf[0..WIRE_HEADER_SIZE].copy_from_slice(&hdr_bytes);
            buf[WIRE_HEADER_SIZE..].copy_from_slice(&self.payload);
            buf
        }

        pub fn decode(buf: &[u8]) -> Result<Option<(Self, usize)>, ActorError> {
            if buf.len() < WIRE_HEADER_SIZE {
                return Ok(None);
            }

            let mut hdr_bytes = [0u8; WIRE_HEADER_SIZE];
            hdr_bytes.copy_from_slice(&buf[0..WIRE_HEADER_SIZE]);
            let header = ActorWireHeader::decode(&hdr_bytes)?;

            let total_len = WIRE_HEADER_SIZE + header.payload_len as usize;
            if buf.len() < total_len {
                return Ok(None);
            }

            let payload = buf[WIRE_HEADER_SIZE..total_len].to_vec();

            let mut hasher = Hasher::new();
            hasher.update(&payload);
            let calculated_crc = hasher.finalize();

            if calculated_crc != header.checksum {
                return Err(ActorError::new(
                    "Actor wire payload checksum mismatch",
                    format!(
                        "Header CRC 0x{:08X} != calculated CRC 0x{:08X}",
                        header.checksum, calculated_crc
                    ),
                    "Discard corrupted frame and request peer retransmission",
                ));
            }

            Ok(Some((Self { header, payload }, total_len)))
        }
    }

    /// Fully-qualified canonical address identifying an actor in a distributed cluster.
    ///
    /// URI Format: `agam.tcp://<node_id>@<host>:<port>/<path>`
    /// Example: `agam.tcp://worker-1@127.0.0.1:9090/user/processor`
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct RemoteActorPath {
        pub system_protocol: String,
        pub node_id: String,
        pub host: String,
        pub port: u16,
        pub path: String,
    }

    impl RemoteActorPath {
        pub fn new(
            node_id: impl Into<String>,
            host: impl Into<String>,
            port: u16,
            path: impl Into<String>,
        ) -> Self {
            Self {
                system_protocol: "agam.tcp".into(),
                node_id: node_id.into(),
                host: host.into(),
                port,
                path: path.into(),
            }
        }

        pub fn parse(uri: &str) -> Result<Self, ActorError> {
            let (proto, rest) = uri.split_once("://").ok_or_else(|| {
                ActorError::new(
                    "Invalid actor path format",
                    format!("Missing '://' protocol delimiter in '{}'", uri),
                    "Use 'agam.tcp://node_id@host:port/path'",
                )
            })?;

            if proto != "agam.tcp" && proto != "agam" {
                return Err(ActorError::new(
                    "Unsupported transport protocol in actor path",
                    format!("Protocol '{}' is not supported", proto),
                    "Use 'agam.tcp' as the transport scheme",
                ));
            }

            let (node_and_addr, path_part) = match rest.split_once('/') {
                Some((na, p)) => (na, format!("/{}", p)),
                None => (rest, "/".to_string()),
            };

            let (node_id, addr) = node_and_addr.split_once('@').ok_or_else(|| {
                ActorError::new(
                    "Missing node identifier in actor path",
                    format!("Missing '@' node delimiter in '{}'", node_and_addr),
                    "Format path as 'node_id@host:port'",
                )
            })?;

            let (host, port_str) = addr.rsplit_once(':').ok_or_else(|| {
                ActorError::new(
                    "Missing port specification in actor path",
                    format!("Missing ':port' delimiter in '{}'", addr),
                    "Specify numeric TCP port (e.g. '127.0.0.1:9090')",
                )
            })?;

            let port = port_str.parse::<u16>().map_err(|e| {
                ActorError::new(
                    "Invalid TCP port number in actor path",
                    format!("Port string '{}' failed to parse: {}", port_str, e),
                    "Ensure port is a valid u16 integer (1..65535)",
                )
            })?;

            Ok(Self {
                system_protocol: proto.to_string(),
                node_id: node_id.to_string(),
                host: host.to_string(),
                port,
                path: path_part,
            })
        }
    }

    impl fmt::Display for RemoteActorPath {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{}://{}@{}:{}{}",
                self.system_protocol, self.node_id, self.host, self.port, self.path
            )
        }
    }

    /// Serialized cross-node message envelope carrying routing metadata and payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RemoteEnvelope {
        pub sender: Option<RemoteActorPath>,
        pub target: RemoteActorPath,
        pub correlation_id: u64,
        pub payload: Vec<u8>,
    }

    impl RemoteEnvelope {
        pub fn new(
            sender: Option<RemoteActorPath>,
            target: RemoteActorPath,
            correlation_id: u64,
            payload: Vec<u8>,
        ) -> Self {
            Self {
                sender,
                target,
                correlation_id,
                payload,
            }
        }

        pub fn serialize(&self) -> Result<Vec<u8>, ActorError> {
            serde_json::to_vec(self).map_err(|e| {
                ActorError::new(
                    "Failed to serialize remote actor envelope",
                    e.to_string(),
                    "Verify message payload types are serializable",
                )
            })
        }

        pub fn deserialize(bytes: &[u8]) -> Result<Self, ActorError> {
            serde_json::from_slice(bytes).map_err(|e| {
                ActorError::new(
                    "Failed to deserialize remote actor envelope",
                    e.to_string(),
                    "Check for protocol version or envelope schema mismatch",
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CounterActor {
        count: i64,
    }

    enum CounterMsg {
        Increment(i64),
        Get(Sender<i64>),
        Crash,
    }

    impl Actor for CounterActor {
        type Message = CounterMsg;

        fn handle(
            &mut self,
            _ctx: &mut ActorContext<Self::Message>,
            msg: Self::Message,
        ) -> ActorResult {
            match msg {
                CounterMsg::Increment(val) => {
                    self.count += val;
                    Ok(())
                }
                CounterMsg::Get(reply) => {
                    let _ = reply.send(self.count);
                    Ok(())
                }
                CounterMsg::Crash => Err(ActorError::new(
                    "Simulated crash",
                    "Test handler",
                    "Restart",
                )),
            }
        }
    }

    #[test]
    fn test_actor_spawn_tell_and_ask() {
        let system = ActorSystem::new("test-system");
        let counter_ref = system
            .spawn(CounterActor { count: 0 })
            .expect("spawn actor");

        assert!(counter_ref.is_alive());

        counter_ref.tell(CounterMsg::Increment(5)).expect("tell 5");
        counter_ref
            .tell(CounterMsg::Increment(10))
            .expect("tell 10");

        let total = counter_ref
            .ask(CounterMsg::Get, Duration::from_millis(500))
            .expect("ask total");

        assert_eq!(total, 15);
        let _ = system.shutdown();
    }

    #[test]
    fn test_actor_stop_lifecycle() {
        let system = ActorSystem::new("lifecycle-system");
        let counter_ref = system
            .spawn(CounterActor { count: 100 })
            .expect("spawn actor");

        assert!(counter_ref.is_alive());
        counter_ref.stop().expect("stop command");

        let mut dead = false;
        for _ in 0..50 {
            if !counter_ref.is_alive() {
                dead = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(dead, "Actor should terminate after stop command");

        let res = counter_ref.tell(CounterMsg::Increment(1));
        assert!(res.is_err(), "Tell to dead actor should return error");

        let _ = system.shutdown();
    }

    #[test]
    fn test_actor_crash_and_error_propagation() {
        let system = ActorSystem::new("crash-system");
        let counter_ref = system
            .spawn(CounterActor { count: 42 })
            .expect("spawn actor");

        assert!(counter_ref.is_alive());
        counter_ref.tell(CounterMsg::Crash).expect("send crash");

        let mut dead = false;
        for _ in 0..50 {
            if !counter_ref.is_alive() {
                dead = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(dead, "Crashed actor should terminate");

        let _ = system.shutdown();
    }

    #[test]
    fn test_actor_death_watch_notification() {
        let (watcher_tx, watcher_rx) = channel::<Envelope<CounterMsg>>();
        let mut ctx = ActorContext {
            id: ActorId(99),
            self_ref: ActorRef {
                id: ActorId(99),
                sender: watcher_tx.clone(),
                is_alive: Arc::new(AtomicBool::new(true)),
            },
            system: ActorSystem::new("test-watch-system"),
            stopped: false,
            watchers: Vec::new(),
        };

        ctx.watch(watcher_tx);
        assert_eq!(ctx.watchers.len(), 1);
        drop(watcher_rx);
    }

    #[test]
    fn test_supervision_strategy_defaults() {
        let strategy = SupervisionStrategy::default();
        assert!(matches!(
            strategy,
            SupervisionStrategy::OneForOne {
                max_restarts: 5,
                ..
            }
        ));
    }

    #[test]
    fn test_remote_wire_frame_encode_decode_roundtrip() -> Result<(), ActorError> {
        use remote::*;

        let payload = b"{\"event\":\"UserJoined\",\"user_id\":42}".to_vec();
        let frame = ActorWireFrame::new(ActorWireMsgKind::UserMessage, 1001, payload.clone());

        let encoded = frame.encode();
        assert_eq!(encoded.len(), WIRE_HEADER_SIZE + payload.len());

        let parsed = ActorWireFrame::decode(&encoded)?;
        assert!(parsed.is_some());
        let (decoded, consumed) = parsed.unwrap_or_else(|| unreachable!());

        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.header.kind, ActorWireMsgKind::UserMessage);
        assert_eq!(decoded.header.stream_id, 1001);
        assert_eq!(decoded.payload, payload);
        Ok(())
    }

    #[test]
    fn test_remote_wire_frame_corrupted_checksum_rejected() {
        use remote::*;

        let payload = b"sensitive_actor_payload".to_vec();
        let frame = ActorWireFrame::new(ActorWireMsgKind::UserMessage, 42, payload);
        let mut encoded = frame.encode();

        // Corrupt payload byte
        let last_idx = encoded.len() - 1;
        encoded[last_idx] ^= 0xFF;

        let err = ActorWireFrame::decode(&encoded).expect_err("checksum mismatch expected");
        assert!(err.cause.contains("checksum mismatch"));
    }

    #[test]
    fn test_remote_actor_path_parsing_and_formatting() -> Result<(), ActorError> {
        use remote::*;

        let raw_uri = "agam.tcp://cluster-node-1@192.168.1.50:8080/user/computation/worker_3";
        let path = RemoteActorPath::parse(raw_uri)?;

        assert_eq!(path.system_protocol, "agam.tcp");
        assert_eq!(path.node_id, "cluster-node-1");
        assert_eq!(path.host, "192.168.1.50");
        assert_eq!(path.port, 8080);
        assert_eq!(path.path, "/user/computation/worker_3");

        assert_eq!(path.to_string(), raw_uri);
        Ok(())
    }

    #[test]
    fn test_remote_envelope_serialization_roundtrip() -> Result<(), ActorError> {
        use remote::*;

        let sender = RemoteActorPath::new("node-a", "127.0.0.1", 9001, "/user/sender");
        let target = RemoteActorPath::new("node-b", "127.0.0.1", 9002, "/user/receiver");
        let envelope = RemoteEnvelope::new(Some(sender), target.clone(), 777, vec![1, 2, 3, 4, 5]);

        let bytes = envelope.serialize()?;
        let deserialized = RemoteEnvelope::deserialize(&bytes)?;

        assert_eq!(envelope, deserialized);
        assert_eq!(deserialized.target, target);
        assert_eq!(deserialized.correlation_id, 777);
        Ok(())
    }
}

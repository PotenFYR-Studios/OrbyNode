//! OrbyNode realtime core — event bus, multiplexed gateway (ADR 009).

// ---------- Public API (implemented below) ----------

/// Stream topic, e.g. `terminal:1`, `agent:17`, `project:5`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Stream(String);

impl Stream {
    pub fn new(name: impl Into<String>) -> Self {
        Stream(name.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Priority classification (Plan §21): critical events must never be dropped
/// on slow clients; droppable ones are replaceable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// Attention/approvals/security — never dropped (P0–P2).
    Critical,
    /// Terminal bytes, metrics — replaceable, may be dropped with resync (P3+).
    Droppable,
}

/// A realtime event (ADR 009 envelope).
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub stream: Stream,
    pub etype: String,
    pub data: serde_json::Value,
    pub priority: Priority,
    /// Raw payload for binary streams (terminal output, §65). JSON events
    /// leave it empty; the gateway sends it as a binary WS frame.
    pub bytes: Vec<u8>,
}

/// A stamped event, as delivered to subscribers.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequenced {
    pub seq: u64,
    pub event: Event,
}

/// Errors surfaced by the gateway to a client session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// Subscription denied by authorization (Plan §60).
    Forbidden,
    /// The stream no longer exists.
    NoSuchStream,
}

/// Authorization hook (Plan §60/§105). M2 stub allows all; M4/M11 supply the
/// real ACL without protocol changes.
pub type AuthzFn = std::sync::Arc<dyn Fn(&str, &Stream) -> bool + Send + Sync>;

pub fn allow_all_authz() -> AuthzFn {
    std::sync::Arc::new(|_, _| true)
}

// ---------- EventBus ----------

/// Replay-ring sizing (in-memory only — Plan §74: transient state never hits
/// the database).
#[derive(Debug, Clone, Copy)]
pub struct ReplayConfig {
    /// Events retained per stream for resume (§62).
    pub per_stream: usize,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        ReplayConfig { per_stream: 1024 }
    }
}

/// Central in-process event bus (ADR 009): stamps a global monotonic sequence,
/// fans out to per-stream subscriber sets, retains a bounded replay ring.
#[derive(Clone)]
pub struct EventBus {
    inner: std::sync::Arc<BusInner>,
}

struct BusInner {
    seq: std::sync::atomic::AtomicU64,
    replay: ReplayConfig,
    streams: std::sync::Mutex<std::collections::HashMap<Stream, std::sync::Arc<StreamState>>>,
}

struct StreamState {
    /// Bounded replay ring (§62). Lock is held only for the push/snapshot.
    ring: std::sync::Mutex<std::collections::VecDeque<Sequenced>>,
    tx: tokio::sync::broadcast::Sender<Sequenced>,
}

impl EventBus {
    pub fn new(cfg: ReplayConfig) -> Self {
        EventBus {
            inner: std::sync::Arc::new(BusInner {
                seq: std::sync::atomic::AtomicU64::new(0),
                replay: cfg,
                streams: std::sync::Mutex::new(std::collections::HashMap::new()),
            }),
        }
    }

    fn stream_state(&self, stream: &Stream) -> std::sync::Arc<StreamState> {
        let mut map = self.inner.streams.lock().expect("bus streams poisoned");
        match map.get(stream) {
            Some(s) => std::sync::Arc::clone(s),
            None => {
                let (tx, _) = tokio::sync::broadcast::channel(4096);
                let state = std::sync::Arc::new(StreamState {
                    ring: std::sync::Mutex::new(std::collections::VecDeque::new()),
                    tx,
                });
                map.insert(stream.clone(), std::sync::Arc::clone(&state));
                state
            }
        }
    }

    /// Stamp, fan out, retain. Never blocks on slow consumers (§66): the
    /// broadcast channel handles lagging receivers, who detect the lag.
    pub fn publish(&self, event: Event) -> u64 {
        let seq = self
            .inner
            .seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        let sequenced = Sequenced { seq, event };
        let cap = self.inner.replay.per_stream;
        let state = self.stream_state(&sequenced.event.stream.clone());
        {
            let mut ring = state.ring.lock().expect("ring poisoned");
            while ring.len() >= cap.max(1) {
                ring.pop_front();
            }
            ring.push_back(sequenced.clone());
        }
        let _ = state.tx.send(sequenced); // Err = no subscribers, fine
        seq
    }

    /// Live subscription to a stream.
    pub fn subscribe(&self, stream: Stream) -> Receiver {
        let state = self.stream_state(&stream);
        Receiver {
            inner: state.tx.subscribe(),
        }
    }

    /// Replay events for `stream` after `after_seq`. `None` when history is
    /// gone (ring wrapped past that point) — client must resnapshot (§62).
    pub fn replay_from(&self, stream: &Stream, after_seq: u64) -> Option<Vec<Sequenced>> {
        let map = self.inner.streams.lock().expect("bus streams poisoned");
        let state = map.get(stream)?;
        let ring = state.ring.lock().expect("ring poisoned");
        // If the ring has advanced past `after_seq`, resume is impossible.
        if let Some(oldest) = ring.front().map(|e| e.seq)
            && after_seq + 1 < oldest
        {
            return None;
        }
        Some(ring.iter().filter(|e| e.seq > after_seq).cloned().collect())
    }

    pub fn current_seq(&self) -> u64 {
        self.inner.seq.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Live per-stream receiver (newtype over broadcast).
pub struct Receiver {
    inner: tokio::sync::broadcast::Receiver<Sequenced>,
}

impl Receiver {
    /// Next event; survives broadcast lag by skipping missed slots (the
    /// client's session layer detects gaps via seq and resyncs).
    pub async fn recv(&mut self) -> Option<Sequenced> {
        loop {
            match self.inner.recv().await {
                Ok(ev) => return Some(ev),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    }
}

// ---------- ClientSession ----------

/// Shared state between session handle and its pumps.
struct Shared {
    queue: tokio::sync::Mutex<std::collections::VecDeque<Sequenced>>,
    overflowed: std::sync::Mutex<std::collections::HashSet<Stream>>,
}

/// One connected client: bounded outgoing queue, per-stream subscriptions,
/// overflow tracking. Publishers never block on a slow session (Plan §66).
#[derive(Clone)]
pub struct ClientSession {
    user: std::sync::Arc<String>,
    bus: EventBus,
    queue_cap: usize,
    shared: std::sync::Arc<Shared>,
    subs: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<Stream>>>,
    authz: AuthzFn,
}

impl ClientSession {
    pub fn new(user: &str, bus: EventBus, queue_cap: usize, authz: AuthzFn) -> Self {
        ClientSession {
            user: std::sync::Arc::new(user.to_owned()),
            bus,
            queue_cap,
            shared: std::sync::Arc::new(Shared {
                queue: tokio::sync::Mutex::new(std::collections::VecDeque::new()),
                overflowed: std::sync::Mutex::new(std::collections::HashSet::new()),
            }),
            subs: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            authz,
        }
    }

    /// Subscribe this session to a stream: retained history is replayed into
    /// the queue first, then a live pump attaches (replay-then-attach keeps
    /// seq order, §62).
    pub fn sub(&self, stream: Stream) -> Result<(), ClientError> {
        {
            let subs = self.subs.lock().expect("subs poisoned");
            if subs.contains(&stream) {
                return Ok(());
            }
        }
        if !(self.authz)(&self.user, &stream) {
            return Err(ClientError::Forbidden);
        }
        // Retained replay before the pump attaches.
        if let Some(replay) = self.bus.replay_from(&stream, 0) {
            for ev in replay {
                self.push_replay(ev);
            }
        }
        self.subs
            .lock()
            .expect("subs poisoned")
            .insert(stream.clone());
        let rx = self.bus.subscribe(stream.clone());
        tokio::spawn(pump(
            rx,
            std::sync::Arc::clone(&self.shared),
            self.queue_cap,
            stream,
        ));
        Ok(())
    }

    pub fn unsub(&self, stream: &Stream) {
        self.subs.lock().expect("subs poisoned").remove(stream);
    }

    /// Non-blocking pull of the next queued event (gateway + tests).
    pub fn try_recv(&self) -> Option<Sequenced> {
        match self.shared.queue.try_lock() {
            Ok(mut q) => q.pop_front(),
            Err(_) => None,
        }
    }

    /// Streams whose droppable events overflowed and need resync (§66).
    pub fn overflowed_streams(&self) -> std::collections::HashSet<Stream> {
        self.shared
            .overflowed
            .lock()
            .expect("overflow poisoned")
            .clone()
    }

    fn push_replay(&self, ev: Sequenced) {
        // Replay inserts are bounded; overflow during replay defers to the
        // same marker logic via the queue bound.
        if let Ok(mut q) = self.shared.queue.try_lock()
            && q.len() < self.queue_cap
        {
            q.push_back(ev);
        }
    }
}

/// Live pump: moves bus events into the session queue, enforcing the bound
/// and marking droppable overflow (§66). Critical events always enqueue.
async fn pump(mut rx: Receiver, shared: std::sync::Arc<Shared>, cap: usize, stream: Stream) {
    loop {
        let Some(ev) = rx.recv().await else { return };
        let mut q = shared.queue.lock().await;
        if q.len() >= cap {
            let droppable = ev.event.priority == Priority::Droppable;
            if droppable {
                shared
                    .overflowed
                    .lock()
                    .expect("overflow poisoned")
                    .insert(stream.clone());
                // Drop this event; a resync will recover state.
            } else {
                // Critical: make room by dropping the oldest droppable event.
                if let Some(pos) = q
                    .iter()
                    .position(|e| e.event.priority == Priority::Droppable)
                {
                    q.remove(pos);
                }
                q.push_back(ev);
            }
        } else {
            q.push_back(ev);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Yield until the pump task has landed everything currently in the bus.
    async fn settle() {
        for _ in 0..100 {
            tokio::task::yield_now().await;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    // ---- EventBus ----

    #[tokio::test]
    async fn published_event_reaches_all_subscribers_once() {
        let bus = EventBus::new(ReplayConfig::default());
        let mut rx1 = bus.subscribe(Stream::new("terminal:1"));
        let mut rx2 = bus.subscribe(Stream::new("terminal:1"));
        bus.publish(Event {
            stream: Stream::new("terminal:1"),
            etype: "terminal.output".into(),
            data: serde_json::json!({"n": 1}),
            priority: Priority::Droppable,
            bytes: Vec::new(),
        });
        let e1 = tokio::time::timeout(Duration::from_secs(2), rx1.recv())
            .await
            .unwrap()
            .unwrap();
        let e2 = tokio::time::timeout(Duration::from_secs(2), rx2.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(e1.event.data, serde_json::json!({"n": 1}));
        assert_eq!(e1.seq, e2.seq, "same global sequence stamped");
    }

    #[tokio::test]
    async fn non_subscribed_stream_not_delivered() {
        let bus = EventBus::new(ReplayConfig::default());
        let mut rx = bus.subscribe(Stream::new("terminal:1"));
        bus.publish(Event {
            stream: Stream::new("terminal:2"),
            etype: "terminal.output".into(),
            data: serde_json::json!({"n": 1}),
            priority: Priority::Droppable,
            bytes: Vec::new(),
        });
        let got = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await;
        assert!(got.is_err(), "must not receive events for other streams");
    }

    #[tokio::test]
    async fn replay_returns_events_after_sequence() {
        let cfg = ReplayConfig { per_stream: 64 };
        let bus = EventBus::new(cfg);
        let stream = Stream::new("agent:5");
        for i in 0..10u64 {
            bus.publish(Event {
                stream: stream.clone(),
                etype: "agent.state".into(),
                data: serde_json::json!({"i": i}),
                priority: Priority::Critical,
                bytes: Vec::new(),
            });
        }
        let replayed = bus.replay_from(&stream, 4).expect("history retained");
        assert_eq!(replayed.len(), 6, "events after seq 4");
        assert_eq!(replayed[0].seq, 5);
        assert_eq!(replayed[0].event.data, serde_json::json!({"i": 4}));
    }

    #[tokio::test]
    async fn replay_history_gone_when_ring_wrapped() {
        let cfg = ReplayConfig { per_stream: 8 };
        let bus = EventBus::new(cfg);
        let stream = Stream::new("agent:5");
        for i in 0..40u64 {
            bus.publish(Event {
                stream: stream.clone(),
                etype: "agent.state".into(),
                data: serde_json::json!({"i": i}),
                priority: Priority::Critical,
                bytes: Vec::new(),
            });
        }
        // seq 5 is long gone: replay must report a gap.
        assert!(
            bus.replay_from(&stream, 5).is_none(),
            "wrapped ring => resync"
        );
    }

    // ---- ClientSession: bounded queue + overflow policy ----

    fn ev(stream: &str, n: u64) -> Event {
        Event {
            stream: Stream::new(stream),
            etype: "terminal.output".into(),
            data: serde_json::json!({"n": n}),
            priority: Priority::Droppable,
            bytes: Vec::new(),
        }
    }

    #[tokio::test]
    async fn slow_client_receives_in_order_until_bound() {
        let bus = EventBus::new(ReplayConfig::default());
        let session = ClientSession::new("c1", bus.clone(), 8, allow_all_authz());
        session.sub(Stream::new("terminal:1")).unwrap();
        for i in 0..6u64 {
            bus.publish(ev("terminal:1", i));
        }
        settle().await;
        let mut seqs = Vec::new();
        while let Some(ev) = session.try_recv() {
            seqs.push(ev.seq);
        }
        let mut sorted = seqs.clone();
        sorted.sort_unstable();
        assert_eq!(seqs, sorted, "delivery in order");
        assert_eq!(seqs.len(), 6);
    }

    #[tokio::test]
    async fn droppable_stream_overflow_is_reported_not_silent() {
        let bus = EventBus::new(ReplayConfig::default());
        let session = ClientSession::new("c1", bus.clone(), 4, allow_all_authz());
        session.sub(Stream::new("terminal:1")).unwrap();
        // Flood far beyond the bound; the client never blocks the publisher.
        for i in 0..64u64 {
            bus.publish(ev("terminal:1", i));
        }
        settle().await;
        // Drain what fits.
        while session.try_recv().is_some() {}
        // The overflow must be visible to the client (Plan §66: request resync).
        assert!(
            session
                .overflowed_streams()
                .contains(&Stream::new("terminal:1")),
            "overflow must be reported"
        );
    }

    #[tokio::test]
    async fn critical_events_survive_overflow() {
        let bus = EventBus::new(ReplayConfig::default());
        let session = ClientSession::new("c1", bus.clone(), 2, allow_all_authz());
        session.sub(Stream::new("terminal:1")).unwrap();
        session.sub(Stream::new("attention:user42")).unwrap();
        for i in 0..32u64 {
            bus.publish(ev("terminal:1", i)); // droppable flood
        }
        bus.publish(Event {
            stream: Stream::new("attention:user42"),
            etype: "attention.created".into(),
            data: serde_json::json!({"what": "approval"}),
            priority: Priority::Critical,
            bytes: Vec::new(),
        });
        settle().await;
        let mut saw_attention = false;
        while let Some(e) = session.try_recv() {
            if e.event.stream.as_str() == "attention:user42" {
                saw_attention = true;
                assert_eq!(e.event.data, serde_json::json!({"what": "approval"}));
            }
        }
        assert!(
            saw_attention,
            "critical events are never dropped (Plan §21)"
        );
    }

    // ---- Authorization ----

    #[tokio::test]
    async fn unauthorized_subscription_is_denied() {
        let bus = EventBus::new(ReplayConfig::default());
        let authz: AuthzFn = std::sync::Arc::new(|user, _s| user != "viewer");
        let session = ClientSession::new("viewer", bus.clone(), 8, authz);
        let res = session.sub(Stream::new("terminal:1"));
        assert_eq!(res, Err(ClientError::Forbidden));
    }
}

//! Multiplexed realtime gateway (ADR 009): one WebSocket per tab.
//!
//! Client→server: JSON ops (`hello`, `sub`, `unsub`).
//! Server→client: JSON event envelopes + ops (`snapshot`, `resume`,
//! `resync_required`, `stream_overflow`, `error`).

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use orbynode_realtime::{Event, EventBus, Priority, Sequenced, Stream};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws_upgrade))
}

async fn ws_upgrade(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| run_gateway(state.clone(), socket))
}

/// One JSON op from the client.
#[derive(serde::Deserialize)]
struct ClientOp {
    op: String,
    #[serde(default)]
    stream: Option<String>,
    #[serde(default)]
    last_seq: Option<u64>,
}

/// One server message: either an op reply or a sequenced event.
#[derive(serde::Serialize)]
struct ServerMsg {
    op: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<String>,
    #[serde(flatten)]
    event: Option<Envelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct Envelope {
    v: u32,
    seq: u64,
    stream: String,
    etype: String,
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    data: serde_json::Value,
    priority: &'static str,
}

fn envelope_of(ev: &Sequenced) -> Envelope {
    Envelope {
        v: 1,
        seq: ev.seq,
        stream: ev.event.stream.as_str().to_owned(),
        etype: ev.event.etype.clone(),
        data: ev.event.data.clone(),
        priority: match ev.event.priority {
            Priority::Critical => "critical",
            Priority::Droppable => "droppable",
        },
    }
}

fn msg_event(ev: &Sequenced) -> ServerMsg {
    ServerMsg {
        op: None,
        stream: None,
        event: Some(envelope_of(ev)),
        detail: None,
    }
}

fn msg_op(op: &str, stream: Option<String>, detail: Option<serde_json::Value>) -> ServerMsg {
    ServerMsg {
        op: Some(op.to_owned()),
        stream,
        event: None,
        detail,
    }
}

/// Forward a `Sequenced` into the client channel, tagging overflow so the
/// pump can instruct a resubscribe (§66).
enum ToClient {
    Event(Sequenced),
    Overflow(Stream),
}

/// Binary stream frame (§65): `[u32 be stream_len][stream][payload]`.
/// Terminal bytes never go through JSON escaping.
fn binary_frame(stream: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + stream.len() + payload.len());
    out.extend_from_slice(&(stream.len() as u32).to_be_bytes());
    out.extend_from_slice(stream.as_bytes());
    out.extend_from_slice(payload);
    out
}

async fn run_gateway(state: AppState, socket: WebSocket) {
    let (mut sink, mut source) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ToClient>(1024);

    // Outbound pump.
    let pump = tokio::spawn(async move {
        while let Some(item) = rx.recv().await {
            let frame = match &item {
                ToClient::Event(ev) if !ev.event.bytes.is_empty() => {
                    Message::Binary(binary_frame(ev.event.stream.as_str(), &ev.event.bytes).into())
                }
                _ => {
                    let msg = match item {
                        ToClient::Event(ev) => msg_event(&ev),
                        ToClient::Overflow(stream) => {
                            msg_op("stream_overflow", Some(stream.as_str().to_owned()), None)
                        }
                    };
                    let Ok(json) = serde_json::to_string(&msg) else {
                        continue;
                    };
                    Message::Text(json.into())
                }
            };
            if sink.send(frame).await.is_err() {
                break;
            }
        }
    });

    let bus: Arc<EventBus> = state.realtime.clone();
    // hello is mandatory before any sub (ADR 009 lifecycle).
    let mut helloed = false;

    while let Some(Ok(msg)) = source.next().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Binary(_) | Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => break,
        };
        let Ok(op) = serde_json::from_str::<ClientOp>(&text) else {
            let _ = tx
                .send(ToClient::Event(Sequenced {
                    seq: 0,
                    event: Event {
                        stream: Stream::new("meta"),
                        etype: "error".into(),
                        data: serde_json::json!({"what": "unparseable op"}),
                        priority: Priority::Critical,
                        bytes: Vec::new(),
                    },
                }))
                .await;
            continue;
        };

        match op.op.as_str() {
            "hello" => {
                helloed = true;
                // Fresh session or resume-from-seq (§62). Snapshots for
                // durable entities arrive with M3+; for now a snapshot
                // acknowledges the session and current global sequence.
                if let Some(last) = op.last_seq {
                    let _ = tx
                        .send(ToClient::Event(Sequenced {
                            seq: bus.current_seq(),
                            event: Event {
                                stream: Stream::new("meta"),
                                etype: if last < bus.current_seq() {
                                    "resume.ack"
                                } else {
                                    "snapshot"
                                }
                                .into(),
                                data: serde_json::json!({"seq": bus.current_seq()}),
                                priority: Priority::Critical,
                                bytes: Vec::new(),
                            },
                        }))
                        .await;
                } else {
                    let _ = tx
                        .send(ToClient::Event(Sequenced {
                            seq: bus.current_seq(),
                            event: Event {
                                stream: Stream::new("meta"),
                                etype: "snapshot".into(),
                                data: serde_json::json!({"seq": bus.current_seq()}),
                                priority: Priority::Critical,
                                bytes: Vec::new(),
                            },
                        }))
                        .await;
                }
            }
            "sub" if helloed => {
                let Some(name) = op.stream.clone() else {
                    continue;
                };
                let stream = Stream::new(name.clone());
                // Subscribe + replay retained history into the channel (§62).
                let session = state.gateway_session(&stream);
                match session {
                    Ok(()) => {
                        // Replay-then-live: retained events first.
                        if let Some(replay) = bus.replay_from(&stream, 0) {
                            for ev in replay {
                                if tx.send(ToClient::Event(ev)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        // Live attach: bus-level subscription forwards here.
                        spawn_live_forwarder(
                            bus.clone(),
                            stream.clone(),
                            state.clone(),
                            tx.clone(),
                        );
                        let _ = tx
                            .send(ToClient::Event(Sequenced {
                                seq: bus.current_seq(),
                                event: Event {
                                    stream: Stream::new("meta"),
                                    etype: "sub.ack".into(),
                                    data: serde_json::json!({"stream": name}),
                                    priority: Priority::Critical,
                                    bytes: Vec::new(),
                                },
                            }))
                            .await;
                    }
                    Err(_e) => {
                        let _ = tx.send(ToClient::Overflow(Stream::new(name))).await;
                    }
                }
            }
            "unsub" if helloed => {
                if let Some(name) = op.stream {
                    state.gateway_unsub(&Stream::new(name));
                }
            }
            _ => {}
        }
    }

    drop(tx);
    let _ = pump.await;
}

/// Forward live events of one stream into the client channel. Task-per-stream
/// is the M2 simplification; a shared selector task lands with profiling in
/// M18 if task counts demand it.
fn spawn_live_forwarder(
    bus: Arc<EventBus>,
    stream: Stream,
    state: AppState,
    tx: tokio::sync::mpsc::Sender<ToClient>,
) {
    if !state.gateway_track(&stream) {
        return;
    }
    tokio::spawn(async move {
        let mut rx = bus.subscribe(stream.clone());
        while let Some(ev) = rx.recv().await {
            if tx.send(ToClient::Event(ev)).await.is_err() {
                break; // client gone
            }
        }
    });
}

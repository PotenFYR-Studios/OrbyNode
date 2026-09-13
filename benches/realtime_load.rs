//! Deterministic realtime load harness (Milestone 18, Plan §141).

use orbynode_realtime::{Event, EventBus, Priority, ReplayConfig, Stream};
use std::time::Instant;

const USERS: usize = 100;
const STREAMS: usize = 100;
const EVENTS_PER_STREAM: usize = 32;

fn event(stream: usize) -> Event {
    Event {
        stream: Stream::new(format!("terminal:{stream}")),
        etype: "terminal.output".into(),
        data: serde_json::json!({}),
        priority: Priority::Droppable,
        bytes: vec![b'x'; 128],
    }
}

#[tokio::main]
async fn main() {
    let bus = EventBus::new(ReplayConfig {
        per_stream: EVENTS_PER_STREAM,
    });
    let mut receivers = Vec::with_capacity(USERS);
    for stream in 0..STREAMS {
        let stream = Stream::new(format!("terminal:{stream}"));
        for _ in 0..USERS / STREAMS.min(USERS) {
            receivers.push(bus.subscribe(stream.clone()));
        }
    }

    let started = Instant::now();
    for generation in 0..EVENTS_PER_STREAM {
        for stream in 0..STREAMS {
            bus.publish(event(stream));
        }
        let _ = generation;
    }
    let publish_elapsed = started.elapsed();

    let mut delivered = 0usize;
    for rx in &mut receivers {
        while rx.try_recv().is_ok() {
            delivered += 1;
        }
    }
    let elapsed = started.elapsed();

    println!(
        "users={} streams={} events={} publish_ms={} total_ms={} delivered={}",
        USERS,
        STREAMS,
        STREAMS * EVENTS_PER_STREAM,
        publish_elapsed.as_millis(),
        elapsed.as_millis(),
        delivered
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_harness_completes_under_budget() {
        let bus = EventBus::new(ReplayConfig {
            per_stream: EVENTS_PER_STREAM,
        });
        let started = Instant::now();
        for generation in 0..EVENTS_PER_STREAM {
            for stream in 0..STREAMS {
                let mut candidate = event(stream);
                candidate.data = serde_json::json!({"generation": generation});
                bus.publish(candidate);
            }
        }
        assert!(
            started.elapsed().as_secs_f64() < 5.0,
            "event burst exceeded budget"
        );
    }
}

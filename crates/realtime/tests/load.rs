//! Milestone 18 realtime scale smoke (Plan §141).

use orbynode_realtime::{Event, EventBus, Priority, ReplayConfig, Stream};
use std::time::{Duration, Instant};

#[tokio::test]
async fn one_hundred_subscribers_remain_bounded() {
    let bus = EventBus::new(ReplayConfig { per_stream: 1_024 });
    let stream = Stream::new("terminal:scale");
    let mut receivers = Vec::new();
    for _ in 0..100 {
        receivers.push(bus.subscribe(stream.clone()));
    }

    let started = Instant::now();
    for generation in 0..32 {
        bus.publish(Event {
            stream: stream.clone(),
            etype: "terminal.output".into(),
            data: serde_json::json!({"generation": generation}),
            priority: Priority::Droppable,
            bytes: vec![b'x'; 128],
        });
    }

    let mut delivered = 0;
    for receiver in &mut receivers {
        while let Ok(event) = receiver.inner().try_recv() {
            let _ = event;
            delivered += 1;
        }
    }

    assert_eq!(delivered, 100 * 32);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "broadcast burst exceeded five second budget"
    );
}

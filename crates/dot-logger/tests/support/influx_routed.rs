#![allow(dead_code)]
use dot_logger::{adapters::influxdb::*, completion::*, config::*, routing::*};
use std::time::Duration;
pub fn plan() -> ValidatedConfig {
    let mut c = super::support::plan().config().clone();
    c.memory.total_bytes += 1024 * 1024;
    c.memory.external_bytes = 1024 * 1024;
    SessionBuilder::new(c)
        .validate(&Default::default())
        .unwrap()
}
pub fn route(db: bool) -> RoutingConfig {
    let mut r = super::support::routing();
    r.targets[1].queue_slots = 1;
    r.targets[1].pending_slots = 0;
    r.targets[1].retained_bytes = 8;
    r.routes[0].targets = if db { vec![10, 20] } else { vec![10] };
    r
}
pub fn tracking() -> CompletionConfig {
    let mut c = super::support::tracking(16);
    c.targets[1].supported = Boundaries::DATABASE;
    c.targets[1].completion = Boundary::DatabaseAcknowledged;
    c
}
pub fn schemas() -> Vec<ProjectionSchema> {
    vec![ProjectionSchema {
        channel_id: 7,
        schema_id: 99,
        schema_version: 1,
        encoding: ProjectionEncoding::I64Le,
    }]
}
pub fn config(endpoint: String) -> Config {
    Config {
        endpoint,
        organization: "test".into(),
        bucket: "isolated".into(),
        token: "secret-test-only".into(),
        session_id: 42,
        measurement: "routed".into(),
        device: "robot".into(),
        channels: vec![Channel {
            id: 7,
            kind: Kind::Integer,
        }],
        collision_policy: CollisionPolicy::RejectNonIncreasing,
        max_points: 8,
        max_batch_bytes: 4096,
        max_event_bytes: 64,
        max_attempts: 1,
        request_timeout: Duration::from_secs(2),
        retry_delay: Duration::from_millis(1),
        ttl: Duration::from_secs(3),
    }
}
pub fn runtime<T>(f: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

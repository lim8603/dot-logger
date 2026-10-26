//! TC-001 dependency compatibility probes, not logger runtime or performance tests.

use std::{borrow::Cow, collections::BTreeMap, io::Cursor, sync::Arc};

#[test]
fn mcap_round_trip_without_network_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    mcap_round_trip(None)
}

#[cfg(feature = "compression-zstd")]
#[test]
fn mcap_round_trip_with_opt_in_zstd() -> Result<(), Box<dyn std::error::Error>> {
    mcap_round_trip(Some(mcap::Compression::Zstd))
}

fn mcap_round_trip(
    compression: Option<mcap::Compression>,
) -> Result<(), Box<dyn std::error::Error>> {
    let message = mcap::Message {
        channel: Arc::new(mcap::Channel {
            id: 1,
            topic: "sensor/status".into(),
            schema: None,
            message_encoding: "application/octet-stream".into(),
            metadata: BTreeMap::new(),
        }),
        sequence: 1,
        log_time: 100,
        publish_time: 90,
        data: Cow::Borrowed(b"ready"),
    };
    let mut output = Cursor::new(Vec::new());
    let mut writer = mcap::WriteOptions::new()
        .compression(compression)
        .create(&mut output)?;
    writer.write(&message)?;
    writer.finish()?;
    drop(writer);
    let messages = mcap::MessageStream::new(output.get_ref())?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(messages, vec![message]);
    Ok(())
}

#[test]
fn rtrb_capacity_and_protobuf_u64_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    use prost::Message;

    // A compatibility probe, not the production envelope schema.
    #[derive(PartialEq, prost::Message)]
    struct Probe {
        #[prost(uint64, tag = "1")]
        sequence: u64,
    }

    let record = Probe { sequence: u64::MAX };
    let (mut producer, mut consumer) = rtrb::RingBuffer::new(1);
    assert!(producer.push(record.encode_to_vec()).is_ok());
    assert!(producer.push(vec![]).is_err());
    let encoded = consumer.pop()?;
    assert_eq!(Probe::decode(encoded.as_slice())?, record);
    Ok(())
}

#[cfg(feature = "remote")]
#[test]
fn remote_transport_is_available_without_exposing_wire_types() {
    let endpoint = tonic::transport::Endpoint::from_static("http://127.0.0.1:50051");
    assert_eq!(endpoint.uri().host(), Some("127.0.0.1"));
}

#[cfg(feature = "influxdb")]
#[test]
fn influxdb_http_dependency_builds_without_starting_io() -> Result<(), reqwest::Error> {
    let request = reqwest::Client::new()
        .post("http://127.0.0.1:8086/api/v2/write")
        .build()?;
    assert_eq!(request.url().path(), "/api/v2/write");
    Ok(())
}

#[cfg(feature = "tracing-adapter")]
#[test]
fn ordinary_logging_dependencies_are_available() {
    assert_eq!(tracing::Level::INFO.as_str(), "INFO");
    assert_eq!(log::Level::Info.as_str(), "INFO");
}

//! Production raw capture -> Router -> DB completion and independent MCAP output.
#![cfg(feature = "influxdb")]
#![allow(clippy::unwrap_used)]
#[path = "support/influx_routed.rs"]
mod fixture;
mod support;
use dot_logger::{
    adapters::influxdb::*, capture::*, completion::*, pool::RawRecordOutcome, query::QueryBudget,
    routing::*, storage::*,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::AtomicBool,
    time::Instant,
};
fn peer(response: &'static str) -> (String, std::thread::JoinHandle<String>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    (
        url,
        std::thread::spawn(move || {
            let end = Instant::now() + std::time::Duration::from_secs(4);
            let mut s = loop {
                if let Ok((s, _)) = l.accept() {
                    break s;
                }
                assert!(Instant::now() < end);
                std::thread::sleep(std::time::Duration::from_millis(1));
            };
            s.set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut data = Vec::new();
            let mut b = [0; 1024];
            loop {
                let n = s.read(&mut b).unwrap();
                assert!(n > 0);
                data.extend_from_slice(&b[..n]);
                if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let h = String::from_utf8_lossy(&data[..i]).to_lowercase();
                    let n: usize = h
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if data.len() >= i + 4 + n {
                        break;
                    }
                }
            }
            if !response.is_empty() {
                s.write_all(response.as_bytes()).unwrap();
            }
            String::from_utf8(data).unwrap()
        }),
    )
}
const ACK: &str = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
fn capture(p: &mut Producer<'_>, bytes: &[u8]) -> CaptureReceipt {
    let mut loan = p.try_acquire(bytes.len(), Importance::Normal).unwrap();
    loan.payload_mut().copy_from_slice(bytes);
    match p.try_record_raw(support::meta(), loan).unwrap() {
        RawRecordOutcome::Accepted(r) => r,
        _ => panic!("skipped"),
    }
}
#[test]
fn routed_ack_keeps_original_until_io_close_and_explicit_completion_publication() {
    let (url, server) = peer(ACK);
    let plan = fixture::plan();
    let route = fixture::route(true);
    let budget = QueryBudget::new(&plan);
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        fixture::tracking(),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        let mut db = RoutedWriter::new(
            Writer::new(&budget, fixture::config(url)).unwrap(),
            completion,
            20,
            fixture::schemas(),
        )
        .unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let receipt = capture(&mut p, &123i64.to_le_bytes());
            dispatch.turn(Instant::now(), &mut [None; 8]).unwrap();
            let file = outputs[0].try_next(Instant::now()).unwrap().unwrap();
            let guard = outputs[1].try_next(Instant::now()).unwrap().unwrap();
            file.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(file);
            let result = db.write_delivery(guard, &AtomicBool::new(false));
            assert_eq!(result.result().unwrap().outcome, Outcome::Acknowledged);
            assert_eq!(result.receipt(), receipt);
            assert_eq!(
                completion
                    .poll(receipt, 20, Boundary::DatabaseAcknowledged)
                    .unwrap()
                    .outcome,
                QueryOutcome::Pending
            );
            assert_eq!(
                dispatch
                    .turn(Instant::now(), &mut [None; 8])
                    .unwrap()
                    .reclaimed,
                0
            );
            assert_eq!(dispatch.snapshot().raw_retained_bytes, 8);
            result.publish().unwrap();
            result.publish().unwrap();
            assert_eq!(
                completion
                    .poll(receipt, 20, Boundary::DatabaseAcknowledged)
                    .unwrap()
                    .outcome,
                QueryOutcome::Complete
            );
            let terminal = result.into_delivery();
            assert!(
                terminal.is_err(),
                "attempted IO must never recover a resubmittable guard"
            );
            drop(terminal);
            assert_eq!(
                dispatch
                    .turn(Instant::now(), &mut [None; 8])
                    .unwrap()
                    .reclaimed,
                1
            );
        });
        drop(db);
    });
    assert_eq!(budget.used_bytes(), 0);
    let request = server.join().unwrap();
    assert!(request.contains(
        "integer=123i,session=\"0000000000000000000000000000002a\",producer=0u,sequence=1u 2\n"
    ));
}

#[test]
fn routed_failures_and_preflight_cancel_publish_distinct_terminal_states() {
    for case in 0..6 {
        let response = match case {
            0 => "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            1 => {
                "HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            }
            _ => "",
        };
        let (url, server) = if case < 3 {
            let (u, p) = peer(response);
            (u, Some(p))
        } else {
            ("http://127.0.0.1:1".into(), None)
        };
        let plan = fixture::plan();
        let route = fixture::route(true);
        let budget = QueryBudget::new(&plan);
        let mut owner = CaptureOwner::new_tracked(
            &plan,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &route,
            fixture::tracking(),
        )
        .unwrap();
        owner.scope(|mut scope| {
            let completion = scope.completion().unwrap();
            let (mut p, a) = scope.next().unwrap();
            let (_, b) = scope.next().unwrap();
            let mut router = Router::new(&plan, vec![a, b], route).unwrap();
            let mut schemas = fixture::schemas();
            if case == 4 {
                schemas[0].schema_id = 100;
            }
            let mut db = RoutedWriter::new(
                Writer::new(&budget, fixture::config(url)).unwrap(),
                completion,
                20,
                schemas,
            )
            .unwrap();
            router.scope(|mut dispatch, mut outputs| {
                let bytes = if case == 5 || case == 3 {
                    vec![1]
                } else {
                    1i64.to_le_bytes().to_vec()
                };
                let receipt = capture(&mut p, &bytes);
                dispatch.turn(Instant::now(), &mut [None; 8]).unwrap();
                let file = outputs[0].try_next(Instant::now()).unwrap().unwrap();
                let guard = outputs[1].try_next(Instant::now()).unwrap().unwrap();
                file.report(Update::Confirmed(Boundary::Written)).unwrap();
                drop(file);
                let result = db.write_delivery(guard, &AtomicBool::new(case == 3));
                let expected = match case {
                    0 => QueryOutcome::Failed(FailureReason::IoFailure),
                    1 | 2 => QueryOutcome::Unknown,
                    3 => QueryOutcome::Dropped(DropReason::Cancelled),
                    _ => QueryOutcome::Failed(FailureReason::AdapterFailure),
                };
                result.publish().unwrap();
                assert_eq!(
                    completion
                        .poll(receipt, 20, Boundary::DatabaseAcknowledged)
                        .unwrap()
                        .outcome,
                    expected
                );
                assert_eq!(
                    completion
                        .poll(receipt, 10, Boundary::Written)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Complete
                );
                drop(result);
                assert_eq!(
                    dispatch
                        .turn(Instant::now(), &mut [None; 8])
                        .unwrap()
                        .reclaimed,
                    1
                );
            });
            drop(db);
        });
        assert_eq!(budget.used_bytes(), 0);
        if let Some(server) = server {
            server.join().unwrap();
        }
    }
}
#[test]
fn foreign_target_and_nested_runtime_return_guard_without_altering_completion() {
    let (url, server) = peer(ACK);
    let plan = fixture::plan();
    let route = fixture::route(true);
    let budget = QueryBudget::new(&plan);
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        fixture::tracking(),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        assert!(matches!(
            RoutedWriter::new(
                Writer::new(&budget, fixture::config(url.clone())).unwrap(),
                completion,
                10,
                fixture::schemas()
            ),
            Err(ProjectionError::InvalidConfig)
        ));
        let mut db = RoutedWriter::new(
            Writer::new(&budget, fixture::config(url)).unwrap(),
            completion,
            20,
            fixture::schemas(),
        )
        .unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let receipt = capture(&mut p, &1i64.to_le_bytes());
            dispatch.turn(Instant::now(), &mut [None; 8]).unwrap();
            let file = outputs[0].try_next(Instant::now()).unwrap().unwrap();
            let db_guard = outputs[1].try_next(Instant::now()).unwrap().unwrap();
            let foreign = db.write_delivery(file, &AtomicBool::new(false));
            assert_eq!(foreign.result(), Err(ProjectionError::Identity));
            assert_eq!(foreign.update(), None);
            assert!(foreign.publish().is_err());
            assert_eq!(
                completion
                    .poll(receipt, 10, Boundary::Written)
                    .unwrap()
                    .outcome,
                QueryOutcome::Pending
            );
            let file = foreign.into_delivery().ok().unwrap();
            file.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(file);
            let nested =
                fixture::runtime(async { db.write_delivery(db_guard, &AtomicBool::new(false)) });
            assert_eq!(nested.result(), Err(ProjectionError::Runtime));
            assert_eq!(nested.update(), None);
            assert_eq!(
                completion
                    .poll(receipt, 20, Boundary::DatabaseAcknowledged)
                    .unwrap()
                    .outcome,
                QueryOutcome::Pending
            );
            let result = db.write_delivery(
                nested.into_delivery().ok().unwrap(),
                &AtomicBool::new(false),
            );
            assert_eq!(result.result().unwrap().outcome, Outcome::Acknowledged);
            drop(result);
            assert_eq!(
                completion
                    .poll(receipt, 20, Boundary::DatabaseAcknowledged)
                    .unwrap()
                    .outcome,
                QueryOutcome::Unknown
            );
            dispatch.turn(Instant::now(), &mut [None; 8]).unwrap();
        });
        drop(db);
    });
    assert_eq!(budget.used_bytes(), 0);
    server.join().unwrap();
}

struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> (Self, std::fs::File) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "dot-influx-isolation-{}-{}.mcap",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let f = std::fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        (Self(path), f)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).unwrap();
    }
}
fn read_request(s: &mut std::net::TcpStream) {
    let mut data = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        let n = s.read(&mut chunk).unwrap();
        assert!(n > 0);
        data.extend_from_slice(&chunk[..n]);
        if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            let h = String::from_utf8_lossy(&data[..i]).to_lowercase();
            let len: usize = h
                .lines()
                .find_map(|l| l.strip_prefix("content-length: "))
                .unwrap()
                .parse()
                .unwrap();
            if data.len() >= i + 4 + len {
                return;
            }
        }
    }
}
#[test]
fn stalled_saturated_db_retains_one_raw_slot_while_all_six_mcap_records_finish() {
    let mut measurements = Vec::new();
    for db_enabled in [false, true] {
        let (listener_url, server, started) = if db_enabled {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            let server = std::thread::spawn(move || {
                let deadline = Instant::now() + std::time::Duration::from_secs(4);
                let mut s = loop {
                    if let Ok((s, _)) = listener.accept() {
                        break s;
                    }
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(1));
                };
                s.set_read_timeout(Some(std::time::Duration::from_secs(4)))
                    .unwrap();
                read_request(&mut s);
                tx.send(()).unwrap();
                let mut byte = [0];
                assert_eq!(
                    s.read(&mut byte).unwrap(),
                    0,
                    "local request socket must close before return"
                );
            });
            (url, Some(server), Some(rx))
        } else {
            ("http://127.0.0.1:1".into(), None, None)
        };
        let plan = fixture::plan();
        let route = fixture::route(db_enabled);
        let budget = QueryBudget::new(&plan);
        let mut owner = CaptureOwner::new_tracked(
            &plan,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &route,
            fixture::tracking(),
        )
        .unwrap();
        let (path, file) = Temp::new();
        let cancel = AtomicBool::new(false);
        owner.scope(|mut scope|{let completion=scope.completion().unwrap();let(mut p,a)=scope.next().unwrap();let(_,b)=scope.next().unwrap();let mut router=Router::new(&plan,vec![a,b],route).unwrap();let mut db=RoutedWriter::new(Writer::new(&budget,fixture::config(listener_url)).unwrap(),completion,20,fixture::schemas()).unwrap();
   let mut writer=SegmentWriter::new(router.writer_budget(10).unwrap(),WriterConfig{max_payload_bytes:64,write_buffer_bytes:4096,chunk_target_bytes:1024,max_records:8,max_chunks:8,max_metadata_bytes:4096},vec![StorageChannel{channel:ChannelConfig::new(7,99,1),topic:"scalar".into(),payload_encoding:"i64-le".into(),schema:None}],file).unwrap();
   router.scope(|mut dispatch,mut outputs|{let first=capture(&mut p,&1i64.to_le_bytes());dispatch.turn(Instant::now(),&mut[None;8]).unwrap();let first_file=outputs[0].try_next(Instant::now()).unwrap().unwrap();let db_guard=if db_enabled{Some(outputs[1].try_next(Instant::now()).unwrap().unwrap())}else{None};
    std::thread::scope(|threads|{let job=db_guard.map(|guard|threads.spawn(||db.write_delivery(guard,&cancel)));if let Some(rx)=started{rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();}
     let began=Instant::now();writer.write_delivery(&first_file).unwrap();drop(first_file);dispatch.turn(Instant::now(),&mut[None;8]).unwrap();
     for seq in 2..=6i64{let receipt=capture(&mut p,&seq.to_le_bytes());dispatch.turn(Instant::now(),&mut[None;8]).unwrap();let guard=outputs[0].try_next(Instant::now()).unwrap().unwrap();writer.write_delivery(&guard).unwrap();assert_eq!(completion.poll(receipt,10,Boundary::Written).unwrap().outcome,QueryOutcome::Complete);if db_enabled{assert_eq!(completion.poll(receipt,20,Boundary::DatabaseAcknowledged).unwrap().outcome,QueryOutcome::Dropped(DropReason::Capacity));}drop(guard);dispatch.turn(Instant::now(),&mut[None;8]).unwrap();}
     drop(writer.finish().unwrap());let file_ns=began.elapsed().as_nanos();let bytes=std::fs::read(&path.0).unwrap();let messages:Vec<_>=NativeReader::new(&bytes,64,4096).unwrap().map(|m|m.unwrap()).collect();assert_eq!(messages.len(),6);for(seq,msg)in messages.iter().enumerate(){let record=msg;assert_eq!(record.payload,&((seq+1)as i64).to_le_bytes());assert_eq!(record.record.receipt.accepted_seq,(seq+1)as u64);}
     let inflight=dispatch.target_snapshot(20).unwrap().in_flight;if db_enabled{assert_eq!(inflight,1);assert_eq!(dispatch.snapshot().raw_retained_bytes,8);assert_eq!(completion.poll(first,20,Boundary::DatabaseAcknowledged).unwrap().outcome,QueryOutcome::Pending);}measurements.push((bytes.len(),file_ns));println!("S3 file isolation db_enabled={db_enabled} records=6 file_bytes={} file_phase_ns={file_ns} db_in_flight={inflight}",bytes.len());
     cancel.store(true,std::sync::atomic::Ordering::Release);if let Some(job)=job{let attempt=job.join().unwrap();assert_eq!(attempt.result().unwrap().outcome,Outcome::Unknown);attempt.publish().unwrap();drop(attempt);assert_eq!(dispatch.turn(Instant::now(),&mut[None;8]).unwrap().reclaimed,1);}
    });
   });drop(db);
  });
        assert_eq!(budget.used_bytes(), 0);
        if let Some(server) = server {
            server.join().unwrap();
        }
    }
    assert_eq!(measurements[0].0, measurements[1].0);
}

#[test]
fn explicit_scalar_event_codecs_validate_original_payload_without_fallback() {
    let cases = [
        (
            ProjectionEncoding::F64Le,
            Kind::Float,
            2.5f64.to_le_bytes().to_vec(),
            Some("float=2.5"),
        ),
        (
            ProjectionEncoding::BoolByte,
            Kind::Boolean,
            vec![1],
            Some("boolean=true"),
        ),
        (
            ProjectionEncoding::Utf8Event,
            Kind::Event,
            b"a\"b".to_vec(),
            Some("event=\"a\\\"b\""),
        ),
        (
            ProjectionEncoding::F64Le,
            Kind::Float,
            f64::NAN.to_le_bytes().to_vec(),
            None,
        ),
        (ProjectionEncoding::BoolByte, Kind::Boolean, vec![2], None),
        (ProjectionEncoding::Utf8Event, Kind::Event, vec![255], None),
    ];
    for (encoding, kind, bytes, expected) in cases {
        let (url, server) = if expected.is_some() {
            let (u, s) = peer(ACK);
            (u, Some(s))
        } else {
            ("http://127.0.0.1:1".into(), None)
        };
        let plan = fixture::plan();
        let route = fixture::route(true);
        let budget = QueryBudget::new(&plan);
        let mut owner = CaptureOwner::new_tracked(
            &plan,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &route,
            fixture::tracking(),
        )
        .unwrap();
        owner.scope(|mut scope| {
            let c = scope.completion().unwrap();
            let (mut p, a) = scope.next().unwrap();
            let (_, b) = scope.next().unwrap();
            let mut router = Router::new(&plan, vec![a, b], route).unwrap();
            let mut cfg = fixture::config(url);
            cfg.channels[0].kind = kind;
            let mut schemas = fixture::schemas();
            schemas[0].encoding = encoding;
            let mut db =
                RoutedWriter::new(Writer::new(&budget, cfg).unwrap(), c, 20, schemas).unwrap();
            router.scope(|mut d, mut out| {
                let receipt = capture(&mut p, &bytes);
                d.turn(Instant::now(), &mut [None; 8]).unwrap();
                drop(out[0].try_next(Instant::now()).unwrap().unwrap());
                let result = db.write_delivery(
                    out[1].try_next(Instant::now()).unwrap().unwrap(),
                    &AtomicBool::new(false),
                );
                if expected.is_some() {
                    assert_eq!(result.result().unwrap().outcome, Outcome::Acknowledged);
                } else {
                    assert_eq!(result.result(), Err(ProjectionError::Payload));
                }
                result.publish().unwrap();
                let outcome = c
                    .poll(receipt, 20, Boundary::DatabaseAcknowledged)
                    .unwrap()
                    .outcome;
                assert_eq!(
                    outcome,
                    if expected.is_some() {
                        QueryOutcome::Complete
                    } else {
                        QueryOutcome::Failed(FailureReason::AdapterFailure)
                    }
                );
                drop(result);
                d.turn(Instant::now(), &mut [None; 8]).unwrap();
            });
            drop(db);
        });
        assert_eq!(budget.used_bytes(), 0);
        if let Some(server) = server {
            assert!(server.join().unwrap().contains(expected.unwrap()));
        }
    }
}
#[test]
fn routed_schema_registration_cannot_exceed_the_shared_partition() {
    let mut config = fixture::plan().config().clone();
    config.memory.external_bytes = 2 * 4096 + 32768;
    let plan = dot_logger::config::SessionBuilder::new(config)
        .validate(&Default::default())
        .unwrap();
    let budget = QueryBudget::new(&plan);
    let route = fixture::route(true);
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        fixture::tracking(),
    )
    .unwrap();
    owner.scope(|scope| {
        let c = scope.completion().unwrap();
        let writer = Writer::new(&budget, fixture::config("http://127.0.0.1:1".into())).unwrap();
        assert!(matches!(
            RoutedWriter::new(writer, c, 20, fixture::schemas()),
            Err(ProjectionError::InvalidConfig)
        ));
    });
    assert_eq!(budget.used_bytes(), 0);
}

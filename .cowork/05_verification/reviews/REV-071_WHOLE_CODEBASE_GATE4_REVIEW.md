# REV-071 — Gate 4 수락 및 공개 전환 전 전체 코드베이스 독립 코드리뷰

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-071 / 2026-11-21 |
| 리뷰어 | Antigravity AI — 리뷰 전용 세션 |
| 대상 | Workspace 전체(`crates/dot-logger`, `crates/dot-logger-client`, `crates/dot-logger-cli`, `crates/dot-logger-viewer`, `examples/`, `tests/`, `proto/`, `scripts/`, `.github/workflows/ci.yml`, `vendor/mcap-0.26.0/`, `test-profiles/`) — `HEAD` `a46c2d7` (요청 등록 `a354f3c`, 근거 기준 `1b0fe85`, 마지막 코드 변경 `8bab94b`) |
| 관련 Task / Gate | [TASK-025](../../04_implementation/tasks/TASK-025.md) — Gate 4 최종 수락 및 리포지토리 공개(`gh repo edit --visibility public` + T3 native CI) 직전 전체 코드 독립 점검 |
| 종합 판정 | **Fail** (Gate 4 수락 및 공개 전환 전 `RV-171~184` 조치 필수 — **P1 3건 · P2 11건 · P3 4건**, 총 18건 `RV-171~188`) |
| 상태 | **Open** — 2026-11-21 리뷰 전용 세션 등록 (`RV-171~188`) |

Gate 4 수락과 저장소 공개 및 T3(GitHub Actions multi-OS native CI) 발동을 앞둔 시점에서 diff가 아닌 현재 workspace 코드 전체를 대상으로 **테마 1(역대 최다 지적 3대 영역 A·B·C)**, **테마 2(보안 ①~⑥)**, **테마 3(신뢰성 ①~⑥)** 관점에서 독립 심층 리뷰를 수행했다.

주요 확인 결과는 다음과 같다.
1. **핵심 결함 (P1 3건 — 공개/T3 CI 및 런타임 정합성 직결):**
   - **`RV-171` (`logger/host.rs` · `control/mod.rs`):** 주기적 일반 동기화(`Host::ordinary_sync`)가 일시적 I/O 정체 등으로 기한을 넘겨 `EXPIRED`로 전환될 때 `Host::hand`가 `Ok(None) => true`로 `pending`에 타깃을 남기면서 `Host::ordinary_sync`가 `if snapshot.finished && pending.is_empty()` 조건으로 `release_sync`를 호출한다. 이로 인해 단일 일반 동기화 슬롯(`sync_slots: 1`)이 영구 누수되어 이후 세션 전체 기간 동안 주기적 파일 sync가 멈추고 `HealthState::Degraded(SyncOverdue)`로 고착되며, 이후 정상 `Logger::shutdown`까지 `ShutdownReport.sync_failed = true`(`is_success() == false`)로 오염된다.
   - **`RV-172` (`.github/workflows/ci.yml` · `scripts/check-feature-matrix.py` · 14개 테스트/하네스):** `LoggerBuilder::file_output`은 Linux 전용 `linux_available_bytes`와 Unix 전용 `FileStore::new`를 사용하는데, `contracts` CI가 `macos-14`·`windows-2025`에서도 플랫폼 가드 없는 facade/`FileStore`/`api-consumer`/`rt-allocation` 테스트를 실행하므로 저장소 공개 후 T3 CI 발동 즉시 비-Linux 러너가 실패한다.
   - **`RV-173` (`.github/workflows/ci.yml` · `scripts/check-feature-matrix.py` · `Cargo.toml`):** `default-members = ["crates/dot-logger"]` 설정 상태에서 CI의 `check-feature-matrix.py`와 `ci.yml` 어디에서도 `dot-logger-client`, `dot-logger-cli`, `dot-logger-viewer`의 `cargo test`를 실행하지 않아 3개 크레이트의 단위·통합 테스트가 CI에서 완전히 누락되어 있다(`clippy --all-targets`는 컴파일/린트만 수행).
2. **주요 개선/보완 필요 항목 (P2 11건, P3 4건):**
   - **테마 1-A(기본 경로·완료/동기화/종료):** `io_loop`에서 `StorageError::Busy` 시 인덱스 전진 없이 무한 재시도하며 중복 손실 checkpoint를 계속 기록하는 문제(`RV-174`), `SyncWork::settled`/`covers_file`·`FileLedger::settle`의 타파일 미동기화 기록 오승인 및 기동기화 구간 미정리 문제(`RV-175`), `begin_shutdown` 이후 전원 경보 시 선택 타깃 `EXPIRED` 오전이 및 `target.log.finish` 실패·`Shutdown::Incomplete` 시 `FacadeReport.unconfirmed` 빈 벡터 반환(`RV-176`), `Boundary::Enqueued` 타깃 계약의 유기 오인 및 폐기 후 `Complete` 오집계(`RV-185`).
   - **테마 1-B(원격·클라이언트 ACK·CLI·viewer):** `RemoteSubscription::next`/`handoff`의 `self.lost` 미검사·유휴 스트림 대기 시 `in_flight` ACK 실패 지연(`TimedOut` 오반환)·EOF 시 대기 `acks` 폐기·viewer bridge `flush` 누락(`RV-177`), `RemoteHub::Subscription::expire` waker 미깨움 및 타이머 밀림(`RV-178`), `dashboard::serve` 취소 시 `work` 태스크 영구 누수 및 `websocket::negotiate`의 `Host`·`Origin` 포트 검증 공백(`RV-179`), CLI `recover` 사전 검증 및 원격 `Query` `blob`/`snapshot` 일관성(`RV-186`).
   - **테마 1-C·2·3(저장·MCAP·손실·공급망·CI/시험):** vendored `mcap` `Attachment`/인덱스 슬라이스 패닉·`u64` 오버플로·`McapString` 선할당(`RV-180`), `op == 7` 선행 허용·`_ => continue`·`Snapshot::scan` `end == 0` 검증 우회(`RV-181`), `loss::summarize`의 part 간 `counter_key` 중복 미검사 및 빈 v1 decode 허용(`RV-182`, `RV-164` 조정 결정은 존중 유지), `QueryConfig::reservation` 메타데이터 상한 불일치(`RV-187`), 공개 전 개인 경로 잔류(`single-crc.patch` 내 `/home/lim8603/...`, 성능 스크립트 내 `/data/build`) 및 패치 누락(`RV-183`), Windows 절대경로 실패(`RV-184`), Miri `--skip logger::` 누락·`/tmp` `Drop` 가드·`test-profiles/local-pc.toml` 정합성(`RV-188`).
3. **사전 확인된 건전성 불변식:** RT 경로 무할당/무락/무블로킹, `pool/storage.rs` `unsafe` 4곳의 61비트 세대+3비트 상태 머신 건전성, 비-loopback HTTPS+상수시간 토큰 비교, 비밀번호·토큰 미유출, 24시간 카운터/자료구조 상한 고정, `.active` → `.mcap` atomic rename 및 `check-remote-protocol.py` / `record_meta.bin`·`loss_report.bin` 일치 등은 양호하게 유지됨을 확인했다.

리뷰 전용 세션 원칙([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md))에 따라 제품 코드·테스트·스크립트·CI 설정은 일체 수정하지 않았다.

---

## 2. 파일 변경표

### 이번 리뷰 세션 산출물 (`.cowork/` 전용)

| 구분 | 파일 | 변경 내용 |
|---|---|---|
| 생성 | [REV-071_WHOLE_CODEBASE_GATE4_REVIEW.md](REV-071_WHOLE_CODEBASE_GATE4_REVIEW.md) | 전체 코드베이스 독립 리뷰 결과(`RV-171~188`) 및 테마 1·2·3 감사 보고서 작성 (`Open`, `Fail`) |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | 미처리 리뷰·코드리뷰 현황(`Open`, P1 3·P2 11·P3 4)·다음 시작점에 `REV-071` 조치 항목 등록 |
| 수정 | [my_state.md](../../members/lim8603/workspace/my_state.md) | 작업자 상태 리뷰·세션·다음 시작점 동기화 |
| 생성 | session_2026-11-21_004.md(비공개 세션 로그: `session_logs/session_2026-11-21_004.md`) | `REV-071` 코드리뷰 전용 세션 수행 로그 기록 |

### 주요 감사 대상 파일 (Read-Only 검토, 제품 코드 수정 0건)

- **Core / Runtime (`crates/dot-logger/src/`):** [logger/host.rs](../../../crates/dot-logger/src/logger/host.rs), [logger/writer.rs](../../../crates/dot-logger/src/logger/writer.rs), [logger/mod.rs](../../../crates/dot-logger/src/logger/mod.rs), [capture/mod.rs](../../../crates/dot-logger/src/capture/mod.rs), [completion/mod.rs](../../../crates/dot-logger/src/completion/mod.rs), [control/mod.rs](../../../crates/dot-logger/src/control/mod.rs), [control/config_update.rs](../../../crates/dot-logger/src/control/config_update.rs), [routing/mod.rs](../../../crates/dot-logger/src/routing/mod.rs), [pool/storage.rs](../../../crates/dot-logger/src/pool/storage.rs), [diagnostics/host.rs](../../../crates/dot-logger/src/diagnostics/host.rs), [diagnostics/mod.rs](../../../crates/dot-logger/src/diagnostics/mod.rs)
- **Storage / Query / MCAP (`crates/dot-logger/src/` & `vendor/`):** [storage/mod.rs](../../../crates/dot-logger/src/storage/mod.rs), [storage/writer.rs](../../../crates/dot-logger/src/storage/writer.rs), [storage/retention.rs](../../../crates/dot-logger/src/storage/retention.rs), [storage/read.rs](../../../crates/dot-logger/src/storage/read.rs), [storage/recovery.rs](../../../crates/dot-logger/src/storage/recovery.rs), [loss.rs](../../../crates/dot-logger/src/loss.rs), [query/mod.rs](../../../crates/dot-logger/src/query/mod.rs), [vendor/mcap-0.26.0/src/read.rs](../../../vendor/mcap-0.26.0/src/read.rs), [vendor/mcap-0.26.0/src/records.rs](../../../vendor/mcap-0.26.0/src/records.rs), [vendor/mcap-0.26.0/single-crc.patch](../../../vendor/mcap-0.26.0/single-crc.patch)
- **Remote / Client / CLI / Viewer:** [adapters/remote/hub.rs](../../../crates/dot-logger/src/adapters/remote/hub.rs), [adapters/remote/server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs), [adapters/remote/wire.rs](../../../crates/dot-logger/src/adapters/remote/wire.rs), [adapters/influxdb/numeric.rs](../../../crates/dot-logger/src/adapters/influxdb/numeric.rs), [dot-logger-client/src/transport.rs](../../../crates/dot-logger-client/src/transport.rs), [dot-logger-client/src/codec.rs](../../../crates/dot-logger-client/src/codec.rs), [dot-logger-cli/src/model.rs](../../../crates/dot-logger-cli/src/model.rs), [dot-logger-cli/src/commands.rs](../../../crates/dot-logger-cli/src/commands.rs), [dot-logger-viewer/src/main.rs](../../../crates/dot-logger-viewer/src/main.rs), [dot-logger-viewer/src/bridge.rs](../../../crates/dot-logger-viewer/src/bridge.rs), [dot-logger-viewer/src/dashboard.rs](../../../crates/dot-logger-viewer/src/dashboard.rs), [dot-logger-viewer/src/websocket.rs](../../../crates/dot-logger-viewer/src/websocket.rs)
- **CI / Scripts / Tests:** [Cargo.toml](../../../Cargo.toml), [.github/workflows/ci.yml](../../../.github/workflows/ci.yml), [scripts/check-feature-matrix.py](../../../scripts/check-feature-matrix.py), [scripts/run-perf-rt.sh](../../../scripts/run-perf-rt.sh), [scripts/run-perf-s3.sh](../../../scripts/run-perf-s3.sh), [test-profiles/local-pc.toml](../../../test-profiles/local-pc.toml), [routed_check_host.rs](../../../crates/dot-logger-cli/tests/routed_check_host.rs), [routed_host.rs](../../../crates/dot-logger-client/tests/routed_host.rs)

---

## 3. 클래스(타입) 변경표

**이번 리뷰 세션의 제품 코드 변경은 없음.** 감사 대상 핵심 타입(`struct` · `enum`)은 아래와 같다.

| 구분 | 소속 파일 | 타입 이름 | 감사 관점 및 주요 발견 |
|---|---|---|---|
| 검토 | [logger/host.rs](../../../crates/dot-logger/src/logger/host.rs), [control/mod.rs](../../../crates/dot-logger/src/control/mod.rs) | `Host`, `ControlOwner`, `SyncWork`, `FacadeReport` | 주기/종료/비상 sync 상태 전이·slot 해제·`unconfirmed` 보고 (`RV-171`, `RV-174`, `RV-175`, `RV-176`) |
| 검토 | [storage/mod.rs](../../../crates/dot-logger/src/storage/mod.rs), [storage/writer.rs](../../../crates/dot-logger/src/storage/writer.rs), [storage/retention.rs](../../../crates/dot-logger/src/storage/retention.rs) | `FileLedger`, `SegmentWriter`, `FinishedSegment`, `SegmentLog`, `FileStore` | 타파일 미동기화 범위(`owed`) 추적/정리 및 OS 플랫폼 제약 (`RV-172`, `RV-175`) |
| 검토 | [completion/mod.rs](../../../crates/dot-logger/src/completion/mod.rs) | `Registry`, `Completion`, `AbandonMarker`, `QueryOutcome` | `Boundary::Enqueued` 완료 마스크 및 폐기/완료 분류 (`RV-185`) |
| 검토 | [pool/storage.rs](../../../crates/dot-logger/src/pool/storage.rs) | `Slot`, `Exclusive`, `Shared` | 워크스페이스 유일 `unsafe` 4곳(61비트 세대+3비트 상태 머신) 건전성 검증 완료 |
| 검토 | [dot-logger-client/src/transport.rs](../../../crates/dot-logger-client/src/transport.rs), [adapters/remote/hub.rs](../../../crates/dot-logger/src/adapters/remote/hub.rs), [adapters/remote/server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs) | `RemoteSubscription`, `Subscription`, `SubscriptionStream` | `self.lost` 보존·in-flight ACK 대기·EOF 잔류 ACK·lease 만료 waker 깨움 (`RV-177`, `RV-178`, `RV-186`) |
| 검토 | [dot-logger-viewer/src/dashboard.rs](../../../crates/dot-logger-viewer/src/dashboard.rs), [dot-logger-viewer/src/websocket.rs](../../../crates/dot-logger-viewer/src/websocket.rs), [dot-logger-viewer/src/bridge.rs](../../../crates/dot-logger-viewer/src/bridge.rs) | `Shared`, `Upstream`, `LiveSubscription` | 대시보드 `work` 태스크 수명, WS `Host`/`Origin` 포트 검증, 종료 시 ACK flush (`RV-177`, `RV-179`) |
| 검토 | [storage/read.rs](../../../crates/dot-logger/src/storage/read.rs), [storage/recovery.rs](../../../crates/dot-logger/src/storage/recovery.rs), [query/mod.rs](../../../crates/dot-logger/src/query/mod.rs), [loss.rs](../../../crates/dot-logger/src/loss.rs) | `NativeReader`, `Snapshot`, `QueryConfig`, `LossReport`, `LossRange` | 선행 opcode `7`·`safe_offset == 0`·part 간 키 중복·메타데이터 예약 일관성 (`RV-180`, `RV-181`, `RV-182`, `RV-187`) |

---

## 4. 리뷰 포인트 — 테마별 감사 매트릭스 및 지적 목록 (`RV-171~188`)

### A. 테마별 건전성 검증 및 지적 매트릭스

| 테마 / 영역 | 직접 확인한 건전성 불변식 | 신규 지적 (`RV-171~188`) |
|---|---|---|
| **테마 1-A. 기본 경로·완료/동기화/종료 경계** | RT 경로(`Writer::try_log`/`try_log_at`/`refresh`/`Drop`, `Producer::*`) 무할당·무락·무블로킹·무포맷팅 유지; `Writer::drop` 1슬롯 `rtrb` 반납 + 호스트 reaper(`is_abandoned` 대기)로 관리 스레드 최종 해제; `ready_for_sync`의 `input.taken < range.last` 선검사 및 연속 watermark 전진; `Delivery`의 `_permit` 선해제 drop 순서; 2세대 runtime config barrier(`State::boundary`/`ready`/`PlanGuard`) | `RV-171` (P1), `RV-174` (P2), `RV-175` (P2), `RV-176` (P2), `RV-185` (P3) |
| **테마 1-B. 원격 전송·클라이언트 ACK/구독·CLI·viewer** | `ACK_BATCH=64`·`ACK_IN_FLIGHT=4` 상한 및 서버 `ack_batch`의 peer 소유권·구독 창 사전 검증; `RemoteClient::connect` 단일 `TcpStream` `io.take()`로 자동 재연결 차단; `RemoteHub::offer` 연속 Gap 병합; CLI exit code `2`/`3`/`4`·`--max-output-bytes` 512B 여유; viewer `QueueGap` 로컬 드롭 및 미수락 구독 재연결 | `RV-177` (P2), `RV-178` (P2), `RV-179` (P2), `RV-186` (P3) |
| **테마 1-C. 파일 저장·native MCAP·손실·회전/복구** | `FileInfo::parse`·`read_frame`·`codec::decode_meta`·`LossReport::decode`(`bounded_wire`) 선검사; `Lease`/`Reservation` 마지막 필드 drop 순서; `Validation::chunk` CRC-32 및 `recovery_chunk_fits` whole-chunk 원자성; `.active` → `.mcap` rename 및 dir `sync_all()`; `Circular`/`Event` pin·보호 구간 보존 및 `QuerySource` reader 선해제; `vendor/mcap` 무압축 단일 CRC 패치 정합 | `RV-175` (P2), `RV-180` (P2), `RV-181` (P2), `RV-182` (P2), `RV-187` (P3) |
| **테마 2. 보안 ①~⑥** | ① gRPC 크기 상한·`Reassembler` 선검사·`Catalog` 1MiB/제어문자 차단·`dashboard.rs` 8KiB/32헤더/`GET`전용/`Host`검증·`dashboard.js` `textContent`+엄격한 CSP·InfluxDB Flux `${...}` 차단; ② 비-loopback HTTPS+토큰, `subtle::ConstantTimeEq` 256B 상수시간 비교, `Debug` 미구현·`set_sensitive`·토큰 제로화, Read/Manage 분리; ③ 동시성/창 상한; ④ `create_new(true)` 및 `symlink_metadata` dir 검사; ⑤ `pool/storage.rs` `unsafe` 4곳 soundness; ⑥ 직접 의존성 `=X.Y.Z` 고정·실제 비밀키/토큰 유출 0건(`test-only-key.pem`만 시험용) | `RV-179` (P2), `RV-180` (P2), `RV-182` (P2), `RV-183` (P2), `RV-186` (P3) |
| **테마 3. 신뢰성 ①~⑥** | ① 전원/crash 후 whole-chunk 복구; ② IO/host `catch_unwind` + `CloseOnExit`/`StopOnExit`/reaper 격리; ③ RPC/HTTP/WS/shutdown 기한 설정; ④ `u64` `checked_add(1)`, `loss::Cell` 포화(`overflow`), `History`/`OperationsHost`/`CheckHost`/`Catalog`/`RemoteHub`/`Reassembler`/`Bridge` 고정 상한으로 24시간 단조 증가 누수 없음; ⑥ `check-remote-protocol.py`(10 messages / 89 fields, `protoc Pass`) 및 `record_meta.bin`·`loss_report.bin` 일치 | `RV-171` (P1), `RV-172` (P1), `RV-173` (P1), `RV-174` (P2), `RV-176` (P2), `RV-177` (P2), `RV-178` (P2), `RV-179` (P2), `RV-181` (P2), `RV-183` (P2), `RV-184` (P2), `RV-188` (P3) |

### B. 지적 항목 요약표

| ID | 중요도 | 테마 | 위치 | 문제 요약 | 판정 | 상태 |
|---|---|---|---|---|---|---|
| **RV-171** | **P1** | 1-A, 3-③/④ | `logger/host.rs:705-745`, `control/mod.rs:434-440, 535-540, 818-835` | 주기적 sync 만료(`EXPIRED`) 시 `Host::hand`가 `pending`에 잔류시켜 `release_sync` 미호출 → 단일 sync 슬롯 영구 누수·주기 sync 영구 중단·`SyncOverdue` 고착·종료 `sync_failed` 오염 | Open | Open |
| **RV-172** | **P1** | 3-⑥ | `.github/workflows/ci.yml:16,33-37`, `check-feature-matrix.py:18-50`, `logger/host.rs:330-339`, `diagnostics/host.rs:354-364`, `storage/retention.rs:55-58` 외 14개 파일 | `file_output`/`FileStore::new` 비-Linux 비지원 경로에 플랫폼 가드가 없는 14개 테스트/하네스로 `macos-14`·`windows-2025` CI 즉시 실패 | Open | Open |
| **RV-173** | **P1** | 3-⑥ | `.github/workflows/ci.yml:33-37`, `check-feature-matrix.py:18-50`, `Cargo.toml:3` | `dot-logger-client`, `dot-logger-cli`, `dot-logger-viewer` 테스트가 CI 어디에서도 실행되지 않음 | Open | Open |
| **RV-174** | **P2** | 1-A, 3-②/④ | `logger/host.rs:885-927` | `io_loop`에서 `sync_work`가 `Busy` 반환 시 `index` 증가 없이 재시도하여 매 회전마다 중복 손실 checkpoint chunk를 기록하며 무한 스핀 | Open | Open |
| **RV-175** | **P2** | 1-A, 1-C | `control/mod.rs:1007-1053`, `storage/mod.rs:215-236`, `storage/writer.rs:330-334,1054-1056,1091-1094,1115-1119`, `storage/retention.rs:235-237,766,940` | `SyncWork::settled`/`covers_file`의 미동기화 타파일 `Written` 오승인, sync 후 `first`/`last` 미초기화, `FileLedger::settle` 구간 미축소 | Open | Open |
| **RV-176** | **P2** | 1-A, 3-③ | `control/mod.rs:532-540`, `logger/host.rs:629-642, 666-668, 763-794` | `begin_shutdown` 이후 전원 경보 시 선택 타깃 `EXPIRED` 오전이 및 `target.log.finish` 실패·`Incomplete` 시 `FacadeReport.unconfirmed` 빈 벡터 | Open | Open |
| **RV-177** | **P2** | 1-B, 3-③ | `dot-logger-client/src/transport.rs:709-794, 801-843`, `dot-logger-viewer/src/bridge.rs:281-282, 303-330` | `RemoteSubscription::next`/`handoff` `self.lost` 미검사, 유휴 스트림 시 `in_flight` ACK `select!` 누락(`TimedOut` 지연 반환), EOF 시 `acks` 조용한 폐기, viewer `flush` 미호출 | Open | Open |
| **RV-178** | **P2** | 1-B, 3-③ | `adapters/remote/hub.rs:185-203`, `adapters/remote/server.rs:462-494` | `Subscription::expire` waker 미깨움 및 `SubscriptionStream` 타이머 리셋으로 만료 구독 `Gone` 지연/`TimedOut` 오종료 | Open | Open |
| **RV-179** | **P2** | 1-B, 2-①, 3-④ | `dot-logger-viewer/src/dashboard.rs:290-305,319-339`, `main.rs:256-274`, `websocket.rs:127-175` | `dashboard::serve` 취소/드롭 시 `work` 태스크 누수(영구 1초 폴링), WS `Host` 헤더 미검증 및 `local_origin` 범위 밖 포트(`:99999`) 통과 | Open | Open |
| **RV-180** | **P2** | 1-C, 2-①/③ | `vendor/mcap-0.26.0/src/read.rs:170-177,716-725,800-805,849-858,926-931,954-959`, `records.rs:142-152,163-175,226-245,293-313` | vendored `mcap` 손상 `Attachment`/Index 슬라이스 패닉·오프셋 오버플로, `McapString` 선할당, vec/map `byte_len` 경계 미검사 | Open | Open |
| **RV-181** | **P2** | 1-C, 3-① | `storage/read.rs:227,339`, `storage/recovery.rs:305,472`, `query/mod.rs:530-532,635` | 첫 `Chunk`/`Metadata` 이전 `op == 7` 허용 및 `read.rs:339` `_ => continue`, `Snapshot::scan` `end == 0` 조기 반환 | Open | Open |
| **RV-182** | **P2** | 1-C, 2-① | `loss.rs:147-153, 593-610, 799-908` | `summarize`의 part 간 `counter_key` 중복 미검사로 손상 checkpoint `complete` 오승인, `decode`의 빈 v1 허용, `LossRange::holds` `part` 경계 미대조 | Open | Open |
| **RV-183** | **P2** | 2-⑥, 3-⑥ | `vendor/mcap-0.26.0/single-crc.patch:1-24`, `Cargo.toml:46-49`, `run-perf-rt.sh:26`, `run-perf-s3.sh:28` | `single-crc.patch` 개인 홈 경로(`/home/lim8603/...`) 및 2개 패치 누락/줄번호 어긋남, 성능 스크립트 `/data/build` 하드코딩 | Open | Open |
| **RV-184** | **P2** | 3-⑥ | `dot-logger-cli/tests/routed_check_host.rs:84`, `dot-logger-client/tests/routed_host.rs:17,45`, `diagnostics/mod.rs:148` | `"/nonexistent/..."` Unix 경로 하드코딩으로 Windows `CheckPlan::build` `InvalidDirectory` 실패 | Open | Open |
| **RV-185** | **P3** | 1-A | `completion/mod.rs:310-322, 980-998, 1013-1034` | `Boundary::Enqueued` 타깃에서 정상 드롭 유기 오인(`0b11110`) 및 취소/만료 폐기 후 `Complete` 오집계 | Open | Open |
| **RV-186** | **P3** | 1-B, 2-① | `dot-logger-cli/src/model.rs:179-187`, `commands.rs:530-565,747-782`, `client/codec.rs:220-224`, `adapters/remote/wire.rs:439-441` | CLI `recover` 중복 sequence 시 `target: None` 사전 검사 누락, `Query` `blob != 0 && payload_len == 0` 및 `snapshot` 변경 미거절 | Open | Open |
| **RV-187** | **P3** | 1-C | `query/mod.rs:341, 345-360` | `QueryConfig::reservation`의 `channels.len()` 상한(`u16::MAX`) 및 `PayloadSchema::name` 바이트 누락 | Open | Open |
| **RV-188** | **P3** | 3-⑤/⑥ | `logger/writer.rs:394,399`, `ci.yml:89`, `test-profiles/local-pc.toml:4,57-58,99-106`, 8개 테스트 파일 | `writer.rs` Miri ignore 누락 및 `--skip logger::` 과잉 제외, 8개 테스트 `/tmp` RAII `Drop` 누락, `local-pc.toml` readiness 낡음 | Open | Open |

---

### C. 개별 지적 상세 (`RV-171` ~ `RV-188`)

---

#### RV-171 — 주기적 일반 동기화 기한 만료 시 단일 sync 슬롯 영구 누수·주기 동기화 중단·`SyncOverdue` 고착 및 종료 실패 오보고 (P1)

- **심각도 / 테마:** `P1` / 테마 1-A(기본 경로·동기화·종료 경계), 테마 3-③/④(기한 보고 및 24시간 장기 운전)
- **위치:**
  - [host.rs:705-745](../../../crates/dot-logger/src/logger/host.rs#L705-L745) (`Host::hand`, `Host::ordinary_sync`)
  - [control/mod.rs:434-440, 535-540, 818-835](../../../crates/dot-logger/src/control/mod.rs#L434-L440) (`ControlOwner::next_sync`, `ControlOwner::poll`, `ControlOwner::drive_shutdown`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. [logger/budget.rs](../../../crates/dot-logger/src/logger/budget.rs)에서 facade는 일반 동기화 슬롯 수 `sync_slots = 1`을 사용한다. `Host::ordinary_sync`(`host.rs:753-758`)가 주기적 sync 토큰(`self.slots[0]`)을 요청해 `self.ordinary = Some((token, self.targets.clone()))`로 보관한다.
  2. 일시적인 디스크 정체나 배압으로 적어도 한 타깃의 `dispatcher.ready_for_sync`가 `Ok(false)`를 반환하는 상태가 `deadline`(`now + 4 * sync_every + 1 s`)까지 지속되면, `ControlOwner::poll(now)`(`control/mod.rs:535-539`)가 아직 `QUEUED`인 타깃 상태를 `EXPIRED`로 전이시킨다.
  3. 다음 턴에 `Host::ordinary_sync`가 `self.hand(...)`를 호출하면 `control.next_sync`는 `state != QUEUED`이므로 `Ok(None)`을 반환하고(`control/mod.rs:434-436`), `Host::hand`는 `Ok(None) => true`(`host.rs:721`)로 매칭해 해당 타깃을 `pending`에 그대로 남긴다.
  4. 이어서 `self.control.sync_snapshot(token, now)?`는 `snapshot.finished == true`(`queued == 0 && in_flight == 0 && applying == 0`, `expired >= 1`)를 반환하지만, `pending.is_empty()`는 **`false`**다.
  5. `host.rs:739`가 `if snapshot.finished && pending.is_empty()`를 조건으로 삼기 때문에 `self.control.release_sync(token)`이 **영원히 호출되지 않고** `self.ordinary = Some((token, pending))`이 매 턴 유지된다.
  6. **잘못된 결과:** ① 유일한 일반 동기화 슬롯(`slots[0]`)이 영구 누수되어 I/O 정체가 풀린 뒤에도 세션 종료 시점까지 주기적 파일 동기화가 영구 중단됨, ② `Logger::health()`가 `HealthState::Degraded(Degradation::SyncOverdue)`에 영구 고착됨, ③ 이후 `Logger::shutdown` 호출 시 `drive_shutdown`(`control/mod.rs:808-824`)이 `slots[0]`의 `EXPIRED` 타깃을 보고 `ShutdownReport.sync_failed = true`를 설정해 정상 종료까지 실패(`is_success() == false`)로 오보고됨.
- **확인 방법:** 코드 추적 추론 (`host.rs:705-760` 및 `control/mod.rs:434-540, 808-836` 대조).
- **수정 제안:**
  - `Host::ordinary_sync`([host.rs:739](../../../crates/dot-logger/src/logger/host.rs#L739))에서 `if snapshot.finished && pending.is_empty()`를 `if snapshot.finished`로 변경한다(`SyncSnapshot.finished` 자체가 `queued == 0 && in_flight == 0 && applying == 0`이므로 남아 있는 `pending` 원소는 모두 종료 상태 `EXPIRED`/`NONE`임이 보장됨).

---

#### RV-172 — 비-Linux(`macos-14`, `windows-2025`) GitHub Actions CI에서 플랫폼 가드 없는 facade/`FileStore` 테스트 14곳이 즉시 실패 (P1)

- **심각도 / 테마:** `P1` / 테마 3-⑥(CI 정합성 및 multi-OS native T3 실행 준비)
- **위치:**
  - [.github/workflows/ci.yml:16, 33-37](../../../.github/workflows/ci.yml#L16), [check-feature-matrix.py:18-50](../../../scripts/check-feature-matrix.py#L18-L50)
  - [logger/host.rs:330-339](../../../crates/dot-logger/src/logger/host.rs#L330-L339), [diagnostics/host.rs:354-364](../../../crates/dot-logger/src/diagnostics/host.rs#L354-L364), [storage/retention.rs:55-58](../../../crates/dot-logger/src/storage/retention.rs#L55-L58)
  - 플랫폼 가드 누락 테스트/하네스:
    - [logger/tests.rs:47](../../../crates/dot-logger/src/logger/tests.rs#L47), [logger/writer.rs:399-421](../../../crates/dot-logger/src/logger/writer.rs#L399-L421), [logger/host.rs:1187-1226,1277,1303](../../../crates/dot-logger/src/logger/host.rs#L1187-L1226)
    - `crates/dot-logger/tests/`: `tc_013_facade.rs`, `tc_013_preset.rs`, `tc_013_routing.rs`, `tc_013_tracing.rs`, [tc_013_calibrate.rs:337-395](../../../crates/dot-logger/tests/tc_013_calibrate.rs#L337-L395), `tc_014_standard.rs`, [tc_017_loss.rs:1144-1181](../../../crates/dot-logger/tests/tc_017_loss.rs#L1144-L1181) (`#[cfg(unix)]`), `tc_010_check_host.rs:2` (`#![cfg(unix)]`)
    - [tests/api-consumer/src/main.rs:394-423](../../../tests/api-consumer/src/main.rs#L394-L423), `tests/rt-allocation/src/tests.rs:1563-1610`, [crates/dot-logger-client/tests/facade_remote.rs:106-384](../../../crates/dot-logger-client/tests/facade_remote.rs#L106-L384)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `open_logs`(`logger/host.rs:330-339`)는 모든 `file_output`에 `linux_available_bytes`를 probe로 넘겨 `FileStore::new` → `SegmentLog::new` → `room()` → `available_bytes()`를 호출한다.
  2. `linux_available_bytes`(`diagnostics/host.rs:360-364`)는 `not(target_os = "linux")`에서 무조건 `Err(ErrorKind::Unsupported)`를 반환하고, `FileStore::new`(`retention.rs:56-58`)은 `!cfg!(unix)`에서 무조건 `Err(ErrorKind::Unsupported)`를 반환한다.
  3. 그러나 `ci.yml:16`의 `contracts` 매트릭스(`ubuntu-24.04`, `windows-2025`, `macos-14`)가 실행하는 `check-feature-matrix.py`는 Linux가 아닌 두 OS에서도 위 14개 파일의 `.file_output(...).start().unwrap()`, `FileCheckRuntime` + `linux_available_bytes`, `FileStore::new().unwrap()` 호출을 그대로 실행한다.
  4. **잘못된 결과:** 저장소 공개 직후 T3 GitHub Actions CI가 `macos-14`·`windows-2025`에서 실행되면 `contracts` 잡이 모든 feature 조합에서 즉시 실패한다.
- **확인 방법:** 코드 추적 및 `grep -rn` 호출처 전수 대조(`tc_013_calibrate.rs:218-233`의 `#[cfg(not(target_os = "linux"))]` 기존 패턴과 비교).
- **수정 제안:**
  - `Logger::builder().file_output(...).start()` 및 `linux_available_bytes` 성공을 전제하는 테스트와 `dot-logger-api-consumer`/`dot-logger-rt-allocation` facade 경로를 `#[cfg(target_os = "linux")]`로 가드하고, 비-Linux에서는 `Unsupported` 반환 및 생성 디렉터리 미잔류 검증을 둔다. `logger/host.rs:1183` 이하 `FileStore::new` 단위 테스트 모듈은 `#[cfg(unix)]`로 가드한다.

---

#### RV-173 — GitHub Actions CI에서 `dot-logger-client` · `dot-logger-cli` · `dot-logger-viewer` 테스트가 전혀 실행되지 않음 (P1)

- **심각도 / 테마:** `P1` / 테마 3-⑥(CI 정합성 및 전체 workspace 검증 누락)
- **위치:**
  - [.github/workflows/ci.yml:33-37](../../../.github/workflows/ci.yml#L33-L37)
  - [check-feature-matrix.py:18-50](../../../scripts/check-feature-matrix.py#L18-L50)
  - [Cargo.toml:2-3](../../../Cargo.toml#L2-L3)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. 루트 `Cargo.toml:3`은 `default-members = ["crates/dot-logger"]`로 지정되어 있다.
  2. `scripts/check-feature-matrix.py`는 `cargo test -p dot-logger`, `cargo tree` 격리 검사, `dot-logger-api-consumer`, `dot-logger-rt-allocation`, `scoped_capture` 예제만 실행하고, `.github/workflows/ci.yml:33-37`의 `clippy --workspace --all-targets`는 컴파일·린트만 수행한다.
  3. **잘못된 결과:** `dot-logger-client`(단위 + 7개 통합 테스트), `dot-logger-cli`(단위 + 4개 통합 테스트), `dot-logger-viewer`(대시보드·WS·bridge·replay 테스트)의 모든 테스트가 GitHub Actions CI에서 단 한 건도 실행되지 않아, 원격 클라이언트·CLI·대시보드 회귀가 CI Pass로 통과할 수 있다.
- **확인 방법:** `.github/workflows/ci.yml` 및 `scripts/check-feature-matrix.py` 전수 검사.
- **수정 제안:**
  - `scripts/check-feature-matrix.py`(또는 `ci.yml` `contracts`)에 `cargo test --locked -p dot-logger-client --no-default-features`, `--all-features`, `-p dot-logger-cli`, `-p dot-logger-viewer` 실행을 추가한다(비-Linux 플랫폼에서는 `RV-172`의 `facade_remote.rs` 가드 병행 적용).

---

#### RV-174 — `io_loop`에서 `StorageError::Busy` 시 인덱스 전진 없이 재시도하여 무한 스핀 및 중복 손실 checkpoint 과다 기록 유발 (P2)

- **심각도 / 테마:** `P2` / 테마 1-A(동기화 경계), 테마 3-②/④(유한 작업량 및 디스크/세그먼트 회전 보호)
- **위치:** [logger/host.rs:885-927](../../../crates/dot-logger/src/logger/host.rs#L885-L927) (`io_loop`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `io_loop`의 `while index < syncs.len()` 루프에서 line 897이 먼저 `target.log.checkpoint(ns(origin, Instant::now()))`를 호출해 손실 checkpoint 청크를 기록·플러시한 뒤, line 903에서 `target.log.sync_work(syncs.swap_remove(index))`를 호출한다.
  2. RT 생산자나 호스트 디스패처가 동일 완료 슬롯을 `INITIALIZING`/`UPDATING`으로 잡고 있어 `sync_work`가 `Err(SyncWorkError { error: StorageError::Busy, work: Some(work) })`를 반환하면 line 908이 `syncs.push(work)`만 실행한다.
  3. `index`가 증가하지 않으므로(`syncs.len() == 1`이면 `swap_remove(0)` 후 `push` 뒤에도 `index = 0 < 1`) 루프는 sleep·배치 처리·`flags.stop` 확인 없이 즉시 다음 반복에 돌입하며, 매 스핀마다 line 897의 `target.log.checkpoint(...)`를 다시 호출해 중복 손실 checkpoint 청크를 계속 파일에 기록한다.
  4. **잘못된 결과:** 일시적 메타데이터 `Busy` 동안 I/O 스레드가 100% CPU로 공회전하며 세그먼트 손실 레포트/회전/디스크 예산을 불필요하게 소진할 수 있다.
- **확인 방법:** 코드 추적 추론 (`logger/host.rs:885-927` 및 `storage/writer.rs:101-110`).
- **수정 제안:**
  - `StorageError::Busy`로 `work`가 반환되면 현재 위치에 되돌린 뒤 `index += 1`로 진행해 `io_loop` 턴당 각 `SyncWork`를 1회씩만 시도하고, `Busy` 재시도 시에는 이미 직전에 수행한 중복 `checkpoint` 기록을 건너뛴다.

---

#### RV-175 — 다중 파일 동기화 증명(`SyncWork::covers_file`/`settled`)의 미동기화 타파일 `Written` 오승인 및 `FileLedger::settle` 구간 미축소 (P2)

- **심각도 / 테마:** `P2` / 테마 1-A(완료·동기화 내구성 증명), 테마 1-C(파일 회전·장부 정합성)
- **위치:**
  - [control/mod.rs:1007-1053](../../../crates/dot-logger/src/control/mod.rs#L1007-L1053) (`SyncWork::covers_file`, `SyncWork::settled`)
  - [storage/mod.rs:215-236](../../../crates/dot-logger/src/storage/mod.rs#L215-L236) (`FileLedger::owe`, `FileLedger::settle`)
  - [storage/writer.rs:330-334, 1054-1056, 1091-1094, 1115-1119](../../../crates/dot-logger/src/storage/writer.rs#L330-L334)
  - [storage/retention.rs:235-237, 766, 940](../../../crates/dot-logger/src/storage/retention.rs#L766)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. **시나리오 A(타파일 미동기화 레코드 허위 `Synced` 승인 — `control/mod.rs:1007-1053`):** 레코드 1(`seq = 1`)이 File 1에 기록(`Boundary::Written` 확인, 미동기화)된 뒤 선행 `SyncWork` 실패(`Failed(SyncFailure)`) 또는 드롭(`Unknown`)으로 레코드 1이 `confirmed = Enqueued | Written, outcome = Unknown | Failed(SyncFailure)` 상태가 된 후, File 2(`seq = 2`)가 열린다. 후속 배리어 `1..=2`가 File 2에 전달되면 `settled()`(`1042-1050행`)가 `status.confirmed`를 확인하지 않고 `QueryOutcome::Unknown`/`Failed`를 통과시키고(`Boundary::Synced` 계약이면 `1007행`이 `foreign.spans` 검사를 아예 생략), File 1이 미동기화인데도 File 2의 `sync_work`가 `SyncResult::Synced`로 승인된다.
  2. **시나리오 B(동기화 후 `first`/`last` 미초기화):** `SegmentLog::publish`(`retention.rs:766`), `SegmentLog::sync_work`(`940행`), `SegmentWriter::sync_work`(`writer.rs:1055`)가 동기화 성공 후 `self.dirty = false`만 내리고 `self.first`/`self.last`를 초기화하지 않는다. 이후 다른 생산자가 레코드 1건을 추가한 뒤 동기화 없이 종료/드롭되면 이미 동기화된 이전 세그먼트 범위까지 합쳐 `FileLedger::owe`에 미동기화로 등록된다.
  3. **시나리오 C(`FileLedger::settle` 구간 미축소):** 복수의 미동기화 `FinishedSegment`(File 1: `1..=5`, File 2: `6..=15`)가 있을 때 File 1이 `sync_work`로 먼저 동기화되어 `FileLedger::settle`(`storage/mod.rs:230-236`)이 호출되어도 `owed.files`만 `2 → 1`로 줄고 `owed.ranges`는 `(1, 15)`로 남는다. 이후 `1..=5`가 완료 윈도우에서 만료되면 File 2의 `sync_work(1..=15)`가 `foreign.spans`의 `(1, 5)` 만료 검사 때문에 영구 거절(`InvalidRecord`)된다.
- **확인 방법:** 코드 추적 추론 (`control/mod.rs`, `storage/mod.rs`, `storage/writer.rs`, `storage/retention.rs` 교차 추적).
- **수정 제안:**
  - `SyncWork::settled`에서 `status.confirmed.contains(Boundary::Written) && !status.confirmed.contains(Boundary::Synced)`이면 `Ok(false)`를 반환하고, `covers_file`에서 `ended_is_synced` 여부와 무관하게 `foreign.spans`를 검사한다.
  - 동기화 성공으로 `dirty = false`가 될 때 미동기화 추적 구간(`first`/`last`)을 초기화하고, `FinishedSegment::settle`이 `(&self.first, &self.last)`를 전달해 `FileLedger::settle`이 `owed.ranges`의 양 끝을 축소하게 한다.

---

#### RV-176 — 종료 중 전원 경보 시 선택 타깃 `EXPIRED` 오전이 및 `FacadeReport.unconfirmed` 누락 (P2)

- **심각도 / 테마:** `P2` / 테마 1-A(종료·비상 경계), 테마 3-③(정확한 종료/미확인 보고)
- **위치:**
  - [control/mod.rs:532-540, 766-775](../../../crates/dot-logger/src/control/mod.rs#L532-L540) (`ControlOwner::poll`, `ControlOwner::drive_shutdown`)
  - [logger/host.rs:629-642, 666-668, 763-794, 943](../../../crates/dot-logger/src/logger/host.rs#L629-L642) (`run`, `unconfirmed`, `facade_report`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. **비상 전원 차단 타이밍 레이스(`control/mod.rs:532-540`):** 비상 모드에서 `begin_shutdown(ShutdownPlan { sync_files: true })`가 먼저 호출되어 종료 슬롯의 비보호 선택 타깃이 `QUEUED`로 동결(`774행`)된 직후 `notify_power`가 도착하면, `ControlOwner::poll`(`532-539행`)이 해당 타깃을 `NONE`이 아닌 `EXPIRED`로 전이시킨다. 그 결과 보호 대상 필수 파일이 기한 내 모두 완료되어도 `ShutdownReport.sync_failed = true`(`is_success() == false`)로 실패 판정된다.
  2. **`FacadeReport.unconfirmed` 공백(`logger/host.rs:629-642, 666-668, 763-794`):** `unconfirmed(&report, &control, producers)`가 `ShutdownPoll::Closed` 직후(`639행`)에 호출되고 IO 스레드의 `target.log.finish(now)`(`943행`) 성공 여부를 기다리지 않으며, `ShutdownReport::is_success()`만 확인한다. 최종 세그먼트 마감이 실패(`io_failed = true`)해도 `report.unconfirmed`는 `Vec::new()`이고, `Shutdown::Incomplete` 경로(`632행`, `_control` 미사용)에서도 항상 빈 벡터가 반환된다.
- **확인 방법:** 코드 추적 추론 (`control/mod.rs` · `logger/host.rs` 교차 확인).
- **수정 제안:**
  - `ControlOwner::poll`에서 종료 전용 슬롯의 `emergency_skip` 비대상 타깃은 `NONE`으로 전환한다(`774행`과 일치).
  - `unconfirmed` 계산을 `facade_report` 내부로 통합해 `!(core.is_success() && !worker_failed && !io_failed)` 조건일 때 `control.shutdown_ranges`로 채운다.

---

#### RV-177 — `RemoteSubscription` `self.lost` 전파 누락·유휴 스트림 ACK 실패 감지 지연·EOF 시 대기 ACK 폐기 (P2)

- **심각도 / 테마:** `P2` / 테마 1-B(원격 클라이언트 ACK 상태 보존 및 파이프라인), 테마 3-③(타임아웃·`Unknown` 보고)
- **위치:**
  - [dot-logger-client/src/transport.rs:674-844](../../../crates/dot-logger-client/src/transport.rs#L674-L844) (`RemoteSubscription::flush`, `next`, `next_inner`, `handoff`)
  - [dot-logger-viewer/src/bridge.rs:281-282, 303-330](../../../crates/dot-logger-viewer/src/bridge.rs#L281-L282) (`Upstream::run`, `Upstream::forward`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. **`self.lost` 미검사(`transport.rs:709, 734, 801`):** `flush`와 달리 `next()`, `next_inner()`, `handoff()`가 진입 시 `self.lost`를 검사하지 않는다. 앞선 `handoff`나 `reap`에서 ACK RPC가 실패해 `self.lost = true, self.stream = None`이 된 뒤 후속 `next()`가 호출되면 `in_flight`가 비어 있고 `self.stream.is_none()`(`747행`)이므로 `Err(ClientError::AcknowledgementUnknown)` 대신 정상 EOF인 **`Ok(None)`**을 반환한다.
  2. **유휴 스트림 대기 시 `in_flight` 미폴링(`transport.rs:744-763`):** `handoff`가 마지막 레코드 ACK RPC를 `self.in_flight`에 스폰해 `self.acks`가 비어 있는 상태(`self.oldest == None`)에서 `next_inner()`를 호출해 `stream.message()`만 대기할 때, 수 밀리초 뒤 그 in-flight ACK RPC가 실패하더라도 새 프레임이 오지 않으면 `self.timeout`(기본 5초) 동안 멈춘 뒤 `AcknowledgementUnknown`이 아닌 `TimedOut`을 먼저 반환한다.
  3. **스트림 EOF 시 `self.acks` 폐기(`transport.rs:764-767`) & viewer `flush` 누락(`bridge.rs:281-282`):** 서버 스트림이 `Ok(None)`을 반환하면 `next_inner()`가 즉시 `self.close()`를 호출해 미전송 `self.acks`를 전송하지 않고 버리며 `self.lost = true`로 만든다(`flush()`를 호출하지 않는 수신자는 유실 인지 불가). viewer의 `Upstream::run`도 `forward` 종료 시 `subscription.flush` 없이 드롭한다.
- **확인 방법:** 코드 추적 추론 (`transport.rs:580-844` 및 `bridge.rs:280-330`).
- **수정 제안:**
  - `next_inner`와 `handoff` 진입 시 `if self.lost { return Err(self.unknown()); }`를 검사한다.
  - `next_inner`의 `tokio::select!`에서 `!self.in_flight.is_empty()`일 때 `self.settle_one()`을 함께 대기해 진행 중 ACK 실패를 즉시 `AcknowledgementUnknown`으로 드러낸다.
  - 스트림 EOF 시 대기 중인 `self.acks`를 전송·정산(`send_acks` + `in_flight` 수거)한 뒤 종료하거나 caller가 `flush`할 수 있게 보전하고, viewer bridge `Upstream::run`에서도 정상/재연결 전 `flush` 여부를 정리한다.

---

#### RV-178 — `RemoteHub` `Subscription::expire`의 waker 미깨움 및 `SubscriptionStream` 타이머 밀림 (P2)

- **심각도 / 테마:** `P2` / 테마 1-B(구독 lease 및 스트림 해제), 테마 3-③(오류 분류 정확성)
- **위치:**
  - [adapters/remote/hub.rs:185-203](../../../crates/dot-logger/src/adapters/remote/hub.rs#L185-L203) (`Subscription::expire`)
  - [adapters/remote/server.rs:462-494](../../../crates/dot-logger/src/adapters/remote/server.rs#L462-L494) (`SubscriptionStream::poll_next`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `records = 2, age = 10s` 구독에서 `t = 0s`에 레코드 1(`expires = 10s`), `t = 6s`에 레코드 2(`expires = 16s`)가 전송되면 `SubscriptionStream::poll_next`(`server.rs:465`)는 `self.timer`를 `t = 16s`로 리셋하고 `s.waker`를 저장한 채 `Poll::Pending`에 들어간다.
  2. `t = 11s`에 레코드 1이 미ACK 상태로 만료될 때 `offer`/`ack`/`discard_generations`가 `Subscription::expire`를 호출해 `s.closed = true`로 설정하지만 `s.waker.take()`를 깨우지 않는다(`Subscription::close`는 `w.wake()`를 호출하나 `expire`는 누락).
  3. **잘못된 결과:** 구독 스트림과 `Registry` 슬롯이 최대 `age`(16s까지) 더 점유된 채 대기하다가 `t = 16s`에 `self.timer`가 발동해 `RemoteError::Gone`(`NotFound`)이 아닌 `RemoteError::TimedOut`(`DeadlineExceeded`)으로 종료된다.
- **확인 방법:** 코드 추적 추론 (`hub.rs:171-242` 및 `server.rs:449-496`).
- **수정 제안:**
  - `Subscription::expire`에서 `s.closed = true` 전환 시 `if let Some(w) = s.waker.take() { w.wake(); }`를 호출해 `SubscriptionStream`이 즉시 깨어나 `RemoteError::Gone`을 반환하도록 한다.

---

#### RV-179 — Viewer `dashboard::serve`의 `work` 배경 태스크 영구 누수 및 `websocket::negotiate` `Host`·`Origin` 포트 검증 공백 (P2)

- **심각도 / 테마:** `P2` / 테마 1-B(viewer), 테마 2-①(HTTP Host/Origin DNS rebinding 방어), 테마 3-④(태스크 누수)
- **위치:**
  - [dot-logger-viewer/src/dashboard.rs:290-305, 319-339](../../../crates/dot-logger-viewer/src/dashboard.rs#L290-L305) (`serve`, `work`)
  - [dot-logger-viewer/src/main.rs:256-274](../../../crates/dot-logger-viewer/src/main.rs#L256-L274) (`run`)
  - [dot-logger-viewer/src/websocket.rs:127-175](../../../crates/dot-logger-viewer/src/websocket.rs#L127-L175) (`negotiate`, `local_origin`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. **태스크 영구 누수(`dashboard.rs:290-339`):** `main.rs:259`가 `stop = std::future::pending()`으로 `dashboard::serve`를 스폰하고 종료 시 `dashboard.abort()`(`273행`)를 호출하거나, 시험/라이브러리 호출자가 `dashboard::serve` future를 드롭하면 `listen(...).await` 지점에서 취소되어 line 303의 `worker.abort()`에 도달하지 못하고 `worker` `JoinHandle`이 detach된다(CLI 프로세스 종료 시에는 OS 프로세스 수거에 가려지지만, 동일 Tokio 런타임 내 다회 실행 시 영구 잔류). 게다가 `work`(`324-327행`)는 `Some(job) = queue.recv()` refutable 패턴을 써서 모든 sender가 드롭(`None`)되어도 종료하지 않고 해당 브랜치만 비활성화한 채 영구히 1초마다 `GetStatus`를 폴링한다.
  2. **WS 핸드셰이크 `Host`/`Origin` 포트 검증 공백(`websocket.rs:127-175`):** `dashboard.rs:462-470`과 달리 `websocket::negotiate`는 `Host` 헤더를 전혀 검증하지 않으며(`Origin` 없는 요청이나 `--allow-origin` 환경에서 비-loopback `Host` 통과), `local_origin`(`157-175행`)은 `origin.parse::<Uri>()`에만 의존해 `http://localhost:99999`처럼 범위 초과 포트를 `port() == None`인 유효 `Origin`으로 통과시킨다.
- **확인 방법:** 코드 추적 추론 (`dashboard.rs`, `websocket.rs`, `main.rs` 직접 대조).
- **수정 제안:**
  - `dashboard.rs`에서 `worker`를 `Drop` 시 `abort()`하는 가드로 감싸고 `work`에서 `queue.recv()`가 `None`이면 즉시 `return`하도록 수정하며 `main.rs`에 정상 `stop` 신호를 연결한다.
  - `websocket.rs::negotiate`에도 `Host` 헤더 loopback 검증을 추가하고 `local_origin`의 포트 파싱을 `1..=65535` 정수로 엄격히 검증한다.

---

#### RV-180 — Vendored `mcap-0.26.0` 판독기의 손상된 `Attachment`/인덱스 슬라이스 패닉 및 `McapString` u32 길이 선할당 (P2)

- **심각도 / 테마:** `P2` / 테마 1-C(MCAP 파싱 안전성), 테마 2-①/③(신뢰할 수 없는 입력 패닉/OOM 방지)
- **위치:**
  - [vendor/mcap-0.26.0/src/read.rs:170-177, 716-725, 800-805, 849-858, 926-931, 954-959](../../../vendor/mcap-0.26.0/src/read.rs#L170-L177)
  - [vendor/mcap-0.26.0/src/records.rs:142-152, 163-175, 226-245, 293-313](../../../vendor/mcap-0.26.0/src/records.rs#L142-L152)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `dot-logger`의 `NativeReader`/`LossReader`/`Snapshot`은 `Fields` 선검사로 보호되지만, `vendor/mcap-0.26.0`의 일반 reader API나 `parse_record(op::ATTACHMENT, body)`(`read.rs:176`)가 손상 입력을 받으면 `body.len() - 4 < header_len`일 때 `&body[header_len..body.len() - 4]`가 슬라이스 패닉을 일으킨다.
  2. `Summary::stream_chunk`(`716행`), `read_message_indexes`(`800행`), `seek_message`(`849행`), `attachment`(`926행`), `metadata`(`954행`)의 `offset + length`와 `+ 9..end`가 unchecked 덧셈/슬라이싱이어서 `chunk_length < 9` 또는 `u64` 오버플로 시 패닉한다.
  3. `records.rs:142-152`의 `McapString`이 `#[br(count = len)]`(`len: u32`)을 사용해 작은 레코드에 `len = 0xffff_ffff`를 넣으면 바이트 잔량 확인 전 4 GiB 할당을 시도하며, `parse_vec`/`parse_string_map`/`parse_int_map`이 루프 종료 후 `byte_len` 경계 정확 일치를 확인하지 않는다.
- **확인 방법:** 코드 추적 추론 (`vendor/mcap-0.26.0/src/read.rs`, `records.rs` 및 `storage/read.rs:348-350` 주석 대조).
- **수정 제안:**
  - `read.rs`의 오프셋 연산·슬라이싱을 `checked_add`/`checked_sub`/`.get(start..end)`로 교체하고, `records.rs` `McapString`의 잔여 바이트 선검사 및 컬렉션 파서 끝 위치 `== byte_len` 검증을 보완한다.

---

#### RV-181 — Native MCAP `op == 7` 선행 허용·`_ => continue` 및 `Snapshot::scan` `safe_offset == 0` 검증 우회 (P2)

- **심각도 / 테마:** `P2` / 테마 1-C(컨테이너 검증·복구/조회 계약), 테마 3-①
- **위치:**
  - [storage/read.rs:227, 339](../../../crates/dot-logger/src/storage/read.rs#L227) (`NativeReader::next_item`, `registrations`)
  - [storage/recovery.rs:305-319, 472](../../../crates/dot-logger/src/storage/recovery.rs#L305-L319) (`recover_inner`)
  - [query/mod.rs:530-532, 635, 647-655](../../../crates/dot-logger/src/query/mod.rs#L530-L532) (`Snapshot::scan`, `loss_unavailable_sources`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. **첫 `Chunk`·`Metadata` 이전 `MessageIndex`(`op == 7`) 허용 및 `read.rs:339` `_ => continue`:** `NativeReader::next_item`, `recover_inner`, `Snapshot::scan` 모두 `op == 7`을 `chunk_seen` 검사 없이 통과시키고 `read.rs:339`는 미지 opcode를 건너뛴다. 특히 `recover_inner`(`recovery.rs:305`)는 `Header` 직후 단일 프레임만 `op == 12`인지 peek해 `budget.recovered_file`(`loss_rows`)를 결정하므로, `loss_rows > 0` 파일에서 `Header`와 `Metadata`(`op == 12`) 사이에 `op == 7`이 있으면 `loss_parts = 0`으로 초기화된 뒤 본 스캔에서 `op == 7`과 `op == 12`가 통과되고 첫 손실 보고에서 `StorageError::NeedRotation`(`Err(RecoveryError { stop: Limit })`)으로 실패한다(`Ok(RecoveredSegment { stop: InvalidRecord })` 위반).
  2. **`Snapshot::scan`의 `end == 0` 조기 반환(`query/mod.rs:530-532`):** `safe_offset == 0`인 소스는 헤더/메타 검증과 `loss_rows`/`actual_target` 설정(`647-648행`) 전에 `Ok(())`로 반환되어, 중복 `(sequence, actual_target)` 검출(`485-491행`)을 우회하고 `loss_unavailable_sources()`(`654행`)에 손실 수집 불가 소스로 잘못 집계된다.
- **확인 방법:** 코드 추적 추론 (`storage/read.rs`, `storage/recovery.rs`, `query/mod.rs` 직접 대조).
- **수정 제안:**
  - `read.rs:339`의 `_ => continue`를 `Err(StorageError::CorruptData)`로 바꾸고, `op == 7`은 `chunk_seen == true`일 때만 허용한다.
  - `query/mod.rs:530`에서 `end < 8`을 모두 `QueryError::CorruptData`로 거절해(`safe_offset == 0` 허용 시에도 최소 헤더/메타 또는 호출부 계약을 분리) `actual_target`/`loss_rows` 미설정 누락을 없앤다.

---

#### RV-182 — `loss::summarize`의 part 간 `counter_key` 중복 미검사 및 `LossReport::decode`/`LossRange::holds` 범위 경계 공백 (P2)

- **심각도 / 테마:** `P2` / 테마 1-C(손실 checkpoint 정합성), 테마 2-①(손상 입력 검증)
- **위치:** [loss.rs:147-153, 593-610, 799-908](../../../crates/dot-logger/src/loss.rs#L147-L153) (`LossRange::holds`, `LossReport::decode`, `summarize`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. **`summarize`의 part 간(및 직접 호출 시 단일 part 내) `counter_key` 중복 미검사(`loss.rs:860-908`):** `LossReport::decode`는 단일 part 내부의 `counter_key` 중복만 검사하고, `summarize`는 다중 part checkpoint 그룹(`groups[at..end]`)의 완결성 판정에서 part 간 `counter_key` 중복(또는 `pub fn summarize` 직접 입력의 단일 part 내 중복)을 검사하지 않는다. 손상된 2-part checkpoint에서 `part 1`이 `part 0`의 키 `K_0`를 중복 수록하고 마지막 키 `K_64`를 누락해도 `complete = true`·`quiescent = true`로 판정되어 `K_64`가 `0`으로 합성되고 다른 키들에 허위 `delta`·`tail_confirmed = true`가 생성된다.
  2. **`LossReport::decode`의 빈 v1 허용 및 `LossRange::holds`의 `part: u32` 경계 미대조(`loss.rs:147-153, 593-610`):** `LossReport::decode`가 `range: None, counts: []`인 빈 v1 레포트를 거절하지 않으며(`summarize` `800행`에서는 `CorruptData`), `LossRange::holds`가 `part` 인덱스를 받지 않아 `u64::from(part) * ROWS_PER_REPORT + listed as u64 <= self.end` 및 `self.end < self.rows && part + 1 == parts` 같은 변조를 단일 part 디코딩에서 거절하지 못한다(단, `REV-069` `RV-164`에서 결정된 `end == rows` 빈 후행 part 호환성은 그대로 유지).
- **확인 방법:** 코드 추적 추론 (`loss.rs:143-155, 587-689, 787-915` 및 [REV-069](REV-069_T7_STORAGE_REMOTE_ACK_CRC_REVIEW.md#L124-L129) 교차 확인).
- **수정 제안:**
  - `summarize`에서 동일 checkpoint 그룹 내(및 입력 part 내) `counter_key` 중복이 발견되면 해당 checkpoint를 비완전 처리하거나 `CorruptData`로 거절한다.
  - `LossReport::decode`에서 `counts.is_empty() && range.is_none()`을 거절하고, `LossRange::holds`에 `part: u32`를 전달해 `u64::from(part) * 64 + listed as u64 <= self.end`와 `self.end == self.rows || (listed == ROWS_PER_REPORT && part + 1 < parts)`를 검증한다.

---

#### RV-183 — 저장소 공개 전 개인 로컬 경로(`/home/lim8603`, `/data/build`) 잔류 및 vendored `mcap` 패치 부정합 (P2)

- **심각도 / 테마:** `P2` / 테마 2-⑥(공급망·패치 추적성), 테마 3-⑥(외부 재현성)
- **위치:**
  - [vendor/mcap-0.26.0/single-crc.patch:1-24](../../../vendor/mcap-0.26.0/single-crc.patch#L1-L24), [Cargo.toml:46-49](../../../Cargo.toml#L46-L49)
  - [scripts/run-perf-rt.sh:26](../../../scripts/run-perf-rt.sh#L26), [scripts/run-perf-s3.sh:28](../../../scripts/run-perf-s3.sh#L28)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `vendor/mcap-0.26.0/single-crc.patch:1`에 로컬 홈 경로(`/home/lim8603/.cargo/registry/...`)가 그대로 남아 있고, [VENDORED.md](../../../vendor/mcap-0.26.0/VENDORED.md)에 적힌 3개 패치(EV-068 단일 CRC, RV-136 `linear_reader.rs` 경계 강화, TASK-031 S3 `chunk_registrations`) 중 1개만 줄 번호가 15줄 어긋난 상태로 수록되어 있으며 `Cargo.toml:46-49` 주석도 TASK-031 S3가 빠져 있다.
  2. `scripts/run-perf-rt.sh:26`과 `scripts/run-perf-s3.sh:28`이 `PERF_BASELINE` 기본값을 개발자 PC 전용 경로(`/data/build/target/release/examples/perf_baseline`)로 하드코딩해 외부 환경에서 환경변수 없이 실행하면 즉시 실패한다.
- **확인 방법:** 파일 직접 읽기 및 upstream `mcap-0.26.0` 대조 확인.
- **수정 제안:**
  - `single-crc.patch`에서 `/home/lim8603` 경로를 제거하고 3개 변경을 모두 포함한 clean unified diff(`a/src/...` → `b/src/...`)로 갱신(또는 제거 후 `VENDORED.md`에 통합)하며 `Cargo.toml:46-49` 주석을 동기화한다.
  - 두 성능 스크립트에서 저장소 기준 `${PERF_BASELINE:-${CARGO_TARGET_DIR:-$repo/target}/release/examples/perf_baseline}`을 기본값으로 사용한다.

---

#### RV-184 — `routed_check_host.rs` · `routed_host.rs`의 Unix 전용 `/nonexistent/...` 경로 하드코딩으로 Windows 테스트 실패 (P2)

- **심각도 / 테마:** `P2` / 테마 3-⑥(multi-OS CI 정합성)
- **위치:**
  - [crates/dot-logger-cli/tests/routed_check_host.rs:84](../../../crates/dot-logger-cli/tests/routed_check_host.rs#L84)
  - [crates/dot-logger-client/tests/routed_host.rs:17, 45](../../../crates/dot-logger-client/tests/routed_host.rs#L17)
  - [crates/dot-logger/src/diagnostics/mod.rs:148-156](../../../crates/dot-logger/src/diagnostics/mod.rs#L148-L156) (`CheckPlan::build`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  - `CheckPlan::build`는 `if !spec.directory.is_absolute()`일 때 `Err(PlanError::InvalidDirectory)`를 반환한다. Windows에서는 드라이브/UNC 접두사가 없는 `"/nonexistent/..."`의 `is_absolute()`가 `false`이므로, `RV-173` 조치 후 `windows-2025`에서 CLI/client 테스트를 실행하면 두 테스트가 `.unwrap()`에서 패닉한다.
- **확인 방법:** `CheckPlan::build`(`diagnostics/mod.rs:148`) 및 두 테스트 파일 직접 확인.
- **수정 제안:**
  - `std::env::temp_dir().join("dot-logger-routed-check-never-created")`로 변경해 모든 OS에서 유효한 비존재 절대경로를 사용한다.

---

#### RV-185 — `Boundary::Enqueued` 타깃의 정상 드롭 유기 오인 및 폐기 후 `Complete` 오집계 (P3)

- **심각도 / 테마:** `P3` / 테마 1-A(완료·손실 경계)
- **위치:** [completion/mod.rs:310-322, 980-998, 1013-1034](../../../crates/dot-logger/src/completion/mod.rs#L310-L322) (`Registry::effective`, `AbandonMarker::drop`, `classify`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `completion: Boundary::Enqueued`로 구성된 타깃에서 소비자가 정상 처리 뒤 `Delivery`를 드롭하면 `AbandonMarker::drop`(`992행`)과 `Registry::effective`(`315행`)가 `0b11110`만 검사하므로 미보고 유기(`Unknown`)로 오인해 `reconcile` 전체 스윕과 `loss::Bank`의 허위 `Cause::Unconfirmed` 손실을 기록한다.
  2. 반대로 `Enqueued` 확인 후 `TargetConsumer::try_next`에서 취소·만료로 `Update::Dropped(Cancelled | Expired)`가 기록된 레코드는 `loss::Bank`에는 손실로 기록되지만 `classify`(`1017행`)가 `confirmed.contains(boundary)`를 `TERM_MASK`보다 먼저 검사해 `Completion::totals()`에서 `Complete`로 집계된다.
- **확인 방법:** 코드 추적 추론 (`completion/mod.rs:310-329, 571-583, 980-1034`).
- **수정 제안:**
  - 유기 판별 마스크에 `Boundaries::only(self.targets[target].completion).0`을 포함하고, `classify`에서 `Boundary::Enqueued` 계약에 `Dropped` 종료 상태가 함께 기록된 경우 종료 원인을 우선 반영하도록 정리한다.

---

#### RV-186 — CLI `recover` 중복 sequence 사전 검증 누락 및 원격 `Query` `blob`/`snapshot` 일관성 공백 (P3)

- **심각도 / 테마:** `P3` / 테마 1-B(CLI·원격 wire 검증), 테마 2-①
- **위치:**
  - [dot-logger-cli/src/model.rs:179-187](../../../crates/dot-logger-cli/src/model.rs#L179-L187), [commands.rs:530-565, 747-782](../../../crates/dot-logger-cli/src/commands.rs#L530-L565)
  - [dot-logger-client/src/codec.rs:220-224](../../../crates/dot-logger-client/src/codec.rs#L220-L224), [adapters/remote/wire.rs:439-441](../../../crates/dot-logger/src/adapters/remote/wire.rs#L439-L441)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `Manifest::validate`는 중복 `sequence`가 없는 다른 소스의 `target: None`을 허용해 `commands::recover`가 출력 디렉터리 생성 및 앞선 파일 복구를 마친 뒤에야(`559행`) `Failure::Input`으로 실패한다.
  2. `codec::reply`·`wire::Reply::native`가 `item.blob != 0 && record.payload_len == 0`을 거절하지 않으며, CLI `remote "query"`(`747-750행`)는 연속 페이지 간 `snapshot` ID 변경(`snapshot != 0 && current != snapshot`)을 검사하지 않는다.
- **확인 방법:** 코드 추적 추론 (`model.rs`, `commands.rs`, `codec.rs`, `wire.rs` 직접 대조).
- **수정 제안:**
  - `repeated_sequences && sources.iter().any(|s| s.target.is_none())`를 `create_dir` 전에 거절하고, `Query` 항목에서 `blob != 0 && payload_len == 0`과 페이지 간 `snapshot` 변경을 `Corrupt`로 거절한다.

---

#### RV-187 — `QueryConfig::reservation`의 `channels.len()` 상한 및 `PayloadSchema::name` 바이트 누락 (P3)

- **심각도 / 테마:** `P3` / 테마 1-C(메타데이터 예약 일관성)
- **위치:** [query/mod.rs:341, 345-360](../../../crates/dot-logger/src/query/mod.rs#L341) (`QueryConfig::reservation`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  - `WriterConfig::reservation`/`registration_bytes`(`writer.rs:122-171, 224`)가 `channels.len() <= u16::MAX - 2` 및 `s.name.len()` 포함 합산을 적용하는 반면, `QueryConfig::reservation`은 `u16::MAX`까지 허용하고 `s.name` 길이를 누락해 `max_metadata_bytes` 검사가 writer/reader와 어긋난다.
- **확인 방법:** `query/mod.rs:328-360`과 `storage/writer.rs:122-171` 직접 대조.
- **수정 제안:**
  - `QueryConfig::reservation`에서도 `crate::storage::writer::registration_bytes(channels)`(및 `u16::MAX as usize - 2`)를 공통 사용한다.

---

#### RV-188 — `writer.rs` Miri ignore 누락·`--skip logger::` 과잉 제외, 8개 테스트 `/tmp` RAII 정리 누락 및 `local-pc.toml` 정합성 (P3)

- **심각도 / 테마:** `P3` / 테마 3-⑤/⑥(시험 신뢰성·Miri 범위·프로파일 정합성)
- **위치:**
  - [logger/writer.rs:394, 399](../../../crates/dot-logger/src/logger/writer.rs#L394), [.github/workflows/ci.yml:89](../../../.github/workflows/ci.yml#L89)
  - `/tmp` 수동 삭제(패닉 시 잔류) 8곳: `fault_storage.rs:481`, `tc_010_check_host.rs:23`, `tc_010_diagnostics.rs:41`, `tc_013_affinity.rs:65`, `dot-logger-client/tests/routed_check.rs:122`, `dot-logger-cli/tests/cli.rs:1197,1264`, `tests/api-consumer/src/main.rs:405`, `tests/rt-allocation/src/tests.rs:735,820,899,983,1709`
  - [test-profiles/local-pc.toml:4, 57-58, 99-106](../../../test-profiles/local-pc.toml#L4)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `writer.rs:399`에 `#[cfg_attr(miri, ignore)]`가 빠져 `ci.yml:89`가 `-- --skip logger::`로 순수 인메모리 `logger::*` 단위 테스트까지 모두 건너뛰며, `--skip` 없이 Miri 실행 시 directory fsync 중단으로 `/tmp/dot-logger-gap009-*`가 잔류한다.
  2. 8개 테스트가 `Drop` 가드 없이 수동 `remove_dir_all`/`remove_file`만 두어 실패/패닉 시 `/tmp` 잔류물이 남는다.
  3. `test-profiles/local-pc.toml`에 `execution = "NotRun"`, `performance_harness = "NotImplemented"`, `actual_dot_probe_control = "NotIntegrated"`, `ups_adapter = "NotImplemented"`가 남아 있어 `python3 scripts/check-test-profile.py test-profiles/local-pc.toml --require-ready` 실행 시 exit code `2`로 실패한다.
- **확인 방법:** 직접 실행 확인(`check-test-profile.py` exit `2`, `grep -rn "temp_dir("` 전수 조사, `writer.rs`·`ci.yml` 직접 대조).
- **수정 제안:**
  - `writer.rs:399`에 `#[cfg_attr(miri, ignore)]` 추가 후 `ci.yml:89`에서 `-- --skip logger::` 제거, 8개 테스트에 RAII `Drop` 임시 경로 가드 적용, `test-profiles/local-pc.toml` 상태 선언 갱신.

---

## 5. 검증 및 인계

### 이번 리뷰 세션에서 수행한 읽기 전용 확인
| 확인 명령 | 결과 |
|---|---|
| `cargo +1.96.0 fmt --all --check` | Pass |
| `python3 scripts/check-remote-protocol.py` | Pass (`Protocol: 10 messages / 89 fields match in server and client; protoc Pass`) |
| `python3 scripts/test_check_integration_surface.py` | Pass (18 tests in 1.829s) |
| `python3 scripts/check-test-profile.py test-profiles/local-pc.toml --require-ready` | Exit `2` (`NotImplemented`/`NotIntegrated` 확인 — `RV-188`) |
| 워크스페이스 전체 `unsafe` / `file_output` / `temp_dir` 전수 grep 및 코드 경로 대조 | 완료 (`RV-171~188` 근거 확보) |
| 제품 코드·테스트·스크립트·CI 설정 변경 | **0건** (Strict Read-Only 준수) |

### 개발 세션 인계 (`REV-071` → Gate 4 수락 → 공개 전환)

- **Gate 4 수락 및 저장소 공개 전환(`gh repo edit --visibility public`) 전**, 다음 개발 세션에서 **우선순위에 따라 `RV-171~188`을 TDD(실패 재현 테스트 작성 → 최소 수정 → 회귀 검증)로 처리**한다.
  - 특히 **`RV-171~173` (P1)**과 **`RV-174~184` (P2)**는 런타임 장기 안정성, 공개 후 T3 multi-OS CI 통과, 보안/입력 검증 및 개인 경로 제거에 직결되므로 Gate 4 최종 수락 전 반드시 해결한다.
  - `RV-185~188` (P3)도 함께 정리해 `Open` 리뷰가 없는 상태로 `REV-071`을 `Closed`로 마감한 뒤, [TASK-025](../../04_implementation/tasks/TASK-025.md) S4 및 Gate 4 수락과 공개 전환 절차를 진행한다.

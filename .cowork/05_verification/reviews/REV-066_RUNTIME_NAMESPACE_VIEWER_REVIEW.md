# REV-066 — REV-059 조치·runtime 통합(D/E)·.logger 예약 토픽·TASK-031 viewer(S2/S3) 통합 코드리뷰

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-066 / 2026-11-18 |
| 리뷰어 | Antigravity AI — 리뷰 전용 세션 |
| 대상 | REV-059 이후 누적 커밋 `ae3b6b3..3f8a46f` (7개 코드 커밋: `8b59aa6`, `874c27c`, `be20250`, `eeae5a3`, `f1444d4`, `a3296dc`, `753794f`) |
| 범위 | REV-059 결함 조치(`8b59aa6`), TASK-034 D/E runtime 채널 설정·native/loss 통합 및 tracing 회귀(`874c27c`, `be20250`), ADR-013 `.logger` 예약 토픽·`standard/2` 개정(`eeae5a3`), TASK-031 S2 실시간 Foxglove WS v1 viewer(`f1444d4`), S3 MCAP 최상위 등록 배치(`a3296dc`), S3 Query/ReadBlob 원격 기록 재생(`753794f`) |
| 종합 판정 | **Conditional** — P1 0 · P2 3 · P3 1 (`RV-156~159`) |
| 상태 | **Open** |

정적 분석(`cargo +1.96.0 fmt --all --check`, `clippy --workspace --all-targets --all-features --locked -- -D warnings`), 워크스페이스 전체 테스트(`cargo +1.96.0 test --workspace --all-features --locked`: 692 passed, 0 failed, 5 ignored — RT 할당 20 passed, viewer 6 passed, doctests 24 passed 포함), strict `rustdoc`(`1.96.0`, `1.85.0` 모두 Pass), `scripts/check-integration-surface.py`(facade 이름 있는 항목 35/60, L1~L3 레시피·실행·README 일치)는 모두 통과했다.

`ae3b6b3..3f8a46f` 구간의 제품 코드를 ADR-013·ADR-014·API §22~24 및 RT/동시성/예산 계약과 대조해 정밀 검토한 결과 다음 4건(P2 3건, P3 1건)을 확인했다:
1. **P2 (`RV-156`):** `dot-logger-viewer` 실시간 bridge에서 `Bridge::subscribe`와 `Lease::drop` 간 동기 해제·즉시 재구독 레이스(`options.feeds` 경계에서 불필요한 구독 거절 및 `LiveBudget` 경합), 스키마 불일치 은퇴 시 `Upstream::run`이 오해 소지 `Notice::Unknown`을 추가 발행하는 문제, 그리고 `Notices`의 채널 구분 없는 단일 슬롯 덮어쓰기 문제를 확인했다.
2. **P2 (`RV-157`):** `dot-logger-viewer` 원격 기록 재생(`replay.rs`)에서 `play` 실패 후 닫히거나 미바인딩·만료된 `RemoteRecording`을 `None`으로 초기화하지 않아 이후 `seek`가 영구 실패하는 문제, 단일 채널 스키마 불일치 시 해당 채널 은퇴·`unadvertise` 대신 전체 재생이 중단되는 문제, 그리고 `--replay-blob-bytes` 허용 상한(`16 MiB`)이 `RemoteConfig.message_bytes`(기본 `256 KiB + 1024`)에 반영되지 않아 무음 절삭되는 문제를 확인했다.
3. **P2 (`RV-158`):** 최상위 MCAP 등록 배치(`a3296dc`) 전환 이후 `storage/recovery.rs`의 `recover_inner`가 등록 레코드 누락·절단 파일의 `EndOfInput`/`DataEnd`를 `InvalidRecord`로 거절하지 못하는 문제, `storage/read.rs`의 `registrations`가 `.logger/meta`·`.logger/loss` 제어 채널 및 사용자 채널 `metadata` 일부를 즉시 검증하지 않는 문제, 그리고 `Validation::schema`/`channel`의 중복 등록 차단이 불완전한 문제를 확인했다.
4. **P3 (`RV-159`):** `logger/mod.rs:38`의 `rustdoc` `redundant_explicit_links` 경고 1건(pinned nightly doc 체크 시 발생)과 `logger/host.rs:538-552`의 `worker_failed` 종료 감지 순서 정렬을 권고한다.

리뷰 전용 세션 원칙(Strict Read-Only)에 따라 제품 코드·저장소 테스트는 수정하지 않았다.

## 2. 파일 변경표

### 이번 리뷰 세션 산출물

| 구분 | 파일 | 변경 내용 |
|---|---|---|
| 생성 | [REV-066](REV-066_RUNTIME_NAMESPACE_VIEWER_REVIEW.md) | `ae3b6b3..3f8a46f` 통합 코드리뷰 결과(`RV-156~159`) 및 처리담당 영역 기록 (`Open`) |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | 미처리 리뷰 목록·코드리뷰 현황에 REV-066 (`Open`, P2 3·P3 1) 등록 및 다음 시작점 동기화 |
| 수정 | [my_state.md](../../members/lim8603/workspace/my_state.md) | 개인 상태 인덱스 리뷰·세션·다음 시작점 동기화 |
| 생성 | [session_2026-11-18_004.md](../../members/lim8603/workspace/session_logs/session_2026-11-18_004.md) | 코드리뷰 세션 로그 (`2026-11-18 #004`) |

### 검토 대상 코드 파일

- **viewer (`crates/dot-logger-viewer/`):** [main.rs](../../../crates/dot-logger-viewer/src/main.rs), [catalog.rs](../../../crates/dot-logger-viewer/src/catalog.rs), [bridge.rs](../../../crates/dot-logger-viewer/src/bridge.rs), [output.rs](../../../crates/dot-logger-viewer/src/output.rs), [websocket.rs](../../../crates/dot-logger-viewer/src/websocket.rs), [replay.rs](../../../crates/dot-logger-viewer/src/replay.rs), [tests.rs](../../../crates/dot-logger-viewer/src/tests.rs), [live_host.rs](../../../crates/dot-logger-viewer/examples/live_host.rs)
- **client (`crates/dot-logger-client/`):** [lib.rs](../../../crates/dot-logger-client/src/lib.rs), [live.rs](../../../crates/dot-logger-client/src/live.rs), [recording.rs](../../../crates/dot-logger-client/src/recording.rs), [transport.rs](../../../crates/dot-logger-client/src/transport.rs), [facade_remote.rs](../../../crates/dot-logger-client/tests/facade_remote.rs)
- **core / cli / vendor:** [logger/mod.rs](../../../crates/dot-logger/src/logger/mod.rs), [logger/host.rs](../../../crates/dot-logger/src/logger/host.rs), [logger/remote.rs](../../../crates/dot-logger/src/logger/remote.rs), [logger/sink.rs](../../../crates/dot-logger/src/logger/sink.rs), [logger/writer.rs](../../../crates/dot-logger/src/logger/writer.rs), [logger/budget.rs](../../../crates/dot-logger/src/logger/budget.rs), [control/config_update.rs](../../../crates/dot-logger/src/control/config_update.rs), [capture/mod.rs](../../../crates/dot-logger/src/capture/mod.rs), [loss.rs](../../../crates/dot-logger/src/loss.rs), [storage/codec.rs](../../../crates/dot-logger/src/storage/codec.rs), [storage/mod.rs](../../../crates/dot-logger/src/storage/mod.rs), [storage/read.rs](../../../crates/dot-logger/src/storage/read.rs), [storage/writer.rs](../../../crates/dot-logger/src/storage/writer.rs), [storage/recovery.rs](../../../crates/dot-logger/src/storage/recovery.rs), [commands.rs](../../../crates/dot-logger-cli/src/commands.rs), [vendor write.rs](../../../vendor/mcap-0.26.0/src/write.rs)

## 3. 클래스(타입) 변경표

**이번 리뷰 세션의 제품 코드 변경은 없음.** 검토 대상 핵심 타입(`struct` · `enum`)은 아래와 같다.

| 구분 | 소속 파일 | 타입 이름 | 역할 및 검토 요약 |
|---|---|---|---|
| 검토 | [main.rs](../../../crates/dot-logger-viewer/src/main.rs) | `Command`, `Options` | viewer CLI 인자·리소스/Origin 제한 검증 (`RV-157`) |
| 검토 | [catalog.rs](../../../crates/dot-logger-viewer/src/catalog.rs) | `Catalog`, `Channel`, `FileCatalog`, `FileChannel`, `Encoding` | 로컬 채널·스키마 매핑, 은퇴(`retired`) 플래그, WS 광고 직렬화 |
| 검토 | [bridge.rs](../../../crates/dot-logger-viewer/src/bridge.rs) | `Bridge`, `Upstream`, `Registry`, `Feed`, `FeedState`, `Lease` | 채널별 상류 gRPC 공유 구독·재연결·ACK·수명 (`RV-156`) |
| 검토 | [output.rs](../../../crates/dot-logger-viewer/src/output.rs) | `Notice`, `Notices`, `Outbox`, `Destination`, `Frame` | 소켓별 유한 건수/바이트 큐 및 Gap/Unknown/Unsupported 상태 집약 (`RV-156`) |
| 검토 | [websocket.rs](../../../crates/dot-logger-viewer/src/websocket.rs) | `Command`, `Subscription`, `Control` | Foxglove WS v1 handshake·Origin 검사·읽기 전용 구독 제어 |
| 검토 | [replay.rs](../../../crates/dot-logger-viewer/src/replay.rs) | `Rate`, `Position`, `Playback`, `Clock`, `Interest`, `Output`, `Progress`, `Driver` | 터미널 `seek`/`speed`, session mono 시계, 세대 무효화·원격 재생 (`RV-157`) |
| 검토 | [live.rs](../../../crates/dot-logger-client/src/live.rs) | `LiveBudget`, `LiveRecord`, `LiveSubscription` | viewer용 공유 수신 예약 및 단일 채널 gRPC 구독/handoff 래퍼 |
| 검토 | [recording.rs](../../../crates/dot-logger-client/src/recording.rs) | `RemoteRecording`, `RecordedPage`, `RecordedItem` | 고정 snapshot `Query`/`ReadBlob` 및 연결 소유권 검증 (`RV-157`) |
| 검토 | [logger/mod.rs](../../../crates/dot-logger/src/logger/mod.rs), [host.rs](../../../crates/dot-logger/src/logger/host.rs), [writer.rs](../../../crates/dot-logger/src/logger/writer.rs) | `Logger`, `Writer`, `ChannelOptions`, `ConfigError`, `Held`, `Configure` | runtime `set_channel`/`refresh`·단일 pending 슬롯·무대기 ack (`RV-159`) |
| 검토 | [storage/read.rs](../../../crates/dot-logger/src/storage/read.rs), [recovery.rs](../../../crates/dot-logger/src/storage/recovery.rs), [writer.rs](../../../crates/dot-logger/src/storage/writer.rs) | `SegmentWriter`, `NativeReader`, `LossReader`, `FileInfo`, `Validation` | `.logger/meta`·`.logger/loss`·`standard/2` 최상위 등록 배치 및 복구 (`RV-158`) |
| 검토 | [loss.rs](../../../crates/dot-logger/src/loss.rs) | `LossDelta` | 회전 파일 Baseline 시작점 및 다중 part/tail 꼬리 판정 |
| 검토 | [write.rs](../../../vendor/mcap-0.26.0/src/write.rs) | `WriteOptions` | `chunk_registrations(false)` 옵트인 최상위 Schema/Channel 배치 |

## 4. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-156 | P2 | `dot-logger-viewer/src/bridge.rs:92-133, 181-195, 205-214, 238-246`, `output.rs:32-107`, `websocket.rs:185-250`, ADR-014 §2·§3 | `Lease::drop` 비동기 종료 지연 중 재구독 시 기존 `Feed` 미재사용 및 `feeds` 한도/`LiveBudget` 경합; 스키마 불일치 은퇴 시 오해 소지 `Notice::Unknown` 추가 발행; 다중 채널 구독 시 `Notices` 단일 슬롯 덮어쓰기 | 살아 있는 비은퇴 `Feed` 재사용(또는 즉시 취소/대기 정리), 은퇴 시 `Unknown` 미발행, 채널별 상태 추적 분리 | 미정 | Open |
| RV-157 | P2 | `dot-logger-viewer/src/replay.rs:283-338, 410-424`, `main.rs:78-80, 108, 217`, `dot-logger-client/src/recording.rs:53-70`, ADR-014 §2·§S3, API §24 | `play` 실패 후 닫히거나 미바인딩/만료된 `RemoteRecording` 영구 재사용으로 후속 `seek` 복구 불가; 단일 채널 스키마 불일치 시 채널 은퇴 대신 전체 재생 중단; `--replay-blob-bytes` > 256 KiB 무음 절삭 | 연결 종료/미바인딩/만료 실패 후 `recording = None` 재연결, 스키마 불일치 시 해당 채널만 은퇴·`unadvertise`, `serve_replay` `message_bytes` 상향 및 `1..=16*MIB-1024` 상한 정렬 | 미정 | Open |
| RV-158 | P2 | `dot-logger/src/storage/recovery.rs:491-497, 641-797`, `storage/read.rs:213-236, 309-449`, ADR-013 §2·§5, API §23.1 | 최상위 등록 배치 전환 후 등록 미완료 파일(`op==12`만 있음 등)이 `EndOfInput`/`DataEnd` 정상 종료로 통과; `read::registrations`의 제어 채널(`.logger/meta`, `.logger/loss`)·채널 메타 검증 공백; `Validation` 중복 등록 미거절 | `Validation` 등록 완결성 검사 추가 및 첫 청크/종료/EOF 검증, `read::registrations` 제어 채널·메타데이터 정렬, `Validation::schema`/`channel` 중복 등록 거절 | 미정 | Open |
| RV-159 | P3 | `dot-logger/src/logger/mod.rs:38`, `logger/host.rs:538-552` | pinned nightly doc 체크 시 `[`Writer`](crate::logger::Writer)` `redundant_explicit_links` 경고 1건; `worker_failed` 감지 전 `remote.serve` 실행 순서 | `[`Writer`]`로 링크 축약, `remote.serve` 앞에서도 `worker_failed` 확인해 즉시 `Closing` 반영 | 미정 | Open |

---

### RV-156 — 실시간 `Bridge::subscribe`/`Lease::drop` 동기 재구독 레이스, 은퇴 시 `Unknown` 추가 발행 및 다중 채널 `Notices` 유실 (P2)

#### [리뷰어]

- **위치:**
  - [bridge.rs:92-133, 179-214, 238-253, 260-287](../../../crates/dot-logger-viewer/src/bridge.rs#L92-L133) (`Bridge::subscribe`, `Feed::offer`, `Lease::drop`, `Upstream::run`, `Upstream::forward`)
  - [output.rs:32-107](../../../crates/dot-logger-viewer/src/output.rs#L32-L107) (`Notices`, `Notice::message`)
  - [websocket.rs:178-251](../../../crates/dot-logger-viewer/src/websocket.rs#L178-L251) (`read`, `handle`)
  - [main.rs:269](../../../crates/dot-logger-viewer/src/main.rs#L269) (`#[tokio::main(flavor = "current_thread")]`)
  - 계약: ADR-014 §2·§3 ("마지막 수요가 사라지면 구독 수명도 끝난다. 상류 Gap, viewer 큐에서 확인한 손실, 종료·재연결 구간의 Unknown을 구분한다", "관측된 매핑 불일치는 미지원으로 보고하고 기존 광고를 회수한다").
- **원인 및 영향:**
  1. **마지막 구독 해제 직후 동기/연속 재구독 시 상류 자원 경합 및 거절 (`bridge.rs:92-133, 205-214`):**
     `dot-logger-viewer`는 `current_thread` 런타임(`main.rs:269`)에서 실행되며, `websocket::read`는 WS 메시지 루프 안에서 `Unsubscribe` 시 `leases.remove`(`Lease::drop`), `Subscribe` 시 `Bridge::subscribe`를 동기 클로저로 호출한다. 마지막 리스너가 떨어질 때 `Lease::drop`은 `state.listeners`를 비우고 `feed.stop.notify_one()`만 호출할 뿐, `registry.feeds`에서 해당 `Feed`를 제거하거나 워커 태스크의 `_permit`(`self.capacity`) 및 `LiveSubscription`(`self.budget` 예약)을 즉시 회수하지 않는다.
     - 따라서 클라이언트가 `Unsubscribe` 후 곧바로(또는 동일 WS 텍스트 프레임/TCP 읽기 배치로 연속 수신된 메시지, 혹은 마지막 1개 남은 `feeds` 슬롯을 다른 채널로 교체하는 구독에서) `Subscribe`를 호출하면 워커 태스크가 아직 스케줄링되지 않아 `_permit`과 `LiveBudget`이 이전 워커에 묶여 있다.
     - 그 결과 `Bridge::subscribe`(`109-115행`)는 `state.listeners.is_empty()`를 이유로 기존 `Feed`를 재사용하지 않고 새 `self.capacity.try_acquire_owned()`를 시도해 `options.feeds` 한도에 걸려 `"upstream channel limit reached"`로 거절되거나, 새 워커가 이전 워커의 `LiveSubscription`이 Drop되기 전에 `LiveBudget`(`options.feeds * options.receiver_records * (options.record_bytes + 1024)`로 정확히 `feeds`분만 예약됨)에서 수신 버퍼를 확보하지 못해 `"upstream unavailable or subscription rejected"`를 낼 수 있다.
  2. **스키마 불일치 은퇴 시 오해 소지 `Notice::Unknown` 추가 발행 (`bridge.rs:181-188, 240-246, 275`):**
     `Feed::offer`(`181-187행`)는 `!self.channel.matches(record)`일 때 `self.channel.retire()`를 호출하고 `Notice::Unsupported`를 알린 뒤 `false`를 반환한다. 이어 `Upstream::forward`(`268-277행`)는 `handoff`의 `Err(ClientError::ConsumerRejected)`를 `"no viewer accepted a record"`로 돌려주며, `Upstream::run`(`240-245행`)은 `if feed.channel.retired() { return; }` 검사 **직전**에 `feed.notify(Notice::Unknown { channel, reason: "no viewer accepted a record" })`를 먼저 발행한다. 이로 인해 스키마 불일치가 원인인데도 소비자에게 `"Unknown: channel ..., no viewer accepted a record; missing range unavailable"` 경고가 추가로 전송된다.
  3. **다중 채널 구독 시 `Notices`의 상태 덮어쓰기 (`output.rs:32-63`):**
     `Notices`는 채널 ID 구분 없이 `Notice` 종류별 크기 5의 배열(`[Option<(u64, Notice)>; 5]`)만 보관하며, `Connected`/`Unknown`은 메시지 포맷(`68-80행`)에 누적 건수(`total`)조차 출력하지 않는다. 한 WebSocket 클라이언트가 2개 이상 채널을 구독한 상태에서 100ms(`STATUS_INTERVAL`) 틱 사이에 두 채널에서 각각 `Connected`, `Unknown`, `UpstreamGap`, `QueueGap`, `Unsupported`가 발생하면 마지막 채널의 `Notice`가 이전 채널의 `Notice`를 덮어써 앞선 채널의 상태 알림(예: 채널 A 재연결 또는 채널 A 스키마 불일치 사유)이 클라이언트에 전달되지 않는다.
- **처리 제안:**
  1. `Upstream::run`이 살아 있는(`!channel.retired()`, 워커 미종료) `Feed`는 `state.listeners.is_empty()`이더라도 동일 채널 재구독 시 기존 `Feed`에 다시 리스너를 추가해 재사용하도록 하고(워커가 `stop.notified()`에서 깨어날 때 `state.listeners.is_empty()`를 재확인해 비어 있을 때만 종료), 반대로 다른 채널을 위해 정리할 때는 `Lease::drop` 또는 `Bridge::subscribe`에서 상류 `Cancellation`을 즉시 취소하거나 종료된 워커의 자원 반납을 보장하도록 수명을 정렬한다.
  2. `Upstream::run`에서 `if !feed.channel.retired()`일 때만 `Notice::Unknown`을 발행하도록 분기한다.
  3. `Notices`가 `(Notice 종류, channel)` 단위로 최신 상태를 추적하거나 제한된 채널별 링/맵으로 100ms 틱 내 다중 채널 상태를 덮어쓰지 않도록 개선한다.

#### [처리담당]

- **판정 결과:** 미정 (`Open`)

---

### RV-157 — 원격 기록 재생(`replay.rs`)의 실패 후 죽은 연결 재사용, 단일 채널 불일치 시 전체 재생 중단 및 `--replay-blob-bytes` 무음 절삭 (P2)

#### [리뷰어]

- **위치:**
  - [replay.rs:283-338, 410-424](../../../crates/dot-logger-viewer/src/replay.rs#L283-L338) (`Driver::run`, `Driver::record`)
  - [main.rs:78-80, 108, 217, 239-242](../../../crates/dot-logger-viewer/src/main.rs#L78-L115) (`Options`, `run`)
  - [recording.rs:53-70](../../../crates/dot-logger-client/src/recording.rs#L53-L70) (`RemoteRecording::connect`)
  - [transport.rs:212-214, 343-350](../../../crates/dot-logger-client/src/transport.rs#L343-L350) (`RemoteClient::request`)
  - 계약: ADR-014 §2 ("관측된 매핑 불일치는 미지원으로 보고하고 기존 광고를 회수한다"), §S3 개정 ("호스트가 스냅샷을 만료하면 화면에 실패를 표시한다"), API §24.
- **원인 및 영향:**
  1. **`Driver::run`의 실패 후 죽은 `RemoteRecording` 영구 재사용 (`replay.rs:296-336`):**
     `Driver::run`은 처음 한 번 `RemoteRecording::connect`에 성공해 `recording = Some(connection)`이 된 이후에는, `self.play(connection, position, &cancel).await`가 `Err(error)`를 반환하더라도 `recording`을 `None`으로 리셋하지 않고 `self.failed(...)`에서 다음 `seek`를 기다린 뒤 **같은 `connection`으로 재시도**한다.
     - 그러나 `RemoteClient`는 어떤 gRPC 오류/타임아웃/연결 단절이 발생하면 `ConnectionState::close()`로 닫히며 이후 모든 `request()`가 `Err(ClientError::Disconnected)`를 반환한다(`transport.rs:348`).
     - 또한 첫 `page`가 실패하여 `snapshot == 0` 상태인 채 `recording`에 남아 있거나, 호스트 TTL 만료로 `RemoteError::Gone`이 반환된 경우에도 새 연결 없이는 새 스냅샷을 열 수 없으므로 이후 터미널에서 `seek`를 입력해도 영구히 `Replay failed`에 갇힌다.
  2. **단일 채널 스키마 불일치 시 전체 재생 중단 (`replay.rs:418-423`):**
     `Driver::record`는 `!channel.matches(metadata)`일 때 `Err(format!("local schema mapping does not match channel {}", metadata.channel_id))`를 반환해 `Driver::play` 전체를 실패로 종료시킨다. ADR-014 §2의 매핑 불일치 계약과 달리 해당 채널을 `retire()`하고 `unadvertise` + `Unsupported` 상태를 내보내는 대신 같은 소켓에서 정상 구독 중인 다른 채널의 재생까지 모두 중단한다.
  3. **`--replay-blob-bytes` 설정 상한과 `RemoteConfig.message_bytes` 불일치 (`main.rs:108, 217`, `recording.rs:58`):**
     `Options::validate`(`main.rs:108`)는 `--replay-blob-bytes`를 `1..=16 * MIB`까지 허용하지만, `run()`(`main.rs:217`)이 `RemoteConfig::new`의 기본 `message_bytes`(`MAX_CHUNK_BYTES + 1024 = 257 KiB`)를 그대로 `serve_replay`에 전달하므로 `RemoteRecording::connect`(`recording.rs:58`)에서 `blob_chunk_bytes.min(config.message_bytes.saturating_sub(1024))`에 의해 항상 `256 KiB` 이하로 무음 절삭된다. 또한 `RemoteClient::connect`의 `message_bytes` 상한이 `16 * 1024 * 1024`(`transport.rs:237`)이고 `codec::request`(`codec.rs:117`)가 `length as usize > max.saturating_sub(1024)`를 거절하므로, `--replay-blob-bytes 16777216`은 gRPC 프레임 오버헤드(`1024`B)와 양립하지 않는다.
- **처리 제안:**
  1. `Driver::run`에서 연결 단절(`Disconnected`/`Transport`/`TimedOut`), 스냅샷 미바인딩 실패, 또는 스냅샷 만료(`Gone`) 등으로 `play`가 `Err`로 끝난 경우(또는 `Err` 발생 시 일괄로) `recording = None`으로 초기화하여 다음 `seek` 시 새 `RemoteRecording::connect`를 수행하도록 한다.
  2. `Driver::record`에서 `!channel.matches(metadata)`인 경우 `channel.retire()`를 호출하고 WS 클라이언트에 `Unsupported` 상태 및 `unadvertise`를 전송한 뒤 해당 레코드를 건너뛰어 다른 구독 채널의 재생을 유지한다.
  3. `serve_replay`(또는 `run`)에서 `upstream.message_bytes = upstream.message_bytes.max(options.replay_blob_bytes.saturating_add(1024))`로 맞추고, `Options::validate`의 `replay_blob_bytes` 상한을 `1..=16 * MIB - 1024`로 정렬한다(또는 `Options::validate`가 `record_bytes`와 마찬가지로 `message_bytes` 상한을 반영).

#### [처리담당]

- **판정 결과:** 미정 (`Open`)

---

### RV-158 — 최상위 MCAP 등록 배치(`a3296dc`) 전환 이후 `recovery.rs`·`read.rs`의 등록 완결성·제어 채널·중복 등록 검증 공백 (P2)

#### [리뷰어]

- **위치:**
  - [recovery.rs:387-395, 491-497, 641-797](../../../crates/dot-logger/src/storage/recovery.rs#L491-L497) (`recover_inner`, `Validation::schema`, `Validation::channel`)
  - [read.rs:213-236, 309-449](../../../crates/dot-logger/src/storage/read.rs#L309-L449) (`NativeReader::next_item`, `registrations`)
  - 계약: ADR-013 §2·§5 ("임의의 표준 MCAP 파일을 제품 파일로 오인하지 않는다. file Metadata·각 Channel의 session/revision/등록도 서로 일치해야 한다"), API §23.1.
- **원인 및 영향:**
  1. **등록 미완료 파일이 `recover_inner`에서 정상 종료(`EndOfInput`/`DataEnd`)로 통과 (`recovery.rs:491-497`):**
     `a3296dc`에서 `SegmentWriter`가 `Schema`/`Channel` 등록을 청크 외부의 최상위 data section에 쓰도록 바뀌면서, 파일에 `dot_logger.file`(`op == 12`)만 있고 `Schema`/`Channel` 일부 또는 `.logger/meta`/`.logger/loss` 채널 등록이 누락·절단된 상태로 `EndOfInput` 또는 `DataEnd`(`op == 15`)에 도달할 수 있다. 그러나 `recover_inner`(`491-497행`)는 `state.file_sequence() != Some(options.source_sequence)`만 확인하고 필수 등록(`meta_seen`, 모든 `seen`, `file.loss_rows > 0`일 때 `loss_seen`) 완결 여부를 확인하지 않는다. 그 결과 등록이 잘린 불완전 입력도 `RecoveryStop::EndOfInput`/`DataEnd`(CLI `recover` 성공 종료)로 수락된다.
  2. **`read::registrations`의 제어 채널 및 사용자 메타데이터 검증 공백 (`read.rs:384-448`):**
     `read::registrations`는 `c.topic == META_TOPIC || c.topic == super::codec::LOSS_TOPIC`(`388-390행`)에서 즉시 `continue`한다. 이로 인해 `NativeReader::new` / `LossReader::new`는 다음 손상·위조를 초기 검증에서 놓친다:
     - `.logger/meta` 채널(ID가 `file.channels + 1`인지, `schema_id`가 `META_SCHEMA`·`META_DESCRIPTOR`인지, `message_encoding == "protobuf"`인지, `metadata`가 비어 있는지) 및 `.logger/loss` 채널(`file.loss_rows > 0`일 때 필수 존재 여부·ID `file.channels + 2`·`loss::SCHEMA`/`DESCRIPTOR`, 반대로 `file.loss_rows == 0`일 때 `.logger/loss` 등록 금지) 여부. (특히 메시지 청크가 0개이거나 손상 전까지 청크를 만나지 않으면 제어 채널이 전혀 없어도 `NativeReader::new`와 순회가 `Ok`로 끝남.)
     - 사용자 `Record::Channel`의 `c.metadata`에 기록된 `"session_id"`, `"revision"`, `"payload_encoding"`의 파일/채널 일치 여부 및 알 수 없는 임의 키 존재 여부(현재 `get("product_channel")`, `get("payload_schema_id")`, `get("payload_schema_version")`만 파싱).
     - `Validation::channel`과 달리 `c.schema_id == 0`이어도 `payload_schema_name`이 붙은 경우나, `NativeReader::next_item`이 `DataEnd`/`EOF`에 도달했을 때 등록 완결성(`meta_seen`, `loss_seen`) 확인.
  3. **`Validation::schema`/`channel`의 중복 등록 및 청크 이후 재등록 미차단 (`recovery.rs:641-797`):**
     `Validation::schema`(`658-705행`)는 `self.loss_schema.is_some_and(|old| old != id)` 등으로 다른 ID일 때만 거절하므로 동일 ID로 `META_SCHEMA`/`loss::SCHEMA`/동일 payload 스키마가 두 번 등장하는 중복 `Schema` 레코드를 허용하며, `Validation::channel`(`717-796행`)도 `self.meta_seen`, `self.loss_seen`, `self.seen[i]`가 이미 `true`인 상태에서 같은 채널 ID가 다시 등장해도 거절하지 않는다. 또한 `recover_inner`와 `NativeReader::next_item`에서 첫 메시지 청크(`op == 6`)가 지나간 뒤 최상위 `op == 3 | 4`가 다시 나와도 차단하지 않는다.
- **처리 제안:**
  1. `Validation`에 필수 등록 완료 판정 메서드(예: `registrations_complete()`)를 두고, `recover_inner`의 `EndOfInput`/`DataEnd` 마감 검사(`491행`) 및 첫 `op == 6` 진입 전과 `NativeReader::next_item`의 끝(`213`, `228행`)에서 미완결 등록을 `InvalidRecord`/`CorruptData`로 거절한다.
  2. `read::registrations`에서 `.logger/meta`·`.logger/loss` 채널의 ID/스키마/인코딩/빈 메타와 `file.loss_rows` 일치, 사용자 채널 `c.metadata` 전체(`session_id`, `revision`, `payload_encoding`, 키 집합 및 `schema_id == 0`일 때 `payload_schema_name` 금지)를 `Validation::channel`과 동일한 계약으로 검증한다.
  3. `Validation::schema`·`Validation::channel`에서 이미 등록된 슬롯(`loss_schema`, `meta_schema`, `payload_schemas[i]`, `meta_seen`, `loss_seen`, `seen[i]`)의 재등록과 첫 메시지 청크 이후의 늦은 최상위 등록을 즉시 거절한다.

#### [처리담당]

- **판정 결과:** 미정 (`Open`)

---

### RV-159 — `logger/mod.rs` rustdoc 중복 링크 경고 및 `host.rs`의 `worker_failed` 확인 순서 정렬 (P3)

#### [리뷰어]

- **위치:**
  - [logger/mod.rs:38](../../../crates/dot-logger/src/logger/mod.rs#L38)
  - [logger/host.rs:538-552](../../../crates/dot-logger/src/logger/host.rs#L538-L552)
- **내용 및 처리 제안:**
  1. **`rustdoc::redundant_explicit_links` 경고 제거 (`logger/mod.rs:38`):**
     `scripts/check-integration-surface.py` 실행 시 내부 `cargo +nightly-2026-10-26 doc -p dot-logger` 단계에서 `logger/mod.rs:38`의 ``[`Writer`](crate::logger::Writer)``에 대해 `warning: redundant explicit link target` 경고가 출력된다. ``[`Writer`]``로 축약해 `nightly` `rustdoc` 경고까지 0건으로 맞춘다.
  2. **`worker_failed` 종료 감지 순서 정렬 (`logger/host.rs:538-552`):**
     `host::run` 루프에서 `remote.serve(...)`(`538행`)가 `if config.flags.worker_failed.load(Ordering::Acquire) { core.control().close(); }` (`549행`)보다 먼저 실행된다. IO 워커가 실패한 직후 첫 턴에 원격 `ApplyConfig`가 들어와 있고 아직 기존 revision을 적용하지 않은 writer가 없다면, 원격 `ApplyConfig`가 한 턴 먼저 새 revision을 게시한 직후 `close()`될 수 있다(로컬 `set_channel`은 `597행`에서 `worker_failed`를 먼저 확인해 즉시 `ConfigError::Closing` 반환). `remote.serve(...)` 호출 전에도 `worker_failed` 확인 및 `core.control().close()`를 수행해 로컬·원격 경로의 종료 판정을 일치시킨다.

#### [처리담당]

- **판정 결과:** 미정 (`Open`)

---

## 5. 검증 및 인계

### 검증 실행 결과

| 검증 | 결과 |
|---|---|
| `cargo +1.96.0 fmt --all --check` | Pass |
| `cargo +1.96.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass |
| `cargo +1.96.0 test --workspace --all-features --locked` | Pass — 692 passed, 0 failed, 5 ignored(외부 DB 시험), RT 할당 20 passed·viewer 6 passed·doctests 24 passed 포함 |
| `RUSTDOCFLAGS='-D warnings' cargo +1.96.0 doc --workspace --all-features --no-deps --locked` | Pass |
| `RUSTDOCFLAGS='-D warnings' cargo +1.85.0 doc --workspace --all-features --no-deps --locked` | Pass |
| `python3 scripts/check-integration-surface.py` | Pass — facade 이름 있는 항목 35/60, L1~L3 레시피·실행·README 일치 (단, 내부 `cargo +nightly-2026-10-26 doc`에서 `redundant_explicit_links` 경고 1건 관측 → `RV-159`) |
| 제품 코드·저장소 테스트 변경 | 없음 (Strict Read-Only 준수) |

### 체크리스트 대조 요약

1. **RT(실시간) 경로 제약:** `Writer::refresh`·`Writer::try_log_at`·`Producer::apply_pending_config`·`CaptureCore::loss_before_submit`·`GeneralProducer::record_until` 경로에 동적 메모리 할당/해제, 블로킹 lock, I/O, 문자열 포맷팅, CAS 무한 루프가 없음을 코드와 `dot-logger-rt-allocation` 20개 시험으로 확인했다. `Held::acknowledge`/`SinkState::acknowledge`는 일반 관리 경로에서 `try_lock`으로 접근해 애플리케이션/포매터 lock을 대기하지 않으며 `WouldBlock`은 다음 관리 턴에 재시도한다.
2. **Rust 언어 및 동시성/메모리 안전성:** `ae3b6b3..3f8a46f` 범위의 제품 코드에 신규 `unsafe` 블록은 추가되지 않았다. `ConfigUpdates`의 bank 전환(`posted` `Release` / producer `Acquire`)과 `loss_before_submit`/`record_until`의 `ActiveCall` + `closing`(`SeqCst`) 경계는 일관되나, `dot-logger-viewer`의 `Lease::drop` 동기 재구독 자원 경합(`RV-156`) 및 `replay.rs`의 실패 후 연결 수명 처리(`RV-157`) 보완이 필요하다.
3. **아키텍처·API 경계 및 유한 예산:** `dot-logger-viewer`는 일반 실행 경로에서 `dot-logger-client`(`LiveBudget`, `LiveSubscription`, `RemoteRecording`, `RemoteConfig`)만 소비하며 `dot-logger` 코어에 직접 의존하지 않는다(`dev-dependencies` 통합 시험·예제 한정). `mcap` `WriteOptions::chunk_registrations(false)` 변경은 최상위 등록 배치에만 영향을 주며 chunk/data CRC·인덱스 불변식을 유지하지만, `recovery.rs`·`read.rs`의 등록 완결성/제어 채널/중복 등록 검증 공백은 `RV-158`로 보완해야 한다.

### 다음 개발 세션 인계

- 다음 개발 세션은 **TASK-031 잔여 검증(Foxglove Desktop · T7 정량 실측) 착수 전에** 본 문서(`REV-066`)의 `RV-156~158`(P2) 및 `RV-159`(P3)에 대해 실패 재현 테스트(Red) 작성 → 최소 코드 수정(Green) → 회귀 검증을 수행하고 `[처리담당]` 영역을 갱신한 뒤 마감한다.

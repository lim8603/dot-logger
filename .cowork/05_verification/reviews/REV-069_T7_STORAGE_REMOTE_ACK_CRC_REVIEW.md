# REV-069 — TASK-031 T7 저장·손실 0 행 생략·원격 묶음/파이프라인 ACK·CRC-32 통합 독립 코드리뷰

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-069 / 2026-11-21 |
| 리뷰어 | Antigravity AI — 리뷰 전용 세션 |
| 대상 | REV-066 조치(`bd02d49`) 이후 누적 커밋 `bd02d49..078ca6c` (7개 코드 커밋: `8dbbe01`, `0be4bfe`, `7edae2a`, `af68578`, `4ac9bf1`, `0637601`, `078ca6c`) |
| 범위 | 손실 checkpoint 인덱스 예약 정합(`8dbbe01`), 손실 checkpoint part 청크 묶음 및 주기 관측 2회 예산(`0be4bfe`), LAN 수신 ACK 측정 예제(`7edae2a`), ADR-013 D2 손실 checkpoint 0 행 생략(`af68578`), ADR-015 원격 수신 묶음·파이프라인 ACK(`4ac9bf1`), `crc32fast` 기반 하드웨어 CRC-32 교체(`0637601`), 파이프라인 ACK 기한 starvation 방지(`078ca6c`) |
| 최초 판정 | **Conditional** — P1 0 · P2 2 · P3 1 (`RV-163~165`) |
| 조치 후 판정 | 미조치 (`Open`) |
| 상태 | **Open** — 후속 개발 세션에서 `RV-163~165` 조치 후 TASK-032 S1 진행 |

정적 분석(`cargo +1.96.0 fmt --all --check`, `cargo +1.96.0 clippy --workspace --all-targets --all-features --locked -- -D warnings`), 워크스페이스 전체 테스트(`cargo +1.96.0 test --workspace --all-features --locked`: 705 passed, 0 failed, 5 ignored — RT 할당 20 passed, viewer 8 passed, doctests 24 passed 포함), strict `rustdoc`(`1.96.0`, `1.85.0` 모두 Pass), `python3 scripts/check-integration-surface.py`(facade 이름 있는 항목 35/60, L1~L3 레시피·실행·README 일치)는 모두 통과했다.

`bd02d49..078ca6c` 구간의 제품 코드를 ADR-013(2026-11-21 0 행 생략 개정)·ADR-015·API 계약 및 중점 점검 항목 ①~⑥과 대조해 정밀 검토한 결과 다음 3건(P2 2건, P3 1건)을 확인했다:
1. **P2 (`RV-163`):** `dot-logger-client`의 `RemoteSubscription` 파이프라인 ACK(`transport.rs`)에서 이전 ACK 실패나 스트림 종료로 폐기된 미전송 ACK 상태를 보존하지 않아 후속 `flush` 호출이 `Ok(())`를 반환하는 문제(특히 `acks`에 미전송 항목이 남은 상태에서 `next`가 EOF/`Ok(None)` 또는 스트림 오류로 닫힌 뒤 `flush`를 호출하면 유실된 ACK가 있는데도 `Ok(())`가 반환됨), `settle_one`이 `.await` 전에 `in_flight.pop_front()`를 수행해 `next` 로컬 대기 타임아웃 시 진행 중 `JoinHandle`과 그 실패 결과가 영구 유실되는 문제, 그리고 서버 `AckRpc`(`server.rs`)가 구독별 협상 창(`sub.records`)이 아닌 서버 전역 `limits.subscription_records`만 사전 검사하는 문제를 확인했다.
2. **P2 (`RV-164`):** 손실 checkpoint v2(`loss.rs`)에서 비영(nonzero) 행 수가 `ROWS_PER_REPORT`(64)의 양의 배수이고 뒤에 0 행이 남아 있을 때 `Bank::sparse`가 64번째 행 직후에 끊어 빈 후행 part를 1개 더 생성하는 문제(ADR-013의 `max(1, ceil(0이 아닌 행/64))` 상한과 불일치), 그리고 `LossReport::decode` 및 `pub fn summarize`가 비최종 v2 part(`end < rows`)의 `counts.len() == ROWS_PER_REPORT` 불변식과 다중 part 연속 구간의 실제 폭 검사(`(r.counts.len() as u64) <= g.end - covered`)를 수행하지 않는 문제를 확인했다.
3. **P3 (`RV-165`):** CLI `tail`(`commands.rs`)이 `limit` 도달 전 조기 종료·타임아웃·후속 `flush` 실패 시 이미 서버가 확인 완료한 앞선 묶음의 `acknowledged` 건수를 `0`으로 보고하는 점, `SegmentWriter::projected_index_bytes`(`writer.rs`)의 `split` 조건에 `self.pending_loss_parts > 0`이 빠져 있는 점, 그리고 `lan_probe.rs` 예제의 서버 `commands: 1` 설정이 HTTP/2 `SETTINGS_MAX_CONCURRENT_STREAMS`를 `2`(구독 1 + ACK 1)로 제한해 클라이언트 4개 동시 ACK 파이프라인을 직렬화하는 점 및 `grpc_receiver`가 Gap 발생 시에도 종료 코드 0(`Ok(())`)을 반환하는 점을 개선 권고한다.

리뷰 전용 세션 원칙(Strict Read-Only)에 따라 제품 코드·저장소 테스트는 수정하지 않았다.

## 2. 파일 변경표

### 이번 리뷰 세션 산출물

| 구분 | 파일 | 변경 내용 |
|---|---|---|
| 생성 | [REV-069](REV-069_T7_STORAGE_REMOTE_ACK_CRC_REVIEW.md) | `bd02d49..078ca6c` 독립 코드리뷰 결과(`RV-163~165`) 및 중점 ①~⑥ 대조 기록 (`Open`) |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | 미처리 리뷰 목록·코드리뷰 현황에 REV-069 (`Open`, P2 2·P3 1) 등록 및 다음 시작점 동기화 |
| 수정 | [my_state.md](../../members/lim8603/workspace/my_state.md) | 개인 상태 인덱스 리뷰·세션·다음 시작점 동기화 |
| 생성 | [session_2026-11-21_002.md](../../members/lim8603/workspace/session_logs/session_2026-11-21_002.md) | 코드리뷰 세션 로그 (`2026-11-21 #002`) |

### 검토 대상 코드 파일 (`bd02d49..078ca6c`)

- **core (`crates/dot-logger/`):** [Cargo.toml](../../../crates/dot-logger/Cargo.toml), [adapters/remote/server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs), [adapters/remote/wire.rs](../../../crates/dot-logger/src/adapters/remote/wire.rs), [adapters/remote/tests.rs](../../../crates/dot-logger/src/adapters/remote/tests.rs), [logger/budget.rs](../../../crates/dot-logger/src/logger/budget.rs), [loss.rs](../../../crates/dot-logger/src/loss.rs), [storage/loss_report.proto](../../../crates/dot-logger/src/storage/loss_report.proto), [storage/loss_report.bin](../../../crates/dot-logger/src/storage/loss_report.bin), [storage/loss_report.v1.bin](../../../crates/dot-logger/src/storage/loss_report.v1.bin), [storage/mod.rs](../../../crates/dot-logger/src/storage/mod.rs), [storage/recovery.rs](../../../crates/dot-logger/src/storage/recovery.rs), [storage/retention.rs](../../../crates/dot-logger/src/storage/retention.rs), [storage/writer.rs](../../../crates/dot-logger/src/storage/writer.rs), [tests/tc_013_facade.rs](../../../crates/dot-logger/tests/tc_013_facade.rs), [tests/tc_017_loss.rs](../../../crates/dot-logger/tests/tc_017_loss.rs)
- **client (`crates/dot-logger-client/`):** [Cargo.toml](../../../crates/dot-logger-client/Cargo.toml), [README.md](../../../crates/dot-logger-client/README.md), [src/codec.rs](../../../crates/dot-logger-client/src/codec.rs), [src/live.rs](../../../crates/dot-logger-client/src/live.rs), [src/transport.rs](../../../crates/dot-logger-client/src/transport.rs), [src/wire.rs](../../../crates/dot-logger-client/src/wire.rs), [examples/lan_probe.rs](../../../crates/dot-logger-client/examples/lan_probe.rs), [examples/support/dot_probe/bridge.rs](../../../crates/dot-logger-client/examples/support/dot_probe/bridge.rs), [examples/support/pipeline/remote.rs](../../../crates/dot-logger-client/examples/support/pipeline/remote.rs), [examples/support/tail/mod.rs](../../../crates/dot-logger-client/examples/support/tail/mod.rs), [tests/transport.rs](../../../crates/dot-logger-client/tests/transport.rs)
- **cli / viewer / workspace:** [Cargo.toml](../../../Cargo.toml), [Cargo.lock](../../../Cargo.lock), [crates/dot-logger-cli/src/commands.rs](../../../crates/dot-logger-cli/src/commands.rs), [crates/dot-logger-cli/tests/cli.rs](../../../crates/dot-logger-cli/tests/cli.rs), [crates/dot-logger-viewer/src/tests.rs](../../../crates/dot-logger-viewer/src/tests.rs)

## 3. 클래스(타입) 변경표

**이번 리뷰 세션의 제품 코드 변경은 없음.** 검토 대상 핵심 타입(`struct` · `enum`)은 아래와 같다.

| 구분 | 소속 파일 | 타입 이름 | 역할 및 검토 요약 |
|---|---|---|---|
| 검토 | [transport.rs](../../../crates/dot-logger-client/src/transport.rs), [live.rs](../../../crates/dot-logger-client/src/live.rs) | `RemoteSubscription`, `LiveSubscription`, `RemoteEvent` | ADR-015 묶음(64)·지연(5 ms)·파이프라인(동시 4 RPC) ACK 및 `flush` (`RV-163`, `RV-165`) |
| 검토 | [adapters/remote/wire.rs](../../../crates/dot-logger/src/adapters/remote/wire.rs), [client/src/wire.rs](../../../crates/dot-logger-client/src/wire.rs), [server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs) | `AckRequest`, `AckItem`, `AckRpc`, `Incoming` | 단건/묶음 ACK wire 디코딩, 서버 `TCP_NODELAY`, 묶음 순서 적용 및 상한 검사 (`RV-163`) |
| 검토 | [loss.rs](../../../crates/dot-logger/src/loss.rs) | `Bank`, `LossReport`, `LossRange`, `LossCount`, `LossDelta`, `WireReport` | ADR-013 D2 개정 v2 0 행 생략(`sparse`), 구간 완결성 검증, 생략 키 0 합성 비교 (`RV-164`) |
| 검토 | [storage/writer.rs](../../../crates/dot-logger/src/storage/writer.rs), [retention.rs](../../../crates/dot-logger/src/storage/retention.rs), [logger/budget.rs](../../../crates/dot-logger/src/logger/budget.rs) | `SegmentWriter`, `WriterConfig`, `SegmentLog` | 손실 part 청크 묶음(`loss_chunks`), 주기 관측 포함 인덱스·청크 예산 정합 (`RV-165`) |
| 검토 | [storage/recovery.rs](../../../crates/dot-logger/src/storage/recovery.rs) | `Validation` | v1/v2 손실 descriptor 호환, v2 `range.rows` 대조 및 `crc32fast` 기반 `crc_update` |
| 검토 | [examples/lan_probe.rs](../../../crates/dot-logger-client/examples/lan_probe.rs) | `OfferStats` | LAN TCP 대 gRPC 소형/대용량 수신 검증·ACK 지연 측정 예제 (`RV-165`) |

## 4. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-163 | P2 | `dot-logger-client/src/transport.rs:514-624, 635-693, 727-769`, `dot-logger/src/adapters/remote/server.rs:374-387`, `hub.rs:243-268`, ADR-015 §1·§2·§4·§5 | ACK 실패 또는 미전송 `acks`가 남은 채 스트림 종료(`close`) 후 `flush` 호출 시 `Ok(())` 반환; `settle_one`이 `.await` 전에 `pop_front()`하여 `next` 로컬 타임아웃 시 진행 중 ACK RPC 결과 유실; 서버 묶음 상한이 구독별 `sub.records` 대신 전역 `limits.subscription_records`만 검사 | `RemoteSubscription`에 미확인/실패 상태 보존 및 스트림 종료 시 미전송 `acks` 감지, `settle_one`에서 `front_mut().await` 완료 후 `pop_front()`, 서버에서 첫 항목 적용 전 `acks.len() <= sub.records` 검증 | 미판정 | Open |
| RV-164 | P2 | `dot-logger/src/loss.rs:393-433, 580-591, 777-858`, `storage/writer.rs:732-741`, ADR-013 D2 개정(2026-11-21) | 비영 행 수가 64의 양의 배수이고 뒤에 0 행이 남을 때 빈 후행 part가 추가 생성되어 `max(1, ceil(M/64))` 상한과 불일치; `decode`·`summarize`에서 비최종 v2 part의 64행 불변식 및 다중 part 구간 폭(`counts.len() <= end - covered`) 미검증 | `Bank::sparse`가 65번째 비영 행에서 끊어 후행 0 행을 같은 part에 흡수하도록 수정, `decode`와 `summarize`에 비최종 part 행 수(`ROWS_PER_REPORT`) 및 `counts.len() <= g.end - covered` 검증 추가 | 미판정 | Open |
| RV-165 | P3 | `dot-logger-cli/src/commands.rs:841-883`, `dot-logger/src/storage/writer.rs:615-617, 917-918`, `dot-logger-client/examples/lan_probe.rs:144-154, 333-344` | CLI `tail` 조기 종료/실패 시 이미 서버가 확인한 `acknowledged` 건수가 `0`으로 출력; `projected_index_bytes`의 `split`에 `pending_loss_parts > 0` 누락; `lan_probe`의 `commands: 1`이 동시 ACK 스트림을 1개로 직렬화하고 `grpc_receiver`가 Gap 발생 시에도 `Ok(())` 반환 | `RemoteSubscription`에 확인 완료 ACK 누적 건수 노출 및 CLI 반영, `projected_index_bytes` `split` 조건 정렬, `lan_probe` `commands: 4` 및 수신 Gap 비정상 종료 반영 | 미판정 | Open |

---

### RV-163 — `RemoteSubscription`의 `AcknowledgementUnknown` 상태 비보존, `settle_one` 타임아웃 시 `in_flight` 핸들 유실 및 서버 묶음 상한의 구독별 `sub.records` 미대조 (P2)

#### [리뷰어]

- **위치:**
  - [transport.rs:514-624, 635-693, 727-769](../../../crates/dot-logger-client/src/transport.rs#L514-L624) (`RemoteSubscription::close`, `unknown`, `reap`, `settle_one`, `send_acks`, `flush`, `next`, `next_inner`, `handoff`)
  - [server.rs:374-387](../../../crates/dot-logger/src/adapters/remote/server.rs#L374-L387) (`AckRpc::call`)
  - [hub.rs:243-268, 348, 504-506](../../../crates/dot-logger/src/adapters/remote/hub.rs#L243-L268) (`Subscription::ack`, `RemoteHub::subscribe`, `RemoteHub::ack`)
  - 계약: [ADR-015](../../03_design_artifacts/adrs/ADR-015_batched_pipelined_remote_ack.md) §1·§2·§4·§5 ("하나의 묶음은 구독의 수신 창(`subscription_records`)을 넘을 수 없다", "이미 보낸 묶음의 실패는 다음 `handoff`·`next`·`flush`에서 `AcknowledgementUnknown`으로 드러난다. 그때 구독을 닫는다", "`flush`는 버퍼를 보내고 진행 중인 ACK RPC를 모두 기다린다"), `transport.rs:607-609` ("`flush`: Ok means the server confirmed every record handed off so far").
- **원인 및 영향:**
  1. **ACK 실패·스트림 종료 후 미확인 상태가 보존되지 않아 후속 `flush`가 `Ok(())`를 반환하는 문제 (`transport.rs:528-554, 610-624, 654-657, 685-688`):**
     - `RemoteSubscription`에는 이미 발생한 ACK 실패나 미전송 ACK 폐기 여부를 보존하는 상태 필드가 없다. `reap()` 또는 `settle_one()`이 실패한 `JoinHandle`을 `self.in_flight`에서 꺼낸 뒤 `self.unknown()`을 호출하면 `self.close()`가 `self.acks.clear()`로 대기열을 비운다. 따라서 `handoff`나 `next`에서 `Err(ClientError::AcknowledgementUnknown)`이 발생한 뒤 호출자가 종료 경로에서 `stream.flush(&cancel).await`를 호출하면(또는 `flush`를 다시 호출하면), `self.acks`와 `self.in_flight`가 비어 있어 `flush`가 **`Ok(())`**를 반환한다.
     - 더 중요한 경로로, 소비자가 `handoff`로 레코드를 수락해 `self.acks`에 미전송 ACK가 대기 중인 상태(`in_flight`가 비어 있지 않고 `oldest < 5 ms`)에서 다음 `stream.next(&cancel).await`를 호출했을 때 서버 스트림이 `Ok(None)`(EOF)이나 오류로 종료되면(`655`, `686행`), `self.close()`가 호출되어 **아직 보내지 않은 `self.acks`를 조용히 `clear()`**하고 `next`는 `Ok(None)`(또는 스트림 오류)을 반환한다. 이어서 호출자가 스트림 루프 탈출 후 `stream.flush(&cancel).await`를 호출하면, 앞서 보낸 `in_flight` RPC만 완료되면 미전송 상태로 삭제된 `self.acks`가 있었음에도 `flush`가 **`Ok(())`**를 반환하여 마지막 인계 레코드들의 ACK 미확인이 `AcknowledgementUnknown`으로 전혀 드러나지 않는다.
  2. **`settle_one`이 `.await` 전에 `in_flight.pop_front()`를 호출하여 `next` 로컬 타임아웃 시 진행 중 ACK RPC 결과를 잃어버리는 문제 (`transport.rs:556-564, 575-577, 646-658, 665-669, 687`):**
     - `settle_one`은 `if let Some(rpc) = self.in_flight.pop_front()`로 핸들을 먼저 제거한 뒤 `rpc.await`를 기다린다.
     - `in_flight.len() == ACK_IN_FLIGHT`(4개)이고 `self.acks`의 `oldest >= 5 ms`인 상태에서 `next(&cancel)`이 `next_inner() -> send_acks() -> settle_one()`을 실행하는 도중 `next`의 바깥 `cancel.guard(self.timeout, ...)`(`646행`)가 타임아웃되면, `next()`는 로컬 대기 타임아웃 규칙(`651행`)에 따라 스트림을 닫지 않고 `Err(ClientError::Remote(RemoteError::TimedOut))`을 반환한다. 이때 이미 `pop_front()`된 첫 `JoinHandle`은 future 취소와 함께 Drop되어 `self.in_flight`에서 사라지므로, 해당 ACK RPC가 실패하거나 타임아웃되더라도 이후 `next`·`handoff`·`flush`가 그 실패를 영구히 놓치고 `Ok(())`를 반환할 수 있다.
  3. **서버 `AckRpc`의 묶음 상한 검사가 구독별 협상 창(`sub.records`) 대신 서버 전역 `limits.subscription_records`만 사용하는 문제 (`server.rs:377-382`, `hub.rs:243-268, 348`):**
     - 클라이언트는 구독 요청 시 `1..=limits.subscription_records` 범위의 `records`를 협상하며 서버는 이를 `Subscription.records`에 보관한다(`hub.rs:348`). 클라이언트도 묶음 상한을 `ACK_BATCH.min(limits.records)`로 제한한다(`transport.rs:482`).
     - 그러나 서버 `AckRpc`(`server.rs:377`)는 `let max = s.hub.shared.limits.subscription_records;`만 `AckRequest::native(max)`에 전달하고 `Subscription`의 실제 `sub.records`를 적용 전에 확인하지 않는다. 그 결과 서버 전역 `limits.subscription_records`(예: 64~1024)보다 작은 구독 창(예: `records = 2`)을 가진 구독에 3개짜리 묶음 요청이 오면, 적용 전에 거절되지 않고 앞선 유효 항목들이 먼저 ACK된 뒤 실패한다.
- **처리 제안:**
  1. `RemoteSubscription`에 sticky 실패 플래그(예: `failed: bool`)를 추가하여 `unknown()` 호출 시 또는 `close()` 시점에 `!self.acks.is_empty()`이면 `self.failed = true`로 기록하고, `reap()`(및 `flush`/`handoff`/`next`) 진입 시 `self.failed`이면 즉시 `Err(ClientError::AcknowledgementUnknown)`을 반환하도록 보강한다. 또한 `next_inner`에서 스트림이 `Ok(None)`이나 오류로 끝날 때 `self.reap()?`을 확인하고 미전송 `acks`가 남아 있었으면 `AcknowledgementUnknown`으로 전환한다.
  2. `settle_one`에서 `self.in_flight.front_mut()`를 제자리에서 `.await`한 뒤 완료된 시점에만 `self.in_flight.pop_front()`를 호출하여, 중간 취소/타임아웃 시 `JoinHandle`이 유실되지 않도록 한다.
  3. 서버 `RemoteHub`/`Subscription`에서 묶음 ACK를 처리할 때 첫 항목을 적용하기 전에 `acks.len() <= sub.records`를 먼저 검증하고 초과 시 `RemoteError::InvalidInput`으로 거절한다.

#### [처리담당]

- **판정 결과:** (개발 세션에서 기록)
- **처리 내용:** (개발 세션에서 기록)
- **검증 근거:** (개발 세션에서 기록)

---

### RV-164 — 손실 checkpoint v2(`loss.rs`)의 64배수 비영 행 경계 빈 후행 part 생성 및 `decode`·`summarize` 구간 폭/비최종 part 행 수 검증 보강 (P2)

#### [리뷰어]

- **위치:**
  - [loss.rs:393-433](../../../crates/dot-logger/src/loss.rs#L393-L433) (`Bank::sparse`)
  - [loss.rs:580-591](../../../crates/dot-logger/src/loss.rs#L580-L591) (`LossReport::decode`)
  - [loss.rs:777-858](../../../crates/dot-logger/src/loss.rs#L777-L858) (`summarize`)
  - [storage/writer.rs:732-741](../../../crates/dot-logger/src/storage/writer.rs#L732-L741) (`SegmentWriter::collect_loss`)
  - 계약: [ADR-013 D2 개정(2026-11-21)](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md#손실-checkpoint-0-행-생략-개정--2026-11-21) ("part `i`는 직전 part의 `rows_end`(첫 part는 0)부터 자기 `rows_end`까지를 덮는다", "실제 part 수는 `max(1, ceil(0이 아닌 행/64))` 이하다. 빈 checkpoint도 행 0개인 part 하나를 남긴다").
- **원인 및 영향:**
  1. **비영(nonzero) 행 수가 64(`ROWS_PER_REPORT`)의 양의 배수이고 뒤에 0 행이 남을 때 빈 후행 part가 추가 생성되는 문제 (`loss.rs:402-412`, `writer.rs:733-741`):**
     - `Bank::sparse`는 `for index in start..self.len()` 루프에서 64번째 비영 행을 `counts`에 추가하는 즉시 `if counts.len() == ROWS_PER_REPORT { end = index + 1; break; }`로 루프를 종료한다.
     - 이때 `index + 1 < self.len()`이고 뒤따르는 `(index + 1)..self.len()` 구간이 모두 0 행이면, `collect_loss`는 `start = end < bank.len()`이므로 다음 `part`를 한 번 더 호출해 `counts = []`, `end = bank.len()`인 빈 후행 part를 추가로 기록한다.
     - 즉 비영 행 수 `M`이 64의 양의 배수(`M = 64, 128, ...`)이고 마지막 비영 행이 `N - 1`보다 앞에 있을 때 실제 part 수가 ADR-013의 `max(1, ceil(M / 64))`(예: `M = 64`일 때 1개)가 아니라 `floor(M / 64) + 1`(예: `M = 64`일 때 2개)이 된다(앞선 각 part가 최소 64개 인덱스를 덮으므로 dense 예약 상한 `ceil(N / 64)`는 넘지 않음).
     - `Bank::sparse`에서 64번째 비영 행을 넣은 직후가 아니라 **65번째 비영 행을 만났을 때** `if counts.len() == ROWS_PER_REPORT { end = index; break; }`로 끊으면, 뒤따르는 0 행들이 현재 part의 `end`에 자연스럽게 포함되어 part 수가 항상 `max(1, ceil(M / 64))`와 일치하고 `part > 0`에서 빈 part가 생성되지 않는다.
  2. **`LossReport::decode` 및 `pub fn summarize`의 비최종 v2 part 행 수와 다중 part 구간 폭(`g.end - covered`) 검증 공백 (`loss.rs:580-591, 789-796, 849-857`):**
     - `LossReport::decode`(`582-586행`)는 v2 part에 대해 `end > 0 && end <= rows && r.counts.len() as u64 <= end`만 확인하고, 비최종 part(`end < rows`)가 반드시 `r.counts.len() == ROWS_PER_REPORT`여야 한다는 불변식을 검사하지 않는다. 그 결과 `part = 0, parts = 2, rows = 100, rows_end = 10, counts = []`처럼 비최종 part가 비어 있거나 64행 미만인 손상/변조 레코드도 `decode`와 `Validation::loss_report`를 통과한다.
     - 또한 공개 API인 `pub fn summarize`(`789-796`, `849-857행`)는 입력 `LossReport`의 `r.range == Some(LossRange { end, rows })`에 대해 기본 범위 불변식(`end == 0 || end > rows || (r.counts.len() as u64) > end || u64::from(r.parts) != rows.div_ceil(ROWS_PER_REPORT as u64)`)을 사전 검사하지 않으며, 다중 part v2 checkpoint의 완전성(`complete`) 판정에서도 `g.end > covered`만 확인할 뿐 해당 part가 덮는 구간 폭 `g.end - covered`가 그 part의 행 수 `r.counts.len() as u64` 이상인지(`(r.counts.len() as u64) <= g.end - covered`) 검사하지 않는다(예: part 0이 `end = 64`, part 1이 `end = 65`로 등록 행 1개 구간만 덮는데 `counts`에 10개 행을 담아도 완전한 checkpoint로 인정되어 생략 키를 0 표본으로 합성함).
- **처리 제안:**
  1. `Bank::sparse`에서 `if self.written(index)`일 때 `if counts.len() == ROWS_PER_REPORT { end = index; break; }`를 먼저 확인한 뒤 `counts.extend(self.get(index))`를 수행하도록 수정한다.
  2. `LossReport::decode`와 `summarize` 사전 검증에서 v2 `LossRange`에 대해 `end > 0 && end <= rows && (end == rows || counts.len() == ROWS_PER_REPORT) && (counts.len() as u64) <= end && u64::from(parts) == rows.div_ceil(ROWS_PER_REPORT as u64)`(및 `part == 0 || !counts.is_empty()`)를 검증하고, `summarize`의 v2 연속 구간 판정에서 `(r.counts.len() as u64) <= g.end - covered` 및 `g.end == rows || r.counts.len() == ROWS_PER_REPORT`를 함께 요구한다.

#### [처리담당]

- **판정 결과:** (개발 세션에서 기록)
- **처리 내용:** (개발 세션에서 기록)
- **검증 근거:** (개발 세션에서 기록)

---

### RV-165 — CLI `tail`의 조기 종료 시 기확인 `acknowledged` 집계, `SegmentWriter::projected_index_bytes` 분할 조건 정렬 및 `lan_probe` 동시 명령 한도·수신 Gap 종료 코드 (P3)

#### [리뷰어]

- **위치:**
  - [commands.rs:841-883](../../../crates/dot-logger-cli/src/commands.rs#L841-L883) (`remote` `"tail"`)
  - [writer.rs:615-617, 917-918](../../../crates/dot-logger/src/storage/writer.rs#L917-L918) (`SegmentWriter::append`, `SegmentWriter::projected_index_bytes`)
  - [lan_probe.rs:144-154, 333-344](../../../crates/dot-logger-client/examples/lan_probe.rs#L144-L154) (`grpc_server`, `grpc_receiver`)
- **내용 및 처리 제안:**
  1. **CLI `tail` 조기 종료/실패 시 이미 서버가 확인한 `acknowledged` 건수가 `0`으로 보고되는 점 (`commands.rs:841-883`):**
     `commands::remote("tail")`는 루프 중 `out.acknowledged`를 갱신하지 않고 `if out.records == limit`에서 `stream.flush(&ctx.cancel).await?`가 성공한 직후에만 `out.acknowledged += handed`를 수행한다. 이로 인해 `limit` 도달 전에 앞선 묶음 ACK RPC들이 이미 서버로부터 정상 확인(`reap`/`settle_one` 완료)되었더라도 이후 레코드 대기 중 `--deadline-ms`, 취소, 출력 한도(`Failure::Limit`), 또는 마지막 묶음의 `flush` 실패가 발생하면 최종 summary/error JSON의 `"acknowledged"`가 이미 서버가 확인한 건수 대신 `0`으로 보고된다. `RemoteSubscription`이 각 `AckRpc`에 담긴 항목 수를 함께 보유해 완료 시 누적 확인 건수(예: `acknowledged(&self) -> usize`)를 갱신·노출하고 CLI `tail`이 이를 `out.acknowledged`에 반영하도록 개선을 권고한다.
  2. **`SegmentWriter::projected_index_bytes`의 `split` 조건에 `self.pending_loss_parts > 0` 반영 (`writer.rs:615-617, 917-918`):**
     `SegmentWriter::append`(`615행`)는 `let split = self.pending_loss_parts > 0 || (self.pending_records > 0 && ...);`로 열려 있는 손실 청크 뒤에 사용자 레코드가 오면 청크를 분리하지만, `projected_index_bytes`(`917행`)는 `self.pending_records > 0 && ...`만 확인한다. 현재 내부 호출(`collect_loss`, `recover_inner`)은 손실 기록 후 즉시 `flush_chunk()`를 호출해 `pending_loss_parts == 0`을 유지하나, `append_loss_report`가 `pub(super)`이므로 `projected_index_bytes`의 `split`에도 `self.pending_loss_parts > 0 || ...`을 포함해 두 메서드의 분할 조건을 일치시키는 것을 권고한다.
  3. **`lan_probe` 예제의 서버 `commands: 1` 설정에 따른 HTTP/2 동시 스트림 제한 및 `grpc_receiver` Gap 발생 시 종료 코드 (`lan_probe.rs:147, 333-344`):**
     - `lan_probe::grpc_server`(`147행`)는 `RemoteLimits { connections: 1, subscriptions: 1, commands: 1, .. }`를 사용한다. `RemoteServer::serve`(`server.rs:183-184`)는 `.concurrency_limit_per_connection(limits.commands)`와 `.max_concurrent_streams((limits.commands + limits.subscriptions) as u32)`를 설정하므로, `commands: 1, subscriptions: 1`일 때 HTTP/2 `SETTINGS_MAX_CONCURRENT_STREAMS`는 `2`가 되어 상시 열려 있는 `Subscribe` 스트림 1개를 제외하면 동시에 진행 가능한 `Ack` 스트림이 1개로 제한된다. 클라이언트의 `ACK_IN_FLIGHT = 4` 파이프라인이 HTTP/2 계층에서 직렬화되지 않도록 `lan_probe`의 `commands`를 `4`로 맞추는 것을 권고한다.
     - 또한 `grpc_receiver`(`333-344행`)는 `gaps > 0`이 발생해 `verified < count`여도 JSON 출력 후 `Ok(())`(종료 코드 0)를 반환하므로, 송신 측(`269행`)과 동일하게 `if verified != count || gaps != 0`일 때 `Err`를 반환하도록 정렬을 권고한다.

#### [처리담당]

- **판정 결과:** (개발 세션에서 기록)
- **처리 내용:** (개발 세션에서 기록)
- **검증 근거:** (개발 세션에서 기록)

---

## 5. 검증 및 인계

### 검증 실행 결과

| 검증 | 결과 |
|---|---|
| `cargo +1.96.0 fmt --all --check` | Pass |
| `cargo +1.96.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass |
| `cargo +1.96.0 test --workspace --all-features --locked` | Pass — 705 passed, 0 failed, 5 ignored(외부 DB 시험), RT 할당 20 passed·viewer 8 passed·doctests 24 passed 포함 |
| `RUSTDOCFLAGS='-D warnings' cargo +1.96.0 doc --workspace --all-features --no-deps --locked` | Pass |
| `RUSTDOCFLAGS='-D warnings' cargo +1.85.0 doc --workspace --all-features --no-deps --locked` | Pass |
| `python3 scripts/check-integration-surface.py` | Pass — facade 이름 있는 항목 35/60, L1~L3 레시피·실행·README 일치 |
| 제품 코드·저장소 테스트 변경 | 없음 (Strict Read-Only 준수) |

### 요청 중점(①~⑥) 및 체크리스트 대조 요약

1. **① 묶음·파이프라인 ACK (`4ac9bf1`, `078ca6c` — ADR-015):**
   - `AckRequest`의 `repeated AckItem items` 확장과 `items.is_empty()` 단건 하위 호환, 단건/묶음 필드 혼용 거절(`wire.rs:568-609`), 서버 `Subscription::ack`의 `transfer` 기준 조회에 따른 도착 순서 무관성(`hub.rs:252-256`), 클라이언트 `ACK_BATCH = 64`·`ACK_IN_FLIGHT = 4` 메모리 상한, 소비자 수락 직후 취소 시 `AcknowledgementUnknown` 즉시 전환(`transport.rs:754-756`), `078ca6c`의 `next_inner` 루프 상단 `ACK_DELAY`(5 ms)/유휴 확인을 통한 ACK starvation 해소, 그리고 CLI·예제(`commands.rs`, `bridge.rs`, `pipeline/remote.rs`, `tail/mod.rs`, `lan_probe.rs`)의 `flush` 호출을 확인했다.
   - 다만 `RemoteSubscription`의 미확인/실패 상태 비보존(스트림 종료/실패 후 `flush`가 `Ok(())` 반환), `settle_one`의 조기 `pop_front()`로 인한 `next` 로컬 타임아웃 시 핸들 유실, 서버 묶음 상한의 구독별 `sub.records` 미대조는 **`RV-163` (P2)**, CLI `tail`의 조기 종료 시 기확인 `acknowledged` 건수 반영은 **`RV-165` (P3)**로 보완이 필요하다.
2. **② 손실 checkpoint 0 행 생략 (`af68578` — ADR-013 D2 개정):**
   - `loss_report.v1.bin`(`DESCRIPTOR_V1`) 유지 및 `Validation::schema`/`loss_report`·`LossReader`·`summarize`의 v1/v2 동시 지원, dense 최악값 예약 유지, 완전한 v2 checkpoint에 한한 생략 키 0 표본 합성(`if complete && first.range.is_some()`), 합성 0 표본의 `spare = max_rows - count` 차감을 통한 `max_rows` 상한 준수를 확인했다.
   - 다만 비영 행 수가 64의 배수일 때 빈 후행 part가 추가 생성되는 문제와 `decode`·`summarize`의 비최종 part 행 수/다중 part 구간 폭(`counts.len() <= g.end - covered`) 검증 공백은 **`RV-164` (P2)**로 보완이 필요하다.
3. **③ checkpoint 인덱스 예약·part 묶음 (`8dbbe01`, `0be4bfe`):**
   - `loss_checkpoint_bounds`를 통해 `LoggerBuilder::plan()`(`budget.rs:731`, baseline+final+주기 2회 = 4회)과 `SegmentLog::new`(`retention.rs:360`, 최소 baseline+final = 2회)가 동일한 산식을 공유하고, 모든 곱셈·덧셈에 `checked_mul`/`checked_add` 오버플로 검사를 적용하며, 기존 애플리케이션 기록 여유(`max_chunks: 4096 + loss_chunks`, `max_index_bytes: 4 MiB + loss_index_bytes`)를 보존함을 확인했다. `recovery_chunk_fits`와 `append`/`append_loss_report`의 청크 분할 산정도 일치하며, `projected_index_bytes`의 `pending_loss_parts` 조건 정렬만 **`RV-165` (P3)**로 권고한다.
4. **④ CRC-32 교체 (`0637601`):**
   - `crc_update(crc, bytes)`가 반전 전(un-inverted) 상태값 계약을 그대로 유지하도록 `crc32fast::Hasher::new_with_initial(!crc)`와 `!hasher.finalize()`를 사용하며, 비트 단위 기준 구현(`reference`)과 표준 검사값(`0xcbf4_3926`)·길이(0~4099 B)·임의 분할·임의 초기 상태(`0`, `1`, `0xffff_ffff`, `0x1234_5678`) 대조 시험이 충분함을 확인했다. `crc32fast = "=1.5.3"`은 기존 `mcap 0.26.0`이 이미 잠근 버전으로 `Cargo.lock`에 `dot-logger -> crc32fast` 엣지 1줄만 추가되었고 MSRV 1.85·기존 audit 범위에 영향을 주지 않는다.
5. **⑤ LAN 측정 예제 (`7edae2a` — `lan_probe`):**
   - `crates/dot-logger-client/Cargo.toml`의 `[[example]] name = "lan_probe" (required-features = ["remote"])`로만 추가되어 공개 API나 feature 경계를 넓히지 않으며, sequence 스탬프 및 전체 payload 패턴 대조 로직이 정확함을 확인했다(`commands: 1` 및 수신 Gap 종료 코드는 **`RV-165` (P3)** 참고).
6. **⑥ 시험부 (`tc_017_loss`, `tc_013_facade`, client/remote 시험):**
   - `tc_013_facade::large_native_loss_checkpoints_fit_facade_storage_limits`는 0 행 생략 이후 체크포인트당 빈 v2 part 1개만 기록해 `/tmp` 쓰기량이 수십 KiB 수준으로 작고 `Temp` Drop 시 즉시 정리되며, `tc_017_loss`의 신규 복구·묶음 청크 시험은 인메모리 `Cursor<Vec<u8>>`를 사용해 디스크 쓰기가 없고 결정적으로 동작한다.
7. **RT(실시간) 경로 제약 및 메모리 안전성:**
   - `bd02d49..078ca6c` 구간의 변경은 모두 일반/관리/저장/원격 경로이며 RT 호출·거절·Drop 경로와 `unsafe` 블록은 변경되지 않았다(`dot-logger-rt-allocation` 20개 시험 전부 Pass).

### 개발 인계 — 2026-11-21 #002

- 다음 개발 세션은 **TASK-032 S1 착수 전에** 본 문서(`REV-069`)의 `RV-163~164`(P2) 및 `RV-165`(P3)에 대해 실패 재현 테스트(Red) 작성 → 최소 코드 수정(Green) → 회귀 검증을 수행하고 `[처리담당]` 영역을 갱신한 뒤 마감한다.

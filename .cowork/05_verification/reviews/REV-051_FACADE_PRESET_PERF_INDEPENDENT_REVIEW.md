# [REV-051] GAP-008·TASK-023 S1·TASK-027 S2~S3e·TASK-028 S1 누적 변경 독립 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-11-07 (`#002`) / AI 독립 코드리뷰 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 코드리뷰. 직전 독립 코드리뷰([REV-045](REV-045_TASK-016_OPERATIONS_BACKEND_INDEPENDENT_REVIEW.md), 소스 마감 `497149b` / 문서 `77013e7`) 이후 누적된 모든 제품·검증 코드 변경분(`77013e7..49df7d3`, 10개 소스 커밋, 32개 파일 `+6,850 / -327`줄 — `REV-046`~`REV-050`은 개발 세션 자가 점검)을 객관적 관점에서 전수 검토했다. **제품·테스트 코드는 단 한 줄도 수정하지 않았다.**
- **대상 형상:** `77013e7..49df7d3` (소스 기준 `13523f1..4465be3`, 총 32개 파일)
  - **원격 어댑터(`13523f1`, GAP-008 / EV-045):**
    - [`crates/dot-logger/src/adapters/remote/server.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/server.rs)
  - **성능 기준선 P0 harness(`3740671`, TASK-023 S1 / EV-046):**
    - [`crates/dot-logger/examples/perf_baseline.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/examples/perf_baseline.rs)
    - [`crates/dot-logger/examples/support/perf/mod.rs`](file:///home/lim8603/src/dot-logger/examples/support/perf/mod.rs)
    - [`crates/dot-logger/tests/tc_012_performance.rs`](file:///home/lim8603/src/dot-logger/tests/tc_012_performance.rs)
  - **코어 분해 및 정확 예약 공식 추출(`1af172c`, `2a805ef`, `21cc198` — TASK-027 S2/S3a·TASK-028 S1a):**
    - [`crates/dot-logger/src/capture/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/capture/mod.rs), [`crates/dot-logger/src/capture/tests.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/capture/tests.rs)
    - [`crates/dot-logger/src/completion/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/completion/mod.rs)
    - [`crates/dot-logger/src/control/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/crates/dot-logger/src/control/mod.rs)
    - [`crates/dot-logger/src/routing/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/routing/mod.rs)
    - [`crates/dot-logger/src/storage/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/storage/mod.rs), [`crates/dot-logger/src/storage/retention.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/storage/retention.rs), [`crates/dot-logger/src/storage/catalog.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/storage/catalog.rs)
    - [`crates/dot-logger/src/adapters/tracing.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/tracing.rs)
  - **기본 실행 계층(`logger` facade)·기본 worker·선택 연결·preset(`64c4d23`, `2a805ef`, `91387ba`, `c284354`, `4465be3` — TASK-027 S2~S3c·TASK-028 S1b):**
    - [`crates/dot-logger/Cargo.toml`](file:///home/lim8603/src/dot-logger/crates/dot-logger/Cargo.toml), [`crates/dot-logger/src/lib.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/lib.rs)
    - [`crates/dot-logger/src/logger/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs)
    - [`crates/dot-logger/src/logger/writer.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs)
    - [`crates/dot-logger/src/logger/host.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs)
    - [`crates/dot-logger/src/logger/budget.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/budget.rs)
    - [`crates/dot-logger/src/logger/preset.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/preset.rs)
    - [`crates/dot-logger/src/logger/health.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/health.rs)
    - [`crates/dot-logger/src/logger/affinity.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/affinity.rs)
    - [`crates/dot-logger/src/logger/sink.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/sink.rs)
    - [`crates/dot-logger/src/logger/remote.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/remote.rs)
    - [`crates/dot-logger/src/logger/tests.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/tests.rs)
  - **통합·소비자·RT 할당 검증(`64c4d23`~`4465be3` — TC-013 F1~F15 / EV-047~052):**
    - [`crates/dot-logger/tests/tc_013_facade.rs`](file:///home/lim8603/src/dot-logger/tests/tc_013_facade.rs)
    - [`crates/dot-logger/tests/tc_013_affinity.rs`](file:///home/lim8603/src/dot-logger/tests/tc_013_affinity.rs)
    - [`crates/dot-logger/tests/tc_013_tracing.rs`](file:///home/lim8603/src/dot-logger/tests/tc_013_tracing.rs)
    - [`crates/dot-logger/tests/tc_013_preset.rs`](file:///home/lim8603/src/dot-logger/tests/tc_013_preset.rs)
    - [`crates/dot-logger-client/tests/facade_remote.rs`](file:///home/lim8603/src/dot-logger-client/tests/facade_remote.rs)
    - [`tests/api-consumer/src/main.rs`](file:///home/lim8603/src/dot-logger/tests/api-consumer/src/main.rs)
    - [`tests/rt-allocation/src/tests.rs`](file:///home/lim8603/src/dot-logger/tests/rt-allocation/src/tests.rs)
- **정적 및 동적 검증 결과:**
  - `cargo fmt --check`: Pass
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: Pass (경고 0건)
  - `cargo test -p dot-logger --all-features`: Pass (단위 39건 + `tc_012_performance` 11건 + `tc_013_facade` 11건 + `tc_013_affinity` 3건 + `tc_013_tracing` 4건 + `tc_013_preset` 7건 + 기존 코어·저장소·복구·원격·문서 테스트 전건 통과)
  - `cargo test -p dot-logger-rt-allocation`: Pass (기존 16건 + `facade::f2_*`·`facade::f3_*` 통과)
  - `cargo test -p dot-logger-client --features remote`: Pass (`facade_remote` 4건 포함 전건 통과)
  - `cargo test -p dot-logger-cli`: Pass (22건 전체 통과)
  - `cargo run -p dot-logger-api-consumer`: Pass (`9437184 bytes reserved`)
  - `RUSTDOCFLAGS='-D warnings' cargo doc --locked -p dot-logger -p dot-logger-client -p dot-logger-cli --all-features --no-deps`: Pass (exit 0)
  - Safe Rust 100% (`logger/`·`perf/`·`remote` 신규·변경 코드 내 `unsafe` 블록 0건)
- **종합 판정:** **Conditional Pass** (P1 1건, P2 5건, P3 2건).
  - 아키텍처 분리(ADR-008 borrowed core 불변 보존 + ADR-012 facade 소유 핸들), RT 핫패스(`try_log`·`try_log_raw`) 무할당·무대기 구조, feature 격리, `unsafe` 0건은 매우 우수하나, **`Writer::drop`과 `host_main` reaper 간 파괴 순서 경쟁으로 RT 스레드에서 대규모 heap 해제가 일어날 수 있는 Critical 결함(RV-120)** 및 TASK-023 S2 성능·자원 측정에 직접 영향을 주는 worker 유휴 스핀·시작 실패 정리·산술 오버플로·facade 메모리 예산 누락·`settle` 오류 분류(RV-121~125)가 확인되어 TASK-023 S2 착수 전 조치가 필요하다.
- **상태:** **Resolved** (2026-11-07 #003 개발 세션 조치, Human 최종 구현 리뷰 대기 — 승인 시 Closed)

---

### 확인한 긍정 사항

1. **ADR-008 Borrowed Core 불변성 유지 및 깔끔한 `CaptureCore` 분해:**
   - [`CaptureOwner`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/capture/mod.rs#L92)의 기존 빌림 공개 API(`CaptureScope<'owner>`, `Producer<'owner>`)를 전혀 깨지 않고 내부를 `pub(crate) struct CaptureCore`와 `Endpoint`로 분해하여 facade와 L4 수동 조립 경로가 완전히 동일한 수집 코어와 예약 공식을 공유함.
2. **신규 모듈 전체 100% Safe Rust 달성:**
   - `logger/` 하위 10개 모듈, `examples/support/perf/`, `adapters/remote/server.rs` 수정분 어디에도 `unsafe`를 추가하지 않고 `nix::sched::sched_setaffinity`, `rtrb` SPSC 링, 원자 상태(`AtomicU8`/`AtomicU64`/`AtomicBool`)만으로 구현함.
3. **RT 수집 핫패스(`Writer::try_log`·`try_log_raw`)의 무할당·무대기 규율:**
   - [`Writer::try_log`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs#L198-L234)와 [`Writer::try_log_raw`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs#L244-L270)는 사전 할당된 `scratch` 슬라이스와 임시 [`Producer::from_parts`](file:///home/lim8603/src/dot-logger/src/capture/mod.rs#L653-L660) 뷰만 사용하며, 뮤텍스·시스템콜·동적 포맷팅 없이 고정 `LogError`를 즉시 반환함.
4. **GAP-008(`RemoteServer::serve`) 및 S3b/S3c 격리 설계:**
   - [`RemoteServer::serve`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/server.rs#L172-L195)가 `JoinSet` 드레인과 `active_streams` 알림을 모두 기다린 뒤 반환하여 HTTP/2 stream task 잔류 경쟁을 근본 해결함.
   - [`TracingSink`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/sink.rs#L115)가 thread-local `IN_SINK` 가드로 포맷 중 재진입 교착을 차단하고, [`remote::Tap`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/remote.rs#L22)은 완료 추적 대상과 분리된 관측 사본만 복사하여 원격 구독자 지연이 파일 기록·종료 판정을 막지 않음.
5. **코어 생성자와 facade 예산 공식의 단일 진실 공급원화(TASK-028 S1a):**
   - `capture`, `completion`, `control`, `routing`, `storage`의 예약 바이트 계산을 순수 함수로 추출해 생성자와 `budget::derive`가 동일 함수를 호출하도록 구조화하여 공식 이중화를 제거함.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-120 | **P1** | [`logger/writer.rs#L174-L281`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs#L174-L281), [`logger/host.rs#L188-L194`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L188-L194) — ADR-012 §3 `Writer::drop` 무해제 불변식 | `Writer::drop`이 `mailbox.push(owned)` 직후 필드를 해제하기 전에 `host_main` reaper가 `!is_empty()`를 관측하고 `mailboxes`·`core`를 먼저 drop하면, **RT 스레드가 마지막 `Arc<CaptureCore>`와 `rtrb` 링버퍼(`Owned`·`RawPool`·링 전체)를 직접 deallocate함** | `Arc<CaptureCore>`를 `Owned` 내부로 이동해 반납함과 함께 넘기고, reaper 대기 조건을 `Consumer::is_empty()` 대신 `!Consumer::is_abandoned()`(`rtrb` `Drop`의 마지막 `Release` 플래그)로 변경 | 수용 | Resolved |
| RV-121 | **P2** | [`logger/host.rs#L730-L732`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L730-L732) — `io_loop` 유휴 대기 / `Budget::batching` | `batch_wait == Duration::ZERO` 허용 시 `idle_wait.min(batch_wait)`가 `Duration::ZERO`가 되어 유휴 상태에서 `dot-logger-io` 스레드가 100% CPU busy-spin 발생 (`batch_wait < idle_wait`일 때도 빈 배치에서 불필요 조기 기상) | 보류 중인 배치(`held_since.is_some()`)가 없을 때는 `tuning.idle_wait`만큼 대기하고, 보류 배치가 있을 때만 남은 `batch_wait` 기한과 `min` 계산 | 수용 | Resolved |
| RV-122 | **P2** | [`logger/host.rs#L74-L89`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L74-L89), [`host.rs#L164-L195`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L164-L195), [`logger/mod.rs#L718-L726`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L718-L726) — API §18.2 시작 실패 원자성 | `host_main` 시작 실패(`open_logs`·`Required` affinity 등) 시 `ready.send(Err)`가 `remote.stop()`·reaper 정리보다 먼저 실행되고 `spawn`이 `JoinHandle`을 join하지 않아, `start()`가 `Err`를 반환한 뒤에도 백그라운드 스레드와 바인딩된 TCP 포트가 살아남음 | `start()` 실패 분기에서 외부로 노출되지 않은 `writers`·`sink`를 먼저 drop한 뒤 `host` `JoinHandle`을 동기 `join`하여 리스너·메모리 완전 정리 후 `Err` 반환 | 수용 | Resolved |
| RV-123 | **P2** | [`logger/mod.rs#L700,L867-L886`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L700), [`host.rs#L190,L416,L559`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L190), [`remote.rs#L120-L124`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/remote.rs#L120-L124), [`preset.rs#L75`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/preset.rs#L75), [`budget.rs#L69-L75,L359-L364`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/budget.rs#L69-L75) | 큰 `Duration`(예: `Duration::MAX`)이나 미검증 `WriterQuota`·`Sizing` 입력 시 `Instant + Duration`, `sync_every * 4`, `u128` 곱셈, `remote::default_limits` 합산에서 산술 오버플로 패닉 가능 (`Logger::drop` 내부 패닉 포함) | 시간 기한은 `checked_add`/`saturating_*`로 상한 고정, `Sizing::slots`·`remote::default_limits`는 포화 연산, `budget::derive`는 `checked_add`/`checked_mul` 실패 시 `StartError::InvalidBudget` 반환 | 수용 | Resolved |
| RV-124 | **P2** | [`logger/budget.rs#L332-L356,L483-L540`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/budget.rs#L332-L356), [`logger/mod.rs#L669-L674`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L669-L674), [`host.rs#L328,L377`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L328) — ADR-012 메모리 예산 계약 | ADR-012가 명시한 facade 고유 사전 할당(`Writer`당 `scratch`·1-slot 반납함, 타깃당 `IoTarget::batch`, host `events` 버퍼)이 `budget::derive` 요구량 및 `Plan::required_bytes`에서 누락됨 | `Writer`별 `small_slot_bytes + mailbox` 및 worker/host 고정 버퍼 바이트를 `budget::derive`(예: `worker`/`control` 파티션)에 반영하고 API §19.3 공식과 동기화 | 수용 | Resolved |
| RV-125 | **P2** | [`logger/host.rs#L784-L818`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L784-L818) — `settle()` 오류 승격 및 `FacadeReport::io_failed` 정합성 | ① `continuous` 보존 한도 도달(`AdapterFailure`) 시 IO 장애가 아닌데도 `flags.io_failed`가 `true`로 설정되어 rustdoc·`OutputHealth::failed`(`false`)와 모순됨 ② `AdapterFailure` 분기 내 `log.flush()`가 실제 `StorageError::Io`/`Poisoned`로 실패해도 `reason`이 `IoFailure`로 승격되지 않아 `status.fail()`이 누락됨 | `log.flush()`가 `Io`/`Poisoned`로 실패하면 `reason = FailureReason::IoFailure`로 승격하고, `reason == FailureReason::IoFailure`일 때만 `flags.io_failed`와 `status.fail()`을 기록 | 수용 | Resolved |
| RV-126 | **P3** | [`lib.rs#L43-L48`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/lib.rs#L43-L48), [`logger/writer.rs#L45-L47`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs#L45-L47), [`logger/mod.rs#L318-L330,L529-L539`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L318-L330) — 공개 표면 일관성 및 rustdoc | ① `prelude`에 TASK-028 S1 `Preset`·`Sizing` 미포함 ② 공개 `logger::EncodeError`에 `Display`·`std::error::Error` 미구현 ③ `PendingShutdown::wait` 재호출 시 타임아웃 동작(`Unresponsive`) 및 채널 기본 `minimum_level`(`Trace`) vs `Channel` 기본 발신 레벨(`Info`) 문서 명확화 여지 | `prelude` 재수출 정리, `EncodeError`에 `Display`+`Error` 구현, `PendingShutdown::wait` 및 API §18.2/§18.8 rustdoc 보강 | 수용 | Resolved |
| RV-127 | **P3** | [`examples/perf_baseline.rs#L99,L116`](file:///home/lim8603/src/dot-logger/crates/dot-logger/examples/perf_baseline.rs#L99), [`examples/support/perf/mod.rs#L358,L441`](file:///home/lim8603/src/dot-logger/crates/dot-logger/examples/support/perf/mod.rs#L358) — P0 CLI 경계값 방어 | `perf_baseline --p0` / `--p0-max`에서 `--payload 0` 전달 시 `write_p0` 검증 전에 `/ payload as u64` 0 나눗셈 패닉 발생; `perf/mod.rs` 시간 차 계산 2곳이 raw `-` 연산 사용 | CLI 인자에서 `payload == 0`을 선행 검증(또는 `.max(1)`)하고 타임스탬프 차이에 `saturating_sub` 적용 | 수용 | Resolved |

---

## 3. 상세 리뷰 내역

### RV-120: `Writer::drop`과 `host_main` reaper 사이의 파괴 순서 경쟁으로 RT 스레드에서 heap 해제가 발생할 수 있음 (P1 — CRITICAL)

#### [리뷰어]
- **위치:**
  - [`crates/dot-logger/src/logger/writer.rs#L174-L182`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs#L174-L182) (`pub struct Writer`)
  - [`crates/dot-logger/src/logger/writer.rs#L274-L281`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs#L274-L281) (`impl Drop for Writer`)
  - [`crates/dot-logger/src/logger/host.rs#L188-L194`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L188-L194) (`host_main` 종료 후 reaper 루프)
- **분석 내용:**
  - Accepted [ADR-012 §3](../../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md) 및 NFR-002의 핵심 불변식은 *"RT 스레드가 마지막에 `Writer`를 drop하더라도 `Writer::drop`은 사전 할당된 1-slot 반납함에 `Owned`만 원자적으로 이동하며, 실제 링버퍼·풀·코어 메모리 해제는 반드시 `dot-logger-host`(reaper) 스레드에서 수행된다"*는 것이다.
  - 그러나 현재 `Writer` 구조체와 `Drop` 구현, 그리고 `host_main` reaper 대기 조건을 결합해 분석하면 다음과 같은 경쟁 창(race window)이 존재한다:
    1. Rust의 파괴 순서 규칙상 `<Writer as Drop>::drop(&mut self)` 본문이 먼저 실행된 **직후**, 컴파일러가 생성한 drop glue가 `Writer`의 필드(`core: Arc<CaptureCore>`, `owned: Option<Owned>`, `mailbox: rtrb::Producer<Owned>`)를 순서대로 drop한다.
    2. `Writer::drop` 본문(`writer.rs:277`)에서 `let _ = self.mailbox.push(owned);`가 실행되는 순간 `rtrb`의 `write_idx`가 `Release`로 갱신되어, `dot-logger-host` 스레드의 `rtrb::Consumer::is_empty()`가 즉시 `false`로 바뀐다.
    3. 만약 OS 스케줄러가 `self.mailbox.push(owned)` 직후(아직 RT 스레드가 `self.core`와 `self.mailbox` 필드의 drop glue를 끝내기 전) RT 스레드를 선점하거나 다른 코어에서 깨어난 `dot-logger-host`가 먼저 [`host.rs:188-194`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L188-L194)를 통과하면:
       - `dot-logger-host`는 `while mailboxes.iter().any(rtrb::Consumer::is_empty)` 루프를 즉시 탈출하여 `drop(mailboxes); drop(reads); drop(core);`를 실행한다.
       - `rtrb` 0.4.0(`src/arc_ring_buffer.rs`)의 `ArcRingBuffer::drop`은 먼저 들어온 쪽(`Consumer`)이 `flags.fetch_or(IS_ABANDONED, AcqRel)`만 표시하고 메모리를 해제하지 않은 채 빠져나가며, **나중에 drop하는 쪽(`Producer`)이 `IS_ABANDONED != 0`을 관측해 `drop_slow()`를 실행하여 내부 `Owned`(`WriteState`·`scratch`·`sampling`·링버퍼)와 `RingBuffer` 힙 할당 전체를 해제**한다!
       - 동시에 `dot-logger-host`가 `drop(core)`를 먼저 마쳤으므로 `Arc<CaptureCore>`의 참조 계수는 `2 -> 1`이 되어 있고, 이어서 실행되는 **RT 스레드의 필드 drop glue가 마지막 `Arc<CaptureCore>`(`1 -> 0`)를 drop하면서 `CaptureCore` 전체(`RawPool`, `Completion::Registry`, `Telemetry::Bank`)를 RT 스레드에서 직접 해제(deallocate)**하게 된다.
  - 현재 [`f3_writer_drop_never_allocates_or_frees_in_any_order`](file:///home/lim8603/src/dot-logger/tests/rt-allocation/src/tests.rs#L748-L785)가 통과한 이유는 `host_main`이 1~100ms 간격으로 `sleep`하는 동안 비경합 상태의 RT 스레드가 수십 나노초 만에 필드 drop을 마쳤기 때문이며, 실제 선점·코어 간 경합 상황에서는 무해제 불변식이 깨진다.
  - 부수적으로 [`facade_report`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L597) 역시 `mailboxes.iter().filter(|m| m.is_empty()).count()`로 `live_writers`를 계산하고 있어 극미세 창에서 유사한 관측 경계가 있다.
- **처리 제안:**
  1. `Writer`의 `core: Arc<CaptureCore>` 필드를 [`Owned`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs#L154) 내부로 이동시킨다(평상시 `try_log`에서는 `self.owned.as_mut()`에서 `&CaptureCore`와 가변 필드를 함께 빌려 사용). 이렇게 하면 `self.mailbox.push(owned)` 시점에 RT 스레드는 더 이상 `Arc<CaptureCore>`를 보유하지 않는다.
  2. `host_main`의 reaper 대기 조건(`host.rs:188`)을 `while mailboxes.iter().any(rtrb::Consumer::is_empty)`에서 **`while !mailboxes.iter().all(rtrb::Consumer::is_abandoned)`**로 변경한다. `rtrb`에서 `IS_ABANDONED`는 `Writer`의 마지막 필드인 `rtrb::Producer<Owned>`의 `ArcRingBuffer::drop` 맨 끝 원자 연산(`fetch_or(IS_ABANDONED, AcqRel)`)에서만 설정되므로, `Consumer::is_abandoned()`가 `true`가 된 뒤 `host`가 `drop(mailboxes)`를 수행하면 **항상 `host`가 두 번째 dropper가 되어 `drop_slow()`와 `Owned`·`CaptureCore` 해제를 100% `dot-logger-host` 스레드가 전담**함이 수학적으로 보장된다.
  3. 재현/회귀 시험: reaper 폴링 간격을 0(tight spin)으로 둔 스트레스 조건 또는 카운팅 할당자 하에서 `Logger` 종료 직후 다수 `Writer`를 동시 drop해도 RT 스레드 deallocation이 `0`임을 검증하는 테스트를 보강한다.

#### [처리담당]
- **판정 결과:** 수용 (`Resolved`)
- **처리 내용:** ① reaper 판정을 `host::writers_returned`/`writers_outstanding`으로 꺼내 `Consumer::is_empty` 대신 `Consumer::is_abandoned`(Writer의 mailbox `Producer` drop 완료) 기준으로 바꿨다. host가 항상 두 번째 dropper가 되어 `drop_slow`·`Owned`·`CaptureCore` 해제를 맡는다. `facade_report`의 `writers_outstanding`도 같은 기준. ② `Arc<CaptureCore>`를 `Owned` 안으로 옮겨 push 뒤 RT 스레드가 공유 자원 참조를 하나도 갖지 않게 했다(필드 선언 순서에 의존하지 않음). ③ rtrb 0.4.0 `ArcRingBuffer::drop`(먼저 drop한 쪽은 플래그만, 나중 쪽이 `drop_slow`) 동작을 소스로 확인.
- **검증 근거:** Red: `logger::tests::reaper_waits_until_the_writer_dropped_its_mailbox_producer` — push 후 Producer 생존 상태를 기존 판정이 "반납 완료"로 판단(결정적 재현). Green 후 RT 할당 F3(0/0) 포함 `dot-logger-rt-allocation` 18건 Pass. [EV-053](../verification_evidence.md#ev-053), 소스 `60ea9bb`

---

### RV-121: `Budget::batching(_, Duration::ZERO)` 설정 시 유휴 상태에서 `dot-logger-io` 스레드가 100% CPU busy-spin 함 (P2)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/logger/host.rs#L728-L734`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L728-L734) (`io_loop`), [`crates/dot-logger/src/logger/budget.rs#L170-L174`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/budget.rs#L170-L174), [`budget.rs#L317-L321`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/budget.rs#L317-L321)
- **분석 내용:**
  - `Budget::batching(records, wait)`는 `records == 0`만 거부하고 `wait == Duration::ZERO`(배치 지연 없이 도착 즉시 기록)는 유효한 설정으로 허용한다.
  - 그런데 `io_loop`의 유휴 분기(`!busy && !closing`)는 다음과 같이 작성되어 있다:
    ```rust
    if !busy {
        if closing {
            break;
        }
        std::thread::sleep(tuning.idle_wait.min(tuning.batch_wait));
    }
    ```
  - `tuning.batch_wait == Duration::ZERO`이면 레코드가 전혀 없는 완전 유휴 상태에서도 `sleep(Duration::ZERO)`가 매 회전 호출되어 `dot-logger-io` 워커 스레드들이 종료될 때까지 코어 100%를 소모하며 busy-spin 한다.
  - 또한 기본값(`idle_wait = 1ms`, `batch_wait = 2ms`)과 달리 사용자가 `batch_wait = 100µs`, `idle_wait = 2ms`처럼 설정한 경우에도, 보류 중인 레코드(`target.batch`)가 하나도 없을 때조차 `100µs`마다 불필요하게 깨어난다.
- **처리 제안:**
  - 모든 타깃의 `held_since`가 `None`(보류 중인 배치 없음)일 때는 `tuning.idle_wait`만큼 대기하고, 보류 중인 배치가 있는 타깃들이 존재할 때만 가장 이른 남은 보류 시간 `tuning.batch_wait.saturating_sub(now.saturating_duration_since(held))`와 `tuning.idle_wait` 중 작은 값으로 `sleep`하도록 개선한다.

#### [처리담당]
- **판정 결과:** 수용 (`Resolved`)
- **처리 내용:** 유휴 대기를 `host::idle_sleep`으로 꺼냈다. 보류 배치가 없으면 `idle_wait`, 보류 배치가 있으면 그 남은 보류 시간과 `idle_wait` 중 작은 값만큼 잔다. `batch_wait = 0`에서도 유휴 spin이 없다. `idle_wait = 0`은 사용자가 명시한 성능 값이라 유지했다.
- **검증 근거:** Red: `idle_io_sleep_ignores_batch_wait_until_a_batch_is_held`(`batch_wait=0`에서 0ns 대기). [EV-053](../verification_evidence.md#ev-053), 소스 `60ea9bb`

---

### RV-122: `LoggerBuilder::start()` 중 `host::spawn` 실패 시 백그라운드 스레드 및 원격 리스너 정리 원자성 미보장 (P2)

#### [리뷰어]
- **위치:**
  - [`crates/dot-logger/src/logger/host.rs#L74-L89`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L74-L89) (`host::spawn`)
  - [`crates/dot-logger/src/logger/host.rs#L164-L195`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L164-L195) (`host_main`)
  - [`crates/dot-logger/src/logger/mod.rs#L718-L726`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L718-L726) (`LoggerBuilder::start`)
- **분석 내용:**
  - [`StartError`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L97) 및 [`LoggerBuilder::start`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L597) 문서와 API §18.2는 *"On error nothing keeps running"*을 보장한다.
  - 그러나 `LoggerBuilder::start`는 `remote::Remote::start`로 TCP 리스너 스레드(`dot-logger-remote`)를 먼저 띄운 뒤(`mod.rs:688`) `host::spawn`을 호출한다(`mod.rs:718`).
  - 만약 `host_main` 내부의 `pin(&config)`(예: `Requirement::Required` CPU 고정 실패)이나 `open_logs`(디렉터리 권한·손상된 기존 세그먼트·보존 한도 오류)에서 실패하면:
    1. `host_main`은 `let _ = ready.send(Err(error));`를 **먼저** 보낸 뒤(`host.rs:169`), 이후에 `remote.stop()`(`host.rs:183`, 최대 2초 소요 가능)과 `mailboxes` reaper 루프(`host.rs:188`)로 진입한다.
    2. 한편 호출자 스레드의 `host::spawn`은 `ready_rx.recv()`에서 `Ok(Err(error))`를 받자마자 `join` 핸들을 그냥 drop(detach)하고 즉시 `Err(error)`를 반환한다(`host.rs:85`).
  - 결과적으로 호출자가 `start()`의 `Err`를 받은 시점에 `dot-logger-remote`가 아직 바인딩된 소켓을 닫지 않아 동일 주소 재시도 시 `AddrInUse`가 날 수 있고, `dot-logger-host`도 백그라운드에서 여전히 `CaptureCore` 메모리를 붙잡고 있게 된다.
- **처리 제안:**
  - `LoggerBuilder::start()`에서 `host::spawn`이 실패한 경우, 아직 외부 호출자에게 단 하나의 `Writer`나 `TracingSink`도 반환되지 않았으므로 호출자 스레드에서 로컬 `writers`와 `sink`를 먼저 명시적으로 `drop`한 뒤 `dot-logger-host`의 `JoinHandle`을 `join`하여 `remote.stop()`과 코어 메모리 해제가 완전히 끝난 상태에서 `Err(error)`를 반환하도록 정리한다.

#### [처리담당]
- **판정 결과:** 수용 (`Resolved`)
- **처리 내용:** `host::spawn` 실패가 `SpawnFailed { error, host: Option<JoinHandle> }`를 돌려준다. `start()`는 아직 내보내지 않은 writer·tracing sink를 먼저 drop(상태를 reaper로 반납)한 뒤 host를 join하고 `Err`를 반환한다. 그래서 반환 시점에 listener·core 메모리가 모두 정리돼 있다. 실패 경로 소요는 시험에서 0.00s.
- **검증 근거:** Red: `facade_remote::f13_host_start_failure_closes_the_listener_before_returning`(읽기 전용 출력 디렉터리로 host 내부 `open_logs` 실패 유도, 같은 주소 재bind가 3/3 `AddrInUse`). root 권한이면 유도 불가로 건너뜀. [EV-053](../verification_evidence.md#ev-053), 소스 `60ea9bb`

---

### RV-123: 극단적 `Duration`·`WriterQuota`·`Sizing` 입력 시 산술 오버플로 패닉 가능 (P2)

#### [리뷰어]
- **위치:**
  1. [`crates/dot-logger/src/logger/mod.rs#L700`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L700), [`mod.rs#L867-L886`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L867-L886) (`sync_every.saturating_mul(4) + 1s`, `Instant::now() + deadline`, `deadline + REPORT_GRACE`)
  2. [`crates/dot-logger/src/logger/host.rs#L190`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L190), [`host.rs#L416`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L416), [`host.rs#L559`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L559) (`wait * 2` — 현재 `.min(100ms)` 선행으로 안전하나 방어적 `saturating_mul` 권장, `now + self.config.drop_deadline`, `now + self.config.sync_every.saturating_mul(4) + 1s`)
  3. [`crates/dot-logger/src/logger/remote.rs#L120-L124`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/remote.rs#L120-L124) (`default_limits`)
  4. [`crates/dot-logger/src/logger/preset.rs#L75`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/preset.rs#L75) (`Sizing::slots`)
  5. [`crates/dot-logger/src/logger/budget.rs#L69-L75`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/budget.rs#L69-L75), [`budget.rs#L359-L364`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/budget.rs#L359-L364) (`WriterQuota::records` / `payload_bytes`, `budget::derive` 내 `small_copy`)
- **분석 내용:**
  - 라이브러리 프로덕션 경로는 호출자가 비정상적으로 큰 구성값을 넘기더라도 패닉 대신 정규 오류(`StartError::InvalidBudget`)나 포화 상한으로 처리해야 한다. 현재 다음 경로들에서 패닉이 발생할 수 있다:
    1. **`Instant + Duration` 및 `Duration + Duration` 오버플로 (`Logger::drop` 패닉 포함):** 호출자가 사실상 무제한 대기를 의도해 `Budget::drop_deadline(Duration::MAX)` 또는 `logger.shutdown(Duration::MAX)`를 전달하면 `deadline + REPORT_GRACE`(`mod.rs:880`) 또는 `Instant::now() + deadline`(`mod.rs:868`, `host.rs:416`)에서 표준 라이브러리가 즉시 `panic!`한다. 특히 `Logger::drop` 경로에서 발생하면 언와인딩 중 이중 패닉(`abort`)으로 이어질 수 있다. `sync_every = Duration::MAX`일 때도 `sync_every.saturating_mul(4) + Duration::from_secs(1)`(`mod.rs:700`, `host.rs:559`)이 `+`에서 오버플로 패닉한다.
    2. **`remote::default_limits` 실행 순서에 따른 오버플로:** [`LoggerBuilder::resolve`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L548-L550)는 `budget::derive`로 `WriterQuota`를 검증하기 **전에** `remote::default_limits(&quotas)`를 먼저 호출한다. 따라서 큰 `WriterQuota`(예: `small_slots = usize::MAX`, `small_slot_bytes = 256`)를 넣고 `.remote(...)`를 켠 뒤 `plan()`/`start()`를 호출하면 `StartError::InvalidBudget`을 반환하기 전에 `q.small_slots * q.small_slot_bytes` 또는 `.sum()`에서 패닉(debug) 또는 래핑(release)이 일어난다.
    3. **`Sizing::slots`의 `u128` 곱셈 오버플로:** `preset.rs:75`의 `u128::from(stream.hz) * self.absorb.as_nanos() * 5`는 `absorb = Duration::MAX`(약 $1.8 \times 10^{19}$ 초 $\approx 1.8 \times 10^{28}$ ns)와 큰 `hz`(예: `u32::MAX`)가 조합되면 $4.29 \times 10^9 \times 1.84 \times 10^{28} \times 5 \approx 3.96 \times 10^{38} > u128::MAX(3.40 \times 10^{38})$가 되어 패닉한다.
    4. **`budget::derive` 내부 합산:** `WriterQuota::records()`(`small_slots + raw_slots`), `payload_bytes()`, `small_copy` 합산(`budget.rs:359-364`)이 일반 `+`/`*`/`.sum()`을 사용하여 극단적 입력에서 `StartError::InvalidBudget` 대신 패닉할 수 있다.
- **처리 제안:**
  - `Instant` 기한 계산은 헬퍼(예: `deadline_at(now, d)`에서 `now.checked_add(d)` 실패 시 먼 미래 상한 `now + Duration::from_secs(86_400 * 365)` 등으로 클램프)와 `saturating_add`를 사용해 `shutdown`/`Logger::drop`에서 절대 패닉하지 않도록 방어한다.
  - `Sizing::slots`와 `remote::default_limits`에는 `saturating_mul` / `saturating_add`를 적용한다.
  - `budget::derive`의 `WriterQuota` 합산에는 `checked_add` / `checked_mul`을 적용해 오버플로 시 `Err(StartError::InvalidBudget { .. })`를 반환하도록 통일한다.

#### [처리담당]
- **판정 결과:** 수용 (`Resolved`)
- **처리 내용:** `budget::later(now, d)`(100년 상한 클램프)와 `budget::sync_overdue`(포화 4배+1초)로 `Instant`/`Duration` 덧셈을 대체(`request_shutdown`·host 연결 끊김 종료·ordinary sync 기한·`overdue`), `REPORT_GRACE`는 `saturating_add`, reaper 백오프는 `saturating_mul`. `Sizing::slots`·`remote::default_limits`는 포화 연산, `WriterQuota::records/payload_bytes/small_bytes`는 `checked_*`로 바꿔 `derive`에서 `InvalidBudget`.
- **검증 근거:** Red: `tc_013_preset::f15_unbounded_durations_start_shut_down_and_drop_without_panic`(`Duration::MAX` 덧셈 패닉), `f15_overflowing_sizes_are_rejected_not_panicking`(preset `u128` 곱셈 패닉), `facade_remote::f13_overflowing_quota_with_remote_is_rejected_not_panicking`(`default_limits` 곱셈 패닉 → 수정 뒤 `Remote(Limit)` 거절). [EV-053](../verification_evidence.md#ev-053), 소스 `60ea9bb`

---

### RV-124: ADR-012에서 약정한 facade 고유 힙 할당(`Writer` scratch·반납함·worker batch/event 버퍼)이 `budget::derive` 메모리 예산에서 누락됨 (P2)

#### [리뷰어]
- **위치:**
  - [`crates/dot-logger/src/logger/budget.rs#L332-L356`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/budget.rs#L332-L356), [`budget.rs#L483-L540`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/budget.rs#L483-L540)
  - [`crates/dot-logger/src/logger/mod.rs#L669-L674`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L669-L674) (`Writer`당 `RingBuffer::new(1)` + `vec![0u8; quota.small_slot_bytes]`)
  - [`crates/dot-logger/src/logger/host.rs#L328`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L328), [`host.rs#L377`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L377) (`events: vec![None; 256]`, 타깃당 `Vec::with_capacity(tuning.batch_records)`)
- **분석 내용:**
  - Accepted [ADR-012 §결과(단점·위험 및 완화)](../../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md)는 다음과 같이 명시하고 있다:
    > *"`Writer`당 반환 mailbox(1-slot 링)·scratch 버퍼가 추가된다. facade 고정 overhead로 control/worker 예산에 포함하고 계산 근거를 공개한다."*
  - TASK-028 S1a(`21cc198`)에서 코어 5개 생성자(`CaptureOwner`, `Registry`, `ControlOwner`, `Router`, `SegmentLog`)의 예약 바이트 공식은 정밀하게 일치시켰으나, 정작 facade 계층이 직접 사전 할당하는 다음 힙 메모리들은 `budget::derive`의 `fixed` 및 `Plan::required_bytes`에 포함되지 않았다:
    1. `Writer`당 `scratch: vec![0u8; quota.small_slot_bytes]` 및 1-slot `rtrb::RingBuffer::<Owned>::new(1)` (`mod.rs:669-674`)
    2. 출력 타깃당 `IoTarget::batch: Vec::<Delivery>::with_capacity(tuning.batch_records)` (`host.rs:377`)
    3. `dot-logger-host` 루프의 `events: vec![None; 256]`, `ranges`, `cutoffs` 등 고정 운영 버퍼 (`host.rs:328, 535, 577`)
  - 예를 들어 32 생산자 × `small_slot_bytes = 64 KiB` 구성이나 16개 출력 × 큰 `batch_records` 구성에서는 수 MiB 규모의 실제 힙 할당이 `Budget::memory_bytes` 한도 검사를 우회하며, 곧 진행할 TASK-023 S2/S3(P4 메모리 예산 일치 검증)에서도 예약량 대비 실할당 오차 원인이 된다.
- **처리 제안:**
  - facade 고유 사전 할당량(`Writer`당 `scratch` + 1-slot `mailbox`, 타깃당 `batch` 벡터 용량, host 고정 이벤트 버퍼)을 순수 계산 함수(예: `facade_overhead_bytes`)로 산출해 `budget::derive`의 `worker`(또는 `control`) 파티션 요구량과 `Plan::required_bytes`에 포함시키고, API §19.3 설명과 `tc_013_preset` F14 검증식을 함께 갱신한다.

#### [처리담당]
- **판정 결과:** 수용 (`Resolved`)
- **처리 내용:** `budget::facade_bytes`가 writer별 scratch(small slot)+1-slot mailbox(`size_of::<RingBuffer<Owned>>() + size_of::<Owned>()`), 출력별 IO batch(`batch_records × size_of::<Delivery>()`), host 이벤트 버퍼(`HOST_EVENTS=256`)·producer별 sync 범위 배열 3개를 계산해 worker partition과 `required_bytes`에 더한다. host는 같은 `HOST_EVENTS` 상수를 쓴다. 스레드 스택(기존 별도 계산)·채널 큐·유한 sync 작업 목록은 제외(rustdoc·API §19.4에 명시).
- **검증 근거:** Red: `tc_013_preset::f15_facade_buffers_are_charged_to_the_worker_partition`(slot 64B→64KiB에도 worker 증가 0). F14 정확 요구량 시험 유지 Pass. [EV-053](../verification_evidence.md#ev-053), 소스 `60ea9bb`

---

### RV-125: `host::settle()`에서 보존 용량 도달(`AdapterFailure`)과 실제 IO 실패(`IoFailure`)의 상태·리포트 분류 불일치 (P2)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/logger/host.rs#L784-L818`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/host.rs#L784-L818) (`settle`)
- **분석 내용:**
  - `write_batch`에서 `target.log.append_batch(...)`가 실패하면 `settle`이 호출되어 부분 기록된 앞쪽 레코드들과 실패한 뒤쪽 레코드들을 정산한다. 이때 두 가지 정합성 문제가 있다:
    1. **보존 한도 도달 시 `FacadeReport::io_failed` 오표시:** `Retention::continuous(max_bytes)` 모드에서 디스크/파일 오류 없이 순수하게 설정된 보존 상한에 도달하면 `RetentionError::Capacity` → `FailureReason::AdapterFailure`로 분류된다. 이때 [`tc_013_facade.rs:421`](file:///home/lim8603/src/dot-logger/crates/dot-logger/tests/tc_013_facade.rs#L421)이 검증하듯 `OutputHealth::failed`는 `false`이고 `HealthState`는 `Degraded(RecordsFailed)`이다. 그런데 `host.rs:809`는 `reason`을 구분하지 않고 무조건 `flags.io_failed.store(true, Ordering::Release);`를 실행한다. 이는 [`FacadeReport::io_failed`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L271)(*"A file write, sync or final publication failed"*)의 정의 및 `OutputHealth::failed`와 모순된다. (`FacadeReport::is_success()`는 이미 `self.core.is_success()`가 `required.failed == 0`을 검사하므로, 필수 출력에서 보존 한도로 기록이 실패하면 `io_failed == false`여도 `is_success()`는 정확히 `false`가 된다.)
    2. **`AdapterFailure` 분기 내 `log.flush()` IO 실패 시 `IoFailure` 미승격:** `reason == FailureReason::AdapterFailure && appended > written`일 때 앞쪽 `appended`건의 내구성을 확보하기 위해 `target.log.flush()`를 호출한다(`host.rs:792`). 만약 이 `flush()`가 실제 디스크 오류(`StorageError::Io(_) | StorageError::Poisoned`)로 실패하면 앞쪽 레코드들은 `Update::Failed(FailureReason::IoFailure)`로 보고되지만(`host.rs:797`), 지역 변수 `reason`은 여전히 `AdapterFailure`로 남아 있어 뒤쪽 레코드들이 `AdapterFailure`로 잘못 보고되고 `target.status.fail()`(`host.rs:810-812`)도 호출되지 않아 `Health`가 `Failed(IoPoisoned)`로 전이되지 않는다.
- **처리 제안:**
  - `target.log.flush()`가 `Err(StorageError::Io(_) | StorageError::Poisoned)`를 반환하면 `reason = FailureReason::IoFailure;`로 승격시킨다.
  - 하단 플래그 갱신에서는 `if reason == FailureReason::IoFailure { flags.io_failed.store(true, Ordering::Release); target.status.fail(); }`로 통일하여 `Health`와 `FacadeReport::io_failed`가 동일한 IO 장애 의미를 갖도록 맞춘다.

#### [처리담당]
- **판정 결과:** 수용 (`Resolved`)
- **처리 내용:** ① `settle`: prefix flush가 `Io`/`Poisoned`로 실패하면 `host::tail_reason`으로 뒤쪽 전체를 `IoFailure`로 승격. `io_failed`·`status.fail()`은 `IoFailure`일 때만 기록하고 보존 한도 등은 `records_failed`만 올린다. ② 같은 결함 유형이 sync 경로에도 있어 함께 정리: 보존 한도로 기록되지 않은 범위를 덮는 barrier가 `InvalidRecord`로 실패하면 barrier만 `Failed`(core `sync_failed`)이고 출력 IO 장애로 표시하지 않는다. 실제 `Io`/`Poisoned`만 IO 장애.
- **검증 근거:** Red: `settle_promotes_a_failed_prefix_flush_to_io_failure`(단위), `tc_013_facade::f9_*`에 `!report.io_failed` 단언 추가(settle 수정 뒤에도 sync 경로로 true → ② 수정 후 Pass). F5 실제 파일 장애는 계속 `io_failed`. [EV-053](../verification_evidence.md#ev-053), 소스 `60ea9bb`

---

### RV-126: `prelude` 재수출 범위(`Preset`·`Sizing`) 및 `EncodeError` 표준 트레이트·rustdoc 세부 정합성 보강 (P3)

#### [리뷰어]
- **위치:**
  - [`crates/dot-logger/src/lib.rs#L43-L48`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/lib.rs#L43-L48) (`pub mod prelude`)
  - [`crates/dot-logger/src/logger/writer.rs#L45-L47`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs#L45-L47) (`pub struct EncodeError`)
  - [`crates/dot-logger/src/logger/mod.rs#L318-L330`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L318-L330) (`PendingShutdown::wait`)
  - [`crates/dot-logger/src/logger/mod.rs#L529-L539`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/mod.rs#L529-L539) (`LoggerBuilder::resolve` 채널 기본 레벨)
- **분석 내용:**
  1. **`prelude` 내 `Preset`·`Sizing` 누락:** API §18.1/§18.6/§19.1에서 L1 최소 통합 경로는 `use dot_logger::prelude::*;`와 `.preset(Preset::control(...))` 조합을 전제로 하나, TASK-028 S1b에서 `Preset`·`Sizing`이 `logger`와 크레이트 루트에만 재수출되고 `prelude`에는 추가되지 않았다.
  2. **`logger::EncodeError`의 `Display` / `std::error::Error` 미구현:** 같은 모듈의 `StartError`, `LookupError`, `ShutdownError` 및 `adapters::tracing::EncodeError`는 모두 `Display`와 `std::error::Error`를 구현하지만 `logger::EncodeError`만 단위 구조체로 `Display`/`Error`가 빠져 있어 비-RT 테스트나 일반 호출부에서 `?` 전파 시 불편하다(정적 문자열 출력이므로 RT 무할당 제약과도 충돌하지 않음).
  3. **rustdoc 명확화 2건:**
     - `PendingShutdown::wait(self, timeout)` 문서는 *"Wait up to `timeout` for the next report"*라고만 적혀 있으나, `Host`는 `Incomplete`를 최초 기한 만료 시 단 1회만 보내고(`incomplete_sent = true`, `host.rs:474`) 이후에는 최종 `Progress::Closed`만 보내므로 `PendingShutdown::wait`가 타임아웃되면 `Ok(Shutdown::Incomplete)`가 아니라 `Err(ShutdownError::Unresponsive(self))`가 반환된다. 이를 `# Errors`에 명시하면 호출자의 오해를 막을 수 있다.
     - API §18.2 표에는 채널 기본값이 `level=Info`로 적혀 있는데, 실제 구현은 발신 핸들 [`Channel::new`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/logger/writer.rs#L80)의 레코드 기본 레벨이 `Level::Info`이고 수집 필터인 `ChannelConfig::minimum_level`은 `Level::Trace`이다(`Channel::level(Level::Debug)`로 낮춘 발신도 통과시키기 위함). 합리적인 설계이므로 API §18.2/rustdoc에 "기본 발신 레벨 `Info`, 채널 수용 하한 `Trace`"임을 한 줄로 구분해 두면 명확하다.
- **처리 제안:**
  - `dot_logger::prelude`에 `Preset`, `Sizing`을 재수출하고, `logger::EncodeError`에 `fmt::Display`와 `std::error::Error`를 구현하며, `PendingShutdown::wait` `# Errors` 및 채널 기본 레벨 문구를 보강한다.

#### [처리담당]
- **판정 결과:** 수용 (`Resolved`)
- **처리 내용:** `prelude`에 `Preset`·`Sizing` 재수출, `logger::EncodeError`에 `Display`("record does not fit the encode buffer")·`std::error::Error` 구현, `PendingShutdown::wait`에 `# Errors`(Incomplete 1회·이후 Unresponsive 의미), `LoggerBuilder::channel` rustdoc과 API §18.2 표에 발신 기본 `Info`·채널 수용 하한 `Trace` 구분.
- **검증 근거:** Red(컴파일): `tc_013_preset::prelude_surface::f15_prelude_has_presets_and_encode_error_is_an_error`. [EV-053](../verification_evidence.md#ev-053), 소스 `60ea9bb`

---

### RV-127: `perf_baseline` CLI `--payload 0` 분모 0 패닉 방어 및 타임스탬프 포화 뺄셈 정리 (P3)

#### [리뷰어]
- **위치:**
  - [`crates/dot-logger/examples/perf_baseline.rs#L99`](file:///home/lim8603/src/dot-logger/crates/dot-logger/examples/perf_baseline.rs#L99), [`perf_baseline.rs#L116`](file:///home/lim8603/src/dot-logger/crates/dot-logger/examples/perf_baseline.rs#L116)
  - [`crates/dot-logger/examples/support/perf/mod.rs#L358`](file:///home/lim8603/src/dot-logger/crates/dot-logger/examples/support/perf/mod.rs#L358), [`perf/mod.rs#L441`](file:///home/lim8603/src/dot-logger/crates/dot-logger/examples/support/perf/mod.rs#L441)
- **분석 내용:**
  - `perf::write_p0` 내부는 `workload.payload_bytes == 0`일 때 `"invalid P0 workload"` 오류를 반환하도록 방어되어 있으나(`perf/mod.rs:603`), 정작 CLI 진입점인 `perf_baseline.rs`의 `--p0`(99행)와 `--p0-max`(116행)에서 `rotation_records = (64 * 1024 * 1024 / payload as u64).max(1)`을 먼저 계산하기 때문에 `--payload 0` 인자를 주면 `write_p0` 도달 전에 0 나눗셈 패닉이 발생한다.
  - 또한 `perf/mod.rs` 대부분은 `saturating_sub`를 일관되게 사용하나 358행(`now_ns() - t`)과 441행(`done - origin`) 두 곳만 일반 `-` 연산이 남아 있다.
- **처리 제안:**
  - `perf_baseline.rs`에서 `payload == 0`이면 즉시 사용법/검증 오류를 반환하도록 가드하고, `perf/mod.rs` 358·441행도 `saturating_sub`로 통일한다.

#### [처리담당]
- **판정 결과:** 수용 (`Resolved`)
- **처리 내용:** `perf::records_for(total, payload)`가 `payload == 0`을 사용법 오류로 반환하고 `--p0`·`--p0-max` 모두 이를 쓴다. `perf/mod.rs` 두 곳 `saturating_sub`. (리뷰 당시 서술한 `64MiB / payload` 식은 현재 `rate×seconds / payload`·`total / payload`였고 같은 0 나눗셈이다.)
- **검증 근거:** Red(컴파일): `tc_012_performance::p0_record_count_rejects_a_zero_payload`. `perf_baseline --p0-max … 0 …` → `payload must be at least 1 byte`, exit 4. [EV-053](../verification_evidence.md#ev-053), 소스 `60ea9bb`

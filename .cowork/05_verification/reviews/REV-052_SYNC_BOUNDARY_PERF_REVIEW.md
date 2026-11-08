# REV-052 — sync 경계·TASK-023 S2a/S2b 독립 코드리뷰

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-052 / 2026-11-07 |
| 리뷰어 | Codex AI — 리뷰 전용 세션 |
| 대상 | REV-051 이후 조치와 TASK-023 S2a/S2b 누적 변경, `49df7d3..93db6e3` |
| 주요 소스 | `60ea9bb`, `1602d5e`, `93e7241`, `c93a44a`, `a92aee6`, `97651f5`, `49f097d`, `eeec11f`, `d0c4851`, `a6938ca` |
| 범위 | 29파일 중 Rust 21파일의 변경 및 관련 완료·저장소·제어 계약. 시작 작업 트리 clean |
| 종합 판정 | **Conditional — P1 2건·P2 1건. 공개 L4 sync 경계 조치와 측정 구간 정렬 필요** |
| 상태 | **Closed** — 2026-11-08 #001 조치·검증([EV-056](../verification_evidence.md#ev-056), 정정 [EV-057](../verification_evidence.md#ev-057)), Human 「승인」 |

채널별 출력 선택·예산 배정·RT Writer 반환·IO 종료 플래그 순서·완료 보고 재시도는 변경 코드와 기존 시험에서 확인했다. 새 `unsafe`는 없다. 기존 전체 회귀는 통과하지만, 부분 라우팅을 위해 완화한 공통 sync 판정이 **저장소 포함 범위 증명**과 **연속 Written watermark**를 깨뜨리는 조건을 발견했다. 두 건 모두 공개 API만 사용하는 저장소 밖 임시 시험에서 재현했다. 현재 facade의 `SegmentLog`·`capture_scope(first=1)` 경로만으로 공개 L4 `SegmentWriter`와 임의 `CompletionScope`의 안전성을 대체할 수 없다.

제품 코드와 저장소 테스트는 수정하지 않았다. 아래 처리 제안은 후속 개발 세션에서 TDD로 수행한다. REV-051의 기존 Closed 판정이나 EV-053~055의 과거 실행 사실은 변경하지 않는다. 이번에 발견한 검증 공백을 이 회차로 추적한다.

## 2. 파일 변경표

이번 리뷰 세션에서 실제 바꾼 파일이다. 대상 제품 변경과 구분한다.

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | [REV-052](REV-052_SYNC_BOUNDARY_PERF_REVIEW.md) | 발견·재현·검증·후속 처리 영역 |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | Open 리뷰 등록·다음 개발 시작점 동기화 |
| 수정 | [TASK-023.md](../../04_implementation/tasks/TASK-023.md) | 리뷰 조치 후 S2c로 다음 액션 정렬 |
| 수정 | [my_state.md](../../members/lim8603/workspace/my_state.md) | 오래된 REV-051 Open 표기를 현재 REV-052로 교체 |
| 생성(로컬·git 제외) | [세션 #004](../../members/lim8603/workspace/session_logs/session_2026-11-07_004.md) | 리뷰 인계와 실행 로그 위치 |

검토한 제품 변경은 `logger/{mod,budget,host,writer,preset,remote,tests}.rs`, `lib.rs`, `routing/mod.rs`, `control/mod.rs`, `storage/{mod,retention}.rs`, 성능 예제 3파일과 통합 시험 5파일이다.

## 3. 클래스(타입) 변경표

**이번 세션의 struct·enum·trait 변경은 없음.** 검토 대상에는 `ChannelOptions`, `ChannelSpec`, `StartError`, `Plan`, `Owned`, `Writer`, `SpawnFailed`, `Delivery`, routing `Input`·`Target`·`TargetWriter`, `StorageWorkload`, `StorageRun`, `WriteCalls`, `Stop`, `Rejection`, `LatencyHistogram`, `SequenceOracle`, `LoggerLoad`, `LoggerRun`, `LoggerVerified`, `CallCost` 등이 포함된다.

## 4. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-128 | P1 | `control/mod.rs:883~922`, `SyncWork::covers_file`; `storage/writer.rs::check_sync` | Written 완료 이력의 Expired를 안전한 파일 범위 제외로 취급해, 이전 파일을 sync하지 않고 새 빈 파일만 sync한 요청이 성공함 | 완료 이력 만료와 파일 내구성 증명을 분리. `SegmentLog`와 단일 segment의 포함 범위 검증 구분 | 수용 | Closed |
| RV-129 | P1 | `routing/mod.rs:849~865`, `Dispatcher::ready_for_sync` | `[2,2]` 부분 범위가 watermark를 2로 올리면 미기록 `[1,1]`에도 SyncWork 발급 | 연속 prefix만 공유 watermark에 반영. 비연속 부분 범위는 요청별 확인 | 수용 | Closed |
| RV-130 | P2 | `examples/support/perf/facade.rs:213~215`, `perf/mod.rs:659`, `perf_baseline.rs:306~309` | P0는 파일 생성 전, logger는 start 완료 뒤부터 계시. 제어 루프는 logger 준비 중에도 측정하므로 부하와 같은 구간이라는 보장이 없음 | 양쪽 setup·예열 뒤 공통 시작 시각/장벽으로 부하와 제어 계시. 실제 중첩 구간 검증 | 수용 | Closed |

### RV-128 — 만료된 Written 이력으로 다른 파일의 sync를 성공시킴 (P1)

#### [리뷰어]

- **위치:** [SyncWork::covers_file](../../../crates/dot-logger/src/control/mod.rs:883), [check_sync](../../../crates/dot-logger/src/storage/writer.rs:90), [FinishedSegment::sync_work](../../../crates/dot-logger/src/storage/writer.rs:267), [SegmentWriter::sync_work](../../../crates/dot-logger/src/storage/writer.rs:648).
- **문제:** `c93a44a`는 파일 밖 구간을 최신 window로 좁히고 `Expired` 거절을 제거했다. 그러나 완료 기준이 `Written`이면 TERMINAL·Expired는 **파일에 쓰였다는 과거 사실**일 뿐 `Synced`나 다른 파일로의 대체 가능성을 증명하지 않는다. `covers_file`은 전체 log 범위를 보관하는 `SegmentLog`뿐 아니라 단일 `SegmentWriter`·`FinishedSegment`도 공통으로 사용한다. 주석의 “이 log가 받은 모든 기록” 전제를 이 세 호출자에 일괄 적용할 수 없다.
- **재현:** 기존 `tc_004_storage::a_new_segment_cannot_acknowledge_an_older_segments_unsynced_records`를 저장소 밖으로 복제했다. `tracking.targets[0].completion = Written`, `history_retention = 1ms`로 설정하고 첫 파일 `finish()` 뒤 10ms 대기했다. `finish`는 sync하지 않는 공개 계약이다(`writer.rs:682`). 이전 receipt가 Expired인 것을 확인한 뒤 같은 target의 새 **빈** `SegmentWriter`에 이전 cutoff의 SyncWork를 전달했다.

```text
old receipt: RecordStatus { confirmed: Boundaries(0), outcome: Expired }
unrelated empty file sync result: Ok(())
barrier after syncing only empty file:
SyncSnapshot { queued: 0, in_flight: 0, applying: 0, succeeded: 1,
  failed: 0, unknown: 0, expired: 0, deadline_elapsed: false, finished: true }
```

- **기대:** 이전 파일을 sync하지 않았으므로 새 파일은 `InvalidRecord`와 원래 work를 반환해야 한다. 현재는 이전 파일에 `sync_all`을 호출하지 않고 barrier 성공을 보고한다. receipt 자체의 Expired는 그대로지만 요청 성공 판정이 거짓이다.
- **영향:** Written을 완료 기준으로 쓰는 공개 L4 조립·수동 segment 교체에서 명시 내구성 요청이 실제 파일을 건너뛸 수 있다. facade `SegmentLog`는 과거 회전 파일을 sync하고 자체 전체 범위를 유지하므로 이 재현과 구분한다.
- **처리 제안:** 단일 segment에서는 범위 밖의 Expired를 보수적으로 거절한다. 부분 라우팅의 “이 target에 오지 않은 오래된 기록”은 완료 이력 만료 대신 지속되는 target별 라우팅/파일 coverage 증명으로 처리한다. 필요하면 전체 log 전용 검증과 단일 segment 검증을 나눈다. 최신 window 밖 범위를 생략하는 최적화도 같은 증명 아래에서만 허용한다. API §19.5의 Expired 문구를 실제 증명 범위로 한정한다.
- **후속 시험:** 기존 Synced 완료 기준의 시험 유지 + Written 기준의 시간 만료·window 재사용 각각에 대해 새 active/finished 파일이 이전 미동기화 파일을 대신하지 못함을 검증한다. F16 부분 라우팅·60초 이상 idle 생산자·동일 log 회전 sync도 함께 회귀한다.

#### [처리담당]

- **판정 결과:** 수용 — 유효(공개 L4 조립에서 재현). 2026-11-08 #001 AI.
- **처리 내용:** 완료 이력 만료와 파일 내구성 증명을 분리했다. target별 `FileLedger`(routing `TargetLedger` 소유, writer 교체 뒤에도 유지)가 ① producer별로 이 target의 파일이 append한 최고 seq와 ② sync 없이 놓인 파일들의 범위(hull)·개수를 기록한다. 각 파일은 시작 시 ①의 값을 `base`로 받는다. `covers_file`은 범위 밖 구간 중 **다른 파일이 미동기화로 가질 수 있는 부분**(base 이하의 owed hull, base 초과·현재 appended 이하)에 대해 Synced·NotRouted·실패 종료의 양의 증명을 요구한다. 그 구간이 window보다 길거나 Expired면 거절한다. 어떤 파일도 append하지 않은 부분만 기존처럼 최신 window의 Pending만 확인한다. 완료 기준이 Synced인 target은 종료가 Synced/실패를 뜻하므로 증명 생략. 부채는 단독 `SegmentWriter`의 `finish`(dirty일 때)·Drop, `SegmentLog` Drop(dirty일 때)에서 기록하고, `FinishedSegment::sync_work`·`sync_recovered` 성공이 해소한다. `SegmentLog` 내부 파일은 log가 전체 범위를 갖고 회전 시 sync하므로 부채를 만들지 않는다(facade 경로 동작 불변). routing 예약에 producer당 24B 추가.
- **후속 정정(2026-11-08 #001, TASK-023 S2c에서 발견):** `97e2b12`의 "이후 파일" 증명 구간이 파일 **첫 기록 이전**의 다른 target 전용 기록까지 포함해, 부분 라우팅 출력에서 완료 slot이 재사용되면 종료 sync를 거절했다(혼합 실행에서 재현). 구간 시작을 파일의 `last` 뒤로 고쳤다 — `b2899a3`, Red→Green `tc_004_storage::an_unrouted_record_before_the_file_never_needs_a_proof`. [EV-057](../verification_evidence.md#ev-057).
- **검증 근거:** 소스 `97e2b12`. Red→Green: `tc_004_storage::{an_expired_written_record_cannot_let_a_new_segment_skip_its_unsynced_file, a_reused_written_record_cannot_let_a_new_segment_skip_its_unsynced_file, a_finished_segment_cannot_skip_a_later_files_expired_unsynced_records}`(세 건 모두 수정 전 잘못된 `Ok`), `tc_005_retention::a_dropped_unsynced_log_keeps_its_records_owed_to_a_later_log`(Drop 부채 기록을 끈 돌연변이에서 Red 확인). 정상 경로: 이전 파일이 같은 work로 sync하면 성공하고 이후 새 파일이 이전 범위를 다시 덮을 수 있다. F16 부분 라우팅·tc_013 facade·기존 `a_new_segment_cannot_acknowledge_*`·회전 sync 회귀 Pass. [EV-056](../verification_evidence.md#ev-056).

### RV-129 — 부분 범위가 공유 Written watermark의 앞쪽 공백을 건너뜀 (P1)

#### [리뷰어]

- **위치:** [Dispatcher::ready_for_sync](../../../crates/dot-logger/src/routing/mod.rs:849), [Completion::scope](../../../crates/dot-logger/src/completion/query.rs:39), [ControlOwner::next_sync](../../../crates/dot-logger/src/control/mod.rs:402).
- **문제:** `seq = max(mark+1, ..., range.first)`에서 `range.first`로 건너뛴 공백을 검증하지 않고 `*mark = seq`를 저장한다. 공개 `Completion::scope`는 first가 1이 아닌 범위를 허용하므로 `written[p]`가 “이 이하 모두 Written/종료”라는 타입 내부 불변식을 잃는다. 이후 더 앞선 범위는 loop 자체를 실행하지 않고 ready=true가 된다.
- **재현:** 완료 window 4, 같은 생산자에서 seq1·seq2를 받아 두 delivery를 꺼낸다. seq1 guard는 Written 보고 없이 보유하고 seq2만 Written으로 보고한다. `[2,2]` 요청을 발급·완료·poll·release한 다음 `[1,1]` 요청을 만든다. `sync_slots=1`에서도 순차 요청으로 재현된다.

```text
completion.poll(seq1, target=10, Written) == Pending
next_sync(scope=[1,1], target=10).is_some() == true
```

- **기대:** seq1은 아직 실제 IO가 끝나지 않았으므로 `next_sync`가 None을 반환해야 한다. 단일 파일 writer의 coverage 검사로 일부 잘못된 work가 거절되더라도 공통 제어 API가 이를 발급해선 안 된다. 별도 어댑터가 이 readiness 보장을 따라 sync 후 `complete(Synced)`하면 미기록 receipt에 Written/Synced가 확정될 수 있다.
- **범위:** facade의 현재 `capture_scope`는 first=1이어서 이 순서로 불변식을 깨지 않는다. 공개 L4 임의 범위 요청이 영향 대상이다.
- **처리 제안:** 전역 mark는 실제로 검증한 **연속 prefix**만 전진시킨다. 부분 요청이 prefix와 떨어져 있으면 그 요청의 범위는 별도로 검사하되 mark를 건너뛰어 갱신하지 않는다. 또는 mark부터 공백까지 확인하는 유한 경로를 사용한다. `range.last-window+1` 생략에는 terminal과 Written의 차이 및 target 완료 기준도 검토한다.
- **후속 시험:** `[2,2]` → `[1,1]`, `[n,n]` → 전체 scope, 겹치는 범위, 역순 완료, window 재사용을 추가하고 F17 연속 부하 sync가 굶지 않는지 함께 확인한다.

#### [처리담당]

- **판정 결과:** 수용 — 유효. 2026-11-08 #001 AI.
- **처리 내용:** `ready_for_sync`는 검사 시작점이 `mark + 1`(window 생략 포함)과 같을 때만 `mark`를 전진시킨다. 더 뒤에서 시작하는 범위는 따로 검사하고 공유 watermark를 건드리지 않는다. 처리 제안의 "terminal과 Written 차이·완료 기준" 검토 결과, 완료 기준이 Enqueued인 target은 Written 전에 종료(및 이력 만료)되므로 window 생략·Expired 통과 경로를 완료 기준 Written/Synced인 target으로 한정하고, 나머지는 기존 drained 조건을 쓴다.
- **검증 근거:** 소스 `97e2b12`. Red→Green: `tc_003_control::{a_partial_barrier_cannot_skip_an_earlier_unwritten_record, a_full_barrier_after_a_partial_one_still_waits_for_the_gap, an_enqueued_completion_target_does_not_treat_an_expired_record_as_written}`. 보호 시험 `overlapping_partial_barriers_wait_for_the_gap_in_either_order`(겹침·역순). F6 연속 부하 주기 sync(tc_013 facade) 회귀 Pass — 굶지 않음. [EV-056](../verification_evidence.md#ev-056).

### RV-130 — P0·logger·제어 루프가 같은 측정 시작점을 공유하지 않음 (P2)

#### [리뷰어]

- **위치:** [write_logger](../../../crates/dot-logger/examples/support/perf/facade.rs:213), [write_p0](../../../crates/dot-logger/examples/support/perf/mod.rs:659), [loops](../../../crates/dot-logger/examples/perf_baseline.rs:306).
- **근거:** P0의 `start`는 writer thread 생성·pin·`Output::open`·버퍼 준비 **전**에 잡힌다. 반면 logger는 `plan`·`start`·채널/Writer 조회가 끝난 **뒤**에 start를 잡는다. “P0도 파일 열기를 setup으로 제외한다”는 facade 주석은 실제 P0 코드와 다르다. 양쪽 `StorageWorkload`가 같아도 효율 분모에 포함한 작업이 다르다.
- **제어 구간:** `loops`는 control/safety thread를 시작한 뒤 `write_logger`를 호출한다. 로거 사전 할당·파일 생성에 걸린 시간 동안에도 control은 지정 release를 소비하고, logger 부하는 자체 start부터 다시 seconds만큼 실행한다. 준비가 지연되면 제어 결과의 앞부분은 부하가 없고 뒷부분 부하는 제어 관측 밖이며, 이를 검출할 공통 시작 시각·중첩 길이도 없다. S3의 예열10초·120초 off/on 비교 전에 정렬해야 한다.
- **처리 제안:** 두 tool에서 동일하게 setup/예열을 분리하고 worker·생산자·control·safety가 준비된 다음 같은 절대 시작 시각을 사용한다. 부하 구간과 최종 내구화 tail을 분리 기록하고 유효 제어 구간이 부하와 겹치는지 판정한다. startup을 포함하는 별도 지표가 필요하면 양쪽 모두 같은 경계를 적용한다.
- **검증:** 정적 경로 대조로 확인. 이번 세션에서 성능/RT 측정은 실행하지 않았다. 후속 시험에서는 한쪽 setup에 인위적 지연을 주어 처리량·제어 관측 구간이 흔들리지 않고, 중첩이 부족하면 수락되지 않는지 검증한다.

#### [처리담당]

- **판정 결과:** 수용 — 유효(정적 대조 확인). 2026-11-08 #001 AI.
- **처리 내용:** 측정 harness에 공통 시작 장벽 `Gate`를 두었다. P0 writer 스레드(pin·`Output::open`·버퍼), logger 생산자 스레드(plan·start·파일 생성·조회 뒤 스레드 시작·pin), control·safety 루프(pin·검증)가 각자 setup을 마친 뒤 같은 절대 CLOCK_MONOTONIC 시각(마지막 도착 + 50 ms)부터 계시한다. control 첫 release = 그 시각. setup 실패는 장벽을 풀어 다른 쪽이 무한 대기하지 않고, `loops`는 load 자신의 오류를 먼저 보고한다. `StorageRun.start_ns`·`ControlRun.origin_ns`를 기록하고 `load_window`가 skew·제어 구간·부하 수락 구간·내구화 tail(분리)·중첩을 계산한다. `LoadWindow::judge`는 skew 0이고 부하가 제어 구간의 99% 이상을 덮을 때만 통과, 아니면 `Rejection::NotConcurrent`. `--p0`/`--p1`은 `LoadWindow` JSON 행을 출력한다.
- **검증 근거:** 소스 `f8f2859`. `tc_012_performance::{gate_starts_every_party_after_the_slowest_setup, gate_releases_every_party_when_one_setup_fails, load_window_needs_a_shared_start_and_coverage_of_the_control_window, a_slow_setup_shifts_neither_the_load_nor_the_control_window(control 쪽 100 ms 인위 지연), a_load_that_ends_early_is_not_concurrent_with_the_control_window}`(새 API라 컴파일 Red → Green). smoke(측정 아님) `--p0`·`--p1` 3초: skew 0, 중첩 99.97%/99.94%, tail 11.5/24.7 ms 분리 보고. [EV-056](../verification_evidence.md#ev-056).

## 5. 검증 및 다음 액션

| 실행 | 결과 |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Pass, 경고 0 |
| `cargo test --workspace --all-features` (1.96.0) | Pass, 509 passed·5 ignored(기존 외부 DB 시험) |
| `cargo +1.85.0 test --workspace --all-features` | Pass, 509 passed·5 ignored |
| 저장소 밖 공개 API 재현 시험 2건 | **의도한 Red 2건**, exit101. 기존 전체 회귀의 성공과 구분 |
| 제품/저장소 테스트 수정 | 없음 |
| Miri·feature matrix 개별 조합·rustdoc 전용 명령 | 이번 회차 미실행. 현재 workspace doctest는 위 전체 시험에 포함 |
| 정식 성능·RT·native | 미실행, 기존 TASK-023/022·T3 이월 유지 |

재현 명령은 `cargo +1.96.0 test --offline --manifest-path /tmp/dot-logger-rev052-probe/Cargo.toml -- --nocapture`다. 임시 crate는 현재 dot-logger를 path dependency로 사용하고 기존 `tests/support/mod.rs`를 읽는다. 로그는 `/tmp/dot-logger-rev052-{tests,msrv-tests,clippy,probe}.log`다. 임시 파일의 영구 보존을 전제로 하지 않으며 재현 조건·결과는 위 RV 상세에 남겼다.

다음 개발 세션은 RV-128·129의 실패 시험을 저장소 테스트로 먼저 옮겨 수정·회귀 검증하고, RV-130을 정렬한 뒤 TASK-023 S2c → S2d → S3로 이어간다. 처리 판정·코드 수정·EV를 각 `[처리담당]`에 기록하고 `Open → In Progress → Resolved → Closed`를 추적한다. 기존 승인 범위의 구현/검증 위임은 유지하며 새로운 승인 요청은 없다. T1~T4 미도래다.

## 6. 조치 결과 — 2026-11-08 #001

| 항목 | 내용 |
|---|---|
| 처리 | RV-128·129 수용·조치 `97e2b12`, RV-130 수용·조치 `f8f2859`. 세 건 모두 TDD Red 확인 후 Green |
| 계약 반영 | [API §19.6](../../03_design_artifacts/interface_contract.md#196-rev-052-조치-반영--2026-11-08), §19.5 ①·④ 한정 문구 |
| 검증 | [EV-056](../verification_evidence.md#ev-056) |
| 상태 | Closed — 2026-11-08 Human 「승인」(RV-128 정정 `b2899a3` 포함) |

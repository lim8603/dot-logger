# TASK-027 — 기본 실행 계층(facade)과 기본 worker

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-008 |
| Story | US-001·002·003 |
| 요구사항 | FR-001 FR-003 FR-008 FR-012 FR-013 NFR-001 NFR-004 NFR-008 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted, ADR-007 범위 안. 소유 모델 [ADR-012](../../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md) Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-07 |
| 다음 액션 | S3c remote 선택 연결 → S3e api-consumer 갱신. S3d는 S3 제외(이월 T4) |
| 의존 | TASK-006·TASK-008·TASK-014·TASK-021 |
| 예상 소요 | 4~6 인일, 리뷰 대기 제외 |

## 목표

`Logger::builder()…start()` 하나로 기본 worker(분배 drain·writer·회전/보존·sync 주기·회수·계측 표본)를 소유해 실행하고, `'static + Send` typed channel 핸들과 `shutdown(deadline)` 보고를 제공한다. 현재 scope 빌림 API·Delivery/SyncWork/pin 의무는 facade 내부로 숨기고 기존 세부 API는 고급 경로로 유지한다.

```rust
let logger = Logger::builder()
    .preset(Preset::control(1000))                 // TASK-028
    .file_output("/data/logs", Retention::ring(64 * GIB))
    .start()?;
let pose = logger.channel::<Pose>("robot/pose")?;  // 일반 경로 등록
let _ = pose.try_log(&msg);                         // RT: 무대기·무할당, 거절은 Err
let report = logger.shutdown(Duration::from_secs(3))?;
```

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 소유 모델·공개 API·worker 구성 설계 | 소유 모델 ADR 초안(Arc 소유 `'static` 핸들 vs scope 유지+내부 thread, RT 경로 Arc clone·할당 0)·API §18·NFR-008 수치 Human 승인 | Done(2026-11-05 Human 승인) |
| S2 | 기본 worker와 파일 경로 | builder→start→raw/소형 기록→회전/sync→shutdown(정상·기한 초과·IO 실패·worker panic) 통합 시험과 RT allocator guard 회귀 Pass | 로컬 Pass(2026-11-05 [EV-047](../../05_verification/verification_evidence.md#ev-047)); worker panic 자동 시험만 GAP-009 |
| S3 | typed channel과 선택 연결 | typed encode(slot 직접 encode)·tracing 어댑터·remote 서버·계측 표본 선택 연결, `health()` 상태 조회, api-consumer 갱신 | In Progress — S3a [EV-048](../../05_verification/verification_evidence.md#ev-048)·S3b [EV-049](../../05_verification/verification_evidence.md#ev-049) 로컬 Pass, S3c·S3e 남음 |

## 변경 대상과 첫 검증

계획 경로: `crates/dot-logger/src/logger/`, `crates/dot-logger/tests/tc_013_facade.rs`, `tests/api-consumer`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **support 코드 없이 builder→start→try_log→shutdown이 최종 file/directory sync까지 완료하고, try_log 구간 할당 0, 기한 초과 shutdown은 Incomplete와 미확인 범위를 보고**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-013`의 facade 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## S1 설계 초안 — 2026-11-05

- [ADR-012](../../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md) Proposed. 옵션 F 권고: `CaptureOwner` 내부를 공유 `CaptureCore`(Arc)와 producer별 endpoint로 분해한다. `'static` `Writer`가 endpoint를 소유하고 `producer(&mut self) -> Producer<'_>` 빌림 뷰로 기존 RT 코드를 그대로 호출한다(할당·Arc clone 0). worker 측은 facade host thread 안에서 기존 scope API를 사용한다.
- Drop 불변식: Writer는 상태를 1-slot 반환 mailbox로 이동만 하고, Logger/host/reaper가 항상 최종 소유자다. S2 DoD에 allocator guard 3가지 Drop 순서 시험을 추가한다.
- 기각: core 전면 Arc(A), 제어 역전 scope(B), 프록시 링(C), `Box::leak`(D), 자기참조(E).
- 계약: [API §18](../../03_design_artifacts/interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05). 초기 스케치와 달리 `writer.try_log(channel, &msg)`(thread당 Writer, `!Sync` FIFO). NFR-008 제안: L1 ≤20줄·개념 ≤6, L2 ≤40줄·≤10, L3 연결당 +6줄, facade 공개 항목 ≤60.

## S2 실행 — 2026-11-05

- 소스 `1af172c`(capture 분해, 공개 시그니처 불변)·`64c4d23`(logger 모듈). TC-013 F1·F4~F8 + rt-allocation F2/F3 Pass, workspace 1.96/1.85 각 460 Pass. 근거 [EV-047](../../05_verification/verification_evidence.md#ev-047), 자가 점검 [REV-046](../../05_verification/reviews/REV-046_TASK-027_S2_FACADE.md) Closed.
- 구현하며 구체화한 규칙(완료 경계 Written + sync barrier, 메모리 분할, routing·파일 규칙, `LogError` variant)은 [API §18.8](../../03_design_artifacts/interface_contract.md#188-s2-구현-반영--2026-11-05).
- 잔여: worker panic 자동 시험(GAP-009), CPU 고정·`health()`(S3), 기존 MSRV clippy lint(GAP-010).

## S3 계획과 S3a 실행 — 2026-11-07

S3 범위가 넓어 하위 단계로 나눈다. 각 단계는 TDD·회귀·커밋 단위다.

| 하위 | 범위 | 상태 |
|---|---|---|
| S3a | `health()`(RV-095)·선택 계측 `telemetry()`·CPU 고정 `worker_cpus`(RV-096)·worker panic 시험(GAP-009) | 로컬 Pass — 소스 `2a805ef`, [EV-048](../../05_verification/verification_evidence.md#ev-048), [REV-047](../../05_verification/reviews/REV-047_TASK-027_S3A_HEALTH.md) Closed, API §18.9 |
| S3b | tracing 선택 연결(L3 +6줄 이하): builder 선언 → 일반 경로 subscriber/`log` sink | 로컬 Pass — 소스 `91387ba`, [EV-049](../../05_verification/verification_evidence.md#ev-049), [REV-048](../../05_verification/reviews/REV-048_TASK-027_S3B_TRACING.md) Closed, API §18.10(연결 +2줄) |
| S3c | remote 선택 연결(L3 +6줄 이하): facade가 runtime·hub·server·운영 backend를 소유 | Planned |
| S3d | slot 직접 encode | **제외(2026-11-07 Human 승인)** — 재설계 조건은 이월 T4 |
| S3e | api-consumer 갱신(facade 공개 표면 사용 확인) | Planned |

**S3d 조사 결과(결정 필요):** 현재 소형 기록은 per-writer scratch에 encode한 뒤 core가 byte ring으로 실제 길이만큼 복사한다. byte ring은 경계에서 감겨(wrap) 연속 slot을 보장하지 않는다. rtrb의 안전한 `write_chunk(n)`은 n바이트를 먼저 0으로 채우므로, 직접 encode하면 복사 대신 slot 전체 0 채우기가 들어가 작은 기록에서 오히려 비싸다. 부분 commit에는 `unsafe`(capture는 `deny`)가 필요하고, 감김 처리에는 queued record에 건너뛰기 길이를 넣는 core 형식 변경이 필요하다. 권고: S3에서 제외하고 scratch 경로를 유지하되, TASK-023 S2 P1(256B)에서 encode+복사 비용을 재고 유의하면 그때 별도 설계로 다룬다. §18.3의 "S3에서 slot 직접 encode로 교체" 문구를 바꾸므로 Human 결정 사항이다.

**2026-11-07 Human 결정:** 「승인」 — 권고대로 S3에서 제외하고 §18.3 문구를 정정했다. "TASK-023 S2 P1(256B)에서 encode·복사 비용이 의미 있게 나오면 별도 설계"는 Human 요청으로 [이월 백로그 T4](../../06_evolution/project_state.md#이월-백로그)에 기록했다.

## S1 Human 결정 — 2026-11-05

리뷰 포인트 5개 모두 권고안: ① ADR-012 옵션 F, ② 기본값 혼합(성능 값은 기본값, 손실 값은 필수), ③ `Logger::drop` 기한 있는 종료, ④ NFR-008 수치 제안대로, ⑤ `writer.try_log(channel, &msg)`. S2 DoD에 TC-013 F1~F8(특히 F2·F3 allocator guard)을 포함한다. 상세는 [API §18.7](../../03_design_artifacts/interface_contract.md#187-human-결정--2026-11-05).

## Human 확인 필요 사항(S1 시점 원문)

- 소유 모델 ADR: 기존 borrowed API 수명 계약(RSK)과 `'static` 핸들 전환의 trade-off.
- 기본 worker 수·CPU 고정·sync 주기의 기본값을 preset에 둘지 builder 필수값으로 둘지.
- `Drop` 시 동작(기한 있는 best-effort 종료 vs 경고만) — 보고를 받을 수 없으므로 명시 `shutdown` 권장 계약.

## 완료 기준

- [ ] 기본 worker·typed channel·shutdown 보고 구현, 기존 RT·예산·완료 계약 회귀 없음.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-013](../../05_verification/test_case.md#tc-013). 실행 결과: S2 [EV-047](../../05_verification/verification_evidence.md#ev-047), S3a [EV-048](../../05_verification/verification_evidence.md#ev-048), S3b [EV-049](../../05_verification/verification_evidence.md#ev-049). 성능 수치는 TASK-023 S2/S3가 이 경로로 측정한다.

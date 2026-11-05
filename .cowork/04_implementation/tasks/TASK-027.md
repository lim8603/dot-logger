# TASK-027 — 기본 실행 계층(facade)과 기본 worker

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-008 |
| Story | US-001·002·003 |
| 요구사항 | FR-001 FR-003 FR-008 FR-012 FR-013 NFR-001 NFR-004 NFR-008 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted, ADR-007 범위 안. 소유 모델 [ADR-012](../../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md) Proposed |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-05 |
| 다음 액션 | S1 Human 승인 대기 — ADR-012·[API §18](../../03_design_artifacts/interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05)·NFR-008 수치(§18.6)·확인 사항 §18.7 |
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
| S1 | 소유 모델·공개 API·worker 구성 설계 | 소유 모델 ADR 초안(Arc 소유 `'static` 핸들 vs scope 유지+내부 thread, RT 경로 Arc clone·할당 0)·API §18·NFR-008 수치 Human 승인 | 초안 완료·승인 대기(2026-11-05) |
| S2 | 기본 worker와 파일 경로 | builder→start→raw/소형 기록→회전/sync→shutdown(정상·기한 초과·IO 실패·worker panic) 통합 시험과 RT allocator guard 회귀 Pass | Planned |
| S3 | typed channel과 선택 연결 | typed encode(slot 직접 encode)·tracing 어댑터·remote 서버·계측 표본 선택 연결, `health()` 상태 조회, api-consumer 갱신 | Planned |

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

## Human 확인 필요 사항

- 소유 모델 ADR: 기존 borrowed API 수명 계약(RSK)과 `'static` 핸들 전환의 trade-off.
- 기본 worker 수·CPU 고정·sync 주기의 기본값을 preset에 둘지 builder 필수값으로 둘지.
- `Drop` 시 동작(기한 있는 best-effort 종료 vs 경고만) — 보고를 받을 수 없으므로 명시 `shutdown` 권장 계약.

## 완료 기준

- [ ] 기본 worker·typed channel·shutdown 보고 구현, 기존 RT·예산·완료 계약 회귀 없음.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-013](../../05_verification/test_case.md#tc-013). 제품 코드·테스트 실행 결과: 없음. 성능 수치는 TASK-023 S2/S3가 이 경로로 측정한다.

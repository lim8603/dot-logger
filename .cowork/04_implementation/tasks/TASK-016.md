# TASK-016 — revision 설정 변경과 전후 비교

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-005 |
| Story | US-004 |
| 요구사항 | FR-009 FR-013 FR-017 NFR-006 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-015·TASK-013 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: `crates/dot-logger/src/control/config_update.rs` · `crates/dot-logger/src/telemetry/compare.rs`. capture/completion/routing/pool/telemetry와 단일 revision writer budget을 연결했다.

첫 실패/리뷰 조건: **old plan 처리 중 재변경 Busy·부분 producer 적용·base revision 충돌**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-009`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] 필터/샘플링/기존 route 적용·기존 수락 불변·입력 조건 차이와 손실 분리 비교.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 착수 승인·기존 위임에 따른 AI 로컬 구현/개발 자가 점검 수락. 직접 Human 추가 리뷰/실측 완료를 뜻하지 않는다.

테스트 정의: [TC-009](../../05_verification/test_case.md#tc-009). 설정 변경13개·비교3개·실제 파일 writer1개·RT allocator1개 추가, EV-030/source542c6e6·source102개·검사10개·Miri496 Pass. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 현재 계약과 검증 범위

- [API §15.2](../../03_design_artifacts/interface_contract.md#152-task-016-로컬-설정-변경전후-비교--2026-11-01): 명시 enable/validate/apply·현재값 preview·posted/per-producer ack·old plan Busy·Conflict·checked revision·고정 예산.
- runtime 최소 level/Normal sampling/기존 target route. Important sampling 우회·raw 동일 loan 반환·기존 수락 route 보존. idle poll/forget 강제 회수 없음.
- 전환 표본은 RevisionTransition, 혼합 revision Window 금지. Report/Snapshot 비교는 입력·장비·boundary·scope 차이와 skipped/용량 거절/출력 손실을 분리한다.
- 단일 revision MCAP 파일은 새 명시 budget/별도 writer로 연결한다. 자동 worker/원격 ApplyConfig·Compare 운영 backend/CLI runtime·실앱/native·실제 PC/RT 비용/오차는 미완료다.
- 새 H/J 결정 없음: 승인된 두 세대/생산자 경계·기존 target 범위 구현이며 새 ADR/의존성/wire를 만들지 않았다.

## 로컬 수락·후속

[EV-030](../../05_verification/verification_evidence.md#ev-030)·[REV-027](../../05_verification/reviews/REV-027_TASK-016_CONFIG_COMPARE.md) Closed. source `542c6e65999de69ea8f682b4418de1c0fa730990`·검사10개/Miri496·source102개 commit blob 일치. native/운영 backend/실앱·실제 PC/RT 비용/오차 후속이 남아 Task는 In Progress, TC-009 전체 Partial·MS-005 In Progress·Gate4/5 Not Ready를 유지한다. 다음 후보 TASK-017은 Planned/미착수다.

2026-11-01 #004: 독립 리뷰 REV-028 RV-063/064 수용·조치·위임 수락/Closed. 실제 poll API `Producer::apply_pending_config()`의 유휴 worker 책임·old 자원 배출 조건, 전환 표본을 비교용 Window로 만들지 않는 호스트 절차를 API/rustdoc에 명시했다. 안정 before→전환/IncompleteWindow→안정 after→Report/Comparison과 인접/겹침 경계를 검증했다. [EV-031](../../05_verification/verification_evidence.md#ev-031)·소스5d447b4. TASK-017 S1을 후속 착수했으며 기존 운영/실측/native 잔여는 유지한다.

## 운영 backend — 2026-11-03 #001

Human 결정(2026-11-03)으로 probe 연동 전에 원격 운영 backend를 구현했다. `telemetry::operations::OperationsHost`가 host 일반 worker에서 GetStatus(`dot_logger.status.v1` 지표)·ApplyConfig(`dot_logger.config_patch.v1`)·Compare(host 등록 snapshot, `dot_logger.compare.v1`)를 처리한다. 게시 없는 세 거절은 새 `RemoteError::{Conflict, Busy, RestartRequired}`로 구분하고 CLI에 각각 exit3/3/2로 노출한다. Status 지표는 기존 `Reply.data`에 싣고 proto는 바꾸지 않았다. 1초 sampling·snapshot 생성·producer poll은 host 책임이다. [API§15.8](../../03_design_artifacts/interface_contract.md#158-운영-backendstatusapplycompare--2026-11-03)·[EV-044](../../05_verification/verification_evidence.md#ev-044)·[REV-044](../../05_verification/reviews/REV-044_TASK-016_OPERATIONS_BACKEND.md) Closed, source `a738ebf`. 실제 dot-probe 화면·PC/RT 비용/오차·native가 남아 Task는 In Progress, TC-009 Partial을 유지한다.

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- native·다른 OS: [TASK-024](TASK-024.md)(T3, GAP-004)
- 이 PC 정량 성능·RT: [TASK-023](TASK-023.md)에서 수행한 범위(EV-064~070)
- 실사용처 적용: [TASK-026](TASK-026.md)(마지막 단계, 일정 판단 밖)

# TASK-020 — tracing 어댑터와 dot-probe 통합 예제

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-006 |
| Story | US-001·002·003·004 |
| 요구사항 | FR-001 FR-002 FR-004 FR-006 FR-017 NFR-004 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-01 |
| 다음 액션 | S1 EV-040 로컬 수락, S2 통합 비교 예제 진행 |
| 의존 | TASK-013·TASK-016·TASK-019 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

S1 구현: `crates/dot-logger/src/adapters/tracing.rs`, `examples/general_logs.rs`, `crates/dot-logger/tests/tc_007_tracing.rs`. S2 dot-probe 통합 비교 예제는 아직 미구현이다. 기존 client의 단일 snapshot 예제와 구분한다.

첫 실패/리뷰 조건: **일반 tracing을 RT 경로로 사용하지 않음·화면 client 데이터 흐름**.

- [x] S1 API 부재 compile Red 확인(EV-040).
- [x] S1 TC-007 하위 신규8개 통과. S2/S3와 TC 전체는 별도 잔여.
- [x] S1 두 Rust workspace/16features·consumer/RT/lint/doc/fmt·예제 확인(EV-040).

## 완료 기준

- [ ] 대표 로그/수치/합성 원본→파일/DB/구독/조절 비교. 실제 dot-probe 수정은 별도 통합 작업에서 범위 확인.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-007](../../05_verification/test_case.md#tc-007). S1 제품 코드·검증 근거: [EV-040](../../05_verification/verification_evidence.md#ev-040). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## TASK-013 REV-022에서 넘긴 화면 수락 조건 — 2026-10-31

원격 구독의 ttl은 서버 idle lease를 겸하고 heartbeat가 없다. 로컬 next wait timeout은 같은 구독에서 재대기할 수 있지만 서버 lease/연결 종료 뒤 새 구독 사이에는 기록 복원/Gap 연속성을 보장하지 않는다. 화면은 종료·새 구독 경계를 불연속으로 표시해야 한다. SDK/EV-026의 제한을 소비하고 장기 tail·연속성 수락을 따로 검증한다. heartbeat/lease wire 확장은 필요성 판단 및 H/J 결정 후 진행한다.

## 단계별 실행 — 2026-11-01 #014

Human 「다음 작업 시작하자」로 REV-038 조치 후 착수했다. 선행 TASK-013/016/019 로컬 수락과 native/실측 이월 상태의 후속 개발 위임을 사용한다.

| 단계 | 업무 | 상태/남은 조건 |
|---|---|---|
| S1 | 일반 경로 tracing/log 이벤트 유한 인코딩·수집 연결 | 로컬 수락 — EV-040/REV-039 Closed |
| S2 | 대표 로그/수치/합성 원본의 파일·DB·구독/조절 비교 예제 | 미착수 |
| S3 | 구독 종료/재연결 불연속 표시 예제·검증 | 미착수 — RV-053 소비 |
| 통합/환경 수락 | 지속 dot-probe/화면·PC/RT/native | 대기 — TASK-026/022·T3 |

TASK-018은 별도 FR-015 범위 승인 대기로 미착수다. S1 수락은 Task 전체 완료가 아니다.

## S1 로컬 수락

- API §14.9의 caller scratch·UTF-8 JSON·모든 값은 text·own fields 한정·명시 capture 계약을 구현했다. native severity만 변환하며 시각/schema/중요도·receipt 의미를 유지한다. Buffer overflow/formatter 오류는 전체 거절한다.
- `tracing_event`/`log_record`는 기존 application subscriber/sink에서 호출하는 훅이며 global Subscriber/Log 구현을 설치하지 않는다. 사용자 formatter에 RT/시간 보장을 적용하지 않는다. 새 worker/heap 큐·의존 버전/lock 변경 없음.
- 기능 부재 compile Red→신규8개 Green. Rust1.85의 예제/시험 format_args E0716은 RV-081에서 동일 호출 소비로 수정했다. 두 Rust workspace각409개/실제 DB4개 ignore, 각각16features/독립 consumer/RT allocator·fmt/clippy/doc/예제 Pass. EV-040·REV-039/RV-081 Closed, 기존 위임으로 S1만 수락했다.
- 다음은 S2 대표 로그/수치/합성 원본→파일·DB·구독/조절 비교 예제다. S3 재연결 불연속·TASK-026 실앱·PC/RT/native(T3)·Task/MS/TC 전체·Gate4/5는 미완료다.

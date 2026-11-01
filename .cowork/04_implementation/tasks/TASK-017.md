# TASK-017 — 예산 제한 간편 점검

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-005 |
| Story | US-004 |
| 요구사항 | FR-016 NFR-005 NFR-006 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-01 |
| 다음 액션 | S2 — 명시 run·전용 경로 배타 소유·실제 IO/배출·sync/정리·결과 분류 TDD |
| 의존 | TASK-015·TASK-013 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

S1 구현 경로: `crates/dot-logger/src/diagnostics/mod.rs` · `tests/tc_010_diagnostics.rs`. CLI/실제 실행 경로는 S2 이후 미구현이다.

첫 실패/리뷰 조건: **dry-run 무부하·쓰기/시간 예산 선도달·수신자 없음·취소**.

- [x] S1의 무부하 계획·예산 판정을 API 부재 compile Red → 최소 구현 Green으로 확인했다.
- [x] S1 하위5개 통과. 실제 IO/실행/정리를 포함한 TC-010 전체는 Partial이다.
- [x] S1 workspace/all-features·MSRV1.85 관련22개·fmt/clippy/rustdoc와 API 문서를 확인했다.

## 완료 기준

- [ ] 계획→명시 실행→해석, 전용 경로 정리·운영 기록 보존·Inconclusive 구분.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-010](../../05_verification/test_case.md#tc-010). S1 코드/검증: [EV-031](../../05_verification/verification_evidence.md#ev-031)·[REV-029](../../05_verification/reviews/REV-029_TASK-017_PLAN_START.md), 소스0622a90. S1만 기존 위임으로 로컬 수락했으며 Task 전체 완료/실제 성능 인증은 아니다. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 작업 단계와 현재 계약 — 2026-11-01 #004

- [x] S1 — 무부하 `CheckPlan::build`·원본 provenance/명시 예산 보존·메모리 선언/경로 문법 검증·`write_allowance` 순수 계산. 신규5개 Red→Green, [API §15.3](../../03_design_artifacts/interface_contract.md#153-task-017-s1-로컬-계획예산-판정--2026-11-01).
- [ ] S2 — plan 뒤 별도 명시 run, 배타적으로 소유한 전용 경로만 생성/정리, bounded workload·실제 physical write/reserve·운영 기록 불변, budget/cancel/IO 실패 뒤 drain/sync/close·최종 범위/원인·Pass/Fail/Aborted/Inconclusive.
- [ ] S3 — 실제 Report/측정 provenance·CLI/runtime/원격 backend 연결 및 실제 수신자·실앱/PC/RT/native 검증. 동일 scope에 필요한 구현 단위로 진행하고 FR-015/TASK-018은 별도 승인 전 착수하지 않는다.

S1의 `required_memory_bytes`는 host의 예약 계산 선언이지 실제 예약/측정 증명이 아니다. 예산 판정은 host 관측의 stateless 계산이며 reservation/IO 취소를 수행하지 않는다. final drain/index/footer/outstanding bytes까지 미리 예약하고 신규 IO를 직렬화하는 host 책임을 API에 명시했다. 네트워크는 미측정/loopback/remote 요청 선언만 구분한다. S2에서 실제 종료·정리와 함께 검증해야 한다.

TASK-013/015 로컬 수락을 선행 근거로 착수했다. 운영 backend/실앱/실측/native 잔여를 완료로 간주하지 않고 순수 로컬 S1에 한정했다. 새 H/J 결정·새 dependency/unsafe/wire/공유 문서 구조 변경 없음. Task/MS/TC 전체·Gate4/5 미완료 유지.

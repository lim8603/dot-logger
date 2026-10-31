# TASK-014 — 운영 계측과 제한된 집계

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-005 |
| Story | US-004 |
| 요구사항 | FR-014 NFR-006 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-31 |
| 다음 액션 | EV-029/sourceeaf736c·REV-025 Closed; PC/RT 비용·오차/native·실앱 후속, 다음 후보 TASK-016 |
| 의존 | TASK-005 |
| 예상 소요 | 1~2 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

실제 경로: `crates/dot-logger/src/telemetry/{mod,counters,aggregate}.rs`, capture/mod.rs·completion/mod.rs, tests/tc_009_telemetry.rs 및 examples/telemetry_overhead.rs.

첫 실패/리뷰 조건: **수락/거절/생략/대상별 결과 합계·counter reset·누락 지표**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-009`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] 고정 cardinality/이력·생산자 계측 atomic 접근·기본 overhead 계측 fixture.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 착수 승인·기존 수락/마감 위임에 따른 AI 로컬 검증 수락(REV-023 Closed). 실제 비용/RT/native 시험의 수락은 별도다.

테스트 정의: [TC-009](../../05_verification/test_case.md#tc-009). 제품 코드·테스트 실행 결과: [EV-027](../../05_verification/verification_evidence.md#ev-027), 소스d99ba3b. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 로컬 구현·위임 수락 — 2026-10-31 #011

Human 「권고대로 진행하자」로 REV-022 EV-026 수락 뒤 착수했다. enable 시 현재 capture/completion 예약과 합산한 control 예산을 먼저 검사하고 producer별 고정 counter+301개 이력을 예약한다. RT는 제출 결과와 별도 raw acquisition 거절을 single-writer relaxed atomic으로만 갱신한다. 관리 sample은 session/revision/epoch/실제 시각 및 전체/필수 결과를 보존하며 10/60/300초 유한 구간 차·reset/missing/overflow/Stale를 제공한다. reset은 독점 owner에서만 수행하고 receipt/completion 결과는 바꾸지 않는다. 대상별 totals는 기존 live/retired 권위를 재사용한다. 동시값은 근접 시점의 혼합이며 배출 후 수량을 정확 대조한다.

EV-027 source97개/검사14개·두 Rust 각16feature/consumer·TC-009 6개·추가 allocator2개·영향 Miri56·commit blob일치로 로컬 구현을 수락했다. 128회 debug fixture는 on/off·순서 반전 경로 검증이며 완료 처리량 비용≤5%·실제 RT/PC/OS/native·실앱 Pass를 뜻하지 않는다. TASK-022/023에 실측을 연결하고 Task는 In Progress를 유지한다. CLI runtime backend는 여전히 Unavailable이며 자동 worker/용량 분모/예측/설정·비교는 후속이다. 다음 개발 후보 TASK-015는 이번 실행에서 자동 착수하지 않는다.

## REV-025 조치·위임 수락 — 2026-10-31 #014

[EV-029](../../05_verification/verification_evidence.md#ev-029) sourceeaf736c·source101개/검사14개·Miri224·commit blob일치, [REV-025](../../05_verification/reviews/REV-025_TASK-014-015_INDEPENDENT_REVIEW.md) Closed. 제출/대여 거절 총계와 capacity/contract/lifecycle 6개 범주를 보존하고 용량 거절만 압력·부하율에 사용한다. 시도율은 재시도 중복을 포함하므로 고유 입력/손실 records로 비교하지 않는다. target_totals 호스트 유한 이력을 실제 window 양끝에 맞추는 절차·실제 completion 연결 시험, poison reset 복구·history_len Observation 반환 migration을 반영했다. CounterCell128B·RT 할당/해제0·추가 이력 크기의 checked control 예약을 확인했다.

Human 「권고대로 진행」과 기존 위임으로 API 정정·로컬 수락/마감을 수행했다. 기존 계약/검증의 이력은 보존하며 실제 비용≤5%·PC/RT 예측 오차≤20%·native 공개 후T3·실앱/운영 backend는 여전히 미완료다. TASK-016 Planned/미착수, Task/MS/TC 전체·Gate4/5 미완료를 유지한다.

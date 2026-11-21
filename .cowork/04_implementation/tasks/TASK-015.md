# TASK-015 — 여유·병목·보존 시간 리포트

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-005 |
| Story | US-004 |
| 요구사항 | FR-014 NFR-005 NFR-006 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-014·TASK-008 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현: [`telemetry/report.rs`](../../../crates/dot-logger/src/telemetry/report.rs)·[`capacity.rs`](../../../crates/dot-logger/src/telemetry/capacity.rs). API§15.1·공개 consumer·TC-009 연결을 따른다.

첫 실패/리뷰 조건: **폐기로 안정된 큐 Overloaded·미측정 분모·음수/0 증가율**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-009`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] Observed/Validated/Estimated/Stale·buffer/disk/retention 계산·오차 시나리오.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] 기존 Human 위임에 따른 AI 로컬 구현/개발 자가 점검 수락. 새 H/J 결정 없음. 직접 Human 추가 리뷰/실측 완료를 뜻하지 않는다.

테스트 정의: [TC-009](../../05_verification/test_case.md#tc-009). 제품 코드·테스트 실행 결과: [EV-028](../../05_verification/verification_evidence.md#ev-028)/소스95a63a6, [REV-024](../../05_verification/reviews/REV-024_TASK-015_REPORT.md) Closed. 각 체크는 로컬 하위 범위이며 실측/native·실앱은 미완료다. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 다음 세션 착수 승인·인계 — 2026-10-31 #011 종료

Human 「세션 마무리해줘. 다음 세션에서 TASK-015 시작하자」. 당시 Planned/미착수였으며 #011 종료 처리에서 제품 코드/시험을 만들지 않았다. #012의 현재 작업은 아래 구현 결과를 따른다. 당시 인계는 작업 재선택 없이 본 Task부터 TDD로 시작하는 것이었다.

- 선행 로컬 근거: TASK-008 EV-013의 승인된 회전/보존, TASK-014 EV-027(source 3164f0d·검사14개/Miri56)의 고정 계측/이력 수락. 선행 Task의 native/실측 이월 상태에서 진행하는 지시이며 검증 면제가 아니다.
- 로딩: project_state의 현재 제약 → 본 상세/Task 목록 → API§15·TC-009 및 필요한 telemetry/retention 코드. 계측 표본의 session/revision/epoch·동시 근접 시점 혼합/배출 후 정확·301개 이력·missing/reset/포화/Stale 의미를 유지한다.
- 첫 실패 조건: 폐기로 큐가 안정돼도 Overloaded, 검증된 완료 능력 분모가 없으면 부하율 Unavailable, 0/음수 순증가율에서 허위 포화/소진 시간을 만들지 않음. buffer bytes/slots·순환 보존 창·disk 소진을 구분한다.
- 미실행 비용≤5%·실제 RT/PC·native 공개 후(T3)·실앱·Gate4/5 조건 유지. 이번 기록은 제품 구현/새 H/J 설계 확정/실측 완료가 아니다.

## 로컬 구현·검증·위임 수락 — 2026-10-31 #012

- EV-028 source100개/검사13개·Miri168·소스95a63a6, 결과·환경·명령·hash(비공개 원시 근거). 두 Rust 각16feature/workspace/core-client·clippy/rustdoc -D warnings·fmt·공개 consumer·기존 RT allocator 회귀 Pass.
- `ReportInput::from_window`는 실제 window의 session/revision/epoch/구간과 global producer/channel 범위를 보존한다. host 공급 output/physical buffer/disk 관측은 같은 구간·조건이어야 하며 기본은 MissingMeasurement다. Rate/Ratio 정수 유리수·Value 근거/만료·검증 profile 일치/유효성·분리된 limit/retention을 구현했다.
- 리포트 TC-009 14개, 기존 계측7개(실제 window→report 추가1), Miri8seeds×21=168. 할당 guard 추가1은 관리 build1024회 heap 할당0/해제0이다. 안정 합성 12조합의 step simulation으로 5초 이상 예측의 오차≤20%를 확인했으며 실제 PC/RT IO 수락은 아니다.
- RV-056~059의 profile session/독립 quota/BeyondRange/Estimated 과부하 provenance는 assertion Red→Green으로 조치했다. 초기 MSRV lint 실패 원본은 before-lint-fix에 보존하고 공통 표현으로 정리한 최종 소스를 재검증했다.
- 취소/종료: 순수 계산 API로 새 IO/worker/자원 수명 없음. 기존 producer/reader 종료 계약을 바꾸지 않는다. 누락/포화·0분모/0구간·역방향/낡은 시각·만료·음수/0 증가율·full 경계를 검증했다.
- 미완료: 실제 capacity 검증·호스트 물리 gauge/지연 수집/자동 worker·CLI runtime backend·실앱, PC/RT 실제 예측 오차≤20%/계측 비용≤5%, 다른 OS/native 공개 후(T3). Task In Progress·MS-005/TC-009 전체·Gate4/5 미완료 유지.

## REV-025 조치·위임 수락 — 2026-10-31 #014

[EV-029](../../05_verification/verification_evidence.md#ev-029) sourceeaf736c·source101개/검사14개·Miri224·commit blob일치, [REV-025](../../05_verification/reviews/REV-025_TASK-014-015_INDEPENDENT_REVIEW.md) Closed. 제출/대여 거절 총계와 capacity/contract/lifecycle 6개 범주를 보존하고 용량 거절만 압력·부하율에 사용한다. 시도율은 재시도 중복을 포함하므로 고유 입력/손실 records로 비교하지 않는다. target_totals 호스트 유한 이력을 실제 window 양끝에 맞추는 절차·실제 completion 연결 시험, poison reset 복구·history_len Observation 반환 migration을 반영했다. CounterCell128B·RT 할당/해제0·추가 이력 크기의 checked control 예약을 확인했다.

Human 「권고대로 진행」과 기존 위임으로 API 정정·로컬 수락/마감을 수행했다. 기존 계약/검증의 이력은 보존하며 실제 비용≤5%·PC/RT 예측 오차≤20%·native 공개 후T3·실앱/운영 backend는 여전히 미완료다. TASK-016 Planned/미착수, Task/MS/TC 전체·Gate4/5 미완료를 유지한다.

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- native·다른 OS: [TASK-024](TASK-024.md)(T3, GAP-004)
- 이 PC 정량 성능·RT: [TASK-023](TASK-023.md)에서 수행한 범위(EV-064~070)
- 실사용처 적용: [TASK-026](TASK-026.md)(마지막 단계, 일정 판단 밖)

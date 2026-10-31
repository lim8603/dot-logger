# TASK-015 — 여유·병목·보존 시간 리포트

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-005 |
| Story | US-004 |
| 요구사항 | FR-014 NFR-005 NFR-006 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-10-31 |
| 다음 액션 | 다음 세션 착수 승인 — 폐기로 안정된 큐 Overloaded·미측정 분모·음수/0 증가율 Red부터 |
| 의존 | TASK-014·TASK-008 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `crates/dot-logger/src/telemetry/{report,capacity}.rs`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **폐기로 안정된 큐 Overloaded·미측정 분모·음수/0 증가율**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-009`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] Observed/Validated/Estimated/Stale·buffer/disk/retention 계산·오차 시나리오.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-009](../../05_verification/test_case.md#tc-009). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 다음 세션 착수 승인·인계 — 2026-10-31 #011 종료

Human 「세션 마무리해줘. 다음 세션에서 TASK-015 시작하자」. 현재 Planned/미착수이며 이번 종료 처리에서 제품 코드/시험을 만들지 않았다. 다음 세션은 작업 재선택 없이 본 Task부터 TDD로 시작한다.

- 선행 로컬 근거: TASK-008 EV-013의 승인된 회전/보존, TASK-014 EV-027(source d99ba3b·검사14개/Miri56)의 고정 계측/이력 수락. 선행 Task의 native/실측 이월 상태에서 진행하는 지시이며 검증 면제가 아니다.
- 로딩: project_state의 현재 제약 → 본 상세/Task 목록 → API§15·TC-009 및 필요한 telemetry/retention 코드. 계측 표본의 session/revision/epoch·동시 근접 시점 혼합/배출 후 정확·301개 이력·missing/reset/포화/Stale 의미를 유지한다.
- 첫 실패 조건: 폐기로 큐가 안정돼도 Overloaded, 검증된 완료 능력 분모가 없으면 부하율 Unavailable, 0/음수 순증가율에서 허위 포화/소진 시간을 만들지 않음. buffer bytes/slots·순환 보존 창·disk 소진을 구분한다.
- 미실행 비용≤5%·실제 RT/PC·native 공개 후(T3)·실앱·Gate4/5 조건 유지. 이번 기록은 제품 구현/새 H/J 설계 확정/실측 완료가 아니다.

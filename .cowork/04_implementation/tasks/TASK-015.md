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
| 마지막 갱신일 | 2026-10-26 |
| 다음 액션 | 폐기로 안정된 큐 Overloaded·미측정 분모·음수/0 증가율 |
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

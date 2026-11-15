# TASK-024 — 24시간·전원 차단·OS 교환 검증

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-001·002·003·004·005 |
| 요구사항 | FR-018 NFR-003 NFR-007 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-15 |
| 다음 액션 | 마지막 단계(일정 판단 밖, 2026-11-15 Human 결정). 현재 PC P6 장애/24h·UPS 모의 차단/복구 oracle 준비. native·다른 OS 행(GAP-004)은 T3 공개 후 — 기능 Task의 native 잔여를 이 Task로 통합(2026-11-13) |
| 의존 | TASK-023 |
| 예상 소요 | 3~5 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `tests/soak/; tests/platform/; 검증 evidence`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **P6 각 장애의 기대 손실/복구 기준과 UPS 시뮬레이션 oracle 준비**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-012`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] 현재 PC 24h 결과·UPS 모의 차단/프로세스 중단·재개/복구를 시뮬레이션 수락. native/3 OS 생성·읽기 교환은 공개 완료 후 실행하고 미실행 GAP 유지(ADR-011).
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-012](../../05_verification/test_case.md#tc-012). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 2026-10-31 수락 방식·검증 시점 갱신

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체하고 UPS는 시뮬레이션으로 판정한다. native는 GitHub 공개 완료 후 진행한다. 일반 Linux/RT는 실제 실행 커널을 기록하며 물리 전원 소실 내구성의 관측 결과로 표시하지 않는다. 기존 수치/예산/24h/소프트웨어 통합 목표와 선행 의존은 유지하며 이번 문서 갱신만으로 제품 시험을 Pass 처리하지 않는다.

## native·다른 OS 잔여 통합 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)으로 기능 Task(TASK-004~017·019~023·027~029)를 Done으로 닫으면서, 각 Task에 흩어져 있던 native·다른 OS 잔여를 이 Task가 소유한다. 트리거는 그대로 T3(GitHub 공개 완료 확인)이다.

- GitHub CI 6 contracts·audit·raw-memory·`integration-surface` job을 공개 시점 source hash로 실행·확인(GAP-004).
- Windows/macOS 등 Tier 2 대상의 생성·읽기 교환(기존 범위).
- 공개 전에는 CI 재시도나 계정·과금 조회를 하지 않는다(기존 결정 유지).

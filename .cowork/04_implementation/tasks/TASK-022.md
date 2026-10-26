# TASK-022 — 실제 장비·플랫폼 검증 프로필

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-001·002·003·004·005 |
| 요구사항 | NFR-001 NFR-002 NFR-003 NFR-007 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-10-26 |
| 다음 액션 | 장비/OS/UPS 프로필을 Human과 확정 |
| 의존 | TASK-001 |
| 예상 소요 | 1~2 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `test-profiles/; .cowork/05_verification/test_strategy.md`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **문서 검토: 장비/OS/CPU/SSD/FS/NIC/UPS·제어 주기·deadline 누락 탐지**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-012`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] Human 장비 확정·P0 도구/프로필·OS 네이티브 환경 확보·최종 수락 조건 승인.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-012](../../05_verification/test_case.md#tc-012). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

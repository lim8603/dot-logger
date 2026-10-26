# TASK-012 — gRPC 구독·ACK·관리 서버

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-004 |
| Story | US-003·005 |
| 요구사항 | FR-005 FR-006 FR-013 NFR-002 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-10-26 |
| 다음 액션 | 느린 구독자·잘못된 조각·ACK 전 disconnect·인증 없는 비-loopback |
| 의존 | TASK-010·TASK-006 |
| 예상 소요 | 2~4 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `proto/logger.proto; crates/dot-logger/src/adapters/remote/`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **느린 구독자·잘못된 조각·ACK 전 disconnect·인증 없는 비-loopback**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-007`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] 제한 reassembly·Gap/Unknown·버전/권한·취소·receiver ACK, 로컬 파일 경로 영향 없음.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-007](../../05_verification/test_case.md#tc-007). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

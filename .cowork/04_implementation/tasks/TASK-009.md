# TASK-009 — 중단 복구와 I/O 장애

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-003 |
| Story | US-002·005 |
| 요구사항 | FR-003 NFR-001 NFR-003 NFR-007 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-30 |
| 다음 액션 | chunk 중간 절단·CRC 손상·ENOSPC·sync/rename 실패 |
| 의존 | TASK-008 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `crates/dot-logger/src/storage/recovery.rs; tests/fault_storage.rs`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **chunk 중간 절단·CRC 손상·ENOSPC·sync/rename 실패**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-005`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] 원본 보존 복구·온전한 prefix/제외 범위 보고·RT 무대기 유지·OS별 장애 의미.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-005](../../05_verification/test_case.md#tc-005). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 착수 — 2026-10-30

Human 「커밋 푸쉬 후, 다음 테스크 진행」에 따라 TASK-008 EV-013 소스/문서 게시 후 착수한다. TASK-008 최종 구현 리뷰·native GAP-004는 남겨두며 후속 개발 진행을 승인받았다. 승인된 별도 복구 파일·원본 무수정·완전 검증 chunk prefix·손상/제외 범위·IO 오류 보존 계약을 TDD로 구현한다.

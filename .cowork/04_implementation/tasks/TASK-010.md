# TASK-010 — 시간·채널 조회와 active snapshot

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-004 |
| Story | US-003·005 |
| 요구사항 | FR-005 NFR-002 NFR-005 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-10-30 |
| 다음 액션 | 쓰는 중 incomplete chunk 비노출·cursor 만료·페이지 bytes/count 이중 제한 |
| 의존 | TASK-009 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `crates/dot-logger/src/query/`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **쓰는 중 incomplete chunk 비노출·cursor 만료·페이지 bytes/count 이중 제한**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-006`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] mono/real/source domain·다중 segment·원본 descriptor/읽기 핀·잘못된 파일 bounds.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] REV-010 RV-012: 오류/취소/리더 중단 경로에서도 `ReadPin`을 보관하고, 실제 읽기 종료 확인 후 반환하는 수명 관리를 검증한다. 고립 회수 조건은 기한만으로 삭제를 허용하지 않는 기존 계약 안에서 검토한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-006](../../05_verification/test_case.md#tc-006). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 리뷰 후속 — REV-010 RV-012 (2026-10-30)

TASK-008의 저수준 `ReadPin`은 만료/Drop으로 삭제 보호를 해제하지 않는다. 토큰을 유실하면 catalog 생존 기간 동안 pin slot과 파일 보호가 남아 `PinLimit`/`Capacity`로 이어질 수 있다. RV-012는 현 계약 유지·`must_use`/문서/회귀 보강으로 처리했으며, 위험 자체를 제거한 것으로 보지 않는다. 본 Task에서 리더가 토큰을 소유하고 오류/취소에도 실제 IO 종료를 확인한 뒤 반환하는 경로를 구현한다. 비정상 리더의 종료 확인·안전한 회수 조건을 함께 검토하고, 공개 소유권/오류 의미를 바꾸는 정책이 필요하면 별도 H/J 결정으로 제시한다. 기한 경과만으로 강제 해제하거나 진행 중인 읽기를 종료한 것으로 간주하지 않는다. 아직 조회/리더 구현에 착수한 것은 아니다. 근거: [REV-010](../../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md), [인터페이스 §13.4](../../03_design_artifacts/interface_contract.md#134-task-008-회전보존-api--2026-10-30).

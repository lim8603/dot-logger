# TASK-029 — 연결 레시피·문서·접점 회귀 가드

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-008 |
| Story | US-001·003 |
| 요구사항 | FR-001 NFR-004 NFR-008 |
| 설계 | [인터페이스](../../03_design_artifacts/interface_contract.md) · [CLI](../../03_design_artifacts/ui_spec.md) |
| ADR | ADR-007 Accepted |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-04 |
| 다음 액션 | TASK-027·028 S1 뒤 레시피 3단계·quickstart·공개 API 회귀 가드 |
| 의존 | TASK-027·TASK-028 |
| 예상 소요 | 1~2 인일, 리뷰 대기 제외 |

## 목표

소비처가 support 모듈 없이 따라 할 수 있는 단계별 레시피와 짧은 quickstart를 제공하고, 접점 크기를 수락 기준으로 측정·회귀 감시한다.

- **L1 일반 로그:** tracing/log만 연결(최소 줄 수).
- **L2 typed facade:** preset→start→typed channel→shutdown.
- **L3 RT 원본:** raw loan·중요도·이벤트 보호·완료 확인.
- README는 quickstart와 레시피 링크 중심으로 줄이고, 계약 상세는 모듈 rustdoc으로 옮긴다.
- 기존 `dot_probe_*` 예제를 facade 경로로 옮기고 support 줄 수 감소를 기록한다.
- 공개 API 변경 감지: api-consumer 컴파일 검사 확장 또는 rustdoc JSON snapshot(도구 추가 시 tech_stack 확인).

## 변경 대상과 첫 검증

계획 경로: `examples/recipes/`, `README.md`, `tests/api-consumer`, `crates/dot-logger-client/examples`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **레시피 예제가 support 모듈 없이 빌드·실행되고, 연동 코드 줄 수·도입 개념 수가 NFR-008 승인 수치를 넘으면 검사 실패**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-013`의 레시피 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] 레시피 3단계·quickstart·rustdoc 이동·예제 이전과 접점 측정 결과.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 레시피의 실패/포화·종료 처리 예시가 실제 계약과 일치한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-013](../../05_verification/test_case.md#tc-013). 제품 코드·테스트 실행 결과: 없음.

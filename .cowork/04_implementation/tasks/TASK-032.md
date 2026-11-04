# TASK-032 — viewer 운영 대시보드

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-004 |
| 요구사항 | FR-006 FR-014 FR-016 FR-017 NFR-006 NFR-008 |
| 설계 | [인터페이스 §15](../../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) · [CLI/화면](../../03_design_artifacts/ui_spec.md) |
| ADR | ADR-007·009 Accepted. UI 기술 결정(S1) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-04 |
| 다음 액션 | S1 — UI 방식(viewer 내장 웹·native·Foxglove 확장 패널) 결정 |
| 의존 | TASK-031·TASK-016·TASK-017 |
| 예상 소요 | 2~4 인일, 리뷰 대기 제외 |

## 목표

Foxglove 계열 도구가 보여주지 않는 dot-logger 고유 운영 관점을 viewer에서 제공한다: 여유·병목·보존 시간 리포트, 대상별 완료/손실/Unknown, 설정 변경 전후 비교, 간편 점검 결과.

- 기본은 읽기 전용. ApplyConfig 같은 변경 동작은 원격 관리 권한과 명시 확인이 있을 때만(포함 여부는 S1 결정).
- Observed/Validated/Estimated/Stale/Unavailable 구분을 화면에서 보존하고 미측정 값을 0이나 추정으로 대체하지 않는다.
- dot-probe 기존 화면(TASK-026)은 별도 유지한다. 이 대시보드는 dot-probe 외 일반 소비처용이다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | UI 방식·화면 범위 결정 | 권고안(viewer 내장 웹: 단일 바이너리·브라우저·3 OS)과 대안 비교, ui_spec 반영안 Human 승인 | Planned |
| S2 | 상태·리포트·완료 화면 | Status/Report/Completion 표시, 값 구분·갱신 실패 표시 | Planned |
| S3 | 전후 비교·점검 화면 | Compare·PlanCheck/RunCheck 결과, (승인 시) 권한 있는 설정 적용 흐름 | Planned |

## 변경 대상과 첫 검증

계획 경로: `crates/dot-logger-viewer/`(대시보드 모듈). 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **Unavailable/Stale 지표를 0·정상으로 표시하지 않고, 원격 연결 끊김·권한 없음에서 마지막 값과 시각·사유를 구분 표시**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-014`의 대시보드 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] 운영 화면 구현과 값 구분·권한 경계 검증.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/연결 끊김·취소/종료를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-014](../../05_verification/test_case.md#tc-014). 제품 코드·테스트 실행 결과: 없음.

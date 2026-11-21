# TASK-032 — viewer 운영 대시보드

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-004 |
| 요구사항 | FR-006 FR-014 FR-016 FR-017 NFR-006 NFR-008 |
| 설계 | [인터페이스 §15](../../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) · [CLI/화면](../../03_design_artifacts/ui_spec.md) |
| ADR | ADR-007·009·014 Accepted. [ADR-016](../../03_design_artifacts/adrs/ADR-016_viewer_embedded_web_dashboard.md) Accepted(S1, 2026-11-21) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-21 |
| 다음 액션 | S1~S3 구현 완료. Task 수락 전 리뷰(완료 기준의 Human 리뷰) |
| 의존 | TASK-031·TASK-016·TASK-017 |
| 예상 소요 | 2~4 인일, 리뷰 대기 제외 |

## 목표

Foxglove 계열 도구가 보여주지 않는 dot-logger 고유 운영 관점을 viewer에서 제공한다: 여유·병목·보존 시간 리포트, 대상별 완료/손실/Unknown, 설정 변경 전후 비교, 간편 점검 결과.

- 읽기 전용(S1 결정, ADR-016). ApplyConfig·RunCheck는 대시보드에 넣지 않고 CLI를 사용한다.
- Observed/Validated/Estimated/Stale/Unavailable 구분을 화면에서 보존하고 미측정 값을 0이나 추정으로 대체하지 않는다.
- dot-probe 기존 화면(TASK-026)은 별도 유지한다. 이 대시보드는 dot-probe 외 일반 소비처용이다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | UI 방식·화면 범위 결정 | 권고안(viewer 내장 웹: 단일 바이너리·브라우저·3 OS)과 대안 비교, ui_spec 반영안 Human 승인 | Done — 2026-11-21 ADR-016·[화면 §7](../../03_design_artifacts/ui_spec.md#7-task-032-운영-대시보드--2026-11-21) |
| S2 | 상태·리포트·완료 화면 | Status/Report/Completion 표시, 값 구분·갱신 실패 표시 | Done — 2026-11-21 `704d055`, [EV-107](../../05_verification/verification_evidence.md#ev-107) |
| S3 | 전후 비교·점검 화면 | Compare 결과·PlanCheck 미리보기, Gone/Unsupported 기능 미제공 표시(변경 동작 없음) | Done — 2026-11-21 `277de2d`, [EV-108](../../05_verification/verification_evidence.md#ev-108) |

## 변경 대상과 첫 검증

코드 경로: [dashboard.rs](../../../crates/dot-logger-viewer/src/dashboard.rs), [자산](../../../crates/dot-logger-viewer/assets/dashboard/), [시험](../../../crates/dot-logger-viewer/src/dashboard_tests.rs). 새 crate 없이 이미 잠긴 `httparse`를 쓴다(ADR-016 구현 기록).

첫 실패/리뷰 조건: **Unavailable/Stale 지표를 0·정상으로 표시하지 않고, 원격 연결 끊김·권한 없음에서 마지막 값과 시각·사유를 구분 표시**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다(S2, EV-107).
- [x] 최소 구현 후 관련 `TC-014`의 대시보드 하위 시나리오를 통과시킨다(EV-107·108).
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다(두 Rust 각 719 Pass, README·ui_spec §7).

## 완료 기준

- [x] 운영 화면 구현과 값 구분·권한 경계 검증(EV-107·108).
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/연결 끊김·취소/종료를 해당 범위에서 검증한다(EV-107·108).
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-014](../../05_verification/test_case.md#tc-014). 결과: S2 [EV-107](../../05_verification/verification_evidence.md#ev-107), S3 [EV-108](../../05_verification/verification_evidence.md#ev-108). 추적표 MS 칸은 Task 수락 때 갱신한다.

## S1 결정 — 2026-11-21

Human 「A. viewer 내장 웹 (Recommended)」·「읽기 전용 (Recommended)」. 비교 근거와 결정 내용은 [ADR-016](../../03_design_artifacts/adrs/ADR-016_viewer_embedded_web_dashboard.md)에 있다.

- viewer가 loopback HTTP로 내장 정적 페이지와 JSON 경로를 제공한다. 외부 CDN·빌드 도구는 쓰지 않는다.
- 조회는 GetStatus(1초, 한 번에 하나)·Compare·PlanCheck뿐이다. 브라우저는 credential을 받지 않는다.
- 값 상태를 보존하고, 실패 시 마지막 성공 값·시각과 실패 사유·시각을 구분한다.
- 화면 범위는 [화면 §7](../../03_design_artifacts/ui_spec.md#7-task-032-운영-대시보드--2026-11-21)을 따른다.

## S2 결과 — 2026-11-21

- `--dashboard ADDR`로 loopback HTTP 대시보드를 연다. `/api/status`는 `dot_logger.viewer.dashboard.v1`이다. 상류 `status.v1`을 그대로 넘기고, 마지막 성공 값과 최근 실패 사유·시각을 분리해 담는다.
- 화면은 연결 머리줄, 최근 표본(Capture·전체·필수 출력), 10/60/300초 window, snapshot 목록을 보여준다. `status.v1`에 대상별 분해가 없어 출력은 전체·필수 합계로 표시한다([화면 §7](../../03_design_artifacts/ui_spec.md#7-task-032-운영-대시보드--2026-11-21)).
- 시험 5건(값 상태 보존, 지표 없음, 실패 뒤 마지막 값 유지, HTTP 경계, 실제 hub 응답·권한 거절·호스트 종료)이 stub에서 Red였고 구현 뒤 Green이다. 브라우저 확인 캡처 3종은 [근거](../../06_evolution/imported_context/task_032_s2_2026-11-21/README.md)에 있다.

## S3 결과 — 2026-11-21

- `/api/compare?before=&after=`와 `/api/plan?id=`는 호스트의 `compare.v1`·`check.v1` plan을 그대로 담거나, 이름 있는 실패(gone·unsupported·viewer_busy 등)를 돌려준다.
- 상태 조회와 조회 요청은 작업자 하나가 한 번에 하나씩 처리한다(상류 명령 슬롯 1개).
- 화면은 보유 snapshot 두 개의 비교(양쪽 조건, 다른 조건, 변화 8항목)와 계획 미리보기를 보여준다. 실행 버튼은 없다.
- 시험 3건(문서 계약, ID 검사, 단일 슬롯·Unsupported 뒤 연결 유지)이 stub에서 Red였고 구현 뒤 Green이다. 실제 `OperationsHost` 데모(`operations_host` 예제)와 브라우저로 확인했다. [근거](../../06_evolution/imported_context/task_032_s3_2026-11-21/README.md)

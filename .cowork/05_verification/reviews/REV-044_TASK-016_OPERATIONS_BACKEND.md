# REV-044 — TASK-016 운영 backend(status·apply·compare) 개발 자가 점검

## 개요

- 일자/담당: 2026-11-03 #001 / AI 개발 세션 자가 점검. 독립 리뷰가 아니다.
- 대상: `telemetry::operations::OperationsHost`, `RemoteError::{Conflict, Busy, RestartRequired}`와 server/client 매핑, `ManagementReply::Status.report`, CLI status/compare/config apply 출력, TC-009 operations·CLI 종단 시험.
- 상태: **Closed** — 아래 P3 3건은 문서화로 처리하고, 결함 수정이 필요한 항목은 없다. 사전 존재 불안정 시험은 GAP-008로 분리했다.
- 판정: **Pass — 로컬 기능 범위.** 근거: [EV-044](../verification_evidence.md#ev-044)·결과(비공개 원시 근거: `imported_context/ops_backend_2026-11-03/results.json`).

## 리뷰 요약

| ID | 중요도 | 위치·계약 | 문제 | 처리 | 상태 |
|---|---|---|---|---|---|
| RV-088 | P3 | `OperationsHost::handle_command` GetStatus | snapshot 목록(최대64개)이 report 크기를 키워, 작은 message 한도에서는 status 전체가 Limit이 된다. 잘라서 보내지는 않는다 | 의도된 동작(부분 report로 오인 방지). API§15.8에 "host는 message 한도에 맞게 슬롯 수를 정한다"를 명시 | Closed |
| RV-089 | P3 | `apply` Closing→Gone | ingress가 닫힌 경우 게시가 없는데도 CLI가 Gone 규칙에 따라 `completion_unconfirmed`를 붙인다 | 보수적 표시이며 성공으로 오인하지 않는다. 기존 Gone 규칙을 바꾸지 않고 문서에 남긴다 | Closed |
| RV-090 | P3 | `parse_patch` | 알 수 없는 필드는 거절하지만 JSON 중복 키는 serde_json 규칙대로 마지막 값이 쓰인다 | 신뢰된 Manage 권한 입력이며 적용 전 기존 `validate` 검사를 그대로 거친다. API§15.8에 명시 | Closed |

## [리뷰어] 점검

- **범위/권한:** Human 결정(2026-11-03)으로 원격 운영 backend를 구현하고, 새 오류 코드 3종과 Status report 확장을 채택했다. 기존 proto 필드(`Reply.data`)와 gRPC 코드만 사용하며 proto 파일·major는 바꾸지 않았다. 새 기술 계열/worker/저장 계약 변경이 없어 ADR 승격 대상이 아니다.
- **RT/할당:** RT 경로 변경 없음. 새 코드는 host 일반 worker에서만 실행되며 RT allocator 시험(feature matrix 내)이 그대로 통과한다.
- **게시 원자성:** apply는 기존 `ConfigUpdates::apply_config`만 호출한다. Conflict/Busy/RestartRequired/InvalidPatch에서 게시가 없음을 core 시험(posted revision 유지)과 CLI 종단 시험으로 확인했다. producer별 전환은 기존 `apply_pending_config` 계약 그대로다.
- **관측 정직성:** status는 Observed/Stale/Unavailable를 원래 사유와 함께 내보내고 미관측 필드를 생략한다(0 채움 없음). 첫 기준 표본 이전 레코드가 window 차분에서 빠지는 기존 의미를 시험에 명시했다. compare는 Comparison의 differences 전체와 Unavailable 사유를 보존한다.
- **wire 호환:** Busy/RestartRequired는 기존 코드(Unavailable/FailedPrecondition)를 공유하므로 고정 `dot-logger-reason` metadata로 구분한다. metadata가 없으면 기존 매핑(Transport/Version)으로 떨어져 성공으로 오인하지 않는다. Disabled의 Unavailable은 Transport로 유지됨을 시험했다.
- **수명/정합:** GetStatus 응답의 revision과 report의 `posted_revision`을 한 번의 읽기로 만든다(초기 구현의 두 번 읽기를 커밋 전 수정). snapshot ID는 인스턴스 안에서 재사용하지 않는다.
- **한계:** 1초 sampling·snapshot 생성·CPU/메모리/지연 측정은 host 책임이다. 실제 dot-probe 화면 연동(TASK-026), PC/RT 비용/오차, native(T3)는 이 수락 범위 밖이다.

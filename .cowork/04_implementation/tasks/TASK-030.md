# TASK-030 — MCAP 표준 호환(표준 도구 판독)

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-003·005 |
| 요구사항 | FR-003 FR-005 FR-010 NFR-004 NFR-007 NFR-008 |
| 설계 | [데이터](../../03_design_artifacts/data_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) |
| ADR | ADR-009·010 Accepted. 표준 저장 모드 ADR 후보(S2 전 채번) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-04 |
| 다음 액션 | S1 — Envelope 해제 표준 MCAP export와 독립 reader·Foxglove 계열 판독 |
| 의존 | TASK-007·TASK-009·TASK-010·TASK-011·TASK-027 |
| 예상 소요 | S1 1~2 + S2 3~4 인일, 리뷰 대기 제외 |

## 목표

현재 모든 채널이 `dot_logger.Envelope` protobuf로 저장되어([writer.rs](../../../crates/dot-logger/src/storage/writer.rs)) 표준 MCAP 도구가 사용자 payload를 디코딩하지 못한다. 사용자 schema·encoding 그대로 판독되는 파일을 제공한다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 표준 export | `dot-logger export --standard`가 `dot-logger/1` 파일을 사용자 schema 채널로 변환, 독립 MCAP reader 대조·Foxglove/Lichtblick 판독 확인, 손실 필드 명시 | Planned |
| S2 | native 표준 쓰기 모드 | ADR 승인 후 writer 옵션으로 표준 모드 기록, 복구·조회·재생이 두 형식 모두 지원, 기존 `dot-logger/1` 파일 판독 유지 | Planned |

## 변경 대상과 첫 검증

계획 경로: `crates/dot-logger/src/storage/`, `crates/dot-logger-cli`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **표준 모드/export 파일에서 독립 reader가 사용자 schema로 payload를 원문 일치 디코딩하고, Envelope 고유 필드(producer·u64 sequence·level·importance·revision·mono/real)는 보존 위치 또는 손실을 명시**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-014`의 표준 판독 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## Human 확인 필요 사항

- MCAP `sequence`는 u32, Envelope sequence는 u64 — 매핑/보조 채널 방식.
- `log_time`에 mono와 real 중 무엇을 둘지(표준 도구 타임라인 vs 순서 보장).
- 메시지 단위 level·importance·producer 보존 방식(보조 채널·채널 분리·metadata).
- ROS 2 encoding(cdr/ros2msg) 지원은 T1 검토 범위로 둔다.

## 완료 기준

- [ ] S1 export·S2 native 모드와 기존 형식 호환.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 손상/부분 파일·복구·예산 경계를 두 형식에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-014](../../05_verification/test_case.md#tc-014). 제품 코드·테스트 실행 결과: 없음.

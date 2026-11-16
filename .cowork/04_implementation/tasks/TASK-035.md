# TASK-035 — 손실 기록의 파일 내 표시와 채널별 손실 수

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-002·003·004 |
| 요구사항 | FR-003 FR-007 FR-014 NFR-006 |
| 설계 | [데이터](../../03_design_artifacts/data_model.md) · [인터페이스 §21](../../03_design_artifacts/interface_contract.md#21-task-030-mcap-표준-호환--2026-11-12) |
| ADR | [ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Proposed — TASK-030 S2와 함께 Human 승인 대기 |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-16 |
| 다음 액션 | S1 형식안 ADR-013 작성 완료 → D1~D3 Human 승인. S2 구현 미착수 |
| 의존 | TASK-030 S2 ADR(같은 저장 형식 결정) |
| 예상 소요 | S1 1 + S2 2~3 인일, 리뷰 대기 제외 |

## 배경 — 2026-11-15 Human 결정

TASK-033 S2·S3에서 확인한 사실:
- 로보틱스 기록기는 손실을 기록 안에 남긴다. PX4 ULog `O` dropout(구간 ms), ArduPilot `DSF`(1 Hz 손실 수·버퍼 여유), Zephyr `--- N messages dropped ---`, rosbag2 topic별 손실 수가 그 예다.
- dot-logger는 손실을 카운터·리포트·원격 Gap으로만 알린다. 파일에는 남기지 않는다([writer.rs](../../../crates/dot-logger/src/storage/writer.rs)는 header·schema·channel·message만 쓴다).
- 수락된 기록만 sequence가 올라가므로 표준 도구에서 구멍이 보이지 않는다.
- 손실 수는 producer 단위이고 채널별로 세지 않는다.

Human 「모두 권고대로 진행하자」로 추가가 결정됐다(UC-04).

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 형식안 | 손실 기록의 위치(전용 topic 또는 MCAP metadata)·필드(채널·producer·개수·구간·사유)·주기, 채널별 카운터, 표준 도구·Foxglove 표시, native 단일 형식의 손실 조회/복구 → Human 승인 | **In Progress — ADR-013 §3 형식안 작성, 승인 대기** |
| S2 | 구현 | 포화 시험에서 손실 구간이 파일에 남고 export·조회로 보인다. 손실 기록 자체가 넘칠 때의 상한 | Planned |

## 완료 기준

- [ ] 손실이 있는 기록 파일에서 손실 개수·구간을 채널별로 복원할 수 있다.
- [ ] native 파일의 손실 checkpoint 판독·복구 일관성 확인.
- [ ] EV 기록, Human 리뷰 완료.

## S1 권고안 — 2026-11-16

[ADR-013 §3](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md#3-손실-수집파일-보존-계약) 참조. native `_dot_logger/loss`의 protobuf 누적 checkpoint, 채널/producer/원래 target·사유·관측 구간·포화/미확인 상태를 제안한다. 제출/대여 거절은 재시도별 시도 수, 출력 결과는 assignment 수이며 고유 메시지 손실로 합산하지 않는다. 기존 telemetry와 독립된 수집·고정 메모리·sync_every 주기·회전/종료 기록을 계획했다. 정확 revision별 손실 분할/개별 발생 시각은 주장하지 않는다.

미배포 지적을 반영한 ADR-013 수정안은 native 단일 형식 전환이며, legacy 파일 판독/쓰기와 별도 legacy 손실 분기를 두지 않는다. 미관측 꼬리/다중 출력 중복 계수 방지는 native 경로에서 검증한다. 승인 전 코드·protobuf·시험 구현 결과는 없다.

# TASK-035 — 손실 기록의 파일 내 표시와 채널별 손실 수

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-002·003·004 |
| 요구사항 | FR-003 FR-007 FR-014 NFR-006 |
| 설계 | [데이터](../../03_design_artifacts/data_model.md) · [인터페이스 §21](../../03_design_artifacts/interface_contract.md#21-task-030-mcap-표준-호환--2026-11-12) |
| ADR | 저장 형식 변경 — TASK-030 S2 표준 저장 모드 ADR과 함께 설계 |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-15 |
| 다음 액션 | S1 — 형식안(기록 위치·필드·표준 export 매핑) → TASK-030 S2 ADR에 포함해 Human 승인 |
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
| S1 | 형식안 | 손실 기록의 위치(전용 topic 또는 MCAP metadata)·필드(채널·producer·개수·구간·사유)·주기, 채널별 카운터, 표준 export·Foxglove 표시, 기존 `dot-logger/1` 파일 판독 호환 → Human 승인 | Planned |
| S2 | 구현 | 포화 시험에서 손실 구간이 파일에 남고 export·조회로 보인다. 손실 기록 자체가 넘칠 때의 상한 | Planned |

## 완료 기준

- [ ] 손실이 있는 기록 파일에서 손실 개수·구간을 채널별로 복원할 수 있다.
- [ ] 기존 파일 판독·복구 호환 유지.
- [ ] EV 기록, Human 리뷰 완료.

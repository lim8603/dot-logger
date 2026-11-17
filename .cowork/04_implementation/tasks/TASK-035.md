# TASK-035 — 손실 기록의 파일 내 표시와 채널별 손실 수

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-002·003·004 |
| 요구사항 | FR-003 FR-007 FR-014 NFR-006 |
| 설계 | [데이터](../../03_design_artifacts/data_model.md) · [인터페이스 §21](../../03_design_artifacts/interface_contract.md#21-task-030-mcap-표준-호환--2026-11-12) |
| ADR | [ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Accepted — TASK-030 S2와 함께 Human 승인(2026-11-16) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-17 |
| 다음 액션 | S1 Done. S2 C 구현·로컬 검증 단위 마감(EV-085). 다음 D TASK-034 runtime 통합 후 E GUI/호출 비용/최종 회귀로 전체 마감 |
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
| S1 | 형식안 | 손실 기록의 위치(전용 topic 또는 MCAP metadata)·필드(채널·producer·개수·구간·사유)·주기, 채널별 카운터, 표준 도구·Foxglove 표시, native 단일 형식의 손실 조회/복구 → Human 승인 | **Done — ADR-013 §3, 2026-11-16 Human 승인** |
| S2 | 구현 | 포화 시험에서 손실 구간이 파일에 남고 native 파일 판독·조회로 보인다. 손실 기록 자체가 넘칠 때의 상한 | C 로컬 구현·검증 Done(EV-085); D 통합 뒤 E GUI/호출 비용·전체 마감 대기 |

## 완료 기준

- [x] 손실이 있는 기록 파일에서 손실 개수·관측 구간을 채널별로 복원할 수 있다(실제 발생 시각 아님, EV-085).
- [x] native 파일의 손실 checkpoint 판독·복구 일관성 로컬 확인(EV-085).
- [x] C 개발 EV 기록·위임 리뷰 마감(REV-058).
- [ ] ADR-013 E: native GUI·counter 추가 전후 호출 비용 대조·D 통합 후 최종 회귀/커버리지 판정. TASK 전체 In Progress 유지.

## S1 권고안 — 2026-11-16

[ADR-013 §3](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md#3-손실-수집파일-보존-계약) 참조. native `_dot_logger/loss`의 protobuf 누적 checkpoint, 채널/producer/원래 target·사유·관측 구간·포화/미확인 상태를 제안한다. 제출/대여 거절은 재시도별 시도 수, 출력 결과는 assignment 수이며 고유 메시지 손실로 합산하지 않는다. 기존 telemetry와 독립된 수집·고정 메모리·sync_every 주기·회전/종료 기록을 계획했다. 정확 revision별 손실 분할/개별 발생 시각은 주장하지 않는다.

미배포 지적을 반영한 ADR-013 수정안은 native 단일 형식 전환이며, legacy 파일 판독/쓰기와 별도 legacy 손실 분기를 두지 않는다. 미관측 꼬리/다중 출력 중복 계수 방지는 native 경로에서 검증한다. 2026-11-16 Human이 D1~D3를 승인했다. S1 Done. 2026-11-17 S2 구현을 재개했으며 현재는 counter/native checkpoint·조회/복구/CLI를 연결하고 검증 중이다.


## S2 C 구현·로컬 검증 — 2026-11-17

[API §23.3](../../03_design_artifacts/interface_contract.md#233-손실-누적-checkpoint-c-구현--2026-11-17)·[EV-085](../../05_verification/verification_evidence.md#ev-085)·[REV-058](../../05_verification/reviews/REV-058_LOSS_CHECKPOINT_C.md)를 따른다. telemetry 독립 고정 counter, native loss report·baseline/주기/회전/final·bounded loss-only 조회/복구·CLI, optional 원래 target identity를 연결했다. retry 시도와 assignment·skip·Unknown·미확인 꼬리를 분리한다. 포화·누락 part·누적 감소는 exact delta를 만들지 않는다.

신규 TC-017 14개·loss 단위4개·CLI4개·allocator1개가 counter/동시성·NoRoute/encode/raw·최종 tail·count/index 한도/coalesce·원 report 복구·전 출력 실패·잘못된 control/observer를 검증한다. 개발 중 RV-146~148을 Red→Green으로 수정했다. 4출력 성능 fixture의 메모리 조정은 새 예약을 반영한 것이며 성능 수치의 수락이 아니다. C 단위의 로컬 수락과 전체 Task 완료를 구분한다. 다음 작업은 TASK-034 runtime WIP(D), 그 뒤 E 표준 GUI/호출 비용/통합·TASK-033 재측정이다.


## REV-059 후속 — 2026-11-17

RV-149~152 조치·자가검토·위임 마감([REV-059](../../05_verification/reviews/REV-059_NATIVE_REVISION_LOSS_REVIEW.md) Closed, [EV-086](../../05_verification/verification_evidence.md#ev-086), 제품 `8b59aa6`). 복구 완전 prefix·손실 관측/차분·종료 watermark·초기 등록 검증을 보강했고 두 Rust 각672 Pass·5 ignored, Miri7·RT20 Pass다. A/B/C 로컬 완료와 Task 전체 In Progress를 유지한다. 다음 D TASK-034 runtime 통합 → E GUI/호출 비용·최종 통합/커버리지 판정이다.

# TASK-034 — facade에 core 기능 노출(실행 중 level·샘플링, 이벤트 보존, 중요도)

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-008 |
| Story | US-001·002·003·004 |
| 요구사항 | FR-007 FR-009 FR-011 FR-017 NFR-008 |
| 설계 | [인터페이스 §18](../../03_design_artifacts/interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05) |
| ADR | ADR-012 Accepted(facade 소유 모델). 실행 중 변경의 저장 계약은 [ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Accepted(2026-11-16 Human 승인) |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-18 |
| 다음 액션 | E 최종 검증·기존 위임 수락 완료(EV-088·REV-061). 후속 TASK-033 S4 |
| 의존 | TASK-027·TASK-029(facade·표면 가드) |
| 예상 소요 | S1 1 + S2 3~4 + S3 1 인일, 리뷰 대기 제외 |

## 배경 — 2026-11-15 Human 결정

TASK-033 S3의 UC↔TC 대응에서 다음 기능이 core(`CaptureOwner`·`SegmentLog`)에는 구현·시험돼 있지만 `Logger` facade에는 없다는 것을 확인했다([TASK-033](TASK-033.md#uctc-대응--s3-2026-11-15)). Human 「모두 권고대로 진행하자」로 ①~③을 이 Task로 등록하고, ④ 요청 시 분할·sync는 백로그 B1로 보냈다.

| # | 기능 | 지금 facade | core 근거 | UC |
|---|---|---|---|---|
| ① | 실행 중 채널 level·샘플링 변경, 원격 ApplyConfig | config update를 켜지 않음, 원격 ApplyConfig Unsupported → `Logged::Skipped` 도달 불가 | `CaptureOwner::enable_config_updates`, `tc_009_config_update` | UC-07·08 |
| ② | 이벤트 전후 구간 보존 trigger | `max_events: 0`·`pre_ns`·`post_ns` 0 고정 | `SegmentLog::protect_event`·`release_event`·`expire_events`, `tc_005_retention` | UC-17 (FR-011 Must) |
| ③ | 단순 경로(`try_log`)의 중요도 지정 | 채널 importance가 항상 Normal, 레코드별 지정은 고급 `producer()`로만 | `ImportanceMapping`, `RecordMetadata.importance` | UC-03 (FR-009 Must) |

API §18 표에 계획만 있고 구현되지 않은 `Writer::try_log_with(.., Meta)`도 S1에서 정리한다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 계약안 | 공개 항목(이름·시그니처)·기본값·RT 무대기/무할당 유지 방법·NFR-008 계수(현재 이름 있는 항목 30/60, L1 ≤20줄·≤6개념) 영향을 API §22로 제안 → Human 승인 | [API §22](../../03_design_artifacts/interface_contract.md#22-task-034-facade에-core-기능-노출--계약안-2026-11-15) Done — 2026-11-15 Human 「승인」(D1~D4 권고안) |
| S2 | 구현 | ①~③ 각각 실패 시험 → 최소 구현. facade `Logged::Skipped`가 실제로 나오는 시험 포함 | D 로컬 Done(EV-087) — ③ 중요도·① 시작 level·② 이벤트에 runtime set_channel/refresh·원격 ApplyConfig를 통합(f38~f44). WIP36c1724를 ADR-013 native/loss 계약에 맞춰 적용 |
| S3 | 레시피·가드 | 표면 snapshot bless, 레시피·README 갱신, 1.96/1.85 회귀 | D 가드35/60 Pass·레시피 불변, runtime README/rustdoc 추가. 두 Rust 각684 Pass·5 ignored(EV-087); 기존 EV-081/082 이력 보존 |

## 완료 기준

- [x] ①~③이 facade에서 동작하고 시험이 단언한다. 기존 RT 무대기·무할당 계약 유지(EV-087).
- [x] NFR-008 35/60·레시피 상한 이내(EV-087).
- [x] 대상 revision·환경·명령·결과를 EV-087에 기록.
- [x] 기존 위임으로 최종 리뷰 수락(REV-061).

## REV-055 후속 조치 — 2026-11-16

RV-137·138은 다중 출력 전송·rollback·오류 우선순위, 실패 배출의 queued reply 정리, 새 mark/주기 배치 쓰기 전 만료 정리로 조치했다. `tc_013_facade` f37과 logger 내부 5개 회귀 시험을 추가했다. [EV-082](../../05_verification/verification_evidence.md#ev-082). 실행 중 설정 변경의 ADR 의존과 기존 구현 결과 Human 확인 대기는 유지한다.

실행 중 변경의 저장 선행 설계는 [ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Accepted(2026-11-16 Human 승인)로 확정했다. 기존 S1/S2(실행 중 변경 제외)/S3 완료와 Human 결과 확인 대기는 유지하며 WIP를 병합하지 않았다.


## D runtime 통합 — 2026-11-18

WIP36c1724를 현재 native 저장/loss에 맞춰 통합했다([EV-087](../../05_verification/verification_evidence.md#ev-087), [REV-060](../../05_verification/reviews/REV-060_RUNTIME_CONFIG_D.md)). set_channel·refresh·held/tracing/반납 ack·원격 ApplyConfig와 실제 파일 r1/r2/r3·세션 누적 손실6건을 확인했다. Busy/Invalid/Closing/Failed·pending/기한·try_lock·RT0을 검증한다. 전체 Task는 E GUI/호출 비용/최종 통합·커버리지/UC 및 최종 리뷰를 남겨 In Progress다. §22.8 및 위의 「미병합」 기록은 통합 전 이력이다.

## E 최종 수락 — 2026-11-18

runtime 레벨·샘플링과 파일 revision·누적 손실을 [EV-088](../../05_verification/verification_evidence.md#ev-088)에서 최종 대조했다. tracing 포매터가 잠금을 쥔 동안의 설정 게시와 패닉 이후 revision 적용·저장을 새 회귀로 확인했다. [REV-061](../../05_verification/reviews/REV-061_ADR_013_E.md)의 기존 위임 수락으로 TASK-034와 MS-008을 마감했다.

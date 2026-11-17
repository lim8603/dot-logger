# TASK-034 — facade에 core 기능 노출(실행 중 level·샘플링, 이벤트 보존, 중요도)

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-008 |
| Story | US-001·002·003·004 |
| 요구사항 | FR-007 FR-009 FR-011 FR-017 NFR-008 |
| 설계 | [인터페이스 §18](../../03_design_artifacts/interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05) |
| ADR | ADR-012 Accepted(facade 소유 모델). 실행 중 변경의 저장 계약은 [ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Accepted(2026-11-16 Human 승인) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-17 |
| 다음 액션 | 기존 결과 Human 확인 대기 유지. A/B/C 로컬 선행 완료(EV-083~085) → WIP36c1724를 native/loss 계약에 맞춰 ① runtime 통합·시험(D) |
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
| S2 | 구현 | ①~③ 각각 실패 시험 → 최소 구현. facade `Logged::Skipped`가 실제로 나오는 시험 포함 | Done(실행 중 변경 제외) — ③ 중요도(`561f2c3`, f25~f30), ① 시작 시 level(`340c19c`, f31), ② 이벤트 보존(`949a74e`, f32~f36). ① 실행 중 변경은 TASK-030 S2 ADR 뒤(§22.8, WIP 브랜치 `36c1724`) |
| S3 | 레시피·가드 | 표면 snapshot bless, 레시피·README 갱신, 1.96/1.85 회귀 | Done — 표면 34/60 Pass, 레시피 계수 불변, README 「선택 옵션」 절, 1.96/1.85 각 625 Pass([EV-081](../../05_verification/verification_evidence.md#ev-081)) |

## 완료 기준

- [ ] ①~③이 facade에서 동작하고 시험이 단언한다. 기존 RT 무대기·무할당 계약 유지.
- [ ] NFR-008 수치 안에 있거나 초과 시 Human 승인.
- [ ] 대상 revision·환경·명령·결과를 EV로 기록.
- [ ] Human 리뷰 완료.

## REV-055 후속 조치 — 2026-11-16

RV-137·138은 다중 출력 전송·rollback·오류 우선순위, 실패 배출의 queued reply 정리, 새 mark/주기 배치 쓰기 전 만료 정리로 조치했다. `tc_013_facade` f37과 logger 내부 5개 회귀 시험을 추가했다. [EV-082](../../05_verification/verification_evidence.md#ev-082). 실행 중 설정 변경의 ADR 의존과 기존 구현 결과 Human 확인 대기는 유지한다.

실행 중 변경의 저장 선행 설계는 [ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Accepted(2026-11-16 Human 승인)로 확정했다. 기존 S1/S2(실행 중 변경 제외)/S3 완료와 Human 결과 확인 대기는 유지하며 WIP를 병합하지 않았다.

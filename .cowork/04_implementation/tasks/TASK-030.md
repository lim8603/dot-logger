# TASK-030 — MCAP 표준 호환(표준 도구 판독)

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-003·005 |
| 요구사항 | FR-003 FR-005 FR-010 NFR-004 NFR-007 NFR-008 |
| 설계 | [데이터](../../03_design_artifacts/data_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) |
| ADR | ADR-009·010 Accepted. [ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Accepted — S2 native 단일 형식·손실·revision 저장 계약 승인(2026-11-16) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-17 |
| 다음 액션 | S2 A/B/C 로컬 단위 Done(EV-083~085). 다음 D TASK-034 runtime WIP 통합 → E GUI/비용 비교·통합·판정 |
| 의존 | TASK-007·TASK-009·TASK-010·TASK-011·TASK-027 |
| 예상 소요 | S1 1~2 + S2 3~4 인일, 리뷰 대기 제외 |

## 목표

사용자 schema·encoding 그대로 판독되는 native 표준 MCAP 파일을 제공한다. 이전 Envelope 저장은 S2 A에서 제거했으며 S1 export 성과는 이력으로 유지한다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 표준 export | `dot-logger export --standard`가 `dot-logger/1` 파일을 사용자 schema 채널로 변환, 독립 MCAP reader 대조·Foxglove/Lichtblick 판독 확인, 손실 필드 명시 | **Done** 2026-11-15 — 로컬 Pass([EV-077](../../05_verification/verification_evidence.md#ev-077)), Foxglove 판독 Human 확인(아래 S1 리뷰 기록). Lichtblick 미실행 |
| S2 | native 표준 단일 경로 | ADR-013 승인안에 따라 native 단일 형식으로 기록·조회·재생·복구. 배포 전 개발 형식의 판독/legacy 쓰기 보존은 제품 범위에서 제외(ADR-013 D1 수정안). **ADR 범위에 함께 넣는 것(2026-11-15):** TASK-035 손실 기록 형식, TASK-034 실행 중 설정 변경을 위한 「revision 변경 시 출력 파일 회전」과 한 출력 안의 여러 revision 조회·복구(API §22.8) | **In Progress — ADR-013 Accepted(2026-11-16), A/B/C 로컬 Done(EV-083~085), 다음 D runtime WIP·E 통합** |

## 변경 대상과 첫 검증

구현 경로: `crates/dot-logger/src/storage/`·`query/`, `crates/dot-logger-cli` 및 예제/소비 시험.

첫 실패/리뷰 조건: **표준 모드/export 파일에서 독립 reader가 사용자 schema로 payload를 원문 일치 디코딩하고, Envelope 고유 필드(producer·u64 sequence·level·importance·revision·mono/real)는 보존 위치 또는 손실을 명시**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-014`의 표준 판독 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## Human 결정 — 2026-11-12

착수 확인 4건 + 구현 중 확인 2건(⑤ facade `Schema::descriptor`, ⑥ `payload_schema_name` 채널 metadata) 모두 권고안([API §21.1](../../03_design_artifacts/interface_contract.md#211-human-결정--2026-11-12)): ① MCAP `sequence` = 하위 32비트, u64는 보조 메타 채널 ② `log_time`·`publish_time` = real, mono는 보조 ③ level·importance·producer·revision·source는 보조 메타 채널 1개(protobuf `RecordMeta`) ④ ROS 2 encoding은 T1 유지.

## S1 리뷰 기록 — 2026-11-15

| 항목 | 결과 |
|---|---|
| `robot/blob`(`vendor-blob`, descriptor 없음) | Foxglove `Unsupported encoding vendor-blob`(2026-11-13 Human 판독). 처리: Human 「권고대로」 — 원문·encoding 유지, 한계 기록([API §21.2](../../03_design_artifacts/interface_contract.md#212-s1-구현-반영--2026-11-12)) |
| CLI summary `"records":0` | 결함 아님. `Output::finish`가 모든 명령 summary에 출력한 `record` 줄 수를 넣는데 export는 `record` 줄을 내지 않는다. 변환 메시지 수는 `export` 행 `messages`. CLI README에 의미를 추가 |
| `robot/pose`·`_dot_logger/meta` | Foxglove Topics 판독(Human, 2026-11-15, 재생성 파일 pose 300·blob 10·meta 310): `robot/pose` = `test.Pose`(protobuf) 300건·99.58 Hz·`.x`/`.y` float32, `_dot_logger/meta` = `dot_logger.RecordMeta`(protobuf) 310건·102.90 Hz·필드 channel·producer·sequence(uint64)·mono·real(int64)·revision·level·importance·source{ticks, clock_domain} 전부 인식. `robot/blob`(channel 3)만 `Unsupported encoding vendor-blob` — 수락한 한계. GUI 값 표시(plot)는 보지 않았고 payload 값 디코딩은 EV-077 protoc 대조로 확인됨 |

**S1 판정: Done(2026-11-15).** 남은 미실행: Lichtblick 판독, 공식 `mcap` Python 대조, 큰 파일 export 시간·메모리, native(T3).

## 완료 기준

- [ ] S1 완료 이력 보존·S2 native 단일 경로 검증, fixture/예제/consumer 갱신 및 구형 profile 명시 거절(ADR-013 수정안).
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] native 손상/부분 파일·복구·예산 경계와 구형 profile 거절을 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-014](../../05_verification/test_case.md#tc-014). S1 구현·검증은 EV-077, 후속 REV-055 RV-136·139·140 조치는 EV-082에 기록한다.

## S2 저장 설계안 — 2026-11-16

[ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Accepted(2026-11-16 Human 「승인」). D1은 배포 전 native 단일 형식 전환·legacy 보존/선택 API 제외, D2는 TASK-035 누적 계수/미확인 상태, D3는 r1→r2→r1도 허용하는 단일 active writer 회전·source별 revision 조회/복구다. 단일 형식 전환은 S2 A, 다중 revision은 S2 B에서 구현했으며 손실은 후속 단계다. 세부 형식·예산·실패/검증 계획은 ADR 본문을 따른다.

2026-11-16 Human의 미배포 지적을 반영해 초안의 기존 파일/명시 legacy 쓰기 호환 전제를 제거했다. 과거 S1 성과와 EV는 이력으로 유지하며, ADR 전체는 이어진 Human 「승인」으로 Accepted가 됐고 제품 구현을 진행할 수 있다.


## S2 A 구현·검증 — 2026-11-16

[EV-083](../../05_verification/verification_evidence.md#ev-083)·[REV-056](../../05_verification/reviews/REV-056_NATIVE_STORAGE_A.md). native writer·borrowed reader·단일 revision query/recovery·예제/fixture/소비 경로 정렬, 구형 Envelope/export API·CLI 제거. 신규 TC-015 11개와 TC-014의 독립 protobuf 사용자/RecordMeta 검사를 실행한다. 구형 converter 전용 23개 시험을 제거하고 native 11개·최소 보존예산1개·CLI 2개(미지원 export·일반 terminal summary cap)로 대체/이관했다. 전체 시험 수 차이는 기능 미지원 결정에 따른 것으로 구형 시험 성공을 새 형식 근거로 재사용하지 않는다. 손상/복구·query/blob/replay·보존·RT allocator 회귀는 유지한다.

| 승인 후 단계 | 상태/다음 조건 |
|---|---|
| A native 쓰기/읽기·구형 경로 제거 | Done — EV-083, 두 Rust 각630 Pass·각18 feature·REV-056 Closed |
| B revision 경계 회전·source별 조회/복구·manifest v2 | Done — 2026-11-17 EV-084, REV-057 Closed |
| C 손실 collector/체크포인트/조회 | Done — 2026-11-17 EV-085, REV-058 Closed(전체 Task는 E 뒤) |
| D runtime facade 설정 변경 | TASK-034 WIP, C 뒤 |
| E 통합·커버리지·일정 판단 | TASK-033 재측정, 전체 저장 변경 뒤 |

A 완료 당시 모든 파일은 `loss_tracking=unavailable`, 단일 revision 조회/복구였다. 현재 A/B/C 구현은 아래를 따르며 native GUI 재판독·정량 성능/커버리지 재측정·다른 OS·24h·실사용처는 미실행이다.


## S2 B 구현·검증 — 2026-11-17

[EV-084](../../05_verification/verification_evidence.md#ev-084)·[REV-057](../../05_verification/reviews/REV-057_REVISION_STORAGE_B.md), [API §23.2](../../03_design_artifacts/interface_contract.md#232-revision-회전source별-조회복구-b-구현--2026-11-17).

- 실제 delivery r1→r2→r1→r2를 순서대로 파일 분리하며 per-producer receipt·catalog/event/pin·FileLedger를 유지한다. 일반 경로 `append_with_budget`는 같은 router/target의 명시 budget만 수락하고, IO batch는 위조 불가능한 Delivery에서 내부 budget을 파생한다.
- 새 source별 revision/sequence snapshot·중복 receipt/identity 누락 거절, blob/replay, source별 recovery sequence와 CLI manifest v2·다중 파일 복구/상한을 구현했다. v1 호환 adapter는 없다.
- 신규 교차 revision 통합 8개와 CLI 4개, 기존 fixture/consumer/RT allocator/UPS 예제를 정렬했다. capacity·보호/pin·create/header/첫 pair 오류·발행 prefix와 sync work를 검증했다. Header-only 정상 빈 조회 공백(RV-145)을 Red→Green으로 수정했다.
- B 검증·위임 수락 결과는 EV-084다. **TASK-030 S2 전체 Done은 아니다.** 모든 파일의 loss는 unavailable이고 C TASK-035 S2 → D TASK-034 runtime WIP → E 통합·커버리지/일정 판단이 남는다.


## S2 C 손실 checkpoint — 2026-11-17

TASK-035 S2의 C 로컬 단위를 구현·검증·위임 수락했다([EV-085](../../05_verification/verification_evidence.md#ev-085)·[REV-058](../../05_verification/reviews/REV-058_LOSS_CHECKPOINT_C.md), API §23.3). facade는 telemetry off여도 누적 loss를 기록하며 고급 경로는 명시 활성화한다. source별 expected target·별도 loss-only API/CLI·원 report 보존 복구를 제공하고 사용자 record/replay와 섞지 않는다. 앞의 A/B 절의 unavailable·C 미완료는 해당 단위 완료 당시 기록이다.

다음 D는 TASK-034 WIP `wip/task-034-s2b-runtime-config`(`36c1724`)를 현재 native 저장·loss 계약에 맞춰 적용·시험하는 것이다. 단순 cherry-pick 성공으로 수락하지 않는다. E의 native GUI·counter 전후 호출 비용·통합/커버리지·일정 판정은 아직 미실행이며 TASK-030·035 전체 In Progress를 유지한다.

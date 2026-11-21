# REV-058 — TASK-035 S2 C 손실 누적 checkpoint 개발 자가검토

| 항목 | 내용 |
|---|---|
| 일자·리뷰어 | 2026-11-17 / AI, code-review 스킬 적용 |
| 대상 | `5215b78` 이후 TASK-035 S2 counter·native control·조회/복구·client/CLI·fixture 변경 |
| 판정 | Pass — 두 Rust 최종 각665·각18 feature·정적/doc·Miri/RT 가드 Pass |
| 상태 | Closed — RV-146~148 수정·최종 회귀·2026-11-17 위임 수락 |
| 근거 | [EV-085](../verification_evidence.md#ev-085), 실행 자료(비공개 원시 근거: `imported_context/task_035_s2_2026-11-17/`) |

## 요약정리

ADR-013 §3의 session 누적 counter를 telemetry와 분리하고 producer 시도·의도적 skip·원래 target assignment terminal 전이를 native loss checkpoint에 보존한다. 사용자 pair/receipt·query/replay와 분리된 제어 데이터이며 source target identity와 bounded loss-only 조회/복구를 연결한다. 출력 보고는 공유 atomic 포화 갱신을 사용하고 RT 단일 writer counter와 구분했다. 자가검토에서 주기 control의 retention index 상한(RV-146), report observer와 file target 대조 공백(RV-147), 사용자 quota 뒤 control-only source 복구와 Limit 분류(RV-148)을 재현·수정했다. D runtime·E GUI/비용 비교/커버리지 전체 마감은 후속이다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | `src/loss.rs`, `storage/loss_report.proto`·`.bin` | 고정 counter grid·포화 관측·bounded protobuf·complete watermark 비교·고정 enum descriptor |
| 수정 | `capture/mod.rs`, `completion/mod.rs`, `routing/mod.rs`, `lib.rs` | 명시 활성화/빌림 뷰·단독 producer counter·원래 채널 index 게시·최초 terminal counter·budget 연결 |
| 수정 | `storage/{mod,writer,retention,read,recovery}.rs` | baseline/주기/회전/final·상한/예약·readers·root availability·report 보존 복구·observer 검증 |
| 수정 | `query/mod.rs`, client `offline.rs`·`lib.rs` | loss-only bounded snapshot, target identity, 사용자 fanout 데이터와 별도 조회 |
| 수정 | `logger/{mod,budget,writer,host}.rs` | telemetry off 기본 활성화·인코딩 실패 귀속·파생 예약·sync 전 표본·증명된 quiescence만 final |
| 수정 | CLI `args.rs`·`model.rs`·`commands.rs`·README | loss 행·관측 필터·nullable delta/ID·source target·복구 전체 part 한도 |
| 생성/수정 | `tc_017_loss.rs`, CLI 시험, allocator 시험, 기존 storage/query/recovery/perf/consumer/예제 fixture | 손실14개·CLI4개·allocator1개 신규, 기존 literal/독립 reader의 physical control 정렬 |
| 수정 | API·데이터·ADR 진행 기록·Task/MS·TC/EV·상태 | 구현/근거·잔여 조건 동기화 |

## 클래스(타입) 변경표

| 구분 | 소속 | 타입 | 역할/변경 |
|---|---|---|---|
| 생성 | loss | `Scope`, `Cause`, `Coverage`, `LossCount`, `LossReport`, `LossDelta`, `Counters` | 공개 고정 코드·관측·누적/차분·관리 뷰 |
| 생성 | loss 내부 | `Cell`, `Bank`, `WireCount`, `WireReport` | 사전 예약 atomics/clock·origin·private protobuf |
| 수정 | capture/completion | `CaptureCore`, `Producer`, `GeneralProducer`, `Registry`, `RecordSlot`, `RoutePlan`, `Reservation` | 공유 bank 소유/빌림·active call guard·채널 index 게시·terminal 최초 집계 |
| 수정/생성 | storage | `WriterBudget`, `WriterConfig`, `SegmentWriter`, `SegmentLog`, `FileInfo`, `NativeReader`, `LossReader`, `RecoveryOptions`, `RecoveryReport`, `Validation` | control part 예산·final 예약·원래 report/target 보존·chunk 전체 검증 |
| 수정/생성 | query/client | `QueryConfig`, `QuerySource`, `Snapshot`, `LossSnapshot`, `OfflineSource` | report 상한·명시 target·loss-only scan/관리 조회 |
| 수정 | CLI/facade | `Source`, `Limits`, `Manifest`, `Writer` | source target·전체 상한·encode 이전 거절 귀속. facade 공개 이름 수 추가 없음 |
| 생성 | 시험 내부 | `Memory`, `State`, `Store`, `Sink`, `Observed`, `Temp`, `Bad` | bounded sink·상태/fault 관측·파일 정리·encode 실패 fixture |

## 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 문제·처리 | 상태 |
|---|---|---|---|---|
| RV-146 | P2 | SegmentLog checkpoint / max_index_bytes | 주기 control이 디스크/count/chunk는 검사하지만 retention index 상한은 누락. 이번 표본+reserved final의 physical index까지 비교 | 수용·수정·EV-085 Pass / Closed |
| RV-147 | P2 | native Validation.loss_report / observer identity | session/part/header는 맞아도 다른 output의 observer를 수락. file target과 report observer 대조 추가 | 수용·수정·EV-085 Pass / Closed |
| RV-148 | P2 | CLI recovery / 사용자·control 독립 quota | 사용자 quota가 0이 되면 뒤 control-only source도 거절, NeedRotation을 IO4로 분류. control 전용 max_records0 지원·Limit3 대조 | 수용·수정·EV-085 Pass / Closed |

### RV-146 — 사용자 기록이 없는 주기 보고의 인덱스 상한

**[리뷰어]** retention.max_index_bytes=4096, max_loss_reports=16, max_segments=1에서 첫 주기 표본까지는 예약 final 포함 허용하지만 이후 표본은 회전/capacity여야 한다. 기존 구현은 주기 표본을 계속 추가했다. writer의 max_chunks/메모리는 유한하지만 명시 retention index 상한 계약을 어겼다. Red(비공개 원시 근거: `imported_context/task_035_s2_2026-11-17/index-red.log.gz`).

**[처리담당]** 수용. `SegmentWriter::projected_loss_index_bytes`에 기존 사용자/제어 message·physical chunk·auxiliary index와 이번 K part·reserved final K part를 포함하고 SegmentLog.checkpoint에서 회전 전에 비교한다. count 포화와 index 포화를 같은 시험의 별도 조건으로 재현한다. 회전 capacity는 원래 파일/누적 counter를 보존하며 final에 coalesced=true, count3/delta3이 남는다. Green(비공개 원시 근거: `imported_context/task_035_s2_2026-11-17/boundary-green.log.gz`).

### RV-147 — 다른 observer로 변조한 CRC 정상 control

**[리뷰어]** 첫 report의 protobuf observer10을20으로 바꾸고 chunk CRC를 다시 계산하면 reader가 이를 정상 관측으로 수락했다. 동일 session의 다른 출력으로 가장한 관측은 누적 watermark 비교의 provenance를 훼손한다. Red(비공개 원시 근거: `imported_context/task_035_s2_2026-11-17/observer-red.log.gz`).

**[처리담당]** 수용. `Validation::loss_report`가 원래 FileInfo.target과 observer를 일치 검증한다. native iterator/query/recovery가 같은 Validation을 사용한다. 회복 파일도 원래 target/report를 보존하므로 새 sink budget의 target을 대조하지 않는다. 변조 control은 반출 전 InvalidRecord로 거절한다. Green(비공개 원시 근거: `imported_context/task_035_s2_2026-11-17/boundary-green.log.gz`).

### RV-148 — 사용자 quota를 채운 뒤 control-only 파일의 복구

**[리뷰어]** 사용자 max_records1, 두 출력 파일(앞 user1+control6, 뒤 user0+control6)에서 앞 파일 뒤 Limit으로 중단해 뒤 손실 관측을 보존하지 못했다. 사용자 기록 상한과 별도인 control part 상한의 계약에 어긋난다. Red(비공개 원시 근거: `imported_context/task_035_s2_2026-11-17/control-quota-red.log.gz`). 또한 max_records0으로 실제 사용자 기록을 거절한 core NeedRotation/Limit이 CLI에서는 IO exit4로 바뀌었다.

**[처리담당]** 수용. writer는 양수 max_loss_reports가 있으면 max_records0의 control-only 예약을 허용하되 append는 사용자 pair 쓰기 전에 NeedRotation으로 거절한다. CLI는 남은 control allowance가 있으면 뒤 source를 처리하고 recovery Limit은 exit3으로 매핑한다. 뒤 control-only는 user0/control6·정상0, 뒤에 사용자 fanout이 있으면 user0·부분3·앞 정상 파일 유지로 둘 다 대조했다. 기존 사용자 상한과 count/bytes/part/chunk 상한은 유지한다. TC-017 baseline 파일도 max_records0으로 쓰고 NativeReader의 사용자 count0과 원 report를 확인한다. Green(비공개 원시 근거: `imported_context/task_035_s2_2026-11-17/control-quota-green.log.gz`).

## 리뷰 포인트

- RT 수집/거절/raw loan/Drop에는 heap·refcount·IO·추가 시계·lock/CAS retry가 없다. bank와 채널/target grid를 대여 전에 예약·초기화한다. producer의 기존 등록 index를 사용하고 active call guard가 counter 증가까지 감싼다. facade encode failure는 core 이전1회다.
- output 최초 terminal 소유권은 기존 slot claim/phase guard로 보호한다. phase Release가 채널 index/sequence를 게시하며 report/reconcile의 Acquire 이후에만 읽는다. 성공적 최초 terminal 전이만 공유 atomic saturating RMW로 집계해 다른 slot의 동시 report 유실과 반복 보고 중복을 막는다. 새 unsafe는 없다.
- count가 MAX이거나 overflow flag를 보면 보수적으로 MAX/overflow를 함께 내보내 찢어진 relaxed 관측이 exact delta가 되지 않는다. complete part 집합·observer 순서·epoch를 확인하고 incomplete 최신값/포화/감소에서 차분을 추측하지 않는다. replicas는 합산하지 않는다.
- 모든 loss 메시지도 chunk 전체의 schema/등록·CRC·길이·원래 session/observer/header와 함께 검증한 뒤 채택한다. 후반 control 손상 때 앞 user pair도 반출하지 않는다. scalar 중복/폭·bool·행 수를 prost 할당 전에 제한한다. ID0 presence를 보존한다.
- live writer의 baseline/final count·physical chunk·control byte/index 예약을 유지한다. capacity는 active 보존/coalesce, IO는 poison/published prefix 유지다. 복구는 현재 bank를 끊고 원래 보고만 복사하며 사용자 기록/제어 part/작업 전체 byte를 별도로 제한한다.
- Unix wall time은 표준 MCAP header용이고 counter epoch monotonic 관측 구간과 다르다. Closing만으로 tail을 증명하지 않는다. 미반납 writer/전 출력 실패/마지막 저장 이후 RAM-only terminal은 미확인이다. 네이티브 GUI·호출 비용·전체 커버리지/UC 판정은 ADR-013 E에서 확인한다.

## 검증과 마감

제품 `5be17b5`의 156개 최종 source SHA/commit blob 일치, 두 Rust 전체 각665 Pass·5 ignored·각18 feature·clippy/strict docs, fmt·표면34/60·Miri17 및 control-only1 재검증·allocator(default19/remote20) Pass를 확인했다. Open(자가 진단) → In Progress(RV-146~148 수정) → Resolved(최종 회귀) → Closed(2026-11-17 위임 수락)로 마감한다. TASK-030·035 전체와 D/E·GUI/비용/커버리지 판정은 후속이며 새 Human 설계 승인으로 대체하지 않는다.


## 새 상한 gate 소비자 확인 (F-11)

| gate | 소비자/실제 확인 |
|---|---|
| loss control required/사전 초기화 | CaptureOwner enable exact/required-1·idempotent·telemetry reset·facade 파생 budget·RT allocator |
| live L≥2K/chunk≥2K+2/final 공간 | 직접 SegmentWriter·SegmentLog·facade·기존 fixture/consumer/예제 literal, count/index 포화·active 보존·final coalesce |
| 복구 사용자/control quota 분리 | recovery는 원 file loss shape로 등록하며 live 표본 없음. max_records0 control-only·후속 user 거절·CLI 전체 part/읽기/기록 한도 |
| 별도 loss snapshot과 explicit target | 일반 QueryRecord/Replay의 중복 receipt 유지, loss-only fanout 미재생·같은 seq 다른 target·wrong target/observer 거절·client/CLI 연결 |
| 물리 control vs 논리 사용자 | 독립 MCAP reader의 user/meta/control 분리·native reader count·recovery records/loss_reports·CLI summary.records0 |

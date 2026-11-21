# REV-037 — TASK-019 S3 원 Delivery·DB 완료·파일 격리 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #012 / AI 개발 자가 점검. [code-review 스킬](../../../.agents/skills/code-review/SKILL.md) 적용, 독립 리뷰 전용 세션과 구분한다.
- 대상: 소스 `86b1f67`, RoutedWriter/DeliveryAttempt·명시 schema mapping·원 guard/실제 로컬 IO·completion Busy·파일 격리 및 기존 TCP proxy fixture 정리.
- 판정: **Pass — S3 로컬 기능·파일 영향 관측 범위**. 미해결 P1/P2/P3 없음.
- 상태: **Closed** — Open → In Progress → Resolved → 기존 위임에 따른 로컬 수락. Task/MS/TC 전체·실앱/정량 성능/native는 미완료.
- 근거: [EV-039](../verification_evidence.md#ev-039)·최종12개 검사/소스114개 hash·commit blob 대조(비공개 원시 근거: `imported_context/task_019_routed_2026-11-01/results.json`).

## 요약표

| ID | 중요도 | 위치/계약 | 발견·제안 | 판정 | 상태 |
|---|---|---|---|---|---|
| RV-076 | P2 | DeliveryAttempt::into_delivery | terminal/실제 시도 뒤 guard도 꺼낼 수 있어 API가 재제출 금지를 강제하지 못함. S1 timestamp 방어와 별도로 owner 복구를 미시도로 제한 | 수용·None update일 때만 Ok, terminal은 Err(Self) 유지 | Closed |
| RV-077 | P2 | RoutedWriter::write_delivery | 미시도 cancel 전에 schema decode/runtime 생성으로 잘못된 payload가 AdapterFailure로 분류됨 | 수용·identity 뒤 cancel 우선, NotSent/Dropped(Cancelled) | Closed |
| RV-078 | P2 | client/tests/transport.rs proxy 정리 | MSRV 전체 회귀에서 예약40,976,384B 잔여/0 기대 실패. 부모 proxy abort 시 자식 upload task handle이 유실되어 합류하지 못함 | 수용·test owner로 handle 전달, 양 방향 abort/await 후 기존 예약0 검사 유지 | Closed |

## [리뷰어] 검토

- **RT/unsafe:** 새 구현은 명시 general worker 전용이고 RT/원자 순서/자체 unsafe 변경 없음. private current-thread runtime을 매 동기 호출에서 완전히 정리한 뒤 원 guard를 반환한다. nested runtime은 미시도 오류, 임의 callback/자동 worker/daemon을 넣지 않는다. DNS/blocking OS 정리는 TTL 뒤에도 기다릴 수 있다.
- **원 schema/identity:** 고정 target/session/초기 revision·원 schema ID/version·I64Le/F64Le/BoolByte/Utf8Event만 허용. 원 metadata.real_ns/receipt·bytes 불변. 불명 raw/image 자동 projection, source/mono clock 변환, live revision/schema migration 없음.
- **결과/완료:** 원 DeliveryAttempt가 native 결과와 고정 Update를 보유한다. 실제 IO 정리 후 publish만 bounded 갱신하며 Busy면 owner를 유지하고 게시만 재시도. ACK/definitive reject/Unknown/미시도 cancel/schema failure 구분. 미게시 Drop은 기존 Unknown fallback, 게시 후에도 guard Drop 전 원본 유지.
- **수명/예산:** schema metadata는 같은 external authority의 추가 예약이며 fixed schema≤128. original/target FIFO/retained bytes/외부 예산은 기존 Router/Writer 계약 유지. terminal guard를 재전송용으로 복구하지 못한다. local runtime Drop은 진행 IO를 남기는 timeout/background shutdown을 사용하지 않는다. 원격 rollback/전체 RSS·DNS hard deadline·완전한 raw-pool 간섭 배제는 약속하지 않는다.
- **파일 격리:** DB in-flight1/raw8B 정지·포화 중 file6건/마감·독립 MCAP read 완료. 추가 DB5건 Capacity drop은 명시하고 전체 무손실로 합산하지 않는다. 동일 off/on 입력의 파일 bytes/원 payload·receipt/schema/mono/real/순서를 대조했다. file phase ns는 단일 쌍의 기능 관측으로 정량 성능 판정 아님.
- **보안/API:** 기존 redacted 오류·민감 header·endpoint/retry 정책 유지. 외부 runtime/wire 타입을 강제 노출하지 않는다. 신규 의존성/DB 운영 설정/공개/CI 변경 없음. 원 completion 상태를 DB 조회 성공이나 disk 내구성과 혼합하지 않는다.
- **범위/설계:** scoped 동기 한 건 API는 원 guard와 취소 가능한 HTTP future의 수명을 분리하지 않기 위한 비RT adapter 내부 선택이다. ADR-007/009 방향·기존 actual-close 계약을 유지하며 새 아키텍처/요구 범위·기술 계열 변경이 아니라 새 ADR로 승격하지 않았다. S1/S2 회귀와 실제 pin DB를 재검증했다.

## [처리담당] 조치·검증

- **RV-076 수용:** 복구 계약 Red(비공개 원시 근거: `imported_context/task_019_routed_2026-11-01/red-recovery.log`) 후 `into_delivery -> Result<Delivery,Self>`로 제한했다. foreign target/nested runtime 미시도 guard만 복구·원 Pending 보존, ACK 뒤 terminal 복구는 Err(owner)로 유지하고 Drop 뒤 raw 회수1을 확인했다. S1 방어가 있어 기존 중복 DB 쓰기를 관측했다는 의미는 아니다.
- **RV-077 수용:** 초기 cancel+invalid scalar Red(비공개 원시 근거: `imported_context/task_019_routed_2026-11-01/red-cancel.log`)에서 잘못된 AdapterFailure를 확인, identity 확인 뒤 decode/runtime/HTTP 전에 cancel 처리해 NotSent/Cancelled·Dropped(Cancelled)를 확인했다.
- **RV-078 수용:** 수정 전 MSRV 회귀(비공개 원시 근거: `imported_context/task_019_routed_2026-11-01/msrv-first.log`)를 보존했다. proxy upload JoinHandle을 oneshot으로 test owner에 전달하고 부모/자식 모두 abort/await 후 기존 budget0 검사를 유지한다. 두 Rust각5회와 최종 전체 회귀 Pass. 제품 remote transport/예산 반환을 바꾸거나 예약0 기대를 완화하지 않았다.
- 신규 routed6개·private completion Busy1개. final Rust1.96/MSRV1.85 workspace각401 Pass/실제 DB4개 일반 회귀에서 ignore, 별도 실제 DB각4개 Pass/cleanup0. 16features·독립 core consumer·RT allocator·MSRV influx-only·fmt/clippy/API docs(-D warnings) Pass. 소스114개 commit blob 일치, 새 Miri/native 실행 없음.
- 실제 DB routed 시험은 원 capture→DB API ACK→원 completion→query receipt/ns와 실제 type failure(400/422·attempts1)→Unknown·guard Drop/raw 회수를 대조했다.

## 파일 영향 관측

구성·수치(비공개 원시 근거: `imported_context/task_019_routed_2026-11-01/file-isolation.json`)·실행 원문 뷰(비공개 원시 근거: `imported_context/task_019_routed_2026-11-01/routed.log`). 동일 원본6건·raw2슬롯·file required/DB optional·DB retained8B·queue1/pending0, OS 임시 namespace 실제 파일로 비교했다.

| DB 조건 | file records | file bytes | file phase ns | DB in-flight |
|---|---:|---:|---:|---:|
| route off | 6 | 3508 | 238318 | 0 |
| 정지·포화 | 6 | 3508 | 203995 | 1 |

순서·원문·metadata와 file Written을 검증했고, DB raw8B는 local socket EOF/런타임 종료·Unknown 게시·Attempt Drop 후 회수했다. 이 수치로 지연 우월/처리량/≤5% 비용/RT/장기 수락을 판정하지 않는다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | [routed.rs](../../../crates/dot-logger/src/adapters/influxdb/routed.rs) | 명시 mapping·원 guard/IO·DB completion 연결 |
| 수정 | core influxdb/mod.rs | 새 public API export |
| 생성 | [tc_008_influx_routed.rs](../../../crates/dot-logger/tests/tc_008_influx_routed.rs)·support/influx_routed.rs | raw/완료/실패·codec·실제 파일 격리 시험 |
| 수정 | [completion/tests.rs](../../../crates/dot-logger/src/completion/tests.rs)·tc_008_influx_live.rs | deterministic Busy·실제 ACK/failure/원 receipt |
| 수정 | [client transport test](../../../crates/dot-logger-client/tests/transport.rs) | proxy 자식 종료·합류 |
| 수정 | README | worker/원본/게시·파일 관측/한계 |

## 클래스(타입) 변경표

| 구분 | 타입 | 역할 |
|---|---|---|
| 생성 | RoutedWriter | frozen DB/schema·일반 worker 동기 한 건 실행 |
| 생성 | ProjectionSchema·ProjectionEncoding·ProjectionError | 원 schema/명시 codec·고정 오류 |
| 생성 | DeliveryAttempt | 원 guard/native 결과·completion 게시·제한된 복구 |
| 생성(시험 내부) | Temp | 자기 파일 namespace 수명/정리 |

## 남은 조건

TASK-019 S1/S2/S3 로컬 기능만 수락한다. TASK-020/026 실앱·화면·운영/설정 전환·PC/RT/RSS/정량 성능/UPS·장기·공개 후 native(T3)는 잔여다. Task In Progress·TC/MS 전체 Partial·Gate4/5 Not Ready 유지. TASK-018은 별도 승인 대기, 다음 개발 후보 TASK-020은 이번 요청에서 착수하지 않았다.

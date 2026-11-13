# TASK-019 — InfluxDB 어댑터와 충돌·장애

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-006 |
| Story | US-001·002·003·004 |
| 요구사항 | FR-004 NFR-003 NFR-005 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-006·TASK-007 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료. TASK-006/007의 로컬 구현·위임 수락을 사용하며 공개 후 native/실측 잔여 상태의 후속 개발 승인을 따른다.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: [`crates/dot-logger/src/adapters/influxdb/`](../../../crates/dot-logger/src/adapters/influxdb/mod.rs), [`tc_008_influxdb.rs`](../../../crates/dot-logger/tests/tc_008_influxdb.rs).

첫 실패/리뷰 조건: **동일 timestamp 충돌·timeout 이후 unknown·부분 batch 오류**.

- [x] S1 기능 부재 compile Red 및 이종 수치 field type 충돌 재현 Red 확인.
- [x] S1 최소 구현 후 TC-008 로컬 TCP 대역 하위 시나리오를 검증했다. 실제 DB/TC 전체는 미완료다.
- [x] S1 영향 회귀·두 Rust·선택 feature/독립 core 소비자·문서 일치 검증(EV-037).

## 완료 기준

- [x] 로컬 범위: 실제 OSS2.9.1 write/query·유한 retry·secret 비노출·DB 정지/포화의 파일 기록 영향 관측(EV-038/039). 정량 성능/다른 환경 수락은 아래 잔여다.
- [x] S1/S2/S3 source·환경·명령·결과/미실행을 EV-037~039에 기록하고 MS-006 추적 notes에 연결했다.
- [x] 로컬 실패/포화·취소/종료·예산·원본/실제 IO·completion Busy 경계 검증(EV-039).
- [x] S1/S2/S3 로컬 구현은 기존 Human 위임 범위로 AI 수락·REV-035~037 Closed. 통합/환경 수락·native는 별도 잔여다.

테스트 정의: [TC-008](../../05_verification/test_case.md#tc-008). 제품 코드·로컬 시험: [EV-037](../../05_verification/verification_evidence.md#ev-037). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.


## S1 유한 projection·HTTP write — 2026-11-01 #010

[API §14.6](../../03_design_artifacts/interface_contract.md#146-task-019-s1-influxdb-v2-유한-write--2026-11-01)·[REV-035](../../05_verification/reviews/REV-035_TASK-019_INFLUX_WRITE.md). ADR-009의 v2/선택 feature/일반 경로 방향 안의 구현이다. OQ-004의 실제 DB 호환 미확정은 v2 선택과 모순되지 않으며 S2 검증으로 유지한다. 배포 버전 채택·요구/오류 의미 변경은 이번에 결정하지 않았다.

- 고정 session/measurement/device/채널·typed fields·원 real ns/receipt, 유한 batch/event/재시도/TTL·공유 external 예약.
- 명시 `RejectNonIncreasing`: 호스트가 새 독점 namespace를 공급해야 한다. 같은 series의 equal/older 시각을 생산자/배치 간 거절한다. out-of-order 지원/기존 DB 충돌 탐지는 아직 없다. 원 timestamp 변경이나 recipe/sequence tag 추가 없음.
- 전체 preflight 실패 시 IO/상태 변경 없음. 시도한 batch는 Unknown/취소 뒤에도 high-water 유지. immutable encoded body 재시도·partial/timeout/transport Unknown·204 API ACK.
- token 민감 header·redacted 오류·HTTPS/literal-loopback HTTP·redirect/proxy/암묵 retry off. body와 예약은 실제 마지막 HTTP body owner까지 유지하며 다음 batch는 Busy.
- 신규10개 TCP/입력/예산 시험+1개 body 수명 단위 시험. 실제 DB server·write/query·파일/RT 비용 수락이 아니다.

| 단계 | 상태 | 다음 결과 |
|---|---|---|
| S1 유한 typed projection·HTTP write | 로컬 구현/검증·위임 수락 | EV-037/REV-035 Closed |
| S2 실제 DB 및 제한 수치 query | 로컬 구현/실제 DB 기능 검증·위임 수락 | EV-038/REV-036 Closed; OSS2.9.1 Linux amd64 digest pin·query/receipt/ns·partial/동일 시각 대조 |
| S3 Router Delivery·완료·파일 격리 | 로컬 기능/파일 영향 관측·위임 수락 | EV-039/REV-037 Closed; 원 guard·실제 로컬 IO 종료·completion/Busy·DB 정지/포화 중 MCAP6건 원문 보존 |
| 통합/환경 수락 | 미완료 | TASK-020/026 실앱·TASK-022 실제 PC·공개 후 native(T3) |

S1은 Task 전체 완료가 아니며 MS-006 In Progress·TC-008 Partial·Gate4/5 Not Ready를 유지한다.


## S2 제한 수치 query·실제 DB 대조 — 2026-11-01 #011

[API§14.7](../../03_design_artifacts/interface_contract.md#147-task-019-s2-제한-수치-query실제-db-검증--2026-11-01)·[EV-038](../../05_verification/verification_evidence.md#ev-038)·[REV-036](../../05_verification/reviews/REV-036_TASK-019_NUMERIC_QUERY.md). 승인된 v2/제한 query 방향 안의 구현이며 사용자 기존 DB를 변경하지 않았다.

- [x] OSS2.9.1/Linux amd64 image digest pin·health v2.9.1 확인. 임시 loopback port·tmpfs·2CPU/512MiB·128pids·일회 token을 사용하는 자기 소유 container만 생성/정리한다. 정확한 pin·재현 명령은 API/EV 근거에 있다.
- [x] 원 session/measurement/device/channel·receipt·Unix ns·typed int/float/bool 조회. half-open 범위·행≤1024·응답≤1MiB·기한/취소, 별도 같은-authority external 예약·원 Page/body 수명, 고정 numeric CSV schema·late error/부분 응답 거절.
- [x] 기능 부재 compile Red·실제 query dialect 불일치 Red → 명시 JSON annotations와 엄격한 parser로 Green. 신규 TCP/실패/예산9개, 실제 DB3개: typed write/query·i64/u64/u32 경계/정확 ns·같은 시각 병합과 adapter 거절·partial type conflict 뒤 유효 bool 행 보존.
- [x] 두 Rust workspace394개씩(실제 DB3개는 일반 회귀에서 ignore), 별도 실제 DB 시험3개씩·16 feature/독립 core·RT allocator·fmt/clippy/API docs 검증. 최종 명령/형상은 EV-038을 따른다.
- [x] S3 원 Delivery/schema projection·DatabaseAcknowledged/Unknown·원본/IO 수명과 로컬 파일 영향 관측은 EV-039/아래 S3에서 완료했다.
- [x] (2026-11-13 이관: 실앱 TASK-026·native TASK-024(T3)·성능 TASK-023) 통합/환경 수락: TASK-020/026 실앱·PC/RT/RSS/비용·공개 후 native(T3), TC-008/MS 전체·Gate4/5 미완료.

이후 브리핑에서는 위 표의 S1/S2/S3/통합·환경 **전체 단계**를 완료/현재/미착수/대기로 구분해 보여준다. 이는 사용자 선호를 [소통 규칙](../../01_cowork_protocol/communication_convention.md)에 반영한 것이다.


## S3 원본·완료·파일 격리 — 2026-11-01 #012

[API§14.8](../../03_design_artifacts/interface_contract.md#148-task-019-s3-원-deliverydb-완료파일-격리--2026-11-01)·[EV-039](../../05_verification/verification_evidence.md#ev-039)·[REV-037](../../05_verification/reviews/REV-037_TASK-019_ROUTED_COMPLETION.md).

- [x] 원 schema ID/version과 I64Le/F64Le/BoolByte/Utf8Event를 명시 등록, 초기 session/revision·DB target 계약 확인. 원 receipt·metadata.real_ns/bytes를 사용하고 자동 raw/image 해석·시각 변경 없음.
- [x] host 일반 worker의 동기식 한 건 실행·전용 current-thread runtime, 로컬 IO/런타임 종료까지 원 Delivery 유지. nested runtime·foreign target은 미시도 guard 복구, terminal/실제 시도는 recovery 거절. OS/DNS 정리는 TTL을 넘을 수 있다.
- [x] native 결과/정해진 Update/원 guard를 DeliveryAttempt에 유지. publish만 completion을 갱신, Busy이면 같은 owner로 게시만 재시도. ACK/실패/미시도 취소/Unknown·미게시 Drop 구분과 실제 guard Drop 후 raw 회수.
- [x] DB off/정지·포화 동일 원본6건을 실제 MCAP에 기록·독립 read 대조. 각6건/3508B·metadata/원문/순서 보존, DB in-flight1·raw8B/추가5건 Capacity drop을 분리 표시. 파일 완료가 DB 취소/ACK보다 먼저 이루어짐을 검증. file phase ns는 기능 관측으로 정량 성능 수락이 아니다.
- [x] 신규 routed6개+completion Busy1개, 두 Rust workspace각401 Pass/실제 DB4개 명시 ignore, 별도 OSS2.9.1 harness각4개 Pass. 실제 원 capture→DB ACK/원 completion→query receipt/ns와 실제 DB type failure→Unknown/원본 회수도 대조했다.
- [x] 전체 회귀에서 발견한 기존 proxy fixture의 자식 업로드 작업 미합류를 수정했다. 자식까지 abort/await한 뒤 기존 예약0 검사를 유지, 두 Rust각5회+최종 workspace 회귀 Pass. 제품 remote transport 동작은 변경하지 않았다.

**로컬 기능 S1/S2/S3는 수락 완료**다. Task In Progress·TC-008 Partial·MS-006/Gate4/5 미완료는 통합/환경·정량 성능/장기·공개 후 native가 남았기 때문이다. TASK-020/026 등의 기능/실앱과 native T3를 별도 추적하며 임의 면제하지 않는다. 다음 개발 후보는 TASK-020이며 이번 S3 요청으로 자동 착수하지 않았다. TASK-018은 별도 FR-015 범위 승인 대기로 미착수다.

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- native·다른 OS: [TASK-024](TASK-024.md)(T3, GAP-004)
- 실사용처 적용: [TASK-026](TASK-026.md)(마지막 단계, 일정 판단 밖)

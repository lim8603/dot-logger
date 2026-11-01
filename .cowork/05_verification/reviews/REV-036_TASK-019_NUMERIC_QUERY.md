# REV-036 — TASK-019 S2 제한 수치 query·실제 DB 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #011 / AI 개발 세션 자가 점검. [code-review 스킬](../../../.agents/skills/code-review/SKILL.md) 적용; 독립 리뷰 전용 세션과 구분한다.
- 대상: 소스 `7d2076a`, numeric query API/같은 session 예약·고정 DB harness·TC-008·README/선택 의존 연결.
- 판정: **Pass — S2 로컬 구현/실제 DB 기능 검증 범위**. 미해결 P1/P2/P3 없음.
- 상태: **Closed** — Open → In Progress → Resolved → 기존 위임에 따른 로컬 수락. Task/MS/TC 전체는 미완료.
- 근거: [EV-038](../verification_evidence.md#ev-038)·[최종 명령/소스111개 hash·commit blob 대조](../../06_evolution/imported_context/task_019_query_2026-11-01/results.json).

## 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견/처리 제안 | 판정 | 상태 |
|---|---|---|---|---|---|
| RV-075 | P2 | numeric query 요청/dialect | plain Flux 요청의 실제 OSS2.9.1 CSV는 datatype annotation이 없어서 엄격한 parser가 모두 Corrupt로 거절. query dialect를 명시하여 실제 엔진 왕복 필요 | 수용·JSON explicit annotations, 실제 Red→Green | Closed |

## [리뷰어] 검토

- **RT/동시성:** 일반 host runtime 전용 API·유한 파서, 자체 unsafe/RT 경로 변경 없음. Reservation의 내부 `reserve_extra`는 기존 원자 ledger를 재사용해 원 session과 동일 external partition에 별도 charge한다. 전역/별도 ledger로 상한을 우회하지 않는다.
- **입력/Flux:** 고정 measurement/device/channel/session/schema·half-open real ns·양수 row/bytes/time budget. 임의 Flux/이벤트·source/mono clock 추정 변환 없음. quote/backslash escape와 interpolation `${` 입력 거절. user numeric/identity는 정수형으로 삽입하며 secret/endpoint를 query/result에 넣지 않는다.
- **응답/진단:** 명시 dtype/header·고정8열 배열·원 session/유효 receipt/정확 ns·strictly increasing 범위 검증. JSON dialect로 타입을 요구한다. 전체 EOF 뒤만 Page 성공, max_rows+1로 truncation 검출, global group 뒤 limit. late error/partial/chunked 초과/truncation/NaN/외부 scope·중복 행은 전체 실패. 서버 error body/URL/token은 결과·Debug 진단에 넣지 않는다.
- **수명/예산:** body/response/row·유한 scratch를 동일 authority에서 별도 예약. HTTP-owned body와 NumericPage가 자기 Arc<Reservation>을 소유해 Writer/public budget handle Drop이 살아 있는 bytes/result 예약을 회수하지 않는다. 원 writer body가 살아 있는 S1 Busy 계약은 유지한다. HTTP/TLS/runtime/OS 메모리는 별도 headroom, remote query engine RAM/CPU·IO hard cap/rollback을 추정하지 않는다.
- **출처/API:** NumericPage의 NumericScope/원 Query가 빈 결과/Writer Drop 뒤에도 session/measurement/device/channel·범위/상한을 보존한다. int/uint receipt를 f64로 경유하지 않는다. DB query 관측을 completion 갱신이나 disk 내구성으로 승격하지 않는다.
- **실제 DB/보안:** OSS2.9.1 Linux amd64 digest/health 확인, 2CPU/512MiB/128pids·loopback 임시 port·독립 tmpfs. random token은 setup HTTP/child env로만 전달하며 로그에서 redaction, evidence에 저장 없음. finally는 자기 container/volume만 제거하고 cleanup 실패는 오류다. 기존 DB/volume/운영 설정/계정/CI를 변경하지 않는다.
- **회귀/설계:** serde_json1.0.154는 기존 pin/lock의 선택 직접 의존이며 core/default client에 추가 강제 의존 없음. ADR-009의 v2/제한 query 범위 내 세부 구현·기능 시험 pin, 운영 DB upgrade/신규 기술 계열 결정 아님. OQ-004 중 이 버전의 기본 DB 호환만 검증, 다른 DB/OS/실앱/성능·S3 잔여 유지.

## [처리담당] RV-075 판정·처리

- **수용:** 실제 첫 live3개 모두 Corrupt 실패. 독립 synthetic protocol probe의 CSV가 annotation 없이 header/data만 있음을 확인했다. 기존 검증 강도를 낮추지 않고 `/api/v2/query`의 JSON `dialect.annotations=[datatype,group,default]`/header를 명시했다. 기존 serde_json pin을 influxdb 선택 feature에 연결했다.
- **검증:** [수정 전 live](../../06_evolution/imported_context/task_019_query_2026-11-01/live-1.96/live.log)·[실제 bare Flux CSV](../../06_evolution/imported_context/task_019_query_2026-11-01/protocol-probe/protocol.csv)·[최종 annotated CSV](../../06_evolution/imported_context/task_019_query_2026-11-01/final-live-1.96/protocol.csv). Rust1.96/MSRV1.85 별도 격리 DB각3개 Pass/cleanup0. 원 ns·i64 MIN/MAX·u64 MAX/u32 MAX receipt·typed value 대조, duplicate DB merge/adapter 거절, partial type failure 뒤 valid bool 행 잔존 확인.
- **최종 회귀:** 두 Rust workspace각394 Pass/실제 DB3개 명시 ignore, 신규 TCP/한계9개. 개발 Rust16features/독립 core consumer/RT allocator·MSRV influx-only·fmt/clippy(-D warnings)/core-client API rustdoc(-D warnings) Pass. 모든 최종 source111개 commit blob 일치. Miri/native 새 실행 없음.
- **수락 경계:** S2 기능 단위만 Closed. S3 Router Delivery/projection·DatabaseAcknowledged/Unknown·원본/진행 IO 수명·DB 실패의 파일 영향 계측, 통합/실측/native/Task·TC 전체·Gate4/5는 미완료.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | [numeric.rs](../../../crates/dot-logger/src/adapters/influxdb/numeric.rs) | bounded numeric query·scope/result·strict CSV·기한/취소 |
| 수정 | [influxdb/mod.rs](../../../crates/dot-logger/src/adapters/influxdb/mod.rs)·[query/mod.rs](../../../crates/dot-logger/src/query/mod.rs) | API export·동일 session 예약 helper |
| 생성 | [tc_008_influx_query.rs](../../../crates/dot-logger/tests/tc_008_influx_query.rs)·[tc_008_influx_live.rs](../../../crates/dot-logger/tests/tc_008_influx_live.rs) | 대역9·실제 DB3개 |
| 생성 | [check-influxdb.py](../../../scripts/check-influxdb.py) | digest/health pin·격리 setup/정리·secret 비저장 |
| 수정 | core Cargo.toml·Cargo.lock·README | 기존 선택 serde_json pin·사용 범위/재현 명령 |

## 클래스(타입) 변경표

| 구분 | 타입 | 역할 |
|---|---|---|
| 생성 | NumericQuery·NumericScope | 명시 range/한계와 불변 source identity |
| 생성 | NumericPoint·NumericValue·NumericPage | 원 receipt/ns/typed value·완전 range·결과 예약 owner |
| 생성 | QueryError | 정적/redacted query 실패 |
| 수정 | Writer | query_numeric 메서드 추가 |
| 수정(내부) | Reservation | 원 ledger authority에서 reserve_extra |

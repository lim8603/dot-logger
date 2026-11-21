# REV-035 — TASK-019 S1 InfluxDB v2 유한 write 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #010 / AI 개발 세션 자가 점검. [code-review 스킬](../../../.agents/skills/code-review/SKILL.md) 적용, 독립 리뷰 세션과 구분한다.
- 대상: 소스 `03c6c35`, 선택 `influxdb` 모듈·TC-008 로컬 대역 시험·Cargo·README.
- 종합 판정: **Pass — S1 로컬 구현/검증 범위**. 미해결 P1/P2/P3 없음. Task 전체는 In Progress.
- 상태: **Closed** — Open → In Progress → Resolved → 2026-10-31 위임 범위 로컬 수락. 실제 DB/query·Delivery·실앱/환경 수락을 대체하지 않는다.
- 검증: [EV-037](../verification_evidence.md#ev-037)·명령/소스/commit blob 대조(비공개 원시 근거).

## 요약표

| ID | 중요도 | 위치/계약 | 문제/처리 제안 | 판정 | 상태 |
|---|---|---|---|---|---|
| RV-073 | P2 | Writer::encode / field type | 한 measurement에서 채널 tag만 달리하여 같은 value field의 int/float/bool 타입을 혼합할 수 있음. field key를 타입별로 분리 | 수용·integer/float/boolean/event로 고정, Red→Green | Closed |
| RV-074 | P2 | HTTP body/외부 예약 수명 | 요청 future/Writer Drop 뒤 HTTP가 body를 소유할 때 writer 단독 예약으로는 수명/다음 배치 상한을 증명할 수 없음 | 수용·body owner의 Arc 예약/Weak 단일 슬롯, 마지막 body 해제까지 유지 | Closed |

## [리뷰어] 점검

- RT/동시성: 새 API는 일반 경로·명시 host runtime 전용. RT/unsafe/원자 순서/기존 routing과 completion 코드는 변경하지 않았다. 새 busy 상태는 단일 `&mut Writer`와 `Weak<BodyData>`의 실제 소유권을 사용하며 반복 spin/무제한 retry 없음.
- 충돌/범위: 새 독점 namespace는 호스트 의무다. 기존 DB 내용/재시작·다른 writer는 조사하지 않는다. session/channel별 high-water가 equal/older 시각을 모든 producer/batch에 대해 거절한다. out-of-order도 명시 거절, 원 timestamp/receipt 유지, tag cardinality는 초기 device/channel 집합으로 한정한다.
- preflight/오류: 전체 batch 검증 뒤 시도 직전 high-water 게시. 실패한 preflight는 상태/IO 변경 없음. attempted Unknown/취소/호출 Drop 뒤 시각 재사용 금지. 204 API ACK, partial/timeout/응답 단절 Unknown, 이전 uncertainty 뒤401도 Unknown 유지. 요청 body는 immutable clone이며 TTL·retry count·Retry-After 상한이 있다.
- 보안: HTTPS/literal-loopback HTTP만 허용, URL credentials/path/query/fragment·redirect/proxy/암묵 retry 없음. Config/Writer Debug 미제공, token sensitive header와 정적/redacted 오류 사용. 응답 error body를 읽거나 로그/결과에 복사하지 않는다.
- 예산/수명: fixed body 상한×2+32KiB 예약은 QueryBudget의 공유 external 한도에 합산된다. HTTP body는 `Bytes::from_owner`의 실제 마지막 소유자와 예약을 함께 유지한다. 이전 body가 살아 있으면 Busy라 여러 body가 한 예약을 공유하며 누적되지 않는다. HTTP/TLS/runtime/OS headroom은 별도이며 전체 RSS/원격 IO rollback/실제 reader 종료를 주장하지 않는다.
- 의존/설계: `bytes`1.12.1은 기존 lockfile 버전을 influxdb 선택 직접 의존으로 연결했으며 core consumer graph에 네트워크/runtime를 추가하지 않는다. ADR-009 방향의 내부 구현, DB 배포 계열/버전 신규 결정 없음. OQ-004 실제 DB 호환·Gate4/5 잔여 유지.

## [처리담당] 판정/검증

- **RV-073 수용:** `Writer::encode`에서 integer/float/boolean/event field key 사용. `heterogeneous_channels_use_distinct_field_types_in_one_measurement`가 기존 value key에서 실패한 Red(비공개 원시 근거)를 보존하고 최종 통과했다. DB 엔진의 실제 type-conflict 동작은 S2에서 대조한다.
- **RV-074 수용:** `BodyData`/`BodyOwner`가 `Arc<Reservation>`을 HTTP-owned bytes에 연결하며 Writer는 `Weak`로 이전 body 유무를 검사한다. 순수 수명 시험에서 local owner 해제 뒤에도 transport clone이 남으면 예약4096B/원문/slot을 유지하고 마지막 clone Drop 뒤0을 확인했다. 실제 TCP 진행 중 취소·timeout은 Unknown, 같은 timestamp 재제출은 Collision, runtime/peer 종료 뒤 공유 예약0을 확인했다. 사전 수명 검토 보강이며 기존 제품에서 누수를 관측했다고 주장하지 않는다.
- 기능 부재 compile Red, 신규10개 TCP/입력/상한 시나리오+1개 body 수명 시험 Pass. 두 Rust workspace각385, 개발 Rust16 feature 조합/독립 core consumer/RT allocator·fmt/clippy/core·client API rustdoc(-D warnings) Pass. 전체 회귀 뒤 테스트만 취소+실제 timeout으로 확장했고 두 Rust 해당10개와 fmt/clippy를 재실행했다(생산 source 불변).
- 새 unsafe 없음·Miri/native 미실행. 실제 DB version pin/제한 수치 query·write/query·Router Delivery/완료·파일 장애 격리 계측·실앱/실측/native는 후속이며 S1 Pass를 확대하지 않는다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | [influxdb/mod.rs](../../../crates/dot-logger/src/adapters/influxdb/mod.rs) | 유한 typed projection·충돌/HTTP/예산·body 수명 |
| 생성 | [tc_008_influxdb.rs](../../../crates/dot-logger/tests/tc_008_influxdb.rs) | TCP·동일 batch/typed fields·취소/timeout·부분/상한 시험 |
| 수정 | Cargo.toml·Cargo.lock·adapters/mod.rs | 기존 bytes 버전의 선택 의존·모듈 연결 |
| 수정 | [README](../../../README.md) | 사용 경계·fresh/exclusive namespace·실제 DB 미검증 |

## 타입 변경표

| 구분 | 타입 | 역할 |
|---|---|---|
| 생성 | Writer·Config·Channel·Kind·CollisionPolicy | 명시 유한 writer와 초기 고정 schema/정책 |
| 생성 | Point·Value | 원 real ns/receipt와 명시 typed field |
| 생성 | WriteReport·Outcome·Stop·Error | API ACK/불명/미시도·기한/취소·정적 오류 |
| 생성(내부) | Series·Limited·BodyData·BodyOwner | high-water·bounded encode·실제 HTTP body/예약 소유권 |

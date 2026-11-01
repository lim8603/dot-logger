# [REV-038] TASK-019 InfluxDB v2 어댑터 독립 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-11-01 / AI 독립 코드리뷰 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 코드리뷰. REV-035~REV-037은 개발 세션별 자가 점검이었으므로, TASK-019 InfluxDB v2 어댑터 전체(S1 유한 Write, S2 제한 수치 Query, S3 원 Delivery·완료·파일 격리)의 구현 형상에 대해 독립적 관점에서 심층 검토를 수행했다. 제품 코드는 전혀 수정하지 않았다.
- **대상 형상:** 커밋 `5bd70dc` 및 TASK-019 S1~S3 구현 파일 전체
  - InfluxDB 어댑터 코어:
    - [`crates/dot-logger/src/adapters/influxdb/mod.rs`](../../../crates/dot-logger/src/adapters/influxdb/mod.rs)
    - [`crates/dot-logger/src/adapters/influxdb/numeric.rs`](../../../crates/dot-logger/src/adapters/influxdb/numeric.rs)
    - [`crates/dot-logger/src/adapters/influxdb/routed.rs`](../../../crates/dot-logger/src/adapters/influxdb/routed.rs)
    - [`crates/dot-logger/src/adapters/mod.rs`](../../../crates/dot-logger/src/adapters/mod.rs)
    - [`crates/dot-logger/src/query/mod.rs`](../../../crates/dot-logger/src/query/mod.rs)
  - 검증 테스트 및 보조 도구:
    - [`crates/dot-logger/tests/tc_008_influxdb.rs`](../../../crates/dot-logger/tests/tc_008_influxdb.rs)
    - [`crates/dot-logger/tests/tc_008_influx_query.rs`](../../../crates/dot-logger/tests/tc_008_influx_query.rs)
    - [`crates/dot-logger/tests/tc_008_influx_live.rs`](../../../crates/dot-logger/tests/tc_008_influx_live.rs)
    - [`crates/dot-logger/tests/tc_008_influx_routed.rs`](../../../crates/dot-logger/tests/tc_008_influx_routed.rs)
    - [`crates/dot-logger/tests/support/influx_routed.rs`](../../../crates/dot-logger/tests/support/influx_routed.rs)
    - [`scripts/check-influxdb.py`](../../../scripts/check-influxdb.py)
- **정적 검증 결과:**
  - `cargo fmt --check`: Pass (포맷 완전 준수)
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: Pass (경고 0건)
  - `cargo test --workspace`: Pass (단위/통합/문서 테스트 100% 통과)
  - `cargo test -p dot-logger-rt-allocation`: 15/15 Pass (RT 수집/대여/반환 경로의 할당/해제 0 불변식 완벽 보존)
  - Safe Rust 100% (신규 구현 내 `unsafe` 블록 0건)
- **종합 판정:** **Pass** (P1 0건, P2 0건, P3 2건). RT 제약의 완전한 격리, 100% Safe Rust, 철저한 단조 시계 충돌 방어 및 선반영, 자원 수명 안전성 및 파일 장애 격리 계약 엄격 준수.
- **상태:** **Closed** (2026-11-01 #014 개발 세션에서 P3 문서 보강·검증 후 기존 위임으로 수락)

---

### 확인한 긍정 사항

1. **RT(실시간) 경로의 철저한 격리 및 무할당 불변식 보존:**
   - InfluxDB v2 어댑터(`Writer`, `RoutedWriter`, `NumericQuery`) 전체 로직은 명시적인 호스트 일반 워커(General-path worker) 전용으로 설계됨.
   - `RoutedWriter::write_delivery` 진입 시 `tokio::runtime::Handle::try_current().is_ok()` 가드를 통해 중첩 비동기 런타임 및 비정상 스레드 호출을 원천 차단함.
   - RT 할당 검증 테스트(`dot-logger-rt-allocation`) 15개 항목 전체 통과로 RT 핫패스의 동적 메모리 할당 및 해제 0 원칙이 확고히 유지됨을 확인함.
2. **100% Safe Rust 및 정밀한 버퍼 수명/소유권 관리:**
   - 어댑터 코드 전체에 `unsafe` 블록이 단 하나도 존재하지 않음.
   - `bytes::Bytes::from_owner(BodyOwner(Arc<BodyData>))` 구조를 적용하여, HTTP 전송 계층이 버퍼를 소유하는 동안 `Arc<Reservation>`이 보존되고 전송 완료 시 자동 해제되도록 함.
   - `Writer.pending: Weak<BodyData>`를 통해 이전 전송 중인 버퍼가 남아있는 경우(`upgrade().is_some()`) `Error::Busy`를 즉시 반환하여 다중 버퍼 누적 및 메모리 초과를 방지함.
3. **선반영(Preflight) 타임스탬프 충돌 방어 및 단조성 보장:**
   - 첫 HTTP 전송 비동기 대기(`await`)에 진입하기 직전에 `series.last`에 전송 예정 시각을 선반영하여, 전송 중 취소되거나 응답이 불확실(`Unknown`)하더라도 동일하거나 과거 타임스탬프가 재제출되는 것을 원천 차단함.
   - 각 채널별 타입 분리(`integer`, `float`, `boolean`, `event`)를 적용하여 동일 measurement 내 채널 간 필드 타입 충돌을 방지함(RV-073 기반영).
4. **동기 어댑터 내 로컬 런타임 격리 및 완전 정리:**
   - `RoutedWriter::write_delivery`는 호출마다 전용 단일 스레드 런타임(`new_current_thread`)을 생성하여 작업을 수행하고, `drop(runtime)`을 통해 로컬 IO 작업이 완전히 중단된 것을 확인한 후에만 원본 `Delivery` 가드를 반환함.
   - 백그라운드 태스크나 좀비 스레드를 남기지 않으며, 불완전 종료로 인한 원본 레코드 조기 회수 버그를 방지함.
5. **파일 출력 장애 격리 실증:**
   - InfluxDB 정지·포화 상황에서도 코어 로컬 파일 출력(MCAP 6건, 3,508B)이 누락이나 지연 없이 완벽히 보존됨을 검증함.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-079 | P3 | [`dot-logger/src/adapters/influxdb/routed.rs#L80-L128`](../../../crates/dot-logger/src/adapters/influxdb/routed.rs#L80-L128), `RoutedWriter::new` | `RoutedWriter::new` 실패 시 단일 `ProjectionError::InvalidConfig`만 반환되므로, 등록 실패 원인(세션 불일치, 타깃 미지원, 채널 수/타입 불일치, writer busy)을 사용자가 파악하기 어려움 | `RoutedWriter::new`의 상단 rustdoc에 등록 성공을 위한 선행조건(Preconditions) 5가지를 명시하여 호출자의 사전 유효성 검증을 안내 | 수용·문서 보강 | Closed |
| RV-080 | P3 | [`dot-logger/src/adapters/influxdb/numeric.rs#L124-L131`](../../../crates/dot-logger/src/adapters/influxdb/numeric.rs#L124-L131), `NumericQuery` 유효 범위 | `NumericQuery::from_ns`와 `to_ns`의 엄격한 경계 조건(`from_ns > i64::MIN + 1`, `to_ns < i64::MAX`)의 사유가 Flux 변환 방어 목적임을 사용자가 알기 어려움 | `NumericQuery` 구조체의 `from_ns`, `to_ns` 필드에 InfluxDB Flux `time()` 변환 한계 및 방어적 경계 조건에 대한 rustdoc 설명 보강 | 수용·문서 보강 | Closed |

---

## 3. 상세 리뷰 내역

### RV-079: `RoutedWriter::new` 선행조건(Preconditions) rustdoc 가이드 보강 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/adapters/influxdb/routed.rs#L80-L128`](../../../crates/dot-logger/src/adapters/influxdb/routed.rs#L80-L128)
- **분석 내용:**
  - `RoutedWriter::new(writer, completion, target, schemas)`는 다음 조건들을 검증하고 하나라도 불일치하면 `ProjectionError::InvalidConfig`를 반환한다:
    1. 대상 `target`이 `completion.targets()`에 존재해야 함
    2. `writer.config.session_id == completion.session_id()` (세션 일치)
    3. 대상 타깃 계약의 `completion == Boundary::DatabaseAcknowledged` 및 지원 목록 포함
    4. `writer.pending.upgrade().is_none()` (이전 HTTP 전송 버퍼 미점유)
    5. `schemas.len() == writer.config.channels.len()` 및 모든 스키마 채널 ID/타입 일치, 중복 없음
  - 에러 열거형이 `InvalidConfig` 단일 variant이므로, 호출자가 구성 실수 시 어떤 계약 위반으로 실패했는지 역추적하기 위해 코드를 직접 확인해야 한다.
- **처리 제안:**
  - `RoutedWriter::new` 상단 rustdoc의 `# Errors` 또는 `# Preconditions` 섹션에 위 5대 필수 선행조건을 명시하여 호출자(상위 호스트 설정자)가 사전에 구성을 바르게 검증할 수 있도록 돕는다.

#### [처리담당]
- **판정 결과:** 수용·Closed.
- **처리 내용:** `RoutedWriter::new` rustdoc에 5개 선행조건과 metadata 예약 실패 조건을 명시했다. 새 오류 variant나 동작 변경은 없다.
- **검증 근거:** 소스 `d16c7e5`; `RUSTDOCFLAGS='-D warnings' cargo doc --locked -p dot-logger --all-features --no-deps`, `cargo fmt --check` exit0. [rustdoc 원문](../../06_evolution/imported_context/task_020_s1_2026-11-01/rev038-doc.log).

---

### RV-080: `NumericQuery` 타임스탬프 유효 범위 제약 근거 rustdoc 명시 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/adapters/influxdb/numeric.rs#L124-L131`](../../../crates/dot-logger/src/adapters/influxdb/numeric.rs#L124-L131), [`crates/dot-logger/src/adapters/influxdb/numeric.rs#L6-L18`](../../../crates/dot-logger/src/adapters/influxdb/numeric.rs#L6-L18)
- **분석 내용:**
  - `query_numeric` 메서드는 `query.from_ns <= i64::MIN + 1 || query.to_ns == i64::MAX || query.from_ns >= query.to_ns`인 경우 `QueryError::InvalidInput`을 즉시 반환한다.
  - 이 제약은 Flux 스크립트의 `time(v: {from})` / `time(v: {to})` 평가 시 InfluxDB 엔진 내부의 RFC3339 변환 오버플로 및 정수 오버플로를 사전에 차단하기 위한 필수적인 방어 로직이다.
  - 현재 `NumericQuery` struct 필드 주석(`Inclusive Unix nanoseconds, without clock conversion`)에는 이 구체적인 허용 하한/상한 범위가 기재되어 있지 않다.
- **처리 제안:**
  - `NumericQuery::from_ns`에 `> i64::MIN + 1`, `NumericQuery::to_ns`에 `< i64::MAX` 범위 제약이 적용됨을 rustdoc 주석으로 명시하여 클라이언트가 쿼리 범위 생성 시 `InvalidInput` 에러를 겪지 않도록 가이드한다.

#### [처리담당]
- **판정 결과:** 수용·Closed.
- **처리 내용:** `NumericQuery::from_ns`의 `> i64::MIN + 1`, `to_ns`의 `< i64::MAX`와 `from_ns < to_ns`를 필드 rustdoc에 명시했다. Flux `time()` 변환 경계 방어 목적을 설명하며 기존 입력 검증은 그대로다.
- **검증 근거:** 소스 `d16c7e5`; 위 rustdoc/fmt exit0. 문서만 바꾸므로 동작을 복제하는 새 테스트는 추가하지 않았다.

---

## 4. 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 신규 | [`crates/dot-logger/src/adapters/influxdb/mod.rs`](../../../crates/dot-logger/src/adapters/influxdb/mod.rs) | 유한 InfluxDB v2 Writer, 설정 검증, HTTP 204/재시도/단조 시계 충돌 방어 및 버퍼 소유권 관리 |
| 신규 | [`crates/dot-logger/src/adapters/influxdb/numeric.rs`](../../../crates/dot-logger/src/adapters/influxdb/numeric.rs) | 제한 수치 Flux 쿼리(`query_numeric`), 엄격한 CSV 파서, `NumericPage` 예약 소유권 보존 |
| 신규 | [`crates/dot-logger/src/adapters/influxdb/routed.rs`](../../../crates/dot-logger/src/adapters/influxdb/routed.rs) | `RoutedWriter` 동기 격리 어댑터, `DeliveryAttempt` 원본 가드 보유 및 제한된 복구/게시 |
| 수정 | [`crates/dot-logger/src/adapters/mod.rs`](../../../crates/dot-logger/src/adapters/mod.rs) | `influxdb` 모듈 연결 및 조건부 컴파일 feature 게이트 |
| 수정 | [`crates/dot-logger/src/query/mod.rs`](../../../crates/dot-logger/src/query/mod.rs) | 동일 세션 external reservation 원자적 추가 차징(`reserve_extra`) 헬퍼 추가 |
| 신규 | [`crates/dot-logger/tests/tc_008_influxdb.rs`](../../../crates/dot-logger/tests/tc_008_influxdb.rs) | HTTP 유한 Write 시나리오, TCP 대역, 타임스탬프 충돌 방어, 취소 및 버퍼 수명 검증 |
| 신규 | [`crates/dot-logger/tests/tc_008_influx_query.rs`](../../../crates/dot-logger/tests/tc_008_influx_query.rs) | numeric query 제한, 엄격한 CSV 파싱 실패 시나리오, 행/응답 바이트 상한 검증 |
| 신규 | [`crates/dot-logger/tests/tc_008_influx_live.rs`](../../../crates/dot-logger/tests/tc_008_influx_live.rs) | 실제 InfluxDB OSS 2.9.1 도커 컨테이너 연동 live write/query 통합 검증 |
| 신규 | [`crates/dot-logger/tests/tc_008_influx_routed.rs`](../../../crates/dot-logger/tests/tc_008_influx_routed.rs) | routed adapter 종단 검증, completion Busy 재시도, DB 포화 시 로컬 파일 출력 격리 검증 |
| 신규 | [`scripts/check-influxdb.py`](../../../scripts/check-influxdb.py) | 고정 이미지 digest/health 기반 InfluxDB 테스트 환경 프로비저닝 및 안전한 정리 스크립트 |

---

## 5. 클래스(타입) 변경표

| 구분 | 타입 | 역할 |
|---|---|---|
| 신규 | `Writer` | 단일 소유자 비동기 HTTP Writer (단조 타임스탬프 방어, 100% Safe 버퍼 소유권 관리) |
| 신규 | `Config` | 어댑터 엔드포인트/조직/버킷/채널/타임아웃 고정 설정 (`Debug` 미구현으로 보안 유지) |
| 신규 | `Channel`, `Kind`, `Value`, `Point` | 불변 채널 스키마 및 고정 타입 필드 projection 구조 |
| 신규 | `WriteReport`, `Outcome`, `Stop`, `Error` | 유한 쓰기 결과 리포트 및 명시적 정적 오류 분류 |
| 신규 | `NumericQuery`, `NumericScope` | 고정 템플릿 실시간 범위 쿼리 명세 및 원본 세션 identity 유지 |
| 신규 | `NumericPoint`, `NumericValue`, `NumericPage` | 엄격 검증된 수치 결과 레코드 및 external reservation을 소유하는 페이지 |
| 신규 | `QueryError` | 서버 에러 원문 및 credential 노출 없는 정적 쿼리 에러 |
| 신규 | `RoutedWriter` | 일반 워커 전용 동기 어댑터 (호출 단위 전용 런타임 수명 격리) |
| 신규 | `ProjectionSchema`, `ProjectionEncoding`, `ProjectionError` | 원본 캡처 스키마 매핑 및 고정 변환 명세 |
| 신규 | `DeliveryAttempt` | 원본 `Delivery` 가드를 보존하는 IO 결과 래퍼 (터미널 재전송 방지 및 제한적 복구) |


## 6. 개발 세션 조치·마감 — 2026-11-01 #014

Human 「다음 작업 시작하자」로 반입했다. Open → In Progress → 문서 수정/rustdoc·fmt 확인으로 Resolved → 기존 구현·검증·리뷰 마감 위임으로 Closed 처리했다. 위 리뷰어 본문과 원래 구현 변경표는 리뷰 당시 이력이다. 이번 조치는 `routed.rs`/`numeric.rs`의 rustdoc만 변경했다. 수치상 성능·RT/native/실앱·Task 전체 수락으로 확대하지 않는다.

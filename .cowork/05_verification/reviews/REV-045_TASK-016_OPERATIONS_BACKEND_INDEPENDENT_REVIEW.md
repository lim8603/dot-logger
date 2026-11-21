# [REV-045] TASK-016 운영 backend(status·apply·compare) 독립 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-11-03 / AI 독립 코드리뷰 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 코드리뷰. REV-044는 개발 세션 내 자가 점검이었으므로, TASK-016 운영 backend(`OperationsHost`, live status 지표 보고, 명시적 `ApplyConfig`, 스냅샷 `Compare`, 신규 오류 3종 및 wire/gRPC/CLI 매핑)의 전체 구현 형상에 대해 독립적 관점에서 심층 검토를 수행했다. 제품 코드는 전혀 수정하지 않았다.
- **대상 형상:** 커밋 `7f9fbd5` 및 운영 backend 연계 파일 전체
  - 코어 호스트 백엔드 및 원격 어댑터:
    - [`crates/dot-logger/src/telemetry/operations.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/telemetry/operations.rs)
    - [`crates/dot-logger/src/telemetry/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/telemetry/mod.rs)
    - [`crates/dot-logger/src/adapters/remote/types.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/types.rs)
    - [`crates/dot-logger/src/adapters/remote/server.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/server.rs)
    - [`crates/dot-logger/src/adapters/remote/wire.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/wire.rs)
    - [`crates/dot-logger/src/adapters/remote/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/mod.rs)
    - [`crates/dot-logger/src/adapters/remote/tests.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/tests.rs)
  - 클라이언트 전송 계층 및 CLI:
    - [`crates/dot-logger-client/src/transport.rs`](file:///home/lim8603/src/dot-logger-client/src/transport.rs)
    - [`crates/dot-logger-client/src/codec.rs`](file:///home/lim8603/src/dot-logger-client/src/codec.rs)
    - [`crates/dot-logger-client/tests/transport.rs`](file:///home/lim8603/src/dot-logger-client/tests/transport.rs)
    - [`crates/dot-logger-cli/src/commands.rs`](file:///home/lim8603/src/dot-logger-cli/src/commands.rs)
    - [`crates/dot-logger-cli/src/output.rs`](file:///home/lim8603/src/dot-logger-cli/src/output.rs)
    - [`crates/dot-logger-cli/tests/operations_host.rs`](file:///home/lim8603/src/dot-logger-cli/tests/operations_host.rs)
    - [`crates/dot-logger-cli/tests/cli.rs`](file:///home/lim8603/src/dot-logger-cli/tests/cli.rs)
  - 검증 및 사양 문서:
    - [`crates/dot-logger/tests/tc_009_operations.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/tests/tc_009_operations.rs)
    - [`crates/dot-logger-cli/README.md`](file:///home/lim8603/src/dot-logger-cli/README.md)
    - [`crates/dot-logger-client/README.md`](file:///home/lim8603/src/dot-logger-client/README.md)
- **정적 및 동적 검증 결과:**
  - `cargo fmt --check`: Pass (포맷 완전 준수)
  - `cargo clippy --all-targets -- -D warnings`: Pass (경고 0건)
  - `cargo test`: Pass (22 unittests + 19 fault_storage + 13 tc_001 + 5 tc_010_diagnostics + 16 tc_010_file_check + 11 tc_011_emergency + 17 doctests 전체 통과)
  - `cargo test -p dot-logger --features remote`: Pass (35 unittests + 19 fault_storage + 13 tc_001 + 5 tc_010_diagnostics + 16 tc_010_file_check + 11 tc_011_emergency + 2 tc_009_operations = 101 tests 통과)
  - `cargo test -p dot-logger-client --features remote`: Pass (1 unittests + 3 connection_lifetime + 7 routed_check + 7 routed_host + 8 tail_screen + 12 transport = 38 tests 통과)
  - `cargo test -p dot-logger-cli`: Pass (5 check_host + 13 cli + 1 operations_host + 3 routed_check_host = 22 tests 통과)
  - `cargo test -p dot-logger-rt-allocation`: Pass (16/16 Pass, RT 핫패스 0 할당 / 0 해제 불변식 완벽 보존)
  - `RUSTDOCFLAGS='-D warnings' cargo doc --locked -p dot-logger -p dot-logger-client -p dot-logger-cli --all-features --no-deps`: Pass (문서 경고 0건)
  - Safe Rust 100% (신규 구현 코드 내 `unsafe` 블록 0건)
- **종합 판정:** **Pass** (P1 0건, P2 0건, P3 3건). RT 경로 무할당 불변식 보존, 호스트 일반 워커 주도형 구조, wire/gRPC/CLI 오류 매핑 정합성, 관측 정직성(0-채움 배제), Safe Rust 100% 준수 확인.
- **상태:** **Closed** (2026-11-03 #003 개발 세션, 소스 `ee57f44`에서 P3 3건 수용·조치)

---

### 확인한 긍정 사항

1. **RT(실시간) 경로 제약의 완벽한 격리:**
   - `OperationsHost`는 수집 코어나 RT 핫패스를 일절 경유하지 않으며, 전적으로 호스트의 일반 워커 쓰레드(`general worker`) 상에서 구동됨.
   - `dot-logger-rt-allocation`의 16개 테스트 전체 통과로 레코드 수집, 거절, drop 경로의 0 할당 / 0 해제 불변식에 전혀 영향을 주지 않음이 실증됨.
2. **100% Safe Rust 및 건전한 오류 처리:**
   - 신규 추가된 `operations.rs` 및 어댑터/클라이언트/CLI 코드 전체에 `unsafe` 블록이 전무함.
   - 라이브러리 프로덕션 코드 내 `unwrap()`, `expect()`, `panic!` 호출이 일절 없으며, 모든 실패 경로가 typed `RemoteError` 및 `ClientError`로 전파됨.
3. **엄격한 게시 원자성 및 라이브 메모리 변경 차단:**
   - `ApplyConfig`는 검증된 JSON 패치에 한해 `apply_config`를 호출하며, 메모리 변경 필드가 포함된 경우 `RestartRequired`로 즉시 거부하고 라이브 반영을 차단함.
   - 선행 revision 충돌 시 `Conflict`, 기존 변경이 드레이닝 중일 때 `Busy`를 반환하고, 실제 변경이 없는 빈 패치는 `InvalidInput`으로 배제하여 불필요한 revision 증가를 방지함.
   - CLI는 `Conflict`, `Busy`, `RestartRequired` 실패 시 `completion_unconfirmed` 플래그를 붙이지 않아, 호스트에 게시되지 않은 실패가 완료 미확인 상태로 오인되지 않도록 보장함.
4. **wire 호환성 및 정직한 오류 코드 다중화:**
   - gRPC 표준 코드의 한계(`Unavailable`, `FailedPrecondition`)를 보완하기 위해 고정 헤더(`dot-logger-reason: busy`, `dot-logger-reason: restart_required`)를 부여함.
   - 클라이언트는 메타데이터가 존재할 때만 세부 에러로 복원하고, 메타데이터가 없는 레거시 또는 타 장애 상황에서는 안전한 기본 gRPC 에러(`Transport`, `Version`)로 폴백하여 실패를 성공으로 오인하거나 잘못된 가정을 하지 않음.
5. **관측 정직성(No Zero-Fill) 및 하위 호환성:**
   - Status 리포트의 표본/윈도우 지표는 `Observed`, `Stale`, `Unavailable` 상태와 원래 사유를 보존하며 인위적인 `0` 값 채움을 원천 배제함.
   - 호스트에 지표 백엔드가 없거나 구형 서버가 응답할 경우 빈 바이트열(`report.is_empty()`)을 통해 `"state": "Unavailable", "reason": "no_runtime_metrics_backend"`를 명시하여 이전 클라이언트/서버와의 상호 호환성을 유지함.
6. **응답 예산(Response Capacity) 초과 시 안전한 Limit 반환:**
   - 직렬화된 JSON 리포트가 커맨드의 응답 버퍼 한도를 초과할 경우 데이터를 자르거나 손상시키지 않고 `RemoteError::Limit`으로 안전하게 거절함.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-091 | P3 | [`crates/dot-logger/src/telemetry/operations.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/telemetry/operations.rs), `OperationsHost` 메서드군 | `OperationsHost::new`, `register_snapshot`, `forget_snapshot`, `compare` 등 주요 API의 rustdoc에 실패 조건 및 반환 오류(`RemoteError`)를 명시하는 `# Errors` 섹션 누락 | 각 메서드 상단 rustdoc에 `# Errors` 섹션을 추가하고 실패 조건(`InvalidInput`, `Limit`, `Gone`)을 명시하여 호스트 연동 시 편의성 개선 | 수용·조치 | Closed |
| RV-092 | P3 | [`crates/dot-logger/src/telemetry/operations.rs#L33`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/telemetry/operations.rs#L33), `pub struct OperationsHost` | 공개(public) 구조체인 `OperationsHost`에 `Debug` 트레이트가 구현되어 있지 않아 호스트 워커나 진단 컨텍스트에서 구조체 로깅/디버깅 불가 | `OperationsHost`에 `#[derive(Debug)]`를 부여하여 트레이트 인체공학(Trait Ergonomics) 및 진단 편의성 향상 | 수용·조치 | Closed |
| RV-093 | P3 | [`crates/dot-logger/src/telemetry/operations.rs#L119-L129`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/telemetry/operations.rs#L119-L129), `status` 리포트 `snapshots` | 스냅샷 등록(`register_snapshot`) 및 해제(`forget_snapshot`)가 반복되면 첫 빈 슬롯 재사용 특성으로 인해 `snapshots` JSON 배열의 ID가 비단조(non-monotonic) 순서(예: `[3, 2]`)로 직렬화될 수 있음 | 직렬화 전 ID 기준 정렬을 수행하거나, API §15.8 문서에 `snapshots` 배열이 ID 순서를 보장하지 않는 비정렬 집합임을 명시하여 클라이언트의 배열 순서 가정 방지 | 수용·조치(대안 A) | Closed |

---

## 3. 상세 리뷰 내역

### RV-091: `OperationsHost` 주요 메서드 rustdoc `# Errors` 가이드 보강 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/telemetry/operations.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/telemetry/operations.rs) (`new`, `register_snapshot`, `forget_snapshot`, `compare`)
- **분석 내용:**
  - `OperationsHost`는 외부 호스트가 인스턴스를 소유하고 운영 명령을 중계하는 진입점이다.
  - 다음 메서드들은 정해진 불변식에 따라 특정 `RemoteError`를 반환한다:
    1. `new(slots: usize)`: `slots`가 `1..=MAX_SLOTS` 범위를 벗어나면 `Err(RemoteError::InvalidInput)`.
    2. `register_snapshot(&mut self, snapshot: Snapshot)`: 슬롯이 가득 찼거나 ID 채번이 `u64` 한도에 도달하면 `Err(RemoteError::Limit)`.
    3. `forget_snapshot(&mut self, id: u64)`: 대상 `id`가 현재 등록 슬롯에 없으면 `Err(RemoteError::Gone)`.
    4. `compare(&self, before: u64, after: u64)`: `before` 또는 `after` 중 하나라도 등록되어 있지 않으면 `Err(RemoteError::Gone)`.
  - 현재는 각 메서드에 한 줄 기능 설명만 있고 `# Errors` 섹션이 명시되어 있지 않아, 호스트 라이브러리 연동자가 내부 구현을 확인하지 않고는 어떤 오류 variant가 발생하는지 파악하기 어렵다.
- **처리 제안:**
  - 각 메서드 상단 rustdoc에 `# Errors` 섹션을 보강하여 발생하는 `RemoteError`의 종류와 조건을 명기한다.

#### [처리담당]
- **판정 결과:** 수용·조치 (`Closed`)
- **처리 내용:** `new`(`InvalidInput`), `register_snapshot`(`Limit`: 슬롯 가득/ID 소진), `forget_snapshot`·`compare`(`Gone`)에 더해 `apply`에도 `# Errors`를 추가해 §15.8 오류 매핑 전체(`Unsupported`·`InvalidInput`·`RestartRequired`·`Conflict`·`Busy`·`Limit`·`Gone`·`Transport`)를 명시했다. 동작 변경은 없다.
- **검증 근거:** 소스 `ee57f44`; `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc --locked -p dot-logger -p dot-logger-client -p dot-logger-cli --all-features --no-deps` exit0. 검증 원문(비공개 원시 근거: `imported_context/rev045_2026-11-03/rev045-verify.log`)

---

### RV-092: `OperationsHost`에 `Debug` 트레이트 유도 추가 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/telemetry/operations.rs#L33`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/telemetry/operations.rs#L33)
- **분석 내용:**
  - `pub struct OperationsHost`는 공개 타입이다. 내부 필드인 `Vec<Option<(u64, Snapshot)>>`와 `u64`는 모두 `Debug`를 완벽히 지원한다(`Snapshot` 또한 `Clone, Copy, Debug`를 가짐).
  - Rust 라이브러리 설계 가이드라인(C-DEBUG)에 따라 모든 공용 타입은 특별한 은닉 이유가 없는 한 `Debug`를 구현하는 것이 권장된다.
  - 호스트 워커나 서비스 데몬 구조체에서 `OperationsHost`를 필드로 가질 때 `#[derive(Debug)]`가 누락되어 있으면 해당 호스트 구조체 전체가 `Debug`를 파생할 수 없게 되는 불편이 발생한다.
- **처리 제안:**
  - `pub struct OperationsHost` 상단에 `#[derive(Debug)]`를 추가한다.

#### [처리담당]
- **판정 결과:** 수용·조치 (`Closed`)
- **처리 내용:** `#[derive(Debug)]`를 추가했다.
- **검증 근거:** 소스 `ee57f44`; `host_slots_are_bounded`에 `{host:?}` 포맷 검사를 추가했다(추가 전 E0277 컴파일 Red → derive 후 Pass). 검증 원문(비공개 원시 근거: `imported_context/rev045_2026-11-03/rev045-verify.log`)

---

### RV-093: `status` 리포트의 `snapshots` 배열 순서 계약 명확화 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/telemetry/operations.rs#L119-L129`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/telemetry/operations.rs#L119-L129)
- **분석 내용:**
  - `status` 메서드는 `self.slots.iter().flatten()`을 순회하여 JSON `snapshots` 배열을 생성한다.
  - `register_snapshot`은 첫 번째 `None` 슬롯을 찾아 배치하고, `forget_snapshot`은 임의의 슬롯을 비운다.
  - 예를 들어 슬롯 0, 1에 스냅샷 1, 2가 등록된 후 스냅샷 1을 forget하고 스냅샷 3을 등록하면 슬롯 0에 3, 슬롯 1에 2가 위치하게 되어, 결과 JSON에서 `[{"id":"3", ...}, {"id":"2", ...}]` 형태로 출력될 수 있다.
  - 클라이언트가 스냅샷 목록을 시간순/ID 순으로 정렬되어 있다고 오인할 경우 UI나 CLI 표시에서 혼선이 생길 여지가 있다.
- **처리 제안:**
  - 대안 A: 직렬화 시 ID 기준 정렬을 적용하여 항상 안정적인 오름차순 배열을 제공한다.
  - 대안 B: API §15.8 문서에 `snapshots` 배열이 ID 순서를 보장하지 않는 비정렬 목록(unordered set)임을 명시한다.

#### [처리담당]
- **판정 결과:** 수용·조치 — 대안 A (`Closed`)
- **처리 내용:** ID가 인스턴스 안에서 재사용되지 않으므로 ID 오름차순은 곧 등록 순서다. 정렬 비용은 일반 worker의 64개 이하 항목이고 RT 경로와 무관하므로, 문서로 비정렬을 허용하기보다 안정 순서를 보장했다. `status`가 보유 snapshot을 ID 기준 정렬 후 직렬화하며, API §15.8에 「ID 오름차순 = 등록 순서」를 명시했다.
- **검증 근거:** 소스 `ee57f44`; `compare_preserves_differences_and_rejects_unknown_or_exhausted_ids` 끝에 forget 후 첫 슬롯 재사용 목록 검사를 추가했다(Red: `["3","2"]` → Pass: `["2","3"]`). `cargo test -p dot-logger --features remote`, `-p dot-logger-cli`, `-p dot-logger-rt-allocation`(16/16, RT 0할당/0해제) 전체 Pass. 검증 원문(비공개 원시 근거: `imported_context/rev045_2026-11-03/rev045-verify.log`)


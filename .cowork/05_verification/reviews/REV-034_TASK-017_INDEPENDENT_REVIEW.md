# [REV-034] TASK-017 고정 routed RunCheck 독립 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-11-01 / AI 리뷰 전용 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 코드리뷰. REV-033은 개발 세션 자가 점검이었으므로, TASK-017 고정 routed RunCheck 커밋(`56e7b9a`) 전체에 대해 독립적 관점에서 심층 검토를 수행했다. 제품 코드는 전혀 수정하지 않았다.
- **대상 형상:** 커밋 `56e7b9a` (15개 파일 변경, +1645 / -521)
  - 코어 호스트/트레이트: [`diagnostics/routed.rs`](../../../crates/dot-logger/src/diagnostics/routed.rs) (신규), [`diagnostics/host.rs`](../../../crates/dot-logger/src/diagnostics/host.rs), [`diagnostics/mod.rs`](../../../crates/dot-logger/src/diagnostics/mod.rs), [`diagnostics/run.rs`](../../../crates/dot-logger/src/diagnostics/run.rs)
  - 클라이언트 스냅샷 점검/브리지: [`src/diagnostics/mod.rs`](../../../crates/dot-logger-client/src/diagnostics/mod.rs) (신규), [`src/diagnostics/bridge.rs`](../../../crates/dot-logger-client/src/diagnostics/bridge.rs) (신규), [`src/lib.rs`](../../../crates/dot-logger-client/src/lib.rs), [`examples/dot_probe_check.rs`](../../../crates/dot-logger-client/examples/dot_probe_check.rs)
  - 테스트 및 CLI 검증: [`tests/routed_host.rs`](../../../crates/dot-logger-client/tests/routed_host.rs) (신규), [`tests/routed_check_host.rs`](../../../crates/dot-logger-cli/tests/routed_check_host.rs) (신규), [`tests/check_host.rs`](../../../crates/dot-logger-cli/tests/check_host.rs)
- **정적 검증 결과:**
  - `cargo fmt --check`: Pass (포맷 완전 준수)
  - `cargo clippy --all-targets -- -D warnings`: Pass (경고 0건)
  - `cargo test --workspace`: Pass (전체 크레이트 단위/통합/문서 테스트 100% 통과)
  - `cargo test -p dot-logger-rt-allocation`: 15/15 Pass (RT allocation/free 0 확인)
  - `cargo test -p dot-logger-client --test routed_host --all-features`: 7/7 Pass
  - `cargo test -p dot-logger-cli --test routed_check_host`: 3/3 Pass
  - `cargo test -p dot-logger-cli --test check_host`: 5/5 Pass
- **종합 판정:** Pass (P1 0건, P2 0건, P3 2건). RT 제약 완벽 분리, 동시성/자원 수명 보장, Safe Rust 100%, 단일 사용 슬롯 계약 엄격 준수.
- **상태:** Open

---

### 확인한 긍정 사항

1. **RT(실시간) 경로의 철저한 격리 및 무할당 유지:**
   - `RoutedCheckBackend`, `SnapshotCheck`, `bridge::execute`, `CheckHost::run_routed` 등 신규 추가된 진단 로직은 전체가 일반 호스트 워커(General-path worker) 및 비-RT 관리 경로에 한정됨.
   - RT 할당 검증 테스트(`dot-logger-rt-allocation`) 15개 항목이 모두 통과하여 RT 수집/대여/반환 경로의 할당/해제 0 불변식이 완벽히 보존됨을 확인함.
2. **엄격한 단일 사용(Single-use) 슬롯 계약 및 다형적 실행 분기:**
   - `CheckHost::run_routed`는 슬롯 탐색 후 `slot.take().ok_or(HostError::Gone)?` 패턴을 사용하여 호출 즉시 등록 슬롯을 비우므로, 동일한 진단 기능의 재실행(replay)이나 ABA 재사용이 원천적으로 차단됨.
   - `Runner::File`과 `Runner::Routed`의 이중 백엔드를 내부 열거형으로 캡슐화하고, 잘못된 백엔드 실행 시도(`run` ↔ `run_routed`) 시 슬롯을 소비하지 않고 `HostError::InvalidInput`으로 안전하게 거절함.
3. **100% Safe Rust 및 완전한 자원 회수/종료(Settlement) 보장:**
   - 신규 구현 코드 전체에 `unsafe` 블록이 단 하나도 존재하지 않음.
   - `bridge.rs`의 `execute`는 내부에서 `hub.close()`, `server.await`, `drop((hub, inbox))`를 반드시 거친 후 종료하도록 설계되어 비정상 종료나 에러 발생 시에도 좀비 태스크 및 포트/메모리 누수가 방지됨.
   - `budget.used_bytes() == 0` 및 `raw_reclaimed == 1`을 검증하여 파이프라인의 공유 메모리와 슬롯이 완전히 회수됨을 입증함.
4. **원본 증거 보존 및 방어적 에러/시계 처리:**
   - RV-069(호스트 데드라인 재검사) 및 RV-070(초기 OS 에러 보존)이 기수용되어, capture 직전 및 handoff 직전 이중 단조 시계 검사가 수행되고 `raw_os_error`가 유실 없이 전달됨.
   - `SnapshotCheck::run`에서 수신된 레코드 페이로드가 원본과 일치하는지(`identical`)를 대조하여 단 1바이트의 변조도 `CheckOutcome::Fail`로 탐지함.
   - `tokio::runtime::Handle::try_current().is_ok()` 방어 코드를 통해 중첩 런타임 호출에 의한 패닉 위험을 사전에 차단함.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-071 | P3 | [`dot-logger-client/src/diagnostics/mod.rs#L27-L30`](../../../crates/dot-logger-client/src/diagnostics/mod.rs#L27-L30), `SnapshotCheck::target_id` | `SnapshotCheck`의 고정 대상 타깃(`target_id = 10`)이 트레이트 메서드로만 노출되어 있어, 호스트 등록 시 호출자가 별도 문서 없이 타깃 ID를 사전에 알기 어려움 | `SnapshotCheck::TARGET_ID: u32 = 10` 연관 상수를 노출하거나 `SnapshotCheck` 상단 rustdoc에 대상 타깃 ID가 10으로 고정됨을 명시 | 대기 (Open) | Open |
| RV-072 | P3 | [`dot-logger-client/src/diagnostics/mod.rs#L31-L35`](../../../crates/dot-logger-client/src/diagnostics/mod.rs#L31-L35), `required_memory_bytes` | `required_memory_bytes`가 24 MiB로 하드코딩되어 있는데, `CheckPlan` 생성자가 요구 예산을 책정할 때 참고할 수 있는 rustdoc 가이드 필요 | `SnapshotCheck` struct 레벨 rustdoc에 16 MiB 코어 로거 예약과 8 MiB 런타임/버퍼 북키핑을 합산한 최소 24 MiB 요구치임을 문서화 | 대기 (Open) | Open |

---

## 3. 상세 리뷰 내역

### RV-071: `SnapshotCheck`의 고정 `target_id` 상수 및 계약 문서화 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger-client/src/diagnostics/mod.rs#L27-L30`](../../../crates/dot-logger-client/src/diagnostics/mod.rs#L27-L30)
- **분석 내용:**
  - `SnapshotCheck`는 `RoutedCheckBackend` 트레이트를 구현하며 `target_id(&self) -> u32`에서 고정값 `10`을 반환한다.
  - `CheckHost::register_routed(plan, target, backend, source)`를 호출할 때 `backend.target_id() != target`이면 `HostError::InvalidInput`이 발생한다.
  - 현재 `TARGET`은 `bridge.rs` 내부 상수(`const TARGET: u32 = 10;`)로 은닉되어 있으며, 외부에 공개된 상수가 없어 호출자가 타깃 ID 불일치 에러를 방지하려면 코드를 직접 읽어야 한다.
- **처리 제안:**
  - `SnapshotCheck`의 연관 상수 `pub const TARGET_ID: u32 = 10;`을 제공하거나, `SnapshotCheck::new` rustdoc에 타깃 ID 10과의 바인딩 계약을 명시할 것을 권고한다.

#### [처리담당]
- **판정 결과:** 대기 (Open)
- **처리 내용:** (후속 개발 세션에서 처리)
- **검증 근거:** (후속 개발 세션에서 작성)

---

### RV-072: `SnapshotCheck::required_memory_bytes` 24 MiB 수치 근거 rustdoc 가이드 보강 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger-client/src/diagnostics/mod.rs#L31-L35`](../../../crates/dot-logger-client/src/diagnostics/mod.rs#L31-L35)
- **분석 내용:**
  - `SnapshotCheck::required_memory_bytes()`는 `24 * 1024 * 1024` (24 MiB)를 반환한다.
  - `CheckHost::register_routed`는 `backend.required_memory_bytes() > s.required_memory_bytes`인 경우 등록을 거절(`HostError::InvalidInput`)하므로, 상위 플랜 생성자는 반드시 `required_memory_bytes >= 24 MiB`로 `CheckPlan`을 빌드해야 한다.
  - 현재는 코드 내부 주석에만 설명(`// 16 MiB logger reservation plus conservative input/JSON/receiver/runtime bookkeeping`)이 존재한다.
- **처리 제안:**
  - `SnapshotCheck` 구조체 rustdoc에 플랜 요구 메모리 수치(최소 24 MiB)와 그 구성 근거(16 MiB 세션 예약 + 8 MiB 런타임/입출력)를 명시하여 상위 호스트 구성자가 올바른 `CheckPlan`을 작성할 수 있도록 돕는다.

#### [처리담당]
- **판정 결과:** 대기 (Open)
- **처리 내용:** (후속 개발 세션에서 처리)
- **검증 근거:** (후속 개발 세션에서 작성)

---

## 4. 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | [`crates/dot-logger/src/diagnostics/routed.rs`](../../../crates/dot-logger/src/diagnostics/routed.rs) | `RoutedCheckBackend` 트레이트, `RoutedCheckReport`, `RoutedCheckEvidence`, `HostCheckEvidence` 정의 |
| 수정 | [`crates/dot-logger/src/diagnostics/host.rs`](../../../crates/dot-logger/src/diagnostics/host.rs) | 고정 File/Routed 다형 등록(`register_routed`) 및 소비(`run_routed`), `handle_command` 다형 디스패치 |
| 수정 | [`crates/dot-logger/src/diagnostics/mod.rs`](../../../crates/dot-logger/src/diagnostics/mod.rs) | routed 모듈 노출 및 심볼 재내보내기 |
| 수정 | [`crates/dot-logger/src/diagnostics/run.rs`](../../../crates/dot-logger/src/diagnostics/run.rs) | diagnostics 내부 가시성 정렬 |
| 생성 | [`crates/dot-logger-client/src/diagnostics/mod.rs`](../../../crates/dot-logger-client/src/diagnostics/mod.rs) | `SnapshotCheck` 구현 (2초/24MiB 예산, 원본 시계 관측, 페이로드 일치 검증) |
| 생성 | [`crates/dot-logger-client/src/diagnostics/bridge.rs`](../../../crates/dot-logger-client/src/diagnostics/bridge.rs) | 실제 프로덕션 capture/routing/loopback TCP 파이프라인 브리지 실행기 |
| 수정 | [`crates/dot-logger-client/src/lib.rs`](../../../crates/dot-logger-client/src/lib.rs) | diagnostics 모듈 공개 |
| 수정 | [`crates/dot-logger-client/Cargo.toml`](../../../crates/dot-logger-client/Cargo.toml) | remote 선택 feature에 `serde_json` 의존성 연계 |
| 수정 | [`crates/dot-logger-client/README.md`](../../../crates/dot-logger-client/README.md) | 진단 브리지 및 스냅샷 점검 사용법 문서화 |
| 생성 | [`crates/dot-logger-client/examples/dot_probe_check.rs`](../../../crates/dot-logger-client/examples/dot_probe_check.rs) | 고정 host 무부하 plan 생성 및 SnapshotCheck 등록/실행 예제 |
| 수정 | [`crates/dot-logger-client/examples/dot_probe_bridge.rs`](../../../crates/dot-logger-client/examples/dot_probe_bridge.rs) | 공용 diagnostics bridge 활용으로 전환 |
| 수정 | [`crates/dot-logger-client/examples/support/dot_probe.rs`](../../../crates/dot-logger-client/examples/support/dot_probe.rs) | 중복 브리지 코드 제거 및 bridge 모듈 re-export |
| 생성 | [`crates/dot-logger-client/tests/routed_host.rs`](../../../crates/dot-logger-client/tests/routed_host.rs) | routed 호스트 단일 실행, 시계 실패/만료, 불변식 검증 테스트 7종 |
| 생성 | [`crates/dot-logger-cli/tests/routed_check_host.rs`](../../../crates/dot-logger-cli/tests/routed_check_host.rs) | CLI 관리 권한 및 routed 호스트 실 TCP 원격 실행 테스트 3종 |
| 수정 | [`crates/dot-logger-cli/tests/check_host.rs`](../../../crates/dot-logger-cli/tests/check_host.rs) | `HostCheckEvidence` 열거형 매칭 적용 |

---

## 5. 클래스(타입) 변경표

| 구분 | 타입 | 소속 파일 | 역할 및 변경 내용 |
|---|---|---|---|
| 생성 (trait) | [`RoutedCheckBackend`](../../../crates/dot-logger/src/diagnostics/routed.rs#L9) | `dot-logger::diagnostics::routed` | 단일 사용의 신뢰된 호스트 라우팅 백엔드 확장 트레이트 (`Send`) |
| 생성 (struct) | [`RoutedCheckReport`](../../../crates/dot-logger/src/diagnostics/routed.rs#L23) | `dot-logger::diagnostics::routed` | 원본 receipt, completion, 시계 구간, 승인/확인 카운터, 자원 회수 결과 보고서 |
| 생성 (struct) | [`RoutedCheckEvidence`](../../../crates/dot-logger/src/diagnostics/routed.rs#L49) | `dot-logger::diagnostics::routed` | 호스트 플랜, 타깃, 보고서, 원본 runtime Report 보존 증거체 |
| 생성 (enum) | [`HostCheckEvidence`](../../../crates/dot-logger/src/diagnostics/routed.rs#L61) | `dot-logger::diagnostics::routed` | `File(CheckEvidence)` 및 `Routed(RoutedCheckEvidence)` 다형 래퍼 |
| 생성 (struct) | [`SnapshotCheck`](../../../crates/dot-logger-client/src/diagnostics/mod.rs#L12) | `dot-logger-client::diagnostics` | dot-probe 스냅샷 검증 및 `RoutedCheckBackend` 구현체 |
| 수정 (struct) | [`CheckHost`](../../../crates/dot-logger/src/diagnostics/host.rs#L59) | `dot-logger::diagnostics::host` | `register_routed`, `run_routed` 추가 및 `handle_command` 다형 분기 지원 |
| 수정 (enum) | [`Runner`](../../../crates/dot-logger/src/diagnostics/host.rs#L55) | `dot-logger::diagnostics::host` | 등록 슬롯 내부에서 `File`과 `Routed` 실행기 캡슐화 |
| 이동/공개 | [`bridge::Action`](../../../crates/dot-logger-client/src/diagnostics/bridge.rs#L28) | `dot-logger-client::diagnostics::bridge` | 소비자 핸드오프 수락/거절 판정 열거형 |
| 이동/공개 | [`bridge::BridgeOutcome`](../../../crates/dot-logger-client/src/diagnostics/bridge.rs#L57) | `dot-logger-client::diagnostics::bridge` | 브리지 파이프라인 기능 결과 (`Pass`, `Fail`, `Aborted`, `Inconclusive`) |
| 이동/공개 | [`bridge::BridgeReport`](../../../crates/dot-logger-client/src/diagnostics/bridge.rs#L69) | `dot-logger-client::diagnostics::bridge` | 브리지 내부 세부 실행 지표 및 자원 상태 구조체 |


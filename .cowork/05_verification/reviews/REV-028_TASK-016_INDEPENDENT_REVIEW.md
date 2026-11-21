# [REV-028] TASK-016 revision 설정 변경·전후 비교 독립 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-11-01 / AI 리뷰 전용 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 코드리뷰. REV-027은 개발 세션 자가 점검이었으므로, TASK-016 구현 커밋(`0247f3e`) 전체에 대해 독립적 관점에서 심층 검토를 수행했다. 제품 코드는 전혀 수정하지 않았다.
- **대상 형상:** 커밋 `0247f3e` (`6f68cd6..0247f3e`, 16개 파일 변경, +2028 / -27)
  - 코어/제어: [`control/config_update.rs`](../../../crates/dot-logger/src/control/config_update.rs) (신규), [`control/mod.rs`](../../../crates/dot-logger/src/control/mod.rs)
  - 계측/비교: [`telemetry/compare.rs`](../../../crates/dot-logger/src/telemetry/compare.rs) (신규), [`telemetry/mod.rs`](../../../crates/dot-logger/src/telemetry/mod.rs), [`telemetry/aggregate.rs`](../../../crates/dot-logger/src/telemetry/aggregate.rs), [`telemetry/capacity.rs`](../../../crates/dot-logger/src/telemetry/capacity.rs)
  - 수집/동시성/수명: [`capture/mod.rs`](../../../crates/dot-logger/src/capture/mod.rs), [`completion/mod.rs`](../../../crates/dot-logger/src/completion/mod.rs), [`routing/mod.rs`](../../../crates/dot-logger/src/routing/mod.rs), [`pool/mod.rs`](../../../crates/dot-logger/src/pool/mod.rs)
  - 테스트 및 검증: [`tc_009_config_update.rs`](../../../crates/dot-logger/tests/tc_009_config_update.rs), [`tc_009_compare.rs`](../../../crates/dot-logger/tests/tc_009_compare.rs), [`tc_004_storage.rs`](../../../crates/dot-logger/tests/tc_004_storage.rs), [`rt-allocation/src/tests.rs`](../../../tests/rt-allocation/src/tests.rs), [`api-consumer/src/main.rs`](../../../tests/api-consumer/src/main.rs)
- **정적 검증 결과:**
  - `cargo fmt --check`: Pass (코드 포맷 준수)
  - `cargo clippy --all-targets -- -D warnings`: Pass (경고 0건)
  - `cargo test --workspace`: Pass (전체 크레이트 100+ 단위/통합 테스트 통과)
  - `cargo test --test tc_009_config_update`: 13/13 Pass
  - `cargo test --test tc_009_compare`: 3/3 Pass
  - `cargo test --test tc_004_storage`: 22/22 Pass
  - RT 무할당 검증: `live_revision_boundary_sampling_raw_errors_and_drop_do_not_allocate` (0, 0) Pass
- **종합 판정:** Pass (P1 0건, P2 0건, P3 2건). RT 제약, 동시성/원자성 안전성, 공개 API 격리가 완벽히 준수됨.
- **상태:** Closed — 2026-11-01 #004 개발 세션에서 Open → In Progress → Resolved → Closed. RV-063/064 수용·문서/회귀 반영, EV-031·소스b060c69, 기존 위임 로컬 수락.

---

### 확인한 긍정 사항

1. **RT(실시간) 경로의 철저한 무할당·무대기 보장:**
   - `Producer::apply_pending_config()` 및 `boundary()`는 원자적 로드/스토어(`posted.load(Acquire)`, `cutoff.store(Relaxed)`, `applied.store(Release)`)만 수행하며 lock 획득이나 힙 할당이 전무함.
   - `sampled_out()`의 Normal 필터링 및 샘플링 카운터 갱신은 고정 크기 배열 인덱스 연산으로 수행되며, RT allocator 테스트에서 (0, 0) 할당/해제 없음을 입증함.
   - 관리 경로용 Mutex(`State::management`)는 생산자 RT 호출 경로에서 절대 호출되지 않음.
2. **동시성 및 원자적 메모리 순서(Memory Ordering)의 건전성:**
   - 관리 스레드가 `next` bank에 새 설정을 기록한 후 `destination.revision.store(Release)`와 `posted.store(Release)`로 게시하고, 생산자는 `posted.load(Acquire)`로 이를 인지하는 올바른 Release-Acquire 짝을 구성함.
   - `State::ready`에서 이전 세대의 `guards.load(Acquire) == 0`, `applied.load(Acquire) == m.revision`, `consumed.load(Acquire) >= cutoff.load(Acquire)` 및 `!registry.pending_revision(old)`의 4중 불변식을 확인한 후 bank를 재사용하므로 ABA 문제 및 조기 덮어쓰기가 원천 차단됨.
3. **Rust 언어 안전성 및 예외 배제:**
   - `config_update.rs`와 `compare.rs`를 포함한 신규 프로덕션 코드 전체에 `unsafe` 블록이 단 하나도 없음 (100% Safe Rust).
   - `unwrap()`, `expect()`, `panic!` 호출이 일체 배제되었으며, 모든 에러 케이스는 정밀한 `ConfigUpdateError` 및 `Value::Unavailable`로 모델링됨.
4. **증거 보존 전후 비교 산술:**
   - `compare.rs`의 `change()` 함수는 `u128` 교차 곱셈(`u128::from(amount) * u128::from(elapsed_ns)`)을 사용하여 부동소수점 오차나 오버플로 없이 정확한 변화량(`RateChange`)을 계산함.
   - 불일치 조건(`Differences`)이 발생하거나 관측치가 Stale/Unavailable인 경우 허위 절감이나 임의의 0으로 추정하지 않고 명시적 사유(`IncomparableConditions`, `InvalidMeasurement`)를 보존함.
5. **MCAP 단일 revision 격리:**
   - `Router::writer_budget_for_revision()`을 도입하여 revision 변경 시 기존 파일에 서로 다른 revision 레코드가 혼합되지 않고 새 파일 writer를 생성할 수 있는 계약을 확립함.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-063 | P3 | [`control/config_update.rs#L258-L265`](../../../crates/dot-logger/src/control/config_update.rs#L258-L265), `State::ready` | 유휴 생산자(idle producer)가 `poll_config()`나 입력을 생성하지 않으면 `p.applied`가 갱신되지 않아 후속 `apply_config`가 `RevisionBusy`로 계속 대기할 수 있음 | 의도된 2세대 메모리 보호 설계이므로 코드는 유지하되, 상위 호스트/런타임 계층에서 유휴 생산자에 대한 주기적 poll 권장사항을 운영 문서화 | 수용·EV-031/b060c69 | Closed |
| RV-064 | P3 | [`telemetry/compare.rs#L154-L159`](../../../crates/dot-logger/src/telemetry/compare.rs#L154-L159), `Differences::invalid_window` | 윈도우 시간 역전 검사(`b.window.to_ns > a.window.from_ns`) 외에, 전후 비교 시 설정 변경 과도기(`RevisionTransition`) 구간이 배제되어야 함을 호스트 가이드에 명시 필요 | `tc_009_compare` 시험 및 compare rustdoc에 과도기 윈도우 배제 지침을 명확히 보강 | 수용·EV-031/b060c69 | Closed |

---

## 3. 상세 리뷰 내역

### RV-063: 유휴 생산자(Idle Producer)로 인한 `RevisionBusy` 유지 및 호스트 운영 가이드 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/control/config_update.rs#L258-L265`](../../../crates/dot-logger/src/control/config_update.rs#L258-L265)
- **분석 내용:**
  - `State::ready` 함수는 다음과 같이 이전 bank의 안전한 회수를 판정합니다:
    ```rust
    fn ready(&self, m: &Management, registry: &Registry) -> bool {
        let Some(old) = m.previous else { return true };
        self.progress.iter().all(|p| {
            p.applied.load(Ordering::Acquire) == m.revision
                && p.consumed.load(Ordering::Acquire) >= p.cutoff.load(Ordering::Acquire)
        }) && self.banks[1 - m.slot].guards.load(Ordering::Acquire) == 0
            && !registry.pending_revision(old)
    }
    ```
  - 만약 특정 생산자가 현재 긴 유휴(idle) 상태에 있어 신규 레코드를 발행하지 않고 명시적인 `poll_config()` 호출도 하지 않는다면, 해당 생산자의 `p.applied`는 여전히 이전 revision에 머무르게 됩니다.
  - 이 경우 `ready()`는 `false`를 반환하고, 추가적인 `apply_config()` 호출은 `ConfigUpdateError::RevisionBusy`를 반환합니다.
- **평가 및 영향:**
  - 이는 유휴 생산자가 갱신되지 않은 상태에서 bank를 3번째 세대로 덮어쓸 경우 발생할 수 있는 메모리 파괴를 방지하기 위한 정당한 2-세대 더블 버퍼링 불변식입니다 (`tc_009_config_update.rs`의 `producer_boundaries_apply_partially_and_idle_producer_keeps_busy`에서도 명시적으로 검증됨).
  - 다만, 실운영 환경에서 다중 생산자를 운용하는 호스트 애플리케이션(TASK-013 CLI runtime / TASK-020 UI / 추후 dot-probe 통합)이 연속적인 설정 변경을 원활히 적용하기 위해서는 유휴 워커에 대해 주기적으로 `poll_config()`를 호출하거나 capture scope를 갱신해 주는 패턴이 필요합니다.
- **처리 제안:**
  - 제품 코드의 수정은 불필요하며, 상위 계층 통합 문서(API§15.2 및 추후 CLI/통합 가이드)에 "유휴 생산자가 존재하는 경우 `poll_config()`를 주기적으로 구동하여 revision sync를 맞춰야 `RevisionBusy`를 해소할 수 있다"는 점을 명시합니다.

#### [처리담당]
- 판정 결과: 수용 / Closed (2026-11-01 #004, 기존 위임에 따른 AI 수락).
- 처리 내용: API §15.2와 `Producer::apply_pending_config()` rustdoc에 소유 worker의 유휴 주기 호출 책임·ack 갱신 없을 때 RevisionBusy·old ingress/완료/실제 guard 별도 정리 조건을 명시했다. 리뷰의 `poll_config()`는 개념적 poll이며 실제 공개 API 이름은 `apply_pending_config()`다. 자동 worker/강제 회수는 추가하지 않았다.
- 검증 근거: [EV-031](../verification_evidence.md#ev-031)·소스b060c69. 기존 `producer_boundaries_apply_partially_and_idle_producer_keeps_busy` 포함 설정13개·비교4개 및 workspace/all-features 회귀 Pass.

---

### RV-064: 윈도우 시간 비교 조건 및 과도기(`RevisionTransition`) 배제 지침 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/telemetry/compare.rs#L154-L159`](../../../crates/dot-logger/src/telemetry/compare.rs#L154-L159)
- **분석 내용:**
  - `Differences` 생성 시 윈도우 유효성을 검사하는 로직:
    ```rust
    invalid_window: b.window.from_ns >= b.window.to_ns
        || a.window.from_ns >= a.window.to_ns
        || b.window.to_ns > a.window.from_ns
        || b.conditions.revision != b.window.revision
        || a.conditions.revision != a.window.revision,
    ```
  - 여기서 `b.window.to_ns > a.window.from_ns`는 before 윈도우와 after 윈도우의 시간 역전 또는 겹침을 정확히 감지하여 `invalid_window = true`로 처리합니다.
  - 만약 `b.window.to_ns == a.window.from_ns`인 경우(연속된 윈도우)에는 `invalid_window`가 되지 않습니다. 그러나 `apply_config`가 발생한 직후의 시점에는 `Telemetry::sample`이 `Observation::Unavailable(Unavailable::RevisionTransition)`을 반환하게 되므로, 전후 비교에 사용되는 윈도우는 전환 과도기 샘플을 포함하지 않는 안정된 윈도우여야 정확한 `RateChange`가 산출됩니다.
- **평가 및 영향:**
  - `tc_009_compare`에서 시간 역전과 Stale/Unavailable 처리가 정확히 검증되어 있으며 산술적 결함은 없습니다.
  - 다만 호스트가 `Snapshot::new`와 `Comparison::build`를 호출할 때 전환 과도기 구간을 피하여 안정 구간(Stable Window)을 선택하도록 안내하는 사용 지침이 권장됩니다.
- **처리 제안:**
  - compare 모듈 문서 및 API§15.2에 과도기 샘플(`RevisionTransition`)이 포함된 윈도우는 자동으로 `Unavailable`이 되므로, 전후 비교 윈도우는 설정 변경 완료 후 안정화된 구간에서 수집된 리포트를 사용해야 한다는 안내를 보강합니다.

#### [처리담당]
- 판정 결과: 수용 / Closed (2026-11-01 #004, 기존 위임에 따른 AI 수락).
- 처리 내용: compare 모듈/Snapshot rustdoc와 API §15.2에 안정 Observed Window만 사용하고 전환 Sample을 0/임의 Window로 바꾸지 않는 호스트 절차를 명시했다. 기존 전환 시험을 안정 before → RevisionTransition → IncompleteWindow → 새 baseline 이후 안정 after → Report/Comparison 종단으로 확장했다. `tc_009_compare`에 인접 구간 허용/겹침 거절을 추가했다. Comparison이 호출자 제작 Window의 과도기 이력을 자체 재검증한다고 표현하지 않는다.
- 검증 근거: [EV-031](../verification_evidence.md#ev-031)·소스b060c69. 설정13개·비교4개, MSRV1.85 동일 시험·workspace/all-features·fmt/clippy/rustdoc Pass.

---

## 4. 검토하지 않은 범위 및 후속 작업

- **운영 백엔드 및 실앱 연동:** 원격 `ApplyConfig` / `Compare` 운영 backend 연동, CLI runtime 연결 및 자동 worker는 TASK-016 범위 외이며 실앱 통합 시점으로 이월되어 있습니다.
- **PC/RT 실측 검증:** 실제 PC/RT 환경에서의 호출 오차(≤20%), 계측 오버헤드(≤5%), UPS 시뮬레이션 및 native CI(T3)는 TASK-022 및 공개 후 후속으로 유지됩니다.
- **다음 개발 작업:** TASK-016 로컬 구현 및 독립 리뷰가 완료되었으며, 다음 계획된 작업은 TASK-017(예산 제한 간편 점검, Planned)입니다.


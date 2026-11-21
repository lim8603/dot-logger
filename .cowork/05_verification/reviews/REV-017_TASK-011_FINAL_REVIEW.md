# [REV-017] TASK-011 로컬 재생 구현 조치 및 안전성 최종 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-10-31 / AI ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md))
- **대상:** TASK-011 리뷰 조치 커밋 [`c6cc5a5`](../../../crates/dot-logger/src/replay/mod.rs) (`crates/dot-logger/src/replay/mod.rs`, `crates/dot-logger/tests/tc_006_replay.rs`) 및 관련 쿼리 모듈
- **종합 판정:** Pass (권고사항 P2 1건, P3 2건)
  - **Safe Rust 100%**: `replay/mod.rs` 내 `unsafe` 0건, 원소유권 및 라이프타임(`&mut self` 바운드)에 의한 동기식 메모리/버퍼 배타성 완벽 보장
  - **RT 경로 제로 오염**: 오프라인 전용 모듈로 RT 캡처 스레드 오염 및 힙 할당 전이 없음 (`tests/rt-allocation` 0건 유지)
  - **터미널 정리 일원화**: RV-026 반영으로 `late` 누적 산술 한계 초과 시 `self.fail(ReplayError::Limit)` 호출을 통해 스냅샷 및 파일 리더를 즉시 터미널 해제하도록 방어 처리 완료
  - **진단 API 완비**: RV-027, RV-028 반영으로 `Speed`의 분자/분모 getter 및 `Display`, `Speed::REALTIME` 상수 제공, 어휘 타입 `Debug` derive 완료
  - **검증 통과**: `cargo fmt`, `cargo clippy -D warnings`, 전체 단위/통합/문서 테스트 125개 및 Miri 12개 테스트 전체 Pass
- **상태:** Closed — 2026-10-31 Human 최종 구현 승인 (EV-020)

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-029 | P3 | [`query/mod.rs:136`](../../../crates/dot-logger/src/query/mod.rs#L136)<br>`AtomicUsize::fetch_update` | Miri 및 최신 Rust 툴체인에서 `fetch_update`의 rename(`try_update`) deprecation 경고 발생 | MSRV 1.85 호환성을 유지하면서 향후 컴파일러 경고 방지를 위한 `try_update` 전환 또는 선별적 suppress 권고 | 수용 — MSRV 유지·reserve 함수에서만 경고 억제 | Closed |
| RV-030 | P3 | [`replay/mod.rs:13-41`](../../../crates/dot-logger/src/replay/mod.rs#L13-L41)<br>[`Speed`](../../../crates/dot-logger/src/replay/mod.rs#L13-L41) | 원본 분수비를 약분하지 않는 `Speed` 설계상 `Speed::new(1, 1) != Speed::new(2, 2)` 성립 | 제어 어댑터 및 CLI에서 동일 재생 속도 판정 시 혼선을 방지하기 위한 동등 비율 확인 메서드(`is_equivalent`) 또는 rustdoc 보강 권고 | 수용 — 성분 비교 유지·is_equivalent 추가 | Closed |
| RV-031 | P2 | [`replay/mod.rs:73-77`](../../../crates/dot-logger/src/replay/mod.rs#L73-L77)<br>[`ReplayError::Display`](../../../crates/dot-logger/src/replay/mod.rs#L73-L77) | `ReplayError`의 `Display` 구현이 `write!(f, "replay error: {self:?}")`로 단순 Debug 포맷에 의존 | 상위 에러 전파 및 CLI 진단 시 명확한 원인 구분을 위한 human-readable 에러 메시지 매핑 구현 권고 | 수용 — variant별 진단 메시지 | Closed |

---

## 3. 상세 리뷰 내역

### RV-029: `query/mod.rs` 내 `AtomicUsize::fetch_update` deprecation 경고

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/query/mod.rs#L136`](../../../crates/dot-logger/src/query/mod.rs#L136)
- **발견한 문제:**
  ```rust
  self.active_readers
      .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
  ```
  1. Miri 및 최신 Rust 컴파일러 진단 시 `warning: use of deprecated method std::sync::atomic::Atomic::<usize>::fetch_update: renamed to try_update for consistency` 경고가 발생합니다.
  2. 현재 프로젝트의 MSRV는 1.85로 고정되어 있으며, `try_update` 표준 도입 시점과 MSRV 호환성을 확인할 필요가 있습니다.
- **처리 제안:**
  - Rust 1.85에서 `try_update` 사용 가능 여부를 점검하고, 가능할 경우 `try_update`로 교체하거나, MSRV 호환 유지가 필요한 경우 국소적으로 경고 억제(`#[allow(deprecated)]`) 주석을 추가할 것을 권고합니다.
- **심각도:** P3 (Note / 툴체인 및 빌드 클린니스)

---

### RV-030: `Speed` 구조체의 동등 비율(equivalence) 판정 편의 제공

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/replay/mod.rs#L13-L41`](../../../crates/dot-logger/src/replay/mod.rs#L13-L41)
- **발견한 문제:**
  1. RV-027에 따라 `Speed`는 사용자 입력 원본(예: 6/4 배속)을 약분하지 않고 유지하도록 설계되었습니다.
  2. 그러나 `#[derive(Eq, PartialEq)]`가 적용되어 있어, 수학적으로 동일한 비율(예: 1/1과 2/2, 혹은 3/2와 6/4)을 비교할 때 `Speed::new(1, 1).unwrap() != Speed::new(2, 2).unwrap()`가 됩니다.
  3. 재생 제어 루프나 CLI에서 "현재 배속과 동일한 요청인지" 검사할 때 불필요한 virtual anchor 재설정이나 `set_speed()` 호출이 발생할 수 있습니다.
- **처리 제안:**
  - `pub fn is_equivalent(&self, other: &Self) -> bool` (예: `u64::from(self.numerator) * u64::from(other.denominator) == u64::from(other.numerator) * u64::from(self.denominator)`) 헬퍼 메서드를 추가하거나, rustdoc에 약분하지 않는 성분 비교 특성을 명확히 안내할 것을 권고합니다.
- **심각도:** P3 (Note / API 사용성 및 제어 최적화)

---

### RV-031: `ReplayError`의 `Display` 구현 개선

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/replay/mod.rs#L73-L77`](../../../crates/dot-logger/src/replay/mod.rs#L73-L77)
- **발견한 문제:**
  ```rust
  impl std::fmt::Display for ReplayError {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          write!(f, "replay error: {self:?}")
      }
  }
  ```
  1. `ReplayError`의 `Display` 구현이 단순히 `write!(f, "replay error: {self:?}")`로 Debug 포맷팅을 그대로 출력하고 있습니다.
  2. 타 모듈의 에러 타입(`CaptureError`, `QueryError`)과의 일관성이 떨어지며, 상위 CLI나 로깅에서 에러를 `%` 포맷으로 출력할 때 상세 원인이 직관적으로 파악되지 않을 수 있습니다.
- **처리 제안:**
  - 각 variant별로 명확한 설명(예: `InvalidInput` -> `"non-mono filter or invalid rate"`, `Limit` -> `"scheduling deadline or generation limit exceeded"`, `StaleGeneration` -> `"retained blob predates current seek generation"` 등)을 매핑하여 가독성 높은 에러 문자열을 반환하도록 개선을 권고합니다.
- **심각도:** P2 (Warning / 에러 진단성 및 API 일관성)

---

## 4. 품질 게이트 및 정적 분석 검증

| 항목 | 명령 / 도구 | 결과 | 비고 |
|---|---|---|---|
| 포맷 검사 | `cargo fmt --check` | Pass | 스타일 컨벤션 100% 준수 |
| 정적 분석 | `cargo clippy --all-targets -- -D warnings` | Pass | 경고 0건 |
| 단위/통합 테스트 | `cargo test` | Pass | 14 unit + 19 fault + 13 cfg + 2 dep + 21 storage + 19 retention + 10 query + 12 replay + 17 doc = 총 125개 테스트 성공 |
| RT 제약 불변식 | `cargo test --manifest-path tests/rt-allocation/Cargo.toml` | Pass | RT 경로 힙 동적 할당 및 해제 0건 유지 (10개 테스트 Pass) |
| Unsafe / 메모리 모델 | `cargo +nightly-2026-10-26 miri test -p dot-logger --test tc_006_replay` | Pass | Miri 12개 검증 Pass (UB/누수 0건, 33.35s) |

---

## 5. 다음 액션

- 본 리뷰는 **Strict Read-Only** 원칙에 따라 제품 코드 수정 없이 생성되었습니다.
- 후속 개발 세션에서 RV-029(P3), RV-030(P3), RV-031(P2) 권고사항에 대한 판정(수용/대안 등) 및 TDD 기반 조치를 진행할 수 있습니다.


## 6. 개발 세션 반입·처리 — 2026-10-31

### [처리담당] RV-029

- 판정: 대안 수용(A — 기존 MSRV/예약 알고리즘 유지). 설치된 nightly 표준 라이브러리 소스는 `try_update` stable since 1.95.0, `fetch_update` deprecation since 1.99.0을 명시한다. 개발1.96의 clippy Pass와 nightly 경고는 서로 다른 툴체인 관측이다.
- `try_update` 직접 교체는 MSRV1.85에 맞지 않으므로 `QueryBudget::reserve` 한 함수에서만 `#[allow(deprecated)]`를 적용했다. AcqRel/Acquire·checked_add·한도·permit 반환은 변경하지 않았다. 문서/경고 처리 전용 형식적 Red는 만들지 않았다.
- 근거: 설치 툴체인 원문 발췌(비공개 원시 근거), EV-020의 두 Rust 회귀 및 `RUSTFLAGS=-D deprecated` Miri Pass.

### [처리담당] RV-030

- 판정: 수용(A — 기존 성분·배속 동작 보존 및 비교 편의 API). `Eq/PartialEq`는 성분 비교를 유지하며 `is_equivalent(&other)`만 비율 비교를 제공한다. u32 성분의 교차곱은 u64에 항상 들어간다. `set_speed`의 anchor 전환 정책은 변경하지 않았다.
- 공개 API 부재 compile Red → 구현 Green. 1/1 대 MAX/MAX·3/2 대6/4·MAX/(MAX-1) 자기 비교와 비동등/대칭·원 성분 보존을 검증했다.

### [처리담당] RV-031

- 판정: 수용(A — 오류 variant/복구·종료 전략을 유지하는 표시 개선). InvalidInput/InvalidState/Limit/StaleGeneration에 원인별 설명, Query에는 기존 QueryError Display를 위임해 원 IO 진단을 보존했다.
- 원 리뷰의 타 모듈과의 불일치 근거는 부분 정정한다. QueryError 역시 Debug 기반 Display를 사용한다. 이번 수용 근거는 replay 사용자 진단성 개선이다. QueryError나 Error::source 계약을 확대하지 않았다.
- 기존 Debug 문자열 대비 행동 Red → 설명 문자열 Green. 실제 io::Error의 PermissionDenied 분류/원인 텍스트가 감싸진 뒤에도 보존됨을 검증했다.

### 검증·수락 경계

[EV-020](../verification_evidence.md#ev-020)·source54개/검사10개·로그 hash(비공개 원시 근거): 두 Rust workspace all-features/consumer/clippy/rustdoc·fmt 및 replay14 Miri 단일 seed Pass. RT guard10·할당/해제0/0, query/replay 예약 내 peak·잔여0 유지. RT/unsafe·의존성/lock·소유권·오류 분류/terminal 정리 변경 없음. code-review 스킬 기준 자가 점검에서 새 P1/P2 미해결 사항은 확인하지 못했다.

원 리뷰 §1~5의 Read-Only/시험 수치는 이전 리뷰 세션의 기록이며 이번 개발 검증은 EV-020으로 구분한다. 상태 이력은 Open → In Progress → Resolved다. 기존 EV-018/019·REV-015/016 Human 승인은 유지하며 이번 추가 조치만 최종 승인 대기다. native 공개 후(T3)·PC 성능/실제 timer·UPS 시뮬레이션·다른 OS는 미실행, TASK-011/MS-004 In Progress·Gate4/5 Not Ready 유지. 소스/문서는 미커밋·미게시다.

## Human 최종 구현 승인 — 2026-10-31

Human 「승인, 다음작업 진행하자」로 직전 보고의 TASK-008 REV-009/010, TASK-010 REV-013/014, TASK-011 REV-017 구현·리뷰 조치를 수락했다. 본 리뷰와 해당 RV 항목은 Closed다. EV-020의 검증 근거 및 현재 구현 범위를 승인하며, 앞선 Resolved/승인 대기 본문은 승인 전 이력으로 보존한다. 낮은 ReadPin 유실의 강제 회수 없음·실제 reader close 뒤 반환 계약과 native 공개 후(T3)·현재 PC 실측/UPS 시뮬레이션 잔여는 유지한다. Task/MS 및 Gate4/5 완료를 뜻하지 않는다. 다음 작업은 TASK-022 프로필 구체화이며 커밋/푸시는 수행하지 않았다.

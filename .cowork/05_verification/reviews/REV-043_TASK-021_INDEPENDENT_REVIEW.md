# [REV-043] TASK-021 UPS 긴급 계획·모의 시험 독립 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-11-02 / AI 독립 코드리뷰 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 코드리뷰. REV-042는 개발 세션 내 자가 점검이었으므로, TASK-021 UPS 긴급 계획·원자 알람 메일박스·순위 파일 동기화·프로세스 중단 및 복구 시뮬레이션의 전체 구현 형상에 대해 독립적 관점에서 심층 검토를 수행했다. 제품 코드는 전혀 수정하지 않았다.
- **대상 형상:** 커밋 `a5fafcd` 및 TASK-021 S1~S3 구현 파일 전체
  - 코어 제어 및 전원 알람 계약:
    - [`crates/dot-logger/src/control/emergency.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/control/emergency.rs)
    - [`crates/dot-logger/src/platform/power.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/platform/power.rs)
    - [`crates/dot-logger/src/control/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/control/mod.rs)
    - [`crates/dot-logger/src/platform.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/platform.rs)
  - 모의 시험 및 검증 예제:
    - [`crates/dot-logger/examples/ups_simulation.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/examples/ups_simulation.rs)
    - [`crates/dot-logger/examples/support/ups/mod.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/examples/support/ups/mod.rs)
  - 테스트 및 런타임 제약 검증:
    - [`crates/dot-logger/tests/tc_011_emergency.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/tests/tc_011_emergency.rs)
    - [`crates/dot-logger/src/control/tests.rs`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/control/tests.rs)
    - [`tests/rt-allocation/src/tests.rs`](file:///home/lim8603/src/dot-logger/tests/rt-allocation/src/tests.rs)
- **정적 및 동적 검증 결과:**
  - `cargo fmt --check`: Pass (포맷 완전 준수)
  - `cargo clippy --all-targets -- -D warnings`: Pass (경고 0건)
  - `cargo test`: Pass (22 unittests + 19 fault_storage + 13 tc_001 + 16 tc_010 + 11 tc_011 + 17 doctests 전체 통과)
  - `cargo test -p dot-logger-rt-allocation`: 16/16 Pass (비상 알람 통지 및 차단 거절 경로의 0 할당 / 0 해제 불변식 완벽 보존)
  - `RUSTDOCFLAGS='-D warnings' cargo doc --locked -p dot-logger --all-features --no-deps`: Pass (문서 경고 0건)
  - Safe Rust 100% (신규 구현 코드 내 `unsafe` 블록 0건)
- **종합 판정:** **Pass** (P1 0건, P2 0건, P3 2건). RT 핫패스 무할당·무대기 불변식 완전 보존, 100% Safe Rust, 순위(Rank) 기반 원자적 순차 동기화, 알람 단축 시 기한 안전성 보장, 실물 UPS 하드웨어 의존 없는 완결된 실제 파일시스템 모의 검증 완료.
- **상태:** **Closed** (2026-11-03 #001 개발 세션에서 P3 문서 보강·검증 후 기존 위임으로 수락)

---

### 확인한 긍정 사항

1. **RT(실시간) 경로의 무할당·무대기 불변식 완벽 보존:**
   - `ControlOwner::notify_power`는 락(lock), 메모리 할당(`Box`/`Vec`/`String`), 파일/네트워크 I/O, 재시도 루프가 전혀 없는 원자적 메일박스(`AtomicU64`, `compare_exchange`, `fetch_min`) 연산으로 구성됨.
   - 알람 발생 즉시 원자적 `self.capture.close()`를 수행하여 추가 레코드 수집을 즉시 차단함.
   - RT 할당 검증 테스트(`tests/rt-allocation/src/tests.rs`)에서 비상 알람 주입 및 거절 경로의 메모리 할당/해제가 정확히 `(0, 0)`임을 실증함.
2. **100% Safe Rust 및 완벽한 자원 수명 관리:**
   - 신규 추가된 비상 제어 및 시뮬레이션 코드 전체에 `unsafe` 블록이 단 하나도 없음.
   - 비상 상태가 트리거되거나 기한이 만료되더라도 이미 진행 중인 쓰기/동기화 가드(`SyncWork`, `Delivery`)를 강제 회수하거나 중단하지 않고, 실제 완료 시점까지 수명을 보존함.
   - `ChildOwner` 구조체의 RAII `Drop` 구현을 통해 프로세스 중단 모의 시 자식 프로세스의 누수 및 좀비화를 방지함.
3. **엄격한 순위(Rank) 기반 순차 동기화 및 일반 동기화 격리:**
   - `EmergencyConfig`의 순위(`rank`)에 따라 하위 랭크 타깃은 상위 랭크 타깃이 `Synced` 또는 최종 확정 상태에 도달할 때까지 작업을 배출하지 않음(`emergency_sync_allowed`).
   - 비상 활성화 시 비상 슬롯 외의 기존 일반 sync 큐는 즉시 `EXPIRED`로 전이시켜 비상 전원 차단 작업과의 리소스 경합을 원천 차단함.
4. **알람 단축(Tightened) 및 다중 알람 경합 안전성 (RV-084 완벽 해소 확인):**
   - 최초 이벤트 ID는 원자적 CAS로 보존하고, 시각 메타데이터는 `fetch_min`으로 단조 감소 형태로만 갱신하여 악의적이거나 잘못된 연장 알람을 원천 배제함.
   - `poll_emergency` 수행 중 더 이른 기한의 알람이 들어오더라도 셧다운 폴링 후 기한을 재관측하여 만료 기한의 안전성을 보장함.
5. **실물 하드웨어 의존 없는 결정론적 ext4 모의 검증 실증:**
   - 승인된 ext4 볼륨 상에서 8MiB/64MiB 페이로드, 지연 4조건(0/50/250/1000ms), 2종 Rust 패스 실행을 통해 실제 `fsync` 성능 및 오류 프로필을 완벽 검증함.
   - exit(17)을 통한 비정상 프로세스 중단 시 잘린 꼬리(`Truncated`) 격리 및 사전 독립 oracle과의 일치성 복구를 확인하여 물리적 전원 차단 장비 없이도 충분한 검증 신뢰성을 확보함.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-086 | P3 | [`crates/dot-logger/src/control/emergency.rs#L105-L128`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/control/emergency.rs#L105-L128), `enable_emergency` | 필수 선행조건 위반 시 `ControlError::InvalidConfig` 단일 variant만 반환되어, 호스트 설정자가 구성 실패 원인(필수 Synced 누락, 중복 타깃, 슬롯 상태, 페이로드 예산 초과 등)을 파악하기 어려움 | `enable_emergency` 상단 rustdoc에 `# Preconditions` 또는 `# Errors` 섹션을 추가하여 필수 구성 조건 6가지를 명시 | 수용·조치 | Closed |
| RV-087 | P3 | [`crates/dot-logger/src/control/emergency.rs#L274-L288`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/control/emergency.rs#L274-L288), `observe_protected_bytes` | `observe_protected_bytes`가 타깃당 1회성(One-shot) 관측 계약이며, `u64::MAX` 값은 센티널로 거부되고 중복 호출 시 `ControlError::Busy`를 반환한다는 점이 rustdoc에 명시되지 않음 | `observe_protected_bytes`의 rustdoc에 1회 호출 계약 및 `u64::MAX` 센티널 불허 조건을 명시하여 호스트 연동 시 오해 방지 | 수용·조치 | Closed |

---

## 3. 상세 리뷰 내역

### RV-086: `enable_emergency` 필수 선행조건 rustdoc 가이드 보강 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/control/emergency.rs#L105-L128`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/control/emergency.rs#L105-L128)
- **분석 내용:**
  - `enable_emergency(&mut self, plan: &ValidatedConfig, config: EmergencyConfig)`는 다음 불변식을 정밀하게 검사하며 하나라도 위반 시 `ControlError::InvalidConfig`를 반환한다:
    1. 비상 계획 중복 활성화 불가 (`self.emergency.is_none()`)
    2. 셧다운 미진행 상태여야 함 (`self.shutdown == 0`)
    3. 활성 프로듀서 및 제출 호출이 없어야 함 (`active_calls == 0`, `producers.accepted == 0`, `slots.phase == FREE`)
    4. 비상 파일 목록이 비어있지 않고 타깃 수를 초과하지 않음 (`!files.is_empty()`, `files.len() <= targets.len()`)
    5. 세션 설정의 모든 필수(Required) `Boundary::Synced` 타깃이 파일 목록에 반드시 포함되어야 함
    6. 타깃 ID 중복 불가 및 모든 타깃이 `Boundary::Synced`를 지원해야 함
    7. 각 파일의 `protected_bytes`는 `0` 또는 `u64::MAX`가 아니어야 하며, 총합이 세션 페이로드 예산(`input_bytes + raw_bytes`) 이내여야 함
  - 상위 호스트 설정자가 세션을 초기화할 때 실수가 발생하면 단순히 `InvalidConfig`로만 실패하므로, 코드 내부를 분석하지 않고도 빠르게 원인을 교정할 수 있도록 rustdoc 문서화가 필요하다.
- **처리 제안:**
  - `enable_emergency` 상단 rustdoc에 `# Preconditions` 섹션을 보강하여 위 주요 검증 요건을 명시한다.

#### [처리담당]
- **판정 결과:** 수용·조치 (`Closed`)
- **처리 내용:** `enable_emergency` rustdoc에 `# Preconditions`(계획 미설치·셧다운 전, 동일 memory 설정, 활성 제출/수락 producer/점유 slot 없음, 비어있지 않은 고유 Synced 지원 타깃, 필수 Synced 타깃 포함, `protected_bytes` 0/`u64::MAX` 금지·payload 예산 이내, control 예산)과 `# Errors`(`InvalidConfig`·`InvalidDeadline`·`ConfigError::BudgetExceeded{Control}` 구분)를 명시했다. 새 오류 variant나 동작 변경은 없다.
- **검증 근거:** 소스 `9cbbfb8`; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc --locked -p dot-logger --all-features --no-deps` exit0, tc_011 11/11·doctest 17/17 Pass, 전체 `cargo test` Pass. [rustdoc/검증 원문](../../06_evolution/imported_context/rev043_2026-11-03/rev043-doc.log). 문서만 바꾸므로 동작을 복제하는 새 테스트는 추가하지 않았다.

---

### RV-087: `observe_protected_bytes` 1회성 관측 계약 rustdoc 명시 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/control/emergency.rs#L274-L288`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/control/emergency.rs#L274-L288)
- **분석 내용:**
  - `observe_protected_bytes(&self, target: u32, bytes: u64)`는 `file.observed_bytes.compare_exchange(u64::MAX, bytes, Ordering::AcqRel, Ordering::Relaxed)`를 사용하여 값을 저장한다.
  - 따라서 동일 타깃에 대해 두 번 이상 호출하면 두 번째 호출은 `ControlError::Busy`를 반환한다(Idempotent하지 않음).
  - 또한 `u64::MAX`는 '미관측(Unobserved)' 상태를 나타내는 센티널 값이므로 인자로 전달할 경우 즉시 `ControlError::Busy`를 반환한다.
  - 이 메서드는 호스트 일반 워커가 수집 차단 및 quiescence 상태를 확인한 후 확정된 동결 페이로드 바이트를 단 1회 등록하는 용도이므로, 호출자가 이를 인지할 수 있도록 rustdoc에 명확한 안내가 필요하다.
- **처리 제안:**
  - `observe_protected_bytes` rustdoc에 타깃당 단 1회 보고 계약과 `u64::MAX` 불허 제약을 명기한다.

#### [처리담당]
- **판정 결과:** 수용·조치 (`Closed`)
- **처리 내용:** `observe_protected_bytes` rustdoc에 타깃당 1회 관측(재호출 비멱등·첫 값 유지), `u64::MAX` 미관측 센티널 거부, 한도 초과 값은 기록되나 `unachievable=true`·`protected_complete=false`가 됨을 명시하고 `# Errors`(미설치/비보호 타깃 `InvalidConfig`, cutoff 미동결·센티널·중복 `Busy`)를 추가했다. 동작 변경은 없다.
- **검증 근거:** 소스 `9cbbfb8`; RV-086과 같은 명령 exit0. [rustdoc/검증 원문](../../06_evolution/imported_context/rev043_2026-11-03/rev043-doc.log).


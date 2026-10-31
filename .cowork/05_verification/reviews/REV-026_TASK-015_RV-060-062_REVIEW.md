# [REV-026] TASK-015 RV-060~062 조치 및 후속 형상 검증 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-11-01 / AI 리뷰 전용 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 코드리뷰. 이전 독립 리뷰([REV-025](REV-025_TASK-014-015_INDEPENDENT_REVIEW.md))에서 도출된 지적사항(RV-060, RV-061, RV-062)의 조치 커밋(`eaf736c`) 및 현재 HEAD 형상(`5a2a776`)에 대한 안전성, 계약 정합성, RT 제약 준수 여부를 검증한다. 제품 코드는 수정하지 않았다.
- **대상 형상:** 커밋 `eaf736c` (diff 기준 `a247d01..eaf736c`), 제품 및 테스트 코드 8개 파일 (+684/−36).
  - [capture/mod.rs](../../../crates/dot-logger/src/capture/mod.rs): 카운터 기록 시 거절 에러 범주별 위임(`counter.reject`)
  - [telemetry/counters.rs](../../../crates/dot-logger/src/telemetry/counters.rs): `CaptureCounts` 6개 범주 필드 추가, `CounterCell` 9슬롯 확장(128B 정렬 유지), 단일 relaxed 카테고리 갱신 및 관리 합산, `Bank::reset`의 `clear_poison()`
  - [telemetry/mod.rs](../../../crates/dot-logger/src/telemetry/mod.rs): `history_len`의 `Observation<usize>` 반환 및 poison 복구 단위 테스트
  - [telemetry/report.rs](../../../crates/dot-logger/src/telemetry/report.rs): `Pressure::Overloaded` 및 부하율(`load`) 계산 시 `rejected_capacity`만 반영, `OutputDelta` 호스트 보존·대조 절차 명시, 6개 범주 필드 리포트 노출 및 invalidate 처리
  - [tc_009_report.rs](../../../crates/dot-logger/tests/tc_009_report.rs), [tc_009_telemetry.rs](../../../crates/dot-logger/tests/tc_009_telemetry.rs): 비용량 거절의 정상 압력 유지 검증, 호스트 실제 윈도우 끝점 매칭 통합 테스트
  - [api-consumer](../../../tests/api-consumer/src/main.rs), [rt-allocation](../../../tests/rt-allocation/src/tests.rs): 공개 API 일치 검증, 분류된 거절 에러 발생 시 RT 할당/해제 0 검증
- **정적 및 동적 검증:**
  - `cargo fmt --check`: Pass
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: Pass
  - `cargo test --workspace --all-features`: Pass (전체 단위/통합/문서 테스트 정상 통과)
  - `cargo +nightly-2026-10-26 miri test -p dot-logger --test tc_009_telemetry --locked`: Pass (10개 테스트 Miri 통과)
  - `cargo +nightly-2026-10-26 miri test -p dot-logger --test tc_009_report --locked`: Pass (15개 테스트 Miri 통과)
- **종합 판정:** Pass (P1 0건, P2 0건, P3 0건). RV-060~062의 결함 및 권고사항이 완벽하게 해결되었으며 부작용이나 추가 결함이 발견되지 않음.
- **상태:** Closed — 2026-11-01 #001 리뷰 세션 종합 판정 Pass (P1/P2/P3 0건), Human 세션 종료 요청으로 수락 마감.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 검토 대상 및 문제 요약 | 처리 확인 및 평가 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-063 | Note / P3 | [`CounterCell::reject`](../../../crates/dot-logger/src/telemetry/counters.rs#L125-L167)·[`read`](../../../crates/dot-logger/src/telemetry/counters.rs#L176-L206) | RV-060 거절 범주(Capacity/Contract/Lifecycle) 분리 및 총계 보존 구현 검토 | 10개 `CaptureError` 전체가 3개 범주로 완전 매칭되며 1회 relaxed 연산으로 O(1) 처리됨. 128B 정렬 유지 및 관리 읽기 시 오버플로우 방어 합산 건전함 확인 | 확인 완료 (양호) | Closed |
| RV-064 | Note / P3 | [`OutputDelta`](../../../crates/dot-logger/src/telemetry/report.rs#L10-L18)·[`host_aligns_target_completion_history`](../../../crates/dot-logger/tests/tc_009_telemetry.rs#L349-L437) | RV-061 호스트 윈도우 양 끝점 대조 계약 및 종단 통합 검증 검토 | API 침습적 변경 없이 호스트의 표본 보존/양 끝점 대조 절차가 rustdoc에 상세 명시되었으며, 불일치 케이스를 포함한 통합 테스트로 건전성 입증됨 | 확인 완료 (양호) | Closed |
| RV-065 | Note / P3 | [`Bank::reset`](../../../crates/dot-logger/src/telemetry/counters.rs#L237)·[`Telemetry::history_len`](../../../crates/dot-logger/src/telemetry/mod.rs#L131-L136) | RV-062 독점 reset의 poison 복구 및 history_len 관측치 명확화 검토 | `clear_poison()`을 통한 패닉 후 복구 정상화, `Observation<usize>` 도입으로 미측정과 0의 구분이 확립됨 | 확인 완료 (양호) | Closed |

---

## 3. 상세 리뷰 내역

### RV-063: 거절 범주 세분화 및 부하율/압력 진단 정밀화 (RV-060 조치 검증)

#### [리뷰어]
- **위치:** [`capture/mod.rs#L530-L540`](../../../crates/dot-logger/src/capture/mod.rs#L530-L540), [`counters.rs#L10-L34`](../../../crates/dot-logger/src/telemetry/counters.rs#L10-L34), [`counters.rs#L125-L167`](../../../crates/dot-logger/src/telemetry/counters.rs#L125-L167), [`report.rs#L262-L270`](../../../crates/dot-logger/src/telemetry/report.rs#L262-L270), [`report.rs#L348-L350`](../../../crates/dot-logger/src/telemetry/report.rs#L348-L350)
- **검토 내용:**
  1. **RT 제약 준수:** `CounterCell::reject`는 `CaptureError`와 `acquisition` 불리언을 받아 인덱스를 매핑한 뒤 `self.increment(index, 1)`만 호출한다. 동적 메모리 할당, 락 대기, 문자열 포맷팅, 루프가 전혀 없다. `tests/rt-allocation/src/tests.rs`의 `classified_telemetry_failures_keep_rt_allocation_and_release_zero`에서 거절 발생 중에도 heap 할당/해제가 `(0, 0)`임을 정량적으로 확인했다.
  2. **오차 없는 매핑 완전성:** `CaptureError`의 10개 variant(`InvalidInput`, `SchemaMismatch`, `TooLarge`, `Capacity`, `CompletionCapacity`, `Closing`, `SequenceExhausted`, `GenerationExhausted`, `Unsupported`, `DeadlineExceeded`)와 `acquisition` 여부 2개 조합(총 20개)이 와일드카드 `_` 없이 명시적으로 매핑되어 있다. 향후 새로운 오류 타입 추가 시 컴파일 타임에 누락을 방지한다.
  3. **압력 및 부하율 왜곡 해소:** `Report::build`의 `loss` 판정식이 `rejected_capacity > 0 || acquire_rejected_capacity > 0 || output 손실`로 정정되었다. `TooLarge`, `Unsupported`, `Closing` 같은 호출자 계약 위반이나 수명 종료 오류가 발생하더라도 허위 `Pressure::Overloaded`를 유발하지 않으며, 부하율 분자에도 `accepted + rejected_capacity`만 반영되어 "용량 관점의 시도 수요(attempt demand)"가 정확히 계산된다.
  4. **방어적 총계 관리:** `CounterCell::read()`에서 개별 카운터의 값을 로드한 후 `rejected = total([1, 5, 6])`, `acquire_rejected = total([4, 7, 8])`로 합산할 때 `checked_add`를 사용하여 `u64` 오버플로우 발생 시 `overflowed = true` 및 `u64::MAX`로 포화 처리한다.
- **평가:** 요구사항과 RT 제약을 완벽히 충족함. 결함 없음.

#### [처리담당]
- 판정 결과: 수용 — Pass 판정으로 추가 코드 수정 없이 수락.
- 검증 근거: `cargo test` 및 Miri, `rt-allocation` RT 무할당 `(0, 0)` 통과.

---

### RV-064: 호스트 윈도우 끝점 대조 계약 및 검증 (RV-061 조치 검증)

#### [리뷰어]
- **위치:** [`telemetry/report.rs#L10-L18`](../../../crates/dot-logger/src/telemetry/report.rs#L10-L18), [`tc_009_telemetry.rs#L349-L437`](../../../crates/dot-logger/tests/tc_009_telemetry.rs#L349-L437)
- **검토 내용:**
  1. **계약 명세:** `OutputDelta` rustdoc 주석에 호스트가 성공한 표본마다 `target_totals`를 유한 이력에 보존하고, `window.from_ns`와 `window.to_ns`를 단순 계산(`to_ns - seconds`)이 아닌 실제 탐색된 끝점 시각과 정확히 매칭해야 한다는 절차가 명시되었다.
  2. **종단 테스트:** `host_aligns_target_completion_history_to_actual_window_endpoints` 테스트를 통해 실제 윈도우 시작점(0s)과 요청 시간 차감 시각(11s - 10s = 1s)이 어긋나는 조건에서 호스트가 실제 끝점을 찾아 차분을 구성하고, 유효한 `OutputDelta`를 `Report::build`에 공급하여 올바른 진단 결과를 도출하는지 검증했다.
- **평가:** 추가적인 복잡한 내부 이력 관리자 도입 없이도 호출자 계약과 종단 테스트로 일관성을 확보함. 결함 없음.

#### [처리담당]
- 판정 결과: 수용 — Pass 판정으로 추가 코드 수정 없이 수락.
- 검증 근거: `tc_009_telemetry` 종단 테스트 통과.

---

### RV-065: Mutex poison 복구 및 history_len 관측치 개선 (RV-062 조치 검증)

#### [리뷰어]
- **위치:** [`telemetry/counters.rs#L237`](../../../crates/dot-logger/src/telemetry/counters.rs#L237), [`telemetry/mod.rs#L131-L136`](../../../crates/dot-logger/src/telemetry/mod.rs#L131-L136), [`telemetry/mod.rs#L147-L179`](../../../crates/dot-logger/src/telemetry/mod.rs#L147-L179)
- **검토 내용:**
  1. **Poison 복구:** 독점 `&mut self`를 보장하는 `Bank::reset`에서 이력을 초기화한 후 `self.history.clear_poison()`을 호출한다. MSRV 1.85 환경에서 안전하게 동작하며, 패닉 발생 후에도 캡처 오너 재할당 없이 계측을 정상 복원할 수 있다.
  2. **관측치 원칙 준수:** `Telemetry::history_len()`이 기존 `usize`에서 `Observation<usize>`를 반환하도록 변경되었다. 락이 poison된 상태에서는 `Observation::Unavailable(Unavailable::HistoryUnavailable)`을 반환하여, "측정되지 않은 데이터는 0으로 보고하지 않는다(Missing data is not reported as zero)"는 모듈 핵심 불변식을 엄격히 만족한다.
  3. **재현 테스트:** `exclusive_reset_recovers_poisoned_history` 테스트에서 의도적 패닉 주입 후 `Unavailable` 확인 → `reset` 후 `Observed(0)` 및 후속 표본 수집 확인이 완벽히 검증되었다.
- **평가:** 언어 안전성 및 아키텍처 불변식에 부합함. 결함 없음.

#### [처리담당]
- 판정 결과: 수용 — Pass 판정으로 추가 코드 수정 없이 수락.
- 검증 근거: `exclusive_reset_recovers_poisoned_history` 테스트 및 Miri 통과.

---

## 4. 검토하지 않은 범위 및 후속 안내

- **실제 PC/RT 실측 오차(≤20%) 및 계측 비용(≤5%):** fixtures/harness 구조는 확인되었으나, 실제 RT 환경(PREEMPT_RT 커널) 부팅 및 벤치마크 실측은 TASK-022 범위로 계속 유지된다.
- **호스트 원격 런타임 및 실앱 적용:** 원격 수집 백엔드 및 실 애플리케이션(TASK-020 등) 통합은 후속 작업에서 진행된다.
- **차기 개발 작업:** 다음 계획된 작업 단위는 `TASK-016`(revision 설정 변경과 전후 비교)이다.


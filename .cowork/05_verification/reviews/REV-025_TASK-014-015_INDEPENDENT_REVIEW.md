# [REV-025] TASK-014 운영 계측·TASK-015 리포트 독립 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-10-31 #013 / AI 리뷰 전용 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 리뷰. REV-023/024는 개발 세션 자가 점검이었다. 그래서 마지막 독립 리뷰(REV-022, 소스 `b3c08bd`) 이후의 제품 변경을 리뷰어 관점에서 다시 검토했다. 제품 코드는 수정하지 않았다.
- **대상 형상:** 소스 `a247d01` (HEAD `d953e1d`는 문서 변경만 포함). `b3c08bd..a247d01` 제품 diff 16개 파일, +2139/−5.
  - TASK-014 `d99ba3b`: [capture/mod.rs](../../../crates/dot-logger/src/capture/mod.rs) enable/reset/counter 호출 지점, [completion/mod.rs](../../../crates/dot-logger/src/completion/mod.rs) `target_totals`/`selected_totals`, [telemetry/mod.rs](../../../crates/dot-logger/src/telemetry/mod.rs)·[counters.rs](../../../crates/dot-logger/src/telemetry/counters.rs)·[aggregate.rs](../../../crates/dot-logger/src/telemetry/aggregate.rs)
  - TASK-015 `a247d01`: [capacity.rs](../../../crates/dot-logger/src/telemetry/capacity.rs)·[report.rs](../../../crates/dot-logger/src/telemetry/report.rs), TC-009 시험·api-consumer·rt-allocation 추가분, README
- **정적 확인:** `cargo fmt --all --check` Pass. `cargo clippy --workspace --all-targets --all-features -- -D warnings` Pass. `cargo test --workspace --all-features` 전체 Pass. Miri는 EV-028과 소스가 같아 다시 실행하지 않았다. 아래 반례는 코드 정독과 기존 시험 대조로 도출했다. 실행 재현은 처리담당 Red 시험에 맡긴다.
- **종합 판정:** Conditional (P2 1, P3 2). P1 없음. RT 할당·대기·unsafe 위반은 확인하지 못했다. 다만 리포트의 병목 진단 의미에 결함이 1건 있다.
- **상태:** Open

### 확인한 긍정 사항

- **RT 경로:** [`CounterCell::record`](../../../crates/dot-logger/src/telemetry/counters.rs#L216-L229)는 relaxed load/store만 사용한다. RMW, 재시도, 할당, 포맷팅이 없다. 포화하면 wrap하지 않고 `u64::MAX`와 overflow flag를 남긴다. rt-allocation 시험이 활성 상태의 수락·거절·생략·raw·Drop에서 할당 0을 확인한다.
- **단일 writer:** [`CaptureOwner::scope`](../../../crates/dot-logger/src/capture/mod.rs#L368)는 `&mut self`를 받는다. 그래서 같은 producer cell에 동시에 존재하는 `Producer`는 하나뿐이다. `reset_telemetry`도 `&mut self`라서 capture/telemetry 대여와 겹치지 않는다. relaxed 갱신이 허용되는 조건이 타입으로 보장된다.
- **예산:** `enable_telemetry`는 cell·`Bank`·301개 slot 크기를 checked 연산으로 합산한다. 그 뒤 control 예산을 먼저 검사하고 할당한다. rt-allocation 시험이 live/peak 메모리를 예약 범위 안에서 확인한다.
- **이력/구간:** `History::push`는 1초 미만 간격과 역행을 거절한다. `window`는 연속 표본의 2초 초과 gap, epoch 변화, 포화를 Unavailable로 처리한다. reset은 history를 비우므로 큰 delta가 생기지 않는다.
- **리포트 산술:** `horizon`의 곱셈은 u64×u64라서 u128에 들어간다. 0 구간·0 능력·음수 증가가 수치 0이나 허위 기한으로 바뀌지 않는다. `first`는 AtLimit과 유한 한계를 BeyondRange보다 우선한다. `buffer()` 검증 뒤의 `capacity - end` 뺄셈은 underflow하지 않는다.
- **대상별 합계:** `selected_totals`의 retired 이력 index `i`는 기존 `required_only` 필터와 같은 `targets[i]` 정렬을 사용한다. live slot은 target id로 거른다. 기존 `totals` 의미는 그대로다.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-060 | P2 | [`try_record_small`](../../../crates/dot-logger/src/capture/mod.rs#L526-L543)·[`try_acquire`](../../../crates/dot-logger/src/capture/mod.rs#L647-L659)·[`try_record_raw`](../../../crates/dot-logger/src/capture/mod.rs#L683-L702) → [`Report::build` loss/load](../../../crates/dot-logger/src/telemetry/report.rs#L246-L252)·[L327-L334](../../../crates/dot-logger/src/telemetry/report.rs#L327-L334), API§15.1 | `rejected`/`acquire_rejected`가 모든 `CaptureError`를 한 값으로 센다. 리포트는 0보다 크면 Observed `Overloaded`로 보고 부하율 분자에도 넣는다. 호출자 계약 오류·종료 중 거절·재시도가 용량 병목으로 진단된다 | 거절을 용량/계약/수명 범주로 나눠 세고, 용량 범주만 Overloaded·부하율에 사용 (계약 문구 변경 동반) | — | Open |
| RV-061 | P3 | [`Sample.outputs`](../../../crates/dot-logger/src/telemetry/aggregate.rs#L16-L20)·[`Window`](../../../crates/dot-logger/src/telemetry/aggregate.rs#L24-L37)·[`ReportInput.output`](../../../crates/dot-logger/src/telemetry/report.rs#L82-L83) | 같은 구간의 대상별 `OutputDelta`를 만들 수단이 없다. 이력은 전체 대상 합계만 보존하고 과거 표본 조회 API도 없다. `from_ns`는 요청 초보다 최대 2초 이를 수 있다 | 구간 끝점 표본 노출 또는 선택 대상 합계 이력화. 최소한 호스트 절차 문서·연결 시험 | — | Open |
| RV-062 | P3 | [`Bank::reset`](../../../crates/dot-logger/src/telemetry/counters.rs#L262-L272)·[`history_len`](../../../crates/dot-logger/src/telemetry/mod.rs#L129-L132) | reset은 poison된 이력을 비우지만 poison 표시는 그대로 둔다. 그래서 owner 수명 동안 복구되지 않는다. `history_len`은 poison 시 0을 반환한다 ("미측정≠0" 원칙 위반) | reset에서 `clear_poison()` 호출(MSRV 1.85에서 사용 가능), `history_len`을 `Option`/`Observation`으로 | — | Open |

---

## 3. 상세 리뷰 내역

### RV-060: 거절 범주 혼합으로 인한 허위 Overloaded·부하율 과대 (P2)

#### [리뷰어]
- **위치:** [capture/mod.rs#L533-L540](../../../crates/dot-logger/src/capture/mod.rs#L533-L540), [#L652-L656](../../../crates/dot-logger/src/capture/mod.rs#L652-L656), [#L691-L698](../../../crates/dot-logger/src/capture/mod.rs#L691-L698), [report.rs#L246-L252](../../../crates/dot-logger/src/telemetry/report.rs#L246-L252), [report.rs#L327-L334](../../../crates/dot-logger/src/telemetry/report.rs#L327-L334)
- **발견한 문제:** 세 호출 지점은 `Err(_)`를 모두 `rejected`(index 1)로, 실패한 `try_acquire`는 모두 `acquire_rejected`로 센다. [`CaptureError`](../../../crates/dot-logger/src/capture/types.rs#L133-L154) 11개 variant는 성격이 다르다.
  - 용량: `Capacity`, `CompletionCapacity`, (general 경로의) `DeadlineExceeded`
  - 호출자 계약: `InvalidInput`, `SchemaMismatch`, `TooLarge`, `Unsupported`
  - 수명/고갈: `Closing`, `SequenceExhausted`, `GenerationExhausted`

  `Report::build`는 `w.counts.rejected > 0 || w.counts.acquire_rejected > 0`이면 `Value::Observed(Pressure::Overloaded)`를 반환한다. 또 `accepted + rejected`를 "offered" 부하율 분자로 사용한다. API§15.1의 "reject/acquire 실패 … 는 Observed Overloaded"도 같은 가정을 따른다. TASK-014 counter 문서는 rejected에 "invalid input and closed ingress"가 포함된다고 밝혀 두었다. 두 계약이 서로 다른 의미를 전제한다.
- **반례 (코드 경로로 확정, 결정적):**
  1. 한 producer가 등록 slot보다 큰 payload를 보낸다(`TooLarge`). [tc_009_telemetry](../../../crates/dot-logger/tests/tc_009_telemetry.rs#L34-L37)에서 확인했듯 `rejected=1`이 된다. 버퍼·출력·디스크에 여유가 있어도 그 구간의 리포트는 `pressure = Observed(Overloaded)`다. 실제 원인은 호출자 버그인데 출처가 "관측"인 용량 과부하로 표시된다.
  2. 정상 종료 중 `Closing` 뒤에 생산자가 계속 제출하면 마지막 10/60초 리포트가 Overloaded가 된다.
  3. raw 경로는 실패 시 loan을 돌려줘 재시도를 허용한다. 같은 기록을 N번 재시도하면 `rejected`가 N 증가한다. 따라서 부하율 분자가 "제출 시도"가 되어 offered records보다 커진다. API§15.1은 acquire 실패를 재시도 중복 때문에 분자에서 뺐다. 같은 근거가 rejected submission에도 적용된다.
  4. raw pool이 없는 producer의 `try_acquire`(`Unsupported`)도 `acquire_rejected`가 되어 Overloaded를 만든다.
- **영향:** TASK-015의 목적은 병목 진단이다. 그런데 계약 오류와 용량 부족을 구분할 수 없고, 부하율이 실제보다 높아질 수 있다. 데이터 손실·RT 침해는 아니다.
- **처리 제안:**
  1. `CounterCell`에 거절 범주 counter를 추가한다. 예: `rejected_capacity`, `rejected_contract`, `rejected_lifecycle`. acquire도 같은 방식으로 나눈다. variant→index match 한 번과 relaxed store 하나로 RT 비용은 그대로다. 현재 `values` 5개(41B)이고 `align(128)` 안에 15개까지 들어간다.
  2. `Report`의 `loss`와 부하율 분자에는 용량 범주만 사용한다. 계약/수명 거절은 별도 필드(예: `contract_rejected`)로 보존하고 Overloaded로 승격하지 않는다. 분자를 "시도"로 유지할지, 재시도 중복 한계를 문서화할지는 처리담당이 결정한다.
  3. API§15.1의 "reject/acquire 실패는 Observed Overloaded" 문구와 `CaptureCounts` 공개 필드를 바꾸는 계약 변경이다. 처리 세션에서 [권한 매트릭스](../../01_cowork_protocol/decision_authority_matrix.md) 수준을 먼저 확인한다(기존 위임 범위 안의 계약 정밀화인지, H/J 판단 대상인지).
  4. Red 시험: `TooLarge`/`Closing`/`Unsupported`만 발생하고 자원에 여유가 있는 window → `Overloaded`가 아니어야 한다. `Capacity` 거절 window는 계속 Overloaded여야 한다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-061: window와 정렬된 대상별 OutputDelta를 만들 수 없음 (P3)

#### [리뷰어]
- **위치:** [aggregate.rs#L16-L37](../../../crates/dot-logger/src/telemetry/aggregate.rs#L16-L37), [aggregate.rs#L94-L138](../../../crates/dot-logger/src/telemetry/aggregate.rs#L94-L138), [report.rs#L73-L88](../../../crates/dot-logger/src/telemetry/report.rs#L73-L88)
- **발견한 문제:** `ReportInput`은 "같은 window의" 대상별 `OutputDelta`를 요구한다. 그러나 라이브러리는 다음을 제공하지 않는다.
  - `Sample.outputs`는 전체 대상(`totals(false)`) 합계다. 대상별(`target_totals`)이 아니다.
  - `Window`는 capture 차분만 담는다. 이력 내부 표본은 `latest()` 외에 조회할 수 없다.
  - `window(seconds)`의 `from_ns`는 탐색 결과로 정해진다. 실제 구간은 `seconds`~`seconds+2초`다. 그래서 호스트가 같은 시각을 미리 알 수 없다.

  결과적으로 호스트는 매 `sample` 시각마다 `target_totals`를 따로 저장하고 `from_ns`로 찾아야 한다. 구간이 어긋나도 `Report::build`는 감지하지 못한다. 그러면 `accepted_rate`와 `completed_rate`가 서로 다른 구간으로 계산된다. 저장소의 연결 시험([tc_009_telemetry#L211](../../../crates/dot-logger/tests/tc_009_telemetry.rs#L211))도 output을 공급하지 않아 이 절차가 검증되지 않았다. 이력이 보존하는 `Sample.outputs`는 리포트 경로에서 쓰이지 않는다.
- **처리 제안:** (택1) ① `Telemetry::window`가 끝점 두 `Sample`을 함께 반환하거나 `sample_at(at_ns)`를 제공한다. ② enable 시 선택한 대상의 `target_totals`를 이력에 함께 보존한다. ③ 현재 API를 유지하고 README/API§15.1에 호스트 절차(표본별 대상 합계 저장 → `from_ns`/`to_ns` 일치 확인 → `pending`은 끝점 값)를 명시한다. 그리고 실제 completion이 있는 window→OutputDelta 연결 시험을 추가한다. 공개 API 확장은 TASK-016/020 소비 시점에 결정해도 된다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-062: 이력 poison이 reset으로 복구되지 않음·history_len 0 반환 (P3)

#### [리뷰어]
- **위치:** [counters.rs#L262-L272](../../../crates/dot-logger/src/telemetry/counters.rs#L262-L272), [mod.rs#L129-L132](../../../crates/dot-logger/src/telemetry/mod.rs#L129-L132), [capture/mod.rs `enable_telemetry`](../../../crates/dot-logger/src/capture/mod.rs#L280-L283)
- **발견한 문제:** `Bank::reset`의 `Err(e) => e.into_inner().clear()`는 데이터만 비우고 `Mutex`의 poison 표시는 지우지 않는다. 그래서 reset 뒤에도 `sample`은 `HistoryUnavailable`, `window`/`latest`는 `Unavailable(HistoryUnavailable)`을 계속 반환한다. 이미 활성화된 경우 `enable_telemetry`는 `Ok(())`로 바로 반환하므로 owner를 다시 만드는 것 외에 복구할 방법이 없다. reset 주석은 독점 접근을 근거로 안전하게 비운다고 설명하는데, 그렇다면 poison 해제도 같은 근거로 정당하다. 또 `history_len()`은 poison 시 `0`을 반환한다. "Missing data is not reported as zero"라는 모듈 원칙과 맞지 않는다.
- **현실성:** 현재 lock 구간(`push`/`window`/`latest`)에서 panic 경로를 찾지 못했다. 발생 가능성은 낮고 향후 변경에 대한 방어 항목이다.
- **처리 제안:** reset에서 비운 뒤 `self.history.clear_poison()`을 호출한다(Rust 1.77 안정화, MSRV 1.85 충족). `history_len`은 `Option<usize>` 또는 `Observation<usize>`로 바꾸거나, poison 시 의미를 rustdoc에 명시한다. 단위 시험에서 강제 poison → reset → sample 성공을 확인한다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

---

## 4. 검토하지 않은 범위·후속

- `examples/telemetry_overhead.rs`는 fixture 구조만 확인했다. 비용 ≤5%·PC/RT 실측은 TASK-022/023 범위이며 이번 리뷰가 판정하지 않는다.
- Miri, native, 실제 장비 시험은 실행하지 않았다. EV-027/028의 기존 결과와 미실행 표시를 그대로 유지한다.
- RV-060은 공개 `CaptureCounts`/`Report` 필드와 API§15.1 문구를 바꿀 수 있다. TASK-016(revision 비교)이 같은 counter를 비교 근거로 소비하므로 TASK-016 착수 전에 처리하는 것을 권고한다.

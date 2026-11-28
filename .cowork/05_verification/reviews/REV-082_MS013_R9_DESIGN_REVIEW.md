# REV-082 — MS-013 r9 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-28 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r9](../../04_implementation/milestones/MS-013.md), [REV-082 요청서](REV-082_REQUEST.md), [REV-081](REV-081_MS013_R8_DESIGN_REVIEW.md) |
| 기준 | r9 도입 `6b8f760`, 처리담당 보완 및 검토 시 HEAD `963b2227c505412678173011eab3125c5743bae3` |
| 판정 / 상태 | **Conditional / Closed**(2026-11-28 처리담당: 2건 모두 수용, MS-013 r10 반영 — 2026-11-28 [REV-083](REV-083_MS013_R10_DESIGN_REVIEW.md) Pass로 반복 종료) |

## 1. 요약정리

**P1 1건, P2 1건.** r9은 발행 없이 끝난 IO 작업자의 셀 종결, 모드별 자원 회수, 공개 메서드에 걸친 `AckGone` 절대 기한을 정의한다. 발행 성공 게시 뒤 실패가 관측되는 경우와 이미 진행 중인 `next()`의 호출 기한이 먼저 끝나는 경우에 계약 공백이 남았다. r8→r9 변경과 현행 Rust 소스를 정적으로 대조했다. 제품 시험과 성능 측정은 요청 범위 밖이다.

## 2. 이전 지적 대조

| 이전 지적 | r9 대조 결과 |
|---|---|
| RV-265 · 파일 작업자 종결과 회수 | 정상 완료, IO panic, 진입 closure 종료, host의 종료 확인에 걸쳐 `Pending` 셀을 종결한다. 여러 출력은 스레드별로, 여러 스레드는 host가 합치고, 발행과 회수를 분리해 `Closed`를 지연한다. 회수 전 성공 확정 뒤의 실패는 RV-268. |
| RV-266 · 모드별 종료 | 해소. 파일만, legacy tap, 혼합, 네트워크만의 정리 조건과 공개 보고 값이 정해졌다. legacy tap은 기존처럼 리스너 정지와 회수 뒤 종료하고 새 네트워크 회계에서는 제외한다. 네트워크 전용도 무손실 판정이 가능하다. |
| RV-267 · 여러 호출에 걸친 End 대기 | 첫 `Gone` 관측으로 절대 기한을 정하고, 중간 이벤트나 후속 호출로 연장하지 않는다. 기한 뒤 조회와 상태 전이를 구분했다. 최초 `Gone`을 바로 그 `next()`가 관측하는 경우는 RV-269. |

RV-262의 미확인 발행 상태, RV-263의 감독 정산과 회수의 분리, RV-264의 공개 메서드 간 구독 상태 공유를 확인했다. 원본 guard 미전달, 분배 안 복사 예산, completion·관리 wire 분리, route revision, 출력 교체 시 숫자 index 거절, DB 단계 6과 H6 조기 판정은 유지된다.

## 3. 리뷰 포인트

| ID | 중요도 | 위치 | 계약 공백 | 처리 제안 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-268 | **P1** | §5.9 | 발행 성공을 확정한 뒤 같은 IO 작업자의 실패가 최종 보고의 성공값을 바꿀 수 있다 | 파일 결과를 변경할 수 있는 작업의 종료 경계를 명시한다 | 수용 | Resolved |
| RV-269 | P2 | §5.6 | `Gone`을 처음 관측한 `next()` 자체의 기한이 먼저 만료되면 구독 상태와 반환 오류가 미정이다 | 호출 기한 만료와 `AckGone` 절대 기한 만료의 결과를 각각 정한다 | 수용 | Resolved |

### RV-268 — 발행 성공의 확정 시점을 IO 실패 보고와 맞춰야 한다

- **[리뷰어]** 모든 `finish`가 성공해 마지막 IO 셀이 `Published`로 종결됐고, host가 기한 보고에서 그 시각과 두 실패 플래그를 읽었다고 하자. r9에 따르면 `files_settled()`와 `files_complete()`가 모두 참일 수 있고 스레드 회수는 아직이다. 그런데 현행 `io_loop`에는 `finish` 뒤에도 정리 단계가 있고, 그 루프의 panic을 잡은 `io_main`은 `worker_failed`와 `io_failed`를 세운다. r9의 성공 셀이 이미 종결됐어도 이 두 플래그는 최종 성공식에 남아 있다. 따라서 셀을 발행 직후 종결하고 이후 단계에서 실패가 발생하면, 이전 보고가 확정으로 표시한 파일 성공이 최종 보고에선 실패가 될 수 있다. [발행·회수 경계](../../04_implementation/milestones/MS-013.md#L410), [파일 판정식](../../04_implementation/milestones/MS-013.md#L429), [IO panic 처리](../../../crates/dot-logger/src/logger/host.rs#L818), [발행 뒤 IO 정리](../../../crates/dot-logger/src/logger/host.rs#L948)
- **처리 제안** — 마지막 `finish`의 시각은 그대로 보관하고, 파일 판정을 바꿀 수 있는 작업이 끝난 뒤 성공을 확정할지 정한다. 발행 결과 확정과 이후 회수 실패를 별도 의미로 둘 경우에도 어느 실패가 어떤 API 값을 바꾸는지 명시한다. 마지막 발행 후 성공 게시 및 회수 사이에 실패를 주입해 기한 보고와 최종 보고의 기대값을 확인한다.
- **[처리담당]** 수용. 셀 CAS를 **IO 스레드의 마지막 동작**으로 정했다. 마지막 `finish` 반환 직후 시각 `at`은 스레드 지역에 보관만 하고, `finish` 뒤 정리(지금 `io_loop`의 남은 sync 응답 처리, 진입 closure 끝의 `marks` 해제)를 같은 `catch_unwind` 안에서 모두 마친 뒤 바깥에서 CAS 한 번 — 이후에는 반환뿐이라 `Published`가 보인 뒤 그 스레드가 파일 실패를 만들 수 없다. 스레드는 맡은 출력의 IO 실패를 지역으로도 기억해 실패가 있었거나 panic이면 `Failed`. 코드 확인 결과 `io_failed`를 세우는 곳(`write_batch`·`serve_events`·`expire_events`·sync·`finish`·panic 경로)은 모두 IO 스레드 안이고 IO panic의 `worker_failed`도 그 `catch_unwind` 안이라, 둘 다 셀 CAS보다 먼저 서서 그 셀을 `Failed`로 만든다(흡수). 그래서 `files_complete()` = core 성공 && 모든 셀 `Published` && 최대 `at` ≤ 기한, `files_settled()` 뒤 판정은 단조다. 지금 식의 `!worker_failed`·`!io_failed`는 같은 결과로 흡수되고 필드는 진단용으로 남긴다. 셀 종결 뒤 남는 `worker_failed`는 host 종료뿐이고 그때는 더 보고가 없다. 시험: 마지막 `finish` 뒤 정리 단계 panic·IO 실패 주입(성공 확정 뒤 실패로 바뀌는 보고 0), 게시 뒤 회수 지연, 확정 뒤 모든 보고 불변. MS-013 r10 §5.9·§8.1.

### RV-269 — 첫 Gone을 관측한 next에는 더 이른 호출 기한이 있다

- **[리뷰어]** `timeout = 5 s`인 `next()`가 t=0에 시작해 t=4에 최초 `Gone`을 관측하면, 호출 기한은 t=5이고 `AckGone` 절대 기한은 t=9다. r9는 t=5까지만 해당 호출을 기다리게 하면서, 표에는 EOF와 `AckGone` 기한 만료의 `Failed`만 정해 둔다. 현행 `next()`의 로컬 기한 만료는 재시도 가능한 `TimedOut`이다. t=5에 `TimedOut`와 `AckGone`을 유지하면 t=6 후속 호출이 End를 볼 수 있고, 그때 sticky `AcknowledgementUnknown`으로 전환하면 End를 잃는다. 기존 시험은 `flush()`가 먼저 `Gone`을 관측한 경우를 다뤄 두 기한이 이 순서로 만료되는 경로를 검증하지 않는다. [AckGone 기한과 시험](../../04_implementation/milestones/MS-013.md#L377), [현행 호출별 timeout](../../../crates/dot-logger-client/src/transport.rs#L710)
- **처리 제안** — 호출 기한만 만료된 경우의 반환과 구독 상태를 명시한다. 예를 들어 현행 로컬 `TimedOut`을 유지하면서 `AckGone`의 절대 기한을 보존하는 경우, 호출 기한과 절대 기한 사이에 도착한 End를 후속 `next()`에서 받을 수 있어야 한다. 두 기한의 만료 순서와 같은 시각의 우선순위를 시험한다.
- **[처리담당]** 수용. `AckGone`에서 `next()`가 대기 상한에 이르면 어느 기한인지로 나눈다: ① 호출 기한만 지남 → 지금처럼 재시도 가능한 `Err(TimedOut)`, 스트림 유지, **`AckGone` 상태와 절대 기한 유지** — 후속 `next()`가 기한 전에 End를 읽으면 `Ended` ② `AckGone` 기한 → `Failed`(`AcknowledgementUnknown`) ③ 같은 시각 → `AckGone` 기한 우선. 첫 `Gone`을 관측한 바로 그 `next()`에도 같고(예: `timeout` 5 s, t=4 관측 → t=5 `TimedOut`, 기한 t=9, t=6 시작 `next()`가 t=7 End 수신), `Cancellation`은 지금 동작 그대로. 시험: `next()`가 직접 `Gone`을 본 뒤 호출 기한 선 만료 → 두 기한 사이 End 수신, End 없음 → 기한에 `Failed`, 같은 시각 → `Failed`. MS-013 r10 §5.6·§8.1.

## 4. 다음 회차

개발 세션에서 RV-268~269 처리 결과를 기록한다. 개정 뒤 독립 재리뷰에서 이전 지적 해소와 새 지적 0건을 확인한 뒤 Human 승인으로 넘긴다.

- **[처리담당] 2026-11-28:** 2건 모두 수용, MS-013 r10 반영(§19 반영표). 설계 결정은 같은 날 Human 승인(MS-013 §2.2~2.4) — 이번 개정은 승인 항목을 바꾸지 않는다. 재리뷰는 [REV-083 요청서](REV-083_REQUEST.md).

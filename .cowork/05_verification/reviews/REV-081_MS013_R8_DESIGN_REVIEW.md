# REV-081 — MS-013 r8 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-28 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r8](../../04_implementation/milestones/MS-013.md), [REV-081 요청서](REV-081_REQUEST.md), [REV-080](REV-080_MS013_R7_DESIGN_REVIEW.md) |
| 기준 | r8 및 검토 시 HEAD `b392eb7f183fd2b0848f040d8834a0dfe84ea5dd` |
| 판정 / 상태 | **Conditional / Resolved**(2026-11-28 처리담당: 3건 모두 수용, MS-013 r9 반영 — REV-082 대조 대기) |

## 1. 요약정리

**P1 2건, P2 1건.** r8은 host 종료 증거를 분리하고 발행 결과 미확인 상태를 표현한다. 네트워크 계수 확정과 스레드 회수도 분리했다. 파일 작업자 실패 및 출력 종류가 없는 구성에서의 `Closed` 조건, `AckGone`의 대기 기한에 관측 가능한 계약 공백이 남았다. r7→r8 변경과 현재 Rust 소스를 정적으로 대조했다. 제품 시험과 성능 측정은 요청 범위 밖이다.

## 2. 이전 지적 대조

| 이전 지적 | r8 대조 결과 |
|---|---|
| RV-261 · host 종료 증거 | 해소. 실제 host의 `run` 종료 뒤 `host_exited`가 공개된다. IO panic이 `worker_failed`만 세운 경우에는 외부 호출자가 명령 셀을 종결하지 않는다. |
| RV-262 · 파일 기한 판정 | 발행 셀 `Pending`을 기한 초과로 확정하던 문제는 해소됐다. IO panic에서 셀을 종결하는 경로와 실제 회수 조건은 RV-265. |
| RV-263 · 감독 정산 뒤 종료 | 네트워크 출력이 있는 경로에서 `finalized`와 `reclaimed`를 분리했고, 정상 이벤트 없이 종료한 스레드도 host가 회수한다. 한쪽 출력 종류가 없는 구성 적용은 RV-266. |
| RV-264 · 공개 ACK API | `flush()`·`handoff()` 관측 시 `AckGone` 전이와 End 이후 ACK 수 고정이 명시됐다. 메서드 간 End 대기 기한은 RV-267. |

RV-256의 epoch 게시 판별과 명령 반환 상한, RV-258의 최초 수신 유예와 후속 대기, RV-259의 End와 비-Gone ACK 실패 선착 경로를 확인했다. 원본 guard 미전달, 복사 예산, 완료·관리 wire 분리, route revision과 빈 route, 설정 교체 시 숫자 index 거절, DB 단계 6과 H6 조기 판정은 유지된다.

## 3. 리뷰 포인트

| ID | 중요도 | 위치 | 계약 공백 | 처리 제안 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-265 | **P1** | §5.9 | 파일 작업자가 발행 없이 끝날 때 셀의 종결이 정의되지 않고, 셀 기록만으로 `Closed`가 가능하다 | 파일 발행 결과와 IO 작업자 회수를 모두 최종 종료 조건에 연결한다 | 수용 | Resolved |
| RV-266 | **P1** | §5.8·§5.9 | 파일 또는 네트워크 출력이 없는 유효 구성에서 양쪽의 완료 조건을 충족하는 방법이 빠졌다 | 실제로 구성된 출력과 legacy tap을 기준으로 종료 분기를 명시한다 | 수용 | Resolved |
| RV-267 | P2 | §5.6 | `flush()`나 `handoff()`가 `Gone`을 먼저 받으면 End 대기 기한의 시작과 재설정 규칙이 없다 | `AckGone`의 기한 기준과 후속 호출의 결과를 명시한다 | 수용 | Resolved |

### RV-265 — 파일 작업자 실패와 회수를 최종 종료에 연결해야 한다

- **[리뷰어]** 파일 IO 스레드가 `finish` 전에 panic하면 현재 `io_main`은 `worker_failed`·`io_failed`를 세우고 `drain_failed`로 빠진다. 이 경로에는 정상 `finish` 호출이 없지만, 현행 host는 IO 스레드를 회수해 실패를 담은 `Closed`를 보낼 수 있다. r8은 IO 스레드만 쓰는 발행 셀을 `finish` 결과로 종결한다고 규정한다. 따라서 IO 실패가 알려지고 스레드가 끝나도 셀이 `Pending`이면 r8의 `Closed` 조건을 영구히 충족하지 못한다. 반대로 셀이 종결된 직후 IO 스레드가 정지하면, 회수 조건이 없는 r8 표는 기존의 「모든 정리 완료」보다 일찍 `Closed`를 허용한다. [발행 셀과 Closed 조건](../../04_implementation/milestones/MS-013.md#L405), [IO panic 경로](../../../crates/dot-logger/src/logger/host.rs#L818), [발행 후 IO 정리](../../../crates/dot-logger/src/logger/host.rs#L948), [기존 최종 보고](../../../crates/dot-logger/src/logger/host.rs#L662)
- **처리 제안** — 발행 없이 IO 작업자가 끝나는 경우에 결과를 `Failed`로 확정할 주체와 증거를 정한다. 기한 보고에서 해당 실패가 이미 확정됐으면 `files_settled()`와 표의 판정을 일치시킨다. 발행·실패 확인과 IO 회수 완료를 구분하고, `finish` 전 panic 및 셀 게시 직후 회수 지연에서 최종 반환을 시험한다.
- **[처리담당]** 수용. 발행 셀을 **IO 스레드마다** 두고(파일 출력은 묶음별로 여러 IO 스레드), 네 경로가 모두 `Pending`에서 나가는 CAS로 종결한다: ① 정상 `finish` 성공 `Published { at }`·실패 `Failed` ② `io_main` panic 경로에서 `drain_failed` 전에 `Failed` ③ IO 스레드 진입 closure가 끝날 때 아직 `Pending`이면 `Failed`(배치 실패로 `io_main` 미진입 포함) ④ 보조로 host가 IO 스레드 종료를 확인했는데 `Pending`이면 `Failed`. 그래서 `finish` 없이 끝난 IO 스레드도 셀이 영구 `Pending`으로 남지 않는다. 셀 합치기: `Failed` 하나 또는 `io_failed`면 실패 확정(다른 셀이 `Pending`이어도 `files_settled()` 참 — 기한 보고 표 행 추가), 모두 `Published`면 최대 `at`으로 판정, 그 밖은 `None`. 발행 결과 `files_published`와 IO 회수 `files_reclaimed`(host가 루프 안에서 IO 스레드 종료 확인·join)를 나눠 `Closed`는 둘 다 요구한다 — 셀 게시 직후 IO 정지는 판정은 확정, `Closed`는 지연(「모든 정리 완료」 유지). 시험: `finish` 전 panic, `io_main` 미진입, 셀 게시 직후 정지, 여러 IO 중 하나만 늦음. MS-013 r9 §5.9·§7 단계 0·§8.1.

### RV-266 — 출력 종류가 빠진 구성의 종료 상태를 정의해야 한다

- **[리뷰어]** FR-019a와 §5.8은 파일만, 파일 + legacy tap, 네트워크만을 지원한다. §5.9의 `Closed` 조건은 발행 셀의 종결과 `network.finalized && network.reclaimed`를 모두 요구한다. 네트워크만 구성에는 `finish`를 수행할 파일이 없고, 파일만 구성에는 회계 완료 이벤트를 낼 네트워크 출력 스레드가 없다. legacy tap은 새 회계의 대상이 아니면서 기존 리스너 종료는 필요하다. 없는 출력의 완료값과 공개 보고 표현이 비어 있어 정상 종료와 파일 성공 여부가 구현 선택에 맡겨진다. 특히 네트워크 전용에서 `files_late`가 `None`이면 현재 성공식으로는 네트워크 수신에 손실이 0이어도 `is_lossless()`가 성공할 수 없다. [모드별 보장표](../../04_implementation/milestones/MS-013.md#L390), [Closed·파일 성공 조건](../../04_implementation/milestones/MS-013.md#L410), [무손실 판정](../../04_implementation/milestones/MS-013.md#L246)
- **처리 제안** — 실제로 구성된 자원만 기다리도록 파일 없음·네트워크 출력 없음의 완료 조건과 보고 값을 정의한다. 네트워크 전용의 정상 종료·무손실 판정과 legacy tap 리스너 회수 후 `Closed`를 모드별로 대조한다.
- **[처리담당]** 수용. `Closed` 조건을 「core Closed && 파일 쪽 완료 && 네트워크 쪽 완료 && legacy tap 쪽 완료」로 쓰고 **구성되지 않은 쪽은 공허 참**으로 했다. 모드별 표(§5.9): 파일만 — 파일 쪽만, `network = None`, `is_lossless()` = `files_complete()`. 파일 + legacy tap — 리스너 정지 요청 뒤 그 스레드 회수까지(지금 `Closed` 직전 `remote.stop()`과 같은 정리, 0.2는 루프 안에서 확인), tap 사본은 회계 대상 아님. 파일 + 네트워크 — 양쪽. 네트워크만 — IO 스레드·셀이 없어 파일 쪽 공허 참, `files_closed_at = None`·`files_late = Some(false)`·`files_complete()` = `core.is_success() && !worker_failed`·`files_settled()` 참. `is_lossless()`를 「구성된 모든 출력 기준」(`files_complete()` && (네트워크 출력 없음 || `finalized` && 사고성 손실 0 && `unacked` 0))으로 다시 정의해 네트워크 전용도 무손실 판정이 가능하다. `FacadeReport.network`는 `Option`. 시험: 모드 4종의 정상 `Closed`·보고 값, tap 리스너 정지 지연 중 `Incomplete`. MS-013 r9 §5.4·§5.8·§5.9·§6·§7 단계 0.

### RV-267 — AckGone의 timeout 경계가 공개 메서드 사이에서 비어 있다

- **[리뷰어]** `flush()`가 먼저 `Gone`을 관측한 후 즉시 `AcknowledgementUnknown`을 반환하면, End를 읽는 것은 나중의 `next()`다. 표의 「기존 client timeout 안에」가 `Gone` 관측부터인지, 첫 `next()` 호출부터인지, 매 `next()`마다 시작하는지 정해져 있지 않다. 현재 타이머는 `next()` 호출별이다. `AckGone`에서 첫 `next()`가 이미 도착한 Record를 반환하고, 그 뒤 End가 최초 기한 밖이지만 두 번째 `next()`의 기한 안에 도착하면 선택한 기준에 따라 End 또는 sticky `AcknowledgementUnknown`으로 갈린다. [End 대기와 공개 상태표](../../04_implementation/milestones/MS-013.md#L358), [현행 next 대기](../../../crates/dot-logger-client/src/transport.rs#L710)
- **처리 제안** — `Gone`의 최초 관측 메서드, 후속 `next()`의 시작, 중간 이벤트가 타이머에 미치는 영향을 명시한다. `flush()`에서 `Gone`을 본 뒤 두 번의 `next()`에 Record와 End가 나뉘어 도착하는 시험에서 기대 결과를 고정한다.
- **[처리담당]** 수용. **`AckGone` 기한 = `Gone`을 처음 관측한 시각 t_g + client의 기존 `timeout`**으로 한 번 정하고, 관측 메서드(`next`·`handoff`·`flush`·reap)와 무관하며 재설정하지 않는다 — 그 사이 `next()`가 Record·heartbeat·표시를 돌려줘도 늘지 않는다. `AckGone`에서 `next()`는 자기 호출의 `timeout`과 이 기한 중 이른 쪽까지 기다리고(t_g 뒤 시작한 호출은 늘 기한이 먼저), 기한 전 End → `Ended`, EOF·기한 도달 → `Failed`(sticky `AcknowledgementUnknown`). 기한이 지난 뒤 처음 불린 메서드는 스트림을 읽지 않고 `Failed`로 전이한다 — 버퍼에 End가 와 있어도 「읽은 시각」이 기한 뒤라 End로 보지 않으므로 결과가 결정적이다(rustdoc에 「End는 기한 전에 `next()`로 읽어야 함」). 시험: `flush()`가 `Gone`을 본 뒤 두 `next()`에 Record·End가 나뉘어 도착(End가 기한 전/후), 첫 Record가 기한을 늘리지 않음, 기한 뒤 첫 호출. MS-013 r9 §5.6·§8.1.

## 4. 다음 회차

개발 세션에서 RV-265~267의 처리 결과를 기록한다. 개정 뒤 독립 재리뷰에서 이전 지적 해소와 새 지적 0건을 확인한 뒤 Human 승인으로 넘긴다.

- **[처리담당] 2026-11-28:** 3건 모두 수용, MS-013 r9 반영(§18 반영표). 재리뷰는 [REV-082 요청서](REV-082_REQUEST.md).

- **[처리담당] r9 보완 대조:** 한 IO 스레드가 여러 파일 출력을 맡으면 모두 발행에 성공한 뒤 마지막 발행 시각으로 셀을 종결한다. 이미 실패한 셀이 있으면 다른 셀이 대기 중이어도 파일 실패가 확정된다. §6의 `Incomplete` 설명을 §5.9의 파일 IO·네트워크 출력·legacy tap 회수 조건과 일치시켰다. `AckGone` 기한 뒤에는 상태를 변경하는 세 메서드에서만 전이를 집행하고 `acknowledged(&self)`는 조회만 한다. 이 내용은 설계 문서 대조 결과이며 실행 검증은 §7·§8.1에 있다.

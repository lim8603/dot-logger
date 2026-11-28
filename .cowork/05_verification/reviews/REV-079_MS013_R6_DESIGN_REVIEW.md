# REV-079 — MS-013 r6 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-28 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r6](../../04_implementation/milestones/MS-013.md), [REV-079 요청서](REV-079_REQUEST.md), [REV-078](REV-078_MS013_R5_DESIGN_REVIEW.md) |
| 기준 | r6 `7fe024c1c680a4a294bbb06a21a8bf720b0eee07`, 검토 시 HEAD `d18fd23` |
| 판정 / 상태 | **Conditional / Closed**(2026-11-28 처리담당: 5건 모두 수용, MS-013 r7 반영 — 2026-11-28 [REV-083](REV-083_MS013_R10_DESIGN_REVIEW.md) Pass로 반복 종료) |

## 1. 요약정리

**P1 3건, P2 2건.** r6은 이전 5건의 주요 수정안을 반영했다. 호출자가 보는 결과와 수락 기준을 대조한 결과, 스위치 host 실패 반환, 진행 중 L3 집계, 종료 기한 보고, ACK 실패와 End의 우선순위, observer 구성 내 빈 route에 계약 공백이 남았다. r5→r6 diff 및 현재 Rust 소스를 정적으로 대조했다. 제품 시험과 성능 측정은 요청 범위 밖이다.

## 2. 이전 지적 대조

| 이전 지적 | r6 대조 결과 |
|---|---|
| RV-251 · 명령 결과 | 같은 상태·거절·취소·커밋의 단일 승자와 미종결 5개·종결 이력 8개가 명시됐다. 후속 상태를 동기 호출에서 어떻게 돌려주는지는 RV-256. |
| RV-252 · 구독별 정산 | 필터와 offer 결과가 entry에 남고 생성을 닫은 뒤 봉인·정산한다. 최종 정산의 근거는 생겼고, 항목 처리 중 Health에서 성립해야 하는 식은 RV-257. |
| RV-253 · 파일 성공 | 마지막 발행 시각으로 `files_late`를 판정하고 최종 보고에 반영한다. 기한 보고를 받을 수 있는 시점 및 반환표의 누락 상태는 RV-258. |
| RV-254 · End 수치 | 서버의 `cut`·`unacked`와 client의 ACK 확인 수를 분리했다. ACK 응답 유실·부분 반영 중 End를 호출자가 실제로 받을 조건은 RV-259. |
| RV-255 · 빈 route | 기존 빈 구성·facade 빈 선택과 observer-only를 분리했다. 출력이 섞인 구성에서 채널 자체가 빈 경우는 RV-260. |

REV-074~077에서 정한 원본 guard 미전달, 같은 분배에서 복사 예산 초과분 정산, 완료 레지스트리·관리 wire와 observer 분리, legacy tap, 출력 교체 시 숫자 index 거절, DB 단계 6, H6 조기 판정은 유지된다. 원자 순서와 링 구현, 성능은 골든·spike 및 구현 검증 범위다.

## 3. 리뷰 포인트

| ID | 중요도 | 위치 | 계약 공백 | 처리 제안 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-256 | **P1** | §5.6, §6 | 최초 결과 대기 중 host가 실패하면 반환형과 조회 ID 전달이 미정 | 커밋 전후 host 실패의 반환·조회 규칙을 정한다 | 수용 | Resolved |
| RV-257 | **P1** | §1.3 FR-022, §5.4 | `Unsettled` 사본이 Health의 L3 보존식에서 빠진다 | 진행 중 사본의 건수·바이트를 식에 포함한다 | 수용 | Resolved |
| RV-258 | **P1** | §5.9, §6 | 호출자 수신 기한과 host 기한 보고 시점이 겹치고 늦은 파일 발행의 반환 행이 빠져 있다 | 공통 반환표에서 수신 상한·스냅샷·후속 wait를 확정한다 | 수용 | Resolved |
| RV-259 | P2 | §5.6, §6 | ACK 응답 유실은 스트림을 닫지만 같은 실행에서 End 수치 시험을 요구한다 | End와 ACK 오류의 우선순위 및 미확인 수의 확정 경계를 명시한다 | 수용 | Resolved |
| RV-260 | P2 | H8, §5.1, §6·8 | observer가 다른 채널에만 있는 구성의 빈 route를 표가 다루지 않는다 | 기록의 route revision에 있는 `observe`를 기준으로 판정한다 | 수용 | Resolved |

### RV-256 — 커밋 직후 host가 종료되면 첫 호출의 반환이 비어 있다

- **[리뷰어]** 변경 요청이 `Committed`에 도달한 뒤 host가 게시 전에 종료되고, 최초 호출자가 100 ms 유예 안에 그 종료를 관측하는 경우다. 설계상 읽는 쪽은 `Committed → HostFailed`로 바꿀 수 있고 호출자는 종결 결과를 반환해야 한다. 결과표에는 `switch_status = HostFailed`만 있다. 이때 호출자는 아직 `CommandId`를 받은 적이 없고 `SwitchError`에 대응하는 반환도 명시돼 있지 않다. host가 `Pending` 상태에서 종료되어 아직 기한이 남은 경우도 실패와 기한 취소 중 무엇을 돌려주는지 정해져 있지 않다. 이력에서 밀려난 결과를 현재 epoch로 확인하라는 설명도, 뒤 명령이 epoch를 전진시켰다면 개별 명령의 적용 여부를 판별할 수 없다. [명령 결과표](../../04_implementation/milestones/MS-013.md#L308), [공개 API](../../04_implementation/milestones/MS-013.md#L425)
- **처리 제안** — host 종료를 처음 관측한 호출자와 추후 조회자의 반환을 커밋 전후로 명시한다. 결과와 ID를 함께 반환하는지, 이력에서 밀려난 뒤 어떤 정보까지 확인 가능한지 정하고 각각 시험한다.
- **[처리담당]** 수용. host 종료를 관측한 쪽(대기 중 호출자 또는 `switch_status` 조회자)이 명령 셀을 CAS로 종결한다. 셀이 `Pending`(커밋 전)이면 `Rejected(HostFailed)` → `Err(SwitchError::WorkerFailed)` = 미적용 보장이고, 기한이 남아 있어도 기다리지 않는다(ID 없음). 셀이 `Committed { epoch: e }`이면 logger 현재 epoch를 읽어 ≥ e면 `Applied`(셀의 receipt), < e면 `HostFailed` → `Err(SwitchError::HostFailed { id, epoch })`. host는 커밋 CAS와 함께 그 명령의 epoch `e`를 셀에 기록하고 `Committed`·`HostFailed` 오류에 `(id, epoch)`를 함께 돌려준다. ⑥은 observe 변경 → 표시·링 게시 → receipt 기록 → **마지막에 epoch 게시** 순서이고, host는 명령을 하나씩 처리하므로 `output_state().epoch ≥ e` ⇔ 그 명령 게시 완료다. 그래서 이력에서 밀려난 뒤, 뒤 명령이 epoch를 전진시킨 경우에도 판별할 수 있다. host 종료 guard가 대기자를 깨우고(보조 10 ms 확인), 반환 상한 기한 + 100 ms는 유지된다. 시험: 커밋 전 host 종료, `Committed` 유예 중 종료(epoch 게시 전·후), `Committed` 반환 뒤 종료, 이력 교체 뒤 epoch 판별. MS-013 r7 §5.6·§5.10·§6·§7 단계 0·§8.1.

### RV-257 — 진행 중 L3 사본이 정상 Health 스냅샷에서 누락된다

- **[리뷰어]** 링 항목이 필터를 통과한 활성 구독 두 곳을 대상으로 하고 첫 offer 직전에 멈춘 경우, 항목 시작 구간에서 `targeted = 2`지만 새 사본의 `filtered = offered = not_offered = 0`이다. 이 상태는 두 hub 잠금 구간 사이라 정상 스냅샷으로 게시할 수 있다. FR-022와 §5.4는 Health에서 단계 내 등식이 성립한다고 요구한다. 대상 구독 하나가 중도 종료되어 항목 끝에서 `not_offered`로 분류되기 전에도 같은 공백이 있다. 최종 정산만으로 중간 스냅샷의 계약은 충족되지 않는다. [FR-022](../../04_implementation/milestones/MS-013.md#L72), [L3 보존식과 상태 전이](../../04_implementation/milestones/MS-013.md#L230)
- **처리 제안** — Health 식에 `offer_pending` 같은 순간값을 건수·payload 바이트로 포함하고, `Unsettled`의 생성·해소와 같은 경계에서 갱신한다. 항목 시작 직후와 부분 fanout 중 스냅샷을 시험하고 최종값은 0으로 확인한다.
- **[처리담당]** 수용. L3 누적식에 순간값 `offer_pending`(현재 항목의 `Unsettled` entry 건수·payload 바이트)을 넣었다: `targeted = filtered + offered + not_offered + offer_pending`. 항목 시작 구간에서 `Unsettled` entry 수만큼 더하고, offer 구간(`Settled`)·항목 끝 구간(도중 종료 구독 → `not_offered`)·봉인 위치 정산(`Unsettled` → `not_offered`)에서 같은 잠금 구간으로 뺀다. 최종 보고에서 `offer_pending = in_queue = ack_pending = 0`. 시험: 항목 시작 직후(대상 2 → `offer_pending` 2), 부분 fanout 중(1), 대상 구독 도중 종료 뒤 항목 끝 전의 Health 스냅샷 등식(건수·바이트), 최종 0. MS-013 r7 §1.3 FR-022·§5.4·§8.1.

### RV-258 — 기한 보고를 언제 받을 수 있는지 반환표에서 정해야 한다

- **[리뷰어]** §5.9는 호출자가 절대 기한까지만 기다리면서 host가 그 기한이 지난 뒤 보낸 `Progress::Incomplete`로 반환하는 행을 함께 둔다. 기한까지 다른 진행 보고가 없고 host가 직후 기한 보고를 게시하면, 기한까지만 기다린 최초 호출에서는 그 보고를 받을 수 없다. 현재 코드는 `REPORT_GRACE`가 5초이고, 최초 호출이 `Unresponsive`로 반환한 뒤 첫 `pending.wait(timeout)`에서도 늦게 도착한 `Incomplete`를 받을 수 있다. r6 표는 이 경로를 최종 `Closed`로만 설명한다. 또한 수신 구간에 늦은 `FilesClosed`만 있고 네트워크는 아직 정리 중인 상태도 표에 빠져 있다. [r6 종료 반환표](../../04_implementation/milestones/MS-013.md#L374), [기존 대기와 유예](../../../crates/dot-logger/src/logger/mod.rs#L379), [host 기한 보고](../../../crates/dot-logger/src/logger/host.rs#L624)
- **처리 제안** — 기한 시 이미 게시된 상태로 반환할지, 명시한 수신 유예까지 기다릴지 정한다. 기한 뒤 첫 `Incomplete`, `FilesClosed.at > 기한`이면서 네트워크 미완료, 후속 `wait(timeout)`의 다음 보고·시간 초과·worker 실패를 반환표에 넣는다. 파일 성공 기준은 발행 시각으로 유지한다.
- **[처리담당]** 수용. 호출자 수신 상한을 **지금처럼 기한 + `REPORT_GRACE`(5 s)**로 두었다(반환 상한도 같음 — r6의 「기한까지만」 철회). 호출자에게 가는 보고는 지금처럼 기한 보고(`Incomplete`, 최대 1회, 작성 시각 ≥ 기한)와 최종 `Closed` 둘뿐이고, `FilesClosed`·`NetworkFinalized`는 host 내부 이벤트로 내렸다. host는 core Closed 뒤 `finish`·네트워크 정리를 기다리는 동안에도 루프에서 기한을 보며(지금은 loop 밖에서 IO join), 기한 턴의 봉인은 `try_lock`이라 못 잡으면 `finalized = false`로 보고하고 다음 턴에 정산한다. `shutdown()` 판정표에 기한 보고 지연(도착 시각은 판정에 쓰지 않음)과 「기한 < `at` ≤ 작성 시각 + 네트워크 미완료」 행을, `wait(timeout)` 판정표에 늦은 기한 보고·`Closed`·시간 초과(`Unresponsive(self)`)·`WorkerFailed`를 넣었다. 파일 성공은 발행 시각 기준 유지. MS-013 r7 §5.9·§6·§7 단계 0·§8.1.

### RV-259 — End를 읽는 경우와 ACK 오류로 종료하는 경우가 겹친다

- **[리뷰어]** 서버에 반영된 ACK 응답이 유실되어 client에서 RPC timeout이 먼저 관측되면, §5.6 우선순위상 Gone이 아니므로 현재처럼 `AcknowledgementUnknown`으로 종료한다. 현재 `unknown()`은 스트림을 닫으므로 이어서 도착하는 End는 읽을 수 없다. 같은 절의 시험 목록은 이 경우에도 End의 `server_unacked`와 `ack_unknown`을 대조하도록 요구한다. 묶음 부분 반영 뒤 Gone이 아닌 오류도 같은 문제가 있다. End가 여러 ACK RPC의 응답보다 먼저 도착하는 경우에는 어떤 응답까지 기다려 `ack_unknown`을 확정하는지 정해져 있지 않다. [End 수치·우선순위·시험](../../04_implementation/milestones/MS-013.md#L333), [client 실패 처리](../../../crates/dot-logger-client/src/transport.rs#L579), [묶음 ACK](../../../crates/dot-logger/src/adapters/remote/hub.rs#L513)
- **처리 제안** — End 선착, Gone 선착, 그 외 ACK 오류 선착의 호출자 결과를 각각 고정한다. End에서 응답 대기 중인 묶음을 어느 시점에 미확인으로 세는지 정하고, End를 보고하는 실행과 ACK 오류로 끝나는 실행의 시험 기대값을 맞춘다.
- **[처리담당]** 수용. 결과를 client가 `next()`에서 **먼저 관측한 것**으로 고정했다(지금 `next_inner`처럼 reap이 먼저라 한 호출에서 둘 다 준비되면 ACK 결과가 먼저). End 선착 → `RemoteEvent::End`. ACK `Gone` 선착 → 계속 읽어 End가 오면 End, EOF·timeout이면 `AcknowledgementUnknown`. Gone 아닌 ACK 실패 선착(transport·RPC 시간 초과 — 응답 유실이 시간 초과로 드러난 경우 포함) → 지금처럼 `AcknowledgementUnknown`·End 없음, 서버 쪽 결과는 서버 보고에만 남는다. `ack_unknown`은 **End를 읽은 순간** 성공 응답을 받지 못한 handoff 사본(미전송 대기·응답 대기 묶음·실패 묶음)으로 확정하고, 응답 대기 RPC는 기다리지 않으며 그 뒤 응답은 반영하지 않는다. End 수치 표를 「End를 읽은 시점의 상황」으로 다시 쓰고, wire 시험의 「ACK 응답 유실 뒤 End」를 End 선착(두 관점 차이)과 시간 초과 선착(End 없음) 두 건으로 나눴다. 한 `next()`에서 ACK 실패와 End가 함께 준비된 경우와 End 뒤 늦은 응답도 시험한다. MS-013 r7 §5.6·§6.

### RV-260 — observer가 다른 채널에만 지정된 경우에도 빈 route가 생긴다

- **[리뷰어]** 고급 Router에서 파일 target과 네트워크 observer를 선언하고, A 채널만 observer를 지정하고 B 채널은 `targets=[]`, `observe=None`으로 둔다. B는 현재 빈 route 동작을 유지한다는 §6에 해당하지만, 기준표의 ②는 「observer 없는 구성」으로 한정됐고 ④는 B의 route에 observer가 있어야 한다. 기존 Router는 구성의 target이 존재하면 채널별 빈 목록을 허용하며, 그 기록을 `Unrouted`와 `NoRoute`로 처리한다. [route 기준표](../../04_implementation/milestones/MS-013.md#L189), [호환 표](../../04_implementation/milestones/MS-013.md#L431), [Router 빈 route 시험](../../../crates/dot-logger/tests/tc_003_routing.rs#L495), [기존 NoRoute 집계](../../../crates/dot-logger/src/completion/mod.rs#L947)
- **처리 제안** — ②를 시작 검증을 통과한 구성에서 해당 route revision의 `targets=[]`와 `observe=None`으로 정의한다. 다른 채널에만 observer가 있는 구성에서도 기존 빈 route의 이벤트·손실·raw 회수를 검증한다.
- **[처리담당]** 수용. §5.1 경우표를 ①만 **구성** 단위, ②·④·⑤는 **기록의 route revision**(`targets`·`observe`) 단위로 정의했다. ② = 시작 검증을 통과한 구성에서 기록의 route가 `targets=[]` && `observe=None` — 구성의 다른 채널에 observer가 있어도 같다(`Unrouted` + `NoRoute`·raw 회수 그대로). 「구성 전체 tracked target 0 허용」만 구성 단위 조건으로 남겼다. H8·§6 유지 행 문구를 맞추고, spike 합격 조건과 §8.1에 「파일 target + 네트워크 observer 구성에서 A 채널 ④·B 채널 ②일 때 B가 observer 없는 구성과 같음」 시험을 넣었다. MS-013 r7 §5.1·H8·§6·§7 단계 0·§8.1.

## 4. 다음 회차

개발 세션에서 RV-256~260을 반입하고 처리 결과를 기록한다. 개정 뒤 독립 재리뷰에서 이전 지적 해소와 새 지적 0건을 확인한 뒤 Human 승인으로 넘긴다.

- **[처리담당] 2026-11-28:** 5건 모두 수용, MS-013 r7 반영(§16 반영표). 재리뷰는 [REV-080 요청서](REV-080_REQUEST.md).

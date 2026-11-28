# REV-079 — MS-013 r6 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-28 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r6](../../04_implementation/milestones/MS-013.md), [REV-079 요청서](REV-079_REQUEST.md), [REV-078](REV-078_MS013_R5_DESIGN_REVIEW.md) |
| 기준 | r6 `7fe024c1c680a4a294bbb06a21a8bf720b0eee07`, 검토 시 HEAD `d18fd23` |
| 판정 / 상태 | **Conditional / Open** |

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
| RV-256 | **P1** | §5.6, §6 | 최초 결과 대기 중 host가 실패하면 반환형과 조회 ID 전달이 미정 | 커밋 전후 host 실패의 반환·조회 규칙을 정한다 | — | Open |
| RV-257 | **P1** | §1.3 FR-022, §5.4 | `Unsettled` 사본이 Health의 L3 보존식에서 빠진다 | 진행 중 사본의 건수·바이트를 식에 포함한다 | — | Open |
| RV-258 | **P1** | §5.9, §6 | 호출자 수신 기한과 host 기한 보고 시점이 겹치고 늦은 파일 발행의 반환 행이 빠져 있다 | 공통 반환표에서 수신 상한·스냅샷·후속 wait를 확정한다 | — | Open |
| RV-259 | P2 | §5.6, §6 | ACK 응답 유실은 스트림을 닫지만 같은 실행에서 End 수치 시험을 요구한다 | End와 ACK 오류의 우선순위 및 미확인 수의 확정 경계를 명시한다 | — | Open |
| RV-260 | P2 | H8, §5.1, §6·8 | observer가 다른 채널에만 있는 구성의 빈 route를 표가 다루지 않는다 | 기록의 route revision에 있는 `observe`를 기준으로 판정한다 | — | Open |

### RV-256 — 커밋 직후 host가 종료되면 첫 호출의 반환이 비어 있다

- **[리뷰어]** 변경 요청이 `Committed`에 도달한 뒤 host가 게시 전에 종료되고, 최초 호출자가 100 ms 유예 안에 그 종료를 관측하는 경우다. 설계상 읽는 쪽은 `Committed → HostFailed`로 바꿀 수 있고 호출자는 종결 결과를 반환해야 한다. 결과표에는 `switch_status = HostFailed`만 있다. 이때 호출자는 아직 `CommandId`를 받은 적이 없고 `SwitchError`에 대응하는 반환도 명시돼 있지 않다. host가 `Pending` 상태에서 종료되어 아직 기한이 남은 경우도 실패와 기한 취소 중 무엇을 돌려주는지 정해져 있지 않다. 이력에서 밀려난 결과를 현재 epoch로 확인하라는 설명도, 뒤 명령이 epoch를 전진시켰다면 개별 명령의 적용 여부를 판별할 수 없다. [명령 결과표](../../04_implementation/milestones/MS-013.md#L308), [공개 API](../../04_implementation/milestones/MS-013.md#L425)
- **처리 제안** — host 종료를 처음 관측한 호출자와 추후 조회자의 반환을 커밋 전후로 명시한다. 결과와 ID를 함께 반환하는지, 이력에서 밀려난 뒤 어떤 정보까지 확인 가능한지 정하고 각각 시험한다.
- **[처리담당]** 대기.

### RV-257 — 진행 중 L3 사본이 정상 Health 스냅샷에서 누락된다

- **[리뷰어]** 링 항목이 필터를 통과한 활성 구독 두 곳을 대상으로 하고 첫 offer 직전에 멈춘 경우, 항목 시작 구간에서 `targeted = 2`지만 새 사본의 `filtered = offered = not_offered = 0`이다. 이 상태는 두 hub 잠금 구간 사이라 정상 스냅샷으로 게시할 수 있다. FR-022와 §5.4는 Health에서 단계 내 등식이 성립한다고 요구한다. 대상 구독 하나가 중도 종료되어 항목 끝에서 `not_offered`로 분류되기 전에도 같은 공백이 있다. 최종 정산만으로 중간 스냅샷의 계약은 충족되지 않는다. [FR-022](../../04_implementation/milestones/MS-013.md#L72), [L3 보존식과 상태 전이](../../04_implementation/milestones/MS-013.md#L230)
- **처리 제안** — Health 식에 `offer_pending` 같은 순간값을 건수·payload 바이트로 포함하고, `Unsettled`의 생성·해소와 같은 경계에서 갱신한다. 항목 시작 직후와 부분 fanout 중 스냅샷을 시험하고 최종값은 0으로 확인한다.
- **[처리담당]** 대기.

### RV-258 — 기한 보고를 언제 받을 수 있는지 반환표에서 정해야 한다

- **[리뷰어]** §5.9는 호출자가 절대 기한까지만 기다리면서 host가 그 기한이 지난 뒤 보낸 `Progress::Incomplete`로 반환하는 행을 함께 둔다. 기한까지 다른 진행 보고가 없고 host가 직후 기한 보고를 게시하면, 기한까지만 기다린 최초 호출에서는 그 보고를 받을 수 없다. 현재 코드는 `REPORT_GRACE`가 5초이고, 최초 호출이 `Unresponsive`로 반환한 뒤 첫 `pending.wait(timeout)`에서도 늦게 도착한 `Incomplete`를 받을 수 있다. r6 표는 이 경로를 최종 `Closed`로만 설명한다. 또한 수신 구간에 늦은 `FilesClosed`만 있고 네트워크는 아직 정리 중인 상태도 표에 빠져 있다. [r6 종료 반환표](../../04_implementation/milestones/MS-013.md#L374), [기존 대기와 유예](../../../crates/dot-logger/src/logger/mod.rs#L379), [host 기한 보고](../../../crates/dot-logger/src/logger/host.rs#L624)
- **처리 제안** — 기한 시 이미 게시된 상태로 반환할지, 명시한 수신 유예까지 기다릴지 정한다. 기한 뒤 첫 `Incomplete`, `FilesClosed.at > 기한`이면서 네트워크 미완료, 후속 `wait(timeout)`의 다음 보고·시간 초과·worker 실패를 반환표에 넣는다. 파일 성공 기준은 발행 시각으로 유지한다.
- **[처리담당]** 대기.

### RV-259 — End를 읽는 경우와 ACK 오류로 종료하는 경우가 겹친다

- **[리뷰어]** 서버에 반영된 ACK 응답이 유실되어 client에서 RPC timeout이 먼저 관측되면, §5.6 우선순위상 Gone이 아니므로 현재처럼 `AcknowledgementUnknown`으로 종료한다. 현재 `unknown()`은 스트림을 닫으므로 이어서 도착하는 End는 읽을 수 없다. 같은 절의 시험 목록은 이 경우에도 End의 `server_unacked`와 `ack_unknown`을 대조하도록 요구한다. 묶음 부분 반영 뒤 Gone이 아닌 오류도 같은 문제가 있다. End가 여러 ACK RPC의 응답보다 먼저 도착하는 경우에는 어떤 응답까지 기다려 `ack_unknown`을 확정하는지 정해져 있지 않다. [End 수치·우선순위·시험](../../04_implementation/milestones/MS-013.md#L333), [client 실패 처리](../../../crates/dot-logger-client/src/transport.rs#L579), [묶음 ACK](../../../crates/dot-logger/src/adapters/remote/hub.rs#L513)
- **처리 제안** — End 선착, Gone 선착, 그 외 ACK 오류 선착의 호출자 결과를 각각 고정한다. End에서 응답 대기 중인 묶음을 어느 시점에 미확인으로 세는지 정하고, End를 보고하는 실행과 ACK 오류로 끝나는 실행의 시험 기대값을 맞춘다.
- **[처리담당]** 대기.

### RV-260 — observer가 다른 채널에만 지정된 경우에도 빈 route가 생긴다

- **[리뷰어]** 고급 Router에서 파일 target과 네트워크 observer를 선언하고, A 채널만 observer를 지정하고 B 채널은 `targets=[]`, `observe=None`으로 둔다. B는 현재 빈 route 동작을 유지한다는 §6에 해당하지만, 기준표의 ②는 「observer 없는 구성」으로 한정됐고 ④는 B의 route에 observer가 있어야 한다. 기존 Router는 구성의 target이 존재하면 채널별 빈 목록을 허용하며, 그 기록을 `Unrouted`와 `NoRoute`로 처리한다. [route 기준표](../../04_implementation/milestones/MS-013.md#L189), [호환 표](../../04_implementation/milestones/MS-013.md#L431), [Router 빈 route 시험](../../../crates/dot-logger/tests/tc_003_routing.rs#L495), [기존 NoRoute 집계](../../../crates/dot-logger/src/completion/mod.rs#L947)
- **처리 제안** — ②를 시작 검증을 통과한 구성에서 해당 route revision의 `targets=[]`와 `observe=None`으로 정의한다. 다른 채널에만 observer가 있는 구성에서도 기존 빈 route의 이벤트·손실·raw 회수를 검증한다.
- **[처리담당]** 대기.

## 4. 다음 회차

개발 세션에서 RV-256~260을 반입하고 처리 결과를 기록한다. 개정 뒤 독립 재리뷰에서 이전 지적 해소와 새 지적 0건을 확인한 뒤 Human 승인으로 넘긴다.

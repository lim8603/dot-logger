# REV-078 — MS-013 r5 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-27 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r5](../../04_implementation/milestones/MS-013.md), [REV-078 요청서](REV-078_REQUEST.md), [REV-077](REV-077_MS013_R4_DESIGN_REVIEW.md) |
| 기준 | r5 `fe0f8d59edf4f18d8949cd985395c63cf22cfda4`, 검토 시 HEAD `72c2851` |
| 판정 / 상태 | **Conditional / Resolved**(2026-11-27 처리담당: 5건 모두 수용, MS-013 r6 반영 — REV-079 대조 대기) |

## 1. 요약정리

**P1 3건, P2 2건.** r5는 ACK·큐 상태의 보존식, 최종 파일 발행 결과의 수집, 표시의 실제 인코딩 상한을 구체화했다. 남은 지적은 스위치 결과의 유일성, 손실 정산에서 시도 여부를 입증할 기준, 파일 성공 기한, End의 미확인 수 의미, observer-only의 상충하는 수락 기준이다. 소스와 계약을 정적으로 대조했다. 제품 시험과 성능 측정은 요청 범위 밖이다.

## 2. 이전 지적 대조

| 이전 지적 | r5 대조 결과 |
|---|---|
| RV-245 · 스위치 기한 | 100 ms 유예 뒤 명령 ID로 반환하는 경로가 생겼다. 같은 상태 요청과 host 실패까지 포함한 명령별 결과 계약은 RV-251. |
| RV-246 · 회계 | L3의 `cut`과 누적 ACK 등식이 반영됐다. 모든 offer 결과의 정산 증거와 최종 봉인 경계는 RV-252. |
| RV-247 · 파일 보고 | 최종 발행 실패, 무보고 `Unresponsive`, 호출자의 관측 전용 경로가 반영됐다. 발행이 성공하더라도 기한을 지난 경우의 판정은 RV-253. |
| RV-248 · observer-only | H8과 §5.1은 observer-only를 `NoRoute`에서 제외하고 observer가 있을 때만 target 0을 허용한다. 나머지 절의 상충하는 기준은 RV-255. |
| RV-249 · 종료 알림 | 정상/강제/전달 불가 순서표가 추가됐다. 송신·수신 미확인 수의 단위와 ACK RPC 오류 우선순위는 RV-254. |
| RV-250 · 표시 크기 | 저장용 12P와 wire 크기를 분리하고 마지막 frame 전체를 실제 인코딩해 확인한다. 경계값 시험으로 남겨도 충분하다. |

REV-074~076에서 확정한 원본 guard 미전달, 같은 분배에서 복사 예산 초과분 정산, 완료 레지스트리와 관리 wire 분리, 파일 tap, 설정 교체 때 숫자 index 거절, DB 단계 6 및 H6 조기 판정은 유지된다.

## 3. 리뷰 포인트

| ID | 중요도 | 위치 | 계약 공백 | 처리 제안 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-251 | **P1** | §5.6, §5.10 | 무효과 응답과 타임아웃이 서로 다른 명령 결과를 낼 수 있고 커밋 뒤 실패의 종결 상태도 없음 | 경쟁하는 모든 경로의 결과를 단일 상태표로 확정 | 수용 | Resolved |
| RV-252 | **P1** | §5.4, §5.9 | 큐 수락만 기록하는 순번으로 Gap 등의 시도를 복구할 수 없고 봉인 범위가 hub에만 걸림 | 시도 전체의 증거와 최종 경계를 계약화 | 수용 | Resolved |
| RV-253 | **P1** | §5.9 | core 기한 충족과 마지막 파일 발행 기한 충족이 구분되지 않음 | 발행 시각을 포함해 파일·전체 종료 판정표 확정 | 수용 | Resolved |
| RV-254 | P2 | §5.6, §6 | End의 서버 미확인 수와 client ACK 미확인 수가 다를 수 있음 | 수치의 주체·단위와 오류 우선순위 명시 | 수용 | Resolved |
| RV-255 | P2 | §4, §5.1, §6~8 | H8의 새 규칙과 기존 `Unrouted`·`NoRoute` 수락 기준이 공존 | 구성·route·observer-only를 나눠 기존 규칙을 명시 | 수용 | Resolved |

### RV-251 — 한 명령의 응답·타임아웃·상태 조회가 같은 결과를 가리켜야 함

- **[리뷰어]** 이미 ON인 상태에서 ON을 요청하면 host는 CAS 없이 `changed: false`를 보낸다. 그 직후 호출자가 응답을 소비하기 전에 기한이 되면 `Pending → Cancelled`가 여전히 성공할 수 있다. 따라서 하나의 명령에 성공 영수증과 취소가 동시에 존재할 수 있다. `Busy`, `Closing`도 동일한 규칙이 필요하다. 변경 명령은 CAS 직후 host가 죽으면 효과가 0회일 수 있는데 `Pending`이 「1회 적용 확정」이며 조회가 영구 Pending인 상태로 정의됐다. 최근 8개의 보관 정책이 미종결 명령과 종결 영수증을 어떻게 다루는지도 빠져 있다. [스위치 계약](../../04_implementation/milestones/MS-013.md#L286), [관리 예약](../../04_implementation/milestones/MS-013.md#L344)
- **처리 제안** — 무효과 성공·거절·취소도 하나의 명령별 결과 경합으로 확정한다. `Pending`은 기한에 게시를 아직 확인하지 못한 상태로 정의하고, host가 종료되면 가능한 최종 결과(효과 없음 또는 적용 여부 불명)를 명시한다. 기한 경로에서는 최근 완료 표와 별도로 미종결 명령을 유지하고, 보관 슬롯을 §5.10 산식에 넣는다. 같은 상태 응답 직후·커밋 CAS 직후 정지/실패·최근 이력 교체에서 결과가 한 가지인지 검증한다.
- **[처리담당]** 수용. 명령 상태 셀 하나에서 **모든 결과가 `Pending`에서 나가는 CAS 하나**로 정해지게 했다: host ① `Rejected(Closing)` ② 같은 상태도 CAS로 `Applied(changed: false)`(r5 「CAS 없이」 철회) ③ `Rejected(Busy)` ④ 기한 지남 `Cancelled` ⑤ `Committed` → ⑥ 게시 → ⑦ `Applied`. 호출자 기한 CAS `Pending → Cancelled`. 진 쪽은 효과 없음. `SwitchError::Pending`을 `Committed(CommandId)`(커밋·게시 미확인)로 개명하고, host가 `Committed`에서 죽으면 `HostFailed`(적용 불명, 이후 분배 없음). 미종결 명령은 자리 5개에서 종결 전 밀려나지 않고(모두 차면 즉시 `Busy`), 종결 이력 8개는 별도 링이며 둘 다 §5.10 산식에 넣었다. 시험: 같은 상태 응답 직후 기한, `Committed` 뒤 host 종료, 이력 교체. MS-013 r6 §5.6·§5.10·§6·§7·§8.1.

### RV-252 — 시도한 모든 사본과 확정된 원본 범위가 회계에 남아야 함

- **[리뷰어]** 링 항목 42를 구독 A에 offer했으나 큐가 가득 차 `Gap`으로 집계됐다고 하자. 그 뒤 worker가 멈추면 큐의 마지막 **수락**만 나타내는 `last_offered_seq`는 42 미만이다. r5의 감독자는 해당 사본을 다시 `not_offered`로 세어 `targeted = filtered + offered + not_offered`를 깨뜨린다. `too_large`·adapter 실패·필터 제외에도 같은 문제가 생긴다. 구독이 중도 종료돼 조회 가능한 상태에서 사라지는 경우에도 현재 위치의 증거가 유지되어야 한다. 또한 기한에 hub만 봉인하고 host가 복사 링과 L1을 계속 갱신하면 `finalized` 보고의 범위가 움직인다. [r5 회계와 봉인](../../04_implementation/milestones/MS-013.md#L211), [hub 포화와 수락 분기](../../../crates/dot-logger/src/adapters/remote/hub.rs#L414), [기한 경로](../../04_implementation/milestones/MS-013.md#L327)
- **처리 제안** — 현재 링 항목의 구독별 최종 분류를 모든 결과에서 확인할 수 있게 한다. 구독 수락과 이미 시도해 정산한 사본을 구분하고, 현재 항목이 종결될 때까지 필요한 구독별 증거를 보존한다. 링 head 예약과 생산을 닫는 경계, 아직 `Progress`가 없는 첫 잔여 항목의 분류, L1부터 hub까지 계수가 고정된 이후에만 `finalized=true`가 되는 조건을 명시한다. Gap·필터·크기 초과·offer 성공 직후와 구독 종료·봉인 후 생산 재개를 중단 시험에 포함한다.
- **[처리담당]** 수용. `last_offered_seq`를 버리고 **현재 링 항목의 구독별 정산 상태**(`Progress.entries` = (구독 ID, `Unsettled`/`Settled`) ≤ S)를 근거로 했다. 항목 시작 구간에서 대상 구독·`targeted`·L2·필터(`filtered` + `Settled`)를, offer 구간에서 결과 종류와 무관하게 계수와 함께 `Settled`를 기록한다. 구독이 도중에 끝나도 entry는 남는다. 네트워크 스레드의 모든 계수 전이는 hub 잠금 구간에서 먼저 봉인을 확인한다. 감독 정산 = ① host가 생산 닫기(L1·링 쓰기 위치 고정) ② 봉인 + 모든 구독 종결 ③ `Unsettled` → `not_offered`, 다음 항목부터 쓰기 위치까지 `abandoned`(Progress 없으면 읽기 위치부터) ④ 한 스냅샷 게시. `finalized`는 L1이 고정된 종료 경로에서만 참. 시험: 항목 시작 전·필터·Gap·`too_large`·offer 직후·항목 끝 직전 중단, 도중 구독 종료, 봉인 뒤 worker 재개·ACK. MS-013 r6 §5.4·§5.10·§8.1.

### RV-253 — 마지막 파일 발행이 늦은 경우를 성공으로 표시하지 않아야 함

- **[리뷰어]** 파일 core가 기한 직전에 Closed가 되고 마지막 `log.finish()`가 기한 뒤 성공하는 실행을 생각하면, 기존 `ShutdownReport::is_success()`는 core 시각과 `deadline_elapsed`를 사용한다. 여기에 `io_failed=false`만 결합하면 최종 발행이 늦어도 `FacadeReport::is_success()`가 참이다. r5는 `FilesClosed.at`를 전달하지만 §5.9는 성공을 어느 시각으로 판정할지 명시하지 않는다. 두 작업이 기한 내 완료한 경우는 규정했으나, 둘 다 정리됐고 파일 발행만 늦어 `PendingShutdown::wait()`가 최종 결과를 내는 경우는 빠져 있다. [r5 종료 보고](../../04_implementation/milestones/MS-013.md#L327), [core 성공 조건](../../../crates/dot-logger/src/control/types.rs#L127), [facade 성공 조건](../../../crates/dot-logger/src/logger/mod.rs#L334)
- **처리 제안** — `files_complete()`가 물리적 최종 발행 완료를 의미하는지, 기한 내 파일 성공까지 의미하는지 정한다. 기한 성공에 최종 파일 발행을 포함한다면 `FilesClosed.at`를 동일 절대 기한과 비교한다. 반환 뒤 정리가 끝나 `Closed`를 받는 경우에도 파일 성공과 기한 초과가 구분되는 판정표를 두고, 기한 직전 core Closed 뒤 늦게 성공한 finish를 시험한다.
- **[처리담당]** 수용. host가 `FilesClosed.at`(마지막 발행 완료 시각)을 같은 절대 기한과 비교해 `files_late`를 넣고, `FacadeReport::files_complete()`·`is_success()`에 `!files_late`를 더했다(파일 성공은 발행 시각 기준, 보고 도착 시각 아님). 기존 host 기한 보고(`Progress::Incomplete`, `host.rs`)를 포함해 반환 경로별 판정표를 두었다 — 기한 보고에 「core Closed, `finish` 진행 중」이면 `files_late = true`로 `files_complete()` false 확정, `pending.wait()`의 최종 `Closed`에서도 늦은 발행은 성공 아님. MS-013 r6 §5.9·§6·§7 단계 0·§8.1.

### RV-254 — End의 미확인 수는 서버와 클라이언트에서 다를 수 있음

- **[리뷰어]** 서버가 ACK를 반영했으나 응답이 유실되면 서버의 `unconfirmed`는 0이고 client는 그 ACK를 미확인으로 본다. 묶음 ACK는 일부를 반영한 뒤 뒤 항목에서 실패할 수도 있다. 반대로 서버가 내보낸 사본 중 client가 handoff하지 않은 것도 서버의 미확인 수에 포함될 수 있다. §5.6의 「맞춰」는 어느 쪽 수를 최종 수치로 보고할지 설명하지 않는다. 기존 ACK RPC의 실패는 client의 `Unknown` 경로로 접히므로 End 우선 정책을 구현할 때 원인 전달 지점도 남겨야 한다. [End 순서표](../../04_implementation/milestones/MS-013.md#L298), [서버 묶음 ACK](../../../crates/dot-logger/src/adapters/remote/hub.rs#L513), [클라이언트 ACK 확인 수](../../../crates/dot-logger-client/src/transport.rs#L630)
- **처리 제안** — End의 값은 서버가 종료 경계에서 관측한 사본 수로 정의하고, client의 별도 ACK 확인 상태를 그 값으로 덮지 않는다. ACK 응답 유실·부분 반영·handoff 전 종료의 보고 규칙과, 직후 frame이 heartbeat/EOF인 경우를 순서표에 넣는다. 해당 경우는 적어도 1건씩 wire 시험에 넣는다.
- **[처리담당]** 수용. `End{reason, cut, unacked}`를 **서버가 그 구독을 종결 처리한 직후의 hub 계수**로 정의하고, client는 그 값을 자기 상태로 덮지 않고 `RemoteEvent::End { reason, server_cut, server_unacked, ack_unknown }`로 따로 보고한다. ACK 응답 유실·묶음 부분 반영·handoff 전 종료·End 뒤 ACK의 두 관점 표를 넣었다. client는 ACK RPC 실패의 gRPC 원인을 `unknown()`으로 접기 전에 내부 보관해 `Gone`일 때만 End 우선(End·EOF·기존 timeout까지 읽고, 그 사이 heartbeat·payload는 평소대로)으로 처리한다. 경우별 wire 시험. MS-013 r6 §5.6·§6.

### RV-255 — observer-only와 기존 빈 route를 서로 다른 수락 조건으로 명시

- **[리뷰어]** H8·§5.1·§8.1은 observer-only에서 `NoRoute` 증가가 0이라고 정했다. §6과 단계 0은 여전히 기존 `NoRoute`로 센다고 적혀 있어 같은 spike에 반대 판정이 있다. §4의 target 0 → `Unrouted`도 기존 이벤트의 계약인 「출력 없음」과 충돌하며, 진단 이벤트를 유지할 경우의 의미가 정해지지 않았다. 또 기존 facade의 빈 출력 선택은 거절하지만 고급 Router는 구성된 target이 있고 특정 채널의 route가 비어 있으면 이를 허용하고 `Unrouted`를 발행한다. §5.1의 「observer 없는 빈 route 거절 유지」를 Router 공통 규칙으로 구현하면 기존 소비자가 깨진다. [r5 구조·observer 계약](../../04_implementation/milestones/MS-013.md#L178), [API 표](../../04_implementation/milestones/MS-013.md#L375), [단계 0](../../04_implementation/milestones/MS-013.md#L392), [기존 Router 빈 route 시험](../../../crates/dot-logger/tests/tc_003_routing.rs#L494), [기존 facade index 검사](../../../crates/dot-logger/src/logger/mod.rs#L1010)
- **처리 제안** — observer-only의 core event·loss·완료 기대값을 한 곳에서 확정하고 단계 0·API 표와 일치시킨다. 구성 전체에 출력이 없는 경우의 거절, 고급 Router의 기존 채널 빈 route, facade의 잘못된 선택을 각각 유지하고 검증한다. 0 target 구성의 실제 소유권과 sync 동작은 이미 명시한 spike로 확인할 수 있다.
- **[처리담당]** 수용. §5.1에 **route 경우표**를 유일 기준으로 두고 H8·§4·§6·§7·§8.1이 그것을 가리키게 했다: ① 구성 전체 출력 0 거절 유지 ② 고급 Router의 채널별 빈 route는 지금처럼 `Unrouted` + `NoRoute`(`tc_003_routing` 시험 불변) ③ facade 빈 선택 `InvalidOutputs` 유지 ④ observer-only만 core 이벤트 없음(`Unrouted`도 내지 않아 「출력 없음」 의미 보존)·손실 계수 없음, 구성 전체 target 0은 observer가 있을 때만 ⑤ 혼합. 판정은 기록이 가진 route revision 기준. §6 「네트워크 전용은 기존 `NoRoute`」·단계 0 「`Unrouted`·`NoRoute` 정산」 문구 제거. MS-013 r6 §2.3 H8·§4·§5.1·§6·§7·§8.1.

## 4. 다음 회차

RV-251~255는 호출자가 관측하는 결과와 수락 기준을 정하는 항목이다. 원자 순서·링 자료구조·실제 성능은 합의된 spike와 후속 검증에 맡긴다. 위 지적을 반영한 개정을 REV-079에서 대조하고, 이전 지적이 모두 해소되고 새 지적이 0건인 회차까지 반복한다.

**[처리담당] 2026-11-27:** 5건 모두 수용해 MS-013 r6에 반영했다(반영표 MS-013 §15). §4의 구분(관측 결과·수락 기준 vs spike 판정)을 따라 계약만 닫았다. RV-253 처리 중 r5 §5.9가 기존 host 기한 보고(`Progress::Incomplete`)를 빠뜨린 것을 함께 바로잡았다. 재리뷰: [REV-079 요청](REV-079_REQUEST.md).

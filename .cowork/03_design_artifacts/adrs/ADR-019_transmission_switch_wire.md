# ADR-019 — 실행 중 전송 스위치와 구독 wire 확장

- **상태:** Accepted. 2026-11-28 Human 「모두 권고대로 승인」(MS-013 §2.1 C4·C8, §2.2 P6, §2.3 H3, §2.4 `SwitchError`·client 표시 요청 구독, §9.2 Q6). 같은 날 [REV-083](../../05_verification/reviews/REV-083_MS013_R10_DESIGN_REVIEW.md) Pass.
- **범위:** INT-001 / [MS-013](../../04_implementation/milestones/MS-013.md) 단계 4([TASK-052](../../04_implementation/tasks/TASK-052.md)). 요구: FR-023.
- **관련:** [ADR-015](ADR-015_batched_pipelined_remote_ack.md)(ACK 묶음·`AcknowledgementUnknown`)·[ADR-017](ADR-017_output_model_copy_observer.md)(observer·링·hub 회계). 설계 본문은 MS-013 §5.6이 원본이다.

## 맥락

R3은 원하는 순간에 네트워크 전송을 켜고 끄기를 요구한다. C4는 「구독 먼저 → 스위치 켜기」 규약과 켤 때 시작 위치 반환을 정했다. C8은 로봇 API를 먼저 만들기로 정했다. 수신 쪽은 켜고 끈 경계를 기록 흐름 안에서 알아야 한다. 꺼진 시간이 lease보다 길면 구독이 닫히는 문제도 있다(RV-215). 리뷰 9회(REV-075~083)에서 명령 결과의 유일성, host 종료 때의 판별, End와 ACK 오류의 순서를 계약으로 닫았다.

## 결정

1. **API:** `Logger::set_output_enabled(name, enabled, deadline) -> Result<SwitchReceipt, SwitchError>`, `output_state(name) -> OutputState { epoch, enabled, subscribers }`, `switch_status(CommandId) -> SwitchStatus`. 반환 상한은 기한 + 100 ms다. 컷오프는 dispatcher가 복사하는 시점이다. 꺼진 동안 dispatcher는 기록당 원자 읽기 1회만 하고, 새 사용자 payload는 복사하지 않는다(`switched_off`). `SwitchReceipt.boundary`는 생산자별 하한(다음에 꺼낼 `accepted_seq`)이다. legacy tap 구성에는 스위치가 없다(`NotFound`).
2. **명령 결과 = 상태 셀 CAS 하나:** 모든 결과는 `Pending`(또는 `Committed`)에서 나가는 CAS 한 번으로 정해지고, 진 쪽은 효과가 없다. `Closing`·`Busy`·`Timeout`·`WorkerFailed`는 **미적용 보장**이다. `Committed { id, epoch }`는 게시를 확인하지 못한 상태이며, host가 재개하면 효과가 정확히 1회 난다. `HostFailed { id, epoch }`는 커밋 뒤 host가 종료해 epoch가 게시되지 않은 상태다. epoch는 변경마다 1씩, 게시의 **마지막 단계**에서만 전진하므로 `output_state().epoch ≥ e`와 「게시 완료」가 같은 뜻이다. host 밖에서 셀을 종결하는 근거는 host 전용 종료 표시 `host_exited` 하나뿐이다(`worker_failed`는 근거가 아니다). 미종결 명령 자리는 5개이고 종결 이력은 8개다.
3. **순서 보장:** 링의 payload 영역과 제어 영역(K = 4)은 하나의 일련번호를 공유한다. 표시는 hub에서 fence이고, 열린 Gap을 먼저 닫는다. 표시는 멀티 chunk 기록 중간에 끼지 않고 ACK 창을 쓰지 않는다. 구독마다 가입 일련번호가 있어 이전 epoch backlog를 새 epoch로 오인하지 않는다. 구독 큐 제어 자리가 넘치면 `MarkerOverflow`로 끝낸다.
4. **wire(요청한 구독에만):** 구독 요청에 protobuf 새 필드 `epoch_markers: Off | Preferred | Required`를 추가하고, 응답에 지원 여부를 표시한다. 새 frame은 3종이다: 표시(`boundary`는 packed `bytes`), 유휴 heartbeat(`max_age/2`, H3 — 건강한 연결의 유휴 lease 유지만 보장), `End{reason, cut, unacked}`(서버 관점 수). 요청하지 않은 구독과 옛 client는 지금 그대로다. `Required` + 옛 서버는 `RemoteError::Unsupported`다. 표시 frame의 최악 길이는 실제 encoder로 재서 `message_bytes` 이하인지 시작 때 검증한다.
5. **client(표시 요청 구독):** `RemoteEvent::Epoch`와 `RemoteEvent::End { reason, server_cut, server_unacked, ack_unknown }`를 추가한다. `ack_unknown`은 End를 읽은 시점에 고정된다. 구독 상태는 `Open`/`AckGone`/`Ended`/`Failed`이고, `next`·`handoff`·`flush`·`acknowledged`가 이를 공유한다. ACK 실패의 원인이 `Gone`이면 스트림을 닫지 않는다. 이때 `AckGone` 기한은 첫 `Gone` 관측 + `timeout`이며 재설정하지 않는다. 그 안에 End를 읽으면 End를 낸다. 호출 기한만 지나면 `TimedOut`(상태 유지)이고, `AckGone` 기한이 지나면 `Failed`다. 원인이 `Gone`이 아닌 ACK 실패는 지금처럼 `AcknowledgementUnknown`이고 End가 없다.
6. **범위 밖:** pre-trigger 창(C4 후속), 원격 명령(gRPC)으로 스위치 조작(C8), 조용한 단절 감지 상한(keepalive 안건).

## 결과와 비용

- 0.2에서 facade·wire·client가 추가된다. `RemoteEvent`를 완전 match하는 사용처는 breaking이다. 새 보고 타입·`EndReason`은 처음부터 `#[non_exhaustive]`다(Q7).
- 같은 major 안에서 옛↔새 client·서버 조합을 wire 호환 행렬로 시험한다(K9).
- 반환 상한·결과 유일성·epoch 판별은 단계 0 spike ③에서 먼저 확인한다. 원자 순서와 실행 시간은 골든·spike·구현 단계의 판정 항목이다(REV-083 §3).

## 검증

[TASK-052](../../04_implementation/tasks/TASK-052.md)과 [TC-015](../../05_verification/test_case.md#tc-015--출력-구성네트워크-전용전송-스위치) 4단계에서 확인한다. 항목은 MS-013 §5.6 시험 목록 전부, 명령 결과표 경로별 결과 하나, End 수치 표, End·ACK·EOF 순서표, End·ACK 오류 선착 순서표, 구독 상태 표, wire 호환 행렬이다.

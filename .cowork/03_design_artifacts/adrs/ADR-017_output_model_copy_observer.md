# ADR-017 — 출력 모델: 파일은 완료 추적 target, 네트워크는 Route 복사 observer

- **상태:** Accepted. 2026-11-28 Human 「모두 권고대로 승인」(MS-013 §2.2 P1~P3·P7, §2.3 H1·H4·H5·H6·H8, §2.4, §9.2 Q2·Q4·Q7). 같은 날 독립 재리뷰 [REV-083](../../05_verification/reviews/REV-083_MS013_R10_DESIGN_REVIEW.md) Pass(새 지적 0건).
- **범위:** INT-001 / [MS-013](../../04_implementation/milestones/MS-013.md) 단계 0~2·5([TASK-048](../../04_implementation/tasks/TASK-048.md)~[050](../../04_implementation/tasks/TASK-050.md)·[053](../../04_implementation/tasks/TASK-053.md)). 요구: FR-019a·021·022, NFR-009·010.
- **관련:** [ADR-007](ADR-007_embedded_ports_adapters_pipeline.md)·[ADR-008](ADR-008_bounded_ownership_completion.md)(완료 계약은 바꾸지 않음)·[ADR-012](ADR-012_facade_owned_writer_handles.md)·[ADR-015](ADR-015_batched_pipelined_remote_ack.md)(ACK 흐름 제어 재사용)·[ADR-018](ADR-018_config_file_and_merge.md)·[ADR-019](ADR-019_transmission_switch_wire.md). 설계 본문은 MS-013 §4·§5.1~5.4·§5.8~5.10이 원본이다.

## 맥락

R1·R2(2026-11-26 Human)는 파일·네트워크 출력을 임의로 조합하고, 파일 없이 정해진 메모리 안에서 네트워크로만 보내는 모드를 요구한다. 지금은 원격 출력이 파일 IO 뒤의 tap(`remote()`)이라 파일 출력이 없으면 시작하지 않는다(`Missing::FileOutput`). r1~r3에서는 네트워크를 완료 추적 대상(`Handed` 경계)으로 넣는 안을 검토했다. 그 안은 느린 구독이 원본 guard를 붙들어 파일·RT 자원을 잠식하고, 종료를 늦추고, 관리 wire 완료 마스크까지 바꿔야 했다(REV-074~076 RV-213·216·229·236).

## 결정

1. **두 종류의 출력:** 파일은 지금처럼 Router의 tracked target이다(Required, 완료 셀에 들어감). 네트워크는 **`Route.observe`의 복사 observer**다. dispatcher가 기록을 분배하는 그 턴에, 같은 `SharedPayload` 수명 안에서 비차단 SPSC 링에 payload를 복사한다. observer는 guard를 받지 않고 completion registry에도 없다. 새 core 완료 경계, 새 `DispatchOutcome`, 관리 `GetCompletion` wire 변경은 없다(r2·r3 `Handed` 철회). 공개 `OutputRole`도 두지 않는다.
2. **route 경우표(H8):** observer-only 기록(`targets=[]` + `observe=Some`)은 셀 0으로 즉시 종결되고, core 이벤트·손실 계수가 없다. 결과는 네트워크 회계 L1에만 남는다. 구성 전체 출력 0, 고급 Router의 빈 route(`Unrouted` + `NoRoute`), facade 빈 선택(`InvalidOutputs`)은 지금 동작 그대로다. ①은 구성 단위로, ②·④·⑤는 기록이 가진 route revision 단위로 판정한다(MS-013 §5.1 표가 유일한 기준이다).
3. **자원 격리:** 원본을 길게 붙드는 일은 구조적으로 0이다. 복사 비용 상한은 턴당 `copy_bytes_per_turn`이다. 넘치면 그 분배의 네트워크 몫만 버리고 `copy_budget_dropped`로 센다. 링이 차면 `ring_full`이다. 가장 오래된 항목은 지우지 않는다. 복사 비용이 파일에 주는 영향의 허용치가 **H6**이다. 고정 프로필의 60 s 무손실 파일 완료율을 P라 할 때, 0.8P 부하에서 RT `Capacity`·파일 실패 0이어야 하고 포화 완료율 감소는 10% 이하여야 한다. 단계 0에서 1차 판정(go/no-go)하고 단계 5에서 확정한다.
4. **ACK(H1):** 모든 구독은 기존 ACK 흐름 제어(ADR-015)를 그대로 쓴다. 결과(`acked`·`unacked`)는 네트워크 회계에만 남기고 원본 완료·`is_success()`에는 묶지 않는다. 「ACK 끄기」 옵션은 없다.
5. **손실 회계(C1):** L1 원본(`observed = copied + ring_full + copy_budget_dropped + switched_off + output_failed`), L2 링, L3 구독 사본(`offer_pending`·`in_queue`·`ack_pending`은 순간값)을 hub 잠금 안에서 센다. 구독 종료 때 `cut`과 `unacked`를 구별한다. 네트워크 스레드 실패와 종료 기한의 정산은 host가 감독한다. 순서는 생산 닫기 → 봉인 → 위치 정산이고, 각 단위를 정확히 1회 정산하며 재실행해도 결과가 같다. `finalized`(계수 확정)와 `reclaimed`(스레드 회수)는 따로 둔다.
6. **선언과 기본 경로:** `network_output(name, NetworkOutput)`은 0.2에서 최대 1개다(H4). `remote()`와 함께 선언하면 `StartError::Conflict`다. 네트워크 출력이 없는 구성의 tap은 지금과 완전히 같다(C5). 출력을 지정하지 않은 채널은 파일 출력이 있으면 모든 파일로, 없으면 모든 출력으로 간다(H5). 시작 조건은 「출력 하나 이상」(`Missing::Output`)이다.
7. **종료(호출자 감독):** 호출자 수신 상한은 지금처럼 기한 + `REPORT_GRACE`이고, 보고는 기한 보고(`Incomplete`, 최대 1회)와 `Closed` 2종이다. `Closed`는 구성된 자원(core·파일 IO·네트워크·legacy tap)만 기다린다. 파일 판정은 IO 스레드별 발행 셀로 한다. 셀 CAS는 IO 스레드의 마지막 동작이라 판정이 단조롭다. `files_complete()`·`files_settled()`·`files_late: Option<bool>`로 보고한다. `is_lossless()`는 구성된 모든 출력 기준이다.
8. **모드별 보장(C10):** 모드는 파일만 / 파일 + legacy tap / 파일 + 네트워크 / 네트워크만 4종이다. `Health.durability`는 파일이 있으면 `File { sync_every }`, 없으면 `None`이다. 네트워크 전용은 내구성이 없고 UPS 마감도 적용하지 않는다. 손실은 네트워크 회계와 선택 `loss_file`이 알린다.

## 결과와 비용

- 파일 경로의 완료 의미와 관리 wire는 그대로다. 대신 core `Route` 공개 struct에 필드가 추가되고(`Route.observe`), facade 보고 타입에 필드·변형이 추가되며, `Missing::FileOutput`이 `Missing::Output`으로 바뀐다. 모두 **0.2 breaking**이다(MS-013 §6).
- 네트워크 손실은 「완료 실패」가 아니라 회계 수치로 드러난다. 파일 성공(`files_complete()`)과 네트워크 무손실은 따로 판정한다.
- 조용한 단절의 감지 상한은 없다. 그 손실은 hub 종료 처리에서 `cut`·`unacked`로 늦게 드러나며, 감지 상한은 keepalive 안건으로 남긴다.
- 무손실 최대 처리량은 보장 목표가 아니라 측정해서 공개하는 값이다(NFR-009, P7). 루프백 4 KiB 이상에서 ≥ 300 MB/s는 내부 목표다.
- H6에서 탈락하면 단계 1 전에 이 결정을 다시 검토한다(예: 복사를 네트워크 스레드 쪽으로 옮기는 대안). 그때는 정정 ADR이 필요하다.

## 검증

단계별 Red 시험과 판정 기준은 MS-013 §8이 원본이다. 단계 0 spike·골든은 [TASK-048](../../04_implementation/tasks/TASK-048.md), core observer는 [TASK-049](../../04_implementation/tasks/TASK-049.md), facade·회계·종료는 [TASK-050](../../04_implementation/tasks/TASK-050.md), 처리량과 H6 확정은 [TASK-053](../../04_implementation/tasks/TASK-053.md)이 맡는다. 시험 케이스는 [TC-015](../../05_verification/test_case.md#tc-015--출력-구성네트워크-전용전송-스위치)다.

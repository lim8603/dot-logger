# REV-068 — TASK-031 T7 마감 변경 자가검토

| 항목 | 내용 |
|---|---|
| 일자 | 2026-11-21 |
| 대상 | `af68578` 손실 checkpoint 0 행 생략, `4ac9bf1` 묶음·파이프라인 ACK, `0637601` 하드웨어 CRC-32 |
| 판정 | RV-162 조치 뒤 수락 |
| 상태 | Closed |

| ID | 중요도 | 확인한 동작 | 처리 상태 |
|---|---|---|---|
| RV-162 | P3 | `next`가 frame을 연속으로 받으면 `biased select`가 frame만 골라 기한이 지난 ACK를 늦게 보낼 수 있다. 진행 중인 RPC가 끝나도 다음 `handoff`·타이머 전까지 대기 ACK를 보내지 않는다 | Closed — 위임받은 수정 범위 |

## 검토 관점

- **0 행 생략:** 예약은 dense 최악값을 유지한다. part 수는 `max(1, ceil(0이 아닌 행/64))` 이하로 예약 안에 들어간다. 생략 키를 0으로 보는 비교는 완전한 v2 checkpoint에만 적용하고, 이렇게 채운 표본도 `max_rows`에 포함한다. 같은 session·epoch의 모든 출력이 같은 Bank를 공유하므로 관측된 키는 모든 observer의 등록 키다. 복구는 v2 구간의 행 수를 파일 metadata와 대조하고, 채널·생산자 등록도 기존처럼 검사한다. [EV-103](../verification_evidence.md#ev-103)
- **묶음 ACK:** 서버는 묶음 항목을 transfer로 찾으므로 동시에 보낸 묶음의 도착 순서와 무관하다. 확인 전 항목은 서버 레코드에만 남아 기존 TTL로 Unknown이 된다. 묶음 길이는 서버에 남은 미확인 레코드 수를 넘을 수 없고, 넘는 요청은 적용 전에 거절한다. 클라이언트의 동시 RPC 4개 상한은 대기열 크기와 함께 ACK 대기 메모리를 제한한다. 소비자 수락 뒤 취소는 보내지 않고 `AcknowledgementUnknown`이다. 확인 결과를 보고하는 CLI·예제는 `flush` 뒤에 보고한다. [EV-104](../verification_evidence.md#ev-104)
- **CRC:** `crc_update`의 입출력은 반전 전 상태값 그대로다. 비트 단위 기준 구현과 표준 검사값, 길이·분할·임의 상태를 대조하는 시험을 교체 전에 넣어 Green을 확인했다. Miri에서도 통과했다. 새 crate는 없고 lockfile에 의존 관계 한 줄만 추가됐다. [EV-105](../verification_evidence.md#ev-105)

## RV-162

**[리뷰어]** `next_inner`의 `tokio::select!`는 frame 수신을 먼저 고른다. 큰 기록의 chunk처럼 frame이 계속 준비돼 있으면 ACK 기한 분기가 선택되지 않는다. 기록마다 `handoff`가 호출되므로 큰 손실은 없다. 다만 지연 상한 5 ms를 보장하지 않고, 진행 중인 RPC가 끝난 뒤 대기 ACK를 즉시 보내지도 않는다.

**[처리담당]** 수용했다. 매 반복에서 진행 중인 RPC를 회수한 뒤, 가장 오래된 ACK가 기한을 넘었거나 진행 중인 RPC가 없으면 frame을 기다리기 전에 대기 ACK를 보낸다. 두 Rust workspace 각 705 Pass, client·CLI·viewer 시험 10회 반복 실패 0, 루프백 소형 60,000건 ACK p99 2.94 ms·대용량 64건 ACK를 확인했다. 위임 범위에서 수락한다.

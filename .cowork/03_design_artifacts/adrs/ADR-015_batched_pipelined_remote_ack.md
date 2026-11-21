# ADR-015 — 원격 수신 ACK의 묶음·파이프라인 전송

- **상태:** Accepted — 2026-11-21 Human 「묶음·파이프라인 ACK (권장)」
- **범위:** INT-001 / MS-009 / TASK-031 T7 P4, NFR-001·002
- **관련:** [ADR-009](ADR-009_storage_transport_toolchain.md) · [ADR-014](ADR-014_viewer_live_websocket.md) · [EV-102](../../05_verification/verification_evidence.md#ev-102) · [API §14.4](../interface_contract.md#144-task-013-native-clientcli-api--2026-10-31)

## 맥락

[EV-102](../../05_verification/verification_evidence.md#ev-102)의 실제 LAN(RTT 평균 2 ms)에서 256 B 기록을 초당 10,000건 받을 때 ACK가 초당 약 440건에 그쳤다. 클라이언트 `handoff`는 기록마다 단항 `Ack` RPC를 보내고 응답을 받은 뒤에야 반환한다. 그래서 수신 처리량의 상한이 1/RTT다. 서버 `TCP_NODELAY`로는 해소되지 않았다. 원격 wire 계약과 `handoff`가 반환하는 시점의 의미가 함께 바뀌므로 ADR로 기록한다.

## 결정

1. **wire:** `AckRequest`에 `repeated AckItem items`를 추가한다. `AckItem`은 transfer·generation·session·producer·sequence를 담는다. `items`가 비어 있으면 기존 단건 필드를 읽는다. 하나의 묶음은 구독의 수신 창(`subscription_records`)을 넘을 수 없다. `major=1`은 유지하며, 서버는 단건과 묶음을 모두 받는다. 새 클라이언트는 묶음만 보내므로 이 변경 이전의 서버와는 ACK가 맞지 않는다. 배포 전이므로 이를 허용한다.
2. **서버 적용:** 묶음 항목을 순서대로 기존 단건 검증(같은 실제 연결·transfer·generation·receipt)으로 처리한다. 첫 실패에서 멈추고 오류를 반환한다. 그 전 항목의 확인은 유지한다. 실패 항목과 이후 항목은 기존 TTL에 따라 Unknown이 된다.
3. **클라이언트:** `handoff`는 소비자가 수락하면 ACK를 구독의 유한 버퍼에 넣고, 서버 응답을 기다리지 않고 반환한다. 버퍼는 묶음 상한이 차거나, 가장 오래된 항목이 짧은 지연 상한을 넘거나, 호출자가 `flush`할 때 RPC 하나로 보낸다. 동시에 진행하는 ACK RPC 수도 유한하며, 상한에 도달하면 가장 오래된 RPC를 기다린다. `next`는 다음 frame을 기다리는 동안 지연 상한에 맞춰 버퍼를 보낸다.
4. **오류 의미:** 서버 확인 전에는 수신 애플리케이션의 완료를 주장하지 않는다는 기존 계약을 유지한다. 이미 보낸 묶음의 실패는 다음 `handoff`·`next`·`flush`에서 `AcknowledgementUnknown`으로 드러난다. 그때 구독을 닫는다. 실패한 묶음과 아직 보내지 않은 항목의 기록은 서버에서 Unknown이다. 자동 재전송은 하지 않는다.
5. **종료:** `flush`는 버퍼를 보내고 진행 중인 ACK RPC를 모두 기다린다. Drop은 보내지 않은 ACK를 확인했다고 주장하지 않는다. 해당 기록은 서버 TTL에 따라 Unknown이 된다.

## 결과와 비용

- 수신 처리량이 RTT 대신 묶음 크기와 동시 RPC 수에 따라 정해진다. ACK 지연은 대략 지연 상한 + RTT다.
- 기록당 RPC 비용이 묶음 단위로 줄어든다.
- 실패가 `handoff` 호출 하나에 대응하지 않고 나중 호출에서 묶음 단위로 보고된다. 소비자는 `flush` 결과로 종료 시점의 확인 여부를 알 수 있다.
- 구현·TDD·LAN 재측정은 TASK-031 T7에서 진행한다. 결정 상수(묶음 상한·지연 상한·동시 RPC 수)는 구현에서 정하고 시험으로 확인한다.

## 구현 기록 — 2026-11-21

제품 `c0f23bb`에서 묶음 상한 64건, 지연 상한 5 ms, 동시 ACK RPC 4개로 구현했다. 진행 중인 RPC가 없으면 즉시 보낸다. 서버 소켓에는 `TCP_NODELAY`를 함께 설정했다. LAN P4 소형은 60,000건 전부 ACK, p99 10.86 ms였다. [EV-104](../../05_verification/verification_evidence.md#ev-104)

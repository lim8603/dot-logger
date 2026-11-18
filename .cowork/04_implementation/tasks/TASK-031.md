# TASK-031 — 원격 소비처 dot-logger-viewer와 시각화 bridge

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-003·004 |
| 요구사항 | FR-005 FR-006 FR-010 NFR-001 NFR-007 NFR-008 |
| 설계 | [인터페이스](../../03_design_artifacts/interface_contract.md) · [CLI/화면](../../03_design_artifacts/ui_spec.md) |
| ADR | ADR-007·009·[014](../../03_design_artifacts/adrs/ADR-014_viewer_live_websocket.md) Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-18 |
| 다음 액션 | S3 — Query/Replay 경로와 Lichtblick 원본 MCAP 재생 방안 확인. S2 Foxglove·T7 추가 검증 |
| 의존 | TASK-012·TASK-013·TASK-030(S1) |
| 예상 소요 | 2~3 인일, 리뷰 대기 제외 |

## 목표

같은 저장소의 별도 crate `dot-logger-viewer`가 로봇 프로세스의 기존 gRPC(Subscribe/Query/Replay/Status)를 소비해 Foxglove WebSocket 프로토콜로 내보낸다. 이미지·점군·그래프 시각화는 Foxglove/Lichtblick에 맡기고 직접 구현하지 않는다.

```
[로봇] 앱 + dot-logger(remote) ──gRPC──▶ [운영 PC] dot-logger-viewer ─ Foxglove WS ─▶ Foxglove / Lichtblick
[오프라인] native MCAP standard/2 ─────────────▶ Foxglove / Lichtblick
```

- viewer는 `dot-logger-client`에만 의존하고 코어 내부를 직접 쓰지 않는다(client API 충분성 검증).
- 로봇 프로세스에는 추가 데몬·코드가 없다(ADR-007 유지). 원격 서버의 loopback 기본·TLS/인증 계약을 따른다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 기술 조사·결정 | 프로토콜/SDK 버전·라이선스·Lichtblick 호환 근거, tech_stack 반영안 Human 승인 | Done — 2026-11-18 Human 승인, ADR-014 |
| S2 | 실시간 bridge | Subscribe→WS 채널, Gap/Unknown·종료/재연결 표시, 유한 큐·느린 소비자 처리 | 로컬 구현·검증 Pass(EV-092). Foxglove 실시간·T7 LAN 실측 잔여 |
| S3 | 기록 소스 | Query/Replay 기반 시간 이동·배속, 파일 열기 | Planned |

## S1 기술 결정안 — 2026-11-18

이전 세션에서 11월 23일 주로 잡은 S1을 Human의 재개 요청에 따라 11월 18일에 착수했다. Human 「승인」을 [ADR-014](../../03_design_artifacts/adrs/ADR-014_viewer_live_websocket.md)에 반영했다. 아래는 승인 전 조사 기록이다.

| 방식 | 근거와 선택 기준 |
|---|---|
| **권고 — 공개 WebSocket v1 + `tokio-tungstenite =0.30.0`** | `foxglove.websocket.v1`의 읽기 전용 부분을 viewer에서 구현한다. 범용 WebSocket 전송 라이브러리는 handshake 기능만 사용한다. 두 Rust에서 독립 컴파일을 확인했다. 소켓별 큐·전송 기한·손실 경고와 구독 해제 시점을 viewer가 통제할 수 있다. Foxglove가 보관 처리한 규격이므로 두 앱에서 버전을 고정해 회귀 검증한다. [공개 규격](https://github.com/foxglove/ws-protocol/blob/main/docs/spec.md) · [전송 crate metadata](https://crates.io/api/v1/crates/tokio-tungstenite/0.30.0) · [하위 crate metadata](https://crates.io/api/v1/crates/tungstenite/0.30.0) |
| 공식 SDK 최신 `foxglove =0.31.0` | SDK는 MIT다. `foxglove.sdk.v1`과 `foxglove.sdk.v1.1`을 협상한다. Rust 하한 1.88이므로 현재 workspace 1.85 기준에서 선택하려면 도구체인 정책 결정이 필요하다. [crate metadata](https://crates.io/api/v1/crates/foxglove/0.31.0) · [태그 handshake](https://github.com/foxglove/foxglove-sdk/blob/sdk/v0.31.0/rust/foxglove/src/websocket/handshake.rs) · [SDK 문서](https://docs.foxglove.dev/docs/sdk) |
| 공식 SDK 이전판 `foxglove =0.25.3` | 조사 시점의 비-yanked 1.85 표기 최신판이며 MIT다. `foxglove.sdk.v1`을 협상한다. SDK는 소비자 데이터 큐가 가득 차면 오래된 메시지를 버리므로 viewer가 송신 전후 손실을 구분하는 방법을 추가 검증해야 한다. 이전 SDK 유지 비용과 전체 의존성 빌드는 별도 평가 사항이다. [crate metadata](https://crates.io/api/v1/crates/foxglove/0.25.3) · [handshake 소스](https://docs.rs/crate/foxglove/0.25.3/source/src/websocket/handshake.rs) · [SDK 버퍼 문서](https://docs.rs/foxglove/0.25.3/foxglove/struct.WebSocketServer.html#buffering) |

Lichtblick Web v1.31.0은 공개 v1과 SDK v1을 모두 제안하며, SDK v1.1은 해당 소스에서 제안하지 않는다. Lichtblick 앱은 MPL-2.0이다. Foxglove 앱은 별도 서비스/소프트웨어 이용 조건을 따르며 두 앱 모두 사용자가 준비하는 외부 소비처로 둔다. viewer 배포에는 전송 crate와 간접 의존성의 고지를 확인한다. [Lichtblick 릴리즈](https://github.com/lichtblick-suite/lichtblick/releases/tag/v1.31.0) · [협상 소스](https://github.com/lichtblick-suite/lichtblick/blob/v1.31.0/packages/suite-base/src/players/FoxgloveWebSocketPlayer/index.ts#L190) · [Lichtblick 라이선스](https://github.com/lichtblick-suite/lichtblick/blob/v1.31.0/LICENSE) · [Foxglove 이용 조건](https://foxglove.dev/legal/terms)

[EV-091](../../05_verification/verification_evidence.md#ev-091)의 루프백 실험에서 Lichtblick은 두 협상 이름으로 같은 3개 protobuf 토픽을 구독하고 값을 표시했다. Foxglove Desktop 3.5.0의 원본 파일 판독은 EV-090, 제품 실시간 검증은 S2에서 수행한다.

### S2 설계 근거

- 운영 PC에서 기본 loopback으로 시작한다. 로봇 연결은 기존 `dot-logger-client`의 TLS·read 권한을 따른다. WS는 `serverInfo`, 채널 광고/해제, 구독/취소, 메시지·상태만 다룬다. 사용자 payload 바이트는 보존한다.
- [현재 wire](../../../proto/logger.proto)의 `Record`에는 채널 ID·스키마 ID/버전·설정 revision이 있다. 기존 구독·상태 응답에 토픽 이름·인코딩·descriptor를 제공하는 카탈로그는 없다. viewer에 명시적인 크기 상한이 있는 로컬 목록을 제공해 채널 ID·스키마 ID/버전을 토픽·인코딩·메시지 타입·descriptor에 매핑한다. 스키마가 바뀌면 새 광고 ID를 사용하고, 매핑이 없거나 맞지 않으면 해당 채널을 미지원으로 표시한다. 구체적인 파일 형식과 갱신 방법은 S2 설계에서 정한다.
- 클라이언트 수·광고 크기·레코드 크기·소켓별 큐의 건수/바이트·쓰기 기한을 제한한다. viewer 큐에 소비자 전달이 수락된 시점에만 기존 구독 handoff로 ACK하며, 이 결과는 GUI 디코딩·렌더링 확인을 의미하지 않는다. 소켓 포화로 실제 버린 건, upstream Gap, 연결 단절 후 범위를 알 수 없는 Unknown을 구분한다. 마지막 수요 해제·종료 시 구독을 Drop하고 서버 측 예약 반환을 시험한다.
- T7 검증은 기존 기준을 사용한다. P4 정상 LAN 소형 10,000건/s와 소비자 1개에서 수신 전달 확인 p99 ≤100 ms, P4-bulk 검증 후 소비 가능 전달 처리량이 같은 장비 TCP 기준의 ≥80%, P5는 P3 쓰기 중 요청한 제한 조회 한 페이지 응답 p95 ≤200 ms다. 원격 포화 후 새 기록 수신과 자원 반환도 확인한다. S2/S3 제품 경로에서 실행할 항목이며 S1 호환 실험에서는 실행하지 않았다. [요구 §12](../../02_project_definition/requirement_spec.md#123-성능-목표-제안) · [TASK-023](TASK-023.md) · [TC-012](../../05_verification/test_case.md#tc-012) · [TC-014](../../05_verification/test_case.md#tc-014)

### S2 실시간 구현과 로컬 검증 — 2026-11-18

제품 `f1444d4`의 [사용법과 예제](../../../crates/dot-logger-viewer/README.md), [EV-092](../../05_verification/verification_evidence.md#ev-092), [REV-063](../../05_verification/reviews/REV-063_TASK_031_LIVE_VIEWER.md)을 따른다. 하나의 native 채널에 한 gRPC 구독을 공유하며 WebSocket의 건수/바이트와 2초 쓰기 기한을 각각 제한한다. 채널당 ACK는 순차 처리된다. 새 카탈로그는 viewer 재시작 시 적용한다. 스키마 불일치가 관측되면 이전 광고를 회수하고 상태를 표시한다.

Lichtblick Web 1.31.0 Raw Messages에서 실제 facade→gRPC→viewer→WS의 `robot/pose` x=51·y=-7과 Rust 통합 시험에서 포화 회복·세션 종료/재연결·구독 반환을 확인했다. Foxglove Desktop 3.5.0의 격리 프로필은 로그인 화면에서 실시간 검증을 멈췄다. P4/P4-bulk·P5의 LAN·기준 TCP 부하 실측은 T7에 유지한다.

### S3 경계

오프라인 native `dot-logger/standard/2` 파일 직접 열기는 Foxglove에서 확인했다. Lichtblick Web 1.31.0은 동일 파일의 토픽은 열거하지만, 메시지가 없는 첫 청크의 시각 0을 재생 시작으로 계산해 재생에 실패했다. S3에서 이 호환 문제를 처리하고 원본 파일 직접 판독을 다시 검증한다. [EV-090](../../05_verification/verification_evidence.md#ev-090) · [EV-091](../../05_verification/verification_evidence.md#ev-091)

원격 Query/Replay는 기존 RPC를 제공하는 host에서 검증한다. 현재 기본 facade 연결에서는 Query/Replay가 Unsupported라, facade 자체의 원격 재생을 목표로 추가하려면 별도 설계 판단이 필요하다. [기존 facade 계약](../../03_design_artifacts/interface_contract.md#1811-s3c-remote-선택-연결--2026-11-07)

## 변경 대상과 첫 검증

제품 경로는 `crates/dot-logger-viewer/`다. 일반 실행의 프로젝트 직접 의존은 `dot-logger-client`이며 통합 시험과 데모는 실제 코어 host를 사용한다.

첫 실패/리뷰 조건: **viewer가 느린 WS 소비자·원격 Gap·서버 종료에서 유한 자원으로 Gap/Unknown을 표시하고, 로봇 측 구독 자원을 실제 해제**.

- [x] S2 실제 TCP/gRPC·WS 통합 시험을 먼저 추가해 제품 진입점이 빠진 Red를 확인했다.
- [x] S2 최소 구현 후 `TC-014`의 실시간 제품 경로·포화와 해제 하위를 로컬 확인했다.
- [x] S2 두 Rust workspace 회귀와 Lichtblick 사용자 경로를 확인했다.

## 완료 기준

- [ ] 실시간·기록 소스 bridge와 Foxglove 계열 도구 판독.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의는 [TC-014](../../05_verification/test_case.md#tc-014), S2 증거는 [EV-092](../../05_verification/verification_evidence.md#ev-092)를 따른다.

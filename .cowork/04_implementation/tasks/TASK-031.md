# TASK-031 — 원격 소비처 dot-logger-viewer와 시각화 bridge

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-003·004 |
| 요구사항 | FR-005 FR-006 FR-010 NFR-001 NFR-007 NFR-008 |
| 설계 | [인터페이스](../../03_design_artifacts/interface_contract.md) · [CLI/화면](../../03_design_artifacts/ui_spec.md) |
| ADR | ADR-007·009·[014](../../03_design_artifacts/adrs/ADR-014_viewer_live_websocket.md) Accepted |
| 담당 | AI |
| 상태 | Done — 2026-11-21 위임 수락 |
| 마지막 갱신일 | 2026-11-21 |
| 다음 액션 | 없음 — [TASK-032](TASK-032.md) S1으로 이어진다. [수락 기록](#수락--2026-11-21) |
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
| S2 | 실시간 bridge | Subscribe→WS 채널, Gap/Unknown·종료/재연결 표시, 유한 큐·느린 소비자 처리 | Done — EV-092·095·096·097. T7 P3·P5(EV-103), LAN P4(EV-104·105) Pass |
| S3 | 기록 소스 | Query/ReadBlob 기반 시간 이동·배속, 파일 열기 | 로컬 Pass — MCAP EV-093, 원격 재생 EV-094·096, 등록·재연결 회귀 EV-095 |

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

제품 `f1444d4`의 [사용법과 예제](../../../crates/dot-logger-viewer/README.md), [EV-092](../../05_verification/verification_evidence.md#ev-092), [REV-063](../../05_verification/reviews/REV-063_TASK_031_LIVE_VIEWER.md)을 따른다. 하나의 native 채널에 한 gRPC 구독을 공유하며 WebSocket의 건수/바이트와 2초 쓰기 기한을 각각 제한한다. 채널당 ACK는 순차 처리된다(2026-11-21 [ADR-015](../../03_design_artifacts/adrs/ADR-015_batched_pipelined_remote_ack.md)부터 묶음·파이프라인 전송). 새 카탈로그는 viewer 재시작 시 적용한다. 스키마 불일치가 관측되면 이전 광고를 회수하고 상태를 표시한다.

Lichtblick Web 1.31.0 Raw Messages에서 실제 facade→gRPC→viewer→WS의 `robot/pose` x=51·y=-7과 Rust 통합 시험에서 포화 회복·세션 종료/재연결·구독 반환을 확인했다. Foxglove Desktop 3.5.0의 격리 프로필은 로그인 화면에서 실시간 검증을 멈췄다. P4/P4-bulk·P5의 LAN·기준 TCP 부하 실측은 T7에 유지한다.

### S3 경계

오프라인 native `dot-logger/standard/2` 파일 직접 열기는 Foxglove에서 확인했다. Lichtblick Web 1.31.0은 동일 파일의 토픽은 열거하지만, 메시지가 없는 첫 청크의 시각 0을 재생 시작으로 계산해 재생에 실패했다. S3에서 이 호환 문제를 처리하고 원본 파일 직접 판독을 다시 검증한다. [EV-090](../../05_verification/verification_evidence.md#ev-090) · [EV-091](../../05_verification/verification_evidence.md#ev-091)

원격 기록은 Query·ReadBlob을 제공하는 host에서 검증한다. 기본 facade 연결은 이 RPC에 Unsupported를 반환한다. [기존 facade 계약](../../03_design_artifacts/interface_contract.md#1811-s3c-remote-선택-연결--2026-11-07)

### S3 파일 호환 구현 — 2026-11-18

`a3296dc`부터 Schema/Channel 등록은 MCAP data section, 사용자 기록·RecordMeta·LossReport는 인덱스가 있는 메시지 청크에 기록한다. `standard/2`에서 두 등록 방식의 판독을 지원한다. Lichtblick 1.31.0의 새 파일 직접 재생과 기존 EV-090 원본을 `recover`로 만든 별도 사본의 재생을 확인했다. 기존 파일을 직접 열면 같은 Lichtblick 문제를 만날 수 있다. [명령 예제](../../../crates/dot-logger-viewer/README.md) · [EV-093](../../05_verification/verification_evidence.md#ev-093)

### S3 원격 재생 경로 — 검토

실제 TCP의 기존 호스트 통합 시험은 파일 Query, `ReadBlob`, replay 제어를 검증한다. 제품 facade는 이 명령의 백엔드가 없다. 기존 `Replay` 요청에는 handle/세대가 있지만 `Subscribe`에는 그 handle을 선택하는 필드가 없고, 응답 Frame의 generation은 데이터 소스 식별자가 아니다. 같은 채널의 live/source별 Replay를 수신 쪽에서 구별할 결합 계약이 필요하다. [gRPC 정의](../../../proto/logger.proto) · [실제 호스트 통합 시험](../../../crates/dot-logger/src/adapters/remote/tests.rs#L762)

| 경로 | 범위와 제약 |
|---|---|
| **권고 — viewer에서 Query/ReadBlob을 읽어 재생** | 기존 read RPC, 고정 snapshot/cursor·페이지·blob 상한 사용. viewer가 시간 이동·배속을 관리하므로 호스트의 Query 백엔드는 필요하며 gRPC wire 변경은 없다. |
| 호스트 Replay가 생성한 스트림을 전달 | 호스트가 기존 Replay 시계를 관리한다. replay handle과 WS 사용자의 gRPC 구독을 연결할 계약, 권한·세대·수명 검증, 실제 호스트 구현이 추가로 필요하다. |

2026-11-18 Human 「제안대로 진행」으로 viewer의 Query/ReadBlob 기반 재생을 채택했다. [ADR-014 개정](../../03_design_artifacts/adrs/ADR-014_viewer_live_websocket.md#s3-기록-재생-개정--2026-11-18)

### S3 원격 재생 구현과 검증

`--replay-from-ns`·`--replay-to-ns`로 session mono 구간을 선택하고 터미널의 `seek MONO_NS`·`speed N/D`로 제어한다. Foxglove v1의 time/data, 원 payload 및 한 소켓당 고정 snapshot을 사용한다. 이동 중 이전 페이지나 blob 응답이 늦어도 이전 데이터를 새 세대에 송신하지 않는다. 실제 MCAP을 읽는 Query 호스트와 Lichtblick 1.31.0에서 처음과 끝 pose, 독립 WS 수신기에서 양쪽 속도의 10개 프레임·시각을 확인했다. [사용법](../../../crates/dot-logger-viewer/README.md#remote-recorded-playback) · [EV-094](../../05_verification/verification_evidence.md#ev-094)

## 변경 대상과 첫 검증

제품 경로는 `crates/dot-logger-viewer/`다. 일반 실행의 프로젝트 직접 의존은 `dot-logger-client`이며 통합 시험과 데모는 실제 코어 host를 사용한다.

첫 실패/리뷰 조건: **viewer가 느린 WS 소비자·원격 Gap·서버 종료에서 유한 자원으로 Gap/Unknown을 표시하고, 로봇 측 구독 자원을 실제 해제**.

- [x] S2 실제 TCP/gRPC·WS 통합 시험을 먼저 추가해 제품 진입점이 빠진 Red를 확인했다.
- [x] S2 최소 구현 후 `TC-014`의 실시간 제품 경로·포화와 해제 하위를 로컬 확인했다.
- [x] S2 두 Rust workspace 회귀와 Lichtblick 사용자 경로를 확인했다.

## 완료 기준

- [x] 실시간·기록 소스 bridge와 Foxglove 계열 도구 판독. EV-092~094·096
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다. MS-009 칸은 TASK-032와 공유하므로 체크하지 않고 상태만 갱신했다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다. EV-092·095·097·103~105
- [x] 리뷰 완료. REV-063~068 Closed, 2026-10-31 위임 기준으로 수락했다. 새 H/J 결정은 ADR-014·015와 ADR-013 개정으로 승인받았다.

테스트 정의는 [TC-014](../../05_verification/test_case.md#tc-014), 검증 근거는 [EV-092](../../05_verification/verification_evidence.md#ev-092)~[EV-094](../../05_verification/verification_evidence.md#ev-094)를 따른다.

## 리뷰 조치 — 2026-11-18

[REV-066](../../05_verification/reviews/REV-066_RUNTIME_NAMESPACE_VIEWER_REVIEW.md)을 마감했다. 실시간 구독 교체와 채널별 상태, 원격 기록 재생의 실패 후 이동과 단일 채널 은퇴, MCAP 등록 판독·복구를 로컬 검증했다. [EV-095](../../05_verification/verification_evidence.md#ev-095)

## 잔여 검증과 재개 순서

- [x] Foxglove Desktop 3.5.0 실시간 구독·원격 기록 재생, 되감기와 ¼배속 표시를 확인했다. [EV-096](../../05_verification/verification_evidence.md#ev-096)
- [x] 루프백 upstream 구독 큐와 원격 연결 한도의 정량 포화 회복을 반복했다. [EV-097](../../05_verification/verification_evidence.md#ev-097)
- [x] T7 LAN P4·P4-bulk를 두 번째 장비에서 [TC-012](../../05_verification/test_case.md#tc-012)에 따라 검증했다. 수정 전에는 ACK 왕복 때문에 소형이 초당 약 440건에서 막혔다(EV-102). [ADR-015](../../03_design_artifacts/adrs/ADR-015_batched_pipelined_remote_ack.md) 묶음·파이프라인 ACK로 소형을 해소했다(EV-104). 유선에서 대용량이 TCP의 약 70%에 그친 원인은 바이트 단위 CRC였고, 하드웨어 CRC-32로 바꿔 쌍별 98.8–100.5%가 됐다. 유선 소형 p99는 4.12 ms다. [EV-102](../../05_verification/verification_evidence.md#ev-102) · [EV-104](../../05_verification/verification_evidence.md#ev-104) · [EV-105](../../05_verification/verification_evidence.md#ev-105)
- [x] [RV-161](../../05_verification/reviews/REV-067_TASK_031_T7_STORAGE.md#rv-161) 조치를 반입해 작은 loss part를 묶어 저장하고 주기 checkpoint 예산을 확보했다. [EV-098](../../05_verification/verification_evidence.md#ev-098)
- [x] 8 GiB 제한의 전체 P3 부하와 함께 P5를 재판정했다. 손실 checkpoint 0 행 생략(ADR-013 개정) 뒤 예열 포함 70초분을 70.0005초에 수락했고, 쓰기와 겹친 조회 p95는 5.03 ms였다. RV-161 Closed. [EV-101](../../05_verification/verification_evidence.md#ev-101) · [EV-103](../../05_verification/verification_evidence.md#ev-103)
- [x] TASK-031 수락을 정리했다. 다음은 [TASK-032](TASK-032.md) 운영 대시보드 S1이다.

## 수락 — 2026-11-21

S1~S3와 T7 성능 검증을 마쳤다. 실시간 bridge(EV-092·095·096), 파일·원격 기록 재생(EV-093·094·096), 포화 회복(EV-097)을 확인했다. 8 GiB P3·P5(EV-103), LAN P4 소형·유선 P4-bulk(EV-104·105)도 통과했다. 리뷰 REV-063~068은 모두 Closed다. T7 과정에서 RV-161 손실 checkpoint 0 행 생략, ADR-015 묶음·파이프라인 ACK, 하드웨어 CRC-32를 반입했다. 2026-10-31 위임 기준에 따라 AI가 수락한다. 원격 viewer의 장기·native·다른 OS 검증은 TASK-024, 실사용처 연동은 TASK-026에 남는다.

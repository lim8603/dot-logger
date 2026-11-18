# ADR-014 — 원격 viewer의 WebSocket v1과 로컬 스키마 목록

- **상태:** Accepted — 2026-11-18 Human 「승인」
- **범위:** INT-001 / MS-009 / TASK-031 S1~S3의 viewer 전송 및 재생
- **관련:** [ADR-007](ADR-007_embedded_ports_adapters_pipeline.md) · [ADR-009](ADR-009_storage_transport_toolchain.md) · [TASK-031](../../04_implementation/tasks/TASK-031.md)

## 맥락과 결정 요인

운영 PC에서 Foxglove와 Lichtblick으로 기존 gRPC 실시간 구독을 볼 수 있어야 한다. 로봇의 기존 배포·스키마·gRPC wire, workspace Rust 1.85 하한을 유지한다. 전송이 밀릴 때 화면별 손실과 상류 미관측 구간을 사용자에게 구분해 표시해야 한다. 외부 프로토콜 호환과 배포·운영 경계가 함께 바뀌므로 ADR로 기록한다.

## 승인한 결정

1. 운영 PC의 별도 `dot-logger-viewer`에서 [공개 Foxglove WebSocket v1](https://github.com/foxglove/ws-protocol/blob/main/docs/spec.md)의 읽기 전용 부분을 `tokio-tungstenite =0.30.0`의 handshake 기능으로 제공한다. 로봇의 데이터는 기존 `dot-logger-client`의 구독·ACK·TLS/read 인증을 이용한다. WS는 loopback으로 제한한다.
2. 로컬 목록의 native 채널 ID·스키마 ID/버전을 토픽·메시지 인코딩·이름·descriptor와 연결한다. viewer가 스키마를 검증한 기록의 원본 payload와 Unix ns를 전달한다. 관측된 매핑 불일치는 미지원으로 보고하고 기존 광고를 회수한다. 새 스키마는 갱신된 목록과 새 광고 ID로 운영자가 게시한다.
3. 채널별로 상류 구독을 공유하고 각 WS 소켓의 큐와 전송 시간을 제한한다. 적어도 하나의 대상 큐가 수락하면 기존 gRPC ACK를 처리한다. 마지막 수요가 사라지면 구독 수명도 끝난다. 상류 Gap, viewer 큐에서 확인한 손실, 종료·재연결 구간의 Unknown을 구분한다.

## 검토한 대안과 후속

- 공식 SDK 0.31.0은 MIT이며 Rust 1.88이 필요하다. SDK 0.25.3은 1.85를 표기하지만 내부 큐의 오래된 데이터 폐기와 viewer의 손실 표시 연동을 더 검증해야 한다. 권고안은 공개 v1의 유지 부담을 가지며 대상 앱을 고정해 회귀를 확인한다. [S1 비교·라이선스](../../04_implementation/tasks/TASK-031.md#s1-기술-결정안--2026-11-18) · [EV-091](../../05_verification/verification_evidence.md#ev-091)
- Query/Replay, 원본 MCAP의 Lichtblick 직접 재생은 TASK-031 S3 범위다. 실시간 검증에서는 원격 서버의 기존 버퍼·lease 상한이 그대로 적용된다.

## S3 기록 재생 개정 — 2026-11-18

Human 「제안대로 진행」에 따라 기존 `Query`·`ReadBlob`을 사용하고 운영 PC의 viewer에서 시간 이동과 배속을 제어한다. 호스트가 제공하는 고정 snapshot과 해당 cursor/blob capability를 그대로 사용한다. 원격 `Replay`와 WS 구독을 연결하려면 별도 결합 계약이 필요하다. [TASK-031 경로 비교](../../04_implementation/tasks/TASK-031.md#s3-원격-재생-경로--검토)

재생 모드는 viewer 프로세스의 터미널에서 제어한다. 영상 도구에는 표준 WS `time`과 데이터 메시지를 같은 session mono 시간축으로 보낸다. 이동 시 기존 viewer 출력/페이지를 세대로 무효화하며 연결당 스냅샷과 RPC는 유한하다. 호스트가 스냅샷을 만료하면 화면에 실패를 표시한다. Query 백엔드가 있는 호스트를 대상으로 한다. [실행 가이드](../../../crates/dot-logger-viewer/README.md#remote-recorded-playback)

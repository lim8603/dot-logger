# REV-063 — TASK-031 S2 실시간 viewer 개발 자가검토

| 항목 | 내용 |
|---|---|
| 일자 | 2026-11-18 |
| 대상 | `2fa1501..b319fe1`, ADR-014 |
| 판정 | 로컬 Pass. Foxglove 실시간·T7 LAN 검증 잔여 |
| 상태 | Closed — 위임된 개발 검토, [EV-092](../verification_evidence.md#ev-092) |

| 검토 지점 | 확인 결과 |
|---|---|
| client/transport와 viewer crate | viewer 실행은 기존 client의 TLS/인증·수신·ACK를 사용한다. 직접 core 의존은 시험·데모 한정이고 RT 수집과 gRPC wire에는 변경이 없다. |
| 구독과 종료 | 채널별 공유 구독의 마지막 WS 수요를 제거하면 기존 stream/connection이 Drop된다. 재시작·viewer 중지·unsubscribe 후 서버 구독 반환을 실제 호스트에서 확인했다. |
| 큐·ACK | 소켓별 유한 건수·바이트와 쓰기 기한을 검토했다. 느린 소비자는 별도 Gap, 대상 모두 거절은 ACK 없음·Unknown, 정상 대상은 같은 상류로 계속 수신한다. |
| 매핑과 노출 | 중복/잘못된 catalog 거절, Origin 제한, 원본 바이트·Unix ns, 미지원 schema에 대한 unadvertise를 확인했다. |

제품 경로에 신규 `unsafe`는 없고 검토에서 열린 코드 지적은 없다. 채널당 ACK 직렬화의 처리량은 T7 측정 대상이다. 파일/조회와 외부 앱 실시간 수락은 [TASK-031](../../04_implementation/tasks/TASK-031.md)에 남겼다.

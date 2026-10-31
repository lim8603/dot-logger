# dot-logger Rust 클라이언트

일반 경로용 Rust API다. 기본 feature는 파일 조회/재생과 협력 취소이며, `remote` feature가 실제 gRPC/TLS 클라이언트와 수신자를 추가한다. 라이브러리는 runtime이나 애플리케이션 worker를 시작하지 않는다. 원격 호출은 호스트 소유 Tokio runtime에서 실행한다.

## 원격 요청과 화면 연결

`RemoteConfig::new(endpoint)` → `RemoteClient::connect(config, &cancel)` → `request(ManagementRequest, &cancel)` 순서다. 반환값은 기존 `ManagementReply`와 고정 `ClientError`다. Query/ReadBlob/Replay/GetCompletion/ApplyConfig/RunCheck/Compare를 지원하고 호스트가 구현하지 않은 작업은 `Remote(Unsupported)`다. 소켓·TLS·인증 실패의 원문이나 credential을 오류에 넣지 않는다.

- HTTP는 literal loopback IP만 허용한다. HTTPS는 명시 CA PEM을 요구하며 외부 주소는 bearer token도 필요하다. `RemoteConfig`는 Debug를 제공하지 않는다.
- 요청/응답의 encoded 상한은 기본 263168B, 허용 2048B~16MiB다. connect/RPC/다음 기록 대기는 기본 5초, 최대 1시간이다. 디코더·Tokio/TLS/HTTP2·애플리케이션 버퍼와 identity/receipt metadata는 별도 host headroom이다. 전체 heap/RSS 한도나 처리량 보장은 아니다.
- 클라이언트는 실제 TCP 연결을 한 번만 만든다. 연결 실패 후 자동 재연결/재전송하지 않는다. `disconnect`/Drop은 실제 IO guard를 닫는다. 새 연결은 새 `RemoteClient`로 명시한다.
- `request`의 취소/기한 초과는 대기를 끝내며 이미 게시된 host 명령/진행 IO를 취소했다고 추정하지 않는다. 관리 명령의 완료가 불명이면 상태/완료를 별도로 조회한다.

실행 가능한 native Query→Replay 제어 예제:

```sh
cargo run -p dot-logger-client --features remote --example remote_query_replay -- http://127.0.0.1:50051
```

예제의 token은 `DOT_LOGGER_TOKEN`, CA 파일은 `DOT_LOGGER_CA`, TLS 이름은 `DOT_LOGGER_TLS_DOMAIN` 환경 변수에서 받는다. 테스트 호스트를 명시적으로 시작한 뒤 실행한다. 예제가 서버나 데몬을 시작하지 않는다.

## 구독과 ACK

`subscribe(&budget, filter, ReceiverConfig, &cancel)`은 bounded `RemoteSubscription`을 돌려준다. `next`는 완전 검증 기록 또는 Gap을 반환하며 읽었다는 이유로 ACK하지 않는다. `handoff(record, consumer, &cancel)`의 동기식 consumer가 수락한 뒤 같은 실제 연결로 ACK하고 서버 응답을 확인한다. 수신 CRC/offset/TTL/generation·연결 identity를 모두 검사한다.

consumer 거절은 `ConsumerRejected`이며 ACK가 없다. 소비 인계 뒤 ACK 결과를 확인하지 못하면 `AcknowledgementUnknown`이다. 새 연결로 이전 ACK를 보내거나 자동 재전송하지 않는다. 이미 인계된 데이터·전송된 ACK를 소급 취소하지 않는다. stream Drop은 해당 구독을 닫고 parent client Drop은 실제 연결을 닫는다. 외부 `PendingRecord`가 남으면 실제 Drop까지 기존 예약이 유지된다.

저수준 `ClientReceiver`는 transport를 직접 붙이는 호스트용이다. 실제 연결마다 `begin_connection`의 `ConnectionId`를 보관하고 push/세대 갱신/end/ACK 추출에 같은 identity를 전달한다. generation gate 한도가 찼다고 살아 있는 연결의 gate만 초기화하지 않는다.

## 오프라인

`open_snapshot(&budget, OfflineConfig, Vec<OfflineSource>, now_ns, &cancel)`은 원본 session/revision/동일 등록 순서·명시 읽기/인덱스/페이지/원본 예산을 사용하는 `Snapshot<File>`을 반환한다. 그 Snapshot의 page/blob API와 `Replay::new`를 그대로 사용한다. 일반 파일을 read-only로 열고 각 source는 독립 reader를 소유한다. 호스트가 prefix의 불변성과 경로 교체/삭제 방지를 보장해야 한다. active 파일에는 명시적인 완전 chunk `safe_offset`이 필요하며 손상 뒤의 prefix를 추정하지 않는다.

`Cancellation::as_atomic()`은 같은 token을 동기식 core IO에 연결한다. 진행 중 OS IO의 강제 중단이나 정확한 시간 상한을 보장하지 않으며 실제 reader 종료 뒤 예약을 반환한다.

## dot-probe 화면 계약

| 결과 | 화면 어댑터의 처리 |
|---|---|
| native Status | session/revision 표시. 미구현 운영 지표는 Unavailable이며 0으로 채우지 않음 |
| Page/Blob | snapshot/cursor/blob capability와 원래 record identity 보관. 경로로 해석하지 않음 |
| Replay transition | 확인된 generation/state 반영. seek 후 host/수신 gate 함께 갱신 |
| RemoteEvent::Record | 유한 소비 큐 인계가 성공한 경우에만 handoff 수락 |
| RemoteEvent::Gap | inclusive transfer 범위와 overflow 표시. 정상 수신으로 합산하지 않음 |
| Cancelled/TimedOut/AcknowledgementUnknown | 미완료/불명을 표시하고 실제 host IO 종료로 해석하지 않음 |
| Completion | Complete/Pending/Failed/Dropped/Unknown/Expired/NotRouted/Busy를 구분 |

화면 렌더링 완료는 수신 ACK 성능 경계가 아니다. 실제 dot-probe 화면 적용은 TASK-020/026에서 수행한다. 현재 증거는 로컬 client/server/CLI이고 실제 앱·다른 OS/native·LAN/RT/전체 RSS 성능을 대신하지 않는다.

### 구독 대기·종료와 오류 분류 (REV-022)

`RemoteConfig::timeout`은 연결/unary/subscribe handshake와 개별 `next()` 대기 상한이다. **로컬 대기만 만료된 경우 같은 구독을 유지**하며, 호출자가 같은 `RemoteSubscription::next`를 다시 호출할 수 있다. decoder와 미완성 기록은 그 구독이 소유한다. 서버가 보낸 오류(lease 만료의 TimedOut 포함)·명시 취소·연결 오류는 stream을 닫는다. 서버 오류 뒤 다음 `next`는 `None`이다. 자동 재연결/재전송은 하지 않는다.

`ReceiverConfig::ttl`은 미완성/미수락 기록 수명이면서 서버에 요청하는 idle lease다(실제 lease는 서버 max_age 이내). heartbeat는 없으므로 서버 lease를 넘는 유휴는 여전히 종료된다. **서로 다른 구독 사이의 누락은 Gap으로 복원/표시되지 않는다.** 화면 호스트는 종료/새 구독 경계를 불연속으로 표시해야 하고 이어진 무손실 스트림으로 표현하면 안 된다. 장기 유휴 tail·재구독 간 복원은 후속 범위다. CLI는 별도의 전체 작업 기한을 유지한다.

handoff의 TTL 만료/폐기된 세대는 consumer를 호출하지 않고 `Remote(Gone)`·ACK 없음으로 반환한다. 실제 consumer 거절은 `ConsumerRejected`다. 응답 손상은 `DataLoss → Corrupt`, 메시지 크기 상한은 `OutOfRange → Limit`로 구분한다. 기존 `InvalidArgument → InvalidInput`은 유지한다. bearer metadata는 sensitive로 표시하며 임의 request Debug 로깅을 추가하지 않는다.

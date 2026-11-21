# REV-018 — TASK-012 gRPC·ACK·관리 어댑터 개발 자가 리뷰

## 개요

- 일자/리뷰어: 2026-10-31 / AI, [code-review 스킬](../../../.agents/skills/code-review/SKILL.md).
- 범위: `adapters/remote`·logger.proto·선택 TLS feature/lock edge·query external authority 공유·remote 계약/소켓/독립 consumer/allocator/feature 검사.
- 상태: Closed — 2026-10-31 Human 기존 로컬 구현 수락·권한 위임. RV-032~035 조치·EV-022 로컬 Pass, native/실측 후속 유지.
- 판정: Conditional — 발견 P2 4건은 모두 조치/회귀 완료. native·LAN/실앱·CLI/후속 관리 backend는 별도다.
- 본 문서는 개발 세션의 자가 점검/조치 자산이며 별도 Read-Only 리뷰 세션이 아니다.

## 리뷰 항목

| ID | 중요도 | 관측·반례 | 처리 | 상태 |
|---|---|---|---|---|
| RV-032 | P2 | 잘못된 Authorization scheme을 token=None으로 처리하면 익명 read-only로 내려가 host에 전달 | 헤더 존재와 token 파싱을 분리, 헤더가 있으면 불일치/불량 credential을 거절 | Closed |
| RV-033 | P2 | seek 뒤 incomplete만 제거하면 늦은 이전 generation 조각이 새 원본으로 조립됨 | bounded 단조 generation gate·ready handoff 검사·과거 gate 복원 거절 | Closed |
| RV-034 | P2 | 작은 encoded 길이와 큰 native Vec capacity가 reply queue에 함께 들어감 | reply enqueue 전 capacity/metadata 한도 검사 | Closed |
| RV-035 | P2 | ReadBlob 요청 길이1에 host가2bytes를 반환해도 전체 message cap 안이면 성공 | host reply enqueue 전에 요청 length와 대조해 초과 거절 | Closed |

### [처리담당] RV-032

실제 gRPC 요청 `Authorization: Basic malformed`에서 기대 PermissionDenied 대신 DeadlineExceeded(host dispatch 후 timeout) 행동 Red를 확인했다. token parse 실패를 익명 허용으로 처리한 것이 원인이다. 익명 허용은 Authorization 헤더 자체가 없을 때로 한정했다. 정상 익명 loopback read·read/manage·TLS·오류 secret 비노출을 회귀한다. 근거: Red(비공개 원시 근거).

### [처리담당] RV-033

`discard_generations(1,2)` 뒤 generation1 chunk가 들어오면 기대 Gone 대신 성공했던 행동 Red다. receiver permit이 소유하는 bounded generation 목록을 추가해 미래 조각과 이미 ready지만 미인계인 owner도 검사한다. generation을 낮추는 요청은 상태를 보존해 Gone으로 거절한다. 실제 소비자/네트워크에 이미 전달된 데이터는 취소했다고 주장하지 않는다. 근거: Red(비공개 원시 근거).

### [처리담당] RV-034 — 호스트 reply의 숨은 할당 용량

- P2 / 작은 길이의 Vec라도 capacity가 wire cap보다 큰 상태로 host 응답 큐에 들어갈 수 있었다. encoded_len 검사만으로는 소유한 heap 용량을 제한하지 못한다.
- `Blob` 길이1/capacity4097을 max4096 응답으로 넘기는 반례에서 Limit 기대 대비 성공 Red를 확인했다. Red(비공개 원시 근거).
- `Command::respond`가 oneshot reply queue에 넣기 전에 native payload capacity와 Page의 Vec/inline capacity 합을 검사하고, wire 변환도 동일 검사를 사용한다. 과대 응답은 Limit으로 종결하며 실제 host IO 완료/Command 수명 조건은 유지한다. 최초 matrix/Miri Pass는 interim-before-reply-capacity에 보존하고 최종 source70개로 다시 검증한다.

### [처리담당] RV-035 — 요청한 blob 범위 초과 응답

실제 gRPC ReadBlob(length=1) 요청에 host가 Blob2bytes를 반환하는 반례에서 기대 ResourceExhausted 대신 성공했던 Red(비공개 원시 근거)를 확인했다. Command::respond는 native ReadBlob 요청의 length도 검사해 과대 결과를 Limit으로 종결한다. 실제 File→고정 Snapshot→Query blob capability→ReadBlob3bytes 경로 및 ACK/Unknown 두 terminal 경로를 함께 회귀했다. 최초 시험들은 interim-before-blob-range에 이력으로 보존하고 최종 소스로 전체 gate를 다시 수행했다.

## 자가 점검·검증 경계

RT 입력·원본 private unsafe·기존 파일/완료 의미를 변경하지 않는다. 원격 작업은 명시적인 general-path다. RemoteDelivery는 ACK 전 guard를 보관하고 send만으로 완료를 게시하지 않는다. QueryBudget의 crate-private 예약 authority를 공유하며 stream·receiver·pending command owner가 실제로 사라져야 예약도 반환한다. native reply와 worker/runtime headroom 구분을 포함한 관리 메모리 경계는 최종 검증에서 확인한다.

최종 결과는 아래 EV-022 근거와 변경표를 따른다. Native 공개 후(T3)·LAN 성능/전체 RSS·실제 dot-probe/관리 backend·CLI는 후속 범위다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | adapters/mod.rs·remote/mod.rs/types.rs/hub.rs/receiver.rs/server.rs/wire.rs/tests.rs | optional adapter·native API·구독/ACK/host inbox·재조립·TCP/TLS/gRPC·계약 시험 |
| 생성 | proto/logger.proto·tc_007_remote.rs·tests/fixtures/remote PEM2개 | version1 wire schema·독립 native API 계약·테스트 전용 TLS fixture |
| 수정 | query/mod.rs·lib.rs | external reservation의 crate-private 공유/remote 모듈 export. 기존 query 동작 유지 |
| 수정 | crate Cargo.toml·Cargo.lock | remote에서 tonic/tls-ring, 기존 log/tokio-rustls dependency edge 활성화. package/version 변경0 |
| 생성/수정 | scripts/check-remote-protocol.py·check-feature-matrix.py | 9message/83field·tag/type/modifier 대조·선택 protoc, core isolation/remote consumer/allocator |
| 수정 | api-consumer·rt-allocation Cargo.toml/소스 | 별도 remote feature의 native consumer·요청 메모리 계측. default/core는 network 미필수 |
| 생성/수정 | Task/API/TC/EV·공유/개인 상태·검증 근거 | 범위·source hash/로그·리뷰 및 미검증 추적 |

## 클래스/타입 변경표

| 구분 | 파일 | 타입·역할 |
|---|---|---|
| 생성 | types.rs | RemoteLimits·Permission·AccessToken·TlsIdentity·ServerConfig·OverflowPolicy·SubscriptionFilter: 예산/권한/secret/listener/필터 |
| 생성 | types.rs | Chunk·Ack·AckState·RemoteItem·ReplayAction·ManagementRequest·ManagementReply·RemoteError: native bounded DTO/요청/응답/오류 |
| 생성 | hub.rs | RemoteHub·Shared·Registry·CommandInbox·Command: 공유 예약/구독 목록·host command owner/응답 |
| 생성 | hub.rs | PendingAck·RemoteDelivery·Transfer·Subscription·SubState·Event: ACK 관측/실제 delivery·전송/Gap/lifecycle |
| 생성 | receiver.rs | Ledger·Permit·Assembly·Reassembler·ReceivedRecord: 공유 예약/동시 조립/기한/ready owner·소비 인계 |
| 생성 | server.rs | RemoteServer·Security·LimitedIo·Incoming·RpcService·UnaryRpc·AckRpc·SubscribeRpc·SubscriptionStream 및 Close guard: 명시 서버/TLS·인증·connection permit·요청/stream 취소 정리 |
| 생성 | wire.rs | private Source·Record·QueryMessage·Request·Item·Reply·SubscribeRequest·Frame·AckRequest: protobuf 경계 전용, core 공개 타입으로 노출하지 않음 |
| 수정 | query/mod.rs | Reservation·QueryBudget: crate-private authority 접근으로 remote와 같은 external ledger 사용 |

테스트 fixture의 Remove/Noop 등 보조 타입은 테스트 범위다. Noop은 MSRV1.85의 Waker::noop으로 정리했다. unsafe/RT 입력·원본 보관 타입의 신규 변경은 없다.

## 최종 검증·수락 경계 — EV-022

- [EV-022](../verification_evidence.md#ev-022)·source70개/검사11개·로그 hash(비공개 원시 근거)·검증 script(비공개 원시 근거). 두 Rust1.85.0/1.96.0 각16feature·core/remote 독립 consumer·RT allocator11·clippy/rustdoc `-D warnings`·fmt·proto9message/83field/선택 protoc Pass다.
- 원격 전용 unit10+public integration3=13개. 실제 loopback/TLS 및 8MiB32조각 CRC/소비 인계 후 ACK·Gap/Unknown·read/manage/secret 오류 비노출·host timeout 뒤 command permit 유지·request/encoded/native capacity 한도·실제 파일Query/ReadBlob/Replay·completion ACK 전 Pending/후 Complete 또는 미확인 Unknown·독립 파일Written 유지.
- 영향 Miri는 public3+receiver2+hub1=6×8 seeds=48 Pass, strict provenance. socket/TLS/실제 파일 시험은 native 로컬 시험이며 Miri로 확대하지 않는다. 기존 unsafe 영역 전체 Miri는 변경하지 않아 반복하지 않았다.
- 원격 코어 hub 초기화+receiver256B/8MiB requested-byte peak5744/8655984 ≤ 예약2668288/27833344B·잔여0. runtime/TLS/HTTP2 전체 heap/RSS나 LAN throughput 상한 증거는 아니다. 기존 RT guard 할당/해제0/0 유지.
- 초기 두 차례 matrix/Miri 결과는 interim 폴더에 보존했으며 현재 근거는 결과의 최종 source70개 hash다. 현재 hash가 같으면 성공 시험을 반복하지 않는다. 기존 package/version 집합은 그대로이고 lock에는 TLS 관련 dependency edge만2개 활성화했다.
- ADR 승격 검토: ADR-007 P&A·ADR-009 tonic/prost/Tokio와 승인된 §14의 예산·소유권·TLS/권한 정책을 구체화하는 범위다. 새 기술 계열/기존 공개 core 소유권·오류·완료 정책을 변경하지 않아 별도 ADR은 만들지 않고 §14.3에 기록했다.

제품 구현/조치 최종 Human 리뷰 전 Closed로 올리지 않는다. 서버/SDK 입력은 opaque handle을 받지만 handle별 host 권한/실제 Snapshot 수명·관리 기능 backend의 구현은 host와 후속 Task 경계다. 실제 dot-probe 보안/운영 연동·CLI·P4/LAN 성능/전체RSS·다른 OS/native 공개 후(T3)·PC 성능/UPS·fresh online audit는 미실행이며 Gate4/5 Not Ready 유지. 소스/문서는 미커밋·미게시다.

세션 마감·게시(2026-10-31): 소스 `d63fdea` origin/main 게시·EV-022 source70개 commit blob 일치. 문서는 별도 [skip ci] 커밋으로 게시한다. 게시/마감 요청은 최종 구현 승인으로 해석하지 않아 Resolved·Human 리뷰 대기를 유지한다. 앞선 미커밋/미게시 표기는 검증 당시 이력이다.

## Human 수락 — 2026-10-31 후속 세션

브리핑·반입 보고 후 Human 「승인」으로 TASK-012 기존 로컬 구현 및 본 리뷰를 수락했다. RV-032~035는 Closed다. REV-019의 추가 개선은 별도로 구현/검증하고 위임된 마감 판단으로 처리한다. 기존 Human 리뷰 대기 문구는 승인 전 이력이다. native·실앱/LAN/RSS·PC 성능/UPS와 Task/MS 전체 완료를 뜻하지 않는다.

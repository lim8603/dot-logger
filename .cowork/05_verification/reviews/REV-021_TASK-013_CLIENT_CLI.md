# REV-021 — TASK-013 Rust 클라이언트·CLI 개발 자가 점검/조치

## 개요

- 일자/담당: 2026-10-31 #008 / AI, 프로젝트 code-review 스킬 기준 적용.
- 대상: 소스 `6863268d591fcd1440d2020129f40136949ceff3`, client transport/codec/취소·offline factory·CLI·예제/사용 지침·feature/proto 검사.
- 성격: 개발 자가 점검과 TDD 조치 자산. 별도의 독립 Read-Only 리뷰 세션이 아니다.
- 판정/상태: Pass / Closed — Human의 TASK-013 진행 지시와 기존 검증·수락/마감 위임에 따라 EV-025 로컬 검증 뒤 AI가 수락했다. 검토 범위에서 새 P1/P2 미해결 항목은 확인하지 못했다.
- 수락 범위: 로컬 Rust client/server·CLI·파일/화면 API 계약. native/다른 OS·실제 dot-probe 적용·LAN/RSS/PC 성능/RT/UPS·후속 운영 backend는 별도다. Task/MS 전체 검증 완료로 확대하지 않는다.

## 리뷰 항목

| ID | 중요도 | 발견/반례 | 조치 | 검증 | 상태 |
|---|---|---|---|---|---|
| RV-039 | P2 | 유효하지 않은 재생 배속도 endpoint connect가 먼저 실행되어 입력 오류가 IO 실패 exit4로 가림 | CLI Context preflight에서 입력/모드/범위를 연결 전에 검증 | Red(비공개 원시 근거: `imported_context/task_013_2026-10-31/red-input-preflight.log`) exit4→기대2·Green | Closed |
| RV-040 | P2 | 중복 channel 등록을 config validate가 valid/exit0으로 수락 | channel ID 중복·동일 schema/version의 descriptor 불일치·metadata 형식/예산 검사 | Red(비공개 원시 근거: `imported_context/task_013_2026-10-31/red-duplicate-channel.log`) exit0→기대2·Green | Closed |
| RV-041 | P2 | 요청한 완료 boundary가 없는데 host Complete를 수락. 불량 응답 ID도 사용자 InvalidInput으로 분류 | native 응답을 요청 boundary와 대조, 불량 wire metadata는 Corrupt로 거절 | Complete Red(비공개 원시 근거: `imported_context/task_013_2026-10-31/red-completion-claim.log`)·분류 Red(비공개 원시 근거: `imported_context/task_013_2026-10-31/red-response-classification.log`)→Green | Closed |
| RV-042 | P2 | generation9 제어에 generation1 응답을 정상 전환으로 수락 | 양수 generation precondition 대조, seek는 checked +1·다른 전환은 동일 generation | Red(비공개 원시 근거: `imported_context/task_013_2026-10-31/red-replay-generation.log`)→Green | Closed |
| RV-043 | P2 | 일반 파일 아닌 offline source를 Limit/부분 완료로 분류하고 open 전 파일 종류 검사가 없음 | 경로 종류를 open 전에 검사하고 열린 descriptor도 재검사. 경로 불변성은 host 책임 유지 | 디렉터리 Red(비공개 원시 근거: `imported_context/task_013_2026-10-31/red-non-regular-source.log`) exit3→기대2·Green. FIFO 실제 실행 시험으로 확대하지 않음 | Closed |
| RV-044 | P2 | non-Unicode token env를 `.ok()`로 None으로 낮춰 잘못된 인증 설정이 가려짐 | NotPresent만 무토큰, NotUnicode는 입력 오류. String token의 길이/형식은 native client 검사 | Red(비공개 원시 근거: `imported_context/task_013_2026-10-31/red-invalid-token-env.log`) exit4→기대2·Unix Green. 실제 인증 우회 실증 주장은 하지 않음 | Closed |
| RV-045 | P2 | host Gone capability가 CLI의 일반 IO 오류로 합쳐져 만료/부재를 구분하지 못함 | `gone_or_expired`/exit3, permission/version도 별도 고정 코드로 표시 | Red(비공개 원시 근거: `imported_context/task_013_2026-10-31/red-expired-capability.log`) exit4→기대3·실제 host Gone Green. TTL 경과를 직접 기다린 시험이 아님 | Closed |

## 처리담당 검토 근거

- RT/unsafe: 새 코드는 일반 경로의 safe Rust다. core/RT/기존 unsafe 소스는 변경하지 않았다. 선택된 core consumer와 기본 client의 normal/build dependency graph에서 네트워크/압축/tracing/subtle 강제를 거절하며 독립 실행과 RT allocator11을 회귀했다.
- 실제 연결: one-shot connector가 TCP를 한 번만 공급하고 재연결 시도에는 오류를 반환한다. IO guard는 parent disconnect/Drop·EOF/오류 때 shared close와 waker 정리를 수행한다. Subscription과 ACK는 같은 Grpc Channel/실제 연결을 사용한다. 새 연결에 old ACK를 자동 전송하지 않는다.
- 수신/ACK: 8MiB32조각/CRC·소비 인계 전 Pending/후 Acknowledged, Gap·거절·끊김 Unknown·소비 인계 중 취소 후 AcknowledgementUnknown을 시험했다. 별도 CLI process의 stdout write/flush 성공에서만 ACK, 닫힌 파이프에서는 exit4/Unknown을 확인했다. 네트워크 send/화면 렌더링을 수신 활용으로 추정하지 않는다.
- 취소/수명: unary 취소 후 실제 host Command를 보관하면 command 한도가 계속 소진되고 실제 respond/종료 뒤에만 다시 사용된다. 외부 PendingRecord의 예약은 실제 Drop까지 유지한다. client timeout/cancel은 이미 실행된 host/OS IO 종료가 아니다.
- 파일/예산: original 등록·read-only 일반 파일·완전 prefix·유한 query/page/blob/read budget을 사용한다. 원본 bytes 불변, 실제 query/blob→replay start/pause/resume/seek/stop, Ctrl-C 정리, 출력 cap을 확인했다. recover는 create_new 독립 파일·file sync이며 directory sync를 주장하지 않는다. 부분 active prefix recovery는 Unsupported이고 실패 출력은 보존한다.
- 프로토콜/API: private wire9message83field의 서버/client 두 사본을 schema와 비교한다. 공개 요청/응답은 native DTO이고 tonic/prost 타입을 강제하지 않는다. decode/runtime/TLS/metadata는 host headroom이며 전체 heap/RSS cap 주장이 아니다. SDK/CLI 문서의 UI 계약은 실제 dot-probe 화면 적용을 대신하지 않는다.
- 진단/기술 선택: token/CA에 Debug를 제공하지 않고 gRPC 원문/argv/임의 payload를 오류에 넣지 않는다. clap4.6.6·기존 serde/JSON·tower/hyper-util·Tokio signal은 승인된 기술 계열의 구현이다. 신규 외부6개는 MIT OR Apache-2.0 선언 및 MSRV 로컬 검증을 기록했다. 기존 package/version 교체0, offline audit 취약점/unsound0·기존 paste 유지보수 경고1. fresh advisory update는 미실행이다.
- ADR 승격: core 소유권/오류/완료·RT/기술 계열의 변경이 아닌 승인된 client/CLI 구체화다. API §14.4·UI/tech_stack와 본 기록에 반영하며 별도 ADR은 만들지 않았다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | client cancel.rs·codec.rs·offline.rs·transport.rs·wire.rs·transport 통합 시험 | 취소·native 변환·read-only Snapshot·실제 TCP/TLS/ACK·schema 사본 |
| 수정 | client Cargo.toml·lib.rs·receiver.rs | remote 선택 의존·export·고정 ClientError 추가, 기존 수신 수명 유지 |
| 생성 | client README·remote_query_replay 예제 | host/UI 계약·native Query→Replay 제어 |
| 생성 | CLI manifest·args/commands/main/model/output·통합 시험/support·README·JSON 예제 | 버전 NDJSON·exit0/2/3/4·파일/원격 명령·Ctrl-C/출력/복구 |
| 수정 | workspace/lock·feature/proto script | CLI 등록·기존 package/version 유지·선택 root 그래프·서버/client 필드 대조 |

## 클래스/타입 변경표

| 구분 | 소속 | 타입 | 역할 |
|---|---|---|---|
| 생성 | client cancel/offline | Cancellation·OfflineConfig·OfflineSource | 공유 취소·원래 등록/읽기 예산·독립 read-only source |
| 생성 | client transport | RemoteConfig·RemoteClient·RemoteEvent·RemoteSubscription | 명시 endpoint/한도·실제 연결·구독/ACK owner |
| 생성 | client transport(private) | ConnectionState·ConnectionIo | close/waker·실제 TCP IO gate |
| 수정 | client receiver | ClientError | Cancelled/ConsumerRejected/AcknowledgementUnknown·고정 진단 |
| 생성 | client wire(private) | Source·Record·QueryMessage·Request·Item·Reply·SubscribeRequest·Frame·AckRequest | schema와 대조하는 내부 prost DTO |
| 생성 | CLI | Context·Output·Failure | 사전 검증/시계·stdout 예산/JSON·종료 분류 |
| 생성 | CLI model(private) | Manifest·Channel·Descriptor·Source·Limits | 유한 원래 등록/파일 선언 및 native 변환 |
| 생성 | CLI test | Fixture | 테스트 전용 파일 namespace 생성/정리 |

## 검증 및 다음 행동

[EV-025](../verification_evidence.md#ev-025)·source92개/검사15개 및 로그 hash(비공개 원시 근거: `imported_context/task_013_2026-10-31/results.json`)·commit blob92개 일치(비공개 원시 근거: `imported_context/task_013_2026-10-31/commit.json`). 두 Rust1.85.0/1.96.0 각16feature·workspace/기본client/core consumer·clippy/rustdoc -D warnings·fmt·proto 두 사본·CLI 선언 예제·cached audit·Miri32 Pass다. CLI10·transport7·codec1·기존 수신 수명3을 포함한다. Miri는 수신 수명3×8=24+codec1×8=8이며 socket/TLS/process/file/signal을 Miri로 실행했다고 주장하지 않는다.

로컬 구현은 수락한다. TASK-013은 로컬 완료/native 이월 상태이며 실제 dot-probe 적용·후속 backend·다른 OS/native·LAN/RSS/PC 성능/RT/UPS·Gate4/5는 잔여다. 다음 개발 후보는 TASK-014이고 이번 요청으로 자동 착수하지 않는다.

# dot-logger Rust 클라이언트

일반 경로용 Rust API다. 기본 feature는 파일 조회/재생과 협력 취소이며, `remote` feature가 실제 gRPC/TLS 클라이언트와 수신자를 추가한다. 일반 클라이언트 API는 runtime이나 애플리케이션 worker를 시작하지 않는다. 명시 실행하는 `diagnostics::SnapshotCheck`는 유한 점검용 current-thread runtime을 소유한다. 원격 호출은 호스트 소유 Tokio runtime에서 실행한다.

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

## dot-probe 실제 tick export 연결 — TASK-017 S3 첫 단위

`dot_probe_bridge`는 dot-probe `robot-serve`가 `PROBE_TICK_REPORT`에 쓰는 JSON snapshot 한 건을 읽는다. 일반 host 경로에서 실제 `CaptureOwner/Producer::try_record_raw` → `Router` → `RemoteHub::offer_delivery` → TCP/gRPC → `RemoteSubscription::handoff` → 1슬롯 소비 큐로 연결하며, 원래 capture receipt의 `ReceiverAcknowledged`를 조회한다. 원본 풀의 guard는 실제 ACK/Unknown 게시까지 유지하고 이후 회수한다. 조립/`next`만으로 ACK하지 않는다.

```sh
# 경로가 없어도 preview 가능: 파일 읽기·서버 bind·부하 없음
cargo run -p dot-logger-client --features remote --example dot_probe_bridge -- --plan /tmp/dot-probe-ticks.json

# 별도 dot-probe 실행 환경에서 ROBOT_DATA_DIR·ROBOT_PEER_ADDR와 함께
# PROBE_TICK_REPORT=/tmp/dot-probe-ticks.json을 지정해 보고서를 생성한다.
# 그 파일에 대해 별도의 명시 실행:
cargo run -p dot-logger-client --features remote --example dot_probe_bridge -- --run /tmp/dot-probe-ticks.json
```

- 예제 host 한정: 입력 ≤64 KiB·최대128 thread, raw/route/수신 큐 각1슬롯, logger 메모리 예산16 MiB, 단일 current-thread runtime, 신규 수집/인계 허용 구간2초. 정리·진행 IO와 ACK settlement는 이 구간 뒤에도 실제 종료까지 소유하며 settlement 관측은 최대3초다. 파일 읽기/OS IO의 강제 종료·프로세스 전체 RSS 상한은 보장하지 않는다. 입력 경로는 앱 exporter가 관리하는 신뢰된 일반 파일이어야 한다.
- `--plan`은 경로를 읽지 않는다. `--run`은 일반 파일을 최대64 KiB+1까지만 읽고 형태를 검증한다. 원래 bytes/누적 tick 카운터는 보존하고 logger의 별도 session/mono time을 붙인다. export의 freshness/원래 boot ID는 파일 형식에 없으므로 추정하지 않는다.
- Pass는 수신 큐의 실제 수락 및 서버 ACK 확인/원래 receipt 완료에 한정한다. 큐 거절·연결 종료/수신 후 ACK 불명은 Inconclusive이며 원격 완료를 주장하지 않는다. 소비 큐 소유권이 이미 넘어갔으면 ACK 불명이어도 소급 취소하지 않는다. 취소는 Aborted다. CLI exit는 Pass/preview 0, 미확정3, IO/형태/transport 오류4다.
- 실제 dot-probe exporter를 이용한 **명시 snapshot 연결 예제**다. logger core/RT 호출·dot-probe 코드/화면을 수정하지 않으며 상시 worker/재연결은 제공하지 않는다. 고정 RunCheck 등록은 아래 `SnapshotCheck`가 연결한다. loopback을 LAN 측정으로 승격하지 않고 capacity/network/RSS/freshness는 Unavailable이다. app exporter 자체의 비용과 logger off/on 비교, 실앱 전체 적용(TASK-020/026), PC/RT·UPS·장기/native 검증은 이 예제로 수락하지 않는다.


## 고정 RunCheck의 실제 수집·수신 점검

`diagnostics::SnapshotCheck::new(host_payload, receiver)`는 host가 선택한 export만 받는다. 이를 `CheckHost::register_routed(plan, 10, Box::new(backend), source)`로 등록하고 `run_routed` 또는 기존 `PlanCheck`/`RunCheck` 명령으로 실행한다. 임의 원격 경로/수신자/입력이나 자동 재시도는 받지 않는다. core의 `RoutedCheckBackend`는 신뢰된 host 확장점이며 보안 sandbox가 아니다.

- 실제 경계: 원 `CaptureReceipt` → raw/routing guard → loopback TCP/gRPC → 1슬롯 소비 큐 수락 → 서버 ACK → 원 receipt 완료 → 실제 연결 종료/예약 회수. byte 비교만으로 수신 ACK를 대신하지 않는다.
- host는 일반 worker에서 호출한다. 이 backend는 Tokio runtime 내부 호출을 오류로 반환하므로 async host는 기존의 별도 일반 worker에서 `handle_command`를 구동한다. 상시 worker는 자동 생성하지 않는다.
- 한 건≤64KiB, target10, 양수 revision, 단일 worker, `LoopbackOnly`/`ReceiverAcknowledged`, admission≤2초다. logger 선언16MiB와 input/JSON/큐/runtime headroom을 합산한 backend 요구량24MiB를 검사한다. 예제의 허용 메모리는32MiB다. 이는 제품 기본값·heap/RSS hard limit이 아니다. 별도의 host 관리/서버 metadata도 host가 예약한다.
- 쓰기 IO는 없으며 실제 physical write는0이다. 파일 점검 전용 directory/write/reserve 선언은 네트워크 scope에서 소비하지 않는다. network bytes/최대 capacity/LAN/RSS/freshness/원 boot identity는 Unavailable이다. 종료/ACK settlement가 admission 기한 뒤에도 이어질 수 있다.
- `handle_command`의 native 반환은 `HostCheckEvidence::{File,Routed}`다. 기존 file 호출자는 `File` variant를 소비한다. reply headroom 부족은 실행 전 거절하고 capability를 보존한다. 원격 waiter가 닫혀도 명령 permit과 실행 evidence는 실제 정리 뒤까지 유지한다. 원 Report/profile의 세션/시계/구간/유효성은 수정하지 않는다.

```sh
# 없는 입력에도 무부하 preview
cargo run -p dot-logger-client --features remote --example dot_probe_check -- --plan /tmp/ticks.json
# host가 선택한 실제 export를 한 번만 명시 실행
cargo run -p dot-logger-client --features remote --example dot_probe_check -- --run /tmp/ticks.json
# 실제 CLI 관리 권한/계획/단일 실행/remote timeout 경로의 재현
cargo test -p dot-logger-cli --test routed_check_host
```

`dot_probe_check`는 전용 로컬 host 실행 예제다. 운영 `CheckHost`에 등록한 capability는 기존 CLI `check plan`/`check run --plan ID`에서 같은 관리 프로토콜로 호출된다. CLI는 별도의 서버를 자동 시작하지 않는다. preview/Pass exit0, Aborted/Inconclusive exit3, Fail/입력 오류 exit4다. 지속 실앱 수집/기존 화면/off-on은 TASK-020/026, 장비·RT/UPS/장기/native 검증은 별도 선행조건을 유지한다.

## 합성 입력 통합·설정 비교 예제 — TASK-020 S2

`dot_probe_pipeline`은 dot-probe 형태의 **합성 입력**을 기존 제품 API로 연결합니다.
실제 dot-probe 소스나 화면을 수정하지 않으며 지속 앱 적용은 TASK-026입니다.

```sh
# DB 환경 변수·파일 접근·소켓 bind 없이 실행 계획만 출력
cargo run --locked -p dot-logger-client --features integration-example --example dot_probe_pipeline -- --plan

# 고정 이미지의 자기 소유 일회 DB에서 실제 통합/CLI까지 검사·정리
python3 scripts/check-influxdb.py --pipeline-example --output /tmp/dot-logger-pipeline-evidence
```

기존 DB에 명시 실행할 때는 `DOT_LOGGER_TEST_INFLUX_URL`, `DOT_LOGGER_TEST_INFLUX_ORG`,
`DOT_LOGGER_TEST_INFLUX_BUCKET`, `DOT_LOGGER_TEST_INFLUX_TOKEN`을 호스트에서 제공한 뒤
`--run <새로운-독점-measurement>`를 사용합니다. 기존 데이터와 겹치지 않는 namespace를
제공해야 하며 DB 데이터는 자동 삭제하지 않습니다. 토큰은 인자/보고서에 출력하지 않습니다.
예제 자신이 만든 임시 MCAP 파일과 loopback 서버는 실제 종료 후 정리합니다.

| 항목 | 변경 전 revision 1 | 변경 후 revision 2 |
|---|---|---|
| 동일 생성 입력 | 로그·I64 수치·64B raw 각10건 | 동일 생성 입력 각10건 |
| 정책 | 모두 수락 | Info 로그 제외·Normal 수치 2회당1회·raw 유지 |
| 입력 결과 | 수락30·생략0 | 수락15·의도적 생략15 |
| 파일/구독 | 각각30건 | 각각15건 |
| DB | 로그·수치20건 | 명시 route off·할당0건 |

JSON phase에는 원 session/epoch·revision/구간·환경/workload/target/boundary/input ID를 보존합니다.
원 capture receipt·metadata·payload를 MCAP 독립 읽기와 실제 TCP/gRPC/1슬롯 소비 큐에서
대조합니다. ACK는 완전 조립 후 소비 큐가 받은 경우에만 보내며, DB 수치는 변경 전 원래
10건의 receipt·Unix ns·값을 query로 대조합니다. raw는 DB에 해석/저장하지 않습니다.
`validate_config → apply_config → producer acknowledgment`로 전환하고 MCAP writer는
revision별로 별도 예약·생성합니다. DB RoutedWriter의 initial revision 계약을 유지하려고
revision 2의 DB 경로는 명시적으로 끕니다. 이 예제는 live DB revision 지원을 추가하지 않습니다.

계측 구간은 **가상 session mono 시계**의 두 10초 창(0~10초, 11~21초)입니다.
실제 sleep/성능 실측을 대신하지 않으며 출력에 `SyntheticFunctionalExample`을 표시합니다.
HTTP·ACK·신규 입력 기한은 실제 Instant 시계로 제한합니다. fanout 변경은 workload 차이로
Comparison에 남겨 인과적 rate 비교는 Unavailable입니다. CPU·RSS·capacity·RT 비용/성능을
수락하지 않습니다. 호스트가 일반 경로에서 순차 실행하는 유한 예제여서 DB 지연과 무관한
지속 파일 처리율도 보장하지 않습니다. 명시 예산32MiB와 런타임/OS RSS는 구분합니다.

종료 코드0은 이 고정 기능 프로필의 모든 선택 경계·독립 읽기·query·정리를 확인한 경우,
3은 DB/구독 결과가 프로필에 미달한 경우, 4는 입력/IO/검증 오류입니다.
S3 구독 종료/재연결 불연속 예제·실제 화면·지속 앱·실측/native는 후속입니다.

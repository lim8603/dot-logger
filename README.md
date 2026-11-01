# dot-logger

로보틱스 프로젝트에 내장할 Rust 로깅·텔레메트리 라이브러리입니다.
구성 검증, 소형/원본 수집, 유한 분배, 완료 추적·제어, MCAP 회전/복구,
조회/재생, gRPC·Rust client/CLI와 운영 계측·리포트의 로컬 구현을 제공합니다.
실제 운영 worker·dot-probe 연동과 PC/RT 성능·다른 OS/native 검증은 후속 작업입니다.

개발 도구체인은 `rust-toolchain.toml`의 Rust 1.96.0, 최소 지원 Rust 버전은 1.85입니다.
`Cargo.lock`으로 직접·간접 의존성을 고정합니다.

```sh
cargo test --workspace --all-features --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
python3 scripts/check-feature-matrix.py --toolchain 1.85.0
cargo run --locked -p dot-logger-api-consumer
```

공개 사용 예제는 [`tests/api-consumer/src/main.rs`](tests/api-consumer/src/main.rs)에 있습니다.
`SessionBuilder::validate`는 메모리 합계, 생산자별 소형/원본 풀, 중요 기록 예약,
워커 스택, CPU 목록, 필수 플랫폼 기능의 지원 상태를 검사합니다.
실행 자원을 만들거나 파일·스레드를 시작하지 않습니다.
`ValidatedConfig`는 검사한 설정과 플랫폼 상태를 읽기 전용으로 보관합니다.
실제 시작 시에는 플랫폼 기능 적용 결과와 최종 할당량도 재검사해야 합니다.

`CaptureOwner::new_tracked`는 원본 풀과 완료 창을 사전 할당하고, `scope` 안에서
수명을 빌린 생산자·소비자·관리 핸들을 제공합니다. `Router`는 유한 출력 큐로 분배하고,
writer가 `Delivery::report`로 실제 처리 결과를 보고합니다. 완료 조회는 IO를 시작하지 않습니다.
`ControlOwner`의 명시 요청으로 받은 `SyncWork`는 실제 동기화가 끝날 때까지 유지해야 합니다.
종료 기한이 지나도 진행 중 IO·loan은 유효하며, 같은 owner로 정리를 계속합니다.
시작 전 만료된 종료 sync는 실제 접근과 선행 sync가 끝난 뒤 미확인 경계를 Unknown으로 정리합니다.
Closed 보고에서도 `is_success()`와 실패·시간 초과를 별도로 확인해야 합니다.

`Router::writer_budget(target)`은 실제 출력 큐 예약을 뺀 잔여 예산을 공유합니다.
`SegmentWriter`는 대상별 활성 writer 하나를 유지하며, 명시한 최대 payload·레코드·chunk·metadata 한도를 검사합니다.
기본 writer는 무압축·CRC/index 활성이고, `flush_chunk`가 성공한 완전한 chunk의 safe offset만 공개합니다.
`write_batch(&[&Delivery])`는 chunk 목표까지 누적하고 마지막 flush 뒤 Written을 묶어 보고합니다.
배치 오류는 append·flush·보고 진행량을 보존하며, Busy 뒤에는 같은 guard의 `report`만 재시도합니다.
`write_delivery`는 즉시 Written이 필요한 단일 레코드 경로입니다.
`WriterConfig::write_buffer_bytes`로 명시한 쓰기 버퍼는 공유 출력 예산에 포함되며, 0이면 직접 씁니다.
`finish`는 summary/footer를 쓰며, `FinishedSegment::sync_work`는 실제 파일 sync 뒤 제어 요청을 확인합니다.
IO 전 sync 검증 오류는 `SyncWorkError::work`로 원래 작업을 돌려주므로 올바른 파일에서 재시도할 수 있습니다.
이미 종료한 다른 세그먼트의 미확인 기록을 새 파일의 sync로 대신 확인하지 않습니다.
다중 세그먼트 회전·동기화 조정은 TASK-008에서 이어갑니다.

메모리 수치는 애플리케이션이 명시합니다. 기본 예산이나 장비 성능을 가정하지 않습니다.
풀 payload는 `slots × slot_bytes`로 계산하고, Important 예약은 slots와 bytes를 함께 보호합니다.
구현된 입력·원본 metadata·출력 참조/복사·완료/제어 metadata는 생성 시 해당 분할 예산에 검사합니다.
완료 창은 생산자별 소형/원본 Important 예약 중 큰 값 이상이어야 하며,
Normal 수락은 다음 Important 예약 개수만큼의 재사용 가능 슬롯을 남깁니다.
파일 인코더·인덱스·upstream summary 복사까지 수량 기반의 보수적 예약과 할당 계측으로 대조합니다.
custom sink 버퍼는 어댑터 예산이며, 실제 worker와 allocator 내부 비용은 별도 예약/여유 예산에 속합니다.
현재 검증은 전체 프로세스 메모리·OS page cache·실제 장비 지연의 보장이 아닙니다.

기본 feature는 비어 있으며 네트워크 런타임과 압축은 선택 의존성입니다.

| Feature | 의존성/계획 경로 |
|---|---|
| `remote` | tonic·Tokio, gRPC 어댑터 |
| `influxdb` | reqwest·Tokio·bytes, InfluxDB v2 유한 write 어댑터 |
| `tracing-adapter` | tracing·log, 일반 로그 어댑터 |
| `compression-zstd` | MCAP의 zstd 압축 |

선택 feature의 구현·수락 범위는 모듈별 계약과 검증 근거를 따릅니다. `tracing-adapter`는 일반 경로의 명시적 tracing/log 인코딩·수집 연결을 제공합니다.
MCAP 제품 파일 API는 `storage` 모듈에 있으며, 독립 consumer는 실제 파일 생성·마감·sync 후 임시 파일을 정리합니다.

`storage::SegmentLog`는 bytes·단조시간·메시지 수·인덱스 bytes/chunk 상한에 따라 파일을 회전합니다.
`RetentionConfig`로 연속·순환·이벤트 모드와 파일/디스크/보호/조회 핀 예산을 명시합니다.
회전은 footer 마감 → 파일 sync → 이름 확정 → directory sync 후에만 closed catalog를 공개합니다.
겹친 이벤트는 보호 bytes를 중복 차감하지 않고 실제 pre/post 레코드 범위와 부족한 과거 창을 보고합니다.
조회 핀은 기한·읽기 bytes를 제한하며 실제 읽기 종료 후 명시 해제해야 삭제할 수 있습니다.
`write_batch`의 부분 실패는 `RetentionBatchError`의 appended/written/reported prefix와 원래 오류를 보존합니다.
같은 대상의 delivery를 생산자 FIFO 순서로 전달하며, 수락 prefix를 다시 쓰지 않습니다.
파일 bytes·index bytes 회전 판정은 pinned MCAP의 보수적 상한으로 실제 크기보다 일찍 회전할 수 있습니다.
`SegmentStore`는 호스트의 fresh filesystem 여유 관측과 directory sync capability를 요구합니다.
표준 `FileStore`는 새 세션/대상 디렉터리를 만들며 Unix directory sync를 구현합니다.
다른 OS에서 해당 primitive를 제공하지 않으면 명시 Unsupported로 거절하며 자동 성공으로 처리하지 않습니다.
실제 디스크 여유 탐색·외부 프로세스와의 공간 경쟁, 재기동 복구·active reader·장비 내구성은 별도 호스트/후속 검증 범위입니다.
의존성의 `compression-zstd` 옵션과 별개로 현재 제품 writer의 첫 구현은 무압축입니다.
플랫폼 상태는 어댑터가 제공하며 기본값은 `Unavailable`입니다.
`Supported`는 기능 탐색 결과이고, 실제 적용 성공이나 실시간 성능 보장이 아닙니다.

CI는 Ubuntu 24.04, Windows Server 2025, macOS 14에서 두 Rust 버전과 16개 feature 조합을 검사하도록 구성했습니다.
Windows 11과 실제 PREEMPT_RT/SSD/UPS 검증은 별도입니다. 원격 CI 실행 전에는 해당 OS 통과로 판정하지 않습니다.
MCAP 0.26.0은 압축 의존성 보안 수정에 따라 승인됐습니다(ADR-010).
2026-10-27 RustSec 검사에서 취약점·안전성 경고는 없었고, 간접 의존성 `paste`의 유지보수 중단 경고 1건을 기록했습니다.
보안 CI는 취약점과 안전성 경고에서 실패하며 유지보수 경고도 출력합니다.
프로젝트 상태·설계·검증 근거는 [`.cowork/06_evolution/project_state.md`](.cowork/06_evolution/project_state.md)를 따릅니다.

### 중단 세그먼트 복구 (TASK-009)

일반 경로의 `storage::recover_segment`는 변경이 멈춘 원본을 `Read + Seek`로 읽고 독립된 빈 sink에 복구한다. 호출자는 원래 session/revision/channel 등록, `RecoveryOptions`의 입력 크기·record/chunk 한도와 출력 `WriterConfig`, 취소 표시를 제공한다. 현재 제품의 `dot-logger/1` 무압축·nonzero chunk CRC 형식을 대상으로 하며, 완전한 chunk의 길이/CRC/참조/봉투/생산자 sequence를 모두 검사한 뒤 복사한다. 손상 뒤를 재동기화하지 않고 `RecoveryReport`로 검증 prefix와 제외 suffix/원인을 보고한다. 원본/사용자 opaque payload를 보존하고 새 표준 summary/보조 인덱스를 만든 뒤 새 파일을 sync한다. 과거 프로세스의 Written/Synced 응답은 복원하거나 추정하지 않는다.

`recover_to_store`는 새 파일 생성→복구/file sync→close→이름 확정→directory sync를 구동한다. `SegmentStore`의 배타적 namespace·기존 파일 덮어쓰기 금지 계약이 필요하며, 기본 `FileStore`의 directory sync는 Unix만 지원한다. 실패한 active/이름 확정 파일은 조사용으로 남고 자동 삭제/재시도·성공 catalog 등록은 하지 않는다. 호스트는 원본을 읽기 전용으로 열고 복구 중 변경되지 않도록 보장하며, 대상별 IO 오류를 기존 completion 실패 보고와 연결한다. 자동 시작 시 디렉터리 발견/재개·worker·StorageDegraded 계측·실제 전원 차단/다른 OS 검증은 별도 후속이다.

### 운영 계측과 여유·보존 리포트 (TASK-014/015)

`CaptureOwner::enable_telemetry`로 고정 카운터와 301개 이력을 명시 활성화하고,
관리 스레드에서 `Telemetry::sample`을 구동합니다. `Telemetry::window(10/60/300)`의
`Observed(Window)`를 `telemetry::report::ReportInput::from_window`에 전달한 뒤
`Report::build(input, profile, now_ns)`로 순수 계산합니다. 실제 사용은
[`tc_009_telemetry.rs`](crates/dot-logger/tests/tc_009_telemetry.rs)의 window 연결 예제를 참조합니다.

입력 건수는 전체 생산자/채널, 완료 건수는 지정 출력의 assignment 단위입니다.
호스트가 같은 session/revision/epoch·측정 구간의 `OutputDelta`, 물리 `BufferTrend`,
`DiskTrend`를 공급해야 합니다. 수집하지 않은 값은 `Unavailable`이며 payload bytes를
실제 점유량으로 대체하지 않습니다. 원본 참조 유지·작업/회수 예약은 해당 물리 자원 관측에
포함하고 독립 큐/풀은 별도 리포트로 계산합니다. 지연 계측·물리 gauge 수집·자동 worker와
CLI 운영 metrics backend는 이 계산 API가 구현하지 않습니다.

거절 총계 `rejected`/`acquire_rejected`는 유지하며 각각 `_capacity`, `_contract`,
`_lifecycle` 범주를 제공합니다. 용량 관련 거절만 과부하 증거로 사용합니다.
잘못된 입력·미지원 호출과 정상 종료/식별자 고갈 거절은 별도 진단이며 용량 부하로
더하지 않습니다. 부하율은 `(accepted + rejected_capacity)` 제출 **시도율**을 검증된
완료 능력과 비교합니다. 같은 raw loan 재시도나 일반 생산자 재시도도 매번 셉니다.
고유 입력 건수·유실 건수로 해석하지 않으며 acquire 실패·skip은 분자에서 제외합니다.
실제 제출 전 GeneralProducer의 종료/기한 사전 거절은 제출 시도가 아닙니다.

대상별 출력 관측은 호스트의 유한 이력으로 정렬합니다. 성공한 `sample`마다 반환된
session/revision/epoch/시각과 선택 대상의 `Completion::target_totals(id)`를 함께 보관하고,
`Window.from_ns`와 `to_ns`에 정확히 맞는 두 표본을 찾습니다. 요청한 10초 구간도 실제로는
최대 2초 일찍 시작할 수 있으므로 `to_ns - 10초`로 시작점을 추정하지 않습니다.
대상·설정된 completion boundary·세션/epoch 불일치, 누락된 끝점, overflow·Busy 또는
감소한 누적값은 `Unavailable`로 둡니다. complete/failed/dropped/unknown은 checked 차분,
`pending`은 마지막 표본의 값입니다. `Sample.outputs`의 전체 합계로 대체하지 않습니다.
호스트 이력은 301개 표본 등 명시한 상한과 자체 메모리 예산으로 관리하고 reset 시 비웁니다.
동시 capture/completion 관측은 가까운 시점이 섞일 수 있으며 전역 원자 snapshot은 아닙니다.
연결 예제는 `tc_009_telemetry.rs`의 `host_aligns_target_completion_history_to_actual_window_endpoints`입니다.

`Telemetry::history_len()`은 `Observation<usize>`를 반환합니다. 이력 오류는 길이 0과
구분하며, 독점 owner의 `reset_telemetry`는 이력을 비운 뒤 poison을 해제합니다.
기존 호출자는 `Observation::Observed(n)`/`Unavailable`을 처리해야 합니다.
`CaptureCounts` 직접 구성 시 새 범주 필드도 설정하거나 `..Default::default()`를 사용합니다.

`capacity::Profile`은 호스트가 확보한 검증 근거를 가리킵니다. 같은 장비/OS/feature·부하/codec/fanout·
revision·출력·완료 경계와 현재 세션의 유효 구간이 일치해야 부하율을 계산합니다.
분모는 global ingress와 비교 가능한 완료 능력이며 fanout assignment 합계를 그대로 쓰지 않습니다.
관측·검증·가정 기반 추정·미측정·만료는 `Value`로 구분합니다. 폐기로 큐가 안정돼도
`Overloaded`가 유지되며 의도적 필터 생략은 별도입니다.

버퍼 bytes/slots 한계, 출력/회수 정지 가정의 흡수 시간, filesystem reserve/출력 quota의
소진 시간, 순환 보존 추정과 실제 보존 timestamp span을 따로 제공합니다. 물리 파일 쓰기·삭제와
filesystem 여유 변화는 다른 관측입니다. `NoGrowth`는 양의 증가가 관측되지 않았다는 뜻이고,
`AtLimit`는 이미 여유가 없는 상태입니다. 유한 시간이 u64 ns를 넘으면 `BeyondRange`로 표시합니다.
안정 합성 입력의 계산 검증은 실제 PC/RT 예측 오차≤20%나 계측 비용≤5% 달성을 뜻하지 않습니다.


## InfluxDB v2 쓰기·수치 조회 — TASK-019 S1/S2

`influxdb` feature의 `adapters::influxdb::Writer::new(&QueryBudget, Config)`는 고정 measurement/device/channel·typed schema·원 session·유한 batch/이벤트/재시도/TTL을 검증하고 공유 external 메모리를 예약합니다. 호스트의 Tokio runtime에서 `write(&[Point], &AtomicBool)`을 명시 호출합니다. 일반 경로 전용이며 worker나 spool을 만들지 않습니다. 원본 bytes를 자동 해석하거나 전송하지 않습니다.

- 호스트가 이 session에 **새롭고 독점적인 series namespace**를 공급해야 합니다. 시작 시 기존 DB를 조사하지 않습니다. `RejectNonIncreasing`은 채널별 같은/과거 real timestamp를 생산자·배치에 걸쳐 거절하며 out-of-order도 실패입니다. timestamp를 바꾸지 않습니다. 실패한 전체 preflight는 high-water를 바꾸지 않고, 시도한 배치는 Unknown이어도 시각을 소비합니다.
- 수치는 `integer`/`float`/`boolean`, 작은 이벤트는 `event` field로 분리합니다. `session`/`producer`/`sequence`는 원 receipt field이며 tag 수를 늘리지 않습니다. 타입이 다른 채널도 동일 field type 충돌을 만들지 않습니다.
- HTTP 204는 API `Acknowledged`입니다. 400/422 부분 실패·응답 단절/timeout·진행 취소는 `Unknown`, 미시도 취소는 `NotSent`입니다. 재시도는 같은 encoded body를 사용합니다. HTTPS 또는 literal loopback HTTP, redirect/proxy/암묵 retry off, token은 sensitive header로만 유지하며 서버 오류 body를 보고서에 넣지 않습니다.
- 공유 예약은 HTTP가 소유한 body의 마지막 해제까지 유지합니다. 이전 body가 남으면 `Busy`로 다음 배치를 막습니다. HTTP/TLS/runtime 내부와 OS/RSS는 별도 headroom이며, 원격 IO 취소/rollback을 추정하지 않습니다.

S2는 OSS2.9.1/Linux amd64 digest 고정 DB에서 write/query를 대조했습니다. S3 원 Delivery/완료와 로컬 파일 격리는 아래 범위에서 검증했으며 실앱/정량 성능/native는 후속입니다. [TASK-019](.cowork/04_implementation/tasks/TASK-019.md)·[API §14.6](.cowork/03_design_artifacts/interface_contract.md#146-task-019-s1-influxdb-v2-유한-write--2026-11-01).


`Writer::query_numeric(NumericQuery, &AtomicBool)`은 고정 numeric 채널의 `[from_ns,to_ns)`를 조회합니다. max_rows≤1024·응답≤1MiB·request timeout/TTL·cancel을 적용하고, 원 session/measurement/device/channel·receipt·Unix ns·typed value를 `NumericPage`에 보존합니다. 행/bytes 상한·손상/late-error·부분 응답·기한 초과는 전체 실패이며 일부 결과를 성공처럼 반환하지 않습니다. 별도 query 예약은 Page와 실제 HTTP body 마지막 소유자까지 유지됩니다. DB engine의 CPU/RAM hard cap이나 원격 rollback을 뜻하지 않습니다. [API§14.7](.cowork/03_design_artifacts/interface_contract.md#147-task-019-s2-제한-수치-query실제-db-검증--2026-11-01).

재현 가능한 실제 DB 기능 시험(임시 loopback container·일회 credential·자기 자원 정리):

```sh
python3 scripts/check-influxdb.py --toolchain 1.96.0 --output /tmp/dot-logger-influx-check
```

Docker 접근/고정 image 다운로드가 필요합니다. 일반 `cargo test`에서는 실제 DB 시험4개를 명시 ignore하며 위 harness가 별도로 실행합니다. 기존 DB/volume/설정은 사용하지 않습니다. fixture/기능 성공을 파일 격리·실앱·장기/RT/성능/native 수락으로 확대하지 않습니다.


## InfluxDB 원본·완료 연결 — TASK-019 S3

`RoutedWriter::new(writer, completion, db_target, schemas)`로 원 session/초기 revision·DB 완료 계약·채널의 schema ID/version과 `I64Le/F64Le/BoolByte/Utf8Event`를 고정합니다. `write_delivery(delivery,cancel)`은 **host의 일반 worker에서 호출하는 동기식 한 건 처리**입니다. 원 metadata/bytes를 바꾸지 않고 전용 current-thread runtime의 로컬 IO 정리까지 Delivery를 유지합니다. 기존 async runtime 안에서는 원 guard를 미시도 오류로 반환합니다. runtime/DNS/OS 정리는 TTL 뒤에도 실제 종료까지 기다릴 수 있습니다.

`DeliveryAttempt::result()`로 write 결과를 확인하고 `publish()`로 원 receipt의 `DatabaseAcknowledged`/실패/폐기/Unknown을 게시합니다. Busy이면 같은 Attempt를 유지하고 publish만 다시 호출합니다. HTTP를 재제출하지 않습니다. 완료를 게시한 뒤에도 실제 Attempt Drop까지 원본을 유지합니다. `into_delivery()`는 identity/runtime/Busy의 미시도 guard만 복구할 수 있습니다. 미게시 Drop은 Unknown입니다.

파일 출력은 별도 host worker에서 진행합니다. 로컬 시험은 DB 정지·포화 중 파일6건/3508B·원문/metadata 보존·Written 완료를 확인했고, DB는 in-flight1/8B·추가5건 Capacity drop을 분리 표시했습니다. paired file-phase ns는 기능 관측이며 처리량/RT/≤5% 비용 수락이 아닙니다. 공유 raw 풀이 모두 고갈되면 새 수집도 거절될 수 있습니다. 실앱·정량 성능/장기·다른 OS/native는 후속입니다. [API§14.8](.cowork/03_design_artifacts/interface_contract.md#148-task-019-s3-원-deliverydb-완료파일-격리--2026-11-01).

### 일반 로그 입력 어댑터 (TASK-020 S1)

`adapters::tracing::{tracing_event, log_record}`는 애플리케이션의 기존 subscriber/sink에서
호출하는 일반 경로 훅입니다. 전역 subscriber/logger·스레드·큐·시계를 자동 등록하지 않습니다.
호스트가 제공한 scratch에 `{"target":"...","fields":{"name":"value"}}` UTF-8 JSON을
인코딩합니다. 모든 필드 값은 텍스트이며 typed 수치 데이터는 별도 수치 API를 사용합니다.
tracing 이벤트 자신의 필드만 보존하며 span context·log key-values는 포함하지 않습니다.
문자열은 그대로 JSON escape하고 나머지 tracing 값은 Debug, log 메시지는 Display로 표현합니다.

`EncodedLog::capture`는 기존 `Producer`에 한 번 제출하며 수준만 source에서 변환합니다.
호스트가 제공한 시각·채널·schema·중요도는 유지합니다. filter/sampling의 Skipped와
Capacity/Closing 등 기존 결과를 그대로 반환하며, 수락 receipt는 입력 수락만 의미합니다.
로그 수준 Error는 Important로 자동 승격하지 않습니다.
버퍼 부족은 `EncodeError::TooLarge`, 사용자 formatter 실패는 `Formatting`으로 전체 거절하며
잘린 이벤트를 제출하지 않습니다. scratch는 호스트 worker 메모리 예산에 포함해야 합니다.
사용자 Debug/Display는 할당·대기·재귀를 할 수 있으므로 이 훅에는 RT 계약을 적용하지 않습니다.

```sh
cargo run --locked -p dot-logger --features tracing-adapter --example general_logs
```

이 예제는 일반 로그→수집/소비까지입니다. 파일·DB·구독/조절 비교 통합 예제(S2),
구독 종료/재연결 화면 불연속 예제(S3), 실제 지속 dot-probe 적용(TASK-026)은 후속입니다.

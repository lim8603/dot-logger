# User Manual

> 사용자 매뉴얼 — dot-logger를 애플리케이션에 붙이고, 기록을 조회·시각화하는 방법

---

## 문서 정보

| 항목 | 내용 |
| --- | --- |
| 프로젝트 | dot-logger — 로보틱스/RTOS용 Rust 로그·텔레메트리 라이브러리 |
| 대상 독자 | ① Rust 애플리케이션 개발자(라이브러리 통합) ② 운영자(CLI·viewer·대시보드로 상태와 기록 확인) |
| 버전 | 0.1.0 (pre-release, API 변경 가능) |
| 기준 | 2026-11-21 main. 사용 예는 TASK-025 S1·S2에서 실제로 실행해 exit 0을 확인한 명령이다 |
| 원본 | 공개 README 4종(루트·cli·client·viewer, 영어)이 사용법의 1차 기준이다. 이 문서는 그 내용을 한국어 작업 문서로 묶고 실패 처리와 FAQ를 덧붙인다 |
| 운영 측면 | 설치·자원 한도·장애 대응·복구는 [운영서](operation_guide.md) |

---

## 1. 개요

### 시스템 소개

dot-logger는 실시간 스레드가 **기다리거나 할당하지 않고** 기록을 넘기게 하는 Rust 라이브러리다.

- 기본 worker가 기록을 표준 MCAP 파일로 쓴다. Foxglove, Lichtblick, `mcap` 도구로 바로 열 수 있다.
- 모든 큐·풀·파일·예약에 선언된 한도가 있다. 들어가지 못한 기록은 조용히 사라지지 않고 개수로 남는다.

### 구성 요소

| 구성 요소 | 용도 | 사용자 |
| --- | --- | --- |
| `dot-logger` (라이브러리) | 수집, 저장, 조회·재생, 어댑터, 텔레메트리 | 개발자 |
| `dot-logger` CLI (`dot-logger-cli`) | 상태, 조회, 재생, 구독(tail), 복구, 손실, 비교, 점검 | 운영자·개발자 |
| `dot-logger-client` | gRPC 서버와 오프라인 파일용 Rust 클라이언트 | 자체 화면을 만드는 개발자 |
| `dot-logger-viewer` | Foxglove WebSocket bridge와 읽기 전용 운영 대시보드 | 운영자 |

### 주요 기능 요약

| # | 기능 | 설명 | 사용 절 |
| --- | --- | --- | --- |
| 1 | 실시간 안전 기록 | `try_log`는 막히거나 할당하지 않는다. 큐가 가득 차면 거절하고 개수로 남긴다 | §3.1 |
| 2 | 메모리 한도 선언 | 메모리·큐·파일 예산을 미리 선언한다. Preset이 제어 주기로 값을 계산한다 | §3.1, §3.2 |
| 3 | 표준 파일 | 사용자 schema·encoding 그대로 native MCAP(`dot-logger/standard/2`)에 저장. 손실 카운터와 메타데이터도 같은 파일에 | §3.6 |
| 4 | 보존 | ring 보존, 이벤트 전후 보존(mark), 설정 변경 시 파일 회전, 중단된 파일 복구 | §3.3, §3.7 |
| 5 | 실시간·기록 조회 | 선택 gRPC 서버(구독·조회·재생·상태), CLI, Rust 클라이언트 | §3.4, §3.5 |
| 6 | 시각화 | viewer로 Foxglove/Lichtblick에 실시간·기록 재생, 읽기 전용 대시보드 | §3.5 |
| 7 | 운영 | 실행 중 level·샘플링 변경, 텔레메트리 창, 전후 비교, 파일 시스템 점검 | §3.3, §3.8 |
| 8 | 연동 | `tracing`/`log` 입력, InfluxDB v2 출력 | §3.2 |

---

## 2. 시작하기

### 설치

Rust 1.85 이상이 필요하다. crates.io에는 아직 없으므로 git 의존성으로 쓴다.

```toml
[dependencies]
dot-logger = { git = "https://github.com/lim8603/dot-logger", tag = "v0.1.0" }
```

- 저장소는 2026-11-21 현재 **비공개**다. 공개 전에는 접근 권한이 있는 계정에서만 받을 수 있다.
- 기본 feature는 없다. 필요한 기능만 켠다(§3.2 표).

### 첫 실행 — L1 레시피

[`examples/recipes/l1_file.rs`](../../examples/recipes/l1_file.rs)가 가장 작은 완전한 예다. 1 kHz 제어 루프용 preset으로 시작하고, 실시간 스레드에서 1,000건을 쓴 뒤 종료 결과를 판정한다.

```sh
cargo run --locked -p dot-logger --example recipe_l1_file
```

통합 순서는 다섯 단계다.

1. 메시지 타입에 `Record`를 구현한다(`SCHEMA`와 `encode`).
2. `Logger::builder()`에 preset(또는 `Budget`), 파일 출력과 보존, 채널, writer를 선언하고 `start()`한다.
3. `logger.channel::<T>(topic)`으로 채널 핸들을, `logger.take_writer(name)`으로 writer를 받는다. writer는 `'static`이라 실시간 스레드로 옮길 수 있다.
4. 실시간 스레드에서 `writer.try_log(channel, &msg)`를 호출한다. 실패는 거절 개수로 남으니 루프를 멈추지 않는다.
5. 끝낼 때 `logger.shutdown(deadline)`의 결과가 `is_success()`인지 확인한다. 모든 기록이 기한 안에 쓰이고 동기화됐을 때만 true다.

### 레시피 목록

각 레시피는 일부러 작게 유지하며, 검사 스크립트가 줄 수와 사용 이름 수를 지킨다.

| 레시피 | 보여 주는 것 | 실행 |
| --- | --- | --- |
| `l1_file.rs` | 실시간 스레드 → 파일, 종료 판정 | `cargo run --locked -p dot-logger --example recipe_l1_file` |
| `l2_rt_writers.rs` | 실시간 writer 2개, 빌린 slot에 직접 쓰는 zero-copy 프레임 | `cargo run --locked -p dot-logger --example recipe_l2_rt_writers` |
| `l3_tracing.rs` | L1 + `tracing`/`log` 입력 | `cargo run --locked -p dot-logger --features tracing-adapter --example recipe_l3_tracing` |
| `l3_remote.rs` | L1 + loopback gRPC 서버(주소 출력 후 종료) | `cargo run --locked -p dot-logger --features remote --example recipe_l3_remote` |

---

## 3. 기능별 사용법

### 3.1 기본 기록 (default path, `dot_logger::logger`)

#### 목적

애플리케이션 코드에서 타입이 있는 메시지를 실시간 안전하게 파일로 남긴다.

#### 사용 절차

1. §2의 L1 순서를 따른다.
2. 크기는 `Preset::control(hz)`로 시작하고, 직접 정해야 하면 `Budget`을 쓴다. `builder.plan()`을 먼저 호출하면 시작하지 않고 계산 결과와 경고(`PlanWarning`)를 볼 수 있다.
3. 보존은 `Retention::ring(bytes)`로 디스크 상한을 정한다.
4. 상태는 `logger.health()`로 확인한다. `Running`, `Degraded(..)`(기록은 계속되지만 일부 손실 또는 동기화 지연), `Failed(..)`(필수 기록이 더 이상 성공할 수 없음, 종료 후 점검)로 나뉜다.

#### 주의사항

- `try_log`가 거절을 돌려줘도 정상 동작이다. 거절은 개수로 남고 상태와 파일의 손실 카운터에서 볼 수 있다.
- 파일 출력 디렉터리는 미리 있어야 한다.
- `shutdown`이 `WorkerFailed`나 `Unresponsive`를 돌려주면 기록이 모두 남았다고 보면 안 된다(§5).

### 3.2 선택 기능 켜기

#### 목적

필요한 기능만 추가해 의존성과 비용을 줄인다.

#### 사용 절차

| Cargo feature | 추가되는 것 |
| --- | --- |
| `remote` | gRPC 서버·클라이언트 타입(tonic, Tokio) |
| `influxdb` | 한도가 있는 InfluxDB v2 쓰기와 숫자 조회 |
| `tracing-adapter` | `tracing`·`log` 입력(`LoggerBuilder::tracing`) |
| `compression-zstd` | MCAP 의존성의 zstd 지원(dot-logger 자체는 무압축 chunk로 쓴다) |
| `calibrate` | 장비 측정과 preset용 JSON 프로필 |

#### 주의사항

- `tracing` 연결에서 떨어진 이벤트 수는 `health().tracing_dropped`에 나온다.

### 3.3 중요 기록·level·이벤트 보존

#### 목적

포화 상황에서도 중요한 기록을 지키고, 실행 중 기록량을 조절하고, 사건 전후를 보존한다.

#### 사용 절차

```rust
.channel_with::<Status>("robot/status", ChannelOptions::new()
    .minimum(Level::Info)                // Info 미만은 건너뜀
    .important_from(Level::Warn))        // Warn 이상은 Important
.writer_with("control", WriterQuota::small(64, 256).important(8)) // Important 전용 slot 8개
...
writer.try_log(alarm.important(), &msg)?;                 // 한 건만 Important로 표시
let revision = logger.set_channel(alarm, Level::Warn, 3)?; // level과 Normal 샘플링 변경
let mark = logger.mark_event()?;                          // 지금 전후 구간 보호
```

1. **중요 기록:** writer가 `WriterQuota::important(n)`로 slot을 예약해야 포화에서 보호된다.
2. **level·샘플링 변경:** `set_channel`은 게시된 설정 revision을 돌려준다. Normal 기록은 첫 건과 `sample_every`번째마다 남긴다(1이면 전부). Important는 샘플링하지 않지만 최소 level은 적용된다. revision 경계에서 파일이 회전한다.
3. **이벤트 보존:** `Retention::ring(..).events(EventWindow::new(앞, 뒤).hold(..).limits(..))`로 선언하고, 일반 스레드에서 `mark_event()`를 호출한다. 보호는 `hold`가 지나거나 `release_event`를 호출하면 끝난다.

#### 주의사항

- 예약 없이 `important_from`만 쓰면 `plan()`이 `PlanWarning::NoImportantReserve`를 보고한다.
- 직접 받은 writer가 오래 쉬면 다음 변경이 `ConfigError::Busy`가 될 수 있다. 그 스레드에서 가끔 `writer.refresh()`를 호출한다.
- revision이 게시됐다고 파일 동기화까지 끝난 것은 아니다.
- `mark_event`는 실시간 스레드가 아닌 일반 스레드에서 호출한다.

### 3.4 CLI로 상태·기록 보기 (`dot-logger`)

#### 목적

실행 중인 호스트의 상태와 기록을 터미널에서 보고, 기록 파일을 오프라인으로 다룬다.

#### 사용 절차

1. 호스트가 `LoggerBuilder::remote`로 서버를 켠다. 기본은 loopback의 임의 포트이고, 실제 주소는 `logger.remote_addr()`가 알려 준다.
2. 원격 명령(`PORT`는 호스트가 출력한 포트):

```sh
cargo run -p dot-logger-cli -- status --endpoint http://127.0.0.1:PORT --json
cargo run -p dot-logger-cli -- query  --endpoint http://127.0.0.1:PORT --from 0 --to 1000000000 --json
cargo run -p dot-logger-cli -- tail   --endpoint http://127.0.0.1:PORT --record-bytes 65536 --limit 10
```

3. 오프라인 파일: [`crates/dot-logger-cli/examples/offline_session.json`](../../crates/dot-logger-cli/examples/offline_session.json)을 호스트 프로젝트의 `.logger/session.json`으로 복사하고 세션·채널·파일별 revision·경로·예산을 채운다.

```sh
cargo run -p dot-logger-cli -- config validate --manifest .logger/session.json --json
cargo run -p dot-logger-cli -- query   --manifest .logger/session.json --from 0 --to 1000000000 --json
cargo run -p dot-logger-cli -- loss    --manifest .logger/session.json --json
```

| 명령 | 용도 |
| --- | --- |
| `status` | 세션·revision과 `dot_logger.status.v1` 지표(Observed/Stale/Unavailable) |
| `query`, `blob` | `[from, to)` 범위 기록을 쪽 단위로, 큰 payload는 blob 범위로 |
| `replay` | 원격 재생 제어 또는 오프라인 시간 재생 |
| `tail` | 구독. N건을 출력하고 ACK를 확인한 뒤 종료 |
| `loss` | 파일에 저장된 손실 카운터(누적·차이·미관측 구간) |
| `compare` | 호스트가 등록한 두 snapshot 비교 |
| `config validate/diff/apply` | manifest 검사, 두 선언 차이 여부, 실행 중 변경 게시(Manage 권한) |
| `check plan/run` | 등록된 점검 계획 미리보기·1회 실행 |
| `recover` | 기록 파일을 새 독립 파일로 복구 |
| `calibrate` | 출력 파일 시스템을 한 번 측정해 preset 프로필 저장 |

#### 주의사항

- `--json` 출력은 NDJSON이고, 마지막 줄이 `summary` 또는 `error`다. 종료 코드는 0(완료), 2(입력·설정·권한·미지원), 3(부분 결과·취소·기한·한도·알 수 없는 ACK), 4(IO·전송·잘못된 응답)다.
- `tail --record-bytes` 기본값은 8 MiB다. 호스트 한도보다 크면 `invalid_input`이 된다(데모 호스트는 64 KiB).
- loopback이 아닌 주소는 HTTPS, CA(`--ca`), 토큰(`DOT_LOGGER_TOKEN`)이 필요하다. 토큰은 인자로 받지 않고 출력하지도 않는다.
- 실패 뒤 CLI는 다시 연결하거나 재전송하지 않는다.

### 3.5 viewer로 Foxglove·대시보드 보기

#### 목적

운영 PC에서 실시간·기록 데이터를 Foxglove/Lichtblick으로 보고, 브라우저 대시보드로 상태를 본다.

#### 사용 절차

```sh
# 데모 호스트(1분 동안 robot/pose x=51, y=-7 발행). 실행마다 새 출력 디렉터리를 쓴다
cargo run --locked -p dot-logger-viewer --example live_host -- ./viewer-demo
# 다른 터미널에서, 데모가 출력한 PORT로
cargo run --locked -p dot-logger-viewer -- \
    --upstream http://127.0.0.1:PORT --catalog ./viewer-demo/viewer-catalog.json \
    --dashboard 127.0.0.1:0
```

1. 출력된 `ws://` URL을 Foxglove/Lichtblick에서 **Foxglove WebSocket** 연결로 연다. Raw Messages에서 `robot/pose`를 고른다.
2. 출력된 `http://` URL을 같은 PC의 브라우저에서 연다. 대시보드는 1초에 최대 한 번 `GetStatus`를 보내고, 각 값의 상태(Observed/Stale/Unavailable)와 사유를 그대로 보여 준다.
3. 스냅샷 비교와 점검 계획 미리보기도 대시보드에서 할 수 있다. 시험은 `operations_host` 예제로 한다.
4. 기록 재생은 `--replay-from-ns`, `--replay-to-ns`, `--replay-speed`를 주고, viewer 터미널에 `speed 2/1`, `seek 1000000000`을 입력한다.
5. 닫힌 MCAP 파일은 viewer 앱에서 바로 연다.

#### 주의사항

- catalog(JSON)는 호스트 등록의 `channel_id`, 32자리 hex `schema_id`, `schema_version`과 맞아야 한다. 최대 1 MiB, 128채널.
- WebSocket과 대시보드는 loopback에서만 듣는다. 브라우저에는 upstream 자격 증명을 보내지 않는다.
- 대시보드는 읽기 전용이다. 설정 변경과 점검 실행은 CLI로 한다.
- 기본 facade 호스트는 snapshot·점검 계획·Query/ReadBlob을 제공하지 않아 해당 기능이 `unsupported`다.
- 일부 이전 `standard/2` 파일은 Lichtblick이 1970년부터 시작하는 범위로 보여 주며 미리 읽기에 실패한다. `recover`로 호환 사본을 만든다(§5).

### 3.6 파일 형식과 외부 도구

#### 목적

기록 파일을 별도 변환 없이 표준 도구로 읽는다.

#### 사용 절차

- 파일은 profile `dot-logger/standard/2`의 native MCAP이다. 사용자 payload와 schema를 그대로 담는다.
- `.logger/meta` 채널에 원래 receipt·시계·설정이, `.logger/loss` 채널에 손실 카운터가 있다.

#### 주의사항

- 개발 중 파일 형식(`dot-logger/standard/1`)은 입력으로 받지 않는다.
- descriptor가 없는 채널은 원래 바이트와 encoding을 그대로 둔다.

### 3.7 중단된 파일 복구

#### 목적

프로세스가 비정상 종료해 닫히지 않은 파일을 새 독립 파일로 살린다.

#### 사용 절차

```sh
cargo run -p dot-logger-cli -- recover --manifest .logger/session.json --output .logger/recovered.mcap --json
```

- 원본 하나면 `--output NEW_FILE`, 여러 개면 `--output NEW_DIRECTORY` 아래 `<sequence>.mcap`로 쓴다.
- 결과 행마다 sequence, revision, 복구 구간과 제외 구간이 나온다.

#### 주의사항

- 기존 경로는 덮어쓰지 않는다. 원본은 읽기 전용으로만 연다.
- 제외 꼬리에는 교체된 index·summary가 포함되므로 그 길이를 손실 메시지 수로 읽으면 안 된다.
- 복구는 원래 프로세스가 그 기록을 썼거나 동기화했다는 증명이 아니다.

### 3.8 고급 경로 (advanced path)

#### 목적

메모리 분할, 수집, 분배, 완료 추적, 전원 경보 대응을 직접 조립한다.

#### 사용 절차

- 모듈: `config`, `capture`, `routing`, `completion`, `control`(sync·종료·전원 경보), `storage`, `query`, `replay`, `adapters`(gRPC·InfluxDB·tracing), `telemetry`, `diagnostics`.
- 계약은 rustdoc에 있다: `cargo doc --open -p dot-logger --all-features`.
- 예: [`examples/general_logs.rs`](../../examples/general_logs.rs), `dot-logger-client`의 `dot_probe_*` 예제, [`ups_simulation`](../../crates/dot-logger/examples/ups_simulation.rs)(`--plan`, `--run DIR`).

#### 주의사항

- 기본 경로가 이 모듈들 위에 만들어졌다. 처음 통합할 때는 기본 경로를 쓰고, 필요한 부분만 고급 경로로 내려간다.

---

## 4. 자주 묻는 질문 (FAQ)

| # | 질문 | 답변 |
| --- | --- | --- |
| 1 | `try_log`가 실패했는데 루프를 멈춰야 하나? | 아니다. 큐가 가득 찬 상태의 정상 거절이고 개수로 남는다. 중요한 기록은 Important slot을 예약한다 |
| 2 | 파일은 어디에 생기나? | 출력 디렉터리 아래 세션별 디렉터리에 생긴다. 쓰는 중에는 `<sequence>.active`, 닫히면 `<sequence>.mcap`이다. 세션 디렉터리는 `logger.directories()`가 선언 순서대로 알려 준다 |
| 3 | 대시보드에 0이 안 보이고 "관측 안 됨"으로 나온다 | 의도된 동작이다. 관측하지 않은 값을 0으로 만들지 않는다. 텔레메트리는 `LoggerBuilder::telemetry()`로 켠다 |
| 4 | `tail`이 `invalid_input`으로 끝난다 | `--record-bytes`나 `--records`가 호스트 한도보다 크다. 한도 안의 값을 준다 |
| 5 | 두 번 구독한 사이의 기록은? | 두 구독 사이에 놓친 기록은 Gap으로 보고되지 않는다. 한 연속 스트림으로 취급하지 않는다 |
| 6 | Windows·macOS에서 쓸 수 있나? | CI 구성은 있지만 실행 결과가 아직 없어 미검증이다. 검증된 환경은 Linux x86_64다 |
| 7 | 압축 파일을 쓰나? | dot-logger는 무압축 chunk로만 쓰고, 압축 chunk는 조회·복구에서 거절한다. `compression-zstd`는 MCAP 의존성의 zstd 지원만 켜며 제품 경로는 쓰지 않는다 |

---

## 5. 문제 해결 (Troubleshooting)

| 증상 | 원인 | 해결 방법 |
| --- | --- | --- |
| `start()`가 `Missing(..)` | 필수 값·선언 누락(목록 전체가 나온다) | 나열된 항목을 builder에 추가한다. `plan()`으로 먼저 확인 |
| `start()`가 `InvalidBudget` | 0이거나 모순된 값(예: Important 예약이 slot 수 이상, 연속 출력에 이벤트 창) | 예산 값을 고친다 |
| `health()`가 `Degraded` | 일부 손실 또는 동기화 지연 | 손실 카운터(`loss`)와 출력별 상태를 보고 예산·디스크 속도를 점검 |
| `health()`가 `Failed` | 필수 기록이 더 이상 성공할 수 없음(IO 장애 등) | 종료하고 원인 점검. [운영서 §5](operation_guide.md#5-장애-대응) |
| `shutdown`이 `WorkerFailed`/`Unresponsive` | worker 중단, 또는 기한이 지나도 진행 없음 | 기록이 다 남았다고 보지 말고 `recover`와 `loss`로 확인 |
| `set_channel`이 `ConfigError::Busy` | 쉬고 있는 writer가 이전 revision을 아직 적용하지 않음 | 그 writer 스레드에서 `writer.refresh()` 호출 후 재시도 |
| `config apply`가 `revision_conflict`/`revision_busy` | 기준 revision이 낡음 / 이전 변경 배출 중 | `status`로 revision을 읽고 재시도 / 잠시 뒤 재시도 |
| `config apply`가 `restart_required` | 메모리 변경 | 호스트 재시작 |
| Lichtblick 재생 범위가 1970년부터, 미리 읽기 실패 | 이전 `standard/2` 파일의 등록 전용 chunk | `recover`로 호환 사본을 만들어 연다 |
| CLI가 `posted_host_io: completion_unconfirmed` | 취소·기한 초과로 호스트 작업 결과를 모름 | `status`나 `completion`으로 나중에 확인. 재실행으로 추측하지 않는다 |

---

## 6. 용어 설명

> 상세 용어 정의는 [도메인 정의서](../02_project_definition/domain_glossary.md)를 참조한다.

| 용어 | 설명 |
| --- | --- |
| default path / advanced path | `logger` 모듈의 기본 경로 / 나머지 모듈을 직접 조립하는 고급 경로 |
| writer | 실시간 스레드가 소유하는 기록 핸들. `try_log`로 기록을 넘긴다 |
| Important / Normal | 포화 보호와 샘플링 대상을 가르는 중요도. Important는 예약 slot을 쓰고 샘플링되지 않는다 |
| revision | 설정 버전. 변경 시 증가하고, 경계에서 파일이 회전한다 |
| event window | `mark_event` 전후로 보호하는 보존 구간 |
| Gap | 구독 중 overflow로 전달되지 못한 전송 구간 |
| Observed / Stale / Unavailable | 지표 상태: 관측됨 / 오래됨 / 관측 불가 |
| receipt | 기록 하나의 원래 식별(세션, producer, sequence 등) |
| manifest | 오프라인 파일 집합을 설명하는 `.logger/session.json` |

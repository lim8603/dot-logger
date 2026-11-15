# dot-logger

로보틱스 프로젝트에 내장하는 Rust 로깅·텔레메트리 라이브러리입니다. 실시간(RT) 스레드는
대기·할당 없이 기록을 넘기고, 기본 worker가 MCAP 파일로 회전·동기화·보존합니다.
메모리와 손실 한도는 모두 유한하게 선언합니다.

## 빠른 시작

`dot_logger::logger`(facade)가 기본 경로입니다. 아래는 L1 레시피
[`examples/recipes/l1_file.rs`](examples/recipes/l1_file.rs) 전문입니다(검사 스크립트가 원본과 같은지 확인합니다).

<!-- recipe:l1_file.rs -->
```rust
//! L1 recipe (NFR-008): record typed messages from a real-time thread into
//! files and judge the shutdown. No support module, `unsafe` or lifetimes.
use dot_logger::logger::{EncodeError, Logger, Preset, Record, Retention, Schema};
use std::time::Duration;

/// Your message type and its encoding (not counted as integration code).
struct Pose {
    x: f32,
    y: f32,
}
impl Record for Pose {
    const SCHEMA: Schema = Schema::new(0x504f_5345, 1, "le-f32x2");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        let out = out.get_mut(..8).ok_or(EncodeError)?;
        out[..4].copy_from_slice(&self.x.to_le_bytes());
        out[4..].copy_from_slice(&self.y.to_le_bytes());
        Ok(8)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::builder()
        .preset(Preset::control(1000)) // 1 kHz control loop: fills memory, sync and drop limits
        .file_output(std::env::temp_dir(), Retention::ring(64 << 20)) // an existing directory
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()?;
    let pose = logger.channel::<Pose>("robot/pose")?;
    let mut writer = logger.take_writer("control")?;
    let rt = std::thread::spawn(move || {
        for x in (0..1000).map(|i| i as f32) {
            // Never waits or allocates; a full queue is a counted rejection.
            let _ = writer.try_log(pose, &Pose { x, y: 0.0 });
        }
    });
    rt.join().map_err(|_| "control thread panicked")?;
    match logger.shutdown(Duration::from_secs(5))? {
        shutdown if shutdown.is_success() => Ok(()),
        shutdown => Err(format!("records not all synchronized: {shutdown:?}").into()),
    }
}
```
<!-- /recipe -->

```sh
cargo run --locked -p dot-logger --example recipe_l1_file
```

- `Preset::control(1000)`이 메모리·sync 주기·종료 기한·writer 할당량을 채웁니다. 직접 정하려면 `Budget`을 씁니다.
- `try_log`는 기다리지 않습니다. 큐가 차면 거절되고 그 수가 집계됩니다.
- `shutdown(..).is_success()`는 모든 기록이 기한 안에 쓰이고 동기화됐을 때만 true입니다.

## 레시피

| 레시피 | 내용 | NFR-008 한도 | 현재 |
|---|---|---|---|
| [L1 `l1_file.rs`](examples/recipes/l1_file.rs) | 사용자 타입 기록 → RT 스레드 기록 → 종료 판정 | ≤ 20줄·개념 ≤ 6 | 20줄·6 |
| [L2 `l2_rt_writers.rs`](examples/recipes/l2_rt_writers.rs) | RT writer 2개, raw loan으로 무복사 프레임 기록 | ≤ 40줄·≤ 10 | 40줄·7 |
| [L3 `l3_tracing.rs`](examples/recipes/l3_tracing.rs) | L1 + `tracing`/`log` 연결 | L1 대비 +6줄 이하 | +3 |
| [L3 `l3_remote.rs`](examples/recipes/l3_remote.rs) | L1 + loopback gRPC 서버(`dot-logger tail`로 구독) | L1 대비 +6줄 이하 | +5 |

```sh
cargo run --locked -p dot-logger --example recipe_l2_rt_writers
cargo run --locked -p dot-logger --features tracing-adapter --example recipe_l3_tracing
cargo run --locked -p dot-logger --features remote --example recipe_l3_remote
```

줄은 rustfmt 결과에서 `use`·주석·빈 줄·사용자 메시지 타입과 `Record` 구현을 빼고 셉니다. 개념은
사용자 코드가 이름으로 쓰는 `dot_logger` 타입·trait·함수 수입니다(계약 [API §18.6·§20](.cowork/03_design_artifacts/interface_contract.md#20-task-029-레시피접점-회귀-가드--2026-11-12)).

## 선택 옵션: 중요도·최소 level·이벤트 보존

기본 경로에 필요할 때만 붙이는 옵션입니다. 계약은 [API §22](.cowork/03_design_artifacts/interface_contract.md#22-task-034-facade에-core-기능-노출--계약안-2026-11-15)에 있습니다.

```rust
let logger = Logger::builder()
    .budget(budget)
    .file_output("/data/logs", Retention::ring(4 << 30)
        .events(EventWindow::new(Duration::from_secs(30), Duration::from_secs(10))
            .hold(Duration::from_secs(3600))
            .limits(8, 512 << 20)))                       // 이벤트 전후 구간 보존
    .channel_with::<Status>("robot/status", ChannelOptions::new()
        .minimum(Level::Info)                             // Info 미만은 Skipped
        .important_from(Level::Warn))                     // Warn 이상은 중요 기록
    .writer_with("control", WriterQuota::small(64, 256).important(8)) // 중요 기록 예약
    .start()?;
writer.try_log(alarm.important(), &msg)?;                 // 기록 하나만 중요로 표시
let mark = logger.mark_event()?;                          // 지금 시각 기준 [-30 s, +10 s] 보호
```

- **중요 기록:** writer 예약(`WriterQuota::important`)이 있을 때만 포화에서 보호됩니다. 예약 없이 `important_from`을 쓰면 `plan()`이 `PlanWarning::NoImportantReserve`를 보고합니다.
- **최소 level:** 시작할 때 정합니다. 실행 중 변경은 저장 계층의 revision 회전 설계(TASK-030 S2) 뒤에 추가됩니다.
- **이벤트 보존:**
  - `mark_event`는 일반 thread에서 부릅니다. RT thread는 그 thread에 알리면 됩니다.
  - `hold`가 끝나거나 `release_event`를 부르면 보호가 풀립니다.

## 두 계층과 문서

- **기본 경로 — `logger`:** builder, 기본 worker, 스레드 간 이동 가능한 `'static` writer, preset·장비 측정 프로필(`calibrate` feature), health·종료 보고.
- **고급 경로 — 나머지 모듈:** 메모리 분할(`config`), 수집(`capture`), 분배(`routing`), 완료 추적(`completion`), sync·종료·UPS 긴급 저장(`control`), MCAP 파일·보존·복구(`storage`), 조회·재생(`query`·`replay`), 어댑터(`adapters`: gRPC·InfluxDB·tracing), 운영 계측·리포트(`telemetry`), 점검(`diagnostics`).

계약 상세는 각 모듈의 rustdoc에 있습니다.

```sh
cargo doc --open -p dot-logger --all-features
```

## Feature

기본 feature는 없습니다.

| Feature | 추가 내용 |
|---|---|
| `remote` | gRPC 서버·클라이언트(tonic·Tokio) |
| `influxdb` | InfluxDB v2 유한 write·수치 조회(reqwest·Tokio) |
| `tracing-adapter` | `tracing`·`log` 입력 |
| `compression-zstd` | 의존성의 MCAP zstd(제품 writer는 무압축) |
| `calibrate` | 장비 측정과 preset용 JSON 프로필 |

## 다른 예제와 도구

| 예제 | 내용 |
|---|---|
| [`dot-logger-cli`](crates/dot-logger-cli) | `dot-logger` CLI(`status`·`query`·`replay`·`tail`·`recover`·`calibrate`·`export --standard` 등) |
| [`dot_probe_facade`](examples/dot_probe_facade.rs) | dot-probe 형태 통합(로그·1 kHz 관절·30 Hz 프레임 → 파일 + gRPC)을 facade만으로, support 없음 |
| [`dot_probe_*`](crates/dot-logger-client/examples) | 고급 경로 통합 예제(InfluxDB 경로·설정 변경 비교 포함), [client README](crates/dot-logger-client/README.md) |
| [`general_logs`](examples/general_logs.rs) | 고급 경로의 tracing/log 입력 |
| [`ups_simulation`](crates/dot-logger/examples/ups_simulation.rs) | UPS 알람·차단 기한 시뮬레이션(`--plan`, `--run DIR`) |
| [`tests/api-consumer`](tests/api-consumer/src/main.rs) | 독립 crate에서의 공개 API 사용 |

## 개발

개발 도구체인은 Rust 1.96.0(`rust-toolchain.toml`), 최소 지원 버전(MSRV)은 1.85입니다. `Cargo.lock`으로 의존성을 고정합니다.

```sh
cargo test --workspace --all-features --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
python3 scripts/check-feature-matrix.py --toolchain 1.85.0
python3 scripts/check-integration-surface.py   # facade 표면 snapshot·레시피 한도(nightly-2026-10-26 필요)
cargo run --locked -p dot-logger-api-consumer
```

`check-integration-surface.py`가 facade 표면 변경을 보고하면 내용을 검토한 뒤 `--bless`로 snapshot을 갱신하고, 같은 커밋에 포함해 리뷰합니다.

## 검증 범위

CI는 Ubuntu 24.04, Windows Server 2025, macOS 14에서 두 Rust 버전과 feature 조합을 검사하도록 구성되어 있습니다. 다만 원격 실행 결과가 나오기 전에는 해당 OS 통과로 판정하지 않습니다. 성능·RT·UPS는 현재 PC 실측과 시뮬레이션으로 검증했습니다(ADR-011). 실제 dot-probe 적용, 장기 시험, 다른 OS의 native 검증은 후속 작업입니다. 보안 검사는 MCAP 0.26.0(ADR-010)과 RustSec 기준이며, 간접 의존성 `paste`의 유지보수 중단 경고 1건을 추적하고 있습니다.

프로젝트 상태·설계·검증 근거는 [`.cowork/06_evolution/project_state.md`](.cowork/06_evolution/project_state.md)를 따릅니다.

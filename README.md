# dot-logger

로보틱스 프로젝트에 내장할 Rust 로깅·텔레메트리 라이브러리입니다.
현재 TASK-002의 시작 전 구성 검증 API를 구현했습니다. 수집·저장·조회 런타임은 후속 작업입니다.

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

메모리 수치는 애플리케이션이 명시합니다. 기본 예산이나 장비 성능을 가정하지 않습니다.
풀 payload는 `slots × slot_bytes`로 계산하고, Important 예약은 slots와 bytes를 함께 보호합니다.
큐·슬롯 메타데이터·인덱스·완료 추적·allocator 비용은 각 구현 작업에서 해당 예산에 추가합니다.
현재 검증을 실제 할당량·전체 프로세스 메모리 상한의 증명으로 해석하지 않습니다.

기본 feature는 비어 있으며 네트워크 런타임과 압축은 선택 의존성입니다.

| Feature | 의존성/계획 경로 |
|---|---|
| `remote` | tonic·Tokio, gRPC 어댑터 |
| `influxdb` | reqwest·Tokio, InfluxDB v2 어댑터 |
| `tracing-adapter` | tracing·log, 일반 로그 어댑터 |
| `compression-zstd` | MCAP의 zstd 압축 |

선택 feature는 현재 의존성 호환 검사 단계이며 해당 제품 기능의 구현 완료를 뜻하지 않습니다.
MCAP 왕복 검사는 의존성 API 확인용입니다. dot-logger의 파일 API는 TASK-007에서 구현합니다.
플랫폼 상태는 어댑터가 제공하며 기본값은 `Unavailable`입니다.
`Supported`는 기능 탐색 결과이고, 실제 적용 성공이나 실시간 성능 보장이 아닙니다.

CI는 Ubuntu 24.04, Windows Server 2025, macOS 14에서 두 Rust 버전과 16개 feature 조합을 검사하도록 구성했습니다.
Windows 11과 실제 PREEMPT_RT/SSD/UPS 검증은 별도입니다. 원격 CI 실행 전에는 해당 OS 통과로 판정하지 않습니다.
MCAP 0.26.0은 압축 의존성 보안 수정에 따라 승인됐습니다(ADR-010).
최종 RustSec 검사에서 취약점·안전성 경고는 없고, 간접 의존성 `paste`의 유지보수 중단 경고 1건은 남습니다.
보안 CI는 취약점과 안전성 경고에서 실패하며 유지보수 경고도 출력합니다.
프로젝트 상태·설계·검증 근거는 [`.cowork/06_evolution/project_state.md`](.cowork/06_evolution/project_state.md)를 따릅니다.

# dot-logger

로보틱스 프로젝트에 내장할 Rust 로깅·텔레메트리 라이브러리입니다.
현재 TASK-007까지 구성 검증, 소형/원본 수집, 유한 분배, 완료 추적·제어와
Protobuf 봉투·MCAP 파일 writer·chunk 인덱스를 구현했습니다.
로컬 실제 파일 쓰기와 명시 sync를 검증했으며, worker 기동·회전/복구·조회/재생·실장비 검증은 후속 작업입니다.

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
| `influxdb` | reqwest·Tokio, InfluxDB v2 어댑터 |
| `tracing-adapter` | tracing·log, 일반 로그 어댑터 |
| `compression-zstd` | MCAP의 zstd 압축 |

선택 feature는 현재 의존성 호환 검사 단계이며 해당 제품 기능의 구현 완료를 뜻하지 않습니다.
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

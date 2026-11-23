# REV-072 — REV-071 이후 코드 변경(`ae22f9d..f30ae5a`) 및 TASK-024 S2 장애 주입 설계 독립 코드리뷰

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-072 / 2026-11-23 |
| 리뷰어 | Antigravity AI — 리뷰 전용 세션 |
| 대상 | REV-071 마감(`ae22f9d`) 이후 코드 변경 `ae22f9d..f30ae5a`(33개 파일: `d9d8b1f`, `378c6ba`, `a0fcfcf`, `9d8bdd3`, `fe6f705`, `4ee4996`, `9f687dd`, `ad11ba3`, `2f6e356`, `84d247b`, `f30ae5a` 등) 및 [TASK-024 §S2 설계](../../04_implementation/tasks/TASK-024.md#s2-설계--2026-11-23) (`HEAD` `b02671f`) |
| 관련 Task / ADR | [TASK-024](../../04_implementation/tasks/TASK-024.md), [TASK-036](../../04_implementation/tasks/TASK-036.md), [TASK-038](../../04_implementation/tasks/TASK-038.md), [TASK-039](../../04_implementation/tasks/TASK-039.md), [ADR-009 개정 1·2](../../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md), [ADR-010 개정 2](../../03_design_artifacts/adrs/ADR-010_mcap_security_update.md) |
| 종합 판정 | **Conditional** — 제품 코드 `ae22f9d..f30ae5a`는 런타임 P1 결함 없이 건전하나, **TASK-024 S2 설계에 코드 계약과 어긋나는 P1 1건(3항목)**과 플랫폼 계약·CI 커버리지·하네스 이식성·외부 소비자 검증의 **P2 5건 · P3 3건**(`RV-193~201`) 보완 필요 |
| 상태 | **Open** |

REV-071 마감(`ae22f9d`) 이후 v0.1.0 배포 가능화, macOS std `Mutex` 예열, Windows/macOS 파일 출력 지원(ADR-009 개정 2), 24h soak/OS 간 교환 하네스, calibrate 간헐 실패 조치, `url 2.5.9` 보안 갱신 및 TASK-024 S2 장애 주입 설계를 5개 중점 관점에서 독립 검토했다.

주요 확인 결과는 다음과 같다.
1. **핵심 결함 (P1 1건 — S2 착수 전 설계 정렬 필수):**
   - **`RV-193` (`TASK-024.md` §S2 설계):** ① `F3a`(probe 공간 부족)의 기대 손실 원인이 `Cause::Capacity`로 적혀 있으나 실제 코드는 `Scope::OutputFailed / Cause::Adapter`를 기록하며, `reserve 0`·링 256 MiB·tmpfs 512m에서는 외부 채움 ≥ 480 MiB가 아니면 `Capacity`에 도달하지 않고, 회복 뒤에도 `shutdown.is_success()`는 `false`다. ② `F2`(느린 구독자 2분 `SIGSTOP`)는 `facade` 원격 `max_age = 10 s`에 의해 `Gap` 정책에서도 10초 뒤 서버가 스트림을 종료(`Gone`)하므로 `SIGCONT` 뒤 동일 스트림 연속 수신이 성립하지 않는다. ③ 공통 Pass `"실패 기록 수 = 손실 보고 합계"`는 `F3b`(ENOSPC `IoPoisoned` 뒤 checkpoint 기록 불가, RT admission은 미폐쇄)와 `F5`(SIGKILL 뒤 RAM 미기록 유실, `flush_chunk` 기준 잔류, 마지막 완료 sync 시퀀스 오라클 부재)에 그대로 적용할 수 없다.
2. **주요 보완 항목 (P2 5건, P3 3건):**
   - **중점 1(플랫폼 의미):** Windows `FileStore::directory_sync_supported() -> true`와 `sync_dir` no-op의 trait 계약 불일치 및 NTFS 저널링 내구성 한계(비-NTFS·ring 삭제 부활)(`RV-194`), `diagnostics::available_bytes`의 Windows 없는 경로 `Ok` 반환·quota·260자 경로 차이 미문서화 및 테스트명 모순(`RV-195`), ADR-009 개정 2 이후에도 기존 facade·소비자 테스트 11곳이 `#[cfg(target_os = "linux")]`로 잠겨 macOS/Windows CI에서 스킵되는 공백(`RV-196`).
   - **중점 3(하네스 `unsafe`):** `soak.rs` glibc 전용 FFI(`mallinfo2`, `malloc_trim`)의 `target_env = "gnu"` 가드 누락(`RV-197`).
   - **중점 4(배포·공급망):** `check-external-consumer.py`의 Linux 전용 실행(Windows `fs4` 미검증) 및 `edition = "2021"` 소비자의 MSRV 1.85 `resolver = "3"` 안내 부재(`RV-198`).
   - **기타 개선 (P3 3건):** Apple `MUTEX_BYTES` 예약 회계 미명시 영역(`RV-199`), `tc_013_calibrate.rs` 뺄셈 언더플로 방어 여지(`RV-200`), `clean-build-cache.sh` 인자 검증·게시 전 내부 `version` 준비(`RV-201`).
3. **확인된 건전성 불변식 및 정적 검증 결과:**
   - **RT 무할당(중점 2):** `crates/dot-logger/src/` 내 `std::sync::Mutex` 9곳 전부 `crate::sync::warm_mutex` 적용 확인. RT 진입 경로에 `Mutex`/`OnceLock`/`thread_local!`/할당 0건. EV-120의 초당 ~13,100회 할당은 **제어·프레임 RT 스레드(0회/s)가 아니라** 일반 경로인 **호스트 스레드 `routing::take_shared`(~10,010회/s, 76%)**와 **IO 스레드 `write_batch` + 배치당 `mcap::ChunkWriter` flush(~3,100회/s, 24%)**에서 발생함을 정적으로 규명했다.
   - **macOS `F_FULLFSYNC`:** 제품 저장 경로가 모두 `std::fs::File::sync_all()`(Rust std 내부에서 Apple 대상 `fcntl(F_FULLFSYNC)` 호출)을 사용하므로 별도 FFI 불필요.
   - **하네스 `unsafe`:** `soak.rs`의 `GlobalAlloc` 위임·무패닉 감산·`MallInfo2` 10×`usize` 배치(`/usr/include/malloc.h` 대조) 건전.
   - **배포·공급망:** `vendor/mcap-0.26.0/local-changes.patch`(209줄)와 캐시 상류 `mcap 0.26.0` 대비 `src/` diff가 100% 일치, `fs4` `default-features = false` 최소 의존성 확인.
   - **로컬 정적 분석·단위 검증:** `cargo +1.96.0 fmt --all --check` Pass, `1.96.0`/`1.85.0` `clippy --workspace --all-targets --all-features --locked -- -D warnings` Pass, `sync::`·`diagnostics::host::`(3 Pass), `os_exchange`(2 Pass), `soak`(2 Pass), `tc_013_calibrate`(11 Pass), `dot-logger-rt-allocation`(19 Pass), `cargo audit --deny unsound` Pass(허용된 `paste` 경고 1건).

리뷰 전용 세션 원칙([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md))에 따라 제품 코드·테스트·스크립트·CI 설정은 일체 수정하지 않았다.

---

## 2. 파일 변경표

### 이번 리뷰 세션 산출물 (`.cowork/` 전용)

| 구분 | 파일 | 변경 내용 |
|---|---|---|
| 생성 | [REV-072_POST_GATE4_PLATFORM_SOAK_S2_REVIEW.md](REV-072_POST_GATE4_PLATFORM_SOAK_S2_REVIEW.md) | REV-071 이후 코드 변경(`ae22f9d..f30ae5a`) 및 TASK-024 S2 설계 독립 리뷰 결과(`RV-193~201`) 작성 (`Open`, `Conditional`) |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | 미처리 리뷰·코드리뷰 현황(`Open`, P1 1·P2 5·P3 3)·다음 시작점에 `REV-072` 등록 |
| 수정 | [my_state.md](../../members/lim8603/workspace/my_state.md) | 작업자 상태 리뷰·세션 로그·다음 시작점 동기화 |
| 생성 | session_2026-11-23_001.md(비공개 세션 로그: `session_logs/session_2026-11-23_001.md`) | `REV-072` 코드리뷰 전용 세션 수행 로그 기록 |

### 주요 검토 대상 파일 (Read-Only 검토, 제품 코드 수정 0건)

- **플랫폼·동기화·진단 (`crates/dot-logger/src/`):** [sync.rs](../../../crates/dot-logger/src/sync.rs), [diagnostics/host.rs](../../../crates/dot-logger/src/diagnostics/host.rs), [diagnostics/mod.rs](../../../crates/dot-logger/src/diagnostics/mod.rs), [storage/retention.rs](../../../crates/dot-logger/src/storage/retention.rs), [storage/mod.rs](../../../crates/dot-logger/src/storage/mod.rs), [storage/writer.rs](../../../crates/dot-logger/src/storage/writer.rs), [calibrate.rs](../../../crates/dot-logger/src/calibrate.rs), [platform.rs](../../../crates/dot-logger/src/platform.rs)
- **RT·호스트·라우팅·손실·원격 (`crates/dot-logger/src/`):** [logger/host.rs](../../../crates/dot-logger/src/logger/host.rs), [logger/writer.rs](../../../crates/dot-logger/src/logger/writer.rs), [logger/health.rs](../../../crates/dot-logger/src/logger/health.rs), [logger/mod.rs](../../../crates/dot-logger/src/logger/mod.rs), [logger/budget.rs](../../../crates/dot-logger/src/logger/budget.rs), [logger/preset.rs](../../../crates/dot-logger/src/logger/preset.rs), [logger/remote.rs](../../../crates/dot-logger/src/logger/remote.rs), [logger/sink.rs](../../../crates/dot-logger/src/logger/sink.rs), [capture/mod.rs](../../../crates/dot-logger/src/capture/mod.rs), [routing/mod.rs](../../../crates/dot-logger/src/routing/mod.rs), [completion/mod.rs](../../../crates/dot-logger/src/completion/mod.rs), [control/config_update.rs](../../../crates/dot-logger/src/control/config_update.rs), [loss.rs](../../../crates/dot-logger/src/loss.rs), [telemetry/counters.rs](../../../crates/dot-logger/src/telemetry/counters.rs), [adapters/remote/hub.rs](../../../crates/dot-logger/src/adapters/remote/hub.rs), [adapters/remote/server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs), [adapters/remote/receiver.rs](../../../crates/dot-logger/src/adapters/remote/receiver.rs), [adapters/remote/types.rs](../../../crates/dot-logger/src/adapters/remote/types.rs)
- **하네스·테스트·스크립트·매니페스트:** [examples/soak.rs](../../../crates/dot-logger/examples/soak.rs), [examples/os_exchange.rs](../../../crates/dot-logger/examples/os_exchange.rs), [tc_013_calibrate.rs](../../../crates/dot-logger/tests/tc_013_calibrate.rs), [tc_008_influx_query.rs](../../../crates/dot-logger/tests/tc_008_influx_query.rs), [tc_008_influx_routed.rs](../../../crates/dot-logger/tests/tc_008_influx_routed.rs), [tc_008_influxdb.rs](../../../crates/dot-logger/tests/tc_008_influxdb.rs), [tests/rt-allocation/src/tests.rs](../../../tests/rt-allocation/src/tests.rs), [tests/api-consumer/src/main.rs](../../../tests/api-consumer/src/main.rs), [Cargo.toml](../../../Cargo.toml), [Cargo.lock](../../../Cargo.lock), [crates/dot-logger/Cargo.toml](../../../crates/dot-logger/Cargo.toml), [crates/dot-logger-viewer/Cargo.toml](../../../crates/dot-logger-viewer/Cargo.toml), [vendor/mcap-0.26.0/Cargo.toml](../../../vendor/mcap-0.26.0/Cargo.toml), [vendor/mcap-0.26.0/VENDORED.md](../../../vendor/mcap-0.26.0/VENDORED.md), [vendor/mcap-0.26.0/local-changes.patch](../../../vendor/mcap-0.26.0/local-changes.patch), [scripts/check-external-consumer.py](../../../scripts/check-external-consumer.py), [scripts/clean-build-cache.sh](../../../scripts/clean-build-cache.sh), [.github/workflows/ci.yml](../../../.github/workflows/ci.yml), [CHANGELOG.md](../../../CHANGELOG.md), [README.md](../../../README.md)
- **설계·기준 문서:** [TASK-024.md](../../04_implementation/tasks/TASK-024.md), [ADR-009](../../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md), [ADR-010](../../03_design_artifacts/adrs/ADR-010_mcap_security_update.md)

---

## 3. 클래스(타입) 변경표

**이번 리뷰 세션의 제품 코드 변경은 없음.** 검토 대상 핵심 타입(`struct` · `enum` · `trait`)은 아래와 같다.

| 구분 | 소속 파일 | 타입 이름 | 검토 관점 및 주요 발견 |
|---|---|---|---|
| 검토 | [storage/retention.rs](../../../crates/dot-logger/src/storage/retention.rs) | `SegmentStore`, `FileStore`, `SegmentLog`, `RetentionError` | Windows `directory_sync_supported()`/`sync_dir` 계약과 NTFS 저널링 의미(`RV-194`), `room()` ring 삭제 및 `Capacity` 경로(`RV-193`) |
| 검토 | [diagnostics/host.rs](../../../crates/dot-logger/src/diagnostics/host.rs) | `available_bytes`, `linux_available_bytes` | Unix `statvfs` vs Windows `fs4::available_space` 없는 경로·quota·버퍼 차이(`RV-195`) |
| 검토 | [sync.rs](../../../crates/dot-logger/src/sync.rs), [storage/mod.rs](../../../crates/dot-logger/src/storage/mod.rs), [telemetry/counters.rs](../../../crates/dot-logger/src/telemetry/counters.rs), [control/config_update.rs](../../../crates/dot-logger/src/control/config_update.rs) | `warm_mutex`, `MUTEX_BYTES`, `FileLedger`, `Bank`, `State` | core std `Mutex` 9곳 전수 예열 확인, Apple `MUTEX_BYTES` 회계 범위(`RV-199`) |
| 검토 | [logger/host.rs](../../../crates/dot-logger/src/logger/host.rs), [logger/health.rs](../../../crates/dot-logger/src/logger/health.rs), [routing/mod.rs](../../../crates/dot-logger/src/routing/mod.rs), [completion/mod.rs](../../../crates/dot-logger/src/completion/mod.rs), [loss.rs](../../../crates/dot-logger/src/loss.rs) | `Host`, `IoTarget`, `HealthState`, `OutputStatus`, `Dispatcher`, `SharedPayload`, `Cause`, `Scope` | EV-120 초당 13,100회 할당 정적 귀속(호스트·IO 스레드), S2 F3a/F3b/F5 실제 상태 전이·손실 원인 대조(`RV-193`) |
| 검토 | [adapters/remote/hub.rs](../../../crates/dot-logger/src/adapters/remote/hub.rs), [adapters/remote/server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs), [logger/remote.rs](../../../crates/dot-logger/src/logger/remote.rs) | `RemoteHub`, `Subscription`, `SubscriptionStream`, `RemoteLimits` | `max_age = 10 s` 만료 시 `Gap` 정책 포함 `s.closed = true` 종료(`RV-193`) |
| 검토 | [examples/soak.rs](../../../crates/dot-logger/examples/soak.rs), [examples/os_exchange.rs](../../../crates/dot-logger/examples/os_exchange.rs) | `Counting`, `MallInfo2`, `Stats`, `Found`, `Fixed` | 하네스 `GlobalAlloc` 안전성 및 glibc FFI의 `target_env = "gnu"` 가드 누락(`RV-197`) |

---

## 4. 리뷰 포인트 — 중점별 검토 매트릭스 및 지적 목록 (`RV-193~201`)

### A. 5대 중점별 검토 결과 매트릭스

| 중점 | 직접 확인한 건전성 사실 | 신규 지적 (`RV-193~201`) |
|---|---|---|
| **중점 1. 플랫폼 의미 (ADR-009 개정 2)** | macOS `File::sync_all()`은 Rust std 내부에서 `fcntl(F_FULLFSYNC)` 사용(별도 FFI 불필요); `FileStore::finalize`는 `drop(sink)` 이후 `try_exists` + `rename` 순서 준수; `os_exchange` 9/9 OS 교차 판독 Pass(EV-119) | `RV-194` (P2), `RV-195` (P2), `RV-196` (P2) |
| **중점 2. RT 무할당 불변식 & 13,100 alloc/s** | core `std::sync::Mutex` 9곳 전부 `warm_mutex` 적용; RT 진입 경로(`Writer::try_log`/`try_log_at`/`refresh`/`Drop`, `Producer::*`, `Loan::drop`)에 `Mutex`·`OnceLock`·`thread_local!`·할당 0건; `dot-logger-rt-allocation` 19/19 Pass; EV-120 초당 13,127회 할당은 **RT 스레드 0회**, **호스트 `routing::take_shared` ~10,010회/s(76%)** + **IO `write_batch`/`mcap::ChunkWriter` ~3,100회/s(24%)** | `RV-199` (P3) |
| **중점 3. 하네스 `unsafe` (`soak.rs`)** | `Counting` `GlobalAlloc` 위임·null 검사·`wrapping_sub` 무패닉·`realloc` 증감 회계 건전; `MallInfo2` 10×`usize` 레이아웃이 `/usr/include/malloc.h`와 완전 일치; `os_exchange` 및 제품 코드에 신규 `unsafe` 0건 | `RV-197` (P2), `RV-200` (P3) |
| **중점 4. 배포·공급망** | `vendor/mcap-0.26.0/local-changes.patch` 209줄이 상류 `mcap 0.26.0` 대비 `src/`와 100% 일치; `fs4` `default-features = false`로 Windows 전용 최소 의존(`windows-sys 0.61.2` 재사용); `url 2.5.9`로 `RUSTSEC-2026-0592` 해소(`cargo audit --deny unsound` Pass); `Cargo.toml` `^` 하한 정렬 | `RV-198` (P2), `RV-201` (P3) |
| **중점 5. TASK-024 S2 설계 검토** | 전제 1·3·4·5·6의 기본 구조 및 Docker cgroup v2/tmpfs 환경 방향(D1=D) 타당; D1=D, D2=A, D3=A 방향 합리적 | `RV-193` (P1 — F3a·F2·F3b/F5 3항목) |

---

### B. 지적 항목 요약표

| ID | 중요도 | 중점 | 위치 | 문제 요약 | 판정 | 상태 |
|---|---|---|---|---|---|---|
| **RV-193** | **P1** | 중점 5 | `TASK-024.md:108-115`, `logger/host.rs:1069-1077,1144-1147`, `completion/mod.rs:1074`, `adapters/remote/hub.rs:185-198` | S2 설계와 실제 코드 계약 불일치 3건: ① F3a는 `Cause::Capacity`가 아닌 `OutputFailed/Cause::Adapter`이며 링 임계·`is_success()==false` 반영 필요 ② F2 2분 `SIGSTOP`은 `max_age=10s`로 `Gap` 정책도 서버 종료 ③ 공통 Pass `"실패 기록 수 = 손실 보고 합계"` 및 F3b/F5 오라클 불일치 | 미정 | Open |
| **RV-194** | **P2** | 중점 1 | `storage/retention.rs:15-24,45-50,91-93`, `CHANGELOG.md:14`, `ADR-009:57` | Windows `FileStore::directory_sync_supported()`가 `true`를 반환하면서 `sync_dir`가 no-op `Ok(())` — trait 계약과 불일치하며, NTFS 저널링은 일관성(atomicity)만 보장하고 지연 flush·링 삭제 부활·비-NTFS(FAT32/exFAT/SMB) 한계가 있음 | 미정 | Open |
| **RV-195** | **P2** | 중점 1 | `diagnostics/host.rs:366-419` | `available_bytes`가 Windows(`fs4::available_space`→`GetVolumePathNameW`)에서는 없는 경로에도 `Ok(볼륨 여유)`를 반환(Unix는 `ENOENT`)하고 quota·260자 경로 버퍼 차이가 있으나 rustdoc 미기재·테스트명 모순 | 미정 | Open |
| **RV-196** | **P2** | 중점 1 | `logger/mod.rs:49`, `logger/writer.rs:393`, `logger/host.rs:1190`, `tc_013_*.rs`, `tc_014_standard.rs`, `facade_remote.rs`, `api-consumer/src/main.rs:320`, `rt-allocation/src/tests.rs:1554` | ADR-009 개정 2로 macOS/Windows 파일 출력이 가능해졌는데도 기존 `RV-172`의 `#[cfg(target_os = "linux")]` 가드 11곳이 남아 macOS/Windows CI에서 facade 통합 테스트가 실행되지 않음 | 미정 | Open |
| **RV-197** | **P2** | 중점 3 | `crates/dot-logger/examples/soak.rs:27,90-125` | glibc 전용 FFI(`mallinfo2`, `malloc_trim`)가 `target_env = "gnu"` 없이 `target_os = "linux"`에 묶여 `*-linux-musl` 타깃 `--all-targets` 링크 실패 | 미정 | Open |
| **RV-198** | **P2** | 중점 4 | `.github/workflows/ci.yml:31-33`, `scripts/check-external-consumer.py:18-45`, `Cargo.toml:14-18` | 외부 소비자 무잠금 빌드가 `ubuntu-24.04`에서만 돌아 Windows `fs4`·macOS `rustix` 미검증, `edition = "2021"` 소비자의 MSRV 1.85 `resolver = "3"` 필요성 미안내 | 미정 | Open |
| **RV-199** | **P3** | 중점 2 | `sync.rs:14`, `adapters/remote/types.rs:59-80`, `receiver.rs:88-92`, `logger/budget.rs:424-450` | Apple `MUTEX_BYTES`(64 B)가 `RemoteHub`·`Reassembler`·`facade_bytes`에는 명시적 가산 대신 여유 상수에 암묵 흡수됨 | 미정 | Open |
| **RV-200** | **P3** | 중점 3 | `crates/dot-logger/tests/tc_013_calibrate.rs:132-145` | `f19_reserve_shrinks_the_write_limit`의 `free - 16 * MIB`, `seen - reserve` 단순 뺄셈이 극단적 저여유 환경에서 언더플로 패닉 가능 | 미정 | Open |
| **RV-201** | **P3** | 중점 4 | `scripts/clean-build-cache.sh:17,29`, `crates/dot-logger-{client,cli,viewer}/Cargo.toml` | `clean-build-cache.sh`의 `--threshold` 인자 검증 부족·GNU `df` 전용 표기 부재, crates.io 게시(MS-010 2단계) 전 내부 path 의존성 `version` 지정 필요 메모 | 미정 | Open |

---

### C. 개별 지적 상세 (`RV-193` ~ `RV-201`)

---

#### RV-193 — TASK-024 S2 장애 주입 설계와 실제 코드 계약 불일치 3건 (P1)

- **심각도 / 중점:** `P1` / 중점 5 (S2 설계 검토)
- **위치:**
  - [TASK-024.md:78-117](../../04_implementation/tasks/TASK-024.md#L78-L117) (`§S2 설계` 전제 2·3·5·6, 시나리오 F2·F3a·F3b·F5, 공통 Pass)
  - [logger/host.rs:905-935, 1069-1078, 1126-1151](../../../crates/dot-logger/src/logger/host.rs#L1069-L1151) (`failure`, `settle`)
  - [completion/mod.rs:1072-1080](../../../crates/dot-logger/src/completion/mod.rs#L1072-L1080), [loss.rs:365-366, 636](../../../crates/dot-logger/src/loss.rs#L365-L366)
  - [adapters/remote/hub.rs:185-208](../../../crates/dot-logger/src/adapters/remote/hub.rs#L185-L208), [logger/remote.rs:143-154](../../../crates/dot-logger/src/logger/remote.rs#L143-L154)
- **실패 시나리오 (설계 그대로 구현 시 오판정 경로):**
  1. **① F3a (probe 공간 부족):**
     - S2 표(`TASK-024.md:109`)는 `"손실 보고 Cause::Capacity"`를 기대하지만, `SegmentLog::room()`이 `RetentionError::Capacity`를 반환하면 `host.rs:1069-1077` `failure()`가 `FailureReason::AdapterFailure`를 반환하고, `completion/mod.rs:1074`·`loss.rs:366`이 이를 **`Scope::OutputFailed`, `Cause::Adapter`**로 기록한다(`Scope::OutputFailed`는 `Io | Sync | Adapter`만 허용). 하네스가 `Cause::Capacity`를 단정하면 실제 계약과 충돌해 실패한다.
     - 또한 `Retention::ring(256 MiB)`의 `segment`는 `32 MiB`(`budget.rs:763`)이고 기본 `reserve = 0`이다. 512 MiB tmpfs에 링 파일이 200 MiB 쌓인 상태에서 채움 파일을 300 MiB만 쓰면 `room()`이 닫힌 링 파일 일부만 지워 `free >= 32 MiB`를 회복해 `RetentionError::Capacity`가 전혀 발생하지 않는다. `Capacity` 실패를 확실히 유도하려면 `reserve`를 설정하거나 채움 파일이 `tmpfs - (reserve + 32 MiB)`를 초과해야 한다.
     - 회복 뒤에도 `OutputStatus.failed_records > 0`·`ShutdownReport.core.required.failed > 0`이므로 `Logger::shutdown`의 `shutdown.is_success()`는 **`false`**다(`soak.rs:551`의 기존 성공 조건과 다름).
  2. **② F2 (느린 구독자 2분 `SIGSTOP`):**
     - S2 표(`TASK-024.md:108`)는 `CLI tail`을 **2분 `SIGSTOP`** 후 `Gap`·`Disconnect` 각각에 대해 `"구독자 gap 이벤트/끊김, ... 재개 뒤 이어 받음"`을 기대한다.
     - 그러나 `logger/remote.rs:151`의 `max_age`는 **10초**이며, `Subscription::expire`(`hub.rs:185-198`)는 `OverflowPolicy::Gap`과 `Disconnect`를 구분하지 않고 미ACK 레코드가 10초를 넘기면 **`s.closed = true`**로 구독을 영구 종료한다(`SubscriptionStream`도 idle 10초 초과 시 `TimedOut`).
     - 따라서 2분 `SIGSTOP` 뒤 `SIGCONT`하면 `Gap` 정책에서도 동일 스트림은 이미 `Gone`으로 끊겨 이어받을 수 없다.
  3. **③ F3b (실제 ENOSPC) · F5 (SIGKILL)와 공통 Pass 불일치:**
     - **F3b:** 실제 ENOSPC로 `IoPoisoned`가 발생해도 `host.rs:1144-1147`은 `flags.worker_failed`를 세우지 않아 `core.control().close()`가 호출되지 않는다. RT `try_log`는 계속 `Accepted`를 반환하고 IO 스레드는 이후 모든 배치를 `RetentionError::Poisoned` → `IoFailure`로 실패시키지만, 이미 poisoned된 `SegmentLog`는 새 `LossReport` checkpoint를 디스크에 기록할 수 없다. 따라서 공통 Pass `"실패 기록 수 = 손실 보고 합계"`는 F3b에서 성립할 수 없다(메모리 `ShutdownReport.core.required.failed`·`OutputHealth.failed_records`로만 대조 가능).
     - **F5:** 경보 없는 SIGKILL은 OS 페이지 캐시를 버리지 않으므로 `.active` 파일에는 마지막 `sync`가 아니라 **마지막 `flush_chunk`** 지점까지 기록되며, 마지막 `flush_chunk` 이후 RAM 유실분은 파일 `LossReport`에 기록되지 않는다(공통 Pass `"실패 기록 수 = 손실 보고 합계"` 예외). 또한 현재 `soak.rs` 표본에는 마지막 완료 sync 직전의 수락 개수가 없어 `"마지막 sync 지점까지 복구"`를 검증할 하한 오라클이 부족하다.
- **확인 방법:** 코드 전수 대조(`logger/host.rs`, `completion/mod.rs`, `loss.rs`, `adapters/remote/hub.rs`, `storage/retention.rs`).
- **수정 제안:**
  - `TASK-024.md` §S2 설계를 하네스 착수 전에 아래처럼 정렬한다:
    1. **F3a:** 기대 손실 보고를 `Scope::OutputFailed, Cause::Adapter`로 정정하고, 채움 크기 계산식(`> tmpfs - reserve - segment` 또는 명시적 `.reserve(...)`)과 회복 후 판정(`failed_records` 증가 중지·이후 기록 전량 존재, `shutdown.is_success() == false` 허용)을 명시한다.
    2. **F2:** `Gap` 정책의 "동일 스트림 Gap 이벤트 후 연속 수신" 시험은 **`max_age`(10 s) 미만의 짧은 정지(예: 3~5 s) 또는 느린 소비 rate 제한**으로 분리하고, 2분 `SIGSTOP`은 `max_age` 만료(`Gone`) → **재접속 후 이어받기** 검증으로 구분한다.
    3. **F3b·F5 및 공통 Pass:** 공통 Pass `"실패 기록 수 = 손실 보고 합계"`의 적용 범위를 파일이 정상적으로 최종 checkpoint를 쓸 수 있는 시나리오(F1·F2·F3a·F4)로 명시하고, F3b는 메모리 `failed_records == ShutdownReport.core.required.failed` + `IoPoisoned` 고착으로, F5는 "표본/외부 동기화로 관측한 마지막 완료 sync 이전 수락 개수 ≤ 복구된 레코드 수 ≤ SIGKILL 시점 수락 개수, 중간 gap 0, 재시작 시 새 session 디렉터리 정상 생성"으로 오라클을 명확히 정의한다.

**[처리담당]**
- 판정 결과: (개발 세션 작성)
- 처리 내용: (개발 세션 작성)
- 검증 근거: (개발 세션 작성)

---

#### RV-194 — Windows `FileStore::directory_sync_supported() -> true` 및 `sync_dir` no-op의 trait 계약·NTFS 내구성 의미 불일치 (P2)

- **심각도 / 중점:** `P2` / 중점 1 (플랫폼 의미 — ADR-009 개정 2)
- **위치:**
  - [storage/retention.rs:15-24, 30-50, 91-93, 513-523, 765-772](../../../crates/dot-logger/src/storage/retention.rs#L15-L55) (`SegmentStore`, `FileStore`, `sync_dir`)
  - [CHANGELOG.md:11-15](../../../CHANGELOG.md#L11-L15)
  - [ADR-009 개정 2](../../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md#개정-2--macoswindows-파일-출력2026-11-22)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `SegmentStore` trait(`retention.rs:15-16, 23-24`)은 `directory_sync_supported()`가 "이 어댑터가 디렉터리 메타데이터 동기화를 구현했는지"를 보고하고, `sync_directory()`는 "미지원 시 에러를 반환해야 한다"고 선언한다(`RetentionError::UnsupportedDirectorySync`: `"never treated as success"`).
  2. 그러나 `9f687dd` 이후 Windows `FileStore`는 `sync_dir`가 아무 시스템 호출도 하지 않고 `Ok(())`를 반환하면서 `directory_sync_supported()`는 `true`를 반환한다.
  3. **NTFS 저널링의 실제 동작 범위:** NTFS `$LogFile` 메타데이터 저널링은 크래시 시 디렉터리 엔트리(`$I30`)의 **일관성/원자성(consistency)**(이름 변경이 전부 반영되거나 전혀 반영되지 않음)을 보장하지만, 볼륨/저널 flush 없이 호출 반환 시점의 **전원 소실 내구성(durability)**을 보장하지 않는다(NTFS는 저널을 지연 체크포인트한다).
     - `publish()`(`retention.rs:765-772`)는 `sink.sync_all()`(`FlushFileBuffers`)로 `.active` 데이터·MFT를 먼저 flush한 뒤 `rename`하므로, NTFS 로컬 볼륨에서 rename 저널이 미flush 상태로 정전돼도 `.active` 파일 본체가 남아 `recover_segment`로 복구 가능하다는 ADR-009 개정 2의 핵심 논리는 데이터 손실 방지 측면에서 성립한다.
     - 그러나 ① `room()`(`retention.rs:513-523`)의 ring 파일 삭제(`remove` + `sync_directory`)는 디렉터리 flush가 없으므로 정전 시 삭제된 파일이 되살아날 수 있고, ② 사용자가 Windows에서 **FAT32/exFAT(SD 카드·USB) 또는 SMB 네트워크 공유**를 출력 경로로 지정하면 NTFS 저널링 전제 자체가 성립하지 않는다.
     - 또한 `FileStore::directory_sync_supported()`를 조회하는 외부 고급 API 호출자는 Windows에서도 실제 디렉터리 동기화가 수행된다고 오해할 수 있다.
- **확인 방법:** 코드 대조(`storage/retention.rs`, `storage/writer.rs`, `diagnostics/file.rs`) 및 Windows `MoveFileExW`/`FlushFileBuffers`/NTFS `$LogFile` 의미 검토. (참고: macOS는 `std::fs::File::sync_all()`이 Rust std 내부에서 `fcntl(F_FULLFSYNC)`를 호출하므로 별도 FFI 불필요.)
- **수정 제안:**
  - `FileStore` rustdoc(`retention.rs:30-35`), `sync_dir` 주석, `ADR-009 개정 2`, `CHANGELOG.md`에서 Windows는 **"디렉터리 flush 미수행(Tier 2), 로컬 NTFS의 메타데이터 원자성(일관성)에만 의존하며 전원 소실 시 rename/unlink 지연 반영 및 비-NTFS(FAT32/exFAT/SMB) 미보장"**을 명시적으로 구분해 적는다.
  - `SegmentStore::directory_sync_supported()` 문서도 `FileStore` Windows 동작과 모순되지 않도록(예: "adapter meets platform directory metadata durability/consistency contract") 정렬하거나, 장기적으로 `FileStore` Windows 옵션/볼륨 FS 검사 필요 여부를 판단한다.

**[처리담당]**
- 판정 결과: (개발 세션 작성)
- 처리 내용: (개발 세션 작성)
- 검증 근거: (개발 세션 작성)

---

#### RV-195 — `diagnostics::available_bytes`의 Windows/Unix 의미 차이(없는 경로·quota·260자 버퍼) 미문서화 및 테스트명 모순 (P2)

- **심각도 / 중점:** `P2` / 중점 1 (플랫폼 의미 — ADR-009 개정 2)
- **위치:**
  - [diagnostics/host.rs:366-419](../../../crates/dot-logger/src/diagnostics/host.rs#L366-L419) (`available_bytes`, `portable_tests`)
  - 의존성 소스 확인: `fs4 1.1.0` `src/windows.rs` (`volume_path`, `statvfs`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `fs4::available_space(path)`는 Windows에서 `GetVolumePathNameW(path, &mut [0u16; 261])` → `GetDiskFreeSpaceExW`를 호출해 `lpFreeBytesAvailableToCaller`를 반환한다.
  2. **없는 경로 처리 비대칭:** Unix `statvfs`는 존재하지 않는 경로에 `Err(NotFound)`를 반환하지만, Windows `GetVolumePathNameW`는 존재하지 않는 경로도 상위 볼륨 루트(`C:\`)로 해석해 **`Ok(볼륨 여유)`**를 반환한다.
     - `LoggerBuilder::start()`는 `directory.is_dir()`를 선검사하므로 시작 시점에는 막히지만, 공개 함수 `diagnostics::available_bytes(path)` 자체나 운전 중 디렉터리/마운트 포인트가 사라진 `FileStore` probe·외부 `CheckRuntime` 호출에서는 Windows만 에러 대신 상위 볼륨 여유를 반환한다(예: 미마운트된 NTFS 정션 경로가 `C:\` 여유로 오측정됨).
  3. **문서·테스트 불일치:** `available_bytes` rustdoc(`host.rs:366-369`)에는 없는 경로 동작 차이, per-user quota 반영 차이(Windows `GetDiskFreeSpaceExW`는 per-user NTFS quota 반영, Unix `statvfs.f_bavail`은 root 예약 제외·per-user `quotactl` 미반영, macOS APFS purgeable 제외), `fs4`의 260-UTF16 볼륨 경로 버퍼 제한이 전혀 적혀 있지 않다. 또한 테스트 함수명(`host.rs:402` `available_bytes_reads_an_existing_directory_and_rejects_a_missing_one`)이 Windows에서는 없는 경로를 검증하지 않음에도 `rejects_a_missing_one`으로 남아 있다.
- **확인 방법:** `fs4-1.1.0/src/windows.rs` 및 `src/lib.rs` 직접 열람, `diagnostics/host.rs` 대조.
- **수정 제안:**
  - 일관된 계약을 원하면 Windows 분기(`host.rs:376-379`)에서 `std::fs::metadata(path)?;`로 경로 존재를 먼저 확인한 뒤 `fs4::available_space(path)`를 호출해 Unix와 동일하게 없는 경로를 `Err(NotFound)`로 통일하고, 테스트의 `#[cfg(unix)]` 조건을 제거해 Windows에서도 missing-path 에러를 검증한다.
  - 또한 `available_bytes` rustdoc에 Unix `f_bavail` vs Windows `lpFreeBytesAvailableToCaller`(per-user quota) 및 `fs4` 260-UTF16 볼륨 경로 한계를 명시한다.

**[처리담당]**
- 판정 결과: (개발 세션 작성)
- 처리 내용: (개발 세션 작성)
- 검증 근거: (개발 세션 작성)

---

#### RV-196 — ADR-009 개정 2 이후에도 기존 facade·소비자 테스트 11곳이 `#[cfg(target_os = "linux")]`로 잠겨 macOS/Windows CI에서 미실행 (P2)

- **심각도 / 중점:** `P2` / 중점 1 (플랫폼 의미 및 multi-OS CI 검증 공백)
- **위치:**
  - [logger/mod.rs:49](../../../crates/dot-logger/src/logger/mod.rs#L49), [logger/writer.rs:393](../../../crates/dot-logger/src/logger/writer.rs#L393), [logger/host.rs:1190](../../../crates/dot-logger/src/logger/host.rs#L1190)
  - [tc_013_facade.rs:3](../../../crates/dot-logger/tests/tc_013_facade.rs#L3), [tc_013_preset.rs:4](../../../crates/dot-logger/tests/tc_013_preset.rs#L4), [tc_013_routing.rs:4](../../../crates/dot-logger/tests/tc_013_routing.rs#L4), [tc_013_tracing.rs:4](../../../crates/dot-logger/tests/tc_013_tracing.rs#L4), [tc_014_standard.rs:4](../../../crates/dot-logger/tests/tc_014_standard.rs#L4)
  - [crates/dot-logger-client/tests/facade_remote.rs:2](../../../crates/dot-logger-client/tests/facade_remote.rs#L2)
  - [tests/api-consumer/src/main.rs:320,393](../../../tests/api-consumer/src/main.rs#L320), [tests/rt-allocation/src/tests.rs:1554](../../../tests/rt-allocation/src/tests.rs#L1554)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. REV-071 `RV-172` 당시 facade `file_output`이 Linux 전용이라 위 11곳에 `"needs the Linux free-space probe (RV-172)"` 주석과 함께 `#[cfg(target_os = "linux")]`를 적용했다.
  2. `9f687dd`·`ad11ba3`에서 macOS·Windows 파일 출력을 공식 지원하도록 변경했으나 위 가드들이 해제되지 않아, 현재 macOS·Windows CI(`contracts`)에서는 `os_exchange` 예제 1개를 제외한 **모든 facade 단위·통합·원격·표준 MCAP·소비자·RT 할당 테스트가 컴파일/실행되지 않는다**.
  3. 결과적으로 macOS·Windows 전용 facade 회귀(예: Windows 파일 잠금·경로 정리, macOS facade RT 할당, 이벤트 보호·회전·종료·원격 조합)가 CI에서 감시되지 않는다.
- **확인 방법:** `grep -rn 'target_os = "linux"'` 전수 확인.
- **수정 제안:**
  - `calibrate`(API §19.9)와 Linux 전용 `worker_cpus`/affinity 테스트·`tc_012_performance`·`tc_010_check_host`(`linux_available_bytes`)만 `target_os = "linux"`로 유지하고, 위 11곳의 facade 테스트 가드는 `#[cfg(any(unix, windows))]`로 넓혀 macOS·Windows native CI에서 실행·검증한다(Linux 전용 개별 테스트만 함수 단위 가드로 분리).

**[처리담당]**
- 판정 결과: (개발 세션 작성)
- 처리 내용: (개발 세션 작성)
- 검증 근거: (개발 세션 작성)

---

#### RV-197 — `examples/soak.rs` glibc 전용 FFI(`mallinfo2`, `malloc_trim`)의 `target_env = "gnu"` 가드 누락 (P2)

- **심각도 / 중점:** `P2` / 중점 3 (하네스 `unsafe`)
- **위치:**
  - [crates/dot-logger/examples/soak.rs:27-29, 90-126](../../../crates/dot-logger/examples/soak.rs#L27-L126) (`mod heap`)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. `soak.rs`의 `mod heap`은 `#[cfg(target_os = "linux")]` 아래에서 `extern "C" { fn mallinfo2() -> MallInfo2; fn malloc_trim(pad: usize) -> i32; }`를 선언·링크한다.
  2. `mallinfo2`(glibc 2.33+)와 `malloc_trim`은 glibc 고유 확장이며 `*-unknown-linux-musl` 타깃에는 존재하지 않는다.
  3. musl Linux 타깃에서 `cargo build/test --all-targets`를 실행하면 `soak` 예제 링크가 `undefined reference to mallinfo2 / malloc_trim`으로 실패한다.
  4. (참고: `GlobalAlloc` 구현 자체는 `System` 위임·null 검사·`wrapping_sub` 무패닉·`realloc` 증감 회계가 건전하며, `MallInfo2`의 `usize`×10 배치도 `/usr/include/malloc.h`와 일치함을 확인했다. 다만 `CALLS`는 `dealloc`을 제외하고 할당·재할당 호출만 세므로 주석/필드 설명을 명확히 해두면 좋다.)
- **확인 방법:** `soak.rs:27-126`과 `/usr/include/malloc.h` 대조.
- **수정 제안:**
  - `mallinfo2`/`malloc_trim` FFI 선언과 호출을 `#[cfg(all(target_os = "linux", target_env = "gnu"))]`로 분리하고, 비-glibc Linux(`#[cfg(all(target_os = "linux", not(target_env = "gnu")))]`)에서는 `glibc: null`, `trim() -> false` 폴백을 두어 카운팅 할당기(`live_bytes`/`low_bytes`/`calls`)는 musl에서도 그대로 빌드·동작하게 한다.

**[처리담당]**
- 판정 결과: (개발 세션 작성)
- 처리 내용: (개발 세션 작성)
- 검증 근거: (개발 세션 작성)

---

#### RV-198 — 외부 소비자 무잠금 검증의 Linux 편중 및 `edition = "2021"` 소비자의 MSRV 1.85 해석 안내 부재 (P2)

- **심각도 / 중점:** `P2` / 중점 4 (배포·공급망)
- **위치:**
  - [.github/workflows/ci.yml:30-33](../../../.github/workflows/ci.yml#L30-L33)
  - [scripts/check-external-consumer.py:18-45](../../../scripts/check-external-consumer.py#L18-L45)
  - [Cargo.toml:14-18, 45](../../../Cargo.toml#L14-L18), [README.md:48-55](../../../README.md#L48-L55)
- **실패 시나리오 (입력·상태 → 잘못된 결과):**
  1. **Windows/macOS 외부 소비자 미검증:** `ci.yml:32`가 `if: matrix.os == 'ubuntu-24.04'`로 제한돼 있어, Windows 전용 `fs4 = "1.1.0"`과 macOS `rustix` 의존성이 `Cargo.lock` 없는 외부 소비자의 최신 호환 버전 해석에서 MSRV 1.85와 함께 정상 컴파일되는지 CI에서 확인하지 못한다.
  2. **`edition = "2021"` 소비자의 MSRV 1.85 해석 함정:** ADR-009 개정으로 의존성을 `^` 범위로 풀고 MSRV 호환을 Cargo `resolver = "3"`에 위임했다. `check-external-consumer.py`는 `edition = "2024"`(자동 `resolver = "3"`)로 생성되므로 Rust 1.85에서 MSRV 인식 해석이 동작하지만, **기존 `edition = "2021"` 소비자 프로젝트**가 `resolver = "3"`를 명시하지 않고 Rust 1.85로 `dot-logger`를 추가하면, `^` 범위 의존성(`tokio`, `clap`, `serde`, `rustix`, `fs4` 등)이 Rust 1.86+를 요구하는 최신 패치/마이너로 해석되어 소비자 빌드가 실패할 수 있다.
- **확인 방법:** `ci.yml`, `check-external-consumer.py`, `Cargo.toml`, `README.md` 대조.
- **수정 제안:**
  - `ci.yml:32`의 Linux 제한을 해제하거나 Windows/macOS에서도 최소 `check-external-consumer.py`의 `check --features full`을 실행해 `fs4`·macOS `rustix`의 무잠금 해석을 검증한다(`RV-196`과 함께 `api-consumer`의 facade 경로도 3 OS에서 확인 가능).
  - `README.md`의 Rust 1.85 요구사항 절에 `edition = "2021"` 프로젝트가 Rust 1.85에서 사용할 때는 `Cargo.toml`에 `resolver = "3"`(또는 `Cargo.lock` 고정)가 필요함을 한 줄 안내한다.

**[처리담당]**
- 판정 결과: (개발 세션 작성)
- 처리 내용: (개발 세션 작성)
- 검증 근거: (개발 세션 작성)

---

#### RV-199 — Apple `MUTEX_BYTES` 예약 회계 미명시 영역 및 std 구현 가정 정리 (P3)

- **심각도 / 중점:** `P3` / 중점 2 (RT 무할당·예약 회계)
- **위치:**
  - [sync.rs:11-14](../../../crates/dot-logger/src/sync.rs#L11-L14)
  - [adapters/remote/types.rs:59-80](../../../crates/dot-logger/src/adapters/remote/types.rs#L59-L80), [adapters/remote/receiver.rs:88-92](../../../crates/dot-logger/src/adapters/remote/receiver.rs#L88-L92), [logger/budget.rs:424-450](../../../crates/dot-logger/src/logger/budget.rs#L424-L450)
- **실패 시나리오 (입력·상태 → 잠재 문제):**
  1. `a0fcfcf`는 core std `Mutex` 9곳을 모두 `warm_mutex`로 예열했으나, `MUTEX_BYTES`(Apple 64 B) 가산은 `FileLedger`, `telemetry::Bank`, `config_update::State` 3곳에만 추가했다.
  2. `RemoteHub`(`subs` 1 + 구독당 `state` 1), `Reassembler`(`generations` 1), `facade_bytes`(`Shared::telemetry`, `Held::writers`, `SinkState::writer`)는 여유 상수(`16384`, `1024` 등)에 암묵 흡수되어 현재 시험은 통과하지만, [coding_convention.md](../../04_implementation/coding_convention.md#L137)의 `"예약 회계에 들어가면 crate::sync::MUTEX_BYTES를 더한다"` 규칙과 일관성을 맞추려면 명시적 가산 또는 여유 상수 흡수 주석이 필요하다.
- **확인 방법:** `grep -rn "warm_mutex\|MUTEX_BYTES"` 전수 대조.
- **수정 제안:**
  - `RemoteLimits::reservation`, `Reassembler::new`, `facade_bytes`에 `crate::sync::MUTEX_BYTES`를 명시적으로 더하거나(또는 여유 상수에 포함됨을 주석으로 기록), `MUTEX_BYTES` 주석에 Apple `pthread_mutex_t` 64 B 가정이 macOS CI RT 할당 시험으로 감시됨을 남긴다.

**[처리담당]**
- 판정 결과: (개발 세션 작성)
- 처리 내용: (개발 세션 작성)
- 검증 근거: (개발 세션 작성)

---

#### RV-200 — `tc_013_calibrate.rs` `f19_reserve_shrinks_the_write_limit` 뺄셈 언더플로 방어 여지 (P3)

- **심각도 / 중점:** `P3` / 중점 3 (테스트 견고성 — TASK-038)
- **위치:**
  - [crates/dot-logger/tests/tc_013_calibrate.rs:132-145](../../../crates/dot-logger/tests/tc_013_calibrate.rs#L132-L145)
- **실패 시나리오 (입력·상태 → 잠재 문제):**
  1. `let reserve = free - 16 * MIB;`와 `seen - reserve`가 단순 `-` 연산이다.
  2. 일반 CI/로컬 환경에서는 `/tmp` 여유가 충분하지만, 아주 작은 tmpfs(`< 16 MiB`)이거나 백그라운드 쓰기가 두 측정 사이에 16 MiB 이상 여유를 줄인 경우 `seen < reserve`로 언더플로 패닉(또는 `NoSpace` 에러)이 발생한다.
- **확인 방법:** `tc_013_calibrate.rs:127-148` 코드 검토 및 로컬 11/11 Pass 확인.
- **수정 제안:**
  - `free.checked_sub(16 * MIB).expect("temp dir needs > 16 MiB free")` 및 `seen.checked_sub(reserve)`로 의도된 전제 조건을 명시적으로 드러낸다.

**[처리담당]**
- 판정 결과: (개발 세션 작성)
- 처리 내용: (개발 세션 작성)
- 검증 근거: (개발 세션 작성)

---

#### RV-201 — `scripts/clean-build-cache.sh` 인자 검증·GNU `df` 의존성 및 crates.io 게시 전 내부 `version` 준비 (P3)

- **심각도 / 중점:** `P3` / 중점 4 (스크립트·배포 준비)
- **위치:**
  - [scripts/clean-build-cache.sh:15-29](../../../scripts/clean-build-cache.sh#L15-L29)
  - [crates/dot-logger-client/Cargo.toml:16](../../../crates/dot-logger-client/Cargo.toml#L16), [crates/dot-logger-cli/Cargo.toml:14-15](../../../crates/dot-logger-cli/Cargo.toml#L14-L15), [crates/dot-logger-viewer/Cargo.toml:10-11](../../../crates/dot-logger-viewer/Cargo.toml#L10-L11)
- **실패 시나리오 (입력·상태 → 잠재 문제):**
  1. `clean-build-cache.sh`에 `--threshold`만 주고 값을 빠뜨리면 `set -u`로 `$2: unbound variable`가 나고, 비숫자 값을 주면 `[ "$before" -le "$threshold" ]`에서 셸 에러가 난다. 또한 `df --output=pcent`는 GNU coreutils(Linux) 전용이다.
  2. `dot-logger-client`·`dot-logger-cli`·`dot-logger-viewer`의 내부 의존성은 현재 `path = "..."`만 있고 `version`이 없다. 현재 `publish = false` 상태라 정상이나, MS-010 2단계 crates.io 게시 시 `version = "0.x.y"` 추가 및 `dot-logger-mcap` 선행 게시가 필요하다.
- **확인 방법:** 스크립트 및 4개 crate `Cargo.toml` 직접 열람.
- **수정 제안:**
  - `clean-build-cache.sh`에 `[ $# -ge 2 ]` 및 숫자 정규식 검증·Linux(GNU `df`) 전용 주석을 추가한다.
  - 내부 의존성 `version` 지정은 MS-010 2단계(crates.io 게시 체크리스트) 항목으로 유지한다.

**[처리담당]**
- 판정 결과: (개발 세션 작성)
- 처리 내용: (개발 세션 작성)
- 검증 근거: (개발 세션 작성)

---

## 5. 요청된 추가 분석 보고

### A. EV-120 초당 ~13,100회 할당 정적 분석 (중점 2 · TASK-039 참고)

`soak-rss-diag-1h.jsonl`(t=60 s → 120 s)에서 관측된 `calls` 증가분은 60초간 787,620건(**13,127건/s**, 제어 5,000건/s + 프레임 10건/s)이다. `soak.rs`의 `Counting` 할당기는 `alloc`·`alloc_zeroed`·`realloc`을 집계하며 `dealloc`은 집계하지 않는다. 정적 호출 그래프 추적 결과는 다음과 같다.

| 스레드 / 경로 | 예상 할당 빈도 | 코드 위치 및 원인 | RT 제약 위반 여부 |
|---|---|---|---|
| **RT 제어 스레드 (`control_loop`)** | **0회/s** | `Writer::try_log` → `Control::encode(&mut owned.scratch)` → `Producer::try_record_small` (`rtrb` + 원자 연산, 할당 0) | **준수 (무할당)** |
| **프레임 스레드 (`frame_loop`)** | **0회/s** | `Producer::try_acquire` → `fill` → `Producer::try_record_raw` (사전 할당 raw pool 대여) | **준수 (무할당)** |
| **호스트 스레드 (`Dispatcher::turn` → `take_shared`)** | **~10,010회/s (~76%)** | [routing/mod.rs:1244](../../../crates/dot-logger/src/routing/mod.rs#L1244) 소형 레코드마다 `vec![0; payload_len].into_boxed_slice()` (**5,000회/s**) + [routing/mod.rs:1256](../../../crates/dot-logger/src/routing/mod.rs#L1256) 모든 레코드마다 `Arc::new(SharedPayload)` (**5,010회/s**) | 일반 경로 (허용) |
| **IO 스레드 (`write_batch` → `SegmentLog::write_batch` → `mcap`)** | **~3,100회/s (~24%)** | [logger/host.rs:1090](../../../crates/dot-logger/src/logger/host.rs#L1090) 배치마다 `Vec<&Delivery>` 생성 + [storage/retention.rs:1011](../../../crates/dot-logger/src/storage/retention.rs#L1011) → [vendor/mcap-0.26.0/src/write.rs:1733,1803-1810](../../../vendor/mcap-0.26.0/src/write.rs#L1733) 매 배치 `flush_chunk()`로 `ChunkWriter` 종료·재생성에 따른 `BTreeMap`/`Vec<MessageIndexEntry>` 재할당 (~200 배치/s × ~15 alloc/배치) | 일반 경로 (허용) |

→ **결론:** RT 제어 스레드는 정적으로 할당하지 않으며, 초당 ~13,100회 할당은 전량 일반 경로(호스트·IO 스레드)의 유한 수명 임시 객체다. TASK-039에서 스레드별 카운터로 실측 확인 시 위 분포가 그대로 관측될 것으로 예상한다.

### B. TASK-024 S2 설계 전제·현재 동작 타당성 및 결정 D1~D3 의견 (중점 5)

1. **전제 1~6 코드 정합성:**
   - **전제 1·4·5:** 코드와 일치.
   - **전제 2:** `RetentionError::Capacity`의 손실 원인은 `Cause::Adapter`이며(`RV-193`), `IoPoisoned` 시 RT admission이 닫히지 않아 이후 수락분이 계속 실패 누적되고 파일 checkpoint 기록이 중단됨을 보강해야 한다.
   - **전제 3:** 코드와 일치(`logger/health.rs:178-179`). `RecordsFailed`가 `SyncOverdue`보다 우선순위가 높아 한 번 실패가 발생하면 `health().state`에서 `SyncOverdue`가 가려지므로 `OutputHealth.since_sync`로 별도 판정해야 한다.
   - **전제 6:** 기본 한도는 일치하나 `max_age = 10 s` 만료가 `Gap` 정책에서도 스트림을 종료함을 F2 설계에 반영해야 한다(`RV-193`).
2. **현재 동작 4가지의 설계상 타당성 의견:**
   - **실제 ENOSPC 시 세션 내 미회복(`IoPoisoned`):** 파일 손상 방지를 위해 현재 설계가 안전하고 타당하다. 다만 유일 필수 출력이 `IoPoisoned`인데도 RT `try_log`가 계속 `Accepted`를 반환하는 점은 운영상 혼선을 줄 수 있어, MS-010 2단계 전 기능 논의에서 재열기(`B1` UC-22)와 함께 `IoPoisoned` 시 admission 정책을 다루는 것이 적절하다.
   - **Ring의 외부 공간 부족 시 자기 파일 삭제:** 블랙박스 링 보존 계약(최신 데이터 우선)에 부합한다. 다만 외부 프로세스 때문에 링 이력 전체가 비워질 수 있으므로 최소 보존 하한/외부 압박 지표 여부는 기능 논의 주제로 적합하다.
   - **`Degraded(RecordsFailed)` 고착:** "세션 내 유실 발생" 래치로서 보수적이나 회복 여부와 `SyncOverdue`를 가린다. 기능 논의 안건(Health 회복 표시)에 올리는 것이 적절하다.
   - **재시작 시 `.active` 미복구:** `start()`의 예측 가능한 기동·무부작용 계약상 기본 미복구가 타당하다. 실사용처 편의를 위한 명시적 복구 헬퍼(manifest 자동화)를 기능 논의에 올리는 방향이 맞다.
3. **결정 D1~D3 리뷰어 의견:**
   - **D1 → 선택지 D(Docker `--device-write-bps`·`--device-write-iops`) 권장:** 의존성 추가 없이 cgroup v2 레벨에서 writeback·fsync 속도를 정확히 제한함이 이미 실증됨. 하네스 착수 시 `chroot` + `SYS_NICE`/`memlock` 1분 스모크 선행 권장.
   - **D2 → 선택지 A(`tc_008` 갈음 + facade DB 출력 없음은 기능 논의 안건) 권장:** facade에 없는 경로를 S2 하네스에 별도 구축하는 비용 대비 효용이 낮음. 단 EV에 "DB 장시간 단절 미실행, `tc_008` 단기 가짜 서버 시험으로 대체"를 명시.
   - **D3 → 선택지 A(F3b·F5는 현재 동작 관측·기록, 변경은 기능 논의) 권장:** 단 `RV-193`의 S2 설계·판정 기준 수정은 S2 하네스 착수 전에 먼저 반영해야 함.

---

## 6. 다음 액션

1. **개발 세션(우선):** `RV-193`(TASK-024 S2 설계 F3a/F2/F3b-F5 정렬)과 `RV-194~198`(P2 5건: Windows `FileStore`/`available_bytes` 계약·문서·테스트, macOS/Windows facade 테스트 재활성화, `soak.rs` `target_env = "gnu"` 가드, 외부 소비자 multi-OS/MSRV 안내) 및 P3 3건을 검토·조치한다.
2. **Human 결정:** S2 설계 결정 `D1~D3`(권장: D1=D, D2=A, D3=A) 확정 및 CI run `45181597098` 확인 후 TASK-038 Done 마감.
3. **후속 진행:** REV-072 조치·마감 뒤 TASK-024 S2 하네스 구현 → F1~F5 실행 → S5 → TASK-039 순서로 진행한다.

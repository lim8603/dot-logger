# 기술스택 등록부

- INT-001 / 2026-10-26 / Approved — Human의 상세 기준선 채택, ADR-007~009.
- 기술/버전 계열의 선택과 실제 설치·호환·성능 검증을 구분한다. 장비별 수락 조건과 의존성 호환 검증은 계획대로 수행한다.

## 확정 기술스택

| 영역 | 채택 | 버전/사양 | 근거 | 검증 경계 |
|---|---|---|---|---|
| 주 언어/빌드 | Rust·Cargo workspace | edition 2024, 개발 1.96.0, MSRV 1.85 | ADR-006·009 | TASK-002 로컬 Ubuntu 26.04.1에서 두 버전·16 feature 조합 Pass(EV-001), 3 OS × 2 Rust native CI·audit Pass(2026-10-27) |
| 아키텍처 | 내장형 Ports and Adapters·제한된 파이프라인 | 생산자별 큐, 비-RT background | ADR-007·008 | 프로세스 장애 완전 격리 아님 |
| 플랫폼 | Linux·macOS·Windows | 첫 기능 매트릭스: Ubuntu 24.04 x86_64, Windows 11 x86_64, macOS 14+ arm64 | ADR-006·009 | hosted native CI는 EV-001 Pass; Windows 11·Linux RT 커널/장비·추가 CPU는 TASK-022에서 별도 검증 |
| 큐 | rtrb | 0.4.0 SPSC | ADR-009 | 전체 API/Drop의 RT 안전성은 TASK-003·004 |
| 파일 | MCAP | mcap 0.26.0(vendor, 무압축 chunk CRC 1회 패치 — ADR-010 개정 1); CRC/index on, 초기 압축 off | ADR-009·010 | 인덱스 상한·active reader·복구/성능 검증 |
| 저장 매체 | SSD | 현재 PC의 CT1000T500SSD8(루트), HP EX950 512GB 2개(/data/build·/data/db)·ext4 | ADR-004·011 | 시험별 대상 경로 고정·실측으로 판정, 제품 사양으로 처리량 추정 금지 |
| 봉투·통신 | Protobuf·gRPC | tonic 0.13.1 / prost 0.13.5 | ADR-009 | bounded 조각·ACK·실제 TCP 대비 성능 검증 |
| 외부 I/O runtime | Tokio | 1.51.6 | ADR-009 | DB/remote background만, RT 경로 제외 |
| 시계열 DB | InfluxDB v2 어댑터 | v2 HTTP write·제한 수치 query; 로컬 기능 pin OSS2.9.1/Linux amd64 | ADR-009·EV-038 | digest/health version 고정 실제 DB write/query·중복/partial 및 Delivery/완료·로컬 파일 격리 EV-039. 다른 버전/OS·실앱·정량 성능/native 잔여, v3 호환 미선언 |
| 클라이언트 | Rust API + CLI, dot-probe 기존 화면 연동 | clap 4.6.6·serde 1.0.229/serde_json 1.0.154 | Human 선택·ADR-009·EV-025 | 로컬 CLI10/transport7·두 Rust Pass, 실제 dot-probe/다른 OS 후속 |
| Linux 공간 관측 구현 | rustix | =1.1.4, std/fs·Linux target 한정; linux-raw-sys0.12.1 lock | TASK-017 S3·API §15.5·EV-033, 승인된 OS probe의 내부 구현 | 일반 경로 statvfs·원 OS 오류, 다른 OS는 Unsupported; 실제 성능 인증 아님 |
| 일반 로그 어댑터 | tracing/log | 0.1.44 / 0.4.29 | ADR-009 | 전용 RT 계약과 분리 |
| 검증 도구 | cargo test/fmt/clippy·allocation guard·선택 Loom/Miri·native OS CI | Miri nightly-2026-10-26(검증 전용), 나머지 patch/CI 환경은 Task에서 고정 | ADR-009 | 모의/빌드 성공을 장비 성능/내구성으로 승격하지 않음 |

## 설치·실측 전 남은 작업

TASK-002에서 `rust-toolchain.toml`로 개발 Rust 1.96.0, workspace `rust-version = "1.85"`로 MSRV를 고정했다. Ubuntu 26.04.1 x86_64에서 Rust 1.85.0/1.96.0의 16가지 feature 조합·독립 consumer·clippy·개발 rustdoc/fmt를 확인했다([EV-001](../05_verification/verification_evidence.md)). `Cargo.lock`으로 patch/간접 의존성도 고정한다. 전역 rustup default는 변경하지 않고 프로젝트 toolchain으로 실행한다.

MCAP 0.25의 선택 zstd 경로에서 발견한 취약점은 Human 승인 [ADR-010](adrs/ADR-010_mcap_security_update.md)에 따라 0.26.0(zstd 0.14.3 / zstd-safe 8.2.0)으로 수정했다. Tokio는 같은 계열 수정판 1.51.6이다. rustix는 RUSTSEC-2026-0486(xattr unsound, dot-logger 미사용 함수) 때문에 같은 계열 1.1.6으로 고정했다(2026-11-11 Human 「올려올려」, [EV-069](../05_verification/verification_evidence.md#ev-069)). 최종 보안 검사 취약점/unsound 0건, `paste` 유지보수 중단 경고 1건은 GAP-005로 추적한다. 외부 177개 의존성의 라이선스 metadata 누락은 0건이며 [목록](../06_evolution/imported_context/task_002_2026-10-26/dependencies.json)에 원문 표현을 보존했다. 제품 배포 라이선스 결정·고지 완성의 의미는 아니다.

CI는 Ubuntu 24.04·Windows Server 2025·macOS 14 × Rust 1.85.0/1.96.0의 6개 구성 검사와 audit 모두 [Pass](https://github.com/lim8603/dot-logger/actions/runs/40504484501)했다(2026-10-27, EV-001, GAP-004 Closed). Windows 11 및 실제 PREEMPT_RT/장비 수락은 별도다. CLI/clap과 이후 추가 의존성은 해당 Task에서 같은 절차로 고정한다.

2026-10-31 ADR-011로 현재 Ubuntu PC를 실제 장치 시험 대체 환경으로 확정했다. TASK-022에서 실행 커널/시험 경로·NIC·제어 부하/기한을 고정하며 UPS는 시뮬레이션으로 판정한다. GitHub native는 공개 완료 후 실행한다. R1은 참조 조건이며 다른 장비·Tier 2에 자동 적용하지 않는다. FR-015 추천 기능은 별도 제안이며 자동 채택하지 않았다.

## 검토했으나 첫 범위에 채택하지 않은 대안

| 대안 | 이유 | 근거 |
|---|---|---|
| 독립 수집 데몬·IPC | 첫 적용의 배포·수명/크래시 회수 복잡성 | ADR-007 |
| 전 RT 경로 async 제출 | 할당·스케줄링·대기의 RT 경로 결합 | ADR-007 |
| 자체 파일 container·raw TCP | 호환·복구·framing·취소 구현 부담 | ADR-009 |
| 전역 공유 MPSC | RT 경합·재시도 상한 검증 부담 | ADR-009 |

## 조사 근거

[2026-10-26 상세설계 조사](../06_evolution/imported_context/research_2026-10-26_detailed_design.md)와 [2026-10-25 초기 조사](../06_evolution/imported_context/research_2026-10-25_logging_telemetry.md)는 조사 당시의 사실·권고 기록이다. 현재 채택 여부의 기준은 이 표와 Accepted ADR이며, 조사에만 등장하는 OTel/기타 도구까지 채택하지 않는다.


## TASK-004 검증 도구 구체화 — 2026-10-27

ADR-009의 선택 검증 도구 중 Miri를 사용했다. 개발/MSRV pin은 그대로이고 검증 전용 `nightly-2026-10-26`(Miri 포함)을 추가 설치했다. strict provenance·8개 seed의 raw 수명/반환 해석 검사는 EV-003 로컬 Pass이며 CI에 동일 명령의 raw-memory job을 추가했다. 해당 변경분 native CI는 게시 후 시도했으나 GitHub 계정 결제/지출 한도로 runner 시작 전에 차단돼 시험은 미실행이다(EV-003/GAP-004 Open). 제품 workspace forbid는 유지하고 dot-logger package deny/private pool-storage allow로 원본 UnsafeCell 접근을 한 모듈에 격리했다. 외부 dependency/lock 변경은 없다. Miri의 공식 사용법·coverage 한계는 [공식 문서](https://github.com/rust-lang/miri#using-miri)를 따르며 실제 장비 성능 수락을 대체하지 않는다.

## 현재 PC 검증 프로필 — 2026-10-31

[ADR-011](adrs/ADR-011_local_pc_simulated_ups_verification.md)·[TASK-022](../04_implementation/tasks/TASK-022.md): Ubuntu26.04.1 LTS x86_64, Ryzen7 H255 8코어16스레드, OS 인식 메모리59,750,248,448B. 현재 실행7.0.0-42-generic(CONFIG_PREEMPT_RT 미설정), 설치7.0.0-38-realtime(CONFIG_PREEMPT_RT=y). 현재 PC 실측/UPS 시뮬레이션을 수락하며 일반/RT 실행을 별도로 기록한다. [환경 원문](../06_evolution/imported_context/verification_policy_2026-10-31/environment.json). 제품 기능·성능 시험은 이번 환경 조사에서 실행하지 않았다.

TASK-012(2026-10-31): 선택 remote에서 tonic/tls-ring을 활성화했다. 기존 lock의 package/version 집합은 유지하고 TLS dependency edge만 연결했다. host 소유 Tokio runtime·실제 TLS/read-manage 권한·bounded RPC는 EV-022 로컬 검증이며 core consumer의 네트워크 격리는 두 Rust16feature로 회귀했다. LAN 성능·native 공개 후 검증은 별도다.

2026-10-31 REV-019 보강(EV-023): 기존 TLS 간접 의존 subtle2.6.1을 remote 전용 직접 의존으로 pin해 고정256B credential/길이 비교에 사용한다. package/version 집합·MSRV/기술 계열·인증 정책은 유지한다. 전체 RPC constant-time/실제 공격 내성의 정량 증거는 아니다.

2026-10-31 TASK-013 EV-025: native client/CLI를 clap4.6.6·기존 serde1.0.229/JSON1.0.154, 기존 tonic/tower0.5.3/hyper-util0.1.21·Tokio1.51.6 signal로 구현했다. MSRV1.85/개발1.96 로컬 검증 완료, 기존 package/version 교체0. 로컬 CLI package1개·외부6개(anstyle/clap/clap_builder/clap_lex/errno/signal-hook-registry)를 추가했으며 결과 JSON에 정확한 pin·license/rust_version metadata가 있다. cached audit 취약점/unsound0·기존 paste 경고1, fresh DB 갱신은 미실행이다. core/default client의 선택 normal/build 그래프 격리와 RT allocator를 유지한다. 단일 실제 TCP/명시 CA/token·자동 reconnect/retry 없음·native DTO/CLI 출력 계약을 API §14.4에 기록했다.

2026-11-01 TASK-017 S3 EV-033: 안전한 OS 공간 조회의 구현 세부로 Linux target에 rustix1.1.4(std/fs)를 고정했다. 기존 package 버전 교체 없이 rustix/linux-raw-sys 2개를 추가하고 own unsafe·subprocess·RT 경로 변경은 없다. 원래 OS 오류·checked available blocks×fragment size·불가능한 카운터 거절을 사용한다. [공식 문서](https://docs.rs/rustix/1.1.4/rustix/fs/fn.statvfs.html)·로컬 소스의 MSRV1.63을 확인했고 프로젝트 MSRV1.85 workspace 회귀를 별도 실행한다. Rust/API/아키텍처·platform tier 변경이 아니며 되돌릴 수 있는 host adapter 세부라 새 ADR로 승격하지 않았다. 새 의존성의 audit와 실제 실행 근거는 EV-033을 따른다.


2026-11-01 TASK-019 S2 EV-038: 승인 v2 계열의 **로컬 기능 시험 프로필**을 OSS2.9.1/Linux amd64 image digest `sha256:a66a9e8b2c93f58bec088c17ff5876540caa095436df5a66e908bdbdb6a36205`와 `/health` v2.9.1로 고정했다. [API§14.7](interface_contract.md#147-task-019-s2-제한-수치-query실제-db-검증--2026-11-01)의 격리 tmpfs/loopback/유한 container에서 실행하며 사용자 기존 DB 업그레이드/운영 배포가 아니다. serde_json1.0.154를 기존 lock 그대로 선택 직접 의존에 추가했다. ADR-009의 v2/제한 query 방향 안의 되돌릴 수 있는 시험/adapter 세부이며 신규 계열/MSRV/저장 계약 변경은 없다. 다른 DB/OS·실앱/성능·공개 후 native는 후속이다.

2026-11-01 TASK-019 S3 EV-039: 기존 reqwest/Tokio와 원 Router Delivery를 일반 worker의 명시 동기 한 건 API로 연결했다. current-thread runtime의 실제 로컬 IO/DNS 정리가 끝날 때까지 guard를 유지한다. 별도 daemon/자동 worker·외부 runtime 타입을 공개 API에 강제하지 않으며 신규 의존성/버전 변경 없음. 로컬 파일 격리/기능 관측과 실앱·정량 성능/다른 OS/native 수락을 구분한다.

2026-11-03 TASK-023 S1 EV-046: 성능 harness 전용으로 Linux dev-dependency의 기존 rustix 1.1.4에 `thread`/`time` feature를 추가했다(per-thread `sched_setaffinity`, `clock_nanosleep` 절대 release). 새 crate·lock·MSRV 변경 없음, 제품 normal 그래프는 std/fs 유지. RT 행의 SCHED_FIFO/메모리 잠금 방법은 RT 요청 때 정한다.

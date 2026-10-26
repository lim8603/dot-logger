# 기술스택 등록부

- INT-001 / 2026-10-26 / Approved — Human의 상세 기준선 채택, ADR-007~009.
- 기술/버전 계열의 선택과 실제 설치·호환·성능 검증을 구분한다. 장비별 수락 조건과 의존성 호환 검증은 계획대로 수행한다.

## 확정 기술스택

| 영역 | 채택 | 버전/사양 | 근거 | 검증 경계 |
|---|---|---|---|---|
| 주 언어/빌드 | Rust·Cargo workspace | edition 2024, 개발 1.96.0, MSRV 1.85 | ADR-006·009 | TASK-002 로컬 Ubuntu 26.04.1에서 두 버전·16 feature 조합 Pass(EV-001), native OS CI 미실행 |
| 아키텍처 | 내장형 Ports and Adapters·제한된 파이프라인 | 생산자별 큐, 비-RT background | ADR-007·008 | 프로세스 장애 완전 격리 아님 |
| 플랫폼 | Linux·macOS·Windows | 첫 기능 매트릭스: Ubuntu 24.04 x86_64, Windows 11 x86_64, macOS 14+ arm64 | ADR-006·009 | Linux RT 커널/장비·추가 CPU는 TASK-022; native 실행 전 |
| 큐 | rtrb | 0.4.0 SPSC | ADR-009 | 전체 API/Drop의 RT 안전성은 TASK-003·004 |
| 파일 | MCAP | mcap 0.26.0; CRC/index on, 초기 압축 off | ADR-009·010 | 인덱스 상한·active reader·복구/성능 검증 |
| 저장 매체 | SSD | SATA/NVMe·용량/제품 미정 | ADR-004 | 처리와 저장/보존 예산 분리 |
| 봉투·통신 | Protobuf·gRPC | tonic 0.13.1 / prost 0.13.5 | ADR-009 | bounded 조각·ACK·실제 TCP 대비 성능 검증 |
| 외부 I/O runtime | Tokio | 1.51.6 | ADR-009 | DB/remote background만, RT 경로 제외 |
| 시계열 DB | InfluxDB v2 어댑터 | v2 HTTP write·제한 수치 query | ADR-009 | 실제 배포 버전 pin·충돌/부분 실패 테스트, v3 query 호환 미선언 |
| 클라이언트 | Rust API + CLI, dot-probe 기존 화면 연동 | clap 4·JSON 출력 | Human 선택·ADR-009 | 독립 GUI 제외, native 클라이언트 테스트 |
| 일반 로그 어댑터 | tracing/log | 0.1.44 / 0.4.29 | ADR-009 | 전용 RT 계약과 분리 |
| 검증 도구 | cargo test/fmt/clippy·allocation guard·선택 Loom/Miri·native OS CI | 세부 도구 patch·CI 실행 환경은 Task에서 고정 | ADR-009 | 모의/빌드 성공을 장비 성능/내구성으로 승격하지 않음 |

## 설치·실측 전 남은 작업

TASK-002에서 `rust-toolchain.toml`로 개발 Rust 1.96.0, workspace `rust-version = "1.85"`로 MSRV를 고정했다. Ubuntu 26.04.1 x86_64에서 Rust 1.85.0/1.96.0의 16가지 feature 조합·독립 consumer·clippy·개발 rustdoc/fmt를 확인했다([EV-001](../05_verification/verification_evidence.md)). `Cargo.lock`으로 patch/간접 의존성도 고정한다. 전역 rustup default는 계속 미설정이며 프로젝트 toolchain으로 실행한다.

MCAP 0.25의 선택 zstd 경로에서 발견한 취약점은 Human 승인 [ADR-010](adrs/ADR-010_mcap_security_update.md)에 따라 0.26.0(zstd 0.14.3 / zstd-safe 8.2.0)으로 수정했다. Tokio는 같은 계열 수정판 1.51.6이다. 최종 보안 검사 취약점/unsound 0건, `paste` 유지보수 중단 경고 1건은 GAP-005로 추적한다. 외부 177개 의존성의 라이선스 metadata 누락은 0건이며 [목록](../06_evolution/imported_context/task_002_2026-10-26/dependencies.json)에 원문 표현을 보존했다. 제품 배포 라이선스 결정·고지 완성의 의미는 아니다.

CI는 Ubuntu 24.04·Windows Server 2025·macOS 14와 두 Rust 버전으로 구성했지만 아직 원격 실행하지 않았다(GAP-004). Windows 11 및 실제 PREEMPT_RT/장비 수락은 별도다. CLI/clap과 이후 추가 의존성은 해당 Task에서 같은 절차로 고정한다.

TASK-022에서 실제 OS/CPU/SSD/FS/NIC/제어 주기/UPS 조건을 확정한다. R1은 참조 조건이며 다른 장비·Tier 2에 자동 적용하지 않는다. FR-015 추천 기능은 별도 제안이며 자동 채택하지 않았다.

## 검토했으나 첫 범위에 채택하지 않은 대안

| 대안 | 이유 | 근거 |
|---|---|---|
| 독립 수집 데몬·IPC | 첫 적용의 배포·수명/크래시 회수 복잡성 | ADR-007 |
| 전 RT 경로 async 제출 | 할당·스케줄링·대기의 RT 경로 결합 | ADR-007 |
| 자체 파일 container·raw TCP | 호환·복구·framing·취소 구현 부담 | ADR-009 |
| 전역 공유 MPSC | RT 경합·재시도 상한 검증 부담 | ADR-009 |

## 조사 근거

[2026-10-26 상세설계 조사](../06_evolution/imported_context/research_2026-10-26_detailed_design.md)와 [2026-10-25 초기 조사](../06_evolution/imported_context/research_2026-10-25_logging_telemetry.md)는 조사 당시의 사실·권고 기록이다. 현재 채택 여부의 기준은 이 표와 Accepted ADR이며, 조사에만 등장하는 OTel/기타 도구까지 채택하지 않는다.

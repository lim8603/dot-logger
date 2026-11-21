# ADR-009 — 파일·외부 연동·도구체인 권고

- 상태: Accepted / 날짜: 2026-10-26 / 승인: 2026-10-26 Human 「권고 기준선 채택 — 장비·호환 검증은 계획대로 진행」
- 관련: INT-001 / MS-001~007 / FR-003~006·010·014·016·017 / NFR-002~007
- 핵심 기술 선택 H 승인 완료. 아래 기술/버전 계열을 채택하며 실제 설치·patch 고정·의존성 호환 검증은 TASK-002에서 수행한다.
- 2026-10-26 정정: MCAP 계열은 [ADR-010](ADR-010_mcap_security_update.md)의 Human 승인으로 0.26.0으로 대체했다. 나머지 결정은 유효하다.

## 결정 요인과 권고

| 영역 | 권고 | 대안과 비용 |
|---|---|---|
| build | Cargo workspace, Rust 2024 edition, MSRV 1.85를 검증 후보로 시작 | 기존 dot-probe edition과 일치. 실제 dependency MSRV 실패 시 근거를 기록하고 MSRV 변경을 재검토 |
| 입력 큐 | rtrb 0.4 계열, producer별 SPSC | 공유 MPSC는 RT 경합/재시도 증명 추가. 큐 자체 wait-free와 전체 API를 구분 |
| 파일 | MCAP 0.26.0(ADR-010), chunk CRC/index on, compression off 기준선 | 자체 container는 호환·복구 비용 증가. SQLite는 큰 원본 혼합의 목표 성능 비교 필요 |
| payload/wire | 버전 있는 Protobuf envelope, 사용자 encoding bytes | 자체 binary protocol보다 도구·진화 규칙 재사용, 직렬화 복사는 비용 포함 |
| 통신 | gRPC tonic 0.13 + prost 0.13, Tokio 1은 remote/DB background에 한정 | REST+WS는 control/stream 프로토콜 두 개, raw TCP는 framing/취소/보안 직접 구현 |
| DB | InfluxDB v2 write 및 제한된 수치 query adapter, 배포 버전 별도 pin | dot-probe 기존 API와 일치. v3 호환 write만으로 query 호환 판단 금지 |
| 클라이언트 | Rust API + CLI, clap 4/JSON 출력 후보, dot-probe 기존 화면 연동 | 독립 GUI는 이번 Human 선택 범위 밖 |
| 입력 어댑터 | tracing 0.1/log 0.4, 일반 경로 | 전용 RT API의 성능 계약을 적용하지 않음 |
| 검증 | cargo test/fmt/clippy, allocation guard, Loom/Miri 선택 검증, native OS CI | 모의 시험만으로 RT/전원 내구성 판정 불가 |

세부 patch는 기존 dot-probe lockfile과 라이선스·MSRV·보안 검사를 거쳐 TASK-002에서 고정한다. major/minor 갱신이 계약/호환을 바꾸면 새 검토가 필요하다. 현재 실행 환경은 rustup default toolchain이 없어 rustc 실행이 불가했다. 도구체인 설치/제품 코드는 이번 설계 작업에서 수행하지 않았다.

## 플랫폼 검증안

첫 네이티브 기능 매트릭스 후보는 Linux Ubuntu 24.04 x86_64, Windows 11 x86_64, macOS 14+ arm64다. 이는 모든 과거 OS·모든 CPU 지원의 선언이 아니다. Linux PREEMPT_RT의 커널/CPU·SSD·NIC·UPS와 실제 제어 주기는 TASK-022에서 확정한다. 실제 배포가 aarch64 Linux·Intel macOS이면 해당 네이티브 행을 추가해야 한다. Tier 2는 기능/교환 수락, 엄격한 성능 합격선은 없다.

## 위험과 완화

- dot-probe는 MCAP 0.25 계열이다. TASK-002의 보안 검사·호환 시험과 Human 승인으로 0.26.0을 채택했다(ADR-010). 기존 앱 통합은 TASK-026에서 검증한다.
- MCAP target chunk 크기는 메모리 hard cap이 아니다. max record·인덱스/회전 상한과 실제 peak를 별도로 검증한다.
- Protobuf/gRPC bulk 복사·HTTP/2 흐름제어는 P4-bulk 기준 TCP 대비 80% 목표를 실측해야 한다. 실패하면 먼저 chunk/복사/worker를 측정하고 대안 ADR로 변경한다.
- DB projection의 동시 timestamp 충돌과 timeout 후 결과 불명을 테스트한다. 원본 전량 보존의 기준은 해당 파일 경로다.
- borrow 기반 RT 핸들/무할당/해제 증명은 rtrb 도입으로 자동 해결되지 않는다.

## 근거

2026-10-26 공식 자료 및 dot-probe 조사(비공개 원시 근거: `imported_context/research_2026-10-26_detailed_design.md`). 모든 성능 수치는 목표 또는 향후 실측이며 외부 벤치마크를 제품 성능으로 재사용하지 않았다.

## 승인 범위

상세설계·기술 선택·추가 시험 목표·MS 계획의 기준선 채택을 승인받았다. 장비별 수락 조건·의존성/MSRV 호환 확인은 TASK-002·022에 남긴다. FR-015/TASK-018은 별도 제안이다. 실제 구현·성능·제품 테스트 통과를 승인한 것은 아니다.

## 2026-10-31 검증 방법·시점 갱신

Human 승인 [ADR-011](ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 수락은 현재 Ubuntu PC 실측으로 대체하고 UPS는 시뮬레이션 결과로 판정한다. GitHub native는 저장소 공개 완료 후 실행한다. 이전 실제 장비/실물 UPS 필수·native 실행 시점은 이 범위에서 갱신하며 기능/완료/오류/예산 계약은 유지한다. 시뮬레이션 수락을 물리 전원 소실 내구성의 관측 증거로 표시하지 않는다.

## 개정 — 라이브러리 의존성 버전 범위(2026-11-21)

- **문제:** 모든 의존성을 `=` 정확한 버전으로 고정했다. 우리 바이너리와 CI 재현에는 맞지만, 라이브러리로 배포하면 사용하는 쪽이 같은 crate의 다른 버전(예: tokio 1.52)을 쓸 때 의존성 해석이 실패한다.
- **결정(2026-11-21 Human 결정(1 A·2 권장대로·3 미리보기·4 TASK-026 뒤·5 권장대로)의 2):** 라이브러리 crate의 의존성은 `^` 범위로 바꾸고, 하한은 지금 검증한 버전으로 둔다. 보안 수정 때문에 올린 하한(tokio 1.51.6, rustix 1.1.6 등)은 그대로 하한이 된다. `Cargo.lock`은 계속 커밋하고 CI는 `--locked`로 재현한다. MSRV(1.85)는 resolver 3의 rust-version 인식 해석에 맡긴다. 하한 버전이 실제로 빌드되는지는 TASK-036에서 확인 방법을 정한다.
- **유지되는 것:** 위 표의 기술 계열 선택과 MSRV.


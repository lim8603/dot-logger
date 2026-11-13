# TASK-033 — 로깅 사용 사례 조사와 커버리지 기반 완료 판정

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-001·002·003·004·005 |
| 요구사항 | FR 전체(Must)·NFR-004·NFR-008 |
| 설계 | [인터페이스](../../03_design_artifacts/interface_contract.md) · [시험 전략](../../05_verification/test_strategy.md) · [추적표](../../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트) |
| ADR | 없음 — 완료 기준은 [quality_gate](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | S2 — 로보틱스/RTOS 로깅 사용 사례 조사·UC 목록 |
| 의존 | 없음(기존 시험·EV-078 기준선 사용) |
| 예상 소요 | 3~5 인일, 리뷰 대기 제외 |

## 목표

2026-11-13 Human 결정에 따라 프로젝트 일정은 실사용처 적용이 아니라 **시험 커버리지**와 **실제 로깅 사용 사례 조사**로 판단한다. 이 Task는 그 판단 근거를 만든다.

1. 로보틱스·RTOS·임베디드 로깅의 실제 사용 사례를 조사해 사용 사례(UC) 목록을 만든다.
2. 각 UC를 기존 TC·시험 함수와 대응시키고, 대응이 없는 UC는 시험을 추가하거나 범위 밖 사유를 기록한다.
3. 라인·함수·리전 커버리지 기준선에서 파일별 공백을 분류하고, 실제 동작 공백은 시험으로 메운다.
4. 목표치(커버리지·UC 대응률)를 제안해 Human 승인을 받고, MS-007 일정 판단에 쓴다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 커버리지 측정 도구와 기준선 | `cargo llvm-cov` workspace all-features 측정·파일별 미실행 줄 목록, 저위 파일 분류 — [EV-078](../../05_verification/verification_evidence.md#ev-078) | Done |
| S2 | 로깅 사용 사례 조사 | 공식 문서·공개 저장소 근거가 붙은 UC 목록(UC-ID·출처·요구 동작·관련 FR/NFR). 조사 대상 후보: ROS 2 rosbag2/MCAP, Foxglove·Lichtblick, PX4 ULog, ArduPilot DataFlash, Zephyr·FreeRTOS 로깅, defmt, spdlog·Quill 같은 비동기 로거, Rust `tracing`/`log` 생태계, 자동차 DLT. 사실·추정 구분 | Planned |
| S3 | UC↔TC 대응과 공백 시험 | UC별 대응 시험 표, 대응 없는 UC의 시험 추가(Red→Green) 또는 범위 밖 사유, 아래 커버리지 공백 중 「실제 동작」 항목 처리 | Planned |
| S4 | 목표치와 완료 판정 | 커버리지·UC 대응률 목표 제안 → Human 승인 → 재측정 결과로 MS-007 일정 판단 | Planned |

## 커버리지 기준선과 공백 분류 — S1 (2026-11-13)

측정: [EV-078](../../05_verification/verification_evidence.md#ev-078). workspace 전체 라인 **91.15%**(18,826줄 중 1,667줄 미실행), 함수 90.92%, 리전 88.27%. doctest와 `#[ignore]` 성능 시험은 측정에 들어가지 않는다.

라인 커버리지 85% 미만 파일을 미실행 줄 단위로 직접 확인해 분류했다.

| 파일 | 라인 | 분류 | 내용 |
|---|---|---|---|
| `dot-logger/src/routing/types.rs` | 0% (6줄) | 비동작 | `Display`·`From` 구현 |
| `dot-logger/src/capture/types.rs` | 63.6% | 비동작 | `Display` 구현 |
| `dot-logger/src/completion/types.rs`·`control/types.rs` | 76~78% | 비동작 | `Display`·`Debug` 구현 |
| `dot-logger/src/logger/writer.rs` | 79.5% | 일부 **실제 동작** | `Schema::new`·`descriptor`는 const 평가라 측정에 안 잡힘(비동작). **다른 Logger의 Channel 거절은 시험이 없었음 → 2026-11-13 추가.** **설정 변경(level 필터) 뒤 facade `try_log`가 `Logged::Skipped`를 돌려주는 경로는 시험 없음 → S3** |
| `dot-logger/src/logger/sink.rs` | 81.8% | 대부분 비동작 | tracing `Subscriber`의 span 관련 빈 메서드·`Debug`. `log` crate의 `enabled`·`flush` 직접 호출은 S3에서 확인 |
| `dot-logger-cli/src/commands.rs` | 74.9% (204줄) | **실제 동작** | remote `blob`·`completion`·`replay` 관리 명령의 CLI 경로, query/replay `--include-payload`의 blob(큰 payload) 읽기, 일부 출력 한도 초과 → S3 |
| `dot-logger-cli/src/output.rs`·`model.rs` | 82~83% | 혼합 | 출력 한도·입력 검증 분기 → S3에서 분류 |
| `dot-logger-client/src/diagnostics/bridge.rs` | 74.4% | **실제 동작** + 범위 의문 | 스냅샷 파일 검증 실패·단계 사이 취소/기한 만료 분기. 이 모듈은 dot-probe export 모양을 전제한 **사용처 전용 코드가 라이브러리 `src`에 있는 경우**다. 범용 라이브러리 기준에서 예제로 옮길지 S3에서 판단(코드 이동은 Human 확인) |
| `dot-logger-client/src/diagnostics/mod.rs` | 73.0% | 혼합 | 위 bridge의 입력 검증 → S3 |
| `dot-logger/src/storage/standard.rs` | 83.8% | 실제 동작 | TASK-030 진행 중 범위(손상 입력 분기) — TASK-030 S1 리뷰에서 처리 |
| `dot-logger/src/completion/query.rs`·`logger/health.rs`·`logger/mod.rs`·`telemetry/capacity.rs`·`adapters/remote/wire.rs`·`client/codec.rs` | 82~86% | 미분류 | S3에서 분류 |

## 2026-11-13 공백 시험 추가

| 시험 | 대상 | 확인 |
|---|---|---|
| `tc_013_facade::f8_channel_from_another_logger_is_rejected_and_never_stored` | facade `try_log`의 세션 검사 | Pass. 세션 검사를 끈 변이에서 실패 확인 후 원복 |
| `tc_013_calibrate::f19_cancellation_during_measurement_stops_and_removes_the_probe` | 보정 측정 루프의 취소(TASK-028 잔여) | Pass. 루프의 취소 검사를 끈 변이에서 `Ok(Profile)`로 끝나 실패 확인 후 원복(tmpfs, 변이 1회 512 MiB) |

두 시험 모두 구현은 이미 올바랐고 시험만 없었다. 그래서 Red는 구현 전 실패가 아니라 변이로 확인했다.

## 완료 기준

- [x] S1 커버리지 기준선·파일별 공백 분류(EV-078).
- [ ] S2 사용 사례 목록(출처 포함).
- [ ] S3 UC↔TC 대응표와 공백 시험(실제 동작 공백 0 또는 사유 기록).
- [ ] S4 목표치 Human 승인과 재측정 결과.
- [ ] Human 리뷰 완료.

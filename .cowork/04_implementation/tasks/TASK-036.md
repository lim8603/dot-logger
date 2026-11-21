# TASK-036 — 배포 가능화(0단계)

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-010 |
| Story | US-001·003 |
| 요구사항 | FR-001 NFR-004 |
| 설계 | [ADR-010 개정 2](../../03_design_artifacts/adrs/ADR-010_mcap_security_update.md#개정-2--배포-가능한-vendor-구조2026-11-21) · [ADR-009 개정](../../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md#개정--라이브러리-의존성-버전-범위2026-11-21) |
| ADR | ADR-009·010 개정(2026-11-21 Human 결정) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-21 |
| 다음 액션 | S5 푸시 뒤 CI의 macOS RT 할당 시험 확인 → Human 리뷰(수락). 별칭 crate 이름·버전은 2026-11-21 Human 「권장대로」 확정 |
| 의존 | 없음(T3 native CI와 병행 가능) |
| 예상 소요 | 1~2 인일 |

## 배경

2026-11-21 워크스페이스 밖 임시 소비자 프로젝트(`dot-logger = { path = … }`)가 `WriteOptions::chunk_registrations` 없음(`E0599`)으로 컴파일되지 않았다. 원인은 패치한 MCAP를 워크스페이스 `[patch]`로만 적용한 구조다. README가 안내하는 git 의존성도 같은 이유로 동작하지 않는다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 재현 시험 | 워크스페이스 밖 소비자 빌드 검사(스크립트와 CI 단계)를 추가해 현재 실패를 확인 | Done — `E0599` 재현 |
| S2 | MCAP 별칭 crate | vendor를 이름을 바꾼 워크스페이스 crate로 옮기고 `package =` 별칭으로 의존, `[patch]` 제거. VENDORED.md·라이선스·패치 파일 정합. S1 통과 | Done — `dot-logger-mcap`(vendor 위치·lib 이름 `mcap` 유지), lock 변경은 그 항목 하나 |
| S3 | 의존성 범위 | 라이브러리 crate 의존성을 `^` 범위로(하한 = 검증 버전·보안 하한). lock 유지, 하한 확인 방법 결정 | Done — `=` 20개 → `^`. 하한 = lock 버전이라 `--locked` CI가 하한을, lockfile 없는 외부 소비자 검사가 최신 호환 버전을 확인 |
| S5 | macOS 잠금 예열 | core std `Mutex` 9개를 생성 시 한 번 잠가(`crate::sync::warm_mutex`) Apple의 첫 잠금 할당을 예열로 옮기고, 예약 회계(telemetry Bank, 설정 변경, FileLedger)에 `MUTEX_BYTES`(Apple 64, 그 밖 0) 추가. macOS RT 할당 시험 3개 ignore 해제 | Done(Linux, [EV-115](../../05_verification/verification_evidence.md#ev-115)) — macOS는 푸시 뒤 CI로 확인 |
| S4 | 문서·회귀 | README 설치 안내, tech_stack, 두 Rust 회귀·feature 조합·Miri·교차 Clippy | Done — README의 git 의존성 안내는 이제 동작해 문구 유지. 회귀 EV-114 |

## 완료 기준

- [x] 워크스페이스 밖 소비자가 git·path 의존성으로 빌드된다(로컬 확인, CI 단계 추가 — 푸시 뒤 CI 결과로 최종 확인).
- [x] 기존 회귀와 출력 바이트(MCAP CRC 포함)가 바뀌지 않는다(MCAP 소스 변경 없음, EV-114).
- [ ] Human 리뷰.

## 결과 — 2026-11-21

- S1: `scripts/check-external-consumer.py` 추가, CI contracts의 Linux 행에 단계 추가. 수정 전 `E0599` 재현.
- S2: vendor 패키지 `dot-logger-mcap` 0.1.0(`publish = false`, lib `mcap`), workspace `mcap = { package = "dot-logger-mcap", version = "0.1.0", path = … }`, `[patch.crates-io]` 제거. VENDORED.md에 패키지 정체성 절 추가.
- S3: manifest의 `=` 고정 20개(workspace 16, viewer 4)를 `^` 범위로. lockfile의 의존성 버전은 그대로.
- S4: 회귀 [EV-114](../../05_verification/verification_evidence.md#ev-114). 같은 기간 T3 두 번째 실행에서 드러난 macOS RT 할당 시험 3개는 처음에 Tier 1 계약으로 보고 macOS에서만 ignore했다.
- S5(2026-11-21 Human 「RT 경로가 아니더라도 시작 시가 아니라면 다른 잠금을 고려해야」 → 「A로 진행」): ignore 대신 잠금 예열로 바꿨다. core의 std `Mutex` 9개(telemetry history·health·FileLedger·설정 변경·tracing sink·facade writers·remote hub 2·receiver)는 생성 시 한 번 잠긴다. 바이트 예약에 들어가는 3곳(telemetry Bank, 설정 변경, FileLedger)은 `MUTEX_BYTES`를 더한다. Linux·Windows는 할당이 없어 0이다. 다른 잠금으로 바꾸는 안은 `unsafe_code = forbid`와 새 의존성(spin)·Linux RT에서의 스핀 단점 때문에 택하지 않았다. 규칙은 coding_convention에 추가.


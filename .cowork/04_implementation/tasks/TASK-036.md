# TASK-036 — 배포 가능화(0단계)

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-010 |
| Story | US-001·003 |
| 요구사항 | FR-001 NFR-004 |
| 설계 | [ADR-010 개정 2](../../03_design_artifacts/adrs/ADR-010_mcap_security_update.md#개정-2--배포-가능한-vendor-구조2026-11-21) · [ADR-009 개정](../../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md#개정--라이브러리-의존성-버전-범위2026-11-21) |
| ADR | ADR-009·010 개정(2026-11-21 Human 결정) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-21 |
| 다음 액션 | S1부터. 워크스페이스 밖 소비자 빌드 실패를 먼저 시험으로 재현 |
| 의존 | 없음(T3 native CI와 병행 가능) |
| 예상 소요 | 1~2 인일 |

## 배경

2026-11-21 워크스페이스 밖 임시 소비자 프로젝트(`dot-logger = { path = … }`)가 `WriteOptions::chunk_registrations` 없음(`E0599`)으로 컴파일되지 않았다. 원인은 패치한 MCAP를 워크스페이스 `[patch]`로만 적용한 구조다. README가 안내하는 git 의존성도 같은 이유로 동작하지 않는다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 재현 시험 | 워크스페이스 밖 소비자 빌드 검사(스크립트와 CI 단계)를 추가해 현재 실패를 확인 | Planned |
| S2 | MCAP 별칭 crate | vendor를 이름을 바꾼 워크스페이스 crate로 옮기고 `package =` 별칭으로 의존, `[patch]` 제거. VENDORED.md·라이선스·패치 파일 정합. S1 통과 | Planned |
| S3 | 의존성 범위 | 라이브러리 crate 의존성을 `^` 범위로(하한 = 검증 버전·보안 하한). lock 유지, 하한 확인 방법 결정 | Planned |
| S4 | 문서·회귀 | README 설치 안내, tech_stack, 두 Rust 회귀·feature 조합·Miri·교차 Clippy | Planned |

## 완료 기준

- [ ] 워크스페이스 밖 소비자가 git·path 의존성으로 빌드된다(CI로 확인).
- [ ] 기존 회귀와 출력 바이트(MCAP CRC 포함)가 바뀌지 않는다.
- [ ] Human 리뷰.

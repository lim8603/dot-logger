# TASK-041 — v0.1.1 패치 릴리즈

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-010 |
| Story | US-001·003 |
| 요구사항 | NFR-004 NFR-007 |
| 설계 | [릴리즈 노트 §버전·호환 정책](../../07_delivery/release_note.md#버전호환-정책) · [MS-010](../milestones/MS-010.md) |
| ADR | ADR-009 개정 2 |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-28 |
| 다음 액션 | S2 릴리즈 커밋 CI Pass 확인 → Human 확인 뒤 S3 태그·pre-release |
| 의존 | TASK-037 |
| 예상 소요 | 0.5 인일 |

## 배경

v0.1.0 미리보기는 macOS·Windows에서 facade 파일 출력이 `start()`부터 `Unsupported`로 실패한다(EV-119에서 발견, [ADR-009 개정 2](../../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md)로 `9f687dd`·`ad11ba3`에서 수정). 2026-11-22 Human 결정 「v0.1.1 때 함께 정리」. 24h soak 결과(EV-120)도 검증 범위에 넣는다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 공개 문서 | `CHANGELOG.md` 0.1.1 절(수정·검증 범위·v0.1.0 알려진 이슈), README 설치 태그·검증 절, 릴리즈 노트 | Done(2026-11-28) — 아래 §S1 결과 |
| S2 | 릴리즈 커밋 CI | 전체 CI Pass | In Progress — 로컬 회귀 Pass, CI 대기 |
| S3 | 태그·pre-release | 주석 태그 `v0.1.1`, GitHub pre-release. 외부 공개 동작이라 Human 확인 뒤 실행 | Planned |

## S1 결과(2026-11-28)

- `CHANGELOG.md`: Unreleased를 `[0.1.1] — 2026-11-28 (preview)`로 옮기고 설치 태그, Fixed(macOS 잠금 바이트 회계·MCAP 헤더 버전), Security(`url` 2.5.9, RUSTSEC-2026-0592), Verified(약 750 시험·3 OS, 파일 교환, 24h EV-120, 4시간 혼합 EV-126, 장애 주입 EV-123), Known issues(실사용처·keepalive·`IoPoisoned` 고착·InfluxDB 3 조회, 나머지는 0.1.0과 같음)를 더했다.
- README: 설치 태그 `v0.1.1`, 검증 절 시험 수 약 750·4시간 혼합·장애 주입 줄.
- 버전: `workspace.package.version`·vendor 별칭 crate `dot-logger-mcap`과 그 의존 선언 0.1.0 → 0.1.1, `Cargo.lock`(`cargo update --workspace`, 워크스페이스 crate 7개만).
- 범위 안 수정(A): MCAP 헤더 `library`가 `"dot-logger/0.1.0"` 고정 문자열이던 것을 `concat!("dot-logger/", env!("CARGO_PKG_VERSION"))`로. 버전을 올리며 발견.
- 릴리즈 노트(한국어): v0.1.1 절(R12~14·I5·I6·F1~F3·S1·K1'~K11).
- 로컬 회귀(Rust 1.96.0, all-features): fmt·Clippy(`-D warnings`) Pass, 750 Pass·실패 0·5 ignored(EV-133과 같음).

## 완료 기준

- [ ] 태그 커밋 CI Pass, 태그 소비자 빌드 확인, Human 확인.

# TASK-037 — v0.1.0 미리보기 릴리즈(1단계)

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-010 |
| Story | US-001·003 |
| 요구사항 | NFR-004 |
| 설계 | [릴리즈 노트 §버전·호환 정책](../../07_delivery/release_note.md#버전호환-정책) · [MS-010](../milestones/MS-010.md) |
| ADR | 없음(MS-010 계획, 2026-11-21 Human 결정 3 「미리보기」) |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-22 |
| 다음 액션 | 없음 — MS-010 2단계(v0.2.0)는 TASK-026 뒤 |
| 의존 | TASK-036 Done, T3 native CI 통과(2026-11-22 충족) |
| 예상 소요 | 0.5 인일 |

## 배경

MS-010 1단계. Gate 5 전이므로 Gate 5 예외(2026-11-21 Human 결정 3)로 GitHub 미리보기를 낸다. 노트에 검증 범위(이 PC Linux 로컬, 3 OS native CI, 24h·실사용처 미실행)를 적는다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 공개 문서 | `CHANGELOG.md`(영어) 작성, README의 설치(`tag = "v0.1.0"`)·검증 절 최신화, 릴리즈 노트 알려진 이슈 갱신 | Done — `f124f19`, 날짜 확정은 태그 커밋 |
| S2 | 릴리즈 커밋 CI | S1 커밋에서 `Rust contracts` 9개 작업 Pass | Done — run `44966993272`([EV-117](../../05_verification/verification_evidence.md#ev-117)) |
| S3 | 태그·pre-release | 주석 태그 `v0.1.0` 푸시, GitHub Release(pre-release, 본문 = CHANGELOG v0.1.0 절). 외부 공개 동작이라 Human 확인 뒤 실행 | Done — 2026-11-22 Human 「CI 통과하면 진행해」, 태그 `e870bc1`, [pre-release](https://github.com/lim8603/dot-logger/releases/tag/v0.1.0) |

## 완료 기준

- [x] `CHANGELOG.md`·README·릴리즈 노트가 v0.1.0 범위와 검증 범위를 사실대로 적는다.
- [x] 태그 커밋의 CI가 통과한다(코드는 `f124f19` run `44966993272`와 같고 태그 커밋은 문서만 변경, EV-117).
- [x] 워크스페이스 밖 소비자가 `tag = "v0.1.0"` git 의존으로 빌드된다(EV-117).
- [x] Human 확인 — 2026-11-22 「CI 통과하면 진행해」.

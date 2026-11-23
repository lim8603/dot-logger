# TASK-041 — v0.1.1 패치 릴리즈

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-010 |
| Story | US-001·003 |
| 요구사항 | NFR-004 NFR-007 |
| 설계 | [릴리즈 노트 §버전·호환 정책](../../07_delivery/release_note.md#버전호환-정책) · [MS-010](../milestones/MS-010.md) |
| ADR | ADR-009 개정 2 |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-23 |
| 다음 액션 | TASK-038·039 뒤 CHANGELOG 정리 → CI → Human 확인 뒤 태그 |
| 의존 | TASK-037 |
| 예상 소요 | 0.5 인일 |

## 배경

v0.1.0 미리보기는 macOS·Windows에서 facade 파일 출력이 `start()`부터 `Unsupported`로 실패한다(EV-119에서 발견, [ADR-009 개정 2](../../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md)로 `9f687dd`·`ad11ba3`에서 수정). 2026-11-22 Human 결정 「v0.1.1 때 함께 정리」. 24h soak 결과(EV-120)도 검증 범위에 넣는다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 공개 문서 | `CHANGELOG.md` 0.1.1 절(수정·검증 범위·v0.1.0 알려진 이슈), README 설치 태그·검증 절, 릴리즈 노트 | Planned |
| S2 | 릴리즈 커밋 CI | 전체 CI Pass | Planned |
| S3 | 태그·pre-release | 주석 태그 `v0.1.1`, GitHub pre-release. 외부 공개 동작이라 Human 확인 뒤 실행 | Planned |

## 완료 기준

- [ ] 태그 커밋 CI Pass, 태그 소비자 빌드 확인, Human 확인.

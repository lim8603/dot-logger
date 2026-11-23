# TASK-038 — calibrate 여유 공간 시험 간헐 실패 견고화

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-004 |
| 요구사항 | FR-016 |
| 설계 | [API §19.8~19.9](../../03_design_artifacts/interface_contract.md) |
| ADR | 없음 |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-23 |
| 다음 액션 | 수정 전 재현 → 수정 뒤 반복 실행 Pass → 커밋·CI |
| 의존 | 없음 |
| 예상 소요 | 0.25 인일 |

## 배경

2026-11-22 run `44987267000`의 `contracts (ubuntu-24.04, 1.85.0)`에서 `tc_013_calibrate::f19_reserve_shrinks_the_write_limit`가 실패했고 재실행은 Pass였다([EV-119](../../05_verification/verification_evidence.md#ev-119)). 2026-11-23 Human이 작은 일을 Task로 올리고 이것부터 진행하라고 지시했다.

원인: 시험이 여유 공간을 잰 뒤 `Calibration::run`이 다시 재는 사이에 같은 바이너리의 병렬 시험(probe 쓰기 3~8 MiB, logger 시작)이 공간을 바꾸면 남는 공간 `2 MiB + Δ`가 흔들린다. Δ > 0이면 `bytes ≤ 2 MiB` 단정이, Δ ≤ −2 MiB이면 `NoSpace`로 실패한다. 제품 코드 결함이 아니라 시험 격리 문제다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 시험 격리 | 디스크에 쓰는 시험을 한 잠금으로 순서화, 남는 공간 16 MiB·상한 24 MiB로 바깥 프로세스 변동 여유 | 코드 작성 |
| S2 | 검증 | 수정 전 코드로 병렬 반복 실행에서 실패 재현, 수정 뒤 같은 반복에서 0회 실패, 1.85·1.96 시험 Pass, CI Pass | Planned |

## 완료 기준

- [ ] 수정 전 재현과 수정 뒤 반복 Pass를 EV에 기록한다.
- [ ] CI 전체 Pass.

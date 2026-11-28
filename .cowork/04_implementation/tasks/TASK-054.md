# TASK-054 — MS-013 단계 6: DB 출력 설계 게이트

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-013](../milestones/MS-013.md) 단계 6 |
| Story | US-002·003 |
| 요구사항 | FR-019b(게이트 전 미수락), FR-004 |
| 설계 | MS-013 §5.7, [MS-012](../milestones/MS-012.md) 결과(EV-132~135) |
| ADR | 게이트에서 새 ADR 여부 판단(ADR-017의 출력 모델 확장) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-28 |
| 다음 액션 | S1 — TASK-050 뒤 언제든(구현 단계와 독립), 기본은 TASK-053 뒤 |
| 의존 | TASK-050, [TASK-044](TASK-044.md) 결정(Record → 필드 매핑) |
| 예상 소요 | 설계 1~2 인일. 구현은 게이트 뒤 별도 Task로 채번 |

## 목표

facade DB 출력을 0.2에 넣을지와 그 계약을 정한다. 이 Task는 **설계 게이트만** 맡는다. 구현 Task는 게이트를 통과한 뒤 채번한다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 게이트 설계안 | §5.7 항목: Record → DB 필드 매핑(TASK-044와 함께), 배치 거절·부분 ACK의 완료 보고, 출력 역할(Required/BestEffort), 파일 없는 DB 전용 구성의 종료, `DatabaseAcknowledged` ≠ 내구성 표기. MS-013 §5.8 DB 행 확정안 | Planned |
| S2 | 리뷰·승인 | 독립 설계 리뷰 → Human 승인(H) → FR-019b 수락 기준과 구현 Task 등록 | Planned |

## Human 확인 필요 사항

- 0.2에 넣을지, 다음 버전으로 미룰지(H).

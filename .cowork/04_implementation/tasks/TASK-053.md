# TASK-053 — MS-013 단계 5: 무손실 최대 처리량 측정·공개, H6 확정

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-013](../milestones/MS-013.md) 단계 5 |
| Story | US-001·004 |
| 요구사항 | NFR-009, H6 — [TC-015](../../05_verification/test_case.md#tc-015--출력-구성네트워크-전용전송-스위치) 5단계 |
| 설계 | MS-013 §8.2 |
| ADR | [ADR-017](../../03_design_artifacts/adrs/ADR-017_output_model_copy_observer.md) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-28 |
| 다음 액션 | S1 — TASK-052 Done 뒤 |
| 의존 | TASK-052 |
| 예상 소요 | 2~2.5 인일 + 실행 1~2시간 |

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 측정 하네스 | 공통 관측점 4개(legacy tap·새 경로), 새 경로 관측점(L1 `observed`·L3 `offered`·링·구독 큐 길이), 무손실 판정 자동화(손실 계수 0·큐 60 s 기울기 ≤ 0), 기록당 비용 분리. 짧은 시험 실행으로 하네스 검증 | Planned |
| S2 | 실행 | 기록 크기 64 B·256 B·4 KiB·64 KiB·1 MiB, 루프백(1 GbE는 장비가 있을 때), H6 확정 판정(0.8P·포화 감소 ≤ 10%). SSD 쓰기 0(tmpfs), **1~2시간 — 시작 전 시간·쓰기량 알림** | Planned |
| S3 | 공개·마감 | README 처리량 표와 조건(구독 수·ACK 창·payload·링크·TLS·CPU·affinity·커널·EV-130 잡음), EV, NFR-009 수락 | Planned |

## 완료 기준

- [ ] 크기별 무손실 최고 속도와 조건 표를 README에 공개, EV.
- [ ] H6 확정 판정을 기록(탈락하면 Human 보고).

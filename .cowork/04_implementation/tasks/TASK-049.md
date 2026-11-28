# TASK-049 — MS-013 단계 1: core 복사 observer와 내부 출력 모델

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-013](../milestones/MS-013.md) 단계 1 |
| Story | US-001·003 |
| 요구사항 | FR-019a·021(core 기반), H8 — [TC-015](../../05_verification/test_case.md#tc-015--출력-구성네트워크-전용전송-스위치) 1단계 |
| 설계 | MS-013 §5.1·§6(core `Route.observe`)·§7 단계 1·§8.1 격리·H8 행 |
| ADR | [ADR-017](../../03_design_artifacts/adrs/ADR-017_output_model_copy_observer.md) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-28 |
| 다음 액션 | S1 — TASK-048 go 판정 뒤 |
| 의존 | TASK-048(H6 go) |
| 예상 소요 | 3~4 인일(TASK-048 S6에서 재산정) |

## 목표

core에 복사 observer(`Route.observe`, observer 링)를 넣고 내부 출력 모델을 정리한다. 공개 변경은 core `Route` 필드 추가 하나다(breaking). 기존 facade 사용은 바꾸지 않는다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 계약·Red | [API](../../03_design_artifacts/interface_contract.md) §25.1(core observer·링·경우표)을 쓰고, observer 내부 시험과 경우표 ①~⑤ 시험이 Red | Planned |
| S2 | 내부 출력 모델 | 출력 = 이름 + 종류 내부 모델로 정리(공개 변경 없음). 골든 + 기존 facade·tc_013·레시피 **수정 없이** Pass | Planned |
| S3 | observer 복사 | `Route.observe`, SPSC 링(payload 영역 + 제어 영역 자리), dispatcher 같은 분배 안 복사, 복사 예산 드롭·시작 검증(`copy_bytes_per_turn ≥ 최대 기록`), 스위치 원자 읽기(내부 플래그, API는 단계 4), observer-only 셀 0·core 이벤트 0. 경우표 시험 Green | Planned |
| S4 | 격리 시험 | 시험 빌드 계측으로 최대 보유 턴 0. raw 2 슬롯·Important·파일 포화에서 「구독 정상」과 「정지」의 파일 완료·RT `Capacity`·raw 반환 턴 수 동일 | Planned |
| S5 | 마감·단독 리뷰 | 외부 소비자 검증(`check-external-consumer.py`)·dot-probe 빌드 영향, facade 공개 표면 불변, 두 Rust 회귀·Clippy·feature·Miri·CI Pass, EV. **단계 1 단독 독립 리뷰** 요청·반입 | Planned |

## 완료 기준

- [ ] 골든·기존 facade 회귀가 수정 없이 Pass.
- [ ] 경우표·격리 시험 Pass, 외부 소비자 빌드 통과.
- [ ] 독립 리뷰 Pass(지적 조치 포함), EV.

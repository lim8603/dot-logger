# TASK-048 — MS-013 단계 0: 레거시 고정·spike·H6 1차 판정

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-013](../milestones/MS-013.md) 단계 0 |
| Story | US-001·003·004 |
| 요구사항 | FR-019a·021·022·023(조기 확인), NFR-009(H6) — [TC-015](../../05_verification/test_case.md#tc-015--출력-구성네트워크-전용전송-스위치) 0단계 |
| 설계 | MS-013 §5.1(경우표)·§5.6(명령 결과표)·§5.9(종료 판정표)·§7 단계 0·§8.2 H6 프로필 |
| ADR | [ADR-017](../../03_design_artifacts/adrs/ADR-017_output_model_copy_observer.md)·[ADR-019](../../03_design_artifacts/adrs/ADR-019_transmission_switch_wire.md) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-28 |
| 다음 액션 | S1 — TASK-041 v0.1.1 뒤 착수. 레거시 골든 |
| 의존 | TASK-041(Q1) |
| 예상 소요 | 3~4 인일 |

## 목표

리팩토링 전에 지금 동작을 골든으로 고정한다. 구현으로만 확인할 수 있는 설계 가정은 **제품 코드 밖 spike**로 먼저 확인한다. 복사 observer가 파일에 주는 영향(H6)을 1차 판정해, 단계 1에 들어갈지(go/no-go) 정한다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 레거시 골든 | 대표 구성의 `Plan` 산식, 출력 ID·sync/barrier 순서, 부분 쓰기 tap prefix, 시작 실패 정리, 파일 내구성을 고정하는 시험이 지금 코드에서 두 Rust Pass. 단계 1·2는 이 시험을 **수정하지 않고** 통과해야 한다 | Planned |
| S2 | spike ①②⑤ — observer 복사·예산·경우표 | 제품 코드 밖(spike 브랜치, 병합 안 함)에서 확인: dispatcher 분배 안 복사의 `SharedPayload` 수명 0턴, tracked target 0개 구성의 completion·sync·loss bank 동작(K12), 복사 예산 드롭, §5.1 경우표 ①~⑤(혼합 구성의 ② 채널 불변 포함). 판정을 EV에 기록 | Planned |
| S3 | spike ③ — 명령 CAS·제어 영역 | §5.6 명령 결과표 경로별 결과 하나, CAS 직후 host 정지 → 기한 + 100 ms 반환·재개 뒤 효과 1회, 커밋 전 host 종료 → `WorkerFailed`·효과 0, epoch 게시 전후 host 종료 → `HostFailed`/`Applied`, IO panic(`worker_failed`만) 중 `Committed` → `Applied` | Planned |
| S4 | spike ④ — 호출자 감독 종료 | §7 단계 0 ④ 목록 전부(무보고 host → `Unresponsive`, `finish` 실패·지연, `finish` 대기 중 host 기한 보고 1회, 봉인 잠금 실패 턴, 발행 셀 기록 전 IO 정지, 네트워크 스레드 panic의 host 정산, 회수 지연, 모드 4종의 `Closed` 조건과 보고 값) | Planned |
| S5 | H6 1차 판정·기준선 | §8.2 H6 프로필로 P, 0.8P, 포화 감소 측정(tmpfs, SSD 쓰기 0, 30분 이내). Windows·macOS 새 스레드 대기 방식의 깨우기 측정. legacy tap 처리량 기준선. **go/no-go를 Human에게 보고**(탈락하면 단계 1 전에 설계 재검토 → 정정 ADR) | Planned |
| S6 | 마감·재산정 | spike 결과로 단계 1~5 인일·스텝을 다시 산정해 MS-013·TASK-049~053 갱신. 골든 커밋, 두 Rust 회귀·Clippy·CI Pass, EV, 자가검토 | Planned |

## Human 확인 필요 사항

- S5 H6 판정이 탈락이거나 경계에 걸리면 진행 여부를 결정한다(H).
- 실행이 30분이나 쓰기량 예상을 넘으면 먼저 알리고 승인을 받는다.

## 완료 기준

- [ ] 골든이 커밋되고 현재 코드에서 Pass.
- [ ] spike ①~⑤ 판정과 H6 1차 판정을 EV에 기록. spike 코드는 제품 브랜치에 병합하지 않는다.
- [ ] 재산정한 일정이 MS-013에 반영.

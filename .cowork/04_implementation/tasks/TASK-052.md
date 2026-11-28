# TASK-052 — MS-013 단계 4: 전송 스위치·표시·heartbeat·End frame

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-013](../milestones/MS-013.md) 단계 4 |
| Story | US-003·004 |
| 요구사항 | FR-023 — [TC-015](../../05_verification/test_case.md#tc-015--출력-구성네트워크-전용전송-스위치) 4단계 |
| 설계 | MS-013 §5.6·§6 wire·client 행·§8.1 FR-023 행 |
| ADR | [ADR-019](../../03_design_artifacts/adrs/ADR-019_transmission_switch_wire.md) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-28 |
| 다음 액션 | S1 — TASK-051 Done 뒤 |
| 의존 | TASK-050(필수)·TASK-051(순서) |
| 예상 소요 | 4~5 인일 |

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 스위치 API·명령 셀 | API §25.4. `set_output_enabled`·`output_state`·`switch_status`, `SwitchReceipt`·`SwitchError`·`CommandId`·`SwitchStatus`, 명령 셀 CAS·명령 자리 5·이력 8, epoch 마지막 게시, `host_exited` 종결. 명령 결과표 경로별 시험 Green | Planned |
| S2 | 링 제어 영역·hub fence | 제어 영역 K = 4·일련번호 공유, 표시 fence(Gap 먼저 닫기·멀티 chunk 사이 없음), 구독 가입 fence·초기 표시, 구독 큐 제어 자리·`MarkerOverflow`, 유휴 heartbeat(H3) | Planned |
| S3 | wire | 구독 요청 `epoch_markers`·응답 지원 표시, 새 frame 3종, End 수치, End·ACK·EOF 순서, 최악 표시 인코딩 길이 시작 검증. `check-remote-protocol.py` 갱신 | Planned |
| S4 | client | `epoch_markers(..)`, `RemoteEvent::Epoch`·`End`, `EndReason`, ACK 실패 원인 보관, 구독 상태 `Open`/`AckGone`/`Ended`/`Failed`(`next`·`handoff`·`flush`·`acknowledged`), `ack_unknown` 확정 경계, `AckGone` 기한·호출 기한. §5.6 wire 시험 목록 Green | Planned |
| S5 | 호환·마감 | wire 호환 행렬(옛·새 client × 옛·새 서버 × Off/Preferred/Required), 외부 소비자 검증, 공개 표면 gate, CHANGELOG, 두 Rust 회귀·Clippy·feature·Miri·CI, EV, 독립 리뷰 | Planned |

## Human 확인 필요 사항

- 이 Task가 끝나면 0.2.0-alpha(「DB 조합 미포함」)를 낼지 MS-010과 함께 결정한다(H).

## 완료 기준

- [ ] FR-023 수락 시험·wire 호환 행렬 Pass.
- [ ] 표면 gate·외부 소비자·CI Pass, 독립 리뷰 Pass, EV.

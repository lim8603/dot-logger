# REV-078 리뷰 요청 — MS-013 r5 재리뷰(REV-077 반영 대조, 5회차)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-27 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지, **MS-013도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) **r5**(Draft, REV-077 6건 반영). 코드 변경 없음 — 설계 재리뷰 |
| 기준 | 이 요청서와 MS-013 r5를 넣은 `main` 커밋(`git log -1 -- .cowork/04_implementation/milestones/MS-013.md`). r4 `3901fa3`, r3 `cda0161`, r2 `d9c2da7`, r1 `da5f6c9` |
| 결과물 | 이 폴더에 `REV-078_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-251`부터. 형식은 [REV-077](REV-077_MS013_R4_DESIGN_REVIEW.md)과 같게 |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-078을 등록한다 |
| 반복 규칙 | 2026-11-27 Human 「지적이 없을때까지 리뷰 요청하자」: 이전 지적이 모두 해소되고 새 지적이 0건인 회차까지 반복. 0건이면 종합 판정 Pass |

## 1. 무엇을 보나

1. **REV-077 대조:** RV-245~250 각각이 처리담당 기록과 MS-013 §14가 가리키는 r5 본문으로 해소됐는지. REV-074~076 해소 상태가 퇴행하지 않았는지.
2. **r5에서 바뀐 계약이 새로 만든 문제:** 「r5에서 바뀐 것」 5항목이 대상이다.
   - §5.6 스위치: CAS 단일 커밋 + 100 ms 유예 + `SwitchError::Pending`·`switch_status`가 반환 상한과 「정확히 1회」를 함께 보장하는지, 같은 상태 요청을 CAS 없이 응답하는 것이 취소와 경합할 때 결과가 일관된지.
   - §5.4 정산: 단조 위치(`Progress.current`·링 읽기 위치·hub `last_offered_seq`)와 hub 봉인으로 모든 중단 지점에서 정확히 한 번·멱등 정산이 되는지, 누적식·순간값이 맞물리는지.
   - §5.9 종료: `FilesClosed`(finish 결과 포함)·`Unresponsive`·`finalized = false` 경로가 기존 공개 타입과 일관된지.
   - H8 개정·§5.1: observer-only를 core 집계에서 제외하는 것이 손실 보고·`.logger/loss`·종료 판정과 맞는지, target 0 완화 범위.
   - §5.6 End·ACK·EOF 순서표와 표시 packed `bytes`·실제 인코딩 검증.
3. **수렴 기준(REV-077 §4를 따름):** 공개 계약·상태 전이·정산식의 공백만 지적하고, Rust 원자 순서·ring 구현·처리량·깨우기 방식·허용치 충족은 골든과 단계 0 spike 판정으로 남긴다. 계약 공백이 없으면 **지적 0건, 종합 판정 Pass**로 반복을 끝낸다.

## 2. 읽을 것

- 대상: MS-013 r5(「r5에서 바뀐 것」과 §14에서 시작, 부록 A 제외).
- 이전 리뷰: REV-074·075·076·077.
- 대조용 코드: REV-076 요청서 §2 목록과 같음. 특히 `crates/dot-logger/src/routing/{mod.rs, types.rs}`(분배·`SharedPayload`·`Route`·`Unrouted`), `crates/dot-logger/src/completion/mod.rs`(`Unrouted` 처리), `crates/dot-logger/src/loss.rs`(`NoRoute`·`IntentionalSkip`), `crates/dot-logger/src/adapters/remote/hub.rs`(구독 FIFO·gap·ACK), `crates/dot-logger/src/logger/{mod.rs, host.rs}`(종료).

## 3. 범위 밖

- 제품 코드 수정·시험 실행, 성능 재측정.
- H1·H3~H8·Q1~Q8의 최종 결정(Human 몫, 의견 환영).

## 4. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
git diff 3901fa3 -- .cowork/04_implementation/milestones/MS-013.md
grep -n "Unrouted\|SharedPayload\|fn dispatch\|pub struct Route" crates/dot-logger/src/routing/*.rs | head -30
grep -n "NoRoute\|IntentionalSkip" crates/dot-logger/src/loss.rs | head
```

빌드·시험은 필요할 때만 `CARGO_TARGET_DIR=/data/build/target`으로, 장시간 실행은 하지 않는다.

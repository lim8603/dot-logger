# REV-080 리뷰 요청 — MS-013 r7 재리뷰(REV-079 반영 대조, 7회차)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-28 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지, **MS-013도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) **r7**(Draft, REV-079 5건 반영). 코드 변경 없음 — 설계 재리뷰 |
| 기준 | 이 요청서와 MS-013 r7을 넣은 `main` 커밋(`git log -1 -- .cowork/04_implementation/milestones/MS-013.md`). r6 `7fe024c`, r5 `fe0f8d5`, r4 `3901fa3`, r3 `cda0161`, r2 `d9c2da7`, r1 `da5f6c9` |
| 결과물 | 이 폴더에 `REV-080_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-261`부터. 형식은 [REV-079](REV-079_MS013_R6_DESIGN_REVIEW.md)와 같게 |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-080을 등록한다 |
| 반복 규칙 | 2026-11-27 Human 「지적이 없을때까지 리뷰 요청하자」: 이전 지적이 모두 해소되고 새 지적이 0건인 회차까지 반복. 0건이면 종합 판정 Pass |

## 1. 무엇을 보나

1. **REV-079 대조:** RV-256~260 각각이 처리담당 기록과 MS-013 §16이 가리키는 r7 본문으로 해소됐는지. REV-074~078 해소 상태가 퇴행하지 않았는지.
2. **r7에서 바뀐 계약이 새로 만든 문제:** 「r7에서 바뀐 것」 5항목이 대상이다.
   - §5.6 명령 결과표의 host 종료 관측 두 행: `Pending → Rejected(HostFailed)` = `WorkerFailed`(미적용 보장), `Committed { epoch }` → epoch 게시 여부로 `Applied`/`HostFailed`. ⑥ 안의 순서(epoch 게시가 마지막)와 「`output_state().epoch ≥ e` ⇔ 게시 완료」가 명령 직렬 처리·이력 교체와 맞는지, 반환 상한(기한 + 100 ms)과 맞는지.
   - §5.4 `offer_pending`: L3 식이 항목 시작·offer·항목 끝·봉인 위치 정산의 모든 정상 스냅샷에서 성립하는지, 최종 0과 `finalized` 조건이 그대로인지.
   - §5.9 두 판정표(`shutdown()`·`wait(timeout)`): 수신 상한 기한 + `REPORT_GRACE`, 호출자 보고 2종(기한 보고·`Closed`)과 host 내부 이벤트(`FilesClosed`·`NetworkFinalized`)의 분리, `finish` 대기 중 host 기한 감시, 봉인 `try_lock` 실패 턴이 기존 `PendingShutdown::wait` 의미·`files_late`와 일관된지.
   - §5.6 End·ACK 오류 선착 순서표와 `ack_unknown` 확정 경계(End 수신 시점): End 수치 표·wire 시험 기대값과 서로 맞는지, 지금 client `next()`의 reap 순서와 맞는지.
   - §5.1 route 경우표의 판정 단위(①은 구성, ②·④·⑤는 기록의 route revision): 혼합 구성의 빈 route 채널이 기존 Router·completion 동작과 맞는지.
3. **수렴 기준(REV-077 §4·REV-078 §4·REV-079를 따름):** 호출자가 관측하는 결과·공개 계약·상태 전이·정산식의 공백만 지적하고, Rust 원자 순서·ring 구현·처리량·깨우기 방식·허용치 충족은 골든과 단계 0 spike 판정으로 남긴다. 계약 공백이 없으면 **지적 0건, 종합 판정 Pass**로 반복을 끝낸다.

## 2. 읽을 것

- 대상: MS-013 r7(「r7에서 바뀐 것」과 §16에서 시작, 부록 A 제외).
- 이전 리뷰: REV-074·075·076·077·078·079.
- 대조용 코드: REV-076 요청서 §2 목록과 같음. 이번 회차에 특히 `crates/dot-logger/src/logger/{mod.rs, host.rs}`(`REPORT_GRACE`·`PendingShutdown::wait`·기한 보고·core Closed 뒤 IO join), `crates/dot-logger-client/src/transport.rs`(`next`·`next_inner`의 reap 순서, `unknown()`), `crates/dot-logger/src/adapters/remote/hub.rs`(`ack_batch`), `crates/dot-logger/src/routing/mod.rs`·`completion/mod.rs`, `crates/dot-logger/tests/tc_003_routing.rs`.

## 3. 범위 밖

- 제품 코드 수정·시험 실행, 성능 재측정.
- H1·H3~H8·Q1~Q8의 최종 결정(Human 몫, 의견 환영).

## 4. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
git diff 7fe024c -- .cowork/04_implementation/milestones/MS-013.md
grep -n "REPORT_GRACE\|Progress::Incomplete\|fn wait" crates/dot-logger/src/logger/*.rs
grep -n "fn next\|fn next_inner\|fn reap\|fn unknown" crates/dot-logger-client/src/transport.rs
```

빌드·시험은 필요할 때만 `CARGO_TARGET_DIR=/data/build/target`으로, 장시간 실행은 하지 않는다.

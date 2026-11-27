# REV-079 리뷰 요청 — MS-013 r6 재리뷰(REV-078 반영 대조, 6회차)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-27 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지, **MS-013도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) **r6**(Draft, REV-078 5건 반영). 코드 변경 없음 — 설계 재리뷰 |
| 기준 | 이 요청서와 MS-013 r6을 넣은 `main` 커밋(`git log -1 -- .cowork/04_implementation/milestones/MS-013.md`). r5 `fe0f8d5`, r4 `3901fa3`, r3 `cda0161`, r2 `d9c2da7`, r1 `da5f6c9` |
| 결과물 | 이 폴더에 `REV-079_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-256`부터. 형식은 [REV-078](REV-078_MS013_R5_DESIGN_REVIEW.md)과 같게 |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-079를 등록한다 |
| 반복 규칙 | 2026-11-27 Human 「지적이 없을때까지 리뷰 요청하자」: 이전 지적이 모두 해소되고 새 지적이 0건인 회차까지 반복. 0건이면 종합 판정 Pass |

## 1. 무엇을 보나

1. **REV-078 대조:** RV-251~255 각각이 처리담당 기록과 MS-013 §15가 가리키는 r6 본문으로 해소됐는지. REV-074~077 해소 상태가 퇴행하지 않았는지.
2. **r6에서 바뀐 계약이 새로 만든 문제:** 「r6에서 바뀐 것」 5항목이 대상이다.
   - §5.6 명령 결과표: 모든 경로가 `Pending`에서 나가는 CAS 하나로 결과 하나를 내는지, `Committed`·`HostFailed`·`Unknown`의 의미와 미종결 자리 5개·종결 이력 8개 보관이 반환 상한(기한 + 100 ms)과 맞는지.
   - §5.4 구독별 정산 상태(`Progress.entries`)와 감독 정산(생산 닫기 → 봉인 + 종결 → 위치 정산 → 게시): 모든 중단 지점에서 누적식이 성립하는지, `finalized` 조건이 L1까지 고정된 뒤에만 참인지.
   - §5.9 판정표: 기존 host 기한 보고(`Progress::Incomplete`)·`Unresponsive`·`pending.wait()`와 `files_late`·`files_complete()`·`is_success()`가 일관된지.
   - §5.6 End 수치(서버 관점 `cut`·`unacked`)와 client `ack_unknown` 분리, ACK `Gone` 원인 보관과 End 우선 읽기 규칙.
   - §5.1 route 경우표 ①~⑤가 기존 Router·completion·facade 동작(고급 Router 채널별 빈 route, facade 빈 선택)과 맞고 다른 절과 모순이 없는지.
3. **수렴 기준(REV-077 §4·REV-078 §4를 따름):** 호출자가 관측하는 결과·공개 계약·상태 전이·정산식의 공백만 지적하고, Rust 원자 순서·ring 구현·처리량·깨우기 방식·허용치 충족은 골든과 단계 0 spike 판정으로 남긴다. 계약 공백이 없으면 **지적 0건, 종합 판정 Pass**로 반복을 끝낸다.

## 2. 읽을 것

- 대상: MS-013 r6(「r6에서 바뀐 것」과 §15에서 시작, 부록 A 제외).
- 이전 리뷰: REV-074·075·076·077·078.
- 대조용 코드: REV-076 요청서 §2 목록과 같음. 이번 회차에 특히 `crates/dot-logger/src/logger/{mod.rs, host.rs}`(`Shutdown`·`PendingShutdown`·기한 보고 `Progress::Incomplete`·`FacadeReport::is_success`), `crates/dot-logger/src/control/types.rs`(`ShutdownReport::is_success`), `crates/dot-logger/src/adapters/remote/hub.rs`(offer·`ack_batch`), `crates/dot-logger-client/src/transport.rs`(ACK RPC 실패 처리), `crates/dot-logger/src/routing/mod.rs`·`completion/mod.rs`(`Unrouted`·`NoRoute`), `crates/dot-logger/tests/tc_003_routing.rs`(빈 route 시험).

## 3. 범위 밖

- 제품 코드 수정·시험 실행, 성능 재측정.
- H1·H3~H8·Q1~Q8의 최종 결정(Human 몫, 의견 환영).

## 4. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
git diff fe0f8d5 -- .cowork/04_implementation/milestones/MS-013.md
grep -n "Progress::Incomplete\|REPORT_GRACE\|fn is_success" crates/dot-logger/src/logger/*.rs crates/dot-logger/src/control/types.rs
grep -n "fn ack_batch\|fn offer" crates/dot-logger/src/adapters/remote/hub.rs
```

빌드·시험은 필요할 때만 `CARGO_TARGET_DIR=/data/build/target`으로, 장시간 실행은 하지 않는다.

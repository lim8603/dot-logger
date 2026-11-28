# REV-081 리뷰 요청 — MS-013 r8 재리뷰(REV-080 반영 대조, 8회차)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-28 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지, **MS-013도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) **r8**(Draft, REV-080 4건 반영). 코드 변경 없음 — 설계 재리뷰 |
| 기준 | 이 요청서와 MS-013 r8을 넣은 `main` 커밋(`git log -1 -- .cowork/04_implementation/milestones/MS-013.md`). r7 `405cf39`, r6 `7fe024c`, r5 `fe0f8d5`, r4 `3901fa3`, r3 `cda0161`, r2 `d9c2da7`, r1 `da5f6c9` |
| 결과물 | 이 폴더에 `REV-081_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-265`부터. 형식은 [REV-080](REV-080_MS013_R7_DESIGN_REVIEW.md)과 같게 |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-081을 등록한다 |
| 반복 규칙 | 2026-11-27 Human 「지적이 없을때까지 리뷰 요청하자」: 이전 지적이 모두 해소되고 새 지적이 0건인 회차까지 반복. 0건이면 종합 판정 Pass |

## 1. 무엇을 보나

1. **REV-080 대조:** RV-261~264 각각이 처리담당 기록과 MS-013 §17이 가리키는 r8 본문으로 해소됐는지. REV-074~079 해소 상태(특히 RV-256·258·259)가 퇴행하지 않았는지.
2. **r8에서 바뀐 계약이 새로 만든 문제:** 「r8에서 바뀐 것」 4항목이 대상이다.
   - §5.6 host 종료 증거 `host_exited`: `worker_failed`(IO panic 포함)와의 구분, IO 실패 중 살아 있는 host의 명령 처리(①의 `Closing`, `Committed` → `Applied`)가 명령 결과표·반환 상한(기한 + 100 ms)과 맞는지.
   - §5.9 발행 셀과 `files_late: Option<bool>`·`files_settled()`: 두 판정표의 행이 셀 상태로 빠짐없이 나뉘는지, `files_complete()`·`is_success()`·`Closed` 조건과 일관된지.
   - §5.4·§5.9 `finalized`/`reclaimed`: host가 스레드 종료를 확인해 join하는 경로, `NetworkFinalized` 없는 실패, 회수 지연 중 `Incomplete`·`wait(timeout)`·분리가 기존 `PendingShutdown` 의미와 맞는지.
   - §5.6 구독 상태 표(`Open`/`AckGone`/`Ended`/`Failed`)와 `next`·`handoff`·`flush`·`acknowledged`: End 수치 표·선착 순서표·불변식 `acknowledged() + ack_unknown = handoff 수`와 서로 맞는지, 지금 client 메서드 동작과의 차이가 명시됐는지.
3. **수렴 기준(REV-077 §4·REV-078 §4·REV-079·REV-080을 따름):** 호출자가 관측하는 결과·공개 계약·상태 전이·정산식의 공백만 지적하고, Rust 원자 순서·ring 구현·처리량·깨우기 방식·허용치 충족은 골든과 단계 0 spike 판정으로 남긴다. 계약 공백이 없으면 **지적 0건, 종합 판정 Pass**로 반복을 끝낸다.

## 2. 읽을 것

- 대상: MS-013 r8(「r8에서 바뀐 것」과 §17에서 시작, 부록 A 제외).
- 이전 리뷰: REV-074~080.
- 대조용 코드: REV-076 요청서 §2 목록과 같음. 이번 회차에 특히 `crates/dot-logger/src/logger/host.rs`(host 스레드 종료 처리·`worker_failed` 설정 지점, `io_main`, 기한 보고, core Closed 뒤 IO join), `crates/dot-logger/src/logger/mod.rs`(`REPORT_GRACE`·`PendingShutdown::wait`·`FacadeReport::is_success`), `crates/dot-logger-client/src/transport.rs`(`next`·`handoff`·`flush`·`acknowledged`·`reap`·`unknown()`).

## 3. 범위 밖

- 제품 코드 수정·시험 실행, 성능 재측정.
- H1·H3~H8·Q1~Q8의 최종 결정(Human 몫, 의견 환영).

## 4. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
git diff 405cf39 -- .cowork/04_implementation/milestones/MS-013.md
grep -n "worker_failed\|fn io_main\|catch_unwind" crates/dot-logger/src/logger/host.rs
grep -n "pub async fn\|pub fn acknowledged\|fn reap\|fn unknown" crates/dot-logger-client/src/transport.rs
```

빌드·시험은 필요할 때만 `CARGO_TARGET_DIR=/data/build/target`으로, 장시간 실행은 하지 않는다.

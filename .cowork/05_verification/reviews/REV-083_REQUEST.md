# REV-083 리뷰 요청 — MS-013 r10 재리뷰(REV-082 반영 대조, 10회차)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-28 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지, **MS-013도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) **r10**(Draft, REV-082 2건 반영. 설계 결정은 2026-11-28 Human 승인 — MS-013 §2.2~2.4·§9.2). 코드 변경 없음 — 설계 재리뷰 |
| 기준 | 이 요청서와 MS-013 r10을 넣은 `main` 커밋(`git log -1 -- .cowork/04_implementation/milestones/MS-013.md`). r9 `963b222`, r8 `b392eb7`, r7 `405cf39`, r6 `7fe024c`, r5 `fe0f8d5`, r4 `3901fa3`, r3 `cda0161`, r2 `d9c2da7`, r1 `da5f6c9` |
| 결과물 | 이 폴더에 `REV-083_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-270`부터. 형식은 [REV-082](REV-082_MS013_R9_DESIGN_REVIEW.md)와 같게 |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-083을 등록한다 |
| 반복 규칙 | 2026-11-27 Human 「지적이 없을때까지 리뷰 요청하자」: 이전 지적이 모두 해소되고 새 지적이 0건인 회차까지 반복. 0건이면 종합 판정 Pass |

## 1. 무엇을 보나

1. **REV-082 대조:** RV-268·269가 처리담당 기록과 MS-013 §19가 가리키는 r10 본문으로 해소됐는지. REV-074~081 해소 상태(특히 RV-262·265·267)가 퇴행하지 않았는지.
2. **r10에서 바뀐 계약이 새로 만든 문제:** 「r10에서 바뀐 것」 2항목이 대상이다.
   - §5.9 셀 CAS 시점(IO 스레드의 마지막 동작)·스레드 지역 실패·흡수 논증·`files_complete()`/`files_settled()` 새 식: 현재 `io_main`·`io_loop`·진입 closure·`io_failed` 설정 지점과 맞는지, 판정 단조성이 두 판정표·모드별 표와 일관된지.
   - §5.6 두 기한의 만료(호출 기한 → `TimedOut`·`AckGone` 유지, `AckGone` 기한 → `Failed`, 같은 시각 우선순위): 구독 상태 표·선착 순서표·현행 `next()`의 `TimedOut` 의미와 일관된지.
3. **승인된 결정:** MS-013 §2.2~2.4·§9.2는 Human이 승인했다. 승인 항목 자체를 뒤집는 의견은 계약 공백과 구분해 「승인 재검토 의견」으로 적는다(지적 수에는 넣지 않음).
4. **수렴 기준(REV-077 §4·REV-078 §4·REV-079~082를 따름):** 호출자가 관측하는 결과·공개 계약·상태 전이·정산식의 공백만 지적하고, Rust 원자 순서·ring 구현·처리량·깨우기 방식·허용치 충족은 골든과 단계 0 spike 판정으로 남긴다. 계약 공백이 없으면 **지적 0건, 종합 판정 Pass**로 반복을 끝낸다.

## 2. 읽을 것

- 대상: MS-013 r10(「r10에서 바뀐 것」과 §19에서 시작, 부록 A 제외).
- 이전 리뷰: REV-074~082.
- 대조용 코드: REV-076 요청서 §2 목록과 같음. 이번 회차에 특히 `crates/dot-logger/src/logger/host.rs`(IO 스레드 진입 closure, `io_main`·`io_loop`의 `finish` 뒤 단계, `io_failed` 설정 지점), `crates/dot-logger-client/src/transport.rs`(`next`의 `cancel.guard(self.timeout, ..)`와 `TimedOut`).

## 3. 범위 밖

- 제품 코드 수정·시험 실행, 성능 재측정.
- 승인된 H1·H3~H8·Q1~Q8·§2.4의 재결정(의견은 위 1-3처럼 분리).

## 4. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
git diff 963b222 -- .cowork/04_implementation/milestones/MS-013.md
grep -n "io_failed.store\|fn io_main\|catch_unwind\|log.finish" crates/dot-logger/src/logger/host.rs
grep -n "cancel\s*\.guard\|TimedOut" crates/dot-logger-client/src/transport.rs
```

빌드·시험은 필요할 때만 `CARGO_TARGET_DIR=/data/build/target`으로, 장시간 실행은 하지 않는다.

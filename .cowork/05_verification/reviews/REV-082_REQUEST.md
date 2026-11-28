# REV-082 리뷰 요청 — MS-013 r9 재리뷰(REV-081 반영 대조, 9회차)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-28 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지, **MS-013도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) **r9**(Draft, REV-081 3건 반영). 코드 변경 없음 — 설계 재리뷰 |
| 기준 | 이 요청서와 MS-013 r9를 넣은 `main` 커밋(`git log -1 -- .cowork/04_implementation/milestones/MS-013.md`). r8 `b392eb7`, r7 `405cf39`, r6 `7fe024c`, r5 `fe0f8d5`, r4 `3901fa3`, r3 `cda0161`, r2 `d9c2da7`, r1 `da5f6c9` |
| 결과물 | 이 폴더에 `REV-082_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-268`부터. 형식은 [REV-081](REV-081_MS013_R8_DESIGN_REVIEW.md)과 같게 |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-082를 등록한다 |
| 반복 규칙 | 2026-11-27 Human 「지적이 없을때까지 리뷰 요청하자」: 이전 지적이 모두 해소되고 새 지적이 0건인 회차까지 반복. 0건이면 종합 판정 Pass |

## 1. 무엇을 보나

1. **REV-081 대조:** RV-265~267 각각이 처리담당 기록과 MS-013 §18이 가리키는 r9 본문으로 해소됐는지. REV-074~080 해소 상태(특히 RV-262·263·264)가 퇴행하지 않았는지.
2. **r9에서 바뀐 계약이 새로 만든 문제:** 「r9에서 바뀐 것」 3항목이 대상이다.
   - §5.9 IO 스레드별 발행 셀·합치는 규칙·종결 주체 네 경로·`files_published`/`files_reclaimed`: 현재 `io_main`·`drain_failed`·IO 스레드 진입 closure와 맞는지, 두 판정표와 `files_complete()`·`files_settled()`가 일관된지.
   - §5.9 모드별 `Closed` 조건표와 §5.4 `is_lossless()`: 파일만·파일 + legacy tap·혼합·네트워크만에서 공허 참·보고 값(`network = None`, 네트워크 전용 `files_late = Some(false)`)이 §5.8 보장표·§6과 맞는지.
   - §5.6 `AckGone` 기한(첫 `Gone` 관측 + `timeout`, 재설정 없음, 「읽은 시각」 기준): 구독 상태 표·선착 순서표·현행 `next()` 호출별 타이머와 일관된지.
3. **수렴 기준(REV-077 §4·REV-078 §4·REV-079~081을 따름):** 호출자가 관측하는 결과·공개 계약·상태 전이·정산식의 공백만 지적하고, Rust 원자 순서·ring 구현·처리량·깨우기 방식·허용치 충족은 골든과 단계 0 spike 판정으로 남긴다. 계약 공백이 없으면 **지적 0건, 종합 판정 Pass**로 반복을 끝낸다.

## 2. 읽을 것

- 대상: MS-013 r9(「r9에서 바뀐 것」과 §18에서 시작, 부록 A 제외).
- 이전 리뷰: REV-074~081.
- 대조용 코드: REV-076 요청서 §2 목록과 같음. 이번 회차에 특히 `crates/dot-logger/src/logger/host.rs`(IO 스레드 생성·진입 closure, `io_main`·`drain_failed`, `finish` 경로, `remote.stop()`·최종 보고), `crates/dot-logger/src/logger/remote.rs`(`stop`), `crates/dot-logger-client/src/transport.rs`(`next`의 호출별 timeout, `handoff`·`flush`).

## 3. 범위 밖

- 제품 코드 수정·시험 실행, 성능 재측정.
- H1·H3~H8·Q1~Q8의 최종 결정(Human 몫, 의견 환영).

## 4. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
git diff b392eb7 -- .cowork/04_implementation/milestones/MS-013.md
grep -n "fn io_main\|fn drain_failed\|spawn_scoped\|remote.stop\|log.finish" crates/dot-logger/src/logger/host.rs
grep -n "pub async fn next\|cancel.guard" crates/dot-logger-client/src/transport.rs
```

빌드·시험은 필요할 때만 `CARGO_TARGET_DIR=/data/build/target`으로, 장시간 실행은 하지 않는다.

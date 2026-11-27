# REV-076 리뷰 요청 — MS-013 r3 재리뷰(REV-075 반영 대조, 3회차)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-27 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지, **MS-013도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) **r3**(Draft, REV-075 10건 반영). 코드 변경 없음 — 설계 재리뷰 |
| 기준 | 이 요청서와 MS-013 r3를 넣은 `main` 커밋(`git log -1 -- .cowork/04_implementation/milestones/MS-013.md`). r2 `d9c2da7`, r1 `da5f6c9` |
| 결과물 | 이 폴더에 `REV-076_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-235`부터. 형식은 [REV-075](REV-075_MS013_R2_DESIGN_REVIEW.md)와 같게(요약 → 이전 지적 대조 → 리뷰 포인트, P1/P2/P3, 종합 판정 Pass/Conditional/Reject) |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-076을 등록한다 |
| 반복 규칙 | 2026-11-27 Human 「지적이 없을때까지 리뷰 요청하자」: **이전 지적이 모두 해소되고 새 지적이 0건인 회차**까지 반영·재리뷰를 반복한다. 0건이면 종합 판정 Pass로 적어 반복을 끝낸다. 그 뒤 Human이 §2.3 H1·H3~H7과 §9.2 Q1~Q8을 결정·승인한다 |

## 1. 무엇을 보나

1. **REV-075 대조:** RV-225~234 각각에 대해 처리담당 기록과 MS-013 §12 반영표가 가리키는 r3 본문이 지적을 실제로 해소했는지. 이전 회차(REV-074 RV-213~224) 해소 상태가 r3에서 퇴행하지 않았는지.
2. **r3의 새 설계가 만든 문제.** 특히:

### 중점 1 — Router 복사 target(§5.1, RV-225)

- dispatcher feedback 경로로 복사 target의 완료(`Handed`·`Dropped`)를 기록할 수 있는지, 그 경로가 지금 무엇을 기록하며(target 포화 등) 용량·포화 시 동작이 계획(「그 턴 분배 중단, 다음 턴 계속」)과 맞는지. 아니라면 단계 0 spike로 확인한다는 서술로 충분한지.
- 분배 중 복사가 core `Router`의 다른 불변식(공유 기록 한도, target ledger, 완료 창, raw 회수, Miri 대상 경로)을 깨지 않는지. 턴별 복사 한도로 다음 턴으로 넘긴 기록의 보유가 다시 RV-213 문제를 만들지 않는지.
- H6 허용치(0.8P 부하에서 거절 0, 포화 감소 ≤ 10%)가 측정 가능하고 판정에 쓸 수 있는지.

### 중점 2 — 회계·종료(§5.4·§5.9, RV-226·228)

- 확장한 등식(L1·L2·L3·ACK)이 모든 종결 경로(정상·실패·기한·강제 정리·`Detached`)에서 닫히는지, 스냅샷 규칙(단계 안 등식 항상, 단계 사이 순간 차이)이 구현 가능하고 사용자가 오해하지 않는지.
- 병행 종료에서 `is_success()`(파일만)와 `network.cleanup`, `Closed`/`Incomplete`의 경계가 일관된지, `Detached` 스레드의 수명·메모리 반환 설명이 RT·예산 계약과 충돌하지 않는지.

### 중점 3 — 스위치·wire(§5.6·§6, RV-227·229·230)

- CAS 중재(`Pending → Committed/Cancelled`)가 host 명령 처리 구조에서 「Timeout = 미적용」을 실제로 보장하는지, 호출자가 「host가 이긴 뒤」 기다리는 시간이 상한을 갖는지.
- 링 안 FIFO 표시 + 구독 큐 제어 자리 1개 + `MarkerOverflow`가 순서·전달을 보장하는지, 새 구독 초기 표시와 진행 중 표시가 경합하지 않는지.
- `epoch_markers` 세 값과 옛 client·옛 서버 조합, 관리 wire 경계 5, `Cause::Policy` 재사용이 기존 저장·판독·CLI와 맞는지.

### 중점 4 — 설정·메모리(§5.5·§5.10, RV-231·232)

- 출력 전체 교체(H7)·자동 이름 예약어·교체 뒤 index가 기존 코드 사용(`file_output` + `ChannelOptions::outputs(&[usize])`)과 섞일 때 혼란이나 조용한 의미 변화를 만들지 않는지.
- 메모리 산식이 `RemoteLimits::reservation()`·facade 메모리 분할(`budget.rs`)과 실제로 맞고, 관리 예약/측정 여유 구분이 FR-021 수락으로 충분한지.

### 중점 5 — 계획

- 단계 0 spike 범위와 단계 1 core 변경이 단독 리뷰 가능한 크기인지, 인일 20~25가 현실적인지, 표면 gate·호환 시험 배치가 맞는지.
- §2.3 H1·H3~H7이 Human이 결정할 수 있을 만큼 선택지와 결과를 보여 주는지.

## 2. 읽을 것

- 대상: MS-013 r3 전체(부록 A는 원문 기록, 대조 불필요). 「r3에서 바뀐 핵심」과 §12에서 시작하면 빠르다.
- 이전 리뷰: REV-074·REV-075(지적·처리담당 기록).
- 대조용 코드: REV-075 요청서 §2 목록 + `crates/dot-logger/src/routing/mod.rs`(dispatcher·feedback·`Router::scope`·`try_next`·reclaim), `crates/dot-logger/src/completion/`(마스크·`Busy`), `crates/dot-logger/src/loss.rs`(`Cause::Policy`), `crates/dot-logger/src/logger/{mod.rs, host.rs, budget.rs, event.rs, remote.rs}`, `crates/dot-logger/src/adapters/remote/{hub.rs, server.rs, wire.rs, types.rs}`, `crates/dot-logger-client/src/transport.rs`, `crates/dot-logger-cli/src/commands.rs`.

## 3. 범위 밖

- 제품 코드 수정·시험 실행(필요하면 「단계 0에서 확인」으로 지적).
- 성능 재측정, MS-012·TASK-047 진행 내용.
- H1·H3~H7·Q1~Q8의 최종 결정(Human 몫, 의견 환영).

## 4. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
git diff d9c2da7 -- .cowork/04_implementation/milestones/MS-013.md | head -600
grep -n "feedback\|fn turn\|reclaim_raw\|DispatchEvent" crates/dot-logger/src/routing/mod.rs | head -40
grep -n "Policy\|NoRoute" crates/dot-logger/src/loss.rs | head
```

빌드·시험은 필요할 때만 `CARGO_TARGET_DIR=/data/build/target`으로, 장시간 실행은 하지 않는다.

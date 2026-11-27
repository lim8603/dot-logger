# REV-077 리뷰 요청 — MS-013 r4 재리뷰(REV-076 반영 대조, 4회차)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-27 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지, **MS-013도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) **r4**(Draft, REV-076 10건 반영). 코드 변경 없음 — 설계 재리뷰 |
| 기준 | 이 요청서와 MS-013 r4를 넣은 `main` 커밋(`git log -1 -- .cowork/04_implementation/milestones/MS-013.md`). r3 `cda0161`, r2 `d9c2da7`, r1 `da5f6c9` |
| 결과물 | 이 폴더에 `REV-077_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-245`부터. 형식은 [REV-076](REV-076_MS013_R3_DESIGN_REVIEW.md)과 같게 |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-077을 등록한다 |
| 반복 규칙 | 2026-11-27 Human 「지적이 없을때까지 리뷰 요청하자」: 이전 지적이 모두 해소되고 새 지적이 0건인 회차까지 반복. 0건이면 종합 판정 Pass |

## 1. 무엇을 보나

1. **REV-076 대조:** RV-235~244 각각이 처리담당 기록과 MS-013 §13 반영표가 가리키는 r4 본문으로 실제로 해소됐는지. REV-074·075 해소 상태가 r4에서 퇴행하지 않았는지(특히 r2·r3의 `Handed` 경계를 철회하면서 RV-217·230이 다른 형태로 다시 열리지 않았는지).
2. **r4의 새 설계가 만든 문제.** 특히:

### 중점 1 — Route 복사 observer(§5.1·§5.3, H8)

- dispatcher가 `SharedPayload`로 분배하는 그 안에서 링에 복사하는 것이 지금 Router 분배 구조(ingress 소비·`SharedPayload` 생성·target 분배·raw 회수)에서 가능한지, 원본 수명·raw zero-copy 계약과 충돌하지 않는지.
- 네트워크를 completion registry에서 뺀 결과: 네트워크 전용 채널이 `Unrouted` → `Cause::NoRoute`(`IntentionalSkip`)로 정산되는 것이 손실 보고·`.logger/loss`·종료 판정에서 오해나 회귀를 만들지 않는지. tracked target이 0개인 구성(파일 없는 네트워크 전용)에서 completion·sync·loss bank·`is_success`가 성립하는지(단계 0 spike로 미룬 것이 적절한지).
- 복사 예산 드롭(같은 분배에서 네트워크 몫만)과 시작 검증이 RV-235를 닫는지.

### 중점 2 — hub 상태 계수와 감독 정산(§5.4, RV-238)

- hub 잠금 안 전이 계수(`queued`·`partial`·`sent`·`acked`)가 지금 hub 구조(구독 FIFO·`gap` 구간·chunk·ACK 창)에 들어갈 수 있는지, 잠금 구간·메모리 비용.
- `Progress` 공유 구조와 감독 1회 정산이 네트워크 스레드 실패의 모든 지점(링 pop 직후, eligible 고정 직후, 부분 offer 중)을 덮는지.

### 중점 3 — 종료(§5.9, RV-239)

- 호출자 스레드 감독 + 기존 `Closed`/`Incomplete`/`PendingShutdown` 재사용이 「파일 성공을 기한에 확인」과 「Closed = 모든 정리 완료」를 함께 만족하는지, `files_complete()`와 `Shutdown::is_success()`의 관계가 명확한지.

### 중점 4 — 스위치·wire(§5.6, RV-237·240·243)

- host 한 단계 커밋 순서(기한 → 같은 상태 → 제어 자리 → CAS → 기록 → 응답)가 「Timeout = 미적용」과 호출자 대기 상한을 보장하는지.
- 링 일련번호·가입 fence·링 위치 기준 초기 epoch·표시의 Gap fence가 순서를 보장하는지, End frame·`epoch_markers` 세 값의 호환.

### 중점 5 — 수렴

- 남은 문제가 **설계 문서로 확정할 수 있는 계약**인지, **구현·측정으로만 확인되는 항목**인지 구분해 주기 바란다. 후자는 「단계 0 spike의 판정 기준으로 충분히 명시됐는가」로 판정하면 반복이 수렴한다(MS-013 K13).

## 2. 읽을 것

- 대상: MS-013 r4(「r4에서 바뀐 핵심」과 §13에서 시작, 부록 A 제외).
- 이전 리뷰: REV-074·075·076.
- 대조용 코드: REV-076 요청서 §2 목록과 같음. 특히 `crates/dot-logger/src/routing/{mod.rs, types.rs}`(분배·`SharedPayload`·`Route`·`Unrouted`), `crates/dot-logger/src/completion/mod.rs`(`Unrouted` 처리), `crates/dot-logger/src/loss.rs`(`NoRoute`·`IntentionalSkip`), `crates/dot-logger/src/adapters/remote/hub.rs`(구독 FIFO·gap·ACK), `crates/dot-logger/src/logger/{mod.rs, host.rs}`(종료).

## 3. 범위 밖

- 제품 코드 수정·시험 실행, 성능 재측정.
- H1·H3~H8·Q1~Q8의 최종 결정(Human 몫, 의견 환영).

## 4. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
git diff cda0161 -- .cowork/04_implementation/milestones/MS-013.md | head -700
grep -n "Unrouted\|SharedPayload\|fn dispatch\|pub struct Route" crates/dot-logger/src/routing/*.rs | head -30
grep -n "NoRoute\|IntentionalSkip" crates/dot-logger/src/loss.rs | head
```

빌드·시험은 필요할 때만 `CARGO_TARGET_DIR=/data/build/target`으로, 장시간 실행은 하지 않는다.

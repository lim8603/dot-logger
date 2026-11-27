# REV-075 리뷰 요청 — MS-013 r2 재리뷰(REV-074 반영 대조)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-27 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지, **MS-013도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) **r2**(Draft, REV-074 12건 반영). 코드 변경 없음 — 설계 재리뷰 |
| 기준 | 이 요청서와 MS-013 r2를 넣은 `main` 커밋(`git log -1 -- .cowork/04_implementation/milestones/MS-013.md`). r1은 `da5f6c9`, REV-074 기준도 같다 |
| 결과물 | 이 폴더에 `REV-075_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-225`부터. 형식은 [REV-074](REV-074_MS013_OUTPUT_DESIGN_REVIEW.md)와 같게(요약 → 절·코드 대조 → 리뷰 포인트, P1/P2/P3, 종합 판정 Pass/Conditional/Reject) |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-075를 등록한다 |
| 이후 | 2026-11-27 Human 결정에 따라 **재리뷰는 이번 1회**다. 결과를 반영한 뒤 Human이 MS-013(§2.3 H1~H5, §9.2 Q1~Q8 포함)을 승인하면 ADR-017~019·요구사항·TASK로 넘어간다. 남은 P1이 있으면 Human 판단으로 넘긴다 |

## 1. 무엇을 보나

1. **REV-074 대조:** [REV-074](REV-074_MS013_OUTPUT_DESIGN_REVIEW.md)의 RV-213~224 각각에 대해, 처리담당 기록과 MS-013 §11 반영표가 가리키는 r2 본문이 지적을 실제로 해소했는지. 「수용」으로 적고 본문이 부족한 곳, 해소 과정에서 새로 생긴 모순을 찾는다.
2. **r2의 새 설계가 만든 문제:** r1에 없던 결정이 들어왔다. 특히 아래를 본다.

### 중점 1 — host 턴 내 복사·원본 반환(§5.1, RV-213 해소안)

- host가 네트워크 target의 소비자로서 분배 턴 안에 큐를 비우는 것이 Router·완료 모델(`TargetConsumer` 소유, delivery 수명, 턴 구조)에서 실제로 가능한지. 불가능하거나 비용이 크다면 그 근거와 대안(K8의 「네트워크 스레드 복사 + 링 대기 상한」)의 격리 보장 차이.
- 「원본 보유 ≤ 1턴」이 small·raw·Important·완료 창 모두에 대해 성립하는지, raw zero-copy 대여(`try_acquire`) 수명 계약과 충돌이 없는지.
- host 스레드 복사 비용이 파일 분배·RT 거절에 주는 영향(K8)을 단계 5까지 미뤄도 되는지.

### 중점 2 — 완료 경계·ACK 분리(§5.3·§5.9, RV-216·217, H1)

- `Boundary::Handed` 추가(마스크 6비트)의 core 영향 범위가 단계 1에 충분히 들어갔는지(조회·집계·abandon·`Boundaries` 상수·직렬화·`.logger/loss` 등 저장 형식에 경계가 기록되는 곳이 있는지).
- 스위치 꺼짐을 「정책 생략」으로 정산하는 `DropReason` 결정이 손실 보고(사고성/정책) 구분과 맞는지.
- ACK를 복사본 수준 보고로 둔 것이 C6(「수신 ACK 완료는 옵션」)의 의도를 해치는지 — H1 판단 자료.
- 종료 순서(§5.9)의 각 단계 예산이 `drop_deadline`(UPS 3 s 포함) 안에서 닫히는지, `is_lossless()` 정의가 §5.4 상태와 일치하는지.

### 중점 3 — 회계(§5.4, RV-214, H2)

- 보존식 4개가 서로 맞물리는지(L1 `handed` = L2 입력, L2 `dispatched` × 구독 = L3 `targeted`), 각 상태가 실제로 관측 가능한 지점에서 세어지는지(`hub.offer`·스트림·종료 경로).
- 조용한 단절 감지 범위 서술이 EV-123 F4와 코드(`max_age` 기록 나이 한도·유휴 lease)에 맞는지.

### 중점 4 — 스위치·wire(§5.6·§6, RV-215, H3)

- `epoch_markers` 협상과 새 frame kind가 기존 client(모르는 kind → `Corrupt`)·옛 서버 조합에서 정말 무영향인지, protobuf 필드 추가와 버전 협상(`codec::version`) 규칙에 맞는지.
- 표시 예약 슬롯·`MarkerOverflow` 종료, heartbeat 주기, 「Timeout = 미적용」 보장이 host 명령 처리 구조에서 성립하는지.
- `boundary`(생산자별 `next_seq`)를 host가 적용 시점에 정확히 알 수 있는지(Router 읽기 상태).

### 중점 5 — 범위·계획

- §6 공개 항목 예산(35 → 약 50/60)과 breaking 목록의 완전성.
- §7 단계 0~5의 수락 조건이 측정 가능하고, 인일 16.5~22가 현실적인지.
- §2.3 H1~H5가 Human이 결정할 수 있을 만큼 선택지와 결과를 보여 주는지.

## 2. 읽을 것

- 대상: MS-013 r2 전체(부록 A는 원문 기록, 대조 불필요).
- 이전 리뷰: REV-074(지적·처리담당 기록).
- 대조용 코드: REV-074 요청서 §2의 목록 + `crates/dot-logger/src/completion/{mod.rs, types.rs}`(마스크), `crates/dot-logger/src/routing/{mod.rs, types.rs}`(target 소비자·`DropReason`), `crates/dot-logger-client/src/transport.rs`(frame 해석), `crates/dot-logger/src/adapters/remote/{server.rs, wire.rs}`(구독 요청·frame·lease).

## 3. 범위 밖

- 제품 코드 수정·시험 실행(필요하면 「단계 0에서 확인」으로 지적).
- 성능 재측정, MS-012·TASK-047 진행 내용.
- H1~H5·Q1~Q8의 최종 결정(Human 몫, 의견 환영).

## 4. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
git diff da5f6c9 -- .cowork/04_implementation/milestones/MS-013.md | head -400
grep -n "& 31\|Boundaries(" crates/dot-logger/src/completion/mod.rs
grep -n "pub enum DropReason" -A16 crates/dot-logger/src/routing/types.rs
grep -n "frame.kind\|Corrupt" crates/dot-logger-client/src/transport.rs
```

빌드·시험은 필요할 때만 `CARGO_TARGET_DIR=/data/build/target`으로, 장시간 실행은 하지 않는다.

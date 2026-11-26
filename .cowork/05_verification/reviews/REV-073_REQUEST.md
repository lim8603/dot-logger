# REV-073 리뷰 요청 — REV-072 이후 변경(`f30ae5a..2632b6c`) 독립 코드리뷰

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-26 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지 |
| 결과물 | 이 폴더에 `REV-073_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-202`부터. 형식은 [REV-072](REV-072_POST_GATE4_PLATFORM_SOAK_S2_REVIEW.md)와 같게(요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트, P1/P2/P3, 종합 판정 Pass/Conditional/Reject) |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-073을 등록한다 |
| 기준 커밋 | 시작 `f30ae5a`(REV-072 대상 끝) · 끝 `2632b6c`(2026-11-26 `main`) |
| CI | `2632b6c` run `45796420659` — 요청 작성 시점 진행 중. 리뷰 시작 때 `gh run view 45796420659`로 결과를 확인하고 판정에 반영한다 |

## 1. 왜 이 범위인가

REV-072는 `ae22f9d..f30ae5a`를 독립 검토했고, 그 조치(`156bd68`·`e20aef4`·`641929c`)는 개발 세션의 **자가검토와 CI로만** Closed 됐다. 그 뒤 4일 동안 시험 시간 의존성 수정, CI 구조 변경, 장시간·장애·지연 측정 하네스, README 공개 수치가 들어왔다. 이 범위는 아직 독립 검토를 받지 않았다. 다음 작업(InfluxDB 연동 시험·샘플 코드, TASK-041 v0.1.1)에 앞서 독립 검토를 받는다.

`git diff --stat f30ae5a..2632b6c -- . ':(exclude).cowork'`: 44개 파일, +3,277 / −188.

## 2. 대상 커밋

| 커밋 | 내용 | 제품 코드(`src/`) |
|---|---|---|
| `156bd68` | REV-072 조치(RV-193~201): 플랫폼 계약, CI 커버리지, 하네스 이식성 | `remote/receiver.rs`·`remote/types.rs`·`diagnostics/host.rs`·`logger/budget.rs`·`logger/host.rs`·`logger/mod.rs`·`logger/writer.rs`·`storage/retention.rs`·`sync.rs` |
| `00f53cd`·`4721903`·`2a63fd8` | 플랫폼별 시험 범위(facade 2개, 16 슬롯 preset 시간) | — |
| `9c868a6` | TASK-024 S2 장애 주입 하네스(`soak.rs`, `scripts/soak-faults.py`) | — |
| `e20aef4`·`641929c` | RV-199 후속 macOS viewer 실시간 구독 회귀: live 예산을 코어 receiver 예약 크기로 | client `live.rs`·`receiver.rs`, viewer `bridge.rs`·`main.rs`, `remote/receiver.rs`·`remote/tests.rs` |
| `f19695f` | soak: 스레드별 heap 사용 집계, RT 스레드 할당 시 실패(TASK-039, EV-125) | — |
| `29c302e`·`b389360` | soak: 외부 읽기·구독·상태·이벤트 혼합(TASK-040, EV-126), 출력 상한 도달 판정 | — |
| `3245c20`·`9852f96`·`77d4d81`·`1795bc1` | 시간 의존 시험 수정(EV-127): client ACK 종료 판정, f17 sync 완료 판정, f32 표시 재요청 | client `transport.rs`(시험 부분) |
| `42a033a` | CI: Windows feature 조합 3 shard, Miri 시험 하나 반복 축소(EV-128) | `storage/mod.rs`(Miri 반복 수) |
| `b94964b`·`0012ef8`·`e595dd0`·`2632b6c` | TASK-042 지연 측정 하네스(`latency.rs`, `run-latency.sh`), EPP 기록, 느린 묶음 진단 | — |
| `5f06969` | `rt-boot.sh generic`(다음 1회 generic 커널 부팅, sudo·GRUB) | — |
| `7584b57`·`e7248d9` | README 지연 표와 해석(EV-129·EV-130) | — |

## 3. 중점 관점

### 중점 1 — REV-072 조치가 지적을 실제로 해소했는가

- [REV-072](REV-072_POST_GATE4_PLATFORM_SOAK_S2_REVIEW.md)의 RV-193~201 각각에 대해, 처리담당 기록([EV-122](../verification_evidence.md#ev-122)·[EV-124](../verification_evidence.md#ev-124))과 코드가 일치하는지, 범위를 바꿔 처리한 곳(RV-196 범위 확대, RV-201 `version`은 MS-010 3단계로 이월, 새로 켠 macOS·Windows 시험의 플랫폼 차이 3건 f9·f11·f15를 해당 플랫폼으로 한정)이 타당한지.
- `e20aef4`·`641929c`: viewer live 예산을 코어 receiver의 예약 크기로 맞춘 변경이 다른 OS·설정에서 예산 부족이나 과다 예약을 만들지 않는지. client `live.rs`·`receiver.rs`의 공개 API 영향.

### 중점 2 — 시간 의존 시험을 고치면서 검출력을 잃지 않았는가

- `3245c20`·`77d4d81`: 「ACK가 모두 확인됐을 때만 정상 종료·`Gone`·`Ok` 허용, 아니면 `AcknowledgementUnknown`」으로 바꾼 단정이 RV-191이 막으려던 결함(미확인 ACK를 정상으로 보고)을 여전히 잡는지.
- `9852f96`: f17이 「마지막 sync 완료 시각」으로 판정하도록 바뀐 뒤에도 예전 결함(부하 중 sync가 멈춤)을 잡는지. EV-127의 Red 확인이 충분한지.
- `1795bc1`: f32의 표시 재요청 보조 함수(`Failed`이고 출력이 정상일 때만 최대 5번)가 실제 표시 실패를 가리지 않는지.
- `00f53cd`·`4721903`·`2a63fd8`: 플랫폼별로 단정을 빼거나 보고로 바꾼 곳이 그 플랫폼의 결함을 놓치게 만들지 않는지.

### 중점 3 — CI 구조 변경이 검증 범위를 유지하는가

- `42a033a` `check-feature-matrix.py --shard K/N`: 모든 조합이 모든 OS × Rust에서 정확히 한 번씩 도는지(누락·중복), protocol 검사와 소비자 검사가 빠지지 않는지, 잘못된 shard 인자 처리.
- Miri 반복 20,000 → 300: 동시 예약 경합 결함(CAS 패배를 `Busy`로 보고)을 Miri에서 여전히 잡는지(EV-128 Red 근거).

### 중점 4 — 하네스의 판정·측정이 믿을 만한가

- `soak.rs`(+649줄)·`scripts/soak-faults.py`(+807)·`scripts/soak-mixed.py`(+354): Pass/Fail 판정이 실패를 놓치는 경로(예외 삼킴, 빈 결과를 Pass로, 카운터 오버플로), 스레드별 heap 집계(`GlobalAlloc` 래퍼, thread-local)의 정확성과 `unsafe` 건전성, 채움 시험의 대상 확인(2026-11-23 호스트 루트 채움 사고 뒤 넣은 tmpfs·크기 확인이 같은 실행 맥락에서 이뤄지는지).
- `latency.rs`(+662)·`run-latency.sh`: 분위수 히스토그램(1/32) 경계, 시계 쌍 보정, `cold`·`sparse`·`multi`·`jitter` 행의 측정이 의도한 것을 재는지, 측정 루프 안 할당(`SlowBatch` 배열 미리 할당), `require_tmpfs` 우회 가능성. `run-latency.sh`의 커널·governor 확인과 `dirty` 판정.
- `rt-boot.sh generic`: sudo로 GRUB `next_entry`를 바꾸는 스크립트의 입력 검증, 실패 시 기본 항목이 바뀌지 않는지, 로그.

### 중점 5 — README 공개 수치와 해석이 근거와 맞는가

- README 「Verification」의 지연 표 5행과 문단이 [EV-129](../verification_evidence.md#ev-129)·[EV-130](../verification_evidence.md#ev-130)·원시 결과([task042-2026-11-25](../perf_runs/task042-2026-11-25/)·[task042-diag-2026-11-26](../perf_runs/task042-diag-2026-11-26/))와 일치하는지. 집계 방식(3회 중앙값, 최대는 3회 중 최대)을 그대로 재계산해 확인한다.
- 해석의 강도: 「This is not the cost of `try_log`」(EV-130 근거: 같은 주기의 50 µs 연산이 약 470 µs, 기록 0건 행의 응답 p99.9 475~507 µs), 「Logging does not change the loop's wake-up delay」, 「five of the six generic-kernel results exceeded 85 µs」 등이 근거보다 강하게 말하지 않는지. EV-130에 적은 한계(H·M 행 최대에도 감속이 섞였을 수 있음)가 README 독자에게 필요한 정보인지.
- 공개 문서가 측정 PC 밖으로 일반화하는 표현이 없는지.

## 4. 범위 밖

- `.cowork/` 문서 자체(위에서 근거로 지정한 EV·Task 절은 대조 대상으로만 읽는다), `vendor/`(변경 없음), dot-probe 저장소.
- 성능 수치의 재측정(이 PC의 루트·재부팅이 필요). 원시 결과로 재계산만 한다.
- 기능 논의 안건(keepalive, 구독 실패 원인 구분, 레벨 skip 비용 등) — [project_state 다음 시작점](../../06_evolution/project_state.md#다음-시작점)에 이미 있다. 이번 변경이 그 안건을 악화시킨 경우만 지적한다.

## 5. 참고 검사 명령

```sh
git diff --stat f30ae5a..2632b6c -- . ':(exclude).cowork'
cargo +1.96.0 fmt --all --check
cargo +1.96.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.85.0 clippy -p dot-logger --all-targets --all-features --locked -- -D warnings
cargo test -p dot-logger --test tc_012_performance --all-features
cargo test -p dot-logger --test tc_013_facade --all-features
cargo test -p dot-logger-client --all-features
python3 scripts/check-feature-matrix.py --toolchain 1.85.0 --shard 0/3   # 1/3, 2/3도
gh run view 45796420659
```

큰 빌드는 `CARGO_TARGET_DIR=/data/build/target`을 쓴다. 장시간·대용량 실행(soak, 채움)은 이 리뷰에서 하지 않는다.

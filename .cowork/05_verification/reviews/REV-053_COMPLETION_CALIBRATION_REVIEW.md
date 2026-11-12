# REV-053 — 완료 이력 정리·장비 측정 누적 코드리뷰

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-053 / 2026-11-11 |
| 리뷰어 | Codex AI — 리뷰 전용 세션 |
| 대상 | REV-052 이후 누적 변경 `93db6e3..8dbd999`, 시작 작업 트리 clean |
| 범위 | 변경 123파일(비 vendor Rust 30파일). REV-052 sync 조치, TASK-023 S2/S3·RT harness, T5 완료 장부 최적화, MCAP 단일 CRC 패치, TASK-028 S2 calibration·preset·CLI 및 관련 시험/계약 |
| 종합 판정 | **Conditional — P1 1건·P2 1건·P3 1건** |
| 상태 | **Closed** — 2026-11-12 3건 모두 수용·조치([EV-072](../verification_evidence.md#ev-072)), 같은 날 Human 「승인」 |

공개 `Completion::reconcile`의 동시 호출로 새 정리 작업이 유실되어 완료 slot 재사용이 막히는 문제를 재현했다. Linux의 전체 회귀는 통과하지만, `calibrate`의 비 Linux 분기는 타입 오류가 있고 기본 feature의 rustdoc 링크도 깨진다. 제품 코드와 저장소 테스트는 수정하지 않았다. 기존 Human 수락과 EV-056~071의 과거 실행 사실은 유지하며, 이번에 확인한 공백을 별도로 추적한다.

## 2. 파일 변경표

| 구분 | 파일 | 변경 내용 |
|---|---|---|
| 생성 | [REV-053](REV-053_COMPLETION_CALIBRATION_REVIEW.md) | 발견·재현 근거·검증 범위·처리담당 영역 |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | Open 리뷰 등록, 후속 개발 시작점 동기화 |

검토한 주요 제품 파일은 `completion/mod.rs`, `control/mod.rs`, `routing/mod.rs`, `storage/{mod,writer,retention}.rs`, `logger/{mod,budget,host,writer,preset}.rs`, `calibrate.rs`, CLI 3파일, 성능 예제 3파일, 성능·RT 스크립트 및 해당 회귀 시험이다. vendor는 로컬 Cargo 원본 0.26.0과 `src/` 전체를 비교하여 `write.rs`의 기록된 두 CRC 변경만 있음을 확인했다.

## 3. 클래스(타입) 변경표

**이번 리뷰 세션의 struct·enum·trait 변경은 없음.** 검토 대상에는 `Registry`, `Completion`, `AbandonMarker`, `ControlOwner`, `Foreign`, `FileLedger`, `SegmentWriter`, `FinishedSegment`, `SegmentLog`, `Calibration`, `Profile`, `Host`, `Fact`, `Sizing`, `PlanWarning`, `Gate`, `StorageWorkload`, `StorageRun` 등이 포함된다.

## 4. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-131 | P1 | `completion/mod.rs:511~558`, API §19.7 완료 장부 정리 | 이전 reconcile의 잔여량 저장이 새 세대의 정리 작업을 지워 slot 재사용이 계속 거절됨 | 정리 작업 소유권을 단일화하고 세대·cursor·잔여량 갱신을 보호 | 수용 | Closed |
| RV-132 | P2 | `calibrate.rs:662~674`, API §19.8~19.9 플랫폼 대체 경로 | 비 Linux 분기의 closure 반환형이 `Fact<String>`으로 고정되어 다른 필드에서 컴파일 오류 | 제네릭 함수 또는 필드별 생성으로 수정, 플랫폼별 시험 기대값 정렬 | 수용 | Closed |
| RV-133 | P3 | `logger/preset.rs:75`, core 선택 feature 경계 | 기본 feature에서는 존재하지 않는 `Self::profile` 링크로 rustdoc 경고/엄격 빌드 실패 | feature에 맞게 링크를 조건화하거나 일반 코드 표기로 변경 | 수용 | Closed |

### RV-131 — 이전 정리 호출이 새 세대의 잔여 작업을 지움 (P1)

#### [리뷰어]

- **위치:** [Completion::reconcile](../../../crates/dot-logger/src/completion/mod.rs:511), 특히 [잔여량 저장](../../../crates/dot-logger/src/completion/mod.rs:555). 도입: `1a31b3e`, 관련 후속 `2f3aad1`.
- **원인:** `swept.swap`, `sweep_left.load/store`, `sweep_missed`가 각각 원자적이어도 하나의 정리 작업으로 직렬화되어 있지 않다. `Completion`은 `Copy`이며 공유 참조에서 호출할 수 있다. 공개 API에 동시 reconcile을 막는 소유권/진입 보호가 없다. 한 호출이 시작할 때 읽은 `left`를 다른 호출이 새 세대를 시작한 뒤에도 그대로 저장한다.
- **재현 순서(window=2):**
  1. seq1(slot1)은 미보고 delivery를 버리고, seq2(slot0)는 Written으로 완료한다. `reconcile(1)`로 slot0만 방문한다.
  2. 호출 A가 `reconcile(1)`로 slot1의 seq1을 종결하고, `sweep_left.store(0)` 직전에 멈춘다.
  3. 생산자는 비워진 slot1에 seq3을 수락하고 그 delivery도 버린다. 호출 B가 새 abandonment 세대를 받아 `sweep_left=2`로 재시작한 뒤 slot0만 방문하여 `sweep_left=1`을 남긴다.
  4. A가 재개하여 이전 작업의 `sweep_left=0`을 저장한다. `swept`는 이미 새 세대여서 이후 `reconcile(2)`를 반복해도 정리가 시작되지 않는다.
  5. seq4를 수락·Written 완료한 뒤 seq5를 제출하면 `Err(CompletionCapacity)`다. seq3이 ACTIVE에 남아 있고 추가 abandonment가 없다면 스스로 회복하지 않는다.
- **관측 결과:** 저장소 밖 복사본에 기존 `completion/tests.rs::setup`을 이용한 `rev053_a_stale_sweep_cannot_erase_a_new_abandonment`를 추가했다. 제품 알고리즘은 그대로 두고 line 555 직전에 test-only thread-local 두 Barrier를 넣어 A의 정지/재개 순서만 고정했다. `cargo test --offline --manifest-path /tmp/dot-logger-rev053-probe/repo/Cargo.toml -p dot-logger --lib rev053_a_stale_sweep -- --nocapture` → **의도한 Red**, exit 101: `abandoned slot must be reusable after reconciliation, got Err(CompletionCapacity)`.
- **영향 범위:** 공개 L4 관리 경로에서 같은 completion의 정리를 동시에 호출할 때 발생한다. 현재 facade host의 직렬 호출만으로 재현되었다고 주장하지 않는다. payload 메모리 안전성 위반을 확인한 것은 아니며, 완료 slot 미회수와 생산자 수락 중단이 확인된 영향이다.
- **처리 제안:** 정리 작업을 nonblocking 진입 claim 등으로 단일 소유하고, 세대 변경·재훑기 플래그·cursor·잔여량을 같은 소유권 아래 갱신한다. 단순히 `store` 하나를 다른 원자 연산으로 바꾸는 것으로 전체 상태 경쟁이 해결되었다고 보지 않는다. 중첩 호출·새 abandonment·UPDATING 재훑기·slot 재사용을 함께 검증한다.

#### [처리담당]

- 판정 결과: **수용** (2026-11-12, 처리담당 AI). router turn(`routing/mod.rs`)과 control poll(`control/mod.rs`)이 같은 completion을 각자 정리하므로 내부 경로에서도 겹칠 수 있다.
- 처리 내용: `Registry.sweeping` 진입 claim(비대기 CAS) 하나로 정리 상태(`swept`·`sweep_left`·`sweep_missed`·`cursor`) 전체를 한 호출만 갱신한다. 겹친 호출은 0을 반환하고, 그 사이 새 abandonment 세대는 다음 호출이 감지한다. claim은 Drop guard로 unwind에서도 해제한다. 리뷰어의 정지 지점(잔여량 저장 직전)에 test-only thread-local hook을 두고 재현 순서를 저장소 시험 `completion::tests::a_stale_sweep_cannot_erase_a_new_abandonment`로 옮겼다. 중첩 호출·새 abandonment·slot 재사용은 이 시험이, UPDATING 재훑기는 기존 `an_abandoned_slot_skipped_while_updating_is_reconciled_later`가 확인한다. API §19.7 ⑪ 항목에 소유권 규칙을 추가했다.
- 검증 근거(EV·시험·커밋): [EV-072](../verification_evidence.md#ev-072). 수정 전 Red(`got Err(CompletionCapacity)`) → Green, Miri 8 seeds. 커밋 `aa62e04`.

### RV-132 — 비 Linux calibration 대체 경로가 컴파일되지 않음 (P2)

#### [리뷰어]

- **위치:** [host::read](../../../crates/dot-logger/src/calibrate.rs:662), [CLI 의존성](../../../crates/dot-logger-cli/Cargo.toml). 도입: `a9dfa0c`.
- **원인:** `let none = || Fact::Unavailable(...)`는 제네릭 함수가 아니다. 첫 `kernel: none()`이 반환형을 `Fact<String>`으로 고정한다. 같은 closure를 `preempt_rt: Fact<bool>`, CPU 목록, 메모리 수치 등에 재사용해 E0308이 발생한다. Linux에서는 이 분기가 cfg로 제거되어 전체 feature 시험에도 드러나지 않는다.
- **확인:** 원본 `Fact`·`Host` 정의와 비 Linux `Host` 생성 블록을 그대로 추출한 `/tmp/dot-logger-rev053-probe/fallback.rs`를 `rustc --edition=2024`로 컴파일하여 **E0308 8건**을 확인했다. 이는 동일 코드의 타입 검사 재현이며 Windows/macOS 전체 cross/native 빌드를 실행한 결과는 아니다.
- **영향:** 비 Linux에서 `calibrate` feature를 켠 라이브러리와, 이를 상시 의존하는 CLI가 빌드되지 않는다. 기본 core feature에는 이 모듈이 없으므로 영향이 없다. `.github/workflows/ci.yml`의 Windows/macOS feature matrix도 해당 분기를 사용한다. T3의 native 실행 시점 이월은 유지하되 확인된 소스 오류는 조치할 수 있다.
- **처리 제안:** `fn unavailable<T>() -> Fact<T>` 또는 각 필드의 `Fact::Unavailable` 생성으로 반환형을 분리한다. 이어서 Linux 한정 helper의 비 Linux lint도 확인한다. 같은 변경의 `tc_013_calibrate` 성공 측정 시험과 CLI `calibrate_saves_a_new_profile_and_never_overwrites`에는 Linux 한정 조건이 없으므로, 비 Linux에서 `linux_available_bytes`가 `Unsupported`를 반환한다는 §19.9 계약에 맞게 성공 시험과 대체 경로 시험을 구분한다. 플랫폼 공통 JSON·preset 시험은 유지한다.

#### [처리담당]

- 판정 결과: **수용** (2026-11-12).
- 처리 내용: 비 Linux 대체 값을 제네릭 `fn none<T>() -> Fact<T>`로 바꿨다. Linux에서만 쓰는 `host::parse_cpus`는 `cfg(target_os = "linux")`로 묶었다(비 Linux dead_code 방지). §19.9 계약대로 측정 성공 시험 4개(`tc_013_calibrate` F19 write/time/reserve 2종)와 CLI `calibrate_saves_a_new_profile_and_never_overwrites`는 Linux 한정으로 표시했다. 비 Linux 대체 경로 시험 `f19_other_platforms_fail_unsupported_without_files`·CLI `calibrate_on_other_platforms_fails_without_writing`(exit 4·파일 없음·기존 프로필 Conflict 유지)를 추가했다. 누락 디렉터리 기대값은 플랫폼별(NotFound/Unsupported)이다.
- 검증 근거(EV·시험·커밋): [EV-072](../verification_evidence.md#ev-072). nightly `-Zbuild-std` 타입 검사로 macOS·Windows Red(E0308 8건) → `-D warnings` 0(lib + 전체 시험 대상). CLI 크레이트는 C 의존성(ring·zstd) 때문에 이 PC에서 교차 검사하지 못했다 — native(T3). 커밋 `aa62e04`.

### RV-133 — 기본 feature 문서의 선택 API 링크가 깨짐 (P3)

#### [리뷰어]

- **위치:** [Sizing::absorb 문서](../../../crates/dot-logger/src/logger/preset.rs:75). 도입: `a9dfa0c`.
- **문제:** `absorb`는 항상 공개되지만 링크 대상 `profile`은 `#[cfg(feature = "calibrate")]`일 때만 존재한다. 기본 feature로 문서를 생성하면 `unresolved link to Self::profile` 경고가 발생한다.
- **재현:** `RUSTDOCFLAGS='-D warnings' cargo doc -p dot-logger --no-default-features --no-deps` → exit 101, `preset.rs:75`의 `rustdoc::broken-intra-doc-links`. 현재 CI 문서 명령은 `--all-features`여서 놓친다.
- **처리 제안:** 조건부 문서 또는 링크 없는 코드 표기를 사용하고 core-only rustdoc도 확인한다. 실행 동작에는 영향이 없다.

#### [처리담당]

- 판정 결과: **수용** (2026-11-12).
- 처리 내용: 링크를 코드 표기 `profile`(feature `calibrate`)로 바꿨다. CI `contracts`에 core-only rustdoc(`-p dot-logger --no-default-features`, `-D warnings`) 단계를 추가해 같은 누락을 막는다.
- 검증 근거(EV·시험·커밋): [EV-072](../verification_evidence.md#ev-072). core-only·all-features rustdoc `-D warnings` 모두 exit 0. 커밋 `aa62e04`.

## 5. 검증 및 인계

| 검증 | 결과 |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Pass, 경고 0 |
| `cargo test --workspace --all-features` (1.96.0) | Pass — 559 passed, 5 ignored(기존 외부 DB 시험) |
| `cargo +1.85.0 test --workspace --all-features --locked` | Pass — 559 passed, 5 ignored |
| RV-131 정지 순서 고정 진단(저장소 밖 복사본) | 의도한 Red — seq5가 CompletionCapacity로 거절됨 |
| RV-132 비 Linux 생성 블록 추출 타입 검사 | 의도한 Red — E0308 8건 |
| core-only rustdoc + `-D warnings` | Fail — RV-133 |
| vendor `src/` ↔ 로컬 registry 원본 비교 | 기록된 단일 CRC 패치의 두 hunk만 존재. chunk/data CRC 검증 시험은 위 전체 회귀에 포함 |
| 제품 코드·저장소 테스트 변경 | 없음 |
| Miri·개별 feature 전체 조합·native·정식 성능/RT 재측정 | 이번 회차 미실행. 기존 T3·TASK-022~024·실앱 후속 유지 |

실행 로그: `/tmp/dot-logger-rev053-{tests,msrv,fmt,clippy,race,fallback,doc}.log`. `/tmp`의 영구 보존은 전제하지 않으며 핵심 재현 조건과 관측값은 위 상세에 기록했다. 이 회차에서는 재부팅이나 대용량 성능 시험을 실행하지 않았다. 새로 도래한 T 항목은 없다.

다음 개발 세션은 RV-131의 결정적 실패 재현을 저장소 시험으로 옮겨 조치하고, RV-132·133의 플랫폼/feature 경계를 정렬한다. 각 `[처리담당]`에 판정·수정·EV를 기록하여 `Open → In Progress → Resolved → Closed`로 추적한 뒤 TASK-029로 이어간다. 기존 승인 범위의 구현·검증 위임을 유지하며 이번 리뷰에서 새 설계 결정을 승인한 것으로 간주하지 않는다.

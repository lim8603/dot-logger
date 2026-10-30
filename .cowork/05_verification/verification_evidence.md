# Verification Evidence

> 검증 근거 인덱스 — 테스트, 리뷰, NFR, 릴리즈 readiness 근거를 한곳에서 요약하고 연결한다

---

## 목적

이 문서는 Verify 단계에서 생성되는 근거를 **게이트 판정용 canonical 인덱스**로 정리한다.

- 세션 로그나 개별 Task 문서에 흩어진 검증 근거를 빠르게 복원한다
- Gate 4, Gate 5 판정 시 "무엇이 검증되었고 어디에 근거가 있는가"를 한 번에 보여 준다
- 원본 로그, 스크린샷, 외부 리포트를 이 문서에 복제하지 않고 **요약 + 위치 링크**만 유지한다
- TDD처럼 테스트량이 누적되는 프로젝트에서도, 이 문서는 로그 저장소가 아니라 **신뢰 가능한 증거 인덱스**로 유지한다

---

## 문서 정보

| 항목 | 내용 |
|------|------|
| 관련 Intent | INT-001 |
| 관련 Milestone | MS-002·003 |
| 관련 Test Strategy | `test_strategy.md` |
| 관련 Test Case | `test_case.md` |
| 버전 | 2026-10-30 / EV-001~017 |

---

## 운영 원칙

- 증거의 유효 범위는 **대상 commit/빌드·실행일·환경/구성·시나리오**로 명시한다. 명령/절차와 결과 위치를 연결하고, 실행 주체가 AI인지 Human인지 구분한다. dirty 작업 트리면 변경 범위도 남긴다.
- 전체 테스트 개수나 로컬 Pass를 제품 전체/다른 환경의 Pass로 확장하지 않는다. 설정·권한·배포물 변경은 선언, 소비 코드, 실제 사용 환경까지 이어지는 경로를 확인한다. 예: 설정 scope 변경이면 단일/다중 workspace와 설치 후 reload 경로 중 영향을 받는 것을 검사한다.
- 관련 코드·계약·환경이 달라지면 이전 Pass는 이력으로 보존하고 현재 판정은 재검증 또는 미검증으로 표시한다. 관련 없는 변경 때문에 전체 테스트를 반복하지 않는다.
- CI/수동 검증을 연기하면 **마지막 검증 revision, 미검증 범위, 담당, 재실행 시점/조건**을 `Open Evidence Gaps`에 남긴다. 비용 때문에 연기한 검증도 Pass로 바꾸지 않는다. 공유 상태에는 공백 ID와 링크만 둔다.
- Task 완료·프로젝트 건강 상태·릴리즈 준비 상태를 구분한다. Green은 Gate 통과 증거가 아니다. 게이트는 해당 대상과 필수 범위의 유효한 근거로 판단하고, 수용한 예외는 승인자·이유·후속 조건을 기록한다.
- 검증 결과나 이월 조건이 바뀐 실행 사이클 종료 시 관련 EV/GAP와 현재 Gate 요약을 갱신한다. 과거 Gate 판정은 버전/날짜를 표시해 현재 판정과 구분한다.

- 이 문서는 Verify/Release 판정의 **증거 요약 인덱스**다.
- 상세 절차와 기대 결과는 `test_strategy.md`, `test_case.md`, `review_checklist.md`, 관련 `TASK-*` 문서에서 관리한다.
- 원본 실행 로그, 외부 리포트, 측정 결과 원문은 원래 위치에 두고, 이 문서에는 무엇을 입증하는지와 위치만 적는다.
- 가능하면 Gate 4, Gate 5 항목은 `EV-*` ID로 추적한다.
- 근거가 아직 부족하면 삭제하지 말고 `Open Evidence Gaps`에 남긴다.

---

## 근거 영역 요약

INT-001 / 2026-10-28 / Build, MS-002 In Progress. TASK-002/003은 Human 리뷰·로컬/native Pass 이력을 유지한다. TASK-004 원본 풀·반환은 EV-003 로컬 Pass·Human 승인·게시 완료, 사용량 소진으로 native 미실행이다. Human은 계정 재확인 불필요·TASK-005 착수를 지시했다. TASK-005 분배·출력 격리·예산은 EV-004 로컬 Pass이며 Human 리뷰는 승인됐고 native는 대기다. TASK-006 완료 추적·명시 sync·종료 coordinator는 EV-005 로컬 Pass·Human 리뷰 승인/native 대기다. 실제 파일/worker·Windows/macOS/PREEMPT_RT·장비 검증은 남아 있다.

## Evidence Index

### EV-001 — TASK-002 구성·공개 API·도구체인·의존성

- 대상: 코드 커밋 `2f9015d`. `23a499c` 위 작업 트리에서 검증한 소스와 commit blob의 SHA-256이 일치함을 세션 마감 시 확인했다. [기계 판독 결과·소스 SHA-256](../06_evolution/imported_context/task_002_2026-10-26/results.json)에 각 소스/manifest·lock/CI/script의 검증 대상 hash를 기록했다.
- 실행: 2026-10-26 / AI / Ubuntu 26.04.1 x86_64 / 개발 Rust 1.96.0·MSRV 1.85.0 / 실제 R1·로봇 수락 환경 아님.
- TDD: [API 미구현 Red](../06_evolution/imported_context/task_002_2026-10-26/red-missing-api.log), [잘못된 구성 수락 Red](../06_evolution/imported_context/task_002_2026-10-26/red-behavior.log) 후 최소 검증 구현. 최종 구성 테스트 13개, 전체 feature 의존성 probe 6개, 불변 구성 compile-fail doctest 1개 Pass. feature별 활성 probe 수는 다르다.

| 범위·명령 | 결과·원문 |
|---|---|
| `python3 scripts/check-feature-matrix.py --toolchain 1.96.0` | 16 조합·독립 public consumer·core에 네트워크/압축 의존 없음 Pass / [로그](../06_evolution/imported_context/task_002_2026-10-26/features-1.96.0.log) |
| 같은 script `--toolchain 1.85.0` | 16 조합·consumer·doctest Pass / [로그](../06_evolution/imported_context/task_002_2026-10-26/features-1.85.0.log) |
| `cargo fmt --all --check` | Pass |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | [1.96.0 Pass](../06_evolution/imported_context/task_002_2026-10-26/clippy-1.96.0.log), [1.85.0 Pass](../06_evolution/imported_context/task_002_2026-10-26/clippy-1.85.0.log) |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked` | [1.96.0 Pass](../06_evolution/imported_context/task_002_2026-10-26/rustdoc.log) |
| `cargo audit --no-fetch --deny unsound --json` / cargo-audit 0.22.2 | [최종 결과](../06_evolution/imported_context/task_002_2026-10-26/audit.json): 취약점 0·unsound 0·unmaintained 1(`paste`, GAP-005). DB `a78282a1414d6c28cc228c4eae3570451affb0dc`, 2026-10-26 갱신 |
| Cargo metadata·lock·기존 dot-probe 비교 | [외부 177개 라이선스/버전 목록](../06_evolution/imported_context/task_002_2026-10-26/dependencies.json), license metadata 누락 0. 배포 라이선스/고지 완료 판정은 아님 |

MCAP 0.25.0 선택 zstd 경로의 취약점 3·zstd-safe unsound 10건은 Human 승인 ADR-010의 MCAP 0.26.0 적용 후 해소됐다. Tokio unsound 1건은 1.51.6 patch로 해소했다. 과거 선택 기준과 변경 근거는 ADR-009·010/세션 로그에 보존한다.

유효 범위는 TASK-002의 시작 전 구성/API·의존성 호환뿐이다. runtime `build`·수집·worker 적용·실제 파일 API·실제 할당 상한·내구성·종료는 후속 Task다. TC-001의 CLI 대조/실행 자원 정리와 다른 TC·실장비는 이 결과로 Pass 처리하지 않는다. 로컬 결과와 아래 native CI 결과의 환경 범위를 구분한다.

### native CI — 2026-10-27

- GitHub: [비공개 저장소](https://github.com/lim8603/dot-logger) / [Rust contracts](https://github.com/lim8603/dot-logger/actions/runs/40504484501) / run `40504484501`, attempt 1, push 이벤트 / 최종 `success`. 기존 main 이력 7개 커밋을 보존했다.
- 대상: `f9894f25f7b43eee6faaa47c98ecd62cb61ff797`. 코드·테스트·workflow·manifest/lock·script의 SHA-256이 기존 로컬 검증 revision `2f9015d1290b8e9fc8bc64076066ce6f8cf46885`와 모두 일치함을 확인했다. 문서 이력만 추가된 revision이다.
- 실행 주체: AI가 게시·결과 수집, GitHub hosted runner가 실제 실행. [실행 metadata·step 결과·소스/log SHA-256](../06_evolution/imported_context/task_002_native_ci_2026-10-27/results.json)과 각 원본 job 로그를 보존했다.

| 조합 / rustc host | 결과 | 근거 |
|---|---|---|
| contracts (windows-2025, 1.85.0) / `x86_64-pc-windows-msvc` | Pass — 16 feature·consumer·clippy·rustdoc, fmt는 1.96.0 | [job](https://github.com/lim8603/dot-logger/actions/runs/40504484501/job/122190178028) · [원본 로그](../06_evolution/imported_context/task_002_native_ci_2026-10-27/contracts-windows-2025-1.85.0.log) |
| contracts (macos-14, 1.96.0) / `aarch64-apple-darwin` | Pass — 16 feature·consumer·clippy·rustdoc, fmt는 1.96.0 | [job](https://github.com/lim8603/dot-logger/actions/runs/40504484501/job/122190178126) · [원본 로그](../06_evolution/imported_context/task_002_native_ci_2026-10-27/contracts-macos-14-1.96.0.log) |
| contracts (ubuntu-24.04, 1.85.0) / `x86_64-unknown-linux-gnu` | Pass — 16 feature·consumer·clippy·rustdoc, fmt는 1.96.0 | [job](https://github.com/lim8603/dot-logger/actions/runs/40504484501/job/122190178216) · [원본 로그](../06_evolution/imported_context/task_002_native_ci_2026-10-27/contracts-ubuntu-24.04-1.85.0.log) |
| contracts (ubuntu-24.04, 1.96.0) / `x86_64-unknown-linux-gnu` | Pass — 16 feature·consumer·clippy·rustdoc, fmt는 1.96.0 | [job](https://github.com/lim8603/dot-logger/actions/runs/40504484501/job/122190178235) · [원본 로그](../06_evolution/imported_context/task_002_native_ci_2026-10-27/contracts-ubuntu-24.04-1.96.0.log) |
| contracts (windows-2025, 1.96.0) / `x86_64-pc-windows-msvc` | Pass — 16 feature·consumer·clippy·rustdoc, fmt는 1.96.0 | [job](https://github.com/lim8603/dot-logger/actions/runs/40504484501/job/122190178270) · [원본 로그](../06_evolution/imported_context/task_002_native_ci_2026-10-27/contracts-windows-2025-1.96.0.log) |
| contracts (macos-14, 1.85.0) / `aarch64-apple-darwin` | Pass — 16 feature·consumer·clippy·rustdoc, fmt는 1.96.0 | [job](https://github.com/lim8603/dot-logger/actions/runs/40504484501/job/122190178345) · [원본 로그](../06_evolution/imported_context/task_002_native_ci_2026-10-27/contracts-macos-14-1.85.0.log) |
| audit / Ubuntu 24.04·Rust 1.96.0·cargo-audit 0.22.2 | Pass — `cargo +1.96.0 audit --deny unsound`, 취약점/unsound 0·기존 paste 경고 1 | [job](https://github.com/lim8603/dot-logger/actions/runs/40504484501/job/122190177723) · [원본 로그](../06_evolution/imported_context/task_002_native_ci_2026-10-27/audit.log) |

6개 구성 작업과 audit 총 7개 작업 Pass. GAP-004는 이 hosted native CI 범위에서 Closed다. Windows Server 2025는 Windows 11 수락을 대신하지 않으며 Ubuntu hosted 환경도 PREEMPT_RT·실제 장비/성능/UPS 수락 근거가 아니다. GAP-001·002·005는 유지한다. TASK-002는 Human 구현 리뷰·기존 로컬 Pass·이 native CI를 근거로 Done이다.

결과 문서·원본 로그만 추가하는 후속 `docs(cowork)` 커밋은 `[skip ci]`로 게시한다. 코드·테스트·CI·lock의 검증 hash는 유지되므로 관련 없는 문서 변경으로 동일 검사를 반복하지 않는다.

### Human 구현 리뷰 — 2026-10-27

사용자가 TASK-002 요약보고 후 「승인하고」로 구현 리뷰를 승인했다. 대상 구현은 `2f9015d`, 로컬 검증은 위 EV-001이다. GitHub MCP 연결·본인 계정 저장소 생성·업로드도 요청했다. native CI는 같은 날 위 7개 작업 Pass로 완료했고 GAP-004를 hosted native CI 범위에서 Closed로 갱신했다.

### EV-002 — TASK-003 소형 수집·예약·일반 대기

- 대상: base `ebb3819524b379d43767ef2caa0f58e693546adb` 위 dirty 작업 트리. capture 모듈/통합 시험·고정 오류 resource·공개 consumer·test-only allocator guard·workspace/lock/script 변경. [결과·20개 소스/manifest SHA-256](../06_evolution/imported_context/task_003_2026-10-27/results.json)으로 검증 내용을 고정했다. 외부 의존성 pin은 변경하지 않았다.
- 코드 커밋: `39f9c728f8c0bcfc433f86c0b97c609caf98d494`. EV-002의 20개 소스/manifest SHA-256과 commit blob 일치를 확인했다. Human 구현 리뷰는 2026-10-27 「승인」으로 완료됐다.
- 실행: 2026-10-27 / AI / Ubuntu 26.04.1 x86_64 / Rust 1.96.0·MSRV 1.85.0. 실제 R1·PREEMPT_RT 환경이 아니다.
- TDD: [capture API 부재 Red](../06_evolution/imported_context/task_003_2026-10-27/red-missing-api.log), [포화/deadline 잘못된 수락 Red](../06_evolution/imported_context/task_003_2026-10-27/red-behavior.log), [확장 동작 Red](../06_evolution/imported_context/task_003_2026-10-27/red-extended.log) → 최소 구현·리팩토링 → [최종 workspace all-features Green](../06_evolution/imported_context/task_003_2026-10-27/green-final.log).
- 제품 코드: [`capture`](../../crates/dot-logger/src/capture/mod.rs), [고정 입력/결과 타입](../../crates/dot-logger/src/capture/types.rs). [소형 통합 시험](../../crates/dot-logger/tests/tc_002_capture.rs) 16개, [private 경계](../../crates/dot-logger/src/capture/tests.rs) 2개, 신규 수명 compile-fail 4개, [allocator guard](../../tests/rt-allocation/src/tests.rs) 1개 Pass. 기존 구성·의존성 회귀도 통과했다.

| 범위·명령 | 결과·원문 |
|---|---|
| `python3 scripts/check-feature-matrix.py --toolchain 1.96.0` | 16 feature·독립 public consumer 수집 왕복·core 의존 격리·allocator guard Pass / [로그](../06_evolution/imported_context/task_003_2026-10-27/features-1.96.0.log) |
| 같은 script `--toolchain 1.85.0` | 같은 범위 Pass / [로그](../06_evolution/imported_context/task_003_2026-10-27/features-1.85.0.log) |
| `cargo fmt --all --check`, `git diff --check` | Pass / [fmt](../06_evolution/imported_context/task_003_2026-10-27/fmt.log) |
| `cargo +<version> clippy --workspace --all-targets --all-features --locked -- -D warnings` | [1.96.0 Pass](../06_evolution/imported_context/task_003_2026-10-27/clippy-1.96.0.log), [1.85.0 Pass](../06_evolution/imported_context/task_003_2026-10-27/clippy-1.85.0.log) |
| `RUSTDOCFLAGS='-D warnings' cargo +<version> doc --workspace --all-features --no-deps --locked` | [1.96.0 Pass](../06_evolution/imported_context/task_003_2026-10-27/rustdoc-1.96.0.log), [1.85.0 Pass](../06_evolution/imported_context/task_003_2026-10-27/rustdoc-1.85.0.log) |

수락/포화·동등 취급·명시/채널/레벨 중요도·입력 mutation 불영향·생산자별 FIFO·2생산자 8,000건 동시 수집·고정 metadata·채널/schema/크기 오류·필터 생략·일반 deadline/재개/close·예산 경계·sequence 소진/카운터 wrap을 검증했다. Important bytes는 Normal 미소비 슬롯 수 × 최대 슬롯 bytes를 제한해 보호한다. envelope/bytes는 safe rtrb chunk API로만 접근하며 bytes 게시 후 envelope 게시, bytes 반환 후 envelope 반환 순서를 유지한다.

allocator guard는 TLS의 실제 할당/해제 감지 보정 후 소형 수락·거절·생략·close·마지막 borrowed 핸들 소멸 구간 **0 allocation / 0 deallocation**을 확인했다. owner 최종 해제는 일반 경로에서 실제 deallocation을 확인했다. instrumentation의 `GlobalAlloc` unsafe는 독립 test-only package의 해당 모듈에 한정하며 제품 crate의 `unsafe_code = forbid`는 그대로다. 저장소 예열은 메모리 잠금/실제 RT 지연 합격을 뜻하지 않는다.

유효 범위는 소형 ingress다. `CaptureReceipt`는 입력 인수만 나타내며 최종 SessionOwner/build·원본 pool/loan·fanout·완료 window·sync/실제 shutdown·파일 API·p99·장비/전원 검증은 미실행이다. TC-002 전체와 Gate 4/5를 Pass로 바꾸지 않는다. 원본 메모리 순서 모델은 TASK-004다. Human 리뷰는 2026-10-27 「승인」으로 완료됐다. 아래 변경분 native CI·audit도 Pass다. TASK-002의 기존 Pass는 해당 revision의 이력으로 유지하며 GAP-004는 현재 변경분까지 hosted native CI 범위 Closed다.

### TASK-003 native CI — 2026-10-27

- [Rust contracts](https://github.com/lim8603/dot-logger/actions/runs/40512793054) / run `40512793054`, attempt 1, push 이벤트 / 최종 `success`.
- 대상 `902043b9b5c001c264bcb9ee65cd16aa1985e822`와 코드 `39f9c728f8c0bcfc433f86c0b97c609caf98d494`의 20개 소스/manifest SHA-256이 EV-002 로컬 검증 내용과 일치한다. 승인 후 소스 변경은 없다.
- 실행 주체: AI가 게시·결과 수집, GitHub hosted runner가 실제 실행. [metadata·step 결과·소스/log SHA-256](../06_evolution/imported_context/task_003_native_ci_2026-10-27/results.json)과 7개 원본 job 로그를 보존했다.

| 조합 | 결과 | 근거 |
|---|---|---|
| audit | Pass — 취약점/unsound 0·paste 유지보수 경고 1(GAP-005) | [job](https://github.com/lim8603/dot-logger/actions/runs/40512793054/job/122218654106) · [로그](../06_evolution/imported_context/task_003_native_ci_2026-10-27/audit.log) |
| contracts (macos-14, 1.85.0) | Pass — 16 feature·consumer·할당 감시·clippy·rustdoc; 개발 Rust는 fmt 포함 | [job](https://github.com/lim8603/dot-logger/actions/runs/40512793054/job/122218654641) · [로그](../06_evolution/imported_context/task_003_native_ci_2026-10-27/macos-14-1.85.0.log) |
| contracts (macos-14, 1.96.0) | Pass — 16 feature·consumer·할당 감시·clippy·rustdoc; 개발 Rust는 fmt 포함 | [job](https://github.com/lim8603/dot-logger/actions/runs/40512793054/job/122218654675) · [로그](../06_evolution/imported_context/task_003_native_ci_2026-10-27/macos-14-1.96.0.log) |
| contracts (ubuntu-24.04, 1.85.0) | Pass — 16 feature·consumer·할당 감시·clippy·rustdoc; 개발 Rust는 fmt 포함 | [job](https://github.com/lim8603/dot-logger/actions/runs/40512793054/job/122218654773) · [로그](../06_evolution/imported_context/task_003_native_ci_2026-10-27/ubuntu-24.04-1.85.0.log) |
| contracts (ubuntu-24.04, 1.96.0) | Pass — 16 feature·consumer·할당 감시·clippy·rustdoc; 개발 Rust는 fmt 포함 | [job](https://github.com/lim8603/dot-logger/actions/runs/40512793054/job/122218654848) · [로그](../06_evolution/imported_context/task_003_native_ci_2026-10-27/ubuntu-24.04-1.96.0.log) |
| contracts (windows-2025, 1.85.0) | Pass — 16 feature·consumer·할당 감시·clippy·rustdoc; 개발 Rust는 fmt 포함 | [job](https://github.com/lim8603/dot-logger/actions/runs/40512793054/job/122218654800) · [로그](../06_evolution/imported_context/task_003_native_ci_2026-10-27/windows-2025-1.85.0.log) |
| contracts (windows-2025, 1.96.0) | Pass — 16 feature·consumer·할당 감시·clippy·rustdoc; 개발 Rust는 fmt 포함 | [job](https://github.com/lim8603/dot-logger/actions/runs/40512793054/job/122218654649) · [로그](../06_evolution/imported_context/task_003_native_ci_2026-10-27/windows-2025-1.96.0.log) |

6개 구성 작업과 audit 총 7개 작업 Pass. TASK-003 Done·GAP-004 Closed(hosted native CI 범위). Windows Server 2025는 Windows 11, Ubuntu hosted는 PREEMPT_RT/실장비 수락을 대신하지 않는다. 원본 풀·회수/메모리 순서 모델·분배/완료·파일·p99·전원 차단 검증은 잔여다. GAP-001·002·005와 Gate 4/5 Not Ready를 유지한다.

### EV-003 — TASK-004 원본 풀·수명·반환

- 대상: base `98580cb3e9a42576a9cf93fb2224b740e8f9a497` 위 dirty 작업 트리. pool·capture 연동·원본 시험/allocator guard·scope 예제·package lint·CI/feature script 변경. [기계 판독 결과·24개 소스/manifest SHA-256](../06_evolution/imported_context/task_004_2026-10-27/results.json)으로 검증 대상을 고정했다. Cargo.lock·외부 dependency는 변경하지 않았다.
- 코드 커밋 `9a4f34cbfc540d69aee0e8504238bf53760ec925`와 EV-003의24개 source/manifest SHA-256이 일치한다. Human 구현 리뷰·게시 요청은 2026-10-27 승인됐다.
- 실행: 2026-10-27 / AI / Ubuntu 26.04.1 x86_64 / Rust 1.96.0·MSRV 1.85.0 / 검증 전용 Miri nightly-2026-10-26. 실제 R1·PREEMPT_RT 수락 환경이 아니다.
- TDD: [원본 API 부재 Red](../06_evolution/imported_context/task_004_2026-10-27/red-missing-api.log), [회수 미구현 Red](../06_evolution/imported_context/task_004_2026-10-27/red-behavior.log), [범위 밖 generation alias Red](../06_evolution/imported_context/task_004_2026-10-27/red-generation.log) 후 구현·리팩토링 → [최종 workspace Green](../06_evolution/imported_context/task_004_2026-10-27/green-final.log).
- 코드: [원본 풀/공개 guard](../../crates/dot-logger/src/pool/mod.rs), [원자 상태·UnsafeCell Safety 경계](../../crates/dot-logger/src/pool/storage.rs), [생산자별 혼합 ingress](../../crates/dot-logger/src/capture/mod.rs). [원본 통합 시험](../../crates/dot-logger/tests/tc_002_pool.rs) 17개, 신규 storage 경계 4개, 신규 compile-fail 7개, [원본 allocation guard](../../tests/rt-allocation/src/tests.rs) 1개 Pass. 기존 소형·구성·의존성 회귀도 Pass다.

| 명령·범위 | 결과·근거 |
|---|---|
| `python3 scripts/check-feature-matrix.py --toolchain 1.96.0` | 16 feature·독립 consumer·기존/원본 allocator guard·scoped_capture 예제 Pass / [로그](../06_evolution/imported_context/task_004_2026-10-27/features-1.96.0.log) |
| 같은 script `--toolchain 1.85.0` | 같은 범위 Pass / [로그](../06_evolution/imported_context/task_004_2026-10-27/features-1.85.0.log) |
| `cargo +<version> clippy --workspace --all-targets --all-features --locked -- -D warnings` | [1.96.0 Pass](../06_evolution/imported_context/task_004_2026-10-27/clippy-1.96.0.log), [1.85.0 Pass](../06_evolution/imported_context/task_004_2026-10-27/clippy-1.85.0.log) |
| `RUSTDOCFLAGS='-D warnings' cargo +<version> doc --workspace --all-features --no-deps --locked` | [1.96.0 Pass](../06_evolution/imported_context/task_004_2026-10-27/rustdoc-1.96.0.log), [1.85.0 Pass](../06_evolution/imported_context/task_004_2026-10-27/rustdoc-1.85.0.log) |
| `MIRIFLAGS='-Zmiri-strict-provenance -Zmiri-many-seeds=0..8' cargo +nightly-2026-10-26 miri test -p dot-logger --lib --test tc_002_pool --locked` | 8 seeds × lib 6개(기존2·신규4)+원본17개 Pass / [로그](../06_evolution/imported_context/task_004_2026-10-27/miri.log) |
| `cargo fmt --all --check`, `git diff --check` | Pass / [fmt](../06_evolution/imported_context/task_004_2026-10-27/fmt.log) |

원본 거절/생략은 동일 slot·generation·pointer·내용의 Loan을 반환한다. 대여 단계부터 Normal의 Important bytes 점유를 막고, 수락 뒤 `try_take_raw`의 불변 reader는 같은 payload pointer를 사용한다. `try_read`의 일반 복사 경로는 별도다. Queued/InUse/Loaned는 회수하지 않고 마지막 guard/borrow 종료 뒤 ReturnPending만 한정 스캔으로 Free로 바꾼다. 원본과 소형은 한 FIFO/sequence를 사용하며 물리 원본 payload·슬롯 metadata·큐 예산을 중복 계산하지 않는다.

원본 allocator guard는 대여·거절·생략·포화된 반환 알림·reader Drop·마지막 RT handle Drop 구간 **allocation/deallocation 0/0**, owner 일반 Drop의 실제 deallocation을 확인했다. 사전 할당/예열은 page locking이나 실제 RT 지연 합격을 뜻하지 않는다. generation+state를 한 atomic word로 비교하고, 범위 밖/stale/중복 token을 차단하며 소진 시 Retired로 유지한다. workspace forbid는 유지하되 dot-logger package만 deny/private pool-storage allow로 설정했다. Sync/UnsafeCell 접근은 해당 모듈에 격리했고 나머지 RT 경로에 callback/I/O/잠금/무한 재시도를 넣지 않았다. 이 unsafe 예외도 Human 구현 리뷰에 포함한다.

Miri는 [공식 사용법](https://github.com/rust-lang/miri#using-miri)과 [다중 seed 실행](https://github.com/rust-lang/miri#testing-multiple-different-executions)에 따라 실행했다. 위 Pass는 실행한 입력/seed의 해석 결과이며 모든 interleaving의 완전한 모델 검사나 soundness 증명이 아니다. 신규 raw-memory CI job에도 동일 명령을 넣었으나 이 변경분 native CI는 미실행이다.

**dot-probe 수명 검토:** 현재 process/control/safety의 'static spawn과 shutdown join 경로를 읽기 전용으로 조사했다. 관측 revision·4개 파일 SHA-256은 results.json에 보존했다. borrowed Producer의 직접 주입에는 생성부/run entry point 변경이 필요하다. [scoped_capture 예제](../../examples/scoped_capture.rs)는 owner를 가진 'static 관리 host 내부의 scoped RT worker·join→drain/reclaim→owner Drop을 두 Rust에서 실제 실행했다. 실제 앱 연결/실장비 검증은 TASK-026이며 dot-probe 파일은 수정하지 않았다. T1은 미도래다.

유효 범위는 원본 풀·입력 수집·수명/반환과 수명 prototype이다. fanout·대상별 완료/이력·파일/durability·실제 shutdown·실제 dot-probe 어댑터·p99/장비/전원 수락은 미실행이다. Human 리뷰 승인·native CI 대기·변경분 native CI 대기(기존 6개 OS/Rust + audit + raw-memory = 8개 job)이며 TC-002 전체·MS-002·Gate 4/5를 Pass로 바꾸지 않는다. 새 RustSec audit은 미실행이고 외부 dependency pin은 동일하다.

### TASK-004 native CI 시도·계정 사유 차단 — 2026-10-27

- 코드 `9a4f34cbfc540d69aee0e8504238bf53760ec925`·승인 문서 `f0ce6eea84004bf32d2a73c675b919bfa0507ad8` 게시 후 [run 40520558735](https://github.com/lim8603/dot-logger/actions/runs/40520558735), attempt 1, push 이벤트 / GitHub 결론 failure.
- 6개 OS/Rust + audit + raw-memory 총 **8개 job 모두 runner_name 비어 있음·steps 0개**다. 공통 failure annotation은 최근 account payments 실패 또는 spending limit 증액 필요를 안내한다. 둘 중 실제 계정 원인은 확정하지 않았다. [원본 run/job/check annotations](../06_evolution/imported_context/task_004_native_ci_2026-10-27/failure.json)·[분류/24개 source hash/다음 조건](../06_evolution/imported_context/task_004_native_ci_2026-10-27/results.json)을 보존했다.
- CI 대상과 EV-003 source/manifest 24개 hash 일치. 시험 실행 전 인프라 차단이므로 제품/native 시험·fresh audit·CI Miri는 **Not Run / Not Verified**다. 첫 job 로그 API는 404(blob 없음)였고 실행 로그를 생성하지 않는다. 기존 로컬/Miri Pass·Human 승인은 유지한다.
- TASK-004는 In Progress·GAP-004 Open으로 남긴다. Human이 GitHub Billing & plans의 결제/한도 조건을 확인한 뒤 동일 workflow를 재실행해 8개 job을 확인한다. 계정/과금 설정을 변경하지 않았고 원인이 그대로인 재시도는 하지 않았다. TASK-003의 마지막 native Pass는 기존 revision의 이력이다.

### EV-004 — TASK-005 분배·출력 격리·예산

- 대상: base `694464e` 위 dirty 작업 트리. routing/·capture peek/등록 metadata·Output resource·독립 consumer·allocator 계측·raw-memory CI recipe. [기계 판독 결과·27개 source/manifest SHA-256](../06_evolution/imported_context/task_005_2026-10-28/results.json)으로 소스 범위를 고정했다. 외부 dependency/Cargo.lock·private pool/storage unsafe 경계·RT producer/loan API는 변경하지 않았다.
- 실행: 2026-10-28 / AI / Ubuntu 26.04.1 x86_64 / Rust 1.85.0·1.96.0 / Miri nightly-2026-10-26. R1·PREEMPT_RT·native Windows/macOS 환경이 아니다.
- TDD: API 부재는 [보조 기록](../06_evolution/imported_context/task_005_2026-10-28/red-missing-api.log)이며, [실제 분배 미구현 Red](../06_evolution/imported_context/task_005_2026-10-28/red-behavior.log) → 최소 분배 구현, [불충분한 small copy 예산 Red](../06_evolution/imported_context/task_005_2026-10-28/red-boundaries.log) → startup 거절, [byte quantum을 단일 최대 payload로 자른 문제 Red](../06_evolution/imported_context/task_005_2026-10-28/red-byte-quantum.log) → credit cap 수정, [실제 풀과 다른 설정 Red](../06_evolution/imported_context/task_005_2026-10-28/red-pool-layout.log) → concrete small/raw slots·slot bytes 등록 대조 → Green/회귀를 확인했다.
- 코드: [`routing/`](../../crates/dot-logger/src/routing/)·[`tc_003_routing.rs`](../../crates/dot-logger/tests/tc_003_routing.rs). 초기 route revision·required/optional 고정, FIFO·record/byte deficit·turn 한도, 대상별 queue/retry/active IO slots/bytes·시도/age·명시 drop/expiry/cancel, raw 공유/마지막 guard 회수, small copy/shared metadata 제한을 검증했다. 짧은 event 버퍼는 작업을 남기며 `fault`는 이전 event prefix와 실패 head를 보존한다.

| 명령·범위 | 결과·근거 |
|---|---|
| `cargo +1.96.0 test --workspace --locked` | 분배 통합19·신규 compile-fail3 포함 기존 회귀 Pass / [Green](../06_evolution/imported_context/task_005_2026-10-28/green-final.log) |
| `python3 scripts/check-feature-matrix.py --toolchain <version>` | 각16 feature·capture→routing→target guard 독립 consumer·기존 RT guard·scoped 예제 Pass / [1.85.0](../06_evolution/imported_context/task_005_2026-10-28/features-1.85.0.log), [1.96.0](../06_evolution/imported_context/task_005_2026-10-28/features-1.96.0.log) |
| `cargo +<version> clippy --workspace --all-targets --all-features --locked -- -D warnings` | 두 Rust Pass / [1.85.0](../06_evolution/imported_context/task_005_2026-10-28/clippy-1.85.0.log), [1.96.0](../06_evolution/imported_context/task_005_2026-10-28/clippy-1.96.0.log) |
| `RUSTDOCFLAGS='-D warnings' cargo +<version> doc --workspace --all-features --no-deps --locked` | 두 Rust Pass / [1.85.0](../06_evolution/imported_context/task_005_2026-10-28/rustdoc-1.85.0.log), [1.96.0](../06_evolution/imported_context/task_005_2026-10-28/rustdoc-1.96.0.log) |
| `cargo +<version> test -p dot-logger-rt-allocation --locked -- --nocapture` | 두 기존 RT guard 0/0; 신규 일반 분배 구성/생성→포화256회/512건→최종 해제 요청 bytes peak2368 / 예약2816 / 잔여0 / [1.85.0](../06_evolution/imported_context/task_005_2026-10-28/allocator-1.85.0.log), [1.96.0](../06_evolution/imported_context/task_005_2026-10-28/allocator.log) |
| `MIRIFLAGS='-Zmiri-strict-provenance -Zmiri-many-seeds=0..8' cargo +nightly-2026-10-26 miri test -p dot-logger --test tc_003_routing --locked` | 19 통합 ×8 seeds Pass, cross-thread 원본 guard 유지/최종 회수 포함 / [Miri](../06_evolution/imported_context/task_005_2026-10-28/miri-routing.log) |
| `cargo fmt --all --check`, `git diff --check` | Pass / [fmt](../06_evolution/imported_context/task_005_2026-10-28/fmt.log), [diff](../06_evolution/imported_context/task_005_2026-10-28/diff-check.log) |

물리 원본 풀은 미사용/ReturnPending 포함 raw 예약 한 번이며, output별 보유 원인(논리 bytes/refs)을 별도로 보고한다. 소형 payload는 일반 경로에서 한 번 복사해 공유하고 최종 guard 후 bytes/header 예약을 반환한다. 큐/retry/진행 IO를 합산하므로 reader가 FIFO에서 꺼내도 예산을 우회하지 않는다. 계측 peak는 해당 구성이 System allocator에 요청한 bytes이며 allocator 내부 metadata/rounding·RSS·page cache·실제 worker 비용의 측정이 아니다. Miri는 실행한 입력/8 seeds의 검사이며 모든 interleaving/메모리 안전성의 완전한 증명이 아니다.

후속: `Enqueued`는 FIFO 진입뿐이다. 완료 window/수락 예산 통합·sync·실제 shutdown·파일 writer·실제 앱/장비 검증은 TASK-006 이후다. raw IO guard를 취소/기한으로 강제 무효화하지 않으며 queued cancel/expiry 배출은 reader 구동이 필요하다. 전역 shared slots/small copy bytes 고갈은 입력을 지연할 수 있어 보편적 성능 격리 보장은 아니다. TC-003 전체·MS-002·Gate4/5 Pass로 승격하지 않는다.

Human 구현 리뷰는 2026-10-28 「승인」으로 완료됐다. native CI/fresh audit Not Run. 사용량 소진이 확인돼 계정 조회/차단 재시도 없이 로컬 검증만 수행했다. raw-memory recipe에 `--test tc_003_routing`을 추가했으나 hosted 실행 증거는 아니다. 공개 전환/계정 변경/commit/push도 수행하지 않았다. TASK-004 native 검증 이월 상태와 승인은 유지한다.

### EV-005 — TASK-006 완료 추적·명시 sync·종료

- 대상: base `694464e` 위 dirty worktree(승인된 TASK-005 + TASK-006). [결과·36개 source/manifest SHA-256 및 EV-004 이후 변경 목록](../06_evolution/imported_context/task_006_2026-10-28/results.json)으로 고정했다. 신규 completion/control·tracked capture/routing feedback·독립 consumer/allocator·CI recipe가 대상이다. 기존 [EV-004 source](../06_evolution/imported_context/task_005_2026-10-28/results.json)는 당시 승인 revision의 이력으로 보존한다. private pool/storage unsafe·dependency/Cargo.lock은 변경하지 않았다.
- 실행: 2026-10-28 / AI / Ubuntu 26.04.1 x86_64 / Rust 1.85.0·1.96.0 / Miri nightly-2026-10-26. 실제 R1·PREEMPT_RT·Windows/macOS native 환경이 아니다.
- TDD: [API 부재 보조 기록](../06_evolution/imported_context/task_006_2026-10-28/red-missing-api.log), [seq 공백/완료 게시 미구현 Red](../06_evolution/imported_context/task_006_2026-10-28/red-behavior.log), [deadline shutdown 미구현 Red](../06_evolution/imported_context/task_006_2026-10-28/red-control.log), [동일 target sync IO 중첩 Red](../06_evolution/imported_context/task_006_2026-10-28/red-overlapping-sync.log) 및 [명시 무라우팅 기록의 이력 age Red](../06_evolution/imported_context/task_006_2026-10-28/red-unrouted-age.log) 후 구현/분리·실패 경계·회귀 검증.
- 통합: 완료12·control14 = **신규26개**, private publication 경계2개·신규 guard 수명 compile-fail2개. window 포화/미완료 미재사용·sequence·동일 loan 거절/skip·Written+SyncFailure·capability·고정 range/page/permit·timeout/cancel·이력 age/retire/수량 대조·feedback Busy·stale Drop·명시 barrier/frozen cutoff·동일 target 직렬화·sync/session/epoch/generation·기한 중 IO·대여 loan·normal slot 포화 중 shutdown slot·close/record 경합을 검사했다.

| 명령·범위 | 결과·근거 |
|---|---|
| `cargo +1.96.0 test --workspace --locked` | 신규 완료/제어25·경계2·compile-fail2 및 기존 구성/소형/원본/분배 회귀 Pass / [workspace](../06_evolution/imported_context/task_006_2026-10-28/workspace.log) |
| `python3 scripts/check-feature-matrix.py --toolchain <version>` | 두 Rust 각16 feature·독립 tracked consumer·기존/신규 allocator guard·scoped 예제 Pass / [1.85.0](../06_evolution/imported_context/task_006_2026-10-28/features-1.85.0.log), [1.96.0](../06_evolution/imported_context/task_006_2026-10-28/features-1.96.0.log) |
| `cargo +<version> clippy --workspace --all-targets --all-features --locked -- -D warnings` | 두 Rust Pass / [1.85.0](../06_evolution/imported_context/task_006_2026-10-28/clippy-1.85.0.log), [1.96.0](../06_evolution/imported_context/task_006_2026-10-28/clippy-1.96.0.log) |
| `RUSTDOCFLAGS='-D warnings' cargo +<version> doc --workspace --all-features --no-deps --locked` | 두 Rust Pass / [1.85.0](../06_evolution/imported_context/task_006_2026-10-28/rustdoc-1.85.0.log), [1.96.0](../06_evolution/imported_context/task_006_2026-10-28/rustdoc-1.96.0.log) |
| `cargo +<version> test -p dot-logger-rt-allocation --locked -- --nocapture` | 기존 소형/원본·신규 tracked RT guard **allocation/deallocation 0/0**. 구성/생성부터 query+sync128회·shutdown/최종 해제 요청 bytes **peak11570 / concrete 예약14962 / 잔여0**. 기존 routing 반복 포화 guard도 Pass / [1.85.0](../06_evolution/imported_context/task_006_2026-10-28/allocator-1.85.0.log), [1.96.0](../06_evolution/imported_context/task_006_2026-10-28/allocator-1.96.0.log) |
| `MIRIFLAGS='-Zmiri-strict-provenance -Zmiri-many-seeds=0..8' cargo +nightly-2026-10-26 miri test -p dot-logger --lib --test tc_002_pool --test tc_003_routing --test tc_003_completion --test tc_003_control --locked` | lib8 + raw17 + routing19 + completion12 + control14 = **70 tests ×8 seeds =560** interpreted tests Pass / [Miri](../06_evolution/imported_context/task_006_2026-10-28/miri.log) |
| `cargo fmt --all --check`, `git diff --check` | Pass / [fmt](../06_evolution/imported_context/task_006_2026-10-28/fmt.log), [diff](../06_evolution/imported_context/task_006_2026-10-28/diff-check.log) |

유효 범위: bounded metadata admission/receipt/query・단계별 확인과 제어 coordinator다. writer 대역의 명시 acknowledgment로 capture→routing→Written→sync→shutdown을 실행했고 completion 조회만으로 sync가 시작되지 않음을 검증했다. 진행 중 IO/loan·피드백·반납이 남으면 Incomplete이며 deadline 뒤 같은 owner로 cleanup하고 실제 접근이 끝나기 전 메모리를 해제하지 않는다. Closed에도 대상 실패/불명·deadline 미준수가 보고될 수 있어 is_success를 별도로 확인한다. 앱 thread join은 애플리케이션 scoped host 책임이다.

물리 원본은 기존 raw 예약 한 번이며 completion/control metadata는 승인된 control 분할, routing refs/copies/feedback은 output 분할에 들어간다. controller 연결은 유일하고 metadata allocations 해제 후 lease를 반환한다. allocator 결과는 해당 시험의 요청 bytes이며 allocator 내부 metadata/rounding·RSS·OS caches·실제 worker 비용의 증거가 아니다. Miri도 실행한 입력/seed의 해석 검사이며 모든 interleaving이나 soundness의 완전한 증명이 아니다.

미실행: 실제 파일 IO/OS sync/rename·backend worker 기동·기록/복구·장비 durability/지연/처리량·Windows/macOS/PREEMPT_RT native·fresh audit. actual file adapter는 TASK-007 이후다. TC-003의 runtime 설정 변경/실제 worker/장비 경합, TC 전체·MS-002·Gate4/5 Pass로 승격하지 않는다. Human 리뷰는 2026-10-28 「승인, 커밋 푸쉬하고 세션 마무리해줘」로 승인됐다. native CI Not Run(사용량 소진·재확인 불필요 지시 유지). 동일 차단 재시도·계정/공개 변경은 하지 않았다. 승인 후 코드 커밋·초기 인증 차단과 해소·게시 완료는 아래 게시 기록에 보존한다. TASK-004/005 기존 구현 승인은 유지한다.

<a id="ev-006"></a>
### EV-006 — TASK-001~006 설계 대조·자가 리뷰

- 요청: 2026-10-28 Human 「이번 세션은 TASK-006까지 설계된대로 구현되었는지 체크 및 자가 리뷰」. 수정 구현·commit/push는 수행하지 않았다.
- 대상: `011d6f17ee9d34e604087de3cbfd82ca9c0b7e48`. EV-005의 36개 source/manifest SHA-256이 현재 작업 트리와 전부 일치한다. 기존 EV-001~005의 실행 이력과 Human 승인은 보존하며, 새로 발견한 반례를 현 구현의 미해결 결함으로 추가한다.
- 방법: TASK-001~006·ADR-008·도메인/데이터/인터페이스 계약을 구성·수집·풀·분배·완료·제어 코드와 대조했다. 기존 feature/Miri 시험은 소스가 같아 반복하지 않았다. 신규 반례 3개만 격리한 git archive 복사본에서 Rust 1.85.0/1.96.0으로 실행했다. 원본 제품 코드·기존 제품 테스트·의존성·CI는 변경하지 않았다.
- 근거: [실행 요약·hash](../06_evolution/imported_context/self_review_task_001_006_2026-10-28/results.json), [재현 실행기](../06_evolution/imported_context/self_review_task_001_006_2026-10-28/reproduce.py), [공개 API 반례](../06_evolution/imported_context/self_review_task_001_006_2026-10-28/review_regressions.rs), [분배 interleaving 반례](../06_evolution/imported_context/self_review_task_001_006_2026-10-28/review_routing.rs). 실행 결과는 **두 Rust 각각 3개 계약 assertion 실패**이며, 제품 합격 시험으로 집계하지 않는다.

| 범위 | 대조 결과 |
|---|---|
| TASK-001 | 승인된 설계·ADR·실행 Task/검증 범위 연결 확인. 설계 기준선은 유지한다. |
| TASK-002 | 예산 합·overflow·quota·required capability·CPU 검증과 구현 경계 일치. 기존 EV-001 유지. |
| TASK-003 | 소형 복사·FIFO·중요 예약·레벨 매핑·일반 deadline 경로 확인. tracked 통합에서는 RV-002로 예약 보호가 깨진다. |
| TASK-004 | 동일 loan 반환·borrowed owner·generation/state·무해제 반환/유한 reclaim·private unsafe 경계 확인. 추가 결함은 확인하지 못했으며 기존 Miri는 모든 interleaving의 증명이 아니다. |
| TASK-005 | 유한 스케줄/출력/retry·공유 원본과 복사 예산 확인. TASK-006에서 연결한 completion feedback에 RV-001이 존재한다. |
| TASK-006 | bounded query/history·명시 sync·guard 유지·전용 shutdown slot 구현 확인. 아래 P1 3건으로 설계 충족 판정 보류. |

| ID / 우선순위 | 위치·계약 | 재현·영향·수정 검토 방향 |
|---|---|---|
| RV-001 / P1 | `completion/mod.rs:422` terminal 전이 검사, `routing/mod.rs:252` feedback; 인터페이스 §3.3·4.2·13.2 | 대상 FIFO 게시 후 `finish`의 Enqueued 반영 전에 writer가 IoFailure를 보고하면, 뒤늦은 Enqueued 확인이 InvalidTransition이 된다. feedback head가 남아 다음 turn도 같은 fault로 신규 분배를 중단한다. private 재현은 실제 submit→writer report→finish helper를 순서대로 구동해 가능한 동시 실행을 결정적으로 선택한다. 생산 코드 함수는 바꾸지 않았다. 실패 사실을 보존하면서 앞선 enqueue 확인의 역전 도착을 처리해야 한다. |
| RV-002 / P1 | `completion/mod.rs:572`, `capture/mod.rs:519`; 인터페이스 §9.2(모든 병목 자원의 Important 예약 보호) | PrioritizeImportant·input slots=8·Important reserve=2·completion window=2 구성을 허용한다. Normal 2개 수락 뒤 입력 예약이 남아 있어도 Important가 CompletionCapacity로 거절된다. 완료 예약에 중요도 분할/Normal 제한도, 이를 보장하는 구성 검사도 없다. 같은 producer의 미완료 Normal이 중요 완료 공간을 소진하지 못하도록 구성·수락 알고리즘을 검토해야 한다. |
| RV-003 / P1 | `control/mod.rs:479` queued sync 만료, `:334` Closing 거절, `:705` 종료 조건; 인터페이스 §10·13.2 | completion=Synced인 필수 대상에 Written 후 shutdown을 시작하고 sync가 시작되기 전에 deadline에 도달한다. 명령은 EXPIRED지만 receipt는 Pending이다. ingress/output/IO/loan/feedback 모두 0이어도 반복 cleanup은 Incomplete이고 next_sync는 None, 새 request_sync는 Closing이다. 같은 owner에서 후속 정리할 상태 전이가 없다. 성공을 꾸미지 않고 미완료 sync를 실패/불명으로 종결하거나, 안전한 후속 sync/cleanup 경로를 제공해야 한다. |
| RV-004 / P2(문서) | `README.md:4` | 현재 구현 설명이 TASK-002 구성 검증 단계에 머물러 TASK-003~006과 다르다. 현재 범위·미구현 파일/worker를 구분해 후속 문서 정렬이 필요하다. 이번 리뷰에서는 원문을 수정하지 않았다. |

판정: **설계대로 구현 완료라고 볼 수 없음 — P1 3건 미수정**. TASK-006 및 MS-002는 In Progress 유지, TASK-007 신규 개발 전에 결함 수정/회귀 검증을 권고한다. 기존 Human 승인을 취소하거나 설계를 변경한 것은 아니다. RV-002의 공개 구성/수락 의미, RV-003의 종료 결과 전이는 승인된 계약을 지키는 수정안을 먼저 구체화한다. 새 범위/계약 변경이 필요하면 H/J 기준을 적용한다.

미검증: 실제 파일/OS sync/worker·Windows/macOS/PREEMPT_RT native·실장비 성능/장애/전원 차단은 종전과 같다. GitHub 계정/사용량 조회·CI 실행/재시도·fresh audit는 수행하지 않았다. T1/T2 미도래, Gate 4/5 Not Ready 유지.

후속: 위 EV-006은 수정 전 `011d6f1`의 리뷰 이력이다. Human 수정/회귀 착수 지시 후 RV-001~004를 수정했고 현 결과는 EV-007을 따른다.

<a id="ev-007"></a>
### EV-007 — TASK-006 자가 리뷰 결함 수정·회귀

- 승인 범위: 2026-10-28 Human 「지금 수정과 회기 검증 진행해도 되겠어?」에 따른 RV-001~004 수정·회귀 착수. 수정분 Human 구현 리뷰·commit/push 승인을 대신하지 않는다.
- 대상: `011d6f1` 위 dirty worktree. [37개 source/manifest SHA-256·환경·명령·결과](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/results.json)로 고정한다(기존 EV-005의36개+README). completion/capture/control·회귀 시험·consumer·README가 변경됐으며 private pool/storage unsafe·dependency/lock·CI는 유지한다. EV-005/006의 과거 hash/판정은 보존한다.
- TDD: 정식 회귀 시험에서 [feedback Red](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/red-feedback.log), [Important Red](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/red-priority.log), [shutdown Red](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/red-shutdown.log)를 먼저 확인했다. 환경/컴파일 실패가 아닌 계약 assertion 실패다. 수정 후 각 시험과 영향 회귀가 Pass했다.

| 결함 | 수정·회귀 결과 |
|---|---|
| RV-001 | 실패/폐기/불명을 유지하면서 늦게 도착한 Enqueued 사실만 병합한다. paused metadata→writer 실패→feedback 배출→다음 기록 분배까지 확인. 이후 Written/Synced의 terminal 덮어쓰기는 계속 거절한다. |
| RV-002 | producer별 R=max(small Important slots, raw Important slots), W≥R 검사. Normal은 다음 R개 cyclic slot의 재사용 가능성을 확인한 뒤 기존 단발 CAS로 수락한다. W=R은 Important 전용, TreatEqually의 R=0 유지. 원형 빈 hole/32회 wrap·소형/원본·동일 loan 반환·RT0/0을 확인했다. |
| RV-003 | 시작 전 만료된 전용 shutdown sync만 실제 자원/feedback·선행 일반 sync 정리 후 유한 cursor로 미확인 Synced를 Unknown 처리한다. Written·Expired·deadline 실패를 보존하고 같은 owner로 실패 Closed에 도달한다. 늦은 raw write/일반 sync 성공과 일반 요청 expiry 후 재요청을 검증했다. |
| RV-004 | README를 현재 코어/대역 writer와 실제 파일/worker 미구현 범위에 맞췄다. 오래된 audit 결과도 날짜를 명시했다. |

신규9개 = 완료 private1·완료 통합3·제어 통합4·allocator1. 현재 완료 통합15·제어18·lib9다. 전체 소비자 검증 중 기존 독립 consumer의 Important16/완료창8 불일치를 새 구성 검사에서 발견했다. [초기 실패](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/features-1.85.0-initial.log)를 보존하고 예제 window32/work_per_poll32로 정렬한 뒤 전체 검사했다.

| 명령·범위 | 결과·근거 |
|---|---|
| `python3 scripts/check-feature-matrix.py --toolchain <version>` | Rust1.85.0/1.96.0 각각16 feature·독립 consumer·scoped 예제·allocator 회귀 Pass / [1.85](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/features-1.85.0.log), [1.96](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/features-1.96.0.log) |
| `cargo +<version> clippy --workspace --all-targets --all-features --locked -- -D warnings` | 두 Rust Pass / [1.85](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/clippy-1.85.0.log), [1.96](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/clippy-1.96.0.log) |
| `RUSTDOCFLAGS='-D warnings' cargo +<version> doc --workspace --all-features --no-deps --locked` | 두 Rust Pass / [1.85](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/rustdoc-1.85.0.log), [1.96](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/rustdoc-1.96.0.log) |
| `cargo +<version> test -p dot-logger-rt-allocation --locked -- --nocapture` | 두 Rust6개 Pass. 기존/신규 Important small/raw admission·거절·마지막 RT handle Drop allocation/deallocation0/0. query/sync128회 requested peak11586 / concrete14978 / 최종 잔여0, routing512 records peak2656/output3168/잔여0 / [1.85](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/allocator-1.85.0.log), [1.96](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/allocator-1.96.0.log) |
| `MIRIFLAGS='-Zmiri-strict-provenance -Zmiri-many-seeds=0..8' cargo +nightly-2026-10-26 miri test -p dot-logger --lib --test tc_002_pool --test tc_003_routing --test tc_003_completion --test tc_003_control --locked` | lib9+raw17+routing19+completion15+control18 =78×8 seeds(624) Pass / [Miri](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/miri.log) |
| `cargo +1.96.0 fmt --all --check`, `git diff --check` | Pass / [fmt](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/fmt.log), source/문서 diff 검사 |

유효 범위: 로컬의 선언된 코어/대역 경로이며 RV-001~004 수정·회귀를 확인했다. Normal RT 경로에는 R개 상태 조회가 추가됐고 실제 p99는 미검증이다. 만료 shutdown의 Unknown은 실제 접근이 모두 끝난 뒤의 최종 정리 결과이며 성공 sync가 아니다. allocator 수치는 요청 bytes이며 allocator 내부/RSS/OS cache를 포함하지 않는다. Miri는 실행한 경로/seed의 해석 검사이며 모든 interleaving 증명이 아니다.

미실행/잔여: 수정분 Human 리뷰·native CI·fresh audit·실제 파일/OS sync/rename·worker·장비 RT/throughput/전원 차단. 사용량/과금 조회·차단 재시도·commit/push는 하지 않았다. GAP-006은 로컬 수정/회귀 범위에서 Closed, GAP-004 및 TASK-006/MS-002 In Progress·Gate4/5 Not Ready는 유지한다.

후속 게시·마감: Human 「커밋 푸쉬하고 세션 마무리」에 따라 코드 `d57c2e3`을 origin/main에 push하고 원격 ref를 확인했다. EV-007의37개 source/manifest hash는 commit blob과 일치한다. [코드 게시 기록](../06_evolution/imported_context/task_006_review_fixes_2026-10-28/publication.json)에 관측 ref와 범위를 남겼으며 `.cowork` 문서는 별도 마감 커밋으로 관리한다. `[skip ci]`를 사용했고 검증 소스가 같아 제품 시험은 반복하지 않았다. 커밋/push 승인은 수정분 Human 구현 리뷰나 native Pass를 대신하지 않는다. `.agents/skills/code-review/SKILL.md`는 별도 추가된 사용자 파일로 이번 작업에서 제외·보존했다.

<a id="ev-008"></a>
### EV-008 — TASK-006 후속 개선 검토·종료 경합 리뷰

- 요청: 2026-10-29 Human 「리뷰 개선 검토부터 하고 TASK-006 검토 진행하자」. 제품 수정 없이 기존 제안과 TASK-006을 검토했다.
- 대상: 제품 코드 `d57c2e3` 및 현재 작업 트리. EV-007의 source/manifest 37개 SHA-256이 모두 일치한다. 기존 전체 matrix/Miri/allocator는 반복하지 않았고 RV-001~004 수정·로컬 Pass 이력을 유지한다.
- [실행·환경·대상 hash](../06_evolution/imported_context/task_006_followup_review_2026-10-29/results.json), [격리 재현 스크립트](../06_evolution/imported_context/task_006_followup_review_2026-10-29/reproduce.py), [재현 테스트](../06_evolution/imported_context/task_006_followup_review_2026-10-29/review_tests.rs), [fixture](../06_evolution/imported_context/task_006_followup_review_2026-10-29/review_support.rs).

| 발견 / 우선순위 | 위치·계약 | 재현·영향·수정 검토 방향 |
|---|---|---|
| RV-005 / P1 | `control/mod.rs:337` 열림 검사와 `:359` 명령 게시, `:738` Closed 판정. 인터페이스 §10·13.2의 종료/IO 계약 | 다른 스레드의 `request_sync`가 열림 검사 후 게시 전에 정지 → `begin_shutdown(sync_files=false)`가 빈 세션에 Closed 성공 → 요청 재개로 토큰 생성 → `next_sync`가 새 SyncWork 발급 → `poll_shutdown`은 sync_in_flight=1인데도 Closed를 유지한다. ControlOwner는 공유 참조로 동시 호출 가능하며 아직 슬롯에 게시되지 않은 제어 요청을 shutdown이 추적하지 않는다. 일반 sync 요청의 진입·게시와 shutdown close/최종 정리를 함께 순서화해야 한다. 검사 한 번 추가만으로는 마지막 검사와 게시 사이의 경합을 제거하지 못한다. |

격리 복사본에 `cfg(test)` 일시정지 hook만 추가해 열림 검사 직후의 합법적인 스케줄을 고정했다. hook은 상태를 바꾸지 않으며 두 scoped thread가 실제 공개 API를 호출한다. 원본 제품 소스·기존 제품 테스트는 수정하지 않았다. hook을 이용한 결정적 실행 순서의 반례이며 무수정 바이너리의 자연 발생 빈도나 모든 interleaving을 측정한 결과는 아니다.

| 새 검증 | 결과 |
|---|---|
| Rust 1.85.0 / 1.96.0 `cargo +<version> test --locked -p dot-logger --lib review_ -- --nocapture --test-threads=1` | 각 exit101, **계약 assertion 실패 1개**. 순차적인 Closed 후 요청 거절 시험과 레이아웃 관측은 각각 Pass(2 Pass/1 Fail). 빌드 오류와 구분한다. [1.85](../06_evolution/imported_context/task_006_followup_review_2026-10-29/review-1.85.0.log), [1.96](../06_evolution/imported_context/task_006_followup_review_2026-10-29/review-1.96.0.log) |

이전 세션 #004 개선 제안의 검토 결과:

| 제안 | 현재 판단 |
|---|---|
| 채널 LUT | capture classify와 completion reserve 모두 채널 선형 탐색이 있다. 유한 탐색 자체는 무대기 위반이 아니고 p99 영향은 미측정이다. 채널 ID는 임의 u32이므로 전체 ID 직접 인덱스 LUT를 곧바로 도입하지 않는다. 내부 채널 인덱스 재사용/압축 매핑 등을 부하·메모리 측정 후 비교한다. Normal의 R개 runway 탐색도 비용에 포함한다. |
| RecordSlot 정렬 | 두 Rust의 현재 x86_64 환경에서 size40/align8, 별도 할당 TargetCell은 size16/align8을 관측했다. false sharing 가능성은 있지만 영향은 미측정이다. RecordSlot 정렬만으로 cell 배열 공유까지 해결하지 못하며, 정렬 시 슬롯당 증가분·전체 control 예산과 함께 TASK-022 장비에서 비교한다. |
| CaptureError 세분화 | 제안의 미등록 채널 사례는 현재 코드에 해당하지 않는다. classify가 InvalidInput으로 먼저 거절하고 tracked 생성이 고정 route/channel 대응을 검증한다. CompletionCapacity는 실제 예약 부족/일시 게시 경합의 기존 계약이다. 공개 enum 변경을 요구할 재현 근거가 없어 현 단계에서는 보류한다. |
| poll_shutdown 계약 | 반환은 bool이 아니라 ShutdownPoll이며 Closed도 실패 보고일 수 있다. 현재 rustdoc·인터페이스 §13.2에 dispatcher/readers/writers 구동과 timeout 후 계속 정리가 이미 설명되어 있다. 고정 2회 호출로 끝나는 계약은 아니다. 반복 구동 예제 추가는 P3 문서 보강 후보다. |

판정: **Reject — RV-005 P1 미수정으로 TASK-006 최종 구현 승인 권고를 보류**한다. RV-001~004 수정의 유효성이나 과거 Human 승인을 취소한 것은 아니다. TASK-006/MS-002는 In Progress, TASK-007은 미착수, Gate4/5 Not Ready다. RV-005 수정·회귀가 다음 시작점이다.

미실행: 새 제품 수정/전체 회귀·native CI·과금/사용량 조회·fresh audit·실제 파일/worker/장비 검증·commit/push. T1/T2는 미도래다. GAP-006은 과거 수정 범위에서 Closed 유지, 새 결함은 GAP-007로 분리한다.

<a id="ev-009"></a>
### EV-009 — TASK-006 RV-005 수정·회귀

- 승인: 2026-10-29 Human 「수정 진행하고, TASK-006 변경사항 보고할때 수정반영사항 같이 보고하자」. RV-005 수정·검증을 실행하고 기존 RV-001~004 반영사항을 함께 보고한다. 최종 구현 리뷰 승인·commit/push 지시를 대신하지 않는다.
- 대상: 제품 기준 `d57c2e3` 위 작업 트리. 제품 변경은 `control/mod.rs`와 새 `control/tests.rs`이며, private unsafe·RT 수집 경로·dependency/lock·CI recipe는 유지한다. [source38개 hash·환경·명령·결과](../06_evolution/imported_context/task_006_rv005_fix_2026-10-29/results.json)를 검증 시작/완료 시 대조했다. EV-007/008은 수정 전 대상/판정 이력으로 보존한다.
- TDD: 정식 private 반례2개가 수정 전 두 Rust에서 `shutdown closed before an admitted sync request was published` assertion으로 실패했다([1.85 Red](../06_evolution/imported_context/task_006_rv005_fix_2026-10-29/red-1.85.0.log), [1.96 Red](../06_evolution/imported_context/task_006_rv005_fix_2026-10-29/red-1.96.0.log)). 최소 수정 후 두 Rust Green, 이후 오류 반환·Busy/Closing을 보강해 최종 private 회귀3개다. 초기 Red 테스트의 배치/줄번호와 최종 보강본은 다르다.

수정: `SyncSubmission` RAII가 일반 sync의 진입부터 게시/오류 반환까지 단발 CAS·원자적 bool로 표시한다. 진입 표시 → 열림 확인 → shutdown close → 정지 확인을 SeqCst로 순서화한다. close 전 열림을 확인한 호출은 게시/반환이 끝나기 전 shutdown 범위 동결/최종 정리를 막는다. 동시 일반 요청은 기존 Busy, close 이후 요청은 Closing이다. 모든 반환 경로에서 표시를 해제하며 잠금 대기·동적 할당·재시도 루프·새 오류 enum은 추가하지 않았다. 실제 IO guard·요청 slot·deadline/만료 정리 계약은 유지한다. test-only scheduler는 스레드별로 등록하고 cfg(test) 밖에서는 제거된다.

| 검증 | 결과·근거 |
|---|---|
| 두 Rust 각16 feature·독립 consumer·scoped 예제 | Pass. capture→routing→Written→명시 sync 대역→shutdown 종단 소비자 유지 / [1.85](../06_evolution/imported_context/task_006_rv005_fix_2026-10-29/features-1.85.0.log), [1.96](../06_evolution/imported_context/task_006_rv005_fix_2026-10-29/features-1.96.0.log) |
| clippy workspace/all-targets/all-features `-D warnings`·rustdoc `-D warnings`·fmt | 두 Rust clippy/rustdoc 및1.96 fmt Pass / 명령·로그는 results.json |
| 신규 private3개 | 게시 중 Closing 유지→실제 sync IO 종료 후 Closed 성공, 게시 중 deadline의 Incomplete 유지→만료 게시 후 실패 Closed, 검증 실패 호출의 표시 해제→IO 없이 Closed 성공. 동시 Busy·close 중/후 Closing도 확인 |
| 기존 완료15·제어18·원본17·분배19 및 다른 계약 | 두 Rust features 회귀 Pass. RV-001~004 회귀·guard/token/epoch·same-owner timeout·ordinary expiry·exact-fit/1-byte 부족·실제 source partition 대조 포함 |
| Miri strict-provenance·8 seeds | lib12+raw17+routing19+completion15+control18=81×8(648) Pass / [로그](../06_evolution/imported_context/task_006_rv005_fix_2026-10-29/miri.log) |
| RT/일반 allocator | 두 Rust6개 Pass. tracked/Important small/raw·거절/Drop allocation/deallocation0/0. query/sync128회 requested peak11586 / concrete14978 / 최종 잔여0. routing512개 peak2656/output3168/최종 잔여0 / [1.85](../06_evolution/imported_context/task_006_rv005_fix_2026-10-29/allocator-1.85.0.log), [1.96](../06_evolution/imported_context/task_006_rv005_fix_2026-10-29/allocator-1.96.0.log) |

F-11 소비자 목록과 검증 범위는 [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-29_001.md)에 기록했다. `request_sync/next_sync/poll/snapshot/release`, 일반/전용 shutdown 요청, 기존 timeout/IO/loan/feedback, 독립 consumer·예제·메모리 계측/예산 경계를 실제 회귀로 확인했다. 사용자 요청에 따라 TASK-006 상세의 RV-001~005 수정 반영표를 함께 갱신했다.

자가 리뷰: **Conditional — RV-005 로컬 수정·회귀 Pass**, GAP-007은 이 범위에서 Closed다. 최종 수정분 Human 구현 리뷰/native·실제 파일/worker/장비 검증은 남아 TASK-006/MS-002 In Progress·Gate4/5 Not Ready를 유지한다. Miri는 실행한 경로/seed의 검사이며 모든 interleaving이나 실제 RT 성능 증명이 아니다. 유한 채널 탐색/정렬은 측정 후 검토하고 근거 없는 공개 오류 enum 확대는 보류한 EV-008 판단을 유지한다.

미실행: hosted native CI·fresh audit·실제 파일/OS sync/worker·장비 성능/전원 차단·commit/push. CI/계정/과금 조회나 차단 재시도는 하지 않았다. 원본 명령 출력은 `.log.gz`에 보존하고 `.log`는 줄 끝/EOF 공백만 정규화한 읽기용 뷰다. 각각의 hash는 results.json에 있다.

최종 AI 검토(동일 날짜): Human 「다음단계로」에 따라 RV-001~005와 종료/오류/guard 수명 계약을 재검토했다. source38개·artifact·원본 압축 로그 hash 및 기록된10개 명령의 exit0을 대조했고 같은 시험은 반복하지 않았다. 추가 P1/P2는 확인하지 못했다. Conditional/로컬 구현 승인 권고를 유지하며 최종 Human 승인과 native·실장비 검증을 대신하지 않는다. 제품 변경 없이 검토 결과와 TASK-007 첫 실패 시험 준비만 TASK-006/공유 상태/기존 작업 로그에 기록했다.

<a id="ev-010"></a>
### EV-010 — TASK-007 MCAP 봉투·writer·인덱스

- 승인: Human 「승인」으로 TASK-006 RV-001~005 최종 구현 리뷰 및 native 이월 상태의 TASK-007 개발 진행을 승인했다. TASK-006은 native 잔여로 In Progress이며 TASK-007 구현 승인은 별도다. commit/push 지시는 아니다.
- 대상: 작업 트리의 [source46개/환경·명령·결과](../06_evolution/imported_context/task_007_2026-10-29/results.json). 기존 EV-009 수정분을 보존하고 storage6개·저장 시험/fixture2개를 추가했다. routing/control/lib·독립 consumer·allocator를 변경했으며 dependency/lock·private unsafe·CI recipe는 유지했다. README도 실제 파일 구현 범위로 정렬한 최종 source46개를 검증했다.
- TDD: [새 API Red](../06_evolution/imported_context/task_007_2026-10-29/red-api.log) 후 stub이 컴파일되는 [동작 Red2개](../06_evolution/imported_context/task_007_2026-10-29/red-behavior.log)를 확인했다. [소유권/coverage Red2개](../06_evolution/imported_context/task_007_2026-10-29/red-ownership.log)는 같은 대상 writer 중복과 새 파일의 과거 미확인 sync 대체를 검출했다. 수정 후 모두 Pass, 최종 저장 통합13개다. 최종 오류 검토의 [Red2개](../06_evolution/imported_context/task_007_2026-10-29/red-final-review.log) 후 CompletionError::Expired 보존과 u32 초과 metadata 구성 거절을 보강했다.

| 검증 | 결과·범위 |
|---|---|
| u64/u128 봉투·시각·schema | full receipt, u32 초과/MAX sequence, mono MAX·wall 역행·signed source/domain, 두 schema version·u32 MAX channel, unknown opaque payload, malformed length/major·음수 real·u32 metadata 상한 거절, 성공한 파일 쓰기 뒤 CompletionError::Expired 분리 Pass |
| 독립 판독 | MCAP Rust reader + 별도 바이트 scanner의 record bounds·chunk CRC/min/max·message index offset 대조, protoc 생성 FileDescriptorSet 및 envelope_v1 golden, CRC32 알려진 벡터·손상/잘린 데이터 확인 Pass |
| 실제 파일 write/flush/sync·종료 | 8MiB 단일 record/작은 chunk target, safe offset 미공개/완전 chunk만 공개, closed footer 포함 실제 File::sync_all→completion/shutdown, sync 실패 Written 보존, 이전 세그먼트 미확인 sync 차단 Pass |
| 자원·오류 | 대상별 writer 하나, 공유 output 총량·finished index lease 유지/해제, exact-fit/1-byte 부족·overflow, record/chunk 상한·상한 거절의 file 불변, flush 실패 Poisoned/이전 safe 유지, Drop 비성공/추가 IO 없음 Pass |
| 두 Rust 각16 features·독립 consumer·예제 | Pass. consumer는 실제 파일 생성→write→finish→sync→shutdown을 구동 / [1.85](../06_evolution/imported_context/task_007_2026-10-29/features-1.85.0.log), [1.96](../06_evolution/imported_context/task_007_2026-10-29/features-1.96.0.log) |
| clippy/rustdoc/fmt | 두 Rust workspace/all-targets/all-features clippy 및 rustdoc `-D warnings`,1.96 fmt Pass / 명령·로그는 results.json |
| Miri | 기존 lib12+raw17+routing19+completion15+control18=81×8=648, 새 봉투2×8=16, 합664 Pass. 파일 통합11개는 일반 로컬 실행 / [core](../06_evolution/imported_context/task_007_2026-10-29/miri-core.log), [codec](../06_evolution/imported_context/task_007_2026-10-29/miri-codec.log) |
| allocator7 | 두 Rust Pass. 저장 max256B/8MiB·각129 records/chunks의 peak104824/8493176 bytes ≤ 예약232064/17008768, 최종 잔여0. 기존 RT0/0, query/sync128회 peak11602/concrete14994/잔여0, routing512개 peak2672/output3184/잔여0 / [1.85](../06_evolution/imported_context/task_007_2026-10-29/allocator-1.85.0.log), [1.96](../06_evolution/imported_context/task_007_2026-10-29/allocator-1.96.0.log) |

메모리 예약은 MCAP0.26의 descriptor/maps/index와 Vec 성장·summary 복사를 수량 상한으로 보수적으로 잡고 실제 requested-byte peak와 대조했다. chunk payload는 direct seek로 쓰며 encoder scratch 복사는 예산에 포함한다. 원본 입력/사용자 sink 버퍼·allocator 내부·OS page cache/RSS는 위 측정 밖이다. 실제 sync 호출 성공을 전원 차단 후 내구성이나 지연 목표 달성으로 확대하지 않는다.

자가 리뷰 Conditional — 로컬 구현/회귀 Pass, Human 구현 리뷰/native·실장비 잔여다. TASK-007/MS-003 In Progress·TC-004 부분 실행·Gate4/5 Not Ready를 유지한다. 자동 회전/보존·directory metadata sync·복구/active reader·제품 압축·worker 기동·OS 교환/golden 이전 major 호환은 후속 Task다. MCAP 형식 근거는 [공식 명세](https://mcap.dev/spec), upstream 메모리 동작은 고정된0.26 registry 소스를 확인했다. `paste` 유지보수 경고(GAP-005)는 그대로이며 fresh audit/계정·과금 조회/CI 재시도·commit/push는 하지 않았다. source/원본 압축 로그 hash는 results.json으로 고정한다.

Human 승인 후속(2026-10-29): TASK-006 RV-001~005 최신 수정분과 native 이월 상태 TASK-007 진행은 승인됐다. TASK-007 EV-010 구현 리뷰는 대기다.

세션 마감(2026-10-29): Human 「세션 마무리해줘」에 따라 EV-010 source46개/검사11개 및 artifact·원본 로그 hash 일치를 재확인하고 시험은 반복하지 않았다. F-11 소비자별 종단 검증 결과는 위 근거와 최신 로그에 유지한다. TASK-006 Human 승인/native 잔여, TASK-007 Human 구현 리뷰/native 대기, TASK-008 미착수다. 상태/개인 인덱스/로그를 동기화했으며 세션 종료를 TASK-007 승인이나 commit/push 지시로 해석하지 않았다.

<a id="ev-011"></a>
### EV-011 — TASK-007 리뷰 RV-006~010 수정·회귀

- 승인: Human 「코드리뷰 받은 내역을 확인하고 수정하자는 말이였어」로 이전 리뷰의 수정 범위·방향을 승인했다. 최신 구현의 최종 Human 리뷰와 native 검증 승격은 별도다.
- 대상: [source46개·환경/명령/결과·로그 hash](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/results.json). 시작 전 EV-010 source46/46 일치를 확인했고 그중8개를 수정했다(README·storage mod/writer·control·completion private tests·저장 통합·consumer·allocator). dependency/lock·private unsafe·RT 구현·CI recipe는 유지했다. 최종 matrix/Miri 검증의 source46 hash가 현재 소스와 일치한다.
- TDD: [API Red](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/red-api.log)는 인터페이스 미존재를 확인했다(동작 증명 아님). 컴파일되는 기존 동작 호환 stub에서 [동작 Red](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/red-behavior.log)를 확인했다. 최초 harness의 dispatcher 턴 수/두 writer 예산 문제는 정정했고, [배치 chunk Red](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/red-batch.log)·[SyncWork 반환 Red](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/red-sync.log)로 실제 결함을 분리했다. [기존 Busy 매핑 Red](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/red-busy-mapping.log) 뒤 공통 매핑/재시도를 회귀했다. 초기 실패 로그도 삭제하지 않고 보존한다.

| 검증 | 결과·범위 |
|---|---|
| RV-006 배치/예약 | 4개 delivery가 max_chunks=1에서 단일 완전 chunk·Written을 만든다. 부분 append 거절은 prefix를 보존해 명시 flush/report하고, flush 실패에서는 Written을 보고하지 않는다. 비어 있지 않은 channel/chunk 쌍 상한을 min(records,chunks×channels)로 제한한다.128채널·10만records/chunks 설정의 예약109987328bytes(설정 산술 시험, 10만건 장비 부하 결과 아님) |
| RV-007/008 sync 소유권·Busy | active/finished 모두 잘못된 target→원 work 반환→올바른 writer 재시도→Synced Pass. IO 실패는 work None·Failed이며 Written 보존. private metadata UPDATING에서 공통 Busy·work 보존 후 재시도 Pass. query permit2개 포화에도 직접 coverage poll은 독립 동작하며, 이전 리뷰의 permit 포화 원인 설명을 정정함 |
| RV-009 버퍼/오류/파일 | 명시 write_buffer_bytes(0=direct)를 공유 예산에 포함.128×64B 레코드의 File write 호출962→9(4KiB), short write3bytes는8268회, 세 파일 바이트 동일·독립 CRC/index/scanner 일치. seek/flush 전 배출·safe offset·Interrupted/부분 IO 실패·Poisoned·buffered Drop 무IO Pass. syscall/장비 처리량·지연 목표를 검증한 것은 아님 |
| RV-010 보고 재시도 | 단일/배치 Written 보고 Busy를 private 스케줄로 재현. 배치는 appended/written/reported=2/2/1로 보존하며 report-only 재시도 뒤 파일 레코드2개·중복 없음 Pass. rustdoc/README/기준 계약 동기화 |
| 저장/기존 회귀 | 저장 통합21개(기존13+새8)·private Busy2개, 실제 file/footer sync와 종료·8MiB·golden·독립 판독·Drop·예산 경계 Pass. 저장 파일 통합19개는 일반 로컬 실행 |
| 두 Rust 각16 features·consumer·예제 | Pass / [1.85](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/features-1.85.0.log), [1.96](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/features-1.96.0.log). 독립 consumer는2 records→배치1chunk→finish→실제 sync→shutdown을 구동함 |
| clippy/rustdoc/fmt | 두 Rust workspace/all-targets/all-features clippy 및 rustdoc -D warnings,1.96 fmt Pass. 명령/로그는 results.json |
| Miri | lib14+raw17+routing19+completion15+control18=83×8=664 Pass. 새2개 private storage Busy 시험은 메모리 sink로 배치/버퍼/active·finished 경로를 실행 / [로그](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/miri-core.log). 변경 없는 봉투2개는 EV-010 Miri 근거를 재사용함 |
| allocator7 | 두 Rust Pass. 버퍼 포함256B/8MiB 각129records/chunks peak108926/8497278 ≤ 예약236160/17012864bytes.128채널·4096records sparse/dense peak2454992/611808 ≤ 예약5838400bytes. 최종 잔여0, 기존 RT0/0·routing/query/sync 메모리 회귀 Pass / [1.85](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/allocator-1.85.0.log), [1.96](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/allocator-1.96.0.log) |

RV-006~010은 로컬 수정 범위에서 해결했다. AI 최종 검토 **Conditional** — 추가 P1/P2 미확인, 최신 구현 Human 리뷰/native(GAP-004)·실장비 검증은 잔여다. 호출자가 batch 크기/보고 지연·buffer/chunk 용량을 명시하며 제품 기본값·처리량/내구성 수락은 TASK-022 후속이다. TASK-007/MS-003 In Progress·TC-004 부분 실행·Gate4/5 Not Ready를 유지한다. 원본 로그는 `.log.gz`, `.log`는 줄 끝/EOF 공백을 정규화한 뷰이고 각각 hash를 보존한다. fresh audit·CI/과금 조회·commit/push는 하지 않았다.

게시·세션 마감(2026-10-29): Human 「커밋 푸쉬하고 세션 마감해줘」로 소스 `71b3e2cbc31ac289d7e61add6f9864ed75f65407`를 origin/main에 게시했다. source46개/검증10개 exit0·원본/압축/뷰 로그20개·artifact3개 및 해당 commit blob46개의 SHA-256 일치를 재확인했으며 소스가 같아 시험은 반복하지 않았다. [게시 기록](../06_evolution/imported_context/task_007_review_fixes_2026-10-29/publication.json)을 보존하고 `.cowork` 마감 문서는 소스와 별도 커밋한다. [skip ci]로 자동 CI 실행을 피했으며 native/장비·Human 최신 구현 리뷰는 잔여다.

<a id="ev-012"></a>
### EV-012 — TASK-008 회전·순환·이벤트 보호

- 승인: 2026-10-30 Human 「승인, TASK-008 진행하자.」로 TASK-007 EV-011 최신 구현/REV-008을 승인하고 native GAP-004 이월 상태의 TASK-008 착수를 승인했다. TASK-007은 native 대기로 In Progress, REV-008 Closed다.
- 대상: `71b3e2c` 게시 소스를 기준으로 한 현재 작업 트리. [source49개·환경/명령/결과·원본 로그 hash](../06_evolution/imported_context/task_008_2026-10-30/results.json), [재현 script](../06_evolution/imported_context/task_008_2026-10-30/verify.py). 기존46개 중 README/storage mod·writer/allocator를 수정하고 catalog·retention·보존 통합3개를 추가했다. dependency/lock·RT 입력·원본 unsafe·CI recipe는 유지했다. 소스는 미게시다.
- TDD: [API Red](../06_evolution/imported_context/task_008_2026-10-30/red-api.log)는 API 부재 컴파일 실패(동작 증명 아님), [행동 Red](../06_evolution/imported_context/task_008_2026-10-30/red-behavior.log)는 컴파일되는 append 미구현에서 첫 회전/보호 행동 실패다. [unlink 뒤 directory sync 실패 Red](../06_evolution/imported_context/task_008_2026-10-30/red-delete.log)에서 stale catalog를 확인하고 RV-011을 수정했다. 테스트 fixture 중요도·오류 enum payload·shutdown API/수락 경계 기대값과 allocator 관측 밖 FileStore 해제의 음수 delta는 harness 정정이다.

| 범위·검증 | 결과·근거 |
|---|---|
| bytes/time/count/index | 각 한도에서 trigger record 유실 없이 회전. 별도 index bytes 상한과 chunk 상한, 독립 MCAP 판독·CRC/summary·u64 receipt 보존. file/index bytes는 pinned MCAP의 보수적 상한이며 최적 파일 크기/지연 보장 아님 |
| 회전·실패 | chunk/footer→file sync→rename→directory sync→catalog. sync/rename/directory 실패는 미공개·Poisoned. unlink 실패는 기존 catalog 보존, unlink 성공 뒤 directory sync 실패는 live catalog에서 제외·Poisoned |
| 세 보존 모드 | 연속은 disk bytes/age/count에서 삭제 없이 정지. 순환은 oldest unprotected closed만 삭제. active/event/read pin으로 확보 불가 시 Capacity. 매 admission 전 fresh free-space port 관측·disk reserve 검사 |
| 이벤트·핀 | pre/post 요청과 실제 retained mono extents·과거 부족/post 대기를 구분. overlap unique bytes·active full-file reserve·향후 post append quota 거절·명시 해제/만료. 핀 count/bytes/deadline·만료 후 실제 release 전 삭제 금지 |
| 저장 보존 통합15개 | [로컬 로그](../06_evolution/imported_context/task_008_2026-10-30/green-e2e.log). Unix 실제 파일1개·메모리 port14개. 전용 namespace 재사용 거절·역행 record mono·독립 reader·Drop 무IO·producer sequence 중복·잘못된 입력/기한·부분 배치 prefix·원 retention cause 보존 |
| control 종단 | 3개 delivery→세그먼트3개→Written→잘못된 target work 반환→원래 log 재시도→active/directory sync→Synced→shutdown. directory sync 실패는 Synced Failed·Written 보존. shutdown의 Written 계약과 별도 Synced 실패 관측을 구분 |
| 두 Rust 각16 features | Pass / [1.85](../06_evolution/imported_context/task_008_2026-10-30/features-1.85.0.log), [1.96](../06_evolution/imported_context/task_008_2026-10-30/features-1.96.0.log). 기존 단일 파일 independent core consumer의 실제 finish/sync/shutdown·예제와 신규 external API integration을 포함 |
| clippy/rustdoc/fmt | 두 Rust workspace/all-targets/all-features clippy·rustdoc -D warnings,1.96 fmt Pass. 상세 명령은 results.json |
| Miri | lib14+raw17+completion15+control18+routing19+retention14=97×8=776 Pass(strict provenance). 6개 program 각각 seed0..7 및 48개 성공 summary를 대조 / [로그](../06_evolution/imported_context/task_008_2026-10-30/miri-core.log). 실제 파일1개는 일반 실행이고 Miri로 확대하지 않음 |
| allocator8 | 두 Rust Pass / [1.85](../06_evolution/imported_context/task_008_2026-10-30/allocator-1.85.0.log), [1.96](../06_evolution/imported_context/task_008_2026-10-30/allocator-1.96.0.log). 신규256B/8MiB peak14451/16791504 ≤ 예약105696/25270752, 최종 잔여0·기존 RT allocation/deallocation0/0. host port/OS cache는 별도 |

자가 리뷰 [REV-009](reviews/REV-009_TASK-008.md)는 RV-011 로컬 수정·회귀 Pass로 **Resolved / Conditional**이다. 추가 미해결 P1/P2는 현재 검토에서 확인하지 못했다. Human 최종 구현 리뷰/native GAP-004·실제 장비 성능/내구성·OS 교환·호스트 free-space probe 실측은 잔여다. 표준 FileStore는 Unix directory sync만 구현하고 미지원 OS는 Unsupported로 명시 거절한다. 다른 OS adapter·재기동/복구 TASK-009·active reader TASK-010·worker/dot-probe는 후속이다. TASK-008/MS-003 In Progress·TC-005 부분 실행·Gate4/5 Not Ready를 유지한다.

source49개와 matrix/Miri source hash 일치·검사10개 exit0을 확인했다. `.log.gz`는 실행 원본, `.log`는 줄 끝/EOF 공백만 정규화한 뷰이며 각각 hash를 고정했다. fresh audit·CI/계정/과금 조회·차단 재시도·commit/push는 하지 않았다.

로컬 커밋·세션 마감(2026-10-30): Human 「커밋하고 세션 마무리하자」에 따라 소스 `73881452f9442e4cc38167d1a83e7fcf59a68110`를 로컬 커밋했다. source49개/commit blob49개·검사10개 exit0·원본/압축/뷰 로그16세트와 artifact hash 일치를 재확인했다. [커밋 기록](../06_evolution/imported_context/task_008_2026-10-30/commit.json)을 보존하며 소스와 마감 문서는 분리 커밋한다. 소스가 같아 시험을 반복하지 않았다. 푸시는 요청되지 않아 수행하지 않았고 TASK-008 Human 최종 구현 리뷰/native·장비는 잔여다.

### EV-013 — TASK-008 REV-010 지적 처리

- 승인/대상: 2026-10-30 Human 「프로젝트 브리핑해주고, 리뷰 지적사항 처리하고, TASK-008 요약 보고해줘.」. 로컬 TASK-008 소스 `7388145` 위 REV-010 조치 작업 트리. [source49개·환경·명령·결과/로그 hash](../06_evolution/imported_context/task_008_review_fixes_2026-10-30/results.json), [재현 script](../06_evolution/imported_context/task_008_review_fixes_2026-10-30/verify.py). 변경은 catalog/retention/보존 통합3파일이며 dependency/lock·RT 입력·unsafe 경계·feature 구성은 동일하다. commit/push·최종 구현 승인은 이 요청에 포함하지 않는다.
- RV-012: 유실 pin은 catalog 생존 중 삭제 보호/slot을 계속 보유한다. 만료/Drop은 실제 IO 종료를 증명하지 않으므로 승인된 삭제 금지 계약을 유지한다. `ReadPin` `must_use`와 API/기준 문서에 유실·오류/취소 시 책임을 명시하고 TASK-010 리더 수명 관리·안전한 고립 회수 조건 검토에 연결했다. 위험 자체의 완전 해소나 강제 회수 구현으로 보지 않는다. [pin-baseline](../06_evolution/imported_context/task_008_review_fixes_2026-10-30/pin-baseline.log)은 수정 전부터 Green인 계약 검증이다.
- RV-013 TDD: 회전 append의 host probe 3회/기대2회 [Red](../06_evolution/imported_context/task_008_review_fixes_2026-10-30/red-probe.log) 후 회전 없는 append만 별도 `room`을 호출했다. 회전의 사전 검사2회는 유지하고 중복1회를 제거했다. 다음 일반 append의 reserve 거절·동일 record 재시도·독립 MCAP 판독까지 [Green](../06_evolution/imported_context/task_008_review_fixes_2026-10-30/green-retention.log)이다. callback 호출 수 검증이며 실제 syscall 비용/장비 처리량 측정은 아니다.
- RV-014: session 경로의 실제 디렉터리 검사/symlink 거절 의도와 trusted namespace 배타적 관리 전제를 rustdoc에 명시했다. 동시 경로 교체 방지를 제공한다고 주장하지 않는다. 문서 전용 형식적 테스트는 추가하지 않았다.

| 범위 | 최종 결과 |
|---|---|
| 보존 통합 | 17개 Pass(기존15+신규2). 실제 Unix 파일1개 포함. 회전/이벤트/핀·reserve·IO 실패·multi-segment control/종료 회귀 |
| Rust 1.85.0·1.96.0 | 각 core/all-features workspace 테스트·독립 consumer·scoped_capture 예제 Pass. optional feature/의존 변경이 없어 이번 수정은 이 두 구성으로 검사했으며 각16조합을 재실행했다고 표시하지 않음 |
| allocator | 두 Rust workspace의 allocator8개 Pass, 기존 RT allocation/deallocation 0/0 가드 및 보존 예약/최종 반환 회귀 유지 |
| clippy/rustdoc/fmt | 두 Rust workspace/all-targets/all-features clippy `-D warnings`·rustdoc `-D warnings`, 1.96 fmt Pass |
| Miri | nightly-2026-10-26 strict provenance, retention16개×seed0..7=128회 Pass. seed8개·성공16개 summary8개를 대조함. 실제 파일1개는 일반 Linux 실행이며 Miri에서 제외 |
| 근거 일치 | 최종 검사14개 exit0, regression/Miri/source49개 hash가 같고 작업 중 소스 변경 없음. 초기 fmt 줄바꿈 실패와 포맷 전 성공 로그는 `pre-format/`에 보존하고 최종 포맷 소스로 확정. `.log.gz`는 원본, `.log`는 후행 공백/EOF만 정규화한 뷰 |

[REV-010](reviews/REV-010_TASK-008_CODE_REVIEW.md)은 RV-012 현 계약 유지/위험 명시·TASK-010 연결, RV-013/014 수정·검증으로 **Resolved / Conditional**이다. [REV-009](reviews/REV-009_TASK-008.md)도 Resolved를 유지하며 Human 최종 구현 리뷰 전 Closed로 올리지 않는다. native GAP-004·실장비/전원/OS 교환·host probe 실측·fresh audit는 미실행이다. 유실 pin의 실제 리더 반환/회수 조건은 TASK-010 미착수 후속이다. TASK-008/MS-003 In Progress·TC-005 부분 실행·Gate4/5 Not Ready를 유지한다. 기존 EV-012/Miri776은 수정 전 검증 이력이며 이번 Miri 범위는 보존128회다.

### EV-014 — TASK-009 중단 복구·IO 장애

- 승인/대상: 2026-10-30 Human 「커밋 푸쉬 후, 다음 테스크 진행」으로 TASK-008 조치 소스14fd1a4/문서1f2f44d를 origin/main에 게시한 뒤 TASK-009를 구현했다. TASK-008 최종 Human 리뷰·REV-009/010 Resolved·native GAP-004는 유지한다. TASK-009 대상은 `1f2f44d` 위 작업 트리다. [source51개·환경/명령/결과·원본 로그 hash](../06_evolution/imported_context/task_009_2026-10-30/results.json), [재현 script](../06_evolution/imported_context/task_009_2026-10-30/verify.py). 소스 변경은 recovery/fault_storage 신규2개와 storage mod/writer·allocator/CI recipe/README다. dependency/lock·RT 입력·unsafe 경계는 동일하다.
- TDD: 컴파일되는 recovery API stub에서 8개 행동 실패·1개 기존 입력 거절 Pass를 확인한 [Red](../06_evolution/imported_context/task_009_2026-10-30/red-recovery.log) 후 prefix/CRC/IO/예산을 구현했다. 자가 리뷰 RV-015는 final sync가 cancel 표시를 게시한 뒤 성공이 반환되는 [반례 Red](../06_evolution/imported_context/task_009_2026-10-30/red-cancel-sync.log) 후 sync 뒤 재검사로 해결했다. 초기 harness unused import/let-and-return lint는 정정·로그 보존했고 별도 제품 결함으로 세지 않는다.

| 범위·검증 | 최종 결과·유효 범위 |
|---|---|
| 원본 보존·완전 prefix | 불변 read-only source·독립 empty sink. chunk 전체 길이/opcode·nonzero CRC·schema/channel 선행 참조·metadata/봉투/sequence·time bounds 검증 뒤에만 재작성. 손상 chunk의 앞선 메시지도 제외하고 뒤 바이트 재동기화 없음 |
| 보고/형식 | source length·검증/복사 chunk 끝·제외 suffix와 stop 원인·수량. 손실 메시지와 교체된 index/summary bytes를 혼동하지 않음. opaque payload/full-width receipts/선택 사용자 schema·새 표준 summary/보조 인덱스 독립 판독. 새 파일 sync를 과거 프로세스의 completion 응답으로 복원하지 않음 |
| IO·공개 순서 | capability→create→복구/file sync→close→rename→directory sync. write/flush/sync/rename/directory/unsupported 주입, raw OS 원인 및 선행 prefix 유지. 실패 시 catalog 성공/암묵 retry/delete 없음. Unix 실제 파일 원본 바이트 불변·독립 output·기존 active 경로 거절 |
| 고갈·취소·격리 | source/record/chunk·공유 메모리·출력 record/chunk 한계. 취소 초기/최종 sync 도중 도착·비어 있지 않은 output·source IO 실패·앞선 검증 prefix 보고. ENOSPC(로컬 raw OS28 주입)는 해당 대상 IoFailure로 보고하면서 다른 대상 Written/후속 capture 수락 유지. 실제 매체 고갈/다른 OS errno 실측과 구분 |
| 복구 통합19개 | [최종 로그](../06_evolution/imported_context/task_009_2026-10-30/green-recovery-final.log) Pass, 일반 Linux 실제 FileStore1개·대역18개. 실제 파일1개는 Miri에서 제외 |
| 두 Rust 각16 features | 1.85.0·1.96.0 각16조합·독립 core consumer·scoped_capture 예제 Pass. [1.85](../06_evolution/imported_context/task_009_2026-10-30/features-1.85.0.log), [1.96](../06_evolution/imported_context/task_009_2026-10-30/features-1.96.0.log) |
| lint/doc/fmt | 두 Rust workspace/all-targets/all-features clippy `-D warnings`·rustdoc `-D warnings`, 1.96 fmt Pass |
| allocator9 | 두 Rust Pass. 신규 복구 256B/8MiB payload peak21386/33572214 ≤ writer+복구 예약213776/50543888 bytes, 최종 잔여0. 기존 RT allocation/deallocation0/0 가드 유지. source/sink adapter·allocator 내부·OS cache는 별도 |
| Miri | nightly-2026-10-26 strict provenance, lib14+raw17+completion15+control18+routing19+retention16+fault_storage18=117×8=936 Pass. program7×seed0..7·성공 summary56개/936회를 대조. [원본 뷰](../06_evolution/imported_context/task_009_2026-10-30/miri-core.log). 실제 파일/8MiB allocator 시험은 Miri 범위가 아님 |
| 근거 일치 | 검사10개 exit0·matrix/Miri/최종 source51개 hash 일치. `.log.gz`는 원본, `.log`는 후행/EOF 공백만 정규화한 뷰. 최종 소스가 같아 성공 후 시험을 반복하지 않음 |

[REV-011](reviews/REV-011_TASK-009.md)은 RV-015 로컬 해결·회귀 Pass로 **Resolved / Conditional**이다. TASK-009 검증 당시 작업 트리는 후속 세션 마감/게시 요청으로 소스 e5f3147을 커밋했고 source51개와 일치한다. Human 최종 구현 리뷰·native GAP-004·실제 장비/전원/OS 교환·host free-space adapter 실측은 잔여다. host가 원래 session/revision·동일 순서의 등록과 source 불변성을 보장해야 한다. 자동 startup directory 발견/기존 namespace 재개·active reader TASK-010·worker/StorageDegraded 계측 TASK-014는 별도 후속이다. TASK-009/MS-003 In Progress·TC-005 부분 실행·Gate4/5 Not Ready를 유지한다. 새 native CI recipe에는 retention/fault_storage Miri 범위를 추가했으나 CI/계정/과금 조회·차단 재시도·fresh audit는 하지 않았다.

### EV-015 — TASK-009 REV-012 P3 권고 조치

- 승인/대상: 2026-10-30 Human 「프로젝트 진행현황 확인하고, 새 리뷰 지적사항 처리하고, TASK-009 짧게 요약보고해줘」. `cdf9a63` 위 dirty 작업 트리, EV-014 source51개 중 `storage/recovery.rs`만 변경했다. [소스51개·환경/명령/결과·로그 SHA-256](../06_evolution/imported_context/task_009_review_fixes_2026-10-30/results.json)·[재현 script](../06_evolution/imported_context/task_009_review_fixes_2026-10-30/verify.py).
- 변경: RV-016 메시지 replay `get(22..)` 경계 확인, RV-017 `matches_text`의 고정64바이트 스택 포맷·encoding 원본 바이트 비교. 공개 계약/형식/예약 산식·의존/lock·RT/unsafe 경계는 동일하다. 기능 결함 수정이 아닌 리팩토링으로 기존 통합 시험을 사용했고 형식적인 Red/새 테스트는 만들지 않았다.

| 범위·명령 | 결과·유효 범위 |
|---|---|
| Rust 1.85.0·1.96.0 `cargo test --workspace --all-features --locked -- --nocapture` | 각각201 Pass(복구 통합19·allocator9 포함, doctest17 포함). 불변 원본/완전 prefix·손상/CRC/참조 거절·취소/IO/실제 Unix FileStore·선택 schema/receipt·독립 판독 회귀 |
| 두 Rust `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass |
| 1.96.0 `cargo fmt --all --check` | Pass |
| allocator | 두 Rust에서 복구256B/8MiB peak21386/33572214 ≤ 예약213776/50543888 bytes, 최종 잔여0. 기존 RT 할당/해제0/0 가드 Pass |
| 근거 일치 | 검사5개 exit0, 시작/완료 source51개 동일. EV-014와 차이는 recovery.rs 한 파일 |

[REV-012](reviews/REV-012_TASK-009_DEEP_REVIEW.md) RV-016/017 수용·Resolved. Human 최종 구현 리뷰·native GAP-004·장비/전원/OS 교환은 잔여이며 TASK-009/MS-003 In Progress·Gate4/5 Not Ready를 유지한다. 현재 변경은 feature 분기/unsafe를 건드리지 않아 이 영향 회귀를 적용했다. EV-014의16-feature matrix/rustdoc/Miri936은 이전 소스의 이력으로 보존하며 이번 변경에서 다시 실행했다고 표시하지 않는다. native CI/계정 조회·차단 재시도·fresh audit·commit/push는 수행하지 않았다.

### EV-016 — TASK-010 시간·채널 조회와 active snapshot

- 승인/대상: 2026-10-30 Human 「승인, 커밋 푸쉬하고 다음 작업 진행하자.」로 TASK-009 최신 구현·REV-011/012 Closed 및 소스fe03468/문서aee3830 origin/main 게시 후 착수. TASK-008 최종 Human 리뷰·선행 native GAP-004는 별도 유지한다. 대상은 `aee3830` 위 dirty 작업 트리의 [source53개·환경/명령/로그 hash](../06_evolution/imported_context/task_010_2026-10-30/results.json), [실행 script](../06_evolution/imported_context/task_010_2026-10-30/verify.py)·[결과 대조 script](../06_evolution/imported_context/task_010_2026-10-30/consolidate.py)다. query/통합2개 신규, storage pin/검증 helper·lib/consumer/allocator/CI recipe 변경. dependency/lock·RT 수집/거절/Drop·unsafe는 동일하다.
- TDD: 미구현 API에서 [행동 Red6개](../06_evolution/imported_context/task_010_2026-10-30/red-query.log) 후 구현했다. [RV-018 만료 복원](../06_evolution/imported_context/task_010_2026-10-30/red-expiry.log)·[RV-019 blob Gone](../06_evolution/imported_context/task_010_2026-10-30/red-blob-expiry.log)·[RV-020 frame Limit](../06_evolution/imported_context/task_010_2026-10-30/red-frame-limit.log)는 각각 컴파일되는 반례 Red 후 조치했다. fixture의 clock/source 축 범위·shared offset·Cursor 이름 충돌, missing docs/Debug/scan tuple/clippy 정리는 harness/구현 정리이며 별도 제품 결함으로 세지 않는다. 대체 실행은 interim*에 보존하고 아래 최종 실행과 구분한다.

| 범위·검증 | 결과·유효 범위 |
|---|---|
| frozen complete prefix | 고정 session/revision·등록·독립 read-only source/유한 segment·safe offset0/완전 chunk·CRC/schema/channel/봉투·producer 순서 검증. 미완성/후속 chunk 비노출, 손상/상한 실패 시 부분 snapshot 없음 |
| 필터/순서·원본 | mono/real/명시 source domain ticks·채널/최소 level·반개구간. 역행/동시각 deterministic tie, page metadata+inline bytes/count 상한. 큰 원본은 blob 위치/길이·bounded read, 다른 snapshot/범위/만료는 Gone, cursor는 identity/Expired, 과거 now로 만료 복원 없음 |
| 읽기 수명/예산 | active pin이 회전/만료 뒤에도 삭제를 막음. 실제 reader Drop 중에도 pin slot이 남는 것을 검사하고 close 뒤에만 반환. 오류/취소/만료·pin slot 재사용·shared external 포화/누적 read·모든 예약 반환 검증. low-level pin 분실/forget의 강제 회수 없음 |
| query9·retention19·fault19 | 최종 두 Rust matrix에서 Pass. fake source/read lifecycle/기존 IO 회귀이며 실제 Unix FileStore 복구1개는 fault19에 포함 |
| 두 Rust 각16 features | Rust1.85/1.96 각각16조합·독립 core API consumer·예제 Pass. consumer는 실제 파일에 기록→sync→독립 descriptor로 query/receipt/payload 확인까지 수행 |
| clippy/rustdoc/fmt | 두 Rust workspace/all-targets/all-features clippy·rustdoc -D warnings 및1.96 fmt Pass |
| allocator10 | 두 Rust Pass, query256B/8MiB peak11143/25176199 ≤ external 예약110776/67217592 bytes, 최종 잔여0. 기존 RT 할당/해제0/0·복구/보존 예산 가드 유지 |
| Miri | nightly-2026-10-26 strict provenance, lib14+raw17+completion15+control18+routing19+retention18+fault18+query9=128×8=1024 Pass. 8 programs×seed0..7·64개 성공 수량 summary를 대조. 실제 파일/8MiB allocator 시험은 Miri 범위가 아님 |
| 근거 일치 | 최종 matrix9+Miri1=검사10개 exit0, 두 실행/현재 source53개 동일. `.log.gz`는 원본, `.log`는 후행/EOF 공백만 정규화한 뷰이며 각 hash는 results.json |

현재 closed 파일도 bounded data-section scan으로 query index를 재구성하며, 표준 summary를 이용한 scan 생략 최적화는 미구현이다. 파일 prefix 불변성/정확한 독립 descriptor·원래 등록은 host 계약이고 clock 변환·OS IO 강제 취소를 제공하지 않는다. 실제 p95/IO 간섭·native/다른 OS/전원/장비·replay/remote/CLI는 잔여다. source/sink host buffer·allocator 내부·OS cache는 logger 예약 밖이다. [REV-013](reviews/REV-013_TASK-010.md)는 RV-018~020 조치·최종 회귀 Pass로 Resolved/Conditional이다. Human 최종 구현 리뷰 전 Closed로 올리지 않으며 TASK-010/MS-004 In Progress·TC-006 부분 실행·Gate4/5 Not Ready를 유지한다. native CI/계정 조회·차단 재시도·fresh audit·TASK-010 commit/push는 수행하지 않았다.

### EV-017 — TASK-010 REV-014 정렬 재사용·API 안내 보강

- 대상: 기준 HEAD `3eeaaa3` 위 미커밋 작업 트리, 제품 변경은 `query/mod.rs`와 `tc_006_query.rs`다. [명령·exit code·소스/로그 SHA-256](../06_evolution/imported_context/task_010_review_fixes_2026-10-30/results.json)에 검증 대상 파일52개의 hash를 고정했다. 이전 EV-016은 당시 source53개에 대한 이력이다.
- 실행: 2026-10-30 / AI / 로컬 Linux x86_64 / Rust1.85.0·1.96.0, Miri nightly-2026-10-26. native/장비 수락 환경은 아니다.
- REV-014 RV-021: 동일 Query(축/from/to/channel/최소 level)의 order를 재사용하고 다른 질의는 같은 예약 버퍼에서 재구성한다. rebuild 직전 무효화, 정렬·생존 검사 성공 뒤 캐시 게시. immutable rows·snapshot identity·cursor 위치·원본 재읽기/예산 차감·취소/만료 의미를 보존한다. 추가 heap cache나 `sort_by_cached_key`를 도입하지 않는다. 고정 `Option<Query>`는 기존 65536B 기본 여유 안이며 allocator peak는 동일하다.
- RV-022/023: `Payload::Inline`은 해당 Page 전용 좌표이며 같은 Page record로 `Page::inline`을 사용하도록 rustdoc/계약을 보강했다. `read_bytes`는 모든 source safe_offset 합계의 초기 scan 차감+후속 inline/blob 재읽기 합계이며 반복 호출도 다시 차감한다.
- 기능 보존 리팩토링이므로 신규 실패 동작을 만들지 않았다. 새 교차 cursor 회귀를 수정 전 baseline에서도 통과시킨 뒤 구현했다. 초기 signed 시간축 상한을 잘못 준 fixture 실패는 `harness-invalid-bounds.log`에 구분 보존하며 제품 Red로 세지 않는다.

| 검증 | 결과·유효 범위 |
|---|---|
| query/retention | query10+retention19 Pass. 모든 필터 변경/빈 결과·이전 cursor 재개·동일 Query 새 cursor·취소 복귀/만료 역행·원본 바이트 회귀. reader close/error/cancel/expiry/회전 기존 통합 경로 포함 |
| 두 Rust 각 core/all-features workspace | Pass. 중간 feature 조합 14개씩은 이번 내부 변경에서 반복하지 않았으며 EV-016의 이전 전체16조합 이력을 현 소스의 재실행으로 표시하지 않는다 |
| 독립 실제 파일 API consumer | 두 Rust Pass — 기록/sync/독립 read-only descriptor 조회·receipt/원본 대조 |
| clippy/rustdoc/fmt | 두 Rust workspace/all-targets/all-features clippy·rustdoc `-D warnings`, 1.96 fmt Pass |
| allocator10 | 두 Rust Pass. query256B/8MiB peak11143/25176199 ≤ 예약110776/67217592B, 잔여0. 기존 RT 할당/해제0/0 유지 |
| Miri | Pass — query10+retention18=28×8=224, strict provenance seed0..7. 실제 파일 retention1개는 기존 Miri 제외 계약 유지 |

[REV-014](reviews/REV-014_TASK-010_CODE_REVIEW.md) RV-021~023은 로컬 조치/검증 Pass로 Resolved다. 원본 로그는 `.log.gz`, `.log`는 줄 끝/EOF 공백만 정규화한 뷰이며 각각의 hash를 results.json에 기록했다. 실제 CPU 처리량·p95/IO 간섭은 측정하지 않았으며 native GAP-004·장비·fresh audit·commit/push는 미실행이다. 변경 없는 pool/control/routing unsafe 영역의 Miri 전체 반복은 하지 않았다. TASK-010/MS-004 In Progress, Human 구현 리뷰 대기·Gate4/5 Not Ready를 유지한다.

## Gate 판정 메모

Gate 4/5: Not Ready — 구현/실제 환경 검증 전. Gate 3는 quality_gate.md의 설계 판정을 따른다.

## Open Evidence Gaps

| ID | 미검증 범위/마지막 revision | 담당·다음 조건 | 상태 |
|---|---|---|---|
| GAP-001 | 실제 OS/CPU/SSD/제어 주기/NIC/UPS 미정. 실행 근거 없음 | Human/AI, TASK-022 환경 확정 후 TC-011·012 | Open |
| GAP-002 | TASK-002/003 로컬/native Pass, TASK-004~006 로컬/Human 승인. TASK-007 EV-011 로컬 Pass·Human 승인, TASK-008 EV-012/013 회전/보존·리뷰 조치 로컬 Pass·Human 리뷰/native·TASK-009 EV-014/015 로컬 Pass·Human 승인/native·TASK-010 EV-016/017 로컬 Pass·Human 리뷰/native·조회·클라이언트/실장비 TC 잔여 | AI, 후속 Task별 TDD/인수 | Open |
| GAP-003 | 프로젝트 toolchain/lock/MSRV·로컬 feature 검증은 EV-001로 확인. 전역 default는 불필요 | AI, dependency 변경 시 동일 검사 재실행 | Closed — 로컬 toolchain 범위; native는 GAP-004 |
| GAP-004 | TASK-004 `f0ce6ee` run `40520558735`의 8개 job은 실행 전 차단. Human 2026-10-28 사용량 전량 소진 확인, 계정 재확인 불필요. TASK-005 EV-004·TASK-006 EV-009·TASK-007 EV-011·TASK-008 EV-012/013·TASK-009 EV-014/015·TASK-010 EV-016/017도 native Not Run. 마지막 native Pass는 TASK-003 `902043b` | CI 실행 여건 확보 뒤 source hash를 고정해 6 contracts+audit+raw-memory(분배 Miri 포함)를 AI가 확인. Human의 TASK-005~008 착수 지시는 개발 진행 예외이며 검증 면제가 아님 | Open — native 후속 검증 |
| GAP-005 | MCAP 간접 build 의존 `paste` 1.0.15 유지보수 중단 RUSTSEC-2024-0436; 로컬 및 2026-10-27 native CI 감사에서 동일 경고 1개 확인 | AI, TASK-007 의존성 재검토 또는 수정 upstream 출현 시 재검사. 취약점·unsound 검사는 계속 실패 조건 유지 | Open |
| GAP-006 | EV-006 자가 리뷰 RV-001~004는 EV-007 수정·신규9개/두 Rust16 features·Miri78×8·RT0/0 회귀 Pass. 수정 전 Red·과거 승인/시험 이력은 보존 | AI, 수정분 Human 리뷰/native(GAP-004) 후속. 현 소스 hash는 EV-007 | Closed — 로컬 수정/회귀 범위 |
| GAP-007 | EV-008 RV-005 P1은 EV-009에서 일반 sync 진입/게시와 shutdown 순서화 수정·정식 회귀3개·두 Rust 각16 features/consumer/예제·Miri81×8·RT0/0 Pass | AI, 최종 수정분 Human 리뷰/native(GAP-004) 후속. 수정 전 반례와 source38개 근거 보존 | Closed — 로컬 수정/회귀 범위 |


## TASK-005/006 승인·게시 완료 — 2026-10-28

Human 「승인, 커밋 푸쉬하고 세션 마무리해줘」로 TASK-006 구현 리뷰·게시·세션 종료를 승인했다. 코드 `c98b7fa4ba8928137f95d40ac2aa693a541ea934`의36개 source/manifest blob은 EV-005와 일치한다. TASK-005 승인은 앞선 요청으로 유지한다. `.cowork/`는 소스와 분리 커밋하며 `[skip ci]`로 CI 자동 실행을 피한다. 초기 HTTPS 인증 차단은 Human의 gh 설치·브라우저 승인 후 OS keyring 인증과 Git credential helper 연결로 해소했다. `git pull --ff-only origin main` 성공 후 코드·승인 문서 `136fe96800c52c1ff31150e75fc23eeba5d6ed4b`를 push했고 `git ls-remote origin refs/heads/main`으로 같은 원격 HEAD를 확인했다. 인증 상태 확인은 게시를 위한 것이며 사용량/과금/visibility/CI 조회나 차단 재시도는 하지 않았다. 원격 게시 성공은 native 검증 Pass/면제의 근거가 아니며 GAP-004는 유지한다.


게시 전 형식 검사(2026-10-28): 새 EV-004/005 `.log`는 줄 끝 공백/EOF 빈 줄만 정규화한 읽기용 뷰다. 정확한 실행 원본은 인접 `.log.gz`에 byte-for-byte 보존했고 각 results.json의 raw_log_archives에 원본/압축 hash를 기록했다. source/시험 의미·결과는 바꾸지 않았다.

2026-10-30 Human 「커밋 푸쉬 후, 다음 테스크 진행」에 따라 EV-013 소스 `14fd1a4`를 커밋했다(source49개/commit blob hash 일치). `.cowork` 문서와 분리해 게시하며 TASK-009 후속 착수는 승인됐다. TASK-008 최종 구현 리뷰·REV-009/010 Resolved·native GAP-004는 유지한다.

세션 마감 — 2026-10-30: Human 「세션 마무리해줘」 및 「커밋 푸쉬도 해줘」에 따라 TASK-009 소스 `e5f3147`를 커밋했다. EV-014 source51개/commit blob 및 검사10개 근거가 일치하여 시험을 반복하지 않았다. `.cowork` 검증/마감 문서212373c와 분리해 origin/main 게시 완료했으며, 구현 Human 승인·Resolved 리뷰·native/장비 이월 상태는 유지한다. [커밋 근거](../06_evolution/imported_context/task_009_2026-10-30/commit.json).

2026-10-30 후속 승인: Human 「승인, 커밋 푸쉬하고 다음 작업 진행하자.」로 TASK-009 최신 구현·EV-015 조치 승인, REV-011/012 Closed. 소스 `fe03468`은 EV-015 source51개와 commit blob hash가 일치하며 동일 성공 시험을 반복하지 않았다. [커밋 대조](../06_evolution/imported_context/task_009_review_fixes_2026-10-30/commit.json). native GAP-004·실장비 검증은 남으며 TASK-010 후속 개발 진행 승인은 검증 면제가 아니다.

후속 커밋 — 2026-10-30: Human 「커밋해줘」에 따라 TASK-010 소스 `8bbec13`를 로컬 커밋했다. EV-016 source53개와 commit blob hash가 일치하여 기존 성공 시험을 반복하지 않았다. [커밋 대조](../06_evolution/imported_context/task_010_2026-10-30/commit.json). 문서는 소스와 분리 커밋하며 푸시는 하지 않는다. 커밋 요청을 최종 구현 승인으로 해석하지 않고 REV-013 Resolved·Human 리뷰/native·장비 대기와 세션 종료 상태를 유지한다. 위 미커밋/commit 미실행 문장은 검증·마감 당시 이력이다.

2026-10-30 후속 지시 「커밋 푸시하고 다음 작업 시작하자」: EV-017 소스 `439d9c5` 로컬 커밋·source52개 commit blob hash 일치 확인, 동일 성공 시험은 반복하지 않았다. 문서는 별도 커밋·[skip ci] 게시하며 TASK-011 개발 진행은 승인됐다. 이 지시를 별도 최종 구현 리뷰 승인으로 확대하지 않아 REV-013/014 Resolved·native/장비 대기는 유지한다.

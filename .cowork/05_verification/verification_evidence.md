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
| 관련 Milestone | MS-002 |
| 관련 Test Strategy | `test_strategy.md` |
| 관련 Test Case | `test_case.md` |
| 버전 | 2026-10-28 / EV-001·002·003·004·005 |

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

## Gate 판정 메모

Gate 4/5: Not Ready — 구현/실제 환경 검증 전. Gate 3는 quality_gate.md의 설계 판정을 따른다.

## Open Evidence Gaps

| ID | 미검증 범위/마지막 revision | 담당·다음 조건 | 상태 |
|---|---|---|---|
| GAP-001 | 실제 OS/CPU/SSD/제어 주기/NIC/UPS 미정. 실행 근거 없음 | Human/AI, TASK-022 환경 확정 후 TC-011·012 | Open |
| GAP-002 | TASK-002 EV-001·TASK-003 EV-002 로컬/native Pass, TASK-004/005/006 코어 로컬 Pass. EV-007 자가 리뷰 결함 수정·회귀 Pass, 수정분 리뷰·실제 저장·클라이언트·실장비 TC 잔여 | AI, 후속 Task별 TDD/인수 | Open |
| GAP-003 | 프로젝트 toolchain/lock/MSRV·로컬 feature 검증은 EV-001로 확인. 전역 default는 불필요 | AI, dependency 변경 시 동일 검사 재실행 | Closed — 로컬 toolchain 범위; native는 GAP-004 |
| GAP-004 | TASK-004 `f0ce6ee` run `40520558735`의 8개 job은 실행 전 차단. Human 2026-10-28 사용량 전량 소진 확인, 계정 재확인 불필요. TASK-005 EV-004·TASK-006 EV-005도 native Not Run. 마지막 native Pass는 TASK-003 `902043b` | CI 실행 여건 확보 뒤 source hash를 고정해 6 contracts+audit+raw-memory(분배 Miri 포함)를 AI가 확인. Human의 TASK-005/006 착수 지시는 개발 진행 예외이며 검증 면제가 아님 | Open — native 후속 검증 |
| GAP-005 | MCAP 간접 build 의존 `paste` 1.0.15 유지보수 중단 RUSTSEC-2024-0436; 로컬 및 2026-10-27 native CI 감사에서 동일 경고 1개 확인 | AI, TASK-007 의존성 재검토 또는 수정 upstream 출현 시 재검사. 취약점·unsound 검사는 계속 실패 조건 유지 | Open |
| GAP-006 | EV-006 자가 리뷰 RV-001~004는 EV-007 수정·신규9개/두 Rust16 features·Miri78×8·RT0/0 회귀 Pass. 수정 전 Red·과거 승인/시험 이력은 보존 | AI, 수정분 Human 리뷰/native(GAP-004) 후속. 현 소스 hash는 EV-007 | Closed — 로컬 수정/회귀 범위 |


## TASK-005/006 승인·게시 완료 — 2026-10-28

Human 「승인, 커밋 푸쉬하고 세션 마무리해줘」로 TASK-006 구현 리뷰·게시·세션 종료를 승인했다. 코드 `c98b7fa4ba8928137f95d40ac2aa693a541ea934`의36개 source/manifest blob은 EV-005와 일치한다. TASK-005 승인은 앞선 요청으로 유지한다. `.cowork/`는 소스와 분리 커밋하며 `[skip ci]`로 CI 자동 실행을 피한다. 초기 HTTPS 인증 차단은 Human의 gh 설치·브라우저 승인 후 OS keyring 인증과 Git credential helper 연결로 해소했다. `git pull --ff-only origin main` 성공 후 코드·승인 문서 `136fe96800c52c1ff31150e75fc23eeba5d6ed4b`를 push했고 `git ls-remote origin refs/heads/main`으로 같은 원격 HEAD를 확인했다. 인증 상태 확인은 게시를 위한 것이며 사용량/과금/visibility/CI 조회나 차단 재시도는 하지 않았다. 원격 게시 성공은 native 검증 Pass/면제의 근거가 아니며 GAP-004는 유지한다.


게시 전 형식 검사(2026-10-28): 새 EV-004/005 `.log`는 줄 끝 공백/EOF 빈 줄만 정규화한 읽기용 뷰다. 정확한 실행 원본은 인접 `.log.gz`에 byte-for-byte 보존했고 각 results.json의 raw_log_archives에 원본/압축 hash를 기록했다. source/시험 의미·결과는 바꾸지 않았다.

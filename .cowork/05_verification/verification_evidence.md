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
| 버전 | 2026-11-01 / EV-001~032 |

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

INT-001 / 2026-11-01 #005 / Build, MS-005 In Progress. TASK-017 S2 로컬 합성 파일 실행·예산/취소·footer/sync/close·소유 경로 정리·결과 분류를 EV-032/REV-030 Closed로 수락했다. S3 운영 free probe·Report/CLI/host/backend·실앱/실측/UPS·native(T3)가 남고 Task/MS/TC 전체 및 Gate4/5 Not Ready를 유지한다. EV-031 이전 로컬 수락과 각 형상의 미검증 이력은 보존한다.

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

### EV-018 — TASK-011 mono 재생·유한 prefetch·시간/취소 제어

- 대상: TASK-010 소스439d9c5·문서35e3ac3 게시 후 작업 트리. 신규 `replay/mod.rs`, `tc_006_replay.rs`; query의 crate-private 1건 page/검증된 buffer 접근, lib export·retention/실제 파일 consumer/allocator·native recipe 변경. [검사 명령·소스54개/로그 SHA-256](../06_evolution/imported_context/task_011_2026-10-30/results.json)에 고정한다.
- 실행: 2026-10-30 / AI / 로컬 Linux x86_64 / Rust1.85.0·1.96.0, Miri nightly-2026-10-26. 실제 장비 수락 환경은 아니다.
- TDD: 미구현 replay import/API Red 뒤 첫 가상시계4개 Green. 자가 리뷰 RV-024(늦은 소비 중 배속 변경 후 후속 간격 붕괴)·RV-025(blob IO 오류 뒤 terminal 정리 누락)의 행동 Red2개→수정→회귀 Green. 잘못된 invalid-seek fixture는 원 query.to가 u64::MAX+1이라 u64::MAX seek가 유효했던 harness 오류로 구분하며 제품 Red로 세지 않는다.

| 범위·검증 | 결과·유효 범위 |
|---|---|
| replay10 | mono/tie 순서·leading gap/빈 선택 End·0.5/1/2배속·pause/resume·host now 역행·late consumer·배속 변경 중 overdue·checked deadline overflow, seek의 세대/미전달 폐기/paused/End 재개·이전 blob 세대/잘못된 range 거절·1건 prefetch/반복 blocked IO 차감 없음·누적 read 상한/오류/취소/만료/stop 예약 반환 |
| query10·retention19 | 기존 snapshot 페이지/정렬/blob 계약 회귀, custom reader Drop 중 pin을 반환하지 않는 것을 검증. 기존 reader 수명 시험에 replay 정상/IO/취소/expiry 4경로를 추가해 실제 close 이후만 pin slot/예약 재사용 |
| 두 Rust 각16features | Pass — 16조합·독립 actual-file consumer·예제/allocator. consumer는 파일 기록/sync→독립 read-only descriptor→query 후 Replay로 receipt/inline payload2건과 End/stop/예약 반환을 대조 |
| clippy/rustdoc/fmt | 두 Rust workspace/all-targets/all-features clippy·rustdoc `-D warnings`, 개발1.96 fmt Pass |
| allocator10 | 두 Rust Pass — query 후 replay256B/8MiB·blocked poll16회·bounded blob256B, peak11143/25176199 ≤ 예약110776/67217592B, 잔여0. RT 할당/해제0/0 유지. Replay는 Snapshot의 버퍼/예약을 공유하고 추가 heap prefetch queue를 만들지 않음 |
| Miri | Pass — query10+replay10+retention18=38×8=304, strict provenance seed0..7. 프로그램3개/seed별 성공 summary24개 대조. 실제 FileStore 시험1개는 기존 Miri 제외 |

[REV-015](reviews/REV-015_TASK-011.md) RV-024/025 조치·최종 검사10개 exit0로 Resolved다. source54개는 matrix/Miri/현재 hash가 일치하며 `.log.gz`는 원본, `.log`는 후행/EOF 공백 정규화 뷰로 각 hash를 기록했다. prefetch1건·동기식 borrowed frame/세대 제어는 로컬 범위이며 외부 adapter의 이미 복사/전송한 record를 실제 취소했다고 주장하지 않는다. 원본의 누락 건수를 시간 간격에서 추정하지 않고 빈/종료 선택을 End로 보고한다. End는 마지막 blob read를 위해 snapshot을 유지하고 stop/Drop·terminal 오류에서 실제 reader close 후 반환한다. 현재 배속은 양수 u32 유리수, 제어 전환은 정수 media ns로 내림하며 실제 timer가 아니다.

native GAP-004·다른 OS·실제 timer 지터/처리량/p95·수집 IO 간섭/장비·remote/live/CLI·fresh audit·TASK-011 commit/push는 미실행이다. TASK-011/MS-004 In Progress·TC-006 하위 검증만 완료하며 Human 리뷰와 Gate4/5 Not Ready를 유지한다. 기존 unsafe pool/control/routing을 변경하지 않아 해당 Miri 전체는 반복하지 않았다.

### EV-019 — TASK-011 REV-016 반입·방어적 정리/진단 API

- 승인/대상: 2026-10-31 Human의 프로젝트 브리핑·리뷰 반입 및 처리·TASK-011 요약 요청. HEAD `2315426`(재생 소스 `026999f`) 위 미커밋 작업 트리의 `replay/mod.rs`·`tc_006_replay.rs` 2파일 변경이다. [명령/환경·source54개/로그 SHA-256](../06_evolution/imported_context/task_011_review_fixes_2026-10-31/results.json)·[재현 script](../06_evolution/imported_context/task_011_review_fixes_2026-10-31/verify.py) 참조.
- 실행: 2026-10-31 / AI / 로컬 Linux x86_64 / Rust1.85.0·1.96.0, Miri nightly-2026-10-26. 실제 수락 장비가 아니다.
- RV-026: 현 불변식 `anchor_host <= due <= now`와 `late=now-due`에서 지적된 checked_add overflow는 불가능하다. 결함 주장은 성립하지 않으며 방어적으로 fail→stop 정리만 통일했다. 상한 근처 늦은 전달과 다음 deadline 한도의 예약 반환 시험은 수정 전에도 Pass([baseline](../06_evolution/imported_context/task_011_review_fixes_2026-10-31/baseline-host-limit.log)); 이를 결함 Red로 세지 않는다.
- TDD: RV-027/028의 접근자·Display·REALTIME·Debug 부재 compile [Red](../06_evolution/imported_context/task_011_review_fixes_2026-10-31/red-public-diagnostics.log) → 구현 후 Green. 공개 진단과 host 경계 시험 2개를 추가했다.

| 범위·검증 | 결과·유효 범위 |
|---|---|
| 두 Rust workspace/all-features | Pass — replay12·query10·retention19 및 기존 전체 단위/통합/doc·allocator10. actual file replay core consumer도 각 별도 실행 Pass |
| clippy/rustdoc/fmt | 두 Rust workspace/all-targets/all-features clippy·rustdoc `-D warnings`, 개발1.96 fmt Pass |
| allocator | 두 Rust RT 할당/해제0/0 유지, query/replay256B·8MiB peak11143/25176199 ≤ 예약110776/67217592B·잔여0 |
| 영향 Miri | replay12×8=96 Pass, strict provenance seed0..7. 병렬 출력의 `passed/failed` 8개 summary 대조 |

검사10개 exit0·검사 전후 source54개 hash 일치. `.log.gz` 원본과 `.log` 후행/EOF 공백 정규화 뷰를 각각 hash로 기록했다. [REV-016](reviews/REV-016_TASK-011_CODE_REVIEW.md) Open→In Progress→Resolved 뒤 2026-10-31 Human 최종 구현 승인으로 REV-015/016·RV-024~028을 Closed로 마감했다. 신규 unsafe/RT/의존성·lock/feature 구성 변경0이며 현 API 소비 경로(poll·배속/seek/pause·blob·종료, 실제 파일 consumer, 예약/RT guard)를 회귀했다. 16 feature 조합과 query/retention Miri 전체는 영향이 없어 반복하지 않았으며 이전 EV-018 범위와 구분한다.

native GAP-004·다른 OS·장비/실제 timer·remote/live/CLI·fresh audit·커밋/푸시는 미실행이다. TASK-011은 Human 최종 구현 승인·REV-015/016 Closed이며 TASK-011/MS-004 In Progress·Gate4/5 Not Ready·TASK-008/010 Human 최종 리뷰 대기를 유지한다.

### EV-020 — TASK-011 REV-017 반입·배속 비교/오류 진단·MSRV 유지

- 요청/대상: 2026-10-31 Human 「프로젝트 브리핑하고, 리뷰 반입하고, 승인 해야 하는 업무 보고해줘」. HEAD `6c833ea`(기존 재생 소스9660d7d) 위 미커밋 작업 트리. query/mod.rs·replay/mod.rs·tc_006_replay.rs 3파일 변경. [명령/환경·source54개/로그 SHA-256](../06_evolution/imported_context/task_011_final_review_fixes_2026-10-31/results.json)·[재현 script](../06_evolution/imported_context/task_011_final_review_fixes_2026-10-31/verify.py).
- 실행: AI / 2026-10-31 / 로컬 Linux x86_64 / Rust1.85.0·1.96.0·Miri nightly-2026-10-26. 현재 PC의 제품 성능 수락 시험과 구분한다.
- TDD: `is_equivalent` API 부재 compile [Red](../06_evolution/imported_context/task_011_final_review_fixes_2026-10-31/red-speed-equivalence.log), 오류 Display 행동 [Red](../06_evolution/imported_context/task_011_final_review_fixes_2026-10-31/red-error-display.log) → 최소 구현 Green. 두 테스트를 추가했다. RV-029는 경고/호환 정렬로 형식적 실패 테스트를 만들지 않았다.

| 검증 | 결과·유효 범위 |
|---|---|
| 두 Rust workspace all-features | Pass — replay14·query10·retention19 및 기존 단위/통합/doc·RT allocator10. 배속 비율과 u32 최대 교차곱/성분 보존, 원인별 메시지·IO 분류/텍스트 보존 검증 |
| 실제 파일 consumer | 두 Rust core Pass — 실제 기록/sync→query/replay·종료·예약 반환 |
| clippy/rustdoc/fmt | 두 Rust workspace/all-targets/all-features `-D warnings`·rustdoc `-D warnings`, 1.96 fmt Pass |
| allocator | 두 Rust RT 할당/해제0/0. query/replay256B·8MiB peak11143/25176199 ≤ 예약110776/67217592B·잔여0 |
| 영향 Miri | replay14 단일 seed·strict provenance Pass. `RUSTFLAGS=-D deprecated`로 query 예약 호출의 nightly 경고 처리도 확인. 기존 EV-019의 8 seeds와 구분 |

검사10개 exit0·검사 전후 source54개 hash 일치. `.log.gz` 원본과 `.log` 정규화 뷰 hash를 보존한다. [REV-017](reviews/REV-017_TASK-011_FINAL_REVIEW.md) RV-029~031 Resolved·추가 조치 Human 승인 대기. RV-029는 표준 라이브러리 소스 근거로 MSRV1.85가 지원하지 않는 try_update 전환 대신 한 함수의 deprecation 억제를 수용했다. RV-030은 Eq 성분 비교와 set_speed 동작 유지, RV-031은 오류 variant·terminal 처리/QueryError 표시·원 IO를 유지한다. 새 H/J 설계 결정·ADR 승격 대상은 없다.

native GAP-004(공개 후 T3), PC 성능/실제 timer·UPS 시뮬레이션·다른 OS·fresh audit·commit/push는 미실행. RT/unsafe·의존성/lock/feature 구성 변경 없음. 16 feature 조합·기존 unsafe 영역 전체 Miri는 영향이 없어 반복하지 않았다. TASK-008/010 최종 구현 리뷰·TASK-022 수치 프로필 잔여, TASK-011/MS-004 In Progress·Yellow·Gate4/5 Not Ready 유지. 기존 EV-018/019 Human 승인은 이번 추가 조치의 승인으로 확대하지 않는다.

### EV-021 — TASK-022 local-pc-v1 프로필 준비·읽기 전용 사전 점검

- 요청: 2026-10-31 Human 「승인, 다음작업 진행하자」. TASK-008 EV-012/013·TASK-010 EV-016/017·TASK-011 EV-020 최종 구현 승인, REV-009/010/013/014/017 Closed. 기존 source54개 hash가 EV-020과 일치해 성공 시험을 반복하지 않았다.
- 준비 대상: [local-pc.toml](../../test-profiles/local-pc.toml)·[사전 점검 script](../../scripts/check-test-profile.py)·[TASK-022 초안](../04_implementation/tasks/TASK-022.md). [관측](../06_evolution/imported_context/task_022_profile_2026-10-31/preflight.json)·[점검/소스/참조 hash](../06_evolution/imported_context/task_022_profile_2026-10-31/checks.json)·[선언 결과](../06_evolution/imported_context/task_022_profile_2026-10-31/plan.json)·[준비 미충족 결과](../06_evolution/imported_context/task_022_profile_2026-10-31/require-ready.json).
- 범위: AI / 현재 Linux generic / 읽기 전용 환경·공간/CPU/도구 관측, 프로필 선언 검증. run 디렉터리/시험 데이터 생성·부하·설정 변경/도구 설치·제품 성능/RT/UPS·native CI·commit/push 없음.
- 문서 첫 검토: 장비 방식은 확정됐으나 P0 도구·기한/합성 작업량·저장 경로/cap·CPU/SMT·RAM2배 실행량이 비어 있는 상태를 확인하고 Draft에 구체화했다. 제품 결함 Red 또는 성능 합격으로 세지 않는다.
- 점검: 기본 선언 exit0(Valid/NotRun), `--require-ready` exit2(승인·RT 부팅·성능 harness·실앱 통합·UPS adapter 잔여). 기간60초/RAM2배 미달, 메모리 합계 불일치, CPU 중복, 최소RAM 조건 약화, UPS 전달지연 예산 초과의 5개 부정 입력을 각각 exit1로 거절했다. 체크가 범용 스키마/모든 malformed 입력을 검증한다고 주장하지 않는다.
- 관측: generic7.0.0-42·CPU2↔10/3↔11 SMT·`/data/build` ext4 noatime·여유341,256,384,512B. 선언 P3 본측정 payload123,867,955,200B, 실제 MemTotal2배119,500,521,472B 이상. 예열/metadata/active 포함168,550,023,168B ≤ cap171,798,691,840B, cap+reserve240,518,168,576B ≤ 현재 여유다. 실제 실행 전에 재검사하며 OS cache/앱 메모리를 logger 예산 밖에서 별도 기록한다.

**판정은 프로필 준비 완료·수치 기준 Draft·제품 시험 Not Run이다.** 장비/UPS 방법은 ADR-011 승인 유지, 새 기한/작업량·경로/자원 한도 확정은 Human 검토 대상(J). TASK-022 In Progress, OQ-002/GAP-001 프로필 확정·실측·GAP-004 공개 후 이월 유지. TC-012/TC-011·P0~P6 합격이나 R1 성능 달성 근거가 아니다.

EV-021 후속 — 2026-10-31 Human의10ms 요청으로 합성 control 주기/기한을10ms, 연산500µs 유지로 조정했다. [새 선언/hash/준비 점검](../06_evolution/imported_context/task_022_profile_2026-10-31/control-10ms-checks.json)에서 Valid exit0·require-ready exit2를 확인했다. 기존 checks.json의 프로필 hash는 [20ms 스냅샷](../06_evolution/imported_context/task_022_profile_2026-10-31/profile-control20ms.toml)과 대응하며 현재 프로필 hash는 새 근거를 따른다. 실제 dot-probe 설정·제품 소스는 수정하지 않았다. 전체 프로필 Draft·제품 성능/RT/UPS NotRun 유지.

EV-021 수치 승인 — 2026-10-31 10ms 주기 조정 보고 후 Human 「승인」으로 local-pc-v1 Approved를 반영했다. [승인 스냅샷/hash·사전 점검](../06_evolution/imported_context/task_022_profile_2026-10-31/approval-checks.json)에서 선언 Valid exit0·require-ready exit2, pending에서 승인 대기 제거를 확인했다. 이전 Draft/20ms·10ms 검증 근거는 각각 당시 스냅샷과 대응하며 현재 프로필 hash는 승인 근거를 따른다. 제품 실행 NotRun·TASK-022 In Progress·선행 harness/실앱 통합/UPS adapter·RT 부팅·GAP-004 공개 후 이월은 유지한다. 앞선 Draft/수치 검토 대기는 승인 전 이력이다.

### EV-022 — TASK-012 bounded gRPC·수신 ACK·host 관리·TLS

- 요청/대상: 2026-10-31 Human 「권장대로 진행」, HEAD `6c833ea` 위 기존 승인 변경을 보존한 미커밋 작업 트리. remote 모듈/서버·native DTO/receiver·proto·external ledger 공유·TLS feature/lock edge·독립 consumer/allocator 및 feature/proto 검사. [소스70개/검사11개/로그 SHA-256](../06_evolution/imported_context/task_012_2026-10-31/results.json)·[재현 script](../06_evolution/imported_context/task_012_2026-10-31/verify.py).
- 실행: AI / 2026-10-31 / 로컬 Linux x86_64 / Rust1.85.0·1.96.0 / Miri nightly-2026-10-26. PC 정량 수락 프로필/P4 LAN 성능 시험과 구분한다.
- TDD: [공개 API 부재 Red](../06_evolution/imported_context/task_012_2026-10-31/red-public-api.log)→native API/구현. 자가 점검 RV-032 malformed auth·RV-033 stale generation·RV-034 reply Vec capacity·RV-035 blob 요청 범위 초과의 행동 Red→조치→Green을 보존했다. [REV-018](reviews/REV-018_TASK-012.md) 참조. 초기 두 차례 성공 실행은 interim-before-reply-capacity/interim-before-blob-range에 분리하고 현재 최종70 source만 현재 근거로 사용한다.

| 검증 | 결과·범위 |
|---|---|
| 두 Rust 각16feature | Pass — core/remote/DB/tracing/zstd 조합, 네트워크 없는 독립 core consumer·remote native consumer·예제·RT allocator core10/remote11 |
| 원격 전용13개 | unit10+public3 Pass. 실제 gRPC8MiB32조각·CRC/인계 뒤 ACK·ACK 전 Pending/미확인 종료 Unknown·Gap 범위/reason·읽기/관리 권한·TLS·malformed auth/버전·TTL/offset/checksum/세대·command timeout 후 실제 owner permit 유지·reply encoded/capacity/요청 길이 |
| 실제 파일/host bridge | OS File write/flush된 complete chunk→독립 read-only snapshot Query blob capability→ReadBlob3bytes·Replay pause·Completion 관측. 실제 수신 ACK 뒤 remote Complete, 미확인 종료는 Unknown, 두 경우 파일 Written 유지 |
| 정적·문서·schema | 두 Rust workspace/all-targets/all-features clippy/rustdoc `-D warnings`,1.96 fmt Pass. proto9message83field/tag/type/modifier 대조 및 설치 protoc compile Pass |
| requested-byte 계측 | remote hub 초기화+receiver256B/8MiB peak5744/8655984 ≤ 합계 예약2668288/27833344B·잔여0. 기존 RT 할당/해제0/0 유지. TLS/HTTP2/runtime 전체 heap/RSS·OS cache/host buffers는 별도 headroom이며 이 계측 범위 밖 |
| 영향 Miri | public3+receiver2+hub1=6×8 seeds=48 Pass·strict provenance. 성공 summary24개 대조. 소켓/TLS/File는 native 로컬이며 Miri로 확대하지 않음 |
| dependency/feature 경계 | package/version 집합 변경0, [lock 변경 근거](../06_evolution/imported_context/task_012_2026-10-31/dependency-edges.json)의 tonic→tokio-rustls·rustls→log edge만 활성화. core consumer에 tokio/tonic/네트워크/압축 미필수 |

검사11개 exit0·검사 전후 source70개 hash 일치. `.log.gz`는 원본, `.log`는 후행/EOF 공백 정규화 뷰로 각각 hash를 기록한다. [REV-018](reviews/REV-018_TASK-012.md) RV-032~035 Resolved·Human 최종 구현 리뷰 대기. Task/MS는 In Progress·Yellow·Gate4/5 Not Ready 유지. 승인된 §14/ADR-007·009 정책 구체화이며 새 ADR은 만들지 않았다.

native GAP-004 공개 후(T3), 다른 OS·실제 LAN/정량 timer/전체 RSS·PC 성능/UPS·dot-probe/운영 보안 정책·TASK-013 CLI/SDK·TASK-016/017 등 관리 backend·fresh online audit·commit/push는 미실행이다. 서버의 테스트 실행은 종료했고 상시 데몬/실제 배포를 수행하지 않았다. 기존 unsafe pool/control/routing 전체 Miri는 변경 영향이 없어 반복하지 않았다.

### EV-023 — TASK-012 REV-019 연결 포화·credential 비교·receiver 수명 보강

- 대상: 소스 `4f791194d1cf2256ee275fec2fe89fab5ad4f3f1`. [source70개/검사9개/hash](../06_evolution/imported_context/task_012_review_fixes_2026-10-31/results.json)·[재현 script](../06_evolution/imported_context/task_012_review_fixes_2026-10-31/verify.py)·[commit blob70개 일치](../06_evolution/imported_context/task_012_review_fixes_2026-10-31/commit.json). EV-022는 이전 형상의 이력이다.
- 실행: 2026-10-31 / AI / 로컬 Linux x86_64 / Rust1.85.0·1.96.0 / Miri nightly-2026-10-26. PC 성능/RT/UPS 수락 실행이 아니다.
- TDD: 고정 크기 Credential primitive 부재 compile Red, 기존 포화 경로에서 즉시 자기 wake1회 행동 Red → 각각 Green. 기능 시험을 타이밍 공격 측정/constant-time 증명으로 세지 않는다.
- 변경: 초과 연결 거절 후10ms timer wake, bind 시 고정256B credential 정규화·subtle2.6.1 bytes/길이 비교, 연결별 receiver/gate 한도·예산 수명 지침. 기존 lock package/version 집합 그대로, remote 전용 subtle 직접 edge1개만 추가했다.
- 결과: 두 Rust각16feature·core/remote 독립 consumer·RT allocator core10/remote11·프로토콜9message83field·clippy/rustdoc `-D warnings`·fmt Pass. 새 회귀3개(포화/credential/gate), 기존 소켓/TLS/관리·ACK/파일/완료 회귀 포함. 영향 Miri receiver3×8=24 + credential1=25 Pass. Miri의 기존 wire native dead_code 경고2개는 socket 테스트 제외에 따른 것이며 native 경고0과 구분한다.
- 원본 로그 `.log.gz`와 공백 정규화 뷰 `.log` hash, source 검사 전후 동일을 결과 JSON에 보존했다. 기존 unsafe 전체 Miri는 변경 영향이 없어 반복하지 않았다.
- 수락: Human 기존 로컬 구현 승인·마감 위임을 근거로 검증 뒤 AI 수락. REV-018/019 Closed, TASK-012는 로컬 Human 리뷰 완료/native·실측 잔여로 In Progress다. TASK-013 착수 승인 유지.
- 미검증: native 공개 후(T3)·다른 OS·PC 성능/RT/UPS/LAN/RSS·실제 dot-probe/운영 backend·전체 인증 시간 특성. Gate4/5 Not Ready 유지.

### EV-024 — TASK-013 착수·클라이언트 연결별 수신 수명 API

- 대상: 소스 `ded6372655eda685a8de7bc39013247446f8e135`, 새 dot-logger-client crate와 workspace/lock 등록. [source74개/검사12개/log hash](../06_evolution/imported_context/task_013_start_2026-10-31/results.json)·[검증 script](../06_evolution/imported_context/task_013_start_2026-10-31/verify.py)·[commit blob74개 일치](../06_evolution/imported_context/task_013_start_2026-10-31/commit.json). 외부 package/version은 그대로이며 로컬 workspace package1개만 추가했다. TASK-012 코드 조치 근거는 EV-023을 유지하고 현재 통합 workspace snapshot은 EV-024다.
- 실행: 2026-10-31 / AI / 로컬 Linux x86_64 / Rust1.85.0·1.96.0 / Miri nightly-2026-10-26. 실제 client transport/CLI·성능/RT/장비 수락 실행이 아니다.
- TDD: 공개 ClientReceiver/ClientError/ReceiverConfig 부재 compile Red → 통합3개 Green. 재연결 후 old fragments/ready handoff/ACK 거절·서버 ID 재사용·외부 old owner 실제 종료까지 예약 보존·동일 연결 세대/consumer 거절·잘못된/중복/다른 연결 거절·잔여 예약0을 확인했다.
- 결과: 두 Rust workspace all-features·client no-default·독립 core consumer·clippy/rustdoc `-D warnings`·fmt Pass. workspace에 기존 core/remote·소켓/TLS/ACK/파일/allocator11 회귀가 포함된다. metadata graph에서 독립 core consumer와 기본 client가 Tokio/tonic/HTTP/압축/tracing/subtle을 요구하지 않음을 확인했다. 영향 client Miri3×8seeds=24 Pass(strict provenance), 검사 전후 source74개 불변. 원본 gzip/공백 정규화 뷰 hash를 별도 보존했다.
- 개발 자가 점검 [REV-020](reviews/REV-020_TASK-013_RECEIVER_START.md) Pass·위임된 첫 단위 수락/마감 Closed. TASK-013/MS-004 In Progress·Gate4/5 Not Ready 유지. 상위 Task 완료 체크를 채우지 않는다.
- 미구현/미검증: 실제 client socket/재연결·endpoint/TLS/token/버전·요청/stream transport·CLI JSON/exit/Ctrl-C·query→replay/오프라인 파일/예제/dot-probe. raw ACK 추출 후 실제 연결로 보내는 경쟁/수명은 transport 구현에서 검증해야 한다. native 공개 후(T3)·PC 성능/RT/UPS/LAN/RSS/다른 OS는 별도다.

<a id="ev-025"></a>
### EV-025 — TASK-013 native client/CLI·오프라인·취소/ACK 사용자 경로

- 대상: 소스 `29e55da0b2dbf46848fc3e413ff6777a79999f64`, [source92개/검사15개/로그 hash](../06_evolution/imported_context/task_013_2026-10-31/results.json)·[재현 script](../06_evolution/imported_context/task_013_2026-10-31/verify.py)·[commit blob92개 일치](../06_evolution/imported_context/task_013_2026-10-31/commit.json). EV-024는 첫 수신 수명 단위의 이전 형상이다. 기존 core/unsafe 제품 소스 변경0이다.
- 실행: 2026-10-31 / AI / 로컬 Linux x86_64 / Rust1.85.0·1.96.0 / Miri nightly-2026-10-26. 다른 OS/native/장비 수락을 뜻하지 않는다.
- TDD: transport 공개 API 부재와 CLI exit/JSON 행동 Red 후 구현. 개발 자가 점검 RV-039~045의 사전 입력 검사·등록 중복·모순/불량 완료 응답·재생 generation·일반 파일·인증 env·Gone 분류 Red→Green을 [REV-021](reviews/REV-021_TASK-013_CLIENT_CLI.md)에 보존했다.

| 검증 | 결과/범위 |
|---|---|
| 두 Rust 각16feature·선택 core/client 그래프 | Pass — core/remote/DB/tracing/zstd·독립 실제 파일 consumer/remote consumer·RT allocator core10/remote11. CLI를 포함한 workspace metadata 대신 선택 normal/build graph를 검사해 격리 유지 |
| 두 Rust workspace·기본 client | Pass — client codec1/기존 수신수명3/transport7·별도 CLI process10과 기존 core/소켓/TLS/파일/완료 회귀. client 기본 feature에서 네트워크 강제 없음 |
| 실제 transport | 8MiB32조각·CRC·인계 전 Pending/후 ACK·Gap/consumer 거절/disconnect Unknown·인계 뒤 취소 AckUnknown, TLS/Read/Manage/비밀값 비노출, 실제 host Command 취소 뒤 permit 유지/완료 후 재사용 |
| CLI 실제 사용자 경로 | 버전 NDJSON/고정 exit0/2/3/4·원래 read-only 파일 Query/Blob→remote Replay start/pause/resume/seek/stop·offline query/timed replay·실제 Ctrl-C/정리·출력 cap·stdout 성공만 ACK/닫힌 파이프 Unknown·독립 복구/source bytes 불변·등록 검증/Gone 표시 |
| clippy/rustdoc/fmt·schema | 두 Rust -D warnings Pass·fmt Pass. 서버/client 두 사본 각각9message83field·protoc Pass. SDK 예제 all-targets compile·CLI JSON 선언 예제 유효 |
| 영향 Miri | strict provenance·8seeds: 기존 수신 수명3×8=24+codec1×8=8, 총32 Pass. 실제 socket/TLS/process/OS file/signal은 native 로컬 실행이며 Miri 범위 밖 |
| cached audit/의존 | cargo-audit0.22.2 `--no-fetch --deny unsound --json` exit0, 캐시 DB1378 advisory·취약점0/unsound0·기존 paste 경고1. DB last-commit/updated는 출력 null, fresh online update 미실행. 기존 package/version 교체0, CLI 로컬1개·외부6개(anstyle1.0.14/clap4.6.6/clap_builder4.6.6/clap_lex1.1.0/errno0.3.14/signal-hook-registry1.4.8) 추가·MIT OR Apache-2.0 선언/MSRV 근거 결과 JSON |

검사 전후 source92개 동일·commit blob92개 일치. 원본 `.log.gz`와 공백 정규화 `.log`의 hash를 구분한다. 코드리뷰 기준의 개발 자가 점검 REV-021은 위임된 AI 로컬 수락으로 Closed다. Task 목록의 12열 헤더/데이터 대 13열 구분선 오류를 12열로 수정했고 UTF-8·26개 Task 보존·실제 Markdown 렌더 table0→1/헤더12/데이터312 cell을 확인했다. [원본/수정 hash·renderer 근거](../06_evolution/imported_context/task_013_2026-10-31/task-registry-repair.json)를 보존했다. Git/원격 동기화는 세션 로그를 따른다.

미실행/후속: GitHub 공개 후(T3) native·다른 OS/실제 dot-probe 화면/운영 보안·TASK-016/017 관리 backend·fresh advisory update·LAN/RSS/PC 성능/RT/UPS/24h. CLI의 Unavailable/Unsupported와 file sync/namespace sync 구분을 유지한다. TASK-013은 로컬 구현 완료/native 잔여로 In Progress, MS-004/Gate4·5 미완료다. 후속 TASK-014 자동 착수 없음.

<a id="ev-026"></a>
### EV-026 — REV-022 client/CLI·server 오류 및 구독 대기 조치

- 형상: `b3c08bd`, [source92개·검사14개·로그 hash](../06_evolution/imported_context/rev_022_fixes_2026-10-31/results.json)·[재현 script](../06_evolution/imported_context/rev_022_fixes_2026-10-31/verify.py)·[commit blob92개 일치](../06_evolution/imported_context/rev_022_fixes_2026-10-31/commit.json).
- TDD: [transport/CLI Red6개](../06_evolution/imported_context/rev_022_fixes_2026-10-31/red.json) → Green. handoff 사전 거절·유휴 local next owner 보존·Corrupt/과대응답 진단·tail preflight·정확 limit·host Command Drop 불명 표시. 추가 실제 TCP 분할 대기 만료/재대기 후 CRC/ACK 검증.
- 2026-10-31 로컬 Linux / Rust1.85.0·1.96.0 각각16feature/consumer·workspace·기본 client·clippy/rustdoc -D warnings, fmt·CLI 예제 검사14개 exit0. transport11·CLI13·수신수명3·codec1, 영향 Miri strict provenance8seeds×4=32 Pass. dependency/lock·wire 두 사본·core unsafe 변경0.
- [REV-022 §8](reviews/REV-022_TASK-013_INDEPENDENT_REVIEW.md#8-조치검증-및-위임-수락--2026-10-31-011) RV-046~055 로컬 조치/판정 수락 및 Closed. RV-049 최적화는 TASK-022 실측 뒤, RV-053 서버 lease 초과 유휴/재구독 연속성은 TASK-020 화면 수락 후속이다. heartbeat/lease wire 변경은 하지 않았다.
- 기존 승인을 재요청하지 않고 AI 위임 수락한다. native 공개 후(T3)·실앱/장기 tail/LAN/RSS/PC/RT/UPS/24h·구버전 바이너리 교차 검증·Gate4/5는 잔여다. cached audit는 dependency 불변으로 기존 EV-025를 재사용하며 fresh audit는 수행하지 않았다.

<a id="ev-027"></a>
### EV-027 — TASK-014 고정 계측·유한 이력·대상별 결과

- 형상: `d99ba3b`, [source97개/검사14개/로그 hash](../06_evolution/imported_context/task_014_2026-10-31/results.json)·[재현 script](../06_evolution/imported_context/task_014_2026-10-31/verify.py)·[디스크/commit blob97개 일치](../06_evolution/imported_context/task_014_2026-10-31/commit.json). 제품 unsafe/외부 dependency/lock·wire 변경0.
- TDD: [telemetry API 부재 compile Red](../06_evolution/imported_context/task_014_2026-10-31/red.json) → TC-009 로컬6개 Green. small/raw 제출·대여 실패·skip/reject·배출 후 수량·retired target·동시 조회·reset/301ring·시간 누락/foreign session·control 부족. 포화 private1개, allocator2개 추가(활성 수집·Drop/표본 할당0/해제0, enable live/peak<=추가 control 예약·1024회 이력 재할당0).
- 2026-10-31 로컬 Linux / Rust1.85.0·1.96.0 각각16feature/독립 consumer/workspace·기본 client·clippy/rustdoc -D warnings, fmt·overhead fixture debug128 smoke 검사14개 exit0. Miri nightly-2026-10-26 strict provenance8seeds×(TC-009 6+포화1)=56 Pass. 초기 allocator 시험의 drop_non_drop lint는 기존 프로젝트 방식으로 정리했으며 실패 로그/당시 results를 before-lint-fix-*로 보존했다.
- [REV-023](reviews/REV-023_TASK-014_TELEMETRY.md) 개발 자가 점검 Pass·기존 위임에 따른 AI 로컬 수락/Closed. 검증 근거는 위의 코드·파일/명령·환경 범위이며 MS-005/TC-009 전체 완료가 아니다.
- 미실행: 기본 계측 완료 처리량 비용≤5%·계측 포함 RT 지연·PC 실측/프로필·LAN/RSS/장기 부하·다른 OS/native 공개 후T3·실앱/자동1초 worker·원격 runtime metrics backend. fixture debug128은 실제 성능 수락이 아니며 수치는 로그의 smoke 이력이다. 용량/예측·추정 오차·설정 변경/비교/간편 점검은 TASK-015~017 및 관련 Task다. Gate4/5 Not Ready를 유지한다.

<a id="ev-028"></a>
### EV-028 — TASK-015 여유·병목·보존 리포트

- 대상: 소스 `a247d018525a67a42c8f76e0c3258fd076e51821`. [source100개/검사13개·명령/환경·로그 SHA-256](../06_evolution/imported_context/task_015_2026-10-31/results.json)·[재현 script](../06_evolution/imported_context/task_015_2026-10-31/verify.py)·[commit blob 대조](../06_evolution/imported_context/task_015_2026-10-31/commit.json). dependency/lock·RT/unsafe·wire 변경0.
- TDD: API 부재 compile Red→리포트14개, 후속 session clock/독립 quota·u64 horizon 초과/AtLimit·Estimated provenance assertion Red→Green. 실제 window→report 통합1개(계측 suite7개), 관리 build1024회 할당0/해제0 추가1·공개 core/remote consumer 회귀. [개발 자가 점검 REV-024](reviews/REV-024_TASK-015_REPORT.md) RV-056~059 조치 후 위임 로컬 수락/Closed.
- 두 Rust1.85.0/1.96.0 각각16feature/독립 consumer/기존 RT allocator·workspace/all-features·core-client·clippy/rustdoc -D warnings, fmt 검사13개 exit0. Miri nightly-2026-10-26 strict provenance8seeds×(report14+telemetry7)=168 Pass. 최종 source hash 불변·commit blob 일치. 첫 MSRV clippy의 closure 불필요 borrow는 공통 Copy 표현으로 조치하고 최종 소스 전체를 재검증했다. 실패 원본은 before-lint-fix에 보존한다.
- 보고 계약: session/revision/epoch/실제 구간·환경/부하/target/boundary 보존, 검증 profile 일치/유효성·미측정/포화/만료·Stale provenance, 손실 안정 큐 Overloaded·의도적 skip 분리. bytes/slots·출력정지·filesystem free/reserve와 output quota·순환 보존/실제 retained span을 구분한다. 정수 유리수·signed 순증가율·ns 올림·AtLimit/NoGrowth/BeyondRange를 사용한다. 원래 관측과 가정을 함께 유지한다.
- 합성 정확도: 4개 일정 증가율×5/13/60초의12조합에서 독립1초 step simulation과 buffer/disk 예측 오차≤20%를 확인했다. 이는 모델 산술 검증이며 실제 PC/RT IO 예측 오차 수락이 아니다. 파일시스템 여유 변화와 gross 실제 writes/deletes·물리 점유/원본 참조 유지의 출처/동일 구간·overflow 검증 및 capacity evidence는 호스트 책임이다.
- 게시: 소스 a247d01 origin/main push 및 [원격 HEAD 일치](../06_evolution/imported_context/task_015_2026-10-31/source-push.json) 확인. 소스/검증 문서는 별도 커밋이며 [skip ci]를 유지했다. native 실행·공개 전환/CI 조회를 수행하지 않았다.
- 미실행: 실제 capacity benchmark·물리 gauge/지연 수집/자동 worker·CLI runtime metrics backend·실앱, PC/RT 실제 예측 오차≤20%/계측 비용≤5%·LAN/RSS/UPS/장기 부하·다른 OS/native 공개 후(T3)·fresh audit. TASK-015/MS-005 In Progress·TC-009 전체·Gate4/5 미완료를 유지한다.

<a id="ev-029"></a>
### EV-029 — REV-025 계측 거절 범주·리포트·이력 복구 정정

- 형상: `eaf736c`, [source101개/검사14개·로그 hash](../06_evolution/imported_context/rev_025_fixes_2026-10-31/results.json)·[재현 script](../06_evolution/imported_context/rev_025_fixes_2026-10-31/verify.py)·[commit blob101개 일치](../06_evolution/imported_context/rev_025_fixes_2026-10-31/commit.json). dependency/lock·unsafe·wire 변경0.
- 권한/범위: Human 상세 브리핑 뒤 「권고대로 진행」, 기존 구현·검증·리뷰 마감·게시 위임. 공개 CaptureCounts/Report 범주 추가·history_len 반환형 정정을 API§15/데이터/README에 반영했고 새 기술·소유권·장기 불가역 결정이 없어 ADR 추가 없음. TASK-016은 Planned/미착수.
- TDD: [Red2개](../06_evolution/imported_context/rev_025_fixes_2026-10-31/red.json) — TooLarge만으로 Observed Overloaded, poison reset 뒤 sample 실패 → Green. 실제 small/raw/acquire의 용량/계약/수명 분리·raw 재시도·전체 총계/차분/reset·합산 포화·정상 자원 부하/pressure·Stale/invalid 진단을 검증했다. 수집 즉시 판정의 범주 store1개·CounterCell128B·RT 할당/해제0을 유지한다. 이력 Sample 크기 증가는 기존 checked control 예약과 allocator live/peak 검사로 포함한다.
- RV-061은 리뷰 제안③으로 수락: 새 공개 sample/target 이력 없이 호스트 유한 이력·실제 양끝/identity/boundary 대조·checked terminal 차분/최종 pending·누락/Busy/overflow 거절 절차를 문서화했다. 실제 0~11초 window(요청10초)에 target10 complete2/failed1/dropped1/unknown1/pending2·ingress6·다른 target 미혼합을 연결 시험했다. 호스트는 입력 identity와 계측 시점을 책임지고 concurrent 근접 시점 혼합은 유지한다.
- 로컬 Linux7.0.0-42-generic / Rust1.85.0·1.96.0 각각16feature·공개 core/remote consumer·workspace/all-features·기본 client·clippy/rustdoc -D warnings·fmt 검사14개 exit0. TC-009 계측10개/리포트15개·telemetry unit3개, Miri strict provenance8seeds×28=224 Pass. F-11 소비자(small/raw/acquire/일반 재시도·counter/window/report·consumer·allocator/예제) 회귀와 source hash 불변 확인.
- [REV-025](reviews/REV-025_TASK-014-015_INDEPENDENT_REVIEW.md) RV-060/062 수용·RV-061 대안 수용, Open→In Progress→Resolved 후 기존 위임으로 로컬 수락/Closed. 시도 부하율은 accepted+rejected_capacity이며 재시도 중복을 포함해 고유 입력/손실량으로 주장하지 않는다. acquire/계약/수명 거절은 분자 제외.
- 게시: 소스 eaf736c origin/main push 및 [원격 HEAD 일치](../06_evolution/imported_context/rev_025_fixes_2026-10-31/source-push.json) 확인. 문서는 별도 커밋으로 게시하며 [skip ci] 유지, CI/계정/과금 조회·native 실행·공개 전환은 수행하지 않았다.
- 미실행: native 공개 후T3·다른 OS, PC/RT 실제 성능/계측 비용≤5%/예측 오차≤20%, 물리 gauge/자동 worker·CLI runtime backend·실앱/장기 tail/LAN/RSS/UPS/24h·fresh audit. TASK-014/015/MS-005·TC-009 전체·Gate4/5는 미완료 유지.

<a id="ev-030"></a>
### EV-030 — TASK-016 revision 설정 변경·전후 비교

- 대상: 소스 `542c6e65999de69ea8f682b4418de1c0fa730990`. [source102개·검사10개/명령·환경·로그 SHA-256](../06_evolution/imported_context/task_016_2026-11-01/results.json) · [검증 script](../06_evolution/imported_context/task_016_2026-11-01/verify.py) · [commit blob102개 SHA-256 일치](../06_evolution/imported_context/task_016_2026-11-01/commit.json). 2026-11-01 / AI / 현재 Linux PC·generic kernel. dependency/lock·unsafe·wire 변경0.
- TDD: publication stub의 Conflict/부분 producer ack/old Delivery Busy 3개 [runtime Red](../06_evolution/imported_context/task_016_2026-11-01/publication-red.log) → Green. 전환 표본 Observed 오인 [runtime Red](../06_evolution/imported_context/task_016_2026-11-01/transition-red.log) → RevisionTransition/혼합 Window 금지 Green. Compare는 [API 부재 compile Red](../06_evolution/imported_context/task_016_2026-11-01/compare-api-red.log) 후 관측 rate/입력·측정 조건/미제공/Stale 차이를 검증했다.
- 설정13개·비교3개·실제 MCAP 단일 revision writer roundtrip1개·RT allocator1개 추가. 기존 수락 route/완료 보존·raw 실제 guard·동일 loan·sampling/Important·idle/부분 적용·이전 capture 후 enable·scope 재대여 위상·잘못된 patch/control 부족·재시작/닫힘/u64 최대·실제 여러 생산자5세대 변경·동일 base 동시 apply 성공1/Conflict1을 확인했다. 실제 수집 before20→after10+skip10과 Report/Snapshot 비교를 연결했고 CPU 절감을 추정하지 않았다.
- Rust1.96.0/MSRV1.85.0 각각16feature·독립 core/remote consumer/기본 client 의존 격리·RT allocator15개·예제, workspace/all-features·clippy/rustdoc -D warnings, fmt **검사10개 exit0**. Miri nightly-2026-10-26 strict provenance8seeds×(lib17+routing19+설정13+비교3+계측10)=**496 Pass**. 검증 중 source102개 hash 불변·commit blob 일치. 새로운 RT 변경 적용/skip/raw/계약 거절/handle Drop 할당/해제0.
- [개발 자가 점검 REV-027](reviews/REV-027_TASK-016_CONFIG_COMPARE.md) Pass·미해결 P1/P2/P3 없음·기존 위임 로컬 수락/Closed. [API §15.2](../03_design_artifacts/interface_contract.md#152-task-016-로컬-설정-변경전후-비교--2026-11-01)의 두 bank·bounded 회수/ack·Conflict/Busy·Normal sampling·기존 target route, 전환/비교 provenance 계약을 확인했다. 신규 핵심 기술/ADR/프로토콜 변경 없음.
- 게시: 소스 542c6e6 origin/main push 및 [원격 HEAD 일치](../06_evolution/imported_context/task_016_2026-11-01/source-push.json) 확인. 문서는 별도 [skip ci] 커밋으로 게시하며 공개/CI/계정/과금 조회나 native 실행은 하지 않았다.
- 미실행: 원격 ApplyConfig/Compare 운영 backend·CLI runtime 연결/자동 worker·실앱, 실제 PC/RT 지연·계측 비용≤5%/오차≤20%, native 공개 후T3/다른 OS·fresh audit·UPS/24h·장기 부하. 단일 revision MCAP 전환은 호스트가 명시 구동한다. TASK-016/MS-005·TC-009 전체·Gate4/5는 미완료다. native 이월은 면제가 아니다.

<a id="ev-031"></a>
### EV-031 — REV-028 반입 조치·TASK-017 S1 계획/예산 착수

- 일자/실행자: 2026-11-01 #004 / AI. 대상: review 조치 `5d447b4` + 계획 API `0622a9025b6753605f3c8d4e92e61abd7c27eab6`. 변경 Rust7개는 현재 commit blob SHA-256과 일치. [명령/결과·대상 hash](../06_evolution/imported_context/task_017_start_2026-11-01/results.json).
- REV-028 RV-063/064: 실제 `Producer::apply_pending_config()`의 유휴 worker 호출·잔여 old 자원 조건, compare 안정 Window/새 baseline 사용 지침을 rustdoc/API §15.2에 반영. 전환 표본·혼합 구간 거절→안정 before/after Report·Comparison 종단과 인접/겹침 경계17개 Pass. 로직 변경 없이 문서·시험 보강, REV-028 Closed.
- TASK-017 S1: `diagnostics::CheckPlan`·명시 예산/메모리 계산 선언·경로 문법·신규 physical write allowance를 API 부재 Red(exit101/E0432)→Green 신규5개로 구현. 무부하/운영 파일 불변·directory 미생성, 시간/쓰기 선도달·reserve/취소·u64 경계·수신자 없음/loopback/remote 미측정 선언 검증. 경로 소유권/실제 메모리 예약·IO 실행 증명이 아니며 결과 Pass/Validated 성능을 생성하지 않음.
- 검증 환경: 현재 Ubuntu PC, 개발 Rust1.96.0, MSRV1.85.0, lock 변경 없음. `cargo test --workspace --all-features --locked`329개(문서 시험 포함), 관련22개 Rust1.96/MSRV1.85 각각 Pass. `cargo fmt --all --check`, workspace/all-targets/all-features clippy -D warnings, workspace rustdoc -D warnings Pass. 명령5개 exit0; 로그 원문은 .log.gz, 읽기용 공백 정규화 뷰는 .log에 보존. Miri는 S1에 unsafe/동시성 변경 없어 추가 실행하지 않았다.
- 수락: [REV-028](reviews/REV-028_TASK-016_INDEPENDENT_REVIEW.md) 반입 조치와 [REV-029](reviews/REV-029_TASK-017_PLAN_START.md) 개발 자가 점검의 S1만 기존 위임으로 수락/Closed. 독립 리뷰/Task 전체 완료로 확장하지 않는다. 기존 소비자(수집/완료/라우팅·MCAP·조회/재생·원격/client/CLI·RT allocator)는 workspace 종단 회귀로 확인했다. 기존 게이트/불변식 강화는 없고 additive 계획 API다.
- 미구현/미검증: TASK-017 S2의 실제 명시 run·전용 경로 배타 소유·부하 재현/실제 physical IO·drain/sync/close/cleanup·최종 범위/Pass/Fail/Aborted/Inconclusive, CLI/runtime/원격 backend 및 실앱/PC/RT 비용/오차·UPS·장기/native(T3)/다른 OS·fresh audit. TC-010 Partial·Task/MS In Progress·Gate4/5 Not Ready 유지.

<a id="ev-032"></a>
### EV-032 — TASK-017 S2 실제 합성 파일 실행·동기화·정리

- 대상/실행자: 소스 `1cae3b93aa64662cc29d945f1c84dba0680a82a4`, 2026-11-01 #005 / AI / 현재 Ubuntu PC. [명령7개·Red witness·source4개 SHA-256/commit blob 일치](../06_evolution/imported_context/task_017_run_2026-11-01/results.json). 무압축 product MCAP 한 파일/한 채널·calling worker1이며 생산 capture/라우팅/원격 부하 검증이 아니다.
- TDD: run API 부재 compile Red, 기록 뒤 공간 관측/시계 오류가 prefix footer를 생략하는 반례 assertion Red, 교체된 출력 경로가 synced로 집계되는 반례 assertion Red → 최소 구현/수정 Green. 원본은 .log.gz, 공백 정규화 뷰는 .log로 보존했다.
- 실제 파일15개: 임시 directory/file create_new·기존 운영 bytes 불변·완료/예산 선도달/취소·초기 reserve/관측 오류·진행 중 관측/시계 오류 뒤 prefix finish/sync·공간 급감 footer 실패·경로/symlink/identity 변경·foreign file cleanup 거절·메모리/identity/workers/workload 거절·unsupported receiver/network 무실행·실제 단조 시계. 실제 MCAP prefix/완성 파일의 독립 읽기·payload/revision 및 writer 예산 원복 확인.
- 내부3개: 짧은 쓰기/부분 OS 오류·seek 재쓰기를 포함한 actual Write bytes·cap/reserve backstop·원 OS/sync 오류 보존. 신규 unsafe/RT/원자 연산·dependency/lock/wire 변경 없음. [API §15.4](../03_design_artifacts/interface_contract.md#154-task-017-s2-로컬-합성-파일-점검--2026-11-01).
- 최종 검증: Rust1.96.0 workspace/all-features **347개(문서 시험 포함)**, core lib/S1/S2 관련 **40개**, MSRV1.85.0 관련 **40개**, fmt·workspace all-targets/all-features clippy -D warnings·workspace rustdoc -D warnings Pass. Miri nightly-2026-10-26 strict provenance 내부3종×8seeds=**24 Pass**. 검사7개 exit0·source4개 commit blob/hash 일치. 실제 OS IO 시험은 Miri로 대체하지 않는다.
- 수락: [REV-030](reviews/REV-030_TASK-017_FILE_RUN.md) 개발 자가 점검 Pass/신규 미해결 RV 없음·기존 위임 S2 로컬 수락/Closed. scope는 synthetic file write/finish/sync/close/정리다. written/synced/실제 Write bytes/파일 길이·원래 stop과 최종 outcome·primary/settlement/cleanup 원인을 보존하며 네트워크/장비 capacity Profile/생산 완료 ACK를 만들지 않는다.
- 미구현/미검증: S3 운영 OS free probe·Report/profile/CLI/runtime/원격 backend·실제 capture/라우팅/receiver/실앱, PC/RT 비용/오차/처리량·UPS/장기·native(T3)/다른 OS·fresh audit. 기능 fixture의 공간 값은 제어된 입력이며 실제 디스크 성능/space/자원 수락 증거가 아니다. trusted namespace 외부 변경/panic/kill 후 자동 복구/정리는 보장하지 않는다. Task/MS In Progress·TC-010 Partial·Gate4/5 Not Ready 유지.

<a id="ev-033"></a>
### EV-033 — TASK-017 S3 로컬 운영 host·Report/profile·CLI 연결

- 소스: `5390a55194cf5bf80ecc74f461e24a07d8f6af10`, source101개 commit blob/hash 일치·검사24개 exit0. [커밋 대조](../06_evolution/imported_context/task_017_s3_2026-11-01/commit.json).

- 일자/실행자: 2026-11-01 #006 / AI / 현재 Ubuntu PC generic kernel. [명령/소스 hash·결과](../06_evolution/imported_context/task_017_s3_2026-11-01/results.json)·[검증 script](../06_evolution/imported_context/task_017_s3_2026-11-01/verify.py)·[REV-031](reviews/REV-031_TASK-017_CHECK_HOST.md) 개발 자가 점검 Closed. [API §15.5](../03_design_artifacts/interface_contract.md#155-task-017-s3-로컬-운영-hostcli-연결--2026-11-01).
- TDD: host API/handle_command 부재 compile Red → Green. 공간 probe 중 시각/취소 도래 뒤 새 record 수락(RV-066) 및 CLI Inconclusive exit0(RV-067) assertion Red → Green. 운영 source session과 전용 check session의 과도한 동일성 검사(RV-068)도 별도 session 보존 반례 Red→Green으로 조치했다. Red witness5개를 같은 근거 폴더에 보존했다.
- 신규10개: core host3·산술1·느린 probe1·실제 CLI5. 무부하 preview·실제 statvfs/파일·원 Report/profile 보존/foreign 거절·만료 유지·고정 슬롯/새 ID·run 단일 사용, read/manage 권한·응답 부족 pre-IO/ID 유지·remote timeout pacing 취소 후 actual settlement·기존 namespace 보존·결과별 exit를 검증했다. 최소 server message5120B와 source profile 포함 JSON의 실제 TCP/CLI 종단 확인.
- 최종 검증: Rust1.96 workspace/all-features357개(문서 시험 포함), core/no-default-features288개, MSRV1.85 workspace/all-features357개 Pass. fmt·workspace all-targets/all-features clippy -D warnings·rustdoc -D warnings·protobuf/prost/protoc Pass. Miri nightly-2026-10-26 strict provenance 관리 수명/공간 산술2종×8seeds=16 Pass. 실제 OS IO/PC 기능 시험을 Miri 성능 수락으로 바꾸지 않는다. 전체16 feature matrix는 이번 회차에 반복하지 않았으며 core/전체 feature/실제 remote CLI 경로를 회귀했다.
- Linux ordinary-path safe rustix1.1.4(std/fs)·linux-raw-sys0.12.1 추가, 기존 package 버전 교체/own unsafe/RT 경로 변경0. audit 실행 DB2295466c(2026-10-30), lock dependency190개·취약점0·unsound0·기존 paste 유지보수 경고1(GAP-005)이다. 실제 capacity/profile 생성·native CI를 실행하지 않았다.
- 게시: 소스5390a55 origin/main 게시·[원격 HEAD 일치](../06_evolution/imported_context/task_017_s3_2026-11-01/source-push.json), 검증/수락 문서는 별도 [skip ci] 커밋.
- 수락: 기존 위임으로 S3 로컬 운영 연결 단위만 수락. 원래 session/revision/epoch/window/conditions·profile 유효구간과 실제 파일 결과를 분리한다. source null/만료를 합성 capacity로 채우지 않고 네트워크/점검 capacity Unavailable을 유지한다. 응답 완료 전까지 Command/permit·실제 IO 소유권 유지, 결과 Vec capacity=len·pre-IO response 상한 확인.
- 미구현/미검증: 생산 capture/routing/실제 수신자 ACK·dot-probe/실앱, PC/RT 처리량/비용≤5%·추정 오차≤20%, UPS/장기/native(T3)/다른 OS. 자동 host worker/결과 저장소/daemon·restart exactly-once는 제공하지 않는다. Task/MS In Progress·TC-010 Partial·Gate4/5 Not Ready·FR-015/TASK-018 미착수 유지.

## 검증 수락 환경·시점 갱신 — 2026-10-31

Human 승인 [ADR-011](../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md): 현재 PC 실측을 실제 장치 테스트 대체 근거로 수락하고, 실물 UPS 없이 시뮬레이션 결과로 판정한다. GitHub native는 공개 완료 후 진행한다. [환경 관측](../06_evolution/imported_context/verification_policy_2026-10-31/environment.json)은 PC 사양/실행 커널 확인이며 성능/UPS 시험 Pass가 아니다.

실행7.0.0-42-generic(CONFIG_PREEMPT_RT 미설정), 설치7.0.0-38-realtime(CONFIG_PREEMPT_RT=y)을 구분한다. 일반/RT 시험은 실제 부팅 커널별로 기록하고 RT 행은 부팅 확인 뒤 실행한다. UPS 모의 시간축·전체 약3초/지연/순위/오류·실제 sync/중단/복구를 '시뮬레이션 수락'으로 판정한다. 이전 실물 장비/UPS 필수 조건은 ADR-011 범위에서 갱신하며 기존 EV의 과거 실행/미실행 이력은 보존한다.

TASK-022 환경/프로필 준비를 시작했다. 아직 수행하지 않은 실제 시계/성능/24h·UPS 시뮬레이션과 native를 Pass로 바꾸지 않으며 Task 완료/Gate4·5는 개별 실행 근거로 판단한다. 이번 변경은 문서/환경 조사이고 제품 소스·시험은 변경/재실행하지 않았다.

## Gate 판정 메모

Gate 4/5: Not Ready — 구현/실제 환경 검증 전. Gate 3는 quality_gate.md의 설계 판정을 따른다.

## Open Evidence Gaps

| ID | 미검증 범위/마지막 revision | 담당·다음 조건 | 상태 |
|---|---|---|---|
| GAP-001 | ADR-011 현재 PC/UPS 방법 및 local-pc-v1 수치 프로필 Human 승인(EV-021). generic 관측·RT 설치 구분, 제품 실측은 NotRun | AI, 선행 harness·실앱 통합/UPS adapter 준비 및 RT 부팅/capability 확인 뒤 해당 행 실행 | Open — 실행 준비/실측 잔여 |
| GAP-002 | TASK-002/003 로컬/native Pass·TASK-004~011 로컬 Pass/Human 승인. TASK-012/013 EV-026·TASK-014/015 EV-029·TASK-016 EV-030까지 로컬 수락·REV-001~027 Closed. REV-028 조치·TASK-017 S1/S2 EV-031/032·REV-029/030 Closed, 생산 점검·운영 free probe/backend·실앱/실측 TC 잔여 | AI, Task별 TDD/인수·실측 및 공개 후 GAP-004 | Open |
| GAP-003 | 프로젝트 toolchain/lock/MSRV·로컬 feature 검증은 EV-001로 확인. 전역 default는 불필요 | AI, dependency 변경 시 동일 검사 재실행 | Closed — 로컬 toolchain 범위; native는 GAP-004 |
| GAP-004 | TASK-004 f0ce6ee run40520558735의8job 실행 전 차단, 마지막 native Pass TASK-003 902043b. TASK-004~017의 최신 변경 native는 Not Run. 2026-10-31 ADR-011로 저장소 공개 완료 후 실행하는 승인된 이월 | AI, T3: GitHub 공개 완료 확인 시 당시 source hash 고정·6 contracts+audit+raw-memory 실행/확인. 공개 전 CI/계정/과금 조회·차단 재시도 없음. 공개 전 준비/로컬 검증 진행 허용 | Open — 공개 완료 후 재개 |
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

세션 마감·게시 — 2026-10-30: Human 「커밋, 푸쉬해주고 세션마무리」에 따라 TASK-011 소스 `026999f`를 origin/main 게시했다. EV-018 source54개·검사10개 exit0·로그19개 원본/뷰 hash 및 모든 commit blob hash 일치를 확인해 성공 시험을 반복하지 않았다. [커밋 대조](../06_evolution/imported_context/task_011_2026-10-30/commit.json)·[소스 게시 근거](../06_evolution/imported_context/task_011_2026-10-30/push.json). 검증/마감 문서는 별도 커밋·[skip ci]로 게시하며 위 commit/push 미실행 설명은 검증 당시 이력이다. REV-015 Resolved·TASK-011/MS-004 In Progress·Human 리뷰/native GAP-004·장비·Gate4/5 Not Ready를 유지한다.

2026-10-31 Human 「승인」: 구현 계약 설명 후 TASK-011의 EV-018/019 구현/조치를 최종 승인했다. REV-015/016 Closed·Human 리뷰 체크 완료. 현재 source54개는 EV-019 hash와 일치해 시험을 반복하지 않았으며 native GAP-004/장비/실제 timer 검증·Gate4/5 Not Ready를 유지한다. 신규 변경은 미커밋·미게시이며 커밋/푸시·다음 Task 착수는 수행하지 않았다.

## 세션 마감·게시 — 2026-10-31

리뷰 조치 소스 `9660d7d4b084a5eb222153ef7ca897d2166192dc`를 origin/main에 게시·원격 hash 일치 확인했다. [EV-019 commit blob 대조](../06_evolution/imported_context/task_011_review_fixes_2026-10-31/commit.json)에서 source54개가 검증 hash와 일치하며 검사10개 exit0/Miri96·로그12쌍 원본/뷰 hash를 재확인해 시험을 반복하지 않았다. [소스 게시 근거](../06_evolution/imported_context/task_011_review_fixes_2026-10-31/push.json). 이전 미커밋/commit·push 미실행 표기는 당시 이력이다.

REV-015/016 Closed·TASK-011 Human 승인·ADR-011 수락/이월을 유지한다. 현재 PC/UPS 시뮬레이션 시험은 아직 미실행, TASK-022는 프로필 준비 In Progress다. native는 공개 완료 후(T3) 실행하며 공개/CI 조회·재시도·커널 변경/재부팅은 수행하지 않았다. Task/MS·Yellow·Gate4/5 Not Ready는 그대로다. 검증/마감 문서는 소스와 별도 커밋·[skip ci]로 게시하고 최종 문서 HEAD/원격 일치는 세션 로그에 기록한다.

## TASK-008/010/011 최종 구현 승인 — 2026-10-31

Human 「승인, 다음작업 진행하자」로 직전 승인 보고 3묶음을 수락했다. EV-012/013·EV-016/017·EV-020 관련 REV-009/010/013/014/017과 해당 RV를 Closed로 마감하고 Task Human 리뷰 체크를 완료했다. 앞선 EV 본문의 Human 대기 표시는 이력이다. 낮은 pin 유실/forget의 강제 회수 없음과 실제 reader 종료 뒤 반환·현재 PC 실측/UPS·native 공개 후(T3) 검증 경계를 수락한 것이며 Task/MS 전체·Gate4/5 완료는 아니다.

## TASK-012 세션 마감·게시 — 2026-10-31

Human 세션 마감 및 커밋/푸시 요청으로 소스 `5f0ad83d171bcc1553691ed17d25f5b71ab1fac7`를 origin/main에 게시했다. EV-022 source70개/검사11개·Miri48·원본/뷰16쌍 hash 및 commit blob70개 일치를 재확인해 성공 시험은 반복하지 않았다. [커밋 근거](../06_evolution/imported_context/task_012_2026-10-31/commit.json)·[소스 게시 근거](../06_evolution/imported_context/task_012_2026-10-31/source-push.json). 앞선 미커밋/미게시 표기는 검증 당시 이력이다. 검증/승인/마감 문서는 별도 [skip ci] 커밋으로 게시하고 최종 문서 HEAD와 두 상태 파일의 원격 blob 일치는 세션 로그에 기록한다.

REV-018 Resolved·TASK-012 최종 구현 Human 리뷰 대기, Task/MS In Progress·Yellow·Gate4/5 Not Ready와 TASK-022 수치 승인/제품 성능 NotRun을 유지한다. native/CI는 공개 후(T3), 실제 LAN/전체RSS/운영 backend/CLI·실앱/PC 실측/UPS는 후속이다. 상시 서버·새 Task·배포·계정/과금/CI 조회나 차단 재시도는 수행하지 않았다.

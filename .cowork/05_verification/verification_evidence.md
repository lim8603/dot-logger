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
| 버전 | 2026-10-27 / EV-001·002 |

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

INT-001 / 2026-10-27 / Build, MS-002 In Progress·TASK-002 Done. TASK-002 구성/의존성 EV-001 로컬/native CI·Human 리뷰 Pass 이력을 유지한다. TASK-003 소형 ingress는 EV-002 로컬/native CI Pass·Human 리뷰 승인으로 Done이다. 원본/분배/완료/저장 런타임·실제 Windows 11/PREEMPT_RT·장비 검증은 남아 있다.

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

## Gate 판정 메모

Gate 4/5: Not Ready — 구현/실제 환경 검증 전. Gate 3는 quality_gate.md의 설계 판정을 따른다.

## Open Evidence Gaps

| ID | 미검증 범위/마지막 revision | 담당·다음 조건 | 상태 |
|---|---|---|---|
| GAP-001 | 실제 OS/CPU/SSD/제어 주기/NIC/UPS 미정. 실행 근거 없음 | Human/AI, TASK-022 환경 확정 후 TC-011·012 | Open |
| GAP-002 | TASK-002 EV-001 로컬/native, TASK-003 소형 ingress EV-002 로컬 Pass. 원본/회수·분배/완료·저장·클라이언트·실장비 TC 잔여 | AI, TASK-003 이후 TDD와 각 MS 인수 | Open |
| GAP-003 | 프로젝트 toolchain/lock/MSRV·로컬 feature 검증은 EV-001로 확인. 전역 default는 불필요 | AI, dependency 변경 시 동일 검사 재실행 | Closed — 로컬 toolchain 범위; native는 GAP-004 |
| GAP-004 | TASK-003 변경분 `902043b`의 Ubuntu 24.04/Windows Server 2025/macOS 14 × Rust 1.85.0/1.96.0 및 audit Pass, run `40512793054`; TASK-002 Pass 이력 유지 | AI, 코드·의존성·CI 변경 시 해당 revision 재검증. 실제 Windows 11/PREEMPT_RT·장비 수락은 GAP-001/TASK-022·024 | Closed — hosted native CI 범위 |
| GAP-005 | MCAP 간접 build 의존 `paste` 1.0.15 유지보수 중단 RUSTSEC-2024-0436; 로컬 및 2026-10-27 native CI 감사에서 동일 경고 1개 확인 | AI, TASK-007 의존성 재검토 또는 수정 upstream 출현 시 재검사. 취약점·unsound 검사는 계속 실패 조건 유지 | Open |

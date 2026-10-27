# TASK-002 — workspace와 플랫폼·설정 계약

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-002 |
| Story | US-001·005 |
| 요구사항 | FR-001 NFR-004 NFR-007 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~010 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-27 |
| 다음 액션 | Human 구현 리뷰 승인 완료. GitHub 연결·저장소 생성·push 후 native CI 확인(GAP-004), 완료 판정·TASK-003 인계 |
| 의존 | TASK-001 |
| 예상 소요 | 1~2 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/dot-logger/src/{lib,config,platform}.rs`, `crates/dot-logger/tests/tc_001_*.rs`, `tests/api-consumer`, `scripts/check-feature-matrix.py`, `.github/workflows/ci.yml`.

첫 실패/리뷰 조건: **quota 합 초과·unsupported required capability 구성의 시작이 실패해야 함**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-001`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] feature 조합·공개 API smoke·fmt/clippy/test, toolchain/patch lock·MSRV 결과 기록.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 시작 전 구성 실패·예산 경계·overflow·플랫폼 실패를 검증했다. 런타임 포화·취소/종료는 이 Task 범위가 아니며 TASK-003~006 등에서 검증한다.
- [x] Human 리뷰 완료 — 2026-10-27 사용자 「승인하고」로 TASK-002 구현 리뷰 승인. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-001](../../05_verification/test_case.md#tc-001). 제품 코드·테스트 실행 결과: [EV-001](../../05_verification/verification_evidence.md), Ubuntu 26.04.1 x86_64 로컬 Pass. Human 리뷰는 2026-10-27 승인 완료, native CI는 미완료. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 실행 결과와 리뷰 범위 — 2026-10-26

- 시작 `23a499c` 위 검증 작업 트리를 코드 커밋 `2f9015d`로 보존했다. 소스 hash·명령·로그는 EV-001에 연결했다.
- Rust 1.96.0 개발 pin, MSRV 1.85.0, Cargo lock. 두 버전 각각 16 feature 조합·독립 공개 API consumer·compile-fail·clippy, 개발 fmt/rustdoc Pass.
- 구성 테스트 13개: 총 64/65 MiB, producer별 input/raw 합계, 최대 단일 slot, Important slots/bytes, TreatEqually, sum/product/누적 overflow, worker stack, sparse CPU/중복/required affinity, optional/required capability 실패 상태.
- 전체 feature의 의존성 probe 6개: MCAP 무압축/zstd 왕복, rtrb·Protobuf, remote/DB/일반 로그 의존 API. 제품 어댑터 구현은 아님.
- MCAP 0.26.0 변경 Human 승인·ADR-010. Tokio 1.51.6 patch. 최종 감사 취약점/unsound 0, `paste` 유지보수 경고 1(GAP-005). 외부 177개 license metadata 확인.
- 시작 전 `validate`만 구현했다. target/schema/durability·build·예열·실제 worker/플랫폼 적용·metadata 포함 실제 할당 예산 검사는 해당 자원 구현 Task에서 추가한다. 물리 풀 예산과 로그 소유 메모리 예산을 시스템 RSS 보장으로 확대하지 않는다.
- 2026-10-26에는 Human 리뷰·native CI가 미완료였다. 2026-10-27 Human 구현 리뷰 승인 후 체크를 갱신했다. CI는 아직 실행 전(GAP-004)이며 실행 확인까지 Task 상태는 In Progress다.

## 세션 마감·재개

2026-10-26 Human 요청으로 커밋 후 세션을 종료했다. 재개 예정은 2026-10-27이다. 커밋 요청은 Human 구현 리뷰 완료로 간주하지 않으며, 다음 세션은 코드/EV-001 리뷰·GAP-004의 CI 실행 조건 확인부터 이어간다.

## 2026-10-27 승인과 GitHub 게시

Human이 TASK-002 구현 리뷰를 승인하고 GitHub MCP 연결·본인 계정 저장소 생성·업로드를 요청했다. GitHub 플러그인 설치/연결은 사용자 인증 대기다. 저장소명은 `dot-logger`, 공개 범위는 비공개로 준비하며 연결 확인 후 생성·push·CI 실행을 확인한다. 승인으로 native CI나 실제 장비 시험을 Pass 처리하지 않는다.

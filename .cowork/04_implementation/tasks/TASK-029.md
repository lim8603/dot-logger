# TASK-029 — 연결 레시피·문서·접점 회귀 가드

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-008 |
| Story | US-001·003 |
| 요구사항 | FR-001 NFR-004 NFR-008 |
| 설계 | [인터페이스](../../03_design_artifacts/interface_contract.md) · [CLI](../../03_design_artifacts/ui_spec.md) |
| ADR | ADR-007 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-027·TASK-028 |
| 예상 소요 | 1~2 인일, 리뷰 대기 제외 |

## 목표

소비처가 support 모듈 없이 따라 할 수 있는 단계별 레시피와 짧은 quickstart를 제공하고, 접점 크기를 수락 기준으로 측정·회귀 감시한다.

레시피 단계는 승인된 [API §18.6](../../03_design_artifacts/interface_contract.md#186-nfr-008-수락-수치-제안)을 따른다([§20.2](../../03_design_artifacts/interface_contract.md#202-레시피-단계186-기준)에서 초안 서술 정렬, 2026-11-12).

- **L1 최소 file 기록:** 시작→RT thread 기록→shutdown 판정(≤ 20줄·개념 ≤ 6).
- **L2 RT 다중 writer:** raw loan·종료 보고 확인(≤ 40줄·≤ 10).
- **L3 선택 연결:** tracing·remote 각각 L1에 +6줄 이하.
- README는 quickstart와 레시피 링크 중심으로 줄이고, 계약 상세는 모듈 rustdoc으로 옮긴다.
- 기존 `dot_probe_*` 예제를 facade 경로로 옮기고 support 줄 수 감소를 기록한다. → 2026-11-12 Human 결정: facade 전용 예제를 추가하고 pipeline(DB·설정 비교)은 고급 경로로 유지([EV-075](../../05_verification/verification_evidence.md#ev-075)).
- 공개 API 변경 감지: rustdoc JSON(nightly-2026-10-26) + 스크립트, 이름 있는 항목 ≤ 60과 전체 표면 snapshot(2026-11-12 Human 결정, [API §20.1](../../03_design_artifacts/interface_contract.md#201-human-결정--2026-11-12)).

## 작업 단위

| 단위 | 내용 | 상태 |
|---|---|---|
| S1 | `scripts/check-api-surface.py`(개수·snapshot)·`scripts/check-recipes.py`(줄·개념·금지 항목) | Human 승인 [EV-073](../../05_verification/verification_evidence.md#ev-073) |
| S2 | `examples/recipes/` L1·L2·L3(tracing·remote), `Shutdown::is_success()` | Human 승인 [EV-073](../../05_verification/verification_evidence.md#ev-073) |
| S3 | quickstart·README 축소·계약 상세 rustdoc 이동 | Human 승인 [EV-074](../../05_verification/verification_evidence.md#ev-074) |
| S4 | facade 전용 `dot_probe_facade` 예제 추가·support 줄 수 비교 기록(pipeline은 고급 경로로 유지, 2026-11-12 Human 결정) | Human 승인 [EV-075](../../05_verification/verification_evidence.md#ev-075), Human 승인 |

## 변경 대상과 첫 검증

경로: `examples/recipes/`·`scripts/check-integration-surface.py`·`tests/api-surface/logger.txt`(S1·S2), `README.md`·모듈 rustdoc(S3), `examples/dot_probe_facade.rs`(S4).

첫 실패/리뷰 조건: **레시피 예제가 support 모듈 없이 빌드·실행되고, 연동 코드 줄 수·도입 개념 수가 NFR-008 승인 수치를 넘으면 검사 실패**.

- [x] (EV-073 Red ①~④) 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] (F21·F22) 최소 구현 후 관련 `TC-013`의 레시피 하위 시나리오를 통과시킨다.
- [x] (EV-074·075·076) 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] (EV-073~075) 레시피 3단계·quickstart·rustdoc 이동·예제 이전과 접점 측정 결과.
- [x] (EV-073~076) 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] (`check-recipes.py`·L1~L3 실행) 레시피의 실패/포화·종료 처리 예시가 실제 계약과 일치한다.
- [x] Human 리뷰 완료 — 2026-11-12 「승인」, REV-054 Closed. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-013](../../05_verification/test_case.md#tc-013) F21·F22. 실행 결과: [EV-073](../../05_verification/verification_evidence.md#ev-073)~[EV-075](../../05_verification/verification_evidence.md#ev-075) 로컬 Pass, 2026-11-12 Human 승인.

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- CI `integration-surface`·레시피 native 실행: [TASK-024](TASK-024.md)(T3)

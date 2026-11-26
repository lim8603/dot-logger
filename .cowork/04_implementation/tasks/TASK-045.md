# TASK-045 — v2 실DB 재검증과 `Writer` 예제

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-012](../milestones/MS-012.md) |
| Story | US-002·003 |
| 요구사항 | FR-004 NFR-003 NFR-005 — TC-008 |
| 설계 | [API §14.6](../../03_design_artifacts/interface_contract.md#146-task-019-s1-influxdb-v2-유한-write--2026-11-01) · [`adapters::influxdb`](../../../crates/dot-logger/src/adapters/influxdb/mod.rs) rustdoc · [TASK-019](TASK-019.md) · API §18.6(레시피 한도) |
| ADR | ADR-009(InfluxDB v2·선택 feature·일반 경로) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-26 |
| 다음 액션 | REV-073 조치 뒤 S1(D1·D2 결정 완료) |
| 의존 | REV-073 조치(MS-012 M2) |
| 예상 소요 | 0.5~1 인일 |

## 배경

실DB 시험 `tc_008_influx_live` 4개와 `dot_probe_pipeline` 실DB 실행은 2026-11-01 이후 다시 돌리지 않았다. 그 뒤 REV-059~072 조치·CI 정리·플랫폼 수정이 들어갔다. InfluxDB를 쓰는 예제는 `dot_probe_pipeline`(support 889줄)뿐이다. README 첫머리 "InfluxDB v2 output"은 facade 경로로 읽힐 수 있다.

확인한 제약(2026-11-26):
- `examples/recipes/`는 facade 전용이고, `scripts/check-integration-surface.py`가 줄 수·개념 수(L3 = L1 + 6줄)와 금지 항목을 검사하며 CI에서 **각 레시피를 실행**한다. 실제 DB가 필요한 고급 경로 예제는 이 틀에 맞지 않는다.
- `Writer::new(&QueryBudget, Config)`의 `QueryBudget`은 검증된 세션 계획(`SessionBuilder` → plan)에서만 만든다. facade `Logger`는 이를 내주지 않는다.

## 결정

| ID | 질문 | 선택지 | 권장 |
|---|---|---|---|
| D1 | 예제 위치 | A. `crates/dot-logger/examples/influxdb_write.rs` 고급 경로 예제(레시피 가드 밖, CI는 컴파일만, 실행은 `check-influxdb.py --example`) / B. `examples/recipes/l3_influxdb.rs` + 가드 한도·실행 규칙 확장(§18.6 승인 한도 변경) | **A**(2026-11-26 Human) — 레시피는 facade 계약의 한도 검사이고, DB 예제는 고급 경로다 |
| D2 | `QueryBudget` 준비 | A. 현재 API로 `SessionBuilder` → plan → `QueryBudget`을 예제에 그대로 / B. facade에 DB용 예산 도우미 추가(API 변경) | **A**(2026-11-26 Human) — 줄 수를 재서 부담이 크면 facade DB 출력 기능 논의 안건의 근거로 올린다 |

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 기준선 재실행 | `python3 scripts/check-influxdb.py --pipeline-example --toolchain {1.96.0, 1.85.0}` — live 4개 + pipeline 시험·예제 Pass, cleanup 0. 실패하면 원인 처리 뒤 이 Task 계속. 출력은 비공개 증거 폴더 | Planned |
| S2 | Red | 하네스에 `--example` 옵션(같은 일회용 DB에서 `influxdb_write` 예제 실행·결과 확인). 예제가 없어 실패하는 것을 확인 | Planned |
| S3 | Green | 예제: `Config`(환경 변수 URL·org·bucket·token) → `Writer::new` → 숫자·이벤트 `Point` 몇 개 `write` → 결과(`Outcome`) 출력 → `query_numeric`으로 읽어 확인. 환경 변수가 없으면 사용법을 출력하고 종료 코드로 구분. 두 Rust에서 `--example` Pass. 줄 수 기록 | Planned |
| S4 | 문서 | README: Integrations 줄과 feature 표에 "고급 경로(`adapters::influxdb`)"임을 명시, 예제 표에 `influxdb_write` 추가. `adapters::influxdb` rustdoc에 예제 링크. CHANGELOG Unreleased | Planned |
| S5 | 마감 | 전체 회귀(두 Rust)·clippy·feature 조합·CI Pass, EV, 자가검토 | Planned |

## 완료 기준

- [ ] S1 v2.9.1 실DB 두 Rust Pass·EV.
- [ ] 예제 실DB 실행 Pass(두 Rust)·CI 컴파일.
- [ ] README·rustdoc 정정.

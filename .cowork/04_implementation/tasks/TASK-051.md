# TASK-051 — MS-013 단계 3: 설정 타입과 옵션 파일

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-013](../milestones/MS-013.md) 단계 3 |
| Story | US-001·004 |
| 요구사항 | FR-020 — [TC-015](../../05_verification/test_case.md#tc-015--출력-구성네트워크-전용전송-스위치) 3단계 |
| 설계 | MS-013 §5.5·§6·§8.1 FR-020 행 |
| ADR | [ADR-018](../../03_design_artifacts/adrs/ADR-018_config_file_and_merge.md) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-28 |
| 다음 액션 | S0 — TASK-050 Done 뒤 |
| 의존 | TASK-050 |
| 예상 소요 | 3~3.5 인일 |

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S0 | 의존성 확인 | `toml`의 MSRV 1.85·`cargo audit`·라이선스 확인, [tech_stack](../../03_design_artifacts/tech_stack.md) 갱신, API §25.3 | Planned |
| S1 | `LoggerConfig`·병합 | `LoggerConfig`(feature 없음)·`LoggerBuilder::config`·병합표 경우별·출력 전체 교체·교체 + 숫자 index 거절·자동 이름 참조 거절·`explain()`·`PlanSources`. 시험 Green | Planned |
| S2 | TOML 파일 | feature `config-file`: `from_toml_path`·`from_toml_str`, `version = 1`·`deny_unknown_fields`·1 MiB 상한·경로 기준·`SecretValue`, `ConfigFileError`(원문·값 미노출)·오류 우선순위 | Planned |
| S3 | 수락 시험 | 같은 바이너리 + 파일 3개(파일만/네트워크만/혼합) 시작·종료, feature 조합 4종, feature 없을 때 `Unsupported` | Planned |
| S4 | 마감 | README·rustdoc·예시 파일, 공개 표면 gate, CHANGELOG, 두 Rust 회귀·Clippy·feature·CI, EV, 독립 리뷰 | Planned |

## 완료 기준

- [ ] FR-020 수락 시험 Pass, 의존성 확인 기록.
- [ ] 표면 gate·CI Pass, 독립 리뷰 Pass, EV.

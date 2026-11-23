# TASK-044 — `Record` derive 또는 prost 연동

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-011 |
| Story | US-001 |
| 요구사항 | NFR-008(통합 접점 경량화) FR-001 |
| 설계 | [MS-011](../milestones/MS-011.md) · [API §18](../../03_design_artifacts/interface_contract.md#13-상세-api-설계안) · TASK-030(표준 MCAP 사용자 스키마) |
| ADR | prost 경로만이면 불필요할 수 있음. proc-macro crate(새 의존성 `syn`·`quote`)를 넣으면 ADR-009 개정 또는 신규 ADR |
| 담당 | AI |
| 상태 | Planned — 0.2 후보, MS-010 2단계 전 기능 논의에서 채택 결정 |
| 마지막 갱신일 | 2026-11-23 |
| 다음 액션 | 기능 논의 안건 → 채택 시 S0 선택지 결정 |
| 의존 | 기능 논의(Human) |
| 예상 소요 | 2~4 인일(A만 1~1.5) |

## 배경

기록 타입마다 `impl Record`에 `const SCHEMA`와 `encode`를 손으로 쓴다(레시피 `examples/recipes/`). `Schema::descriptor`로 protobuf `FileDescriptorSet`을 붙이면 Foxglove가 바로 해석하지만, 생성 도구·예제가 없다. Foxglove SDK는 derive와 protobuf 연동으로 이 부담을 없앤다. `prost`는 이미 의존성(손실 보고)에 있다.

## 선택지

| ID | 내용 | 장점 | 비용·위험 |
|---|---|---|---|
| A | **prost 연동:** `prost::Message` 타입을 `Record`로 쓰는 매크로 또는 래퍼(`prost_record!(Type, "pkg.Type", DESCRIPTOR)`), `encode`는 `&mut [u8]`에 직접(할당 없음), 크기 초과는 `EncodeError`, `prost-build`의 기술자를 `Schema::descriptor`에 | 새 의존성 없음, Foxglove 바로 해석, ROS 밖 사용자에 익숙 | `.proto` 작성 필요, String·Vec 필드는 RT에서 미리 채운 값만 |
| B | **`#[derive(Record)]`(proc-macro crate `dot-logger-derive`):** 고정 필드 구조체 → 인코딩과 스키마 자동 생성. 인코딩 후보: CDR + `ros2msg` 스키마(Foxglove·ROS 2 호환, T1과 연결) / 고정 리틀엔디언 + jsonschema 불가(Foxglove는 바이너리 jsonschema 해석 안 함) | Rust 타입만으로 끝남 | 새 crate·의존성(`syn`·`quote`·`proc-macro2`), 인코딩 형식 결정·ADR, crates.io 게시 순서(MS-010 3단계)에 crate 추가 |
| C | A 다음 B | 빨리 하나를 내고 derive는 수요 확인 뒤 | 두 번 공개 |

권장: **C**(A를 0.2에, B는 TASK-026 실사용 피드백이나 T1 ROS 2 검토와 함께).

## 작업 단계(채택 시)

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S0 | 결정 | 선택지 A/B/C, B면 인코딩·ADR | Planned |
| S1 | A 구현 | 연동 API, RT 할당 시험, Foxglove 판독 확인, 레시피·통합 표면(NFR-008) 갱신 | Planned |
| S2 | B 구현(선택 시) | derive crate, 생성 코드 시험, 표준 MCAP·Foxglove 판독, 다른 OS CI | Planned |
| S3 | 문서 | README·CHANGELOG·API 문서 | Planned |

## 완료 기준(채택 시)

- [ ] 사용자가 `encode`를 손으로 쓰지 않고 기록 타입을 정의할 수 있다.
- [ ] 생성된 기록을 Foxglove가 스키마로 해석한다.
- [ ] RT 할당 시험과 3 OS CI가 통과한다.

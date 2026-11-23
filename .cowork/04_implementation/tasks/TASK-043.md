# TASK-043 — RT용 미룬 서식 로그(서식 id + 인자)

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-011 |
| Story | US-001·002 |
| 요구사항 | NFR-001 FR-001 (UC-14 구조화 필드와 인접) |
| 설계 | [MS-011](../milestones/MS-011.md) · [API §18](../../03_design_artifacts/interface_contract.md#13-상세-api-설계안) · [데이터 모델](../../03_design_artifacts/data_model.md) |
| ADR | 신규 ADR 필요(공개 API·저장 형식) |
| 담당 | AI |
| 상태 | Planned — **0.2 후보**, MS-010 2단계 전 기능 논의에서 채택 결정 |
| 마지막 갱신일 | 2026-11-23 |
| 다음 액션 | 기능 논의 안건 → 채택 시 S0 설계·ADR |
| 의존 | 기능 논의(Human), TASK-042 하네스(비용 측정) |
| 예상 소요 | 4~7 인일 + ADR |

## 배경

RT 스레드에서 사람이 읽는 글줄을 남길 경로가 없다. tracing/`log` 입력(`logger/sink.rs`)은 일반 스레드 전용이고, RT에서는 사용자가 `Record` 타입을 직접 정의해야 한다. NanoLog·defmt는 서식 문자열을 컴파일 시점에 id로 바꾸고 RT에서는 id와 인자만 기록한 뒤, 읽을 때 서식을 만든다. 같은 방식이면 RT의 드문 글줄(경고·상태 전이)을 무할당·무서식으로 남길 수 있다. dot-probe에도 바로 쓰인다(TASK-026).

## 설계 쟁점(S0에서 ADR로)

| 쟁점 | 후보 |
|---|---|
| 서식 id | 컴파일 시점 상수 해시(서식+위치) / 링크 시점 정적 표(`linkme`·`inventory` 같은 의존성) / builder에 명시 등록 |
| 서식 표 저장 | 파일 metadata·예약 토픽(`.logger`, ADR-016)에 세션별 서식 표를 한 번 기록 — RT 첫 사용 때 등록하면 할당이 생기므로 시작 시점 수집이 필요 |
| 인자 인코딩 | 정수·실수·bool·짧은 `&str`(잘림 표시)·열거형 — 고정 크기 태그 + 값, 소형 slot 안 |
| API 모양 | `rt_log!(writer, Level::Warn, "x={} y={}", x, y)` 매크로 → 기존 `try_log` 경로·손실 회계·레벨 필터 그대로 |
| 읽기·표시 | `NativeReader` 보조 함수로 글줄 복원, CLI `query`·`tail` 출력, viewer에서 Foxglove `foxglove.Log`로 변환 또는 표준 MCAP 쓰기 시 Log 스키마 |
| 버전 호환 | 서식 표·인코딩에 형식 버전, 모르는 id는 원시 인자로 표시 |

## 작업 단계(채택 시)

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S0 | 설계·ADR | 위 쟁점 결정, ADR Human 승인 | Planned |
| S1 | 기록 | 매크로·인자 인코딩·서식 표 수집, RT 할당 시험(`tests/rt-allocation`) Pass | Planned |
| S2 | 저장 | 서식 표 기록·회전마다 재기록, 손실 회계 그대로 | Planned |
| S3 | 읽기·표시 | 복원 API, CLI·viewer 표시, 다른 OS 파일 교환 확인 | Planned |
| S4 | 비용·문서 | TASK-042 하네스로 호출 비용, README 레시피, CHANGELOG | Planned |

## 완료 기준(채택 시)

- [ ] RT 스레드에서 할당·서식·잠금 없이 글줄을 기록한다(RT 할당 시험).
- [ ] 기록된 파일에서 원래 글줄을 복원해 CLI·viewer로 볼 수 있다.
- [ ] 호출 비용이 TASK-042 표에 같은 방법으로 실린다.

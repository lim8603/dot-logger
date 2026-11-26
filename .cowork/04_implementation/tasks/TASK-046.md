# TASK-046 — InfluxDB 3 Core 호환 확인

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-012](../milestones/MS-012.md) |
| Story | US-002·003 |
| 요구사항 | FR-004 NFR-005 — TC-008 |
| 설계 | [`adapters::influxdb`](../../../crates/dot-logger/src/adapters/influxdb/mod.rs) rustdoc · [TASK-045](TASK-045.md) |
| ADR | ADR-009(InfluxDB v2 선택) — 지원 범위 기록만, 변경 없음 |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-26 |
| 다음 액션 | TASK-045 뒤 S0 |
| 의존 | TASK-045(하네스 `--example`·예제) |
| 예상 소요 | 0.5~1.5 인일 |

## 배경

Docker Hub `influxdb:latest`는 2026-04부터 InfluxDB 3 Core를 가리킨다. 새 사용자가 태그 없이 받으면 v3가 된다. v3 Core 문서는 v2 호환 쓰기 `/api/v2/write`와 v2 `Token` 인증을 지원한다고 적고, 질의는 SQL·InfluxQL이다. 우리 `query_numeric`은 Flux(`/api/v2/query`)라 v3에서 거절될 가능성이 높지만 문서에 명시는 없다.

시험 전 확인할 것(문서·실행으로):
- 컨테이너 기동: `influxdb3 serve --node-id … --object-store {memory|file}`, 헬스 확인 경로와 인증 필요 여부, 관리자 토큰 생성(`influxdb3 create token --admin`), 데이터베이스 생성(v2 `bucket` ↔ v3 database, `org` 무시 여부).
- 쓰기 의미: 정수 `i`·불리언·문자열 필드, 태그(`channel`·`device`), ns 정밀도, 같은 시각 중복 점의 병합, **타입 충돌이 섞인 배치의 부분 수락**(v2는 400/422 + 유효 행 유지 → `Unknown`).

## 결정

| ID | 질문 | 선택지 | 권장 |
|---|---|---|---|
| D1 | 버전 고정 | S0 시점 최신 안정 3.x Core 이미지를 digest로 고정 | 권장대로 |
| D2 | 저장소 | A. `--object-store memory` / B. `file` + tmpfs | **B** — v2 하네스와 같은 tmpfs 크기 상한·정리 방식, 디스크 쓰기 0 |
| D3 | Flux 조회가 안 될 때 | 지원 범위로 기록, SQL/InfluxQL 조회 추가는 기능 논의 안건(MS-012 M3) | 권장대로 |

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S0 | 고정·사전 확인 | D1·D2. 이미지 digest·버전, 기동·토큰·DB 생성 절차를 실제 컨테이너로 확인 | Planned |
| S1 | 하네스 | `check-influxdb.py --server v3`: 같은 격리 조건(루프백·tmpfs·CPU/메모리/pids 상한·무작위 토큰·redaction·cleanup). 결과 확인용 SQL 조회(`/api/v3/query_sql`)는 하네스(Python)에서 | Planned |
| S2 | 실행 | v3에서 live 4개 + `influxdb_write` 예제 실행, 시험별 결과 분류(Pass / v3 차이로 실패 / 결함). 쓰기 결과는 하네스 SQL 조회로 값·타입·ns·태그 대조. 차이가 제품 결함이면 원인 보고 뒤 처리 범위를 Human과 정함 | Planned |
| S3 | 문서 | 지원표(v2 2.9.1: 쓰기·조회 / v3 Core x.y: 쓰기 범위·조회 여부·차이)를 rustdoc·README에. `latest` 태그 주의. live 시험 v3 차이는 시험 이름·ignore 사유로 남김 | Planned |
| S4 | 마감 | EV, 자가검토, 기능 논의 안건 갱신(필요 시) | Planned |

## 완료 기준

- [ ] v3 Core 고정 버전에서 쓰기 호환 범위 확인·EV.
- [ ] 조회 지원 여부 확인·문서화, 필요 시 기능 논의 안건 등록.

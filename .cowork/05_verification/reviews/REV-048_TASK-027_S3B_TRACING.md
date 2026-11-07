# REV-048 — TASK-027 S3b tracing 선택 연결 개발 자가 점검

## 개요

- 일자/담당: 2026-11-07 #001 / AI 개발 세션 자가 점검. 독립 리뷰가 아니다.
- 대상: 소스 `91387ba` — `logger/sink.rs`(`TracingSink`·내부 `SinkState`·`EventRecord`/`LogRecord`), `mod.rs`(`LoggerBuilder::tracing`·`Logger::tracing`·shutdown 회수), `health.rs`(`tracing_dropped`), `adapters/tracing.rs`(level 변환 `pub(crate)`), TC-013 F12(`tests/tc_013_tracing.rs`).
- 상태: **Closed** — 결함 수정이 필요한 항목은 없다.
- 판정: **Pass — 로컬 기능 범위.** 근거: [EV-049](../verification_evidence.md#ev-049).

## 리뷰 요약

| ID | 중요도 | 위치·계약 | 문제 | 처리 | 상태 |
|---|---|---|---|---|---|
| RV-105 | P2 | sink lock 재진입 | 이벤트 formatting(사용자 `Debug`) 안에서 `log` 호출이 같은 sink로 들어오면 같은 thread가 `Mutex`를 다시 잡아 교착한다 | thread-local 재진입 표시로 중첩 이벤트를 버리고 센다. 가드를 빼는 돌연변이에서 F12가 교착(timeout)함을 확인 | 조치 |
| RV-106 | P2 | global sink와 종료 | `set_global_default`로 설치한 sink는 해제되지 않아 내부 writer가 반납되지 않으면 host reaper가 끝나지 않는다 | shutdown/Drop 시작 시 Logger가 writer를 회수. 회수를 빼는 돌연변이에서 F12의 `writers_outstanding == 0` 단언이 실패함을 확인 | 조치 |
| RV-107 | P3 | 다중 thread 직렬화 | 여러 일반 thread의 이벤트가 하나의 `Mutex<Writer>`에서 직렬화된다. formatting도 lock 안에서 실행된다 | 일반 경로 계약으로 문서화. 처리량이 문제면 thread별 writer 선언(`take_writer`)이 대안 | 문서화 |
| RV-108 | P3 | 유실 카운트 범위 | tracing이 같은 thread 중첩 dispatch를 자체로 막아 sink에 오지 않는 이벤트는 `tracing_dropped`에 없다 | §18.10에 명시 | 문서화 |
| RV-109 | P3 | 고정 schema 99/v1 | JSON text 채널 schema id가 기존 예제의 99/v1 고정값이다 | 기존 `general_logs` 예제와 일치. 표준 MCAP schema 매핑은 TASK-030 | 문서화 |

## [리뷰어] 점검

- **범위/권한:** API §18.1 S3 범위 안. 새 의존성·Cargo.lock 변경·unsafe 없음. feature `tracing-adapter` 없이는 코드·필드 노출이 없고(`Health::tracing_dropped`는 항상 존재하며 `None`), feature 16조합 두 toolchain 확인.
- **RT 경로:** `Writer::try_log` 코드는 그대로다. sink는 일반 경로이며 RT writer와 별도 producer slot을 쓴다. RT 기록·할당 계약에 영향 없음(rt-allocation 회귀 Pass).
- **유실 정직성:** 수락하지 못한 이벤트는 모두 세며 core `Skipped`(level/sampling)는 세지 않는다. 선언하지 않았으면 `None`이다.
- **시험 신뢰도:** F12 3개는 구현과 함께 작성했다. 돌연변이 2종(RV-105·106)으로 검출을 확인했고 TC-013 tracing·facade binary를 20회 반복해 모두 통과했다.

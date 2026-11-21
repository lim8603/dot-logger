# REV-047 — TASK-027 S3a health·계측·CPU 고정 개발 자가 점검

## 개요

- 일자/담당: 2026-11-07 #001 / AI 개발 세션 자가 점검. 독립 리뷰가 아니다.
- 대상: 소스 `691f937` — `logger/health.rs`(`Health`·`HealthState`·`Degradation`·`Failure`·`OutputHealth`·내부 `Shared`/`OutputStatus`), `logger/affinity.rs`, `budget.rs`(`Budget::worker_cpus`·platform 관측), `host.rs`(출력별 상태 기록·telemetry 표본·thread 고정·command 수신단 해제), `mod.rs`(`LoggerBuilder::telemetry`·`Logger::health`), `logger/tests.rs`(GAP-009), TC-013 F9~F11, `adapters/tracing.rs` lint(GAP-010).
- 상태: **Closed** — 결함 수정이 필요한 항목은 없다. P3은 문서화 또는 S3 후속으로 처리했다.
- 판정: **Pass — 로컬 기능 범위.** 근거: [EV-048](../verification_evidence.md#ev-048).

## 리뷰 요약

| ID | 중요도 | 위치·계약 | 문제 | 처리 | 상태 |
|---|---|---|---|---|---|
| RV-100 | P2 | host 사망 뒤 `shutdown` | host가 panic으로 끝나도 command 수신단이 reaper 대기 동안 살아 있어, `shutdown`이 회신 없이 deadline + 5초를 기다린 뒤 `Unresponsive`를 반환했다 | reaper 진입 전 수신단을 놓아 즉시 `WorkerFailed`. 돌연변이(해제 제거)로 GAP-009 host 시험이 실패함을 확인 | 조치 |
| RV-101 | P3 | 시작 중 host panic | `start()` 진행 중 panic이 나면 `run`의 namespace 정리 경로를 거치지 않아 생성된 빈 namespace가 남을 수 있다(오류 반환 경로는 정리) | 기존 S2와 같은 범위. 시작 시 panic 원인은 내부 결함뿐이라 문서화만 | 문서화 |
| RV-102 | P3 | `Health` 목록 할당 | `health()`는 출력 수만큼 `Vec`을 할당하고 telemetry Mutex를 잠깐 잡는다 | 일반 경로 API로 rustdoc 명시. RT thread에서 부르지 않는다 | 문서화 |
| RV-103 | P3 | 출력 실패 판정 중복 경로 | sync 실패 직후(작업 소진)와 다음 barrier(`Poisoned` 작업 반환) 두 경로가 모두 `failed`를 세운다. 하나만 끄면 F9가 잡지 못한다 | 두 경로를 모두 끄는 돌연변이로 F9 검출 확인. 의도한 중복(어느 경로든 관측) | 확인 |
| RV-104 | P3 | Linux 외 CPU 고정 | 다른 OS 대상이 설치돼 있지 않아 `cfg(not(target_os = "linux"))` 경로는 컴파일하지 않았다 | 코드 검토만. native(T3)·공개 후 확인 | 미실행 |

## [리뷰어] 점검

- **범위/권한:** API §18.4·§18.5·§18.1 S3 범위 안. 새 의존성·Cargo.lock 변경·unsafe 없음. rustix 기존 의존성에 `thread` feature만 추가했다.
- **RT/할당:** `Writer`·`try_log` 코드는 바뀌지 않았다. telemetry를 켜면 기존 core 계약대로 RT 기록이 producer 로컬 atomic을 갱신한다(비용은 TASK-023 S3 계측 비용 행). rt-allocation F2/F3 회귀 Pass.
- **상태 의미:** 측정하지 않은 값은 `None`/`Unavailable`이며 0으로 쓰지 않는다. `IoPoisoned`는 required 출력 계약에서 이후 성공이 불가능함을 뜻하므로 Degraded가 아니라 Failed로 분류했다.
- **시작 원자성:** CPU 고정 결과를 IO thread까지 받은 뒤 ready를 보낸다. Required 실패는 생성한 namespace를 지우고 `start()`가 오류를 반환한다(F11 미허용 CPU 시험은 검증 단계 거절, 적용 실패 자체는 재현 수단 없음).
- **시험 신뢰도:** F9/F10/F11·GAP-009 시험은 구현과 함께 작성했다. 돌연변이 2종(command 해제 제거, 출력 실패 표시 전부 제거)으로 해당 시험이 실패함을 확인했다. TC-013 두 binary와 logger unit을 20회 반복해 모두 통과했다. F11은 같은 프로세스의 다른 시험 worker thread가 섞이지 않도록 별도 binary로 분리했다.

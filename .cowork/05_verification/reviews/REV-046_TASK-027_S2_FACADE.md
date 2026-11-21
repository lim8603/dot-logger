# REV-046 — TASK-027 S2 facade·기본 worker 개발 자가 점검

## 개요

- 일자/담당: 2026-11-05 #001 / AI 개발 세션 자가 점검. 독립 리뷰가 아니다.
- 대상: `capture` 내부 분해(`CaptureCore`·endpoint, 소스 `04a7cfa`), `logger` 모듈(`Logger`·`LoggerBuilder`·`Budget`·`Retention`·`WriterQuota`·`Writer`·`Channel<T>`·`Record`·`Schema`·host/IO worker·`FacadeReport`, 소스 `1e651b4`), TC-013 F1~F8 시험·rt-allocation F2/F3.
- 상태: **Closed** — 결함 수정이 필요한 항목은 없다. 시험 공백 1건은 GAP-009, 기존 MSRV lint는 GAP-010으로 분리했다. 나머지 P3은 API §18.8 문서화와 S3 범위로 처리했다.
- 판정: **Pass — 로컬 기능 범위.** 근거: [EV-047](../verification_evidence.md#ev-047)·결과(비공개 원시 근거).

## 리뷰 요약

| ID | 중요도 | 위치·계약 | 문제 | 처리 | 상태 |
|---|---|---|---|---|---|
| RV-094 | P2 | TC-013 F5 worker panic | IO/host panic 처리(catch_unwind, Failed 배출, closing)는 구현했지만, panic을 일으킬 시험 주입점이 없어 자동 시험이 없다 | 공개 API에 숨은 hook을 넣지 않는다. S3에서 crate 내부 시험 또는 feature-gated 주입점을 검토 | GAP-009 |
| RV-095 | P3 | 주기 sync 결과 | 주기 barrier의 실패·만료가 shutdown 보고 전까지 사용자에게 보이지 않는다 | S3 `health()`에서 Degraded로 노출. completion 계약은 Written이라 기록 수는 정확 | S3 |
| RV-096 | P3 | API §18.4 CPU affinity | S2 `Budget`에는 CPU 목록 설정이 없고 thread 고정을 하지 않는다 | §18.8에 S3 이후로 명시. ADR-012 §6의 성능 값 기본값(고정 없음)과 일치 | 문서화 |
| RV-097 | P3 | `budget::derive` 메모리 분할 | raw·input·worker stack은 정확히 계산하고, 나머지는 control 1/4·output 3/4의 고정 비율로 나눈다. 총량이 충분해도 비율 때문에 한 쪽이 모자랄 수 있다 | 부족하면 해당 partition과 부족량을 `StartError::Budget`으로 반환(F6). TASK-028 Sizing에서 정밀화 | 문서화 |
| RV-098 | P3 | macOS 등 비 Linux Unix | 여유 공간 관측이 Linux statvfs 전용이라 비 Linux에서는 `start()`가 Storage 오류로 거절된다 | Tier 2 capability 한계로 §18.8에 명시. 조용한 성공은 없다 | 문서화 |
| RV-099 | P3 | API §18.3 `LogError` | 초안의 `EmergencyPowerOff`는 core `CaptureError`에 없다(긴급 종료는 Closing으로 보인다). 대신 `InvalidInput`·`Exhausted`가 필요했다 | §18.8에 실제 variant를 기록 | 문서화 |

## [리뷰어] 점검

- **범위/권한:** ADR-012·API §18 승인 범위 안의 구현이다. 새 의존성·Cargo.lock 변경·unsafe는 없다. 공개 소유권 계약은 ADR-012 그대로이고, core의 공개 시그니처는 바뀌지 않았다.
- **RT/할당:** `try_log`·`try_log_at`·`producer()` 뷰는 참조 복사와 기존 `try_record_small`만 수행한다. Capacity 거절·raw loan·shutdown 뒤 Closing까지 allocator 호출 (0,0)을 F2로 확인했다. 시계 읽기(`Instant`·`SystemTime`)는 RT 비용 항목이며 TASK-023에서 잰다.
- **Drop 불변식:** `Writer::drop`은 1-slot 반납함으로 이동만 하고, host가 반납 확인 전까지 `Arc<CaptureCore>`·read endpoint·반납함 수신단을 보유한다. 세 순서(동작 중·shutdown 뒤·Logger Drop 뒤) 모두 (0,0)을 F3로 확인했다.
- **완료 의미:** file target 완료 경계는 Written이다. 내구성은 주기 barrier와 shutdown 전용 sync로 확인하고, `is_success()`가 sync 실패·기한 초과·IO 실패를 모두 거절한다. 기한 초과는 F4로 Incomplete → Closed(실패 판정)를 확인했다. 강제 해제는 없다.
- **IO 실패:** 쓰기 일괄 실패는 기존 prefix 계약(`appended`/`written`/`reported`)대로 처리한다. 이미 append한 prefix를 다시 쓰지 않는다. 디렉터리 소실(F5)은 `io_failed`이며 성공이 아니다.
- **시험 신뢰도:** F4·F5·F7은 구현 뒤에 작성했으므로, `Logger::drop` 종료 처리와 `io_failed` 표시를 일부러 끄는 돌연변이로 F5·F7이 실패함을 확인했다. F1은 30회 반복 모두 통과했다.
- **한계:** CPU 고정·`health()`·slot 직접 encode·tracing/remote 연결은 S3, 성능은 TASK-023 S2/S3, native는 T3다.

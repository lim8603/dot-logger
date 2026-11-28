# REV-080 — MS-013 r7 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-28 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r7](../../04_implementation/milestones/MS-013.md), [REV-080 요청서](REV-080_REQUEST.md), [REV-079](REV-079_MS013_R6_DESIGN_REVIEW.md) |
| 기준 | r7 및 검토 시 HEAD `405cf39607273cd5f8d85d77e4ab0775020c1be8` |
| 판정 / 상태 | **Conditional / Open** |

## 1. 요약정리

**P1 3건, P2 1건.** r7의 `offer_pending`과 기록별 route revision 규칙은 이전 지적의 반례를 해소한다. 스위치의 host 종료 증거, 파일 발행 완료와 host 확인 사이의 기한 판정, 감독 정산 뒤 최종 종료 경로, End 이후 ACK API의 계약이 남았다. r6→r7 변경과 현행 Rust 소스를 정적으로 대조했다. 제품 시험과 성능 측정은 요청 범위 밖이다.

## 2. 이전 지적 대조

| 이전 지적 | r7 대조 결과 |
|---|---|
| RV-256 · 명령 결과 | 최초 반환에 `WorkerFailed`와 `HostFailed { id, epoch }`가 생겼다. 실제 host 종료 뒤에는 직렬 게시 epoch로 이력 퇴역 전후의 게시 완료를 판별할 수 있다. host 종료를 증명하는 신호는 RV-261. |
| RV-257 · Health 보존식 | 해소. `Unsettled` 생성과 해소를 `offer_pending`에 건수·바이트로 반영한다. 항목 시작 시 예시의 `targeted = filtered + offer_pending`은 그 항목의 증가분이며, 누적식은 표의 네 항을 따른다. 정상·감독 종결의 순간값은 0이다. |
| RV-258 · 종료 반환 | 기한 + `REPORT_GRACE`, 늦은 기한 보고, 후속 `wait(timeout)`의 반환이 명시됐다. 기한 보고를 작성할 때 완료 이벤트를 아직 관측하지 못한 경우와 감독 정산 이후 최종 보고는 RV-262·263. |
| RV-259 · End와 ACK 우선순위 | End 선착과 비-Gone 실패 선착의 시험을 분리했고 미전송·응답 대기·실패 ACK의 End 시점 수를 정의했다. `next()` 밖에서 ACK 실패를 먼저 관측하는 경우와 End 이후의 공개 메서드는 RV-264. |
| RV-260 · 혼합 구성의 빈 route | 해소. 시작 검증과 기록의 route revision을 분리했고, 다른 채널에 observer가 있어도 `targets=[]`·`observe=None`인 채널은 기존 `Unrouted`·`NoRoute`·raw 회수를 유지한다. |

REV-074~078의 원본 guard 미전달, 같은 분배 안 복사 예산과 드롭, completion·관리 wire 분리, legacy tap, 출력 교체 시 숫자 index 거절, DB 단계 6과 H6 조기 판정은 유지된다.

## 3. 리뷰 포인트

| ID | 중요도 | 위치 | 계약 공백 | 처리 제안 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-261 | **P1** | §5.6 | 파일 IO 실패를 host 종료로 해석해 살아 있는 host의 명령을 종결할 수 있다 | host가 더는 게시할 수 없음을 증명하는 종료 신호를 지정한다 | — | Open |
| RV-262 | **P1** | §5.9 | 기한 내 발행됐지만 host가 완료 이벤트를 읽기 전이면 지연 발행으로 확정할 수 있다 | 발행 상태·host 관측·보고 시각을 구분한다 | — | Open |
| RV-263 | **P1** | §5.4·§5.9 | 감독 정산 뒤 정상 종료 이벤트를 낼 worker가 없으면 최종 `Closed` 근거가 비어 있다 | 정상 완료, 실패 후 회수, 분리 시 종료 조건을 명시한다 | — | Open |
| RV-264 | P2 | §5.6·§6 | ACK 실패를 `flush()`나 `handoff()`에서 먼저 관측할 때와 End 이후 ACK API의 동작이 빠져 있다 | 구독 종료 상태를 공개 메서드 전체에 적용한다 | — | Open |

### RV-261 — 명령을 종결하려면 host의 게시 권한이 끝났음이 확인돼야 한다

- **[리뷰어]** §5.6은 `worker_failed`를 10 ms마다 확인해 host 종료로 처리한다. 현재 이 플래그는 파일 IO 스레드가 panic한 경우에도 설정된다. 이때 host는 입구를 닫고 실패를 정산하며 계속 살아 있다. 예를 들어 host가 스위치 `Committed` 상태에서 선점되고 IO 스레드가 이 플래그를 세우면, 호출자는 epoch 게시 전이라 `HostFailed`를 반환할 수 있다. 이후 host가 재개하면 표시와 epoch를 게시할 수 있어 종결 결과와 관측된 효과가 어긋난다. [명령 종료 규칙](../../04_implementation/milestones/MS-013.md#L320), [IO 실패 플래그](../../../crates/dot-logger/src/logger/host.rs#L818), [살아 있는 host의 실패 처리](../../../crates/dot-logger/src/logger/host.rs#L538)
- **처리 제안** — 명령 종결에 사용할 host 전용 종료 증거를 명시한다. 파일 실패 시 살아 있는 host가 처리하는 거절·종료 진입과, 실제 host 종료 시 외부에서 하는 셀 종결을 각각 시험한다.
- **[처리담당]** 대기.

### RV-262 — 아직 읽지 못한 완료 이벤트로 발행 기한을 확정할 수 없다

- **[리뷰어]** IO 스레드가 기한 D 전에 `finish`를 완료하고 `at ≤ D`인 이벤트를 보냈으나, host가 D 뒤 첫 턴에서 그 이벤트를 반영하기 전에 기한 보고를 만든 경우다. 표에 따르면 host가 아는 상태는 core Closed와 파일 완료 미확인이어서 `files_late = true`로 확정될 수 있다. 후속 최종 보고에서는 동일한 발행이 기한 내 성공으로 바뀐다. 이벤트가 발행 완료 뒤 늦게 게시되는 경우도 수신 순서를 정하는 것만으로는 해소되지 않는다. [내부 완료 이벤트](../../04_implementation/milestones/MS-013.md#L392), [기한 보고 파일 판정](../../04_implementation/milestones/MS-013.md#L397)
- **처리 제안** — 기한 보고에서 발행 완료 시각을 읽을 수 있는 경계를 정한다. 아직 결과를 알 수 있는 근거가 없다면 미확인 상태와 확정 지연을 구분한다. 발행 직후 게시 전 정지 및 이벤트 게시 직후 host 확인 지연을 시험한다.
- **[처리담당]** 대기.

### RV-263 — 감독 정산과 최종 종료의 조건을 연결해야 한다

- **[리뷰어]** 네트워크 스레드가 실패해 정상 경로의 `NetworkFinalized`를 보낼 수 없다고 하자. host는 봉인·위치 정산을 완료할 수 있고 파일도 성공할 수 있다. §5.4는 이때 네트워크 집계가 `finalized = true`일 수 있다고 정하지만, §5.9는 내부 이벤트 대기와 「파일·네트워크 모두 끝난 뒤」만 명시한다. 실패한 worker가 회수돼도 어떤 증거로 정상 완료 이벤트 없이 `Closed`에 도달하는지, 회수가 장기간 끝나지 않을 때 호출자가 받는 상태는 무엇인지 정해져 있지 않다. [집계 확정 조건](../../04_implementation/milestones/MS-013.md#L243), [내부 이벤트와 최종 보고](../../04_implementation/milestones/MS-013.md#L392)
- **처리 제안** — 계수 확정과 리소스 회수의 완료 조건을 정하고, 실패·강제 종료에서 누가 완료를 통지하는지 명시한다. `NetworkFinalized` 없이 worker를 회수한 경우와 계수 확정 뒤에도 worker가 살아 있는 경우의 `wait(timeout)`을 시험한다.
- **[처리담당]** 대기.

### RV-264 — End 우선 정책과 최종 ACK 수를 공개 메서드 전체에 적용해야 한다

- **[리뷰어]** 현재 client는 `next()`뿐 아니라 `handoff()`와 `flush()`에서도 ACK 실패를 읽어 `unknown()`으로 스트림을 닫는다. 예를 들어 마지막 `handoff()` 뒤 `flush()`가 `Gone`을 먼저 받으면, 스트림에 End가 도착해도 이후 `next()`로 볼 수 없다. r7의 표는 `next()`에서 먼저 관측하는 경우만 다룬다. 반대로 End를 먼저 받았을 때는 늦은 RPC 성공을 반영하지 않는다고 했지만, 현행 `acknowledged()`는 백그라운드 RPC 성공이 올리는 누적값을 읽고 `flush()`는 미확인 handoff가 있으면 오류를 돌려준다. End의 `ack_unknown`을 확정한 뒤 이 두 메서드가 어떤 상태를 보고하는지 계약이 필요하다. [End 우선순위](../../04_implementation/milestones/MS-013.md#L357), [ACK 성공의 공개 계수와 flush](../../../crates/dot-logger-client/src/transport.rs#L665), [handoff의 ACK 확인](../../../crates/dot-logger-client/src/transport.rs#L874)
- **처리 제안** — 먼저 `Gone`을 관측한 메서드가 `flush()` 또는 `handoff()`인 경우의 반환과 후속 End 관측 가능성을 정한다. End 뒤 `next()`·`flush()`·`acknowledged()`가 공유하는 종료 상태를 정의하고, 늦은 ACK 응답 경합과 함께 시험한다.
- **[처리담당]** 대기.

## 4. 다음 회차

개발 세션에서 RV-261~264의 처리 결과를 기록한다. 개정 뒤 독립 재리뷰에서 이전 지적 해소와 새 지적 0건을 확인한 뒤 Human 승인으로 넘긴다.

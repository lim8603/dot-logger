# [REV-006] TASK-006 제미나이 관점 심층 코드리뷰

## 1. 리뷰 개요
* **일자 / 리뷰어:** 2026-10-28 / Gemini (심층 코드리뷰)
* **대상 Task / 기준 커밋:** [TASK-006](../04_implementation/tasks/TASK-006.md) / commit `d57c2e3` (RV-001~004 수정분, [EV-007](../05_verification/verification_evidence.md#ev-007))
* **종합 판정:** Note/Suggestion (P2 권고 2건, P3 제안 3건)
* **진행 상태:** Closed (EV-008 검토 및 rustdoc 보강 완료, 장비 측정 과제는 TASK-022로 연결)

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 점검 내용 요약 | 판정 결과 | 상태 |
|---|---|---|---|---|---|
| RV-D6-01 | Info | `completion/mod.rs:572`<br>동시성/원자성 | RV-002 Runway 원자적 로드(`Ordering::Acquire`)의 보수적 안전성 대조 | 통과 (Pass) | Closed |
| RV-D6-02 | P2 (제안) | `completion/types.rs`<br>캐시라인 정렬 | `RecordSlot`(40B) 인접 배치로 인한 False Sharing 잠재성 (`#[repr(align(64))]` 검토 권고) | 보류 (TASK-022) | Closed |
| RV-D6-03 | P2 (제안) | `completion/mod.rs`<br>RT 경로 지터 | RT 수집 경로 내 채널 라우팅 선형 탐색($O(N)$) → Direct Indexing/LUT 전환 권고 | 보류 (측정 후) | Closed |
| RV-D6-04 | P3 (문서) | `control/mod.rs`<br>상태 머신 수렴 | `poll_shutdown`의 2-Phase 수렴 루프 계약 외부 API/rustdoc 명시 권고 | 수용 (Accept) | Closed |
| RV-D6-05 | P3 (진단) | `capture/mod.rs`<br>에러 매핑 | `CompletionCapacity` 단일 에러를 설정 오류(`InvalidConfig`)와 용량 부족(`Busy`)으로 세분화 권고 | 보류 (계약 유지) | Closed |

---

## 3. 상세 리뷰 및 처리 내역

### RV-D6-01: RV-002 Important 활주로(Runway) 검사의 보수적 안전성 확인

#### [리뷰어]
* **분석:** `Completion::reserve`에서 Normal 레코드 수락 시 현재 인덱스부터 `important_reserve`개 연속 슬롯의 `EMPTY | TERMINAL` 상태를 원자적 load(`Ordering::Acquire`)로 검사.
* **평가:** `Producer` 인스턴스는 Rust 소유권 모델상 생산자별로 1개씩 격리되어 동시 `reserve` 충돌이 없음. 백그라운드 소비자가 슬롯을 `TERMINAL`로 바꾸는 시점과의 race가 있더라도 보수적으로 거절(`Busy`)할 뿐, 미완료 슬롯을 빈 슬롯으로 착각해 Important 슬롯을 침범하는 버그는 구조적으로 차단됨. (보수적 안전성 Pass)

#### [처리담당 (2026-10-29 #001 / EV-008)]
* **판정 결과:** 수용 (Pass 확인)
* **검증 근거:** 정식 반례 테스트 및 Miri 통과로 보수적 안전성 재확인.

---

### RV-D6-02: RecordSlot 인접 배치와 False Sharing 잠재성

#### [리뷰어]
* **분석:** `RecordSlot` 구조체는 약 40바이트로, 64바이트 캐시 라인 경계에서 인접한 두 슬롯이 하나의 캐시 라인을 공유할 수 있음.
* **피드백:** 생산자 스레드가 다음 슬롯의 `phase`를 CAS하는 동안 워커 스레드가 인접 슬롯의 `terminal_at`을 업데이트하면 L1/L2 캐시 라인 핑퐁이 발생할 수 있음. 향후 TASK-022(실제 장비 벤치마크) 진행 시 `#[repr(align(64))]` 적용 전후의 p99 지연 시간 비교를 권고함.

#### [처리담당 (2026-10-29 #001 / EV-008)]
* **판정 결과:** 보류 (TASK-022 실장비 측정 시 비교)
* **처리 내용:** 메모리 공간 낭비와 캐시 핑퐁 간의 trade-off이므로, 실장비 하드웨어(CPU/캐시 구조) 확정 후 TASK-022에서 벤치마크 측정 후 적용 여부 결정.

---

### RV-D6-03: RT 수집 경로 내 채널 라우팅 선형 탐색($O(N)$)

#### [리뷰어]
* **분석:** `completion/mod.rs`의 `reserve`에서 `routes.iter().find(|r| r.channel_id == channel)`로 매 수집마다 선형 탐색($O(N)$) 수행.
* **피드백:** 채널 수가 작을 때는 L1 캐시로 빠르지만, 최대 사양(128채널)으로 확장될 경우 지터 요인이 될 수 있음. 채널 ID 순차 할당 시 Direct Indexing 또는 고정 크기 LUT 전환 검토 권고.

#### [처리담당 (2026-10-29 #001 / EV-008)]
* **판정 결과:** 보류 (실제 부하 측정 후 판단)
* **처리 내용:** 128채널 캐시 히트율 및 실제 RT p99 측정 후 필요 시 LUT 전환 추진.

---

### RV-D6-04: poll_shutdown의 2-Phase 수렴 루프 계약 문서화

#### [리뷰어]
* **분석:** `settle_expired_shutdown` 도입으로 만료된 셧다운 동기화는 `APPLY_EXPIRED` 상태로 전환된 후, 다음 `advance_sync` 호출에서 최종 마감됨.
* **피드백:** 잔여 I/O 누락을 방지하는 우수한 2단계 상태 머신임. 단, 호출자가 `poll_shutdown()`을 1회만 호출하고 멈추지 않고 `true`가 반환될 때까지 폴링 루프를 돌아야 완전히 수렴한다는 계약을 rustdoc에 명시 권고.

#### [처리담당 (2026-10-29 #001)]
* **판정 결과:** 수용 (Accept)
* **처리 내용:** `poll_shutdown` rustdoc에 완전 종료를 위해 `true` 반환까지 반복 구동해야 한다는 계약 문서 보강 완료.

---

### RV-D6-05: 에러 매핑 세분화 (InvalidConfig vs Busy)

#### [리뷰어]
* **분석:** `capture/mod.rs`에서 `reserve` 실패 시 `InvalidConfig`(채널 미등록)와 `Busy`(슬롯 부족)가 모두 `CaptureError::CompletionCapacity` 단일 에러로 변환됨.
* **피드백:** 일시적 용량 부족과 설정 오류를 구분하여 진단할 수 있도록 향후 에러 enum 세분화 권고.

#### [처리담당 (2026-10-29 #001 / EV-008)]
* **판정 결과:** 기각/보류 (Maintain Contract)
* **처리 내용:** 미등록 채널은 ingress 진입 전 `try_record` 단계에서 이미 `InvalidInput`으로 차단되므로, completion 내부의 이중 검사는 방어적 폴백임. 불필요한 공개 enum 변경을 지양하고 현재 계약 유지.

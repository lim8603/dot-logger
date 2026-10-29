# [REV-003] TASK-007 MCAP 봉투·writer·인덱스 코드리뷰

## 1. 리뷰 개요
* **일자 / 리뷰어:** 2026-10-29 / AI ([code-review skill](../../../.agents/skills/code-review/SKILL.md))
* **대상 Task / 기준 커밋:** [TASK-007](../04_implementation/tasks/TASK-007.md) / commit `d57c2e3` + 작업트리 (EV-010 기준 소스 46개)
* **종합 판정:** Conditional (P1 0건, P2 3건, P3 2건)
* **진행 상태:** Resolved (EV-011 로컬 수정 및 회귀 Pass, commit `71b3e2c`, 최신 구현 Human 승인 대기)

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-006 | P2 (J) | `storage/writer.rs:449`<br>`storage/writer.rs:124` | `write_delivery`가 레코드마다 chunk를 마감하여 chunk 헤더/인덱스 오버헤드 급증 | 배치 경계 API 도입 및 인덱스 예약 상한 축소 | 수용 (Accept) | Resolved |
| RV-007 | P2 | `storage/writer.rs:471`<br>`control/mod.rs` | `sync_work`가 IO 전 검증 실패(target 불일치/Busy) 시 `SyncWork`를 소비해 Unknown 확정 | IO 전 검증 실패 시 `SyncWork` 소유권 반환 (`Result<(), (SyncWork, E)>`) | 수용 (Accept) | Resolved |
| RV-008 | P3 | `storage/writer.rs:200` | `FinishedSegment::sync_work`와 `SegmentWriter` 간의 오류 매핑 불일치 | 공통 private 검증 헬퍼로 오류 매핑 일원화 | 수용 (Accept) | Resolved |
| RV-009 | P2 | `storage/writer.rs:24`<br>`Sink` 구조체 | 비버퍼 Direct sink로 레코드당 다수의 소형 write syscall 발생 추정 | 예산 내 고정 용량 쓰기 버퍼 추가 (seek/flush 시 자동 배출) | 수용 (Accept) | Resolved |
| RV-010 | P3 | `storage/writer.rs:447` | `write_delivery` Busy 후 재시도 방법(중복 쓰기 방지) 안내 불명확 | rustdoc에 report-only 재시도 경로 명시 및 문서 보강 | 수용 (Accept) | Resolved |

---

## 3. 상세 리뷰 및 처리 내역

### RV-006: 레코드 단위 chunk 마감으로 인한 메타데이터 오버헤드 및 예약 낭비

#### [리뷰어]
* **발견한 문제:**
  1. `append` 직후 `flush_chunk`를 호출하여 레코드 1개당 chunk 1개가 생성됨.
  2. payload 64B 소형 레코드마다 chunk 헤더 및 인덱스가 추가되어 파일 크기가 2배 이상 팽창하고 메모리 할당이 빈번해짐.
  3. `max_chunks × channels × 512B` 예약식으로 인해 대규모 채널/레코드 환경에서 수 GB의 메모리가 예약됨.
* **처리 제안:**
  - `write_batch` 배치 단위 쓰기 인터페이스를 도입하여 chunk target 바이트 범위 내에서 여러 delivery를 묶어 flush.
  - 인덱스 예약을 $\min(\text{records}, \text{chunks} \times \text{channels})$로 현실화.
* **심각도:** P2 (Warning / Human 판단 J 필요)

#### [처리담당 (개발 세션 — 2026-10-29 #003)]
* **판정 결과:** 수용 (Accept — Human 승인)
* **처리 내용:**
  - `write_batch(&mut self, &[Delivery])` 구현: chunk target 크기까지 레코드를 묶고, flush 완료 후 일괄 Written 보고.
  - `BatchWriteError`에 `appended_count`, `written_count`, `reported_count` prefix를 제공하여 부분 실패 시 중복 쓰기를 방지하고 미보고분만 재시도 가능하게 개선.
  - 인덱스 예약 공식을 $\min(\text{records}, \text{chunks} \times \text{channels})$로 조정하여 output 버퍼 낭비 방지.
* **검증 근거:** [EV-011](../05_verification/verification_evidence.md#ev-011), `write_batch_chunks_by_target_bytes` 단위 테스트 통과, commit `71b3e2c`.

---

### RV-007: IO 전 사전 검증 실패 시 SyncWork 유실 및 복구 불가 Unknown 확정

#### [리뷰어]
* **발견한 문제:**
  - `SegmentWriter::sync_work`가 target/session 불일치나 `covers_file` 실패 시 `SyncWork`를 값으로 소비하고 Drop시킴.
  - 이로 인해 호출자의 단순 라우팅 실수나 일시적 `Busy` 상황에서도 barrier가 영구적인 `ACK_UNKNOWN`으로 확정되어 재시도 불가.
* **처리 제안:**
  - IO 실행 전 사전 검증 단계에서 실패할 경우 `SyncWork`를 호출자에게 다시 반환 (`Result<(), (SyncWork, StorageError)>`).
* **심각도:** P2 (Warning / 계약 건전성)

#### [처리담당 (개발 세션 — 2026-10-29 #003)]
* **판정 결과:** 수용 (Accept)
* **처리 내용:**
  - pre-IO 검증 실패 시 `Err((SyncWork, StorageError))` 형태로 소유권을 보존하여 반환하도록 시그니처 변경.
  - 호출자가 올바른 writer에 재전달하거나 `Busy` 해소 후 재시도할 수 있도록 계약 보장.
* **검증 근거:** [EV-011](../05_verification/verification_evidence.md#ev-011), `sync_sink_reject_returns_work` 단위 테스트 통과, commit `71b3e2c`.

---

### RV-008: FinishedSegment와 SegmentWriter 간의 sync_work 오류 매핑 불일치

#### [리뷰어]
* **발견한 문제:** `SegmentWriter`는 `ControlError::Busy`를 `StorageError::Busy`로 반환하는 반면, `FinishedSegment`는 모든 오류를 `InvalidRecord`로 일괄 매핑하여 일관성 결여.
* **처리 제안:** 공통 검증 헬퍼를 작성하여 두 경로의 에러 매핑을 통일.
* **심각도:** P3 (Suggestion / 일관성)

#### [처리담당 (개발 세션 — 2026-10-29 #003)]
* **판정 결과:** 수용 (Accept)
* **처리 내용:** 공통 사전 검증 로직을 단일화하여 두 경로 모두 `Busy` 및 `Control` 에러를 동일한 의미의 `StorageError`로 일관되게 매핑.
* **검증 근거:** [EV-011](../05_verification/verification_evidence.md#ev-011), commit `71b3e2c`.

---

### RV-009: 비버퍼 Direct sink로 인한 다수의 소형 write syscall 발생 추정

#### [리뷰어]
* **발견한 문제:** `Sink`가 `std::fs::File`에 직접 쓰기를 수행하여 레코드 인코딩 시 수많은 소형 syscall이 발생, CPU 낭비 및 지연 증가 우려.
* **처리 제안:** 예산 범위 내에서 동작하는 고정 크기 내부 쓰기 버퍼를 두고, `seek`이나 `flush` 시 자동으로 배출하도록 구성.
* **심각도:** P2 (Warning / 성능)

#### [처리담당 (개발 세션 — 2026-10-29 #003)]
* **판정 결과:** 수용 (Accept)
* **처리 내용:**
  - `write_buffer_bytes` 설정(0=direct)을 지원하고, 공유 output 예산에 버퍼 용량을 산입.
  - `seek` 및 `flush` 진입 시 내부 버퍼를 완전히 drain하여 safe offset 정합성 유지. Drop 시에는 IO를 수행하지 않고 안전 폐기.
* **검증 근거:** [EV-011](../05_verification/verification_evidence.md#ev-011), 버퍼링 단위 테스트 통과, commit `71b3e2c`.

---

### RV-010: write_delivery Busy 반환 후 재시도 방법 안내 불명확

#### [리뷰어]
* **발견한 문제:** 파일 쓰기 후 confirmation 보고 중 Busy가 발생했을 때 같은 delivery로 다시 쓰기를 호출하면 시퀀스 중복(`InvalidRecord`) 오류가 발생함.
* **처리 제안:** rustdoc에 "파일 쓰기는 완료되었으므로 `Delivery::report(Update::Confirmed(Boundary::Written))`만 재호출한다"는 점을 명시.
* **심각도:** P3 (Suggestion / 문서)

#### [처리담당 (개발 세션 — 2026-10-29 #003)]
* **판정 결과:** 수용 (Accept)
* **처리 내용:** rustdoc 및 배치 오류 처리 문서에 report-only 재시도 경로 및 prefix 반환(`written_count`) 활용법 명시.
* **검증 근거:** [EV-011](../05_verification/verification_evidence.md#ev-011), commit `71b3e2c`.

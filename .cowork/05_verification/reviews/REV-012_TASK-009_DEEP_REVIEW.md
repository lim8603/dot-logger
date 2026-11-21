# [REV-012] TASK-009 중단 복구·IO 장애 심층 코드리뷰 (Deep Code Review)

## 1. 개요

- **일자/리뷰어:** 2026-10-30 / AI ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md))
- **대상:** TASK-009 복구·IO 장애 구현 커밋 `5bbb45c` (`crates/dot-logger/src/storage/recovery.rs`, `writer.rs`, `mod.rs`, `tests/fault_storage.rs`, `tests/rt-allocation/src/tests.rs`)
- **종합 판정:** Pass (권고사항 P3 2건)
  - RT 경로 오염 없음, Safe Rust 100% (unsafe 0건), 철저한 산술/경계 바운드 검사, 메모리 누수 0, TDD 결함 주입 19개 및 할당 상한 검증 Pass.
- **상태:** Closed — 2026-10-30 Human 「승인, 커밋 푸쉬하고 다음 작업 진행하자.」로 TASK-009 최종 구현 승인. native/장비 GAP은 별도 유지.

---

## 2. 리뷰 요약표

| ID | 중요도 | 위치 / 대상 | 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-016 | P3 | `recovery.rs:321` (`recover_inner`) | 중첩 메시지 디코딩 시 `&record[22..]` 고정 인덱싱 슬라이스 사용 | 상위 `state.chunk()`의 사전 검증 불변식에 의존하므로, 방어적 코딩 관점에서 `record.get(22..)` 패턴 매칭으로 잠재적 패닉 경로 원천 차단 권고 | 수용·방어적 슬라이스 조회 반영(EV-015) | Resolved |
| RV-017 | P3 | `recovery.rs:470, 515, 518` (`Validation`) | 스키마 및 채널 검증 루프 내 반복적 `format!` 힙 동적 할당 | 복구(Management) 경로이므로 허용되나, 채널 수 증가 시 할당 오버헤드 완화를 위해 고정 스택 버퍼 포맷팅 또는 사전 계산 캐싱 제안 | 수용·64바이트 스택 포맷/직접 비교 반영(EV-015) | Resolved |

---

## 3. 상세 내역

### RV-016: 중첩 메시지 디코딩 시 방어적 슬라이싱 적용 권고 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/storage/recovery.rs#L320-L323`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/storage/recovery.rs#L320-L323)
- **발견점:**
  ```rust
  if opcode == 5 {
      let stored = decode_record(&record[22..], options.writer.max_payload_bytes)?;
      writer.append(stored.record, &stored.payload)?;
  }
  ```
  현재 구현에서는 청크 전체를 선행 검증하는 `state.chunk(body)`에서 opcode 5 레코드의 헤더(22바이트) 및 필드를 이미 완벽히 검증했기 때문에 런타임에 슬라이스 인덱스 패닉이 발생하지 않습니다.
  그러나 장기적 유지보수 관점에서 상위 사전 검증 불변식과 재순회(replay) 로직 간의 결합도가 존재하므로, `record.get(22..).ok_or_else(...)` 형태의 방어적 슬라이싱을 적용하면 예기치 못한 패닉 가능성을 원천 배제할 수 있습니다.
- **처리 제안:**
  `record.get(22..).ok_or(Some(StorageError::CorruptData))?` 등으로 대체.
- **심각도:** Note (P3)

#### [처리담당] — 2026-10-30
- 판정: 수용(A — 기존 계약 안의 방어적 리팩토링).
- `recover_inner`의 메시지 재생에서 `record.get(22..).ok_or(Some(StorageError::CorruptData))?`로 변경했다. 선행 chunk 검증은 유지하고 슬라이스 경계를 독립적으로 확인한다.
- 검증: [EV-015](../verification_evidence.md#ev-015)의 두 Rust workspace 회귀(각201개, 복구 통합19개 포함)·clippy·fmt Pass. 기능 결함 재현이 아닌 리팩토링으로 기존 실패/손상/취소 통합 시험을 회귀했다.


---

### RV-017: 스키마/채널 검증 시 반복적 동적 문자열 포맷팅 최적화 권고 (P3)

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/storage/recovery.rs#L469-L477`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/storage/recovery.rs#L469-L477), [`L515-L520`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/storage/recovery.rs#L515-L520)
- **발견점:**
  `Validation::schema`에서 등록된 각 채널과의 일치 여부를 대조할 때:
  ```rust
  name == format!("payload.{:032x}.v{}", c.channel.schema_id, c.channel.schema_version).as_bytes()
  ```
  루프 내부에서 매번 `format!`으로 `String`을 힙 할당하고 비교합니다. 채널 수가 많거나 복구 대상 파일에 스키마 레코드가 여러 번 등장할 경우 불필요한 메모리 할당이 발생합니다.
  또한 `Validation::channel`의 메타데이터 키/값 검증에서도 `format!("{:032x}", ...)` 및 `to_string()`이 반복 호출됩니다.
- **처리 제안:**
  복구 경로는 RT 경로가 아니므로 치명적이지 않으나, 성능 향상 및 불필요한 GC 부담을 줄이기 위해 스택 버퍼(`write!(&mut buf, ...)` 등) 또는 사전 계산된 topic/schema 이름 문자열을 비교하는 구조로 리팩토링할 것을 권고합니다.
- **심각도:** Note (P3)

#### [처리담당] — 2026-10-30
- 판정: 수용(A — 관측 동작/허용 형식 불변인 내부 최적화).
- `matches_text`는 `format_args!`를 64바이트 스택 슬라이스에 기록해 비교한다. 현재 최장 스키마 이름은 `payload.`8 + 32 hex + `.v`2 + u32 최대10 = 52바이트다. 포맷 쓰기 실패는 불일치로 처리한다. 숫자 파싱으로 바꾸지 않아 기존 소문자/제로 패딩/10진수 표현을 그대로 검증한다.
- `Validation::schema/channel`의 `format!`·`to_string` 및 `payload_encoding.clone()`을 제거했다. encoding은 원본 바이트를 직접 비교하여 길이 제한이 새로 생기지 않는다. 공개 타입/예약 산식·RT 경로/의존성은 동일하다.
- 검증: [EV-015](../verification_evidence.md#ev-015)에서 선택 사용자 schema 복구·잘못된 참조 거절·독립 MCAP 판독·allocator9/RT 가드·두 Rust 회귀 Pass. 전체 복구는 여전히 관리 경로이며 처리량 개선을 실측한 것은 아니다. Rust에 GC는 없으며 본 조치는 임시 힙 할당/해제 감소다.


---

## 4. 정밀 체크리스트 점검 결과

### A. RT(실시간) 경로 제약 (CRITICAL) — PASS
- [x] **동적 메모리 할당/해제 금지:** 복구 서브시스템은 별도의 관리 경로(Management path)에 완전히 격리되어 있으며, RT 수집/기록 경로(`capture/mod.rs`, `pool/mod.rs`)에 어떠한 힙 할당이나 구조 변경도 가하지 않음.
- [x] **Blocking & I/O 금지:** RT 경로에 Lock 대기나 동기 I/O 침투 없음.
- [x] **RT 메모리 상한:** `tests/rt-allocation/src/tests.rs`의 `recovery_scratch_and_rebuilt_indexes_stay_inside_shared_output_reservations`를 통해 복구 중 메모리 피크가 엄격한 `reservation` 상한 내에 머물며 잔여 누수가 0임을 실측 검증 완료.

### B. Rust 언어 및 동시성/메모리 안전성 — PASS
- [x] **`unsafe` 최소화 및 Safety:** `crates/dot-logger/src/storage/recovery.rs` 내 `unsafe` 블록 **0건** (100% Safe Rust).
- [x] **원자적 메모리 순서(Memory Ordering):** `cancel: &AtomicBool`에 대해 일관되게 `Ordering::Acquire`를 적용하여 취소 신호의 가시성을 정확히 보장.
- [x] **Untrusted 입출력 바운드 방어:** `read_frame`에서 `frozen source length` 대조, 산술 오버플로우 방지(`checked_add`), 고정 버퍼 상한(`options.limits.max_record_bytes`) 초과 시 DoS 방지(`RecoveryStop::Limit`).
- [x] **CRC 검증:** IEEE CRC-32 fixed table(`CRC_TABLE`)을 채택하여 바이트 단위 선형 검증 및 chunk/data_crc 무결성 보장.
- [x] **취소 및 Drop 안전성:** RV-015에서 조치된 최종 sync 후 취소 재검사 및 `into_inner()`를 통한 파일 close 후 rename/directory sync 순서 준수.

### C. 아키텍처 및 API 경계 — PASS
- [x] **공개 API 오염 방지:** 외부 MCAP 타입이나 내부 복구 wire 형식을 외부에 노출하지 않고 `RecoveredSegment`, `RecoveryReport`, `RecoveryOptions`, `RecoveryConfig`, `RecoveryStop`으로 완벽히 은닉.
- [x] **불변 원본 계약 준수:** 원본은 `R: Read + Seek` (No Write)로만 접근하며 원본 손상/수정 없이 독립적인 새 Segment로만 출력.
- [x] **TDD 및 테스트 충분성:** `fault_storage.rs`에 19개의 결함 주입 테스트(CRC 손상, 중단, 알 수 없는 채널, 디스크 가득참 등) 및 Miri 936회 회귀 통과.

---

## 5. 리뷰 당시 결론 및 다음 액션 (수정 전 이력)

TASK-009의 복구 및 IO 장애 처리 구현은 설계 계약과 실시간/안전성 제약을 철저히 준수하고 있으며 완성도가 매우 높습니다.
발견된 2건의 사항(RV-016, RV-017)은 기능적 결함이나 안전성 문제가 아닌 방어적 코딩 및 코드 최적화를 위한 권고사항(P3)입니다.
- **다음 액션:**
  1. 본 심층 리뷰 문서(`REV-012`)를 `project_state.md`의 미처리 리뷰 목록에 `Open`으로 등록.
  2. 후속 개발 세션 또는 Human 리뷰 시 P3 권고사항(방어적 슬라이싱, 스택 버퍼 최적화) 반영 여부 결정.
  3. TASK-009 최종 구현 승인 후 다음 계획 태스크(TASK-010 active snapshot 등) 진행.


## 6. 처리 결과 — 2026-10-30

Human 「프로젝트 진행현황 확인하고, 새 리뷰 지적사항 처리하고, TASK-009 짧게 요약보고해줘」에 따라 RV-016/017을 반영했다. EV-015 소스51개/명령/로그 hash(비공개 원시 근거: `imported_context/task_009_review_fixes_2026-10-30/results.json`) 기준 검사5개 exit0·source 불변을 확인했다. 자가 검토에서 추가 미해결 P1/P2는 확인하지 못했다. 위 리뷰어의 최초 판정은 수정 전 `5bbb45c`/EV-014의 이력으로 보존한다. 현재 조치는 Resolved이며 Closed로 올리지 않는다. TASK-009 Human 최종 리뷰/native·장비 검증은 잔여다. commit/push는 수행하지 않았다.

2026-10-30 후속 승인: Human이 TASK-009 최신 구현·REV-012 조치를 승인했다. 소스 `5c60399`은 EV-015 source51개/commit blob hash와 일치한다. 본 리뷰를 Closed로 전환하며 과거 Resolved/Human 대기 문장은 당시 이력으로 보존한다. native GAP-004·실장비 검증은 미완료다.

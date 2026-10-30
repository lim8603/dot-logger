# [REV-014] TASK-010 시간·채널 조회 및 active snapshot 심층 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-10-30 / AI ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md))
- **대상:** TASK-010 조회·읽기 수명 구현 커밋 [`8bbec13`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs) (`crates/dot-logger/src/query/mod.rs`, `storage/catalog.rs`, `storage/retention.rs`, `storage/recovery.rs`, `storage/mod.rs`, `lib.rs`, `tests/tc_006_query.rs`, `tests/tc_005_retention.rs`, `tests/api-consumer/src/main.rs`, `tests/rt-allocation/src/tests.rs`)
- **종합 판정:** Pass (권고사항 P2 1건, P3 2건)
  - Safe Rust 100% (신규 `unsafe` 0건), RT 경로 오염 없음 (RT 제로 할당·무대기 불변식 유지)
  - 외부 파티션 사전 예약(`QueryBudget`), 유한 페이지/인라인/블롭 한도 관리, Reader-owned permit drop 순서 안전성 및 회전 추적 완결
  - Miri 1024회 및 1.85.0/1.96.0 듀얼 툴체인 matrix 검증 완료
- **상태:** Resolved (2026-10-30 RV-021~023 조치·EV-017 로컬 Pass, Human 최종 구현 리뷰 대기)

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-021 | P2 | [`query/mod.rs:569-587`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs#L569-L587)<br>[`Snapshot::page`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs#L556-L646) | `Cursor` 페이지 조회마다 전체 rows 재순회, 재필터링 및 $O(N \log N)$ 재정렬 수행 | 고정 버퍼 무할당 원칙은 충족하나 대용량 시 오버헤드 완화를 위해 연속 커서 감지 재사용 또는 `sort_by_cached_key` 최적화 제안 | 수용 — 동일 Query 정렬 재사용 | Resolved |
| RV-022 | P3 | [`query/mod.rs:188-198`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs#L188-L198)<br>[`Payload::Inline`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs#L190-L195) | `Payload::Inline`의 내부 버퍼 `offset` 필드가 public으로 노출되어 내부 구조 노출 | 불투명(opaque) 접근자로 캡슐화하거나 `Page::inline` 경유 접근 권장 rustdoc 명시 | 수용 — Page::inline 안내 | Resolved |
| RV-023 | P3 | [`query/mod.rs:405-410, 428`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs#L405-L410)<br>[`QueryConfig::read_bytes`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs#L67) | `Snapshot::new` 초기 스캔 시 source 전액 차감으로 인한 `read_bytes` 고갈 가능성 | `read_bytes`가 초기 전체 스캔 + 후속 재판독 바이트 합계임을 rustdoc에 명확히 안내 | 수용 — scan+재읽기 예산 명시 | Resolved |

---

## 3. 상세 리뷰 내역

### RV-021: `Cursor` 페이지 조회 시 $O(M \cdot N \log N)$ 반복 정렬 및 필터링 오버헤드

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/query/mod.rs#L569-L587`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs#L569-L587)
- **발견한 문제:**
  ```rust
  for (i, row) in self.rows.iter().enumerate() {
      self.alive(now, cancel)?;
      if matches_filter(row, cursor.query) {
          let time = timestamp(row, cursor.query.basis)?;
          if cursor.query.from <= time && time < cursor.query.to {
              self.order.push(i);
          }
      }
  }
  self.order.sort_unstable_by_key(|i| {
      let r = &self.rows[*i];
      (
          timestamp(r, cursor.query.basis).unwrap_or(0),
          r.record.receipt.producer_id,
          r.record.receipt.accepted_seq,
          self.sources[r.source].sequence,
          r.payload_offset,
      )
  });
  ```
  1. `Snapshot::page`는 각 페이지를 반환할 때마다 `self.order`를 비우고 전체 `self.rows`를 처음부터 다시 필터링하며 `sort_unstable_by_key`를 수행합니다.
  2. 또한 `sort_unstable_by_key`의 클로저 내에서 매 원소 비교마다 `timestamp(r, cursor.query.basis)` 함수가 반복 호출됩니다.
  3. 동일한 질의에 대해 여러 페이지를 순차적으로 탐색(`cursor.position > 0`)할 때, 직전 `page` 호출에서 이미 정렬된 `self.order`를 완전히 버리고 매번 $N$개 항목을 재순회·재정렬하므로, $M$개 페이지를 읽는 총 시간 복잡도가 $O(M \cdot N \log N)$에 달합니다.
  4. 사전 할당된 고정 버퍼(`order: Vec::with_capacity(config.max_records)`) 내에서 무할당으로 결정적 상태를 유지하는 설계 목적상 결함은 아니나, `max_records`가 커질수록(예: 10만 건 이상) CPU 연산 오버헤드가 누적됩니다.
- **처리 제안:**
  - 동일 커서 질의 연속 호출 시 기정렬된 `self.order`의 캐시 재사용(마지막 정렬된 `(query, basis)` 메타데이터 보관)을 검토하거나,
  - 정렬 시 매번 `timestamp`를 재계산하지 않도록 `sort_by_cached_key` 또는 필터링 시 `(timestamp, row_id)`를 함께 보관하는 방안을 후속 성능 최적화로 제안합니다.
- **심각도:** P2 (Warning / 성능 및 확장성)

#### [처리담당]
- 판정 결과: 수용 — 동일 Query 정렬 index 재사용
- 처리 내용: `Snapshot::page`가 모든 Query 필드(Eq/PartialEq)가 같은 경우 기존 order를 재사용한다. 질의 변경 시 같은 사전 예약 버퍼에서 재구성하고 시작 전 캐시 무효화·정렬/생존 검사 완료 뒤 게시한다. cursor 위치/identity·결정적 tie·payload 재읽기/차감·취소/만료 검사는 유지한다. `sort_by_cached_key`는 추가 할당 여지가 있어 사용하지 않았고 기존 `sort_unstable_by_key`를 유지했다. 동일 질의 M개 페이지의 반복 scan/sort는 최초 1회로 줄며 실제 장비 성능은 미측정이다.
- 검증 근거: [EV-017](../verification_evidence.md#ev-017) 및 [검사 명령/소스·로그 hash](../../06_evolution/imported_context/task_010_review_fixes_2026-10-30/results.json). 2026-10-30 작업 트리(기준 HEAD `3eeaaa3`), 미커밋. interleaved cursor 회귀는 리팩토링 전후 동일 결과를 확인했다.

---

### RV-022: `Payload::Inline`의 내부 버퍼 오프셋 노출

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/query/mod.rs#L188-L198`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs#L188-L198)
- **발견한 문제:**
  ```rust
  pub enum Payload {
      Inline {
          offset: usize,
          length: usize,
      },
      Blob(BlobHandle),
  }
  ```
  1. `Payload::Inline`의 `offset` 필드는 `Page` 내부의 private `data: Vec<u8>` 버퍼에서의 시작 인덱스입니다.
  2. 외부 호출자는 `page.inline(item)` 메서드를 통해 안전하게 바이트 슬라이스를 얻어야 하며, raw `offset` 값을 직접 활용할 필요가 없습니다.
  3. `Page::inline`이 `self.data.get(offset..offset.checked_add(length)?)`로 슬라이스 경계를 검사하므로 메모리 비안전성(panic / OOB)은 발생하지 않으나, 내부 버퍼 레이아웃이 공개 API에 직접 노출되는 설계상 아쉬움이 있습니다.
- **처리 제안:**
  - `Payload::Inline`의 필드를 비공개화(private)하고 불투명 접근자를 제공하거나, `Page::inline` 사용을 필수적으로 안내하는 rustdoc 주석을 보강합니다.
- **심각도:** P3 (Note / API 경계 및 캡슐화)

#### [처리담당]
- 판정 결과: 수용 — 호환 유지·rustdoc 보강
- 처리 내용: `Payload::Inline`과 `Page::inline`에 해당 Page에서 얻은 record만 전달하도록 명시했다. offset은 파일 위치가 아니며 Page 사이 재사용 불가, 범위 검사는 하지만 Page identity는 검사하지 않음을 설명한다. 공개 필드 비공개화는 도입하지 않았다.
- 검증 근거: [EV-017](../verification_evidence.md#ev-017) 및 [검사 명령/소스·로그 hash](../../06_evolution/imported_context/task_010_review_fixes_2026-10-30/results.json). 2026-10-30 작업 트리(기준 HEAD `3eeaaa3`), 미커밋. interleaved cursor 회귀는 리팩토링 전후 동일 결과를 확인했다.

---

### RV-023: `Snapshot::new` 초기 스캔 시 source 전액 차감으로 인한 `read_bytes` 고갈 가능성

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/query/mod.rs#L405-L410, L428`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/query/mod.rs#L405-L410)
- **발견한 문제:**
  ```rust
  // Charge a conservative full prefix once. Later page/blob rereads charge again.
  self.charge(index, end, now)?;
  ```
  1. `Snapshot::new` 시점에 각 소스의 전체 `safe_offset`이 `self.charge(index, end, now)`를 통해 `self.remaining`(초기값 `QueryConfig::read_bytes`)에서 일괄 차감됩니다.
  2. 이후 `page` 호출에서 인라인 페이로드를 읽거나 `read_blob`을 호출할 때도 실제 물리 읽기 바이트 수만큼 `charge`가 추가 차감됩니다.
  3. 만약 호출자가 `read_bytes`를 단순히 원본 파일 크기(`max_source_bytes`) 수준으로 설정할 경우, `Snapshot::new` 초기 스캔 직후 잔여량이 0이 되어 이후의 `page`나 `read_blob`이 즉각 `QueryError::Limit`으로 거절될 수 있습니다.
- **처리 제안:**
  - `QueryConfig::read_bytes` rustdoc 주석에 "초기 스캔(모든 source safe_offset 합산) + 후속 페이지 인라인 및 blob 재판독 바이트 합계를 포함해야 함"을 명확히 설명합니다.
- **심각도:** P3 (Note / 문서화 및 운영 가이드)

#### [처리담당]
- 판정 결과: 수용 — 누적 예산 rustdoc 보강
- 처리 내용: `QueryConfig::read_bytes`에 모든 source safe_offset 합계의 초기 scan 차감과 이후 inline/blob 재읽기 합계가 필요함을 명시했다. prefix만 예약하면 payload 재읽기 여유가 없고 반복 호출도 다시 차감됨을 안내한다. API §14.1도 동기화했다.
- 검증 근거: [EV-017](../verification_evidence.md#ev-017) 및 [검사 명령/소스·로그 hash](../../06_evolution/imported_context/task_010_review_fixes_2026-10-30/results.json). 2026-10-30 작업 트리(기준 HEAD `3eeaaa3`), 미커밋. interleaved cursor 회귀는 리팩토링 전후 동일 결과를 확인했다.


## 개발 세션 검증 요약 — 2026-10-30

EV-017 검사14개 exit0: 두 Rust core/all-features workspace·독립 실제 파일 API consumer·clippy/rustdoc·allocator10·fmt, 영향 Miri query10+retention18×8=224 Pass. 기능 보존 회귀를 리팩토링 전후 확인했으며 실제 장비 성능 수락은 하지 않았다. code-review 스킬 기준 자가 점검에서 RT/unsafe/public ownership·오류 의미 변경 없음, 캐시의 query 일치/실패 시 무효화·bounded buffer를 확인했다. Human 최종 구현 리뷰/native GAP-004·장비는 잔여이며 Closed로 올리지 않는다.

# [REV-010] TASK-008 회전·보존·이벤트 보호 심층 코드리뷰

## 1. 리뷰 개요
* **일자 / 리뷰어:** 2026-10-30 / AI ([code-review skill](../../../.agents/skills/code-review/SKILL.md))
* **대상 Task / 기준 커밋:** [TASK-008](../../04_implementation/tasks/TASK-008.md) / commit [`e6669ac`](../../../crates/dot-logger/src/storage/retention.rs) (`c932ee4` 대비 회전·보존·이벤트 보호 추가 구현분, EV-012 기준 소스 49개)
* **종합 판정:** Conditional (Pass 조건부 수용 — P1 0건, P2 1건, P3 2건)
* **진행 상태:** Closed — 2026-10-31 Human 최종 구현 승인 (EV-012/013)

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-012 | P2 | [`catalog.rs:285-350`](../../../crates/dot-logger/src/storage/catalog.rs#L285-L350)<br>[`ReadPin`](../../../crates/dot-logger/src/storage/catalog.rs#L101-L107) | `ReadPin` 누수 시 만료(deadline) 후에도 파일 삭제가 영구 차단되는 고립(Orphaned Pin) 리스크 | `expire_pins` 검토 또는 TASK-010(Active Reader) 연계 강제 회수 정책 정립 | 현 계약 유지·위험 문서화·TASK-010 연결 | Closed |
| RV-013 | P3 | [`retention.rs:476-479`](../../../crates/dot-logger/src/storage/retention.rs#L476-L479) | `SegmentLog::append` 회전 시 `room(now, false)` 중복 호출로 여유 공간 IO 조회 1회 낭비 | `if rotated.is_none() { self.room(now, false)?; }` 분기로 중복 IO 제거 | 수정·회귀 Pass | Closed |
| RV-014 | P3 | [`retention.rs:47-68`](../../../crates/dot-logger/src/storage/retention.rs#L47-L68) | `FileStore::new`의 심볼릭 링크 방어(`symlink_metadata` 검사)의 보안 의도 rustdoc 명시 부재 | rustdoc 주석에 session 디렉터리 symlink traversal 방어 불변식 보강 | 수정·회귀 Pass | Closed |

---

## 3. 상세 리뷰 내역

### RV-012: `ReadPin` 미해제(누수) 시 만료 후에도 세그먼트 삭제가 영구 차단되는 고립(Orphan) 리스크

#### [리뷰어]
* **발견한 문제:**
  1. [`ReadPin`](../../../crates/dot-logger/src/storage/catalog.rs#L101-L107)은 RAII [`Drop`](../../../crates/dot-logger/src/storage/catalog.rs) 구현이 없으며, 오직 [`SegmentLog::release_pin`](../../../crates/dot-logger/src/storage/retention.rs#L641-L643)에 값을 전달해야만 [`Catalog::pins`](../../../crates/dot-logger/src/storage/catalog.rs#L127)에서 제거된다.
  2. 만약 리더 프로세스나 호출자가 버그 또는 비정상 종료로 `ReadPin`을 `release_pin` 없이 유실(drop)하면, `deadline`이 지나 [`consume_pin`](../../../crates/dot-logger/src/storage/catalog.rs#L316-L338)에서 `RetentionError::Expired`가 반환되더라도 [`Catalog::pinned`](../../../crates/dot-logger/src/storage/catalog.rs#L195-L197)는 계속 `true`를 반환한다.
  3. 이로 인해 Circular 모드에서 해당 세그먼트가 영구적으로 [`oldest_deletable`](../../../crates/dot-logger/src/storage/catalog.rs#L198-L202) 대상에서 제외되어 디스크 회수가 차단되고, 누출된 pin 개수가 `max_pins`에 도달하면 신규 `pin` 발급(`RetentionError::PinLimit`)이 불가능해진다.
  4. "물리적 IO 중인 파일의 무단 삭제 방지"라는 설계 원칙은 타당하나, 비정상 리더의 장기 고립에 대한 관리적 회수 경로가 부재하다.
* **처리 제안:**
  - `SegmentLog`에 명시적 만료 pin 정리 API([`expire_events`](../../../crates/dot-logger/src/storage/retention.rs#L615-L618)와 유사한 형태) 또는 타임아웃 경과 후 강제 회수 정책을 검토한다.
  - 또는 후속 과제인 [TASK-010(Active Reader)](../../04_implementation/tasks/TASK-010.md) 인터페이스 설계 시 비정상 리더 감지 및 pin 강제 반환 계약을 연계한다.
* **심각도:** P2 (Warning / 계약·자원 건전성)

#### [처리담당]
* **판정 결과:** 위험 수용·현 계약 유지(A — 승인된 pin 수명 계약의 명시/회귀 보강). 기한 기반 강제 회수 제안은 채택하지 않고, 리더 종료 확인과 반환 경로를 TASK-010에 연결한다.
* **처리 내용:** `ReadPin`에 `must_use`와 유실 시 catalog 생존 기간의 slot/삭제 보호 유지·`PinLimit`/`Capacity` 가능성을 명시했다. `pin`/`release_pin` rustdoc와 인터페이스 §13.4에 오류/취소에도 토큰을 보관하고 실제 IO 종료 후 반환할 책임을 보강했다. TASK-010 완료 체크에 리더 수명 관리와 안전한 고립 회수 조건 검토를 추가했다. 현재 API에서 강제 회수나 RAII 자동 해제를 도입하지 않았다.
* **판단 근거:** 인터페이스 §13.4와 EV-012의 승인된 계약은 만료/Drop을 실제 IO 종료로 간주하지 않는다. 같은 프로세스의 내장형 logger이므로 전체 프로세스 종료 후에도 같은 live catalog가 남는다는 가정은 해당하지 않지만, logger가 살아 있는 동안 호출자가 토큰을 잃는 위험은 유효하다. `must_use`는 명시적 Drop/forget을 방지하지 않으며 위험은 TASK-010 후속으로 남는다.
* **검증 근거:** `pin-baseline.log`에서 수정 전부터 유실 pin의 만료 후 `PinLimit`/`Capacity`와 삭제 금지를 확인했다. 이는 계약 확인용 Green이며 결함 수정 Red로 세지 않는다. 기존 실제 `release_pin` 뒤 삭제 회귀도 함께 실행한다. 최종 결과는 [EV-013](../verification_evidence.md#ev-013)에 기록했다.

---

### RV-013: `SegmentLog::append` 회전 시 `room(now, false)` 중복 호출로 인한 여유 공간 IO 낭비

#### [리뷰어]
* **발견한 문제:**
  - [`SegmentLog::append`](../../../crates/dot-logger/src/storage/retention.rs#L424-L523)에서 회전 조건 충족 시 `self.rotate(now)?`를 호출한다.
  - `rotate` 내부에서 이미 `self.room(now, true)?`와 `self.open(now)?`(내부에서 `self.room(now, false)?` 실행)을 차례로 완료하여 여유 공간과 세그먼트 상한을 검증했다.
  - 그러나 479행에서 `self.room(now, false)?;`가 무조건 다시 한 번 호출되어, [`SegmentStore::available_bytes`](../../../crates/dot-logger/src/storage/retention.rs#L18) 호스트 syscall/closure가 1회 중복 실행된다.
* **처리 제안:**
  - `if rotated.is_none() { self.room(now, false)?; }`로 조건을 좁혀 회전 발생 시의 중복 검사를 생략한다.
* **심각도:** P3 (Note / 효율성 및 코드 정리)

#### [처리담당]
* **판정 결과:** 수용(A — 승인된 회전/여유 공간 검사 안의 중복 호출 제거).
* **처리 내용:** `append`는 회전하지 않았을 때만 별도 `room`을 호출한다. 회전 시 기존 `rotate`/`open`의 검사 2회는 유지하고 마지막 중복 1회를 제거했다. 일반 append는 fresh probe와 reserve 거절을 유지한다.
* **검증 근거:** `rotating_append_avoids_a_redundant_host_probe_and_later_appends_still_check_reserve`에서 기존 3회/기대 2회 실패(Red, `red-probe.log`) 뒤 2회(Green)를 확인했다. 다음 append의 reserve 부족 거절·재시도·독립 MCAP 판독을 함께 검증했다. EV-013 참조. 호스트 probe 호출 횟수 검증이며 장비 syscall 비용/성능 측정 결과는 아니다.

---

### RV-014: `FileStore::new`의 심볼릭 링크 방어 검사의 보안 의도 rustdoc 명시 부재

#### [리뷰어]
* **발견한 문제:**
  - [`FileStore::new`](../../../crates/dot-logger/src/storage/retention.rs#L50-L68)의 `session` 디렉터리 생성부에서 `symlink_metadata(&session)?.file_type().is_dir()`를 검사하여, 심볼릭 링크를 통한 session 경로 탈취를 효과적으로 차단하고 있다.
  - 그러나 rustdoc 주석에는 단순 "fresh target namespace below an existing trusted base directory"로만 기술되어 있어, 이 검사가 보안상의 symlink traversal 방어 불변식임을 코드 열람자가 즉시 파악하기 어렵다.
* **처리 제안:**
  - rustdoc에 session 경로가 실제 디렉터리여야 하며 symlink가 허용되지 않는 보안 불변식을 명시한다.
* **심각도:** P3 (Note / 문서화 보강)

#### [처리담당]
* **판정 결과:** 수용(A — 보안 의도 문서 보강).
* **처리 내용:** `FileStore::new` rustdoc에 기존 session 경로가 symlink가 아닌 실제 디렉터리여야 함을 명시했다. 검사는 사전 경로 검사이며 동시 경로 교체를 막는 primitive는 아니므로, trusted base/session의 배타적 관리 전제도 명시했다. 제품 IO 동작은 변경하지 않았다.
* **검증 근거:** 코드와 문서 대조 및 두 Rust rustdoc `-D warnings` Pass로 확인했다(EV-013). 문서 수정 전용 Red/형식적 단위 테스트는 추가하지 않았다.


## 최종 처리 결과 — 2026-10-30

[EV-013](../verification_evidence.md#ev-013) / source49개·명령/로그 hash(비공개 원시 근거: `imported_context/task_008_review_fixes_2026-10-30/results.json`): 보존 통합17개(신규2)·두 Rust core/all-features workspace/consumer/예제/allocator8/clippy/rustdoc/fmt, 보존 Miri16×8=128회 Pass다. RV-012의 잔여 위험은 문서와 TASK-010 체크에 명시했으며 토큰 유실을 자동 복구한 것은 아니다. Human 구현 리뷰/native·장비는 잔여로 Conditional/Resolved를 유지한다. 새 REV를 만들어 처리 이력을 분산하지 않았고 제품 수정은 리뷰 전용 세션이 아닌 개발 세션 #003에서 수행했다. commit/push는 수행하지 않았다.

## Human 최종 구현 승인 — 2026-10-31

Human 「승인, 다음작업 진행하자」로 직전 보고의 TASK-008 REV-009/010, TASK-010 REV-013/014, TASK-011 REV-017 구현·리뷰 조치를 수락했다. 본 리뷰와 해당 RV 항목은 Closed다. EV-012/013의 검증 근거 및 현재 구현 범위를 승인하며, 앞선 Resolved/승인 대기 본문은 승인 전 이력으로 보존한다. 낮은 ReadPin 유실의 강제 회수 없음·실제 reader close 뒤 반환 계약과 native 공개 후(T3)·현재 PC 실측/UPS 시뮬레이션 잔여는 유지한다. Task/MS 및 Gate4/5 완료를 뜻하지 않는다. 다음 작업은 TASK-022 프로필 구체화이며 커밋/푸시는 수행하지 않았다.

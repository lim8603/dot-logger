# [REV-016] TASK-011 로컬 오프라인 재생 및 가상 시계 제어 심층 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-10-30 / AI ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md))
- **대상:** TASK-011 로컬 재생 구현 커밋 [`ccecf4a`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs) (`crates/dot-logger/src/replay/mod.rs`, `crates/dot-logger/src/query/mod.rs`, `crates/dot-logger/src/lib.rs`, `crates/dot-logger/tests/tc_006_replay.rs`, `crates/dot-logger/tests/tc_005_retention.rs`, `tests/api-consumer/src/main.rs`, `tests/rt-allocation/src/tests.rs`, `.github/workflows/ci.yml`)
- **종합 판정:** Pass (권고사항 P2 1건, P3 2건)
  - **Safe Rust 100%**: 신규 `unsafe` 0건, 원소유권 및 라이프타임(`&mut self` 바운드)에 의한 동기식 메모리/버퍼 배타성 보장
  - **RT 경로 제로 오염**: 오프라인 전용 모듈로 RT 캡처 스레드 오염 및 힙 할당 전이 없음 (`tests/rt-allocation` 0건 유지)
  - **자원 수명/결정론**: 단일 레코드 프리페치(`Pending`), 체크된 세대(`generation`), 유한 양수 유리수 배속(`Speed`), late consumer 시계 지연, 소유 스냅샷 및 Reader close 보장
  - **검증 완료**: `cargo fmt`, `cargo clippy -D warnings`, 전체 단위/통합/문서 테스트 및 Miri 10회 통과
- **상태:** Closed (2026-10-31 Open → In Progress → Resolved/EV-019 Pass → Human 최종 구현 승인)

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-026 | P2 | [`replay/mod.rs:363-367`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs#L363-L367)<br>[`Replay::poll`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs#L294-L386) | late 누적 산술 오버플로 시 `self.fail(ReplayError::Limit)` 미호출로 인한 리소스 미정리 및 상태 불일치 가능성 | `checked_add` 실패 시 `self.fail(ReplayError::Limit)`을 호출하여 터미널 정지 및 스냅샷/리더 리소스를 즉시 해제하도록 일원화 | 방어적 개선 수용·현 불변식상 오버플로 불가 | Closed |
| RV-027 | P3 | [`replay/mod.rs:13-28`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs#L13-L28)<br>[`Speed`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs#L13-L28) | `Speed` 구조체의 필드가 private이나 외부 접근자(`numerator`, `denominator`) 및 `Display` 구현 부재 | `pub fn numerator(&self) -> u32`, `pub fn denominator(&self) -> u32` 접근자 및 `Display` 구현 추가 권장 | 수용 | Closed |
| RV-028 | P3 | [`replay/mod.rs:70-104`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs#L70-L104)<br>[`ReplayPoll`, `ReplayFrame`, `ReplayPayload`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs#L70-L104) | 핵심 어휘 타입인 `ReplayPoll`, `ReplayFrame`, `ReplayPayload`에 `Debug` derive 부재 | 테스트 assertion(`assert_matches!`), 로깅 및 CLI 진단 편의를 위해 `#[derive(Debug)]` 추가 권장 | 수용 | Closed |

---

## 3. 상세 리뷰 내역

### RV-026: `Replay::poll` late 누적 시 산술 오버플로에 대한 터미널 실패 처리 일원화

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/replay/mod.rs#L363-L367`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs#L363-L367)
- **발견한 문제:**
  ```rust
  let Some(due) = u64::try_from(delay)
      .ok()
      .and_then(|n| self.anchor_host.checked_add(n))
  else {
      return self.fail(ReplayError::Limit);
  };
  ...
  let media = u128::from(pending.record.record.metadata.mono_ns);
  if self.anchor_media > media {
      self.anchor_media = media;
      self.anchor_host = now;
  } else {
      self.anchor_host = self
          .anchor_host
          .checked_add(late)
          .ok_or(ReplayError::Limit)?; // <-- self.fail() 미호출
  }
  ```
  1. `Replay::poll`의 전반부(L342-L347)에서는 `due` 계산 중 `checked_add` 산술 오버플로 발생 시 `self.fail(ReplayError::Limit)`을 호출하여 터미널 `Stopped` 상태로 전이하고 스냅샷/리더 리소스를 즉시 안전하게 해제합니다.
  2. 반면 후반부(L363-L367)의 `self.anchor_host.checked_add(late)` 실패 시에는 `self.fail()`을 거치지 않고 `Err(ReplayError::Limit)`을 바로 반환합니다.
  3. 이 경우 `self.state`는 여전히 `Playing`으로 남아있고 `self.pending.delivered`는 미설정 상태이며, 소유한 스냅샷 및 Reader 파일 핸들이 정리되지 않고 유지됩니다.
  4. 시간축이 단조 증가하므로 이후 재호출 시에도 복구가 불가능한 터미널 오류 성격을 띠므로, 전반부와 마찬가지로 `self.fail(ReplayError::Limit)`을 통해 명시적으로 닫아주는 것이 일관된 리소스 수명 관리에 부합합니다.
- **처리 제안:**
  - `self.anchor_host.checked_add(late)` 실패 시 `self.fail(ReplayError::Limit)`을 호출하여 터미널 정지 및 리소스 정리를 수행하도록 수정합니다.
- **심각도:** P2 (Warning / 상태 일관성 및 리소스 수명)

#### [처리담당]
- 판정 결과: **대안 — 실제 결함 주장은 현 불변식에서 성립하지 않으며 방어적 처리 일원화는 수용**. 전달 분기 진입 시 `anchor_host <= due <= now`, `late = now - due`이므로 `anchor_host + late <= now <= u64::MAX`다. 따라서 공개 API의 유효 상태에서 해당 덧셈 실패를 재현할 수 없으며 기존 상태/리소스 누수를 확인한 것으로 보고하지 않는다.
- 처리 내용: `Replay::poll`의 checked_add 실패를 `self.fail(ReplayError::Limit)`로 통일하고 도달 불가 근거를 주석으로 남겼다. 실제로 도달 가능한 다음 deadline의 표현 한도와 구분했다. 승인된 terminal 정리 계약 안의 A 범위이며 새 오류 전략/ADR 결정은 없다.
- 검증 근거: **EV-019**. `late_delivery_near_host_limit_preserves_spacing_or_stops_and_releases_budget`는 방어 수정 전에도 Pass(`baseline-host-limit.log`)다. host 상한 직전 늦은 전달 뒤 간격 유지, 다음 deadline overflow의 Stopped/예약0을 검증하며 해당 checked_add의 Red라고 주장하지 않는다. replay12·Miri12×8=96 Pass. 소스는 미커밋 작업 트리로 results.json의 SHA-256을 기준으로 한다.

---

### RV-027: `Speed` 구조체의 분자/분모 접근자 및 `Display` 구현 부재

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/replay/mod.rs#L13-L28`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs#L13-L28)
- **발견한 문제:**
  ```rust
  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  pub struct Speed {
      numerator: u32,
      denominator: u32,
  }
  ```
  1. `Speed`의 `numerator`와 `denominator` 필드가 비공개(private)이며, 외부에서 이를 읽을 수 있는 getter 함수가 제공되지 않습니다.
  2. `Replay`를 사용하는 외부 애플리케이션, 어댑터, 또는 CLI에서 현재 설정된 재생 배속을 사용자에게 표시하거나(예: "1.5x" 또는 "3/2 배속"), 메트릭/로깅으로 남길 수 있는 수단이 없습니다.
  3. 또한 `std::fmt::Display` 트레이트가 구현되어 있지 않아 문자열 변환 시 `{:?}`에만 의존해야 합니다.
- **처리 제안:**
  - `pub fn numerator(&self) -> u32` 및 `pub fn denominator(&self) -> u32` getter 메서드를 추가합니다.
  - `std::fmt::Display` 구현(예: `write!(f, "{}/{}", self.numerator, self.denominator)`) 및 기본 상수(예: `Speed::REALTIME`) 추가를 권고합니다.
- **심각도:** P3 (Note / API 사용성 및 진단 편의)

#### [처리담당]
- 판정 결과: 수용(A — 승인된 배속 표현의 조회/진단 편의).
- 처리 내용: `Speed::numerator`/`denominator` 접근자, 원 성분을 약분하지 않는 `Display`(`6/4`), `Speed::REALTIME`(1/1)을 추가했다. private 필드와 0 거절 계약은 보존한다.
- 검증 근거: **EV-019**, `speed_and_borrowed_deliveries_support_public_diagnostics`의 미구현 compile Red(`red-public-diagnostics.log`) → 구현 후 Green. 두 Rust workspace/all-features·public consumer·clippy/rustdoc/fmt Pass. 소스 hash는 results.json 참조.

---

### RV-028: 어휘 타입 `ReplayPoll`, `ReplayFrame`, `ReplayPayload`의 `Debug` derive 부재

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/replay/mod.rs#L70-L104`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/replay/mod.rs#L70-L104)
- **발견한 문제:**
  1. `ReplayPayload<'a>`, `ReplayFrame<'a>`, `ReplayPoll<'a>` 타입에 `#[derive(Debug)]`가 누락되어 있습니다.
  2. 표준 Rust API 설계 관례상 poll 계열 반환 열거형 및 프레임 구조체는 로깅, `assert_matches!`, `assert_eq!`, 에러 진단 메시지 출력을 위해 `Debug`를 필수로 구현해야 합니다.
  3. 내부 필드인 `CapturedRecord`, `BlobHandle`, `&[u8]`, `u64`가 모두 `Debug`를 만족하므로 무비용으로 derive 가능합니다.
- **처리 제안:**
  - `ReplayPayload`, `ReplayFrame`, `ReplayPoll`에 `#[derive(Debug)]`를 추가합니다.
- **심각도:** P3 (Note / API 관례 및 진단 편의)

#### [처리담당]
- 판정 결과: 수용(A — 동작 변경 없는 진단 trait 추가). Debug는 진단 편의이며 기존 API에서 필수였다는 의미로 해석하지 않는다.
- 처리 내용: `ReplayPoll`, `ReplayFrame`, `ReplayPayload`에 `#[derive(Debug)]`를 추가했다. 자동 포맷 호출/RT 경로 변경은 없으며 명시적 Debug 포맷 시 inline bytes도 표시한다.
- 검증 근거: **EV-019**, 동일 public diagnostics 시험의 Debug trait 부재 compile Red → Waiting/Ready/frame/payload 진단 Green. 두 Rust allocator10 Pass·RT 할당/해제0/0, 영향 Miri96 Pass. 소스 hash는 results.json 참조.

---

## 4. 품질 게이트 및 정적 분석 검증

| 항목 | 명령 / 도구 | 결과 | 비고 |
|---|---|---|---|
| 포맷 검사 | `cargo fmt --check` | Pass | 스타일 컨벤션 100% 준수 |
| 정적 분석 | `cargo clippy --all-targets -- -D warnings` | Pass | 경고 0건 |
| 단위/통합 테스트 | `cargo test` | Pass | 14 unit + 19 fault + 13 cfg + 2 dep + 21 storage + 19 retention + 10 query + 10 replay + 17 doc = 총 125개 테스트 성공 |
| Unsafe / 메모리 모델 | `cargo +nightly-2026-10-26 miri test -p dot-logger --test tc_006_replay` | Pass | Miri 10개 검증 Pass (UB/누수 0건) |
| RT 제약 불변식 | `tests/rt-allocation` | Pass | RT 경로 힙 동적 할당 0건 유지 |


## 5. 개발 세션 반입·처리 결과 — 2026-10-31

Human 「프로젝트 진행상황 브리핑하고, 리뷰 반입 및 처리, TASK-011 요약 리포트 해줘」에 따라 기존 리뷰를 개발 세션에 반입했다. 리뷰어의 원문은 이력으로 보존하고 위 처리담당 판정으로 현 판단을 구분한다. RV-026은 방어 개선, RV-027/028은 수용·반영으로 전 항목 Resolved다.

[EV-019](../verification_evidence.md#ev-019)·검사10개/source54개/로그 hash(비공개 원시 근거: `imported_context/task_011_review_fixes_2026-10-31/results.json`): Rust1.85/1.96 workspace all-features(각 replay12 포함)·독립 실제 파일 core consumer·clippy/rustdoc·fmt 및 replay Miri96 Pass. RT/unsafe/의존성·lock·feature 구성은 변경하지 않았다. feature 16조합 전체·query/retention Miri 전체는 기존 EV-018 이력이며 이번에는 영향 범위를 검증했다. native GAP-004·장비/실제 timer·fresh audit는 미실행이다. 신규 미해결 P1/P2는 이번 변경 자가 점검에서 확인되지 않았다.

커밋/푸시·Human 최종 구현 리뷰 승인은 이 요청에 포함하지 않는다. REV-015/016 Resolved·TASK-011/MS-004 In Progress를 유지하며 TASK-012는 자동 착수하지 않는다.

## Human 최종 구현 승인 — 2026-10-31

구현 계약 확인(배속·pause/resume·seek/stop·세대·late 간격·prefetch/읽기 예산·reader 수명) 보고 후 Human 「승인」을 TASK-011 최종 구현 승인으로 반영했다. 승인 대상은 EV-018 구현 및 EV-019/source54개 hash로 고정된 현재 리뷰 조치 작업 트리다. REV-015/016 및 RV-024~028을 Closed로 마감한다. 이전 본문의 Resolved/Human 대기 표기는 승인 전 이력이다.

native GAP-004·다른 OS/장비·실제 timer/성능 검증은 잔여이며 TASK-011/MS-004 In Progress·Gate4/5 Not Ready를 유지한다. 소스 hash가 EV-019와 같아 성공 시험은 반복하지 않았다. 이번 승인은 구현 수락이며 커밋/푸시·TASK-012 착수는 수행하지 않았다.

## 세션 마감·게시 — 2026-10-31

Human 「커밋 푸쉬 후 세션 마무리해줘」에 따라 리뷰 조치 소스 `c6cc5a5`를 origin/main에 게시하고 원격 hash 일치를 확인했다. EV-019 source54개·검사10개 exit0·Miri96·원본/뷰 로그12쌍 hash와 source commit blob54개가 일치해 동일 성공 시험을 반복하지 않았다. 커밋 대조(비공개 원시 근거: `imported_context/task_011_review_fixes_2026-10-31/commit.json`)·소스 게시 근거(비공개 원시 근거: `imported_context/task_011_review_fixes_2026-10-31/push.json`). 이전 미커밋/미게시 문장은 검증 당시 이력이다.

TASK-011 Human 승인·REV-015/016 Closed, ADR-011 Accepted를 유지한다. 현재 PC 실측/UPS 시뮬레이션을 수락하며 native는 공개 완료 후(T3) 진행한다. 다음 시작점은 TASK-022의 실행 커널 확인·P0/제어 부하/기한/경로/예산 프로필 준비다. 현재 generic 실행/RT 설치를 구분하며 재부팅·부트 변경은 수행하지 않았다. TASK-008/010 Human 리뷰·PC 실측/UPS/24h 미실행·Gate4/5 Not Ready는 잔여다. 검증/수락/마감 문서는 별도 docs(cowork)·[skip ci] 커밋으로 게시하며 세션 종료 후 새 Task를 자동 착수하지 않는다.

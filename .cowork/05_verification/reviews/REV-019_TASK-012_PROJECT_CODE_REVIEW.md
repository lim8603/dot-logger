# [REV-019] TASK-012 원격 어댑터 및 프로젝트 종합 심층 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-10-31 / AI ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md))
- **대상:** TASK-012 원격 어댑터(gRPC/ACK/관리) 구현 및 프로젝트 전체 형상 (커밋 [`5f0ad83`](../../../crates/dot-logger/src/adapters/remote/mod.rs) 기준)
  - `crates/dot-logger/src/adapters/remote/` (`mod.rs`, `types.rs`, `hub.rs`, `receiver.rs`, `server.rs`, `wire.rs`, `tests.rs`)
  - `crates/dot-logger/proto/logger.proto`
  - `crates/dot-logger/tests/tc_007_remote.rs`
  - 코어 수집 경로(`crates/dot-logger/src/capture/`) 및 RT 제약 격리 상태
- **종합 판정:** Pass (P2 권고 2건, P3 참고 1건)
  - **RT 경로 엄격 격리**: 원격 gRPC/어댑터(`adapters/remote`)는 `feature = "remote"`로 완전 격리되어 있으며, RT 수집 코어(`capture/`)의 무할당·무대기 제약을 전혀 침해하지 않음 (`tests/rt-allocation` 할당/해제 0/0 확인).
  - **동시성 및 메모리 안전성**: Tonic/Tokio 기반의 비동기 I/O와 원격 버퍼 예약(`QueryBudget`), 단조 `AtomicU8` 상태 머신(Pending → Acknowledged / Unknown / Dropped), 세대(`generation`) 기반 분기 보호가 엄격히 적용됨.
  - **소유권 및 취소 안전성(Cancellation Safety)**: 클라이언트 타임아웃 발생 시에도 호스트 I/O 커맨드(`CommandInbox`/`Command`)는 조기 반환되지 않고 실제 호스트 완료 시까지 리소스를 유지함.
  - **정적 분석 및 테스트 전체 통과**: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, 전체 단위/통합 테스트 24+19+13+21+19+10+14+3=123건 Pass.
- **상태:** Closed — 2026-10-31 #007 EV-023 조치/검증 Pass·Human 위임에 따른 AI 마감. 최초 리뷰 및 #006 반입 계획은 당시 이력으로 보존한다.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-036 | P2 | [`Incoming::poll_next`](../../../crates/dot-logger/src/adapters/remote/server.rs#L195) | 연결 포화·연속 유입 시 즉시 재폴링으로 CPU 소비 증가 가능 | 기존 초과 연결 거절·한도를 유지하며 재폴링 대기 도입 | 수용 — 재현/구현 대기 | Closed |
| RV-037 | P2 (원 리뷰) | [`equal`](../../../crates/dot-logger/src/adapters/remote/server.rs#L42) | 길이 불일치 조기 반환 존재. 실제 타이밍 공격은 미검증 | 비교 경로 보강 검토, 기능 시험과 시간 특성 증거 구분 | 부분 수용 — 방어 보강, 취약점 확정 유보 | Closed |
| RV-038 | P3 | [`Reassembler::discard_generations`](../../../crates/dot-logger/src/adapters/remote/receiver.rs#L203) | 유한 세대 gate가 서로 다른 구독 ID로 소진될 수 있음 | 연결별 수신자 수명·재연결 시 재생성 지침 보강 | 수용 — 문서 조치 대기, 자동 제거 미채택 | Closed |

---

## 3. 상세 리뷰 내역

### RV-036: `Incoming::poll_next` 연결 한도 초과 시 비지 루프(busy spin) 완화 권고

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/adapters/remote/server.rs#L197-L207`](../../../crates/dot-logger/src/adapters/remote/server.rs#L197-L207)
- **발견한 문제:**
  ```rust
  Poll::Ready(Ok((stream, _))) => match self.permits.clone().try_acquire_owned() {
      Ok(permit) => Poll::Ready(Some(Ok(LimitedIo {
          stream,
          _permit: permit,
      }))),
      Err(_) => {
          drop(stream);
          cx.waker().wake_by_ref();
          Poll::Pending
      }
  },
  ```
  1. `LimitedIo` 스트림을 수락할 때 `self.permits.try_acquire_owned()`를 통해 동시 연결 수를 제한합니다.
  2. 한도에 도달한 상태(`Err(_)`)에서 들어온 `stream`을 드롭하고 `cx.waker().wake_by_ref()`를 호출한 뒤 `Poll::Pending`을 반환합니다.
  3. 만약 리스너 소켓에 백로그 연결이 연속해서 인입되고 있는 상황이라면, 리스너가 여전히 읽기 가능 상태이므로 Tokio 이벤트 루프가 즉시 `poll_next`를 다시 호출하게 됩니다.
  4. 이로 인해 한도가 꽉 찬 상태에서 대량의 연결 시도가 들어올 경우 CPU 코어를 100% 점유하는 바쁜 대기(busy spin) 현상이 유발될 수 있습니다.
- **처리 제안:**
  - 연결 한도 초과 시 즉시 `wake_by_ref()`를 부르지 않고, 기존 연결이 종료되어 세마포어 퍼밋이 반환될 때 리스너 태스크가 깨어나도록 연동하거나, 혹은 미세한 백오프(예: 지수 백오프 또는 yield/sleep)를 주어 CPU 급증을 방지하도록 개선을 권고합니다.
- **심각도:** P2 (Warning / 서비스 가용성 및 리소스 방어)

#### [처리담당] — 2026-10-31 반입
- 판정: 수용. `poll_accept` 성공 후 permit 취득 실패 시 stream을 닫고 자기 자신을 즉시 깨우는 경로가 확인된다. 대량 연결이 계속 도착하면 재폴링 비용이 지속될 수 있다. 연결 유입이 멎으면 다음 `poll_accept`가 Pending이므로, 포화만으로 영구 spin하거나 CPU 100%가 된다고 확정하지 않는다.
- 처리 계획: 기존 초과 연결 거절·연결 수 한도를 유지하며 재폴링에 유한 대기를 둔다. 단순히 `wake_by_ref`만 삭제하면 이후 진행에 필요한 wake가 사라질 수 있으므로 대기와 wake 등록을 함께 검증한다.
- 첫 검증: 연결 한도를 채운 상태에서 초과 연결을 연속 유입하고, 즉시 자기 wake 반복 억제·기존 연결 동작·permit 반환 후 새 연결 수락·shutdown 진행을 확인한다. CPU 실측 수치는 별도 증거 없이 주장하지 않는다.
- 현재 근거는 코드 대조뿐이다. 재현 테스트·코드 수정·새 EV는 아직 없다.

---

### RV-037: 토큰 constant-time 비교 시 길이 정보 조기 반환 방어 권고

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/adapters/remote/server.rs#L42-L47`](../../../crates/dot-logger/src/adapters/remote/server.rs#L42-L47)
- **발견한 문제:**
  ```rust
  fn equal(a: &[u8], b: &[u8]) -> bool {
      if a.len() != b.len() {
          return false;
      }
      a.iter().zip(b).fold(0u8, |d, (a, b)| d | (a ^ b)) == 0
  }
  ```
  1. 원격 gRPC 요청의 `Authorization: Bearer <token>` 헤더를 검증할 때 Timing Attack을 방어하기 위해 바이트 XOR 폴딩(`fold(0u8, ...)`)을 구현하였습니다.
  2. 그러나 첫 줄에서 `a.len() != b.len()`인 경우 즉시 `false`를 반환합니다.
  3. 이는 토큰 길이가 일치하지 않는 경우와 길이가 일치하지만 내용이 틀린 경우 간에 응답 시간의 미세한 차이를 발생시켜, 공격자가 유효한 토큰의 정확한 길이를 추론할 수 있는 정보 누출(Timing Side-Channel) 여지를 남깁니다.
- **처리 제안:**
  - 토큰 길이에 구애받지 않고 일정한 시간 동안 검증을 수행하도록 하거나, 또는 `subtle::ConstantTimeEq` 등의 표준 constant-time 비교 방식을 도입하는 것을 권고합니다. (현재 허용 토큰 길이가 16..=256자 그래픽 문자로 제한되어 있어 즉각적인 취약점은 아니나 방어 심도를 위해 권고함).
- **심각도:** P2 (Warning / 보안 심도 방어)

#### [처리담당] — 2026-10-31 반입
- 판정: 부분 수용. 길이 불일치 조기 반환과 일치 길이의 XOR fold는 확인했다. 소스 수준의 fold만으로 최적화 후 constant-time을 보장할 수 없으며, 길이 분기 관측만으로 원격 공격 성립·인증 우회를 확정할 수도 없다. 원 리뷰 P2는 이력으로 보존하고 방어 보강 대상으로 처리한다.
- 처리 계획: 기존 token 형식/길이·read/manage·잘못된 헤더 거절·secret 비노출 정책 안에서 비교 경로를 보강한다. 길이 조기 반환 제거만으로 인증 전체가 constant-time이라고 보고하지 않는다. 새 의존성이나 인증 전략 변경이 필요하면 해당 H/J 결정을 별도로 제시한다.
- 첫 검증: 정상 token·같은 길이의 오답·다른 길이/최대 길이 초과 입력·권한 구분·malformed header 회귀. 기능 시험 결과를 타이밍 보장의 증거로 사용하지 않는다.
- 현재 근거는 `server.rs`와 `AccessToken` 검증 코드 대조다. 공격 실증·시간 측정·코드 수정·새 EV는 아직 없다.

---

### RV-038: `Reassembler`의 generation gates 누적에 대한 수명 주기 관리 명확화

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/adapters/remote/receiver.rs#L216-L226`](../../../crates/dot-logger/src/adapters/remote/receiver.rs#L216-L226)
- **발견한 문제:**
  ```rust
  if let Some((_, old)) = gates.iter_mut().find(|(s, _)| *s == subscription) {
      if generation < *old {
          return Err(RemoteError::Gone);
      }
      *old = generation;
  } else {
      if gates.len() >= self.records {
          return Err(RemoteError::Limit);
      }
      gates.push((subscription, generation));
  }
  ```
  1. `Reassembler`는 수신한 조각들의 세대 순서를 보장하기 위해 `gates: Vec<(u64, u64)>`에 `(subscription, generation)` 쌍을 유지합니다.
  2. 새로운 `subscription`이 추가될 때 `gates.len() >= self.records`이면 `RemoteError::Limit`을 반환하여 메모리 예산을 엄격히 방어합니다.
  3. 주석에 "Tracking is limited to the configured record count; recreate the receiver for a new connection"으로 명시되어 있으나, 장기 실행되는 수신자 애플리케이션에서 재연결마다 새 subscription id를 발급받을 경우 점진적으로 `Limit`에 도달할 수 있습니다.
- **처리 제안:**
  - 클라이언트/수신자 통합 가이드에 수신자 인스턴스의 재사용 범위를 명문화하거나, 종료된 subscription을 명시적으로 정리하는 인터페이스 제공을 검토할 것을 권고합니다.
- **심각도:** P3 (Note / API 사용 가이드 및 운영성)

#### [처리담당] — 2026-10-31 반입
- 판정: 문서 보강 수용. gate는 `discard_generations`에서 새 구독 ID를 등록할 때 누적되며, 모든 수신 조각이 자동으로 gate를 추가하는 것은 아니다. `records` 상한과 새 연결마다 receiver를 재생성하라는 rustdoc은 이미 있다. 무제한 누수가 아닌 유한 예산의 수명 지침 문제다.
- 처리 계획: API §14.3과 TASK-013의 재연결/receiver 소유 지침에 기존 계약을 명확히 한다. 연결별 Reassembler를 만들고 이전 연결의 미인계 owner는 실제 종료까지 기존 예산을 보유한다.
- 오래된 gate의 자동 삭제는 미채택한다. 삭제 뒤 늦은 이전 세대 조각이나 ready owner의 handoff가 다시 허용될 수 있어 기존 RV-033 방어를 약화할 수 있다.
- 확인할 경계: 서로 다른 구독 gate의 상한 도달·동일 구독 generation 갱신·재연결 시 새 receiver 생성·이전 owner의 ACK 거절/예산 반환. 문서 보강 자체에 형식적인 실패 테스트를 만들지는 않는다. 현재 문서 조치는 미완료다.

---

## 4. 종합 평가 및 다음 액션

1. **종합 평가:**
   - TASK-012의 원격 gRPC/ACK/관리 어댑터 구현은 승인된 인터페이스 계약(`§14.3`)과 P&A 아키텍처를 완벽히 충족하며, RT 코어에 대한 무영향성 및 리소스 한도 방어(EV-022)가 검증되었습니다.
   - 발견된 RV-036, RV-037, RV-038은 프로덕션 운영 환경에서의 가용성·보안 방어 심도를 높이기 위한 개선 권고사항(P2/P3)으로, 현재 기능 동작을 저해하는 Critical(P1) 결함은 없습니다.
2. **다음 액션:**
   - RV-036/037 구현·회귀와 RV-038 문서 조치 후 검증 근거를 기록하고 Resolved로 올린다. 이번 반입은 제품 개선 구현이나 Human 수락을 뜻하지 않는다.
   - 이후 REV-018/REV-019와 TASK-012 로컬 구현의 Human 최종 수락을 받는다. native·LAN/실앱·전체 RSS·현재 PC 성능/UPS는 별도 미검증이다.
   - 후속 [TASK-013](../../04_implementation/tasks/TASK-013.md) Rust 클라이언트/CLI 착수는 TASK-012 수락 후 지시를 따른다.

## 5. 반입 검증 및 승인 경계 — 2026-10-31

- EV-022 `results.json`의 source70개 SHA-256을 현재 디스크와 대조해 전부 일치했다. 기록된 검사11개 exit0·영향 Miri48 Pass를 유지하며 동일 성공 시험은 재실행하지 않았다.
- 이번 세션은 기존 리뷰 반입·판정·승인 업무 보고 범위다. 새 리뷰 회차·EV를 만들지 않았고 제품 코드/테스트/의존성도 변경하지 않았다. 최초 리뷰의 Pass는 리뷰어 평가이며 개선 조치 완료나 최종 수락을 뜻하지 않는다.
- 승인된 계약 안의 재현·수정·문서 보강은 A 범위다. 각 세부 단계마다 재승인받지 않는다. TASK-012 최종 구현 수락은 §1E의 Human 리뷰 사항이며 TASK-013 전환은 현재 작업 지시를 따른다.


## 6. 조치·검증 및 위임에 따른 마감 — 2026-10-31 #007

Human 「승인」으로 기존 TASK-012 로컬 구현을 수락하고 「모든 권한을 위임」하여 추가 조치의 검증·수락/마감 판단과 TASK-013 착수를 맡겼다. 이번 조치는 [EV-023](../verification_evidence.md#ev-023)·소스 `4f79119`로 검증 후 AI가 수락했다. 상태 이력은 Open → In Progress(#006 반입) → Resolved(조치/검증) → Closed(위임된 마감)다. 앞선 “미수정/구현 대기”는 #006 당시 상태다.

| 항목 | 실제 조치 | 검증 | 결과 |
|---|---|---|---|
| RV-036 | Incoming은 기존 초과 TCP 거절 후 고정10ms timer를 등록한다. permit 한도/기존 연결 수명은 유지한다 | 기존 구현에서 즉시 자기 wake1회 행동 Red → Green. 초과 연결 EOF·대기 wake·permit 반환 뒤 수락/반환 및 실제 소켓/TLS/종료 회귀 | Closed |
| RV-037 | bind 시 Credential을 고정256B+길이로 정규화하고 subtle2.6.1의 고정 크기 bytes/길이 비교를 사용한다 | primitive 부재 compile Red → Green. 정답·동일/다른 길이 오답·NUL padding·16/256/257 경계, 기존 TLS/read-manage/malformed auth/secret 비노출 회귀·credential Miri1 | Closed — 전체 인증 constant-time/공격 불가능 주장은 하지 않음 |
| RV-038 | Reassembler rustdoc·API §14.3에 연결별 소유/재생성·구독 gate 한도·old owner/ACK·예산 수명 명시. 자동 gate 삭제 미도입 | gate 한도 후 stale handoff 거절 유지·동일 gate 갱신·새 receiver와 기존 예산의 독립 수명, receiver3×8 seeds Miri24 | Closed — TASK-013 transport 소비자 후속 |

최종 source70개/검사9개 exit0·두 Rust1.85.0/1.96.0 각16feature·core/remote consumer·RT allocator11·clippy/rustdoc `-D warnings`·fmt·proto·Miri25 Pass. commit blob70개도 검증 hash와 일치한다. Miri는 기존 socket 전용 코드 제외에 따른 wire native 두 메서드 dead_code 경고가 있으며 native clippy/rustdoc 경고0과 구분한다.

새 package/version은 추가하지 않았다. 기존 TLS 간접 의존 subtle2.6.1을 remote 전용 직접 edge로 고정했다. 핵심 기술 계열·공개 인증 전략/오류/ACK 의미 변경이 아니므로 별도 ADR 승격 없이 API/tech_stack에 기록한다. RT 입력·unsafe·core 네트워크 격리는 유지한다. native 공개 후(T3)·LAN/실앱/RSS·PC/RT/UPS·Gate4/5는 미완료다.

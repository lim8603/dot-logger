# [REV-019] TASK-012 원격 어댑터 및 프로젝트 종합 심층 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-10-31 / AI ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md))
- **대상:** TASK-012 원격 어댑터(gRPC/ACK/관리) 구현 및 프로젝트 전체 형상 (커밋 [`5f0ad83`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/mod.rs) 기준)
  - `crates/dot-logger/src/adapters/remote/` (`mod.rs`, `types.rs`, `hub.rs`, `receiver.rs`, `server.rs`, `wire.rs`, `tests.rs`)
  - `crates/dot-logger/proto/logger.proto`
  - `crates/dot-logger/tests/tc_007_remote.rs`
  - 코어 수집 경로(`crates/dot-logger/src/capture/`) 및 RT 제약 격리 상태
- **종합 판정:** Pass (P2 권고 2건, P3 참고 1건)
  - **RT 경로 엄격 격리**: 원격 gRPC/어댑터(`adapters/remote`)는 `feature = "remote"`로 완전 격리되어 있으며, RT 수집 코어(`capture/`)의 무할당·무대기 제약을 전혀 침해하지 않음 (`tests/rt-allocation` 할당/해제 0/0 확인).
  - **동시성 및 메모리 안전성**: Tonic/Tokio 기반의 비동기 I/O와 원격 버퍼 예약(`QueryBudget`), 단조 `AtomicU8` 상태 머신(Pending → Acknowledged / Unknown / Dropped), 세대(`generation`) 기반 분기 보호가 엄격히 적용됨.
  - **소유권 및 취소 안전성(Cancellation Safety)**: 클라이언트 타임아웃 발생 시에도 호스트 I/O 커맨드(`CommandInbox`/`Command`)는 조기 반환되지 않고 실제 호스트 완료 시까지 리소스를 유지함.
  - **정적 분석 및 테스트 전체 통과**: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, 전체 단위/통합 테스트 24+19+13+21+19+10+14+3=123건 Pass.
- **상태:** Open (독립 코드리뷰 세션 생성 규정에 따라 Open 등록)

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-036 | P2 | [`server.rs:197-207`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/server.rs#L197-L207)<br>[`Incoming::poll_next`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/server.rs#L195-L211) | 최대 동시 TCP 연결 한도 도달 시 `wake_by_ref(); Poll::Pending`으로 인한 반복적 폴링(busy loop) 가능성 | 연결 한도 초과 시 즉시 재웨이크하는 대신 `permits` 세마포어 반환 시점에 웨이크하거나 백오프 지연 적용 권고 | 미판정 | Open |
| RV-037 | P2 | [`server.rs:42-47`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/server.rs#L42-L47)<br>[`equal`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/server.rs#L42-L47) | 토큰 비교 함수 `equal`에서 슬라이스 길이가 다를 경우 즉시 `false` 반환으로 인한 토큰 길이 타이밍 정보 누출 | 토큰 길이 차이에 대한 조기 반환을 배제하는 완전한 Constant-time 비교 로직 적용 권고 | 미판정 | Open |
| RV-038 | P3 | [`receiver.rs:216-226`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/receiver.rs#L216-L226)<br>[`Reassembler::discard_generations`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/receiver.rs#L203-L231) | 장기 실행 프로세스에서 다양한 subscription id가 유입될 때 `gates` 벡터가 `records` 한도에 도달하여 `RemoteError::Limit` 발생 가능 | 장시간 실행 시 수신자(`Reassembler`)의 수명 주기 지침 문서화 또는 사용하지 않는 오래된 subscription 항목 정리 방안 검토 | 미판정 | Open |

---

## 3. 상세 리뷰 내역

### RV-036: `Incoming::poll_next` 연결 한도 초과 시 비지 루프(busy spin) 완화 권고

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/adapters/remote/server.rs#L197-L207`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/server.rs#L197-L207)
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

#### [처리담당]
- (후속 개발 세션에서 판정 및 조치 예정)

---

### RV-037: 토큰 constant-time 비교 시 길이 정보 조기 반환 방어 권고

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/adapters/remote/server.rs#L42-L47`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/server.rs#L42-L47)
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

#### [처리담당]
- (후속 개발 세션에서 판정 및 조치 예정)

---

### RV-038: `Reassembler`의 generation gates 누적에 대한 수명 주기 관리 명확화

#### [리뷰어]
- **위치:** [`crates/dot-logger/src/adapters/remote/receiver.rs#L216-L226`](file:///home/lim8603/src/dot-logger/crates/dot-logger/src/adapters/remote/receiver.rs#L216-L226)
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

#### [처리담당]
- (후속 개발 세션에서 판정 및 조치 예정)

---

## 4. 종합 평가 및 다음 액션

1. **종합 평가:**
   - TASK-012의 원격 gRPC/ACK/관리 어댑터 구현은 승인된 인터페이스 계약(`§14.3`)과 P&A 아키텍처를 완벽히 충족하며, RT 코어에 대한 무영향성 및 리소스 한도 방어(EV-022)가 검증되었습니다.
   - 발견된 RV-036, RV-037, RV-038은 프로덕션 운영 환경에서의 가용성·보안 방어 심도를 높이기 위한 개선 권고사항(P2/P3)으로, 현재 기능 동작을 저해하는 Critical(P1) 결함은 없습니다.
2. **다음 액션:**
   - Human의 REV-018/REV-019 리뷰 확인 및 TASK-012 최종 수락 판정.
   - 승인 완료 시 후속 개발 작업인 [TASK-013](../04_implementation/tasks/TASK-013.md) (Rust 클라이언트 라이브러리 및 CLI 도구) 착수 준비.

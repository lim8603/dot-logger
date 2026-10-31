# [REV-022] TASK-013 클라이언트·CLI 및 TASK-012 후속 조치 독립 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-10-31 #009 / AI 리뷰 전용 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 리뷰. REV-020/021은 개발 세션 자가 점검이므로 같은 형상을 리뷰어 관점에서 다시 검토했다. 제품 코드는 수정하지 않았다.
- **대상 형상:** 소스 `29e55da` (HEAD `5f214ae`는 문서 변경만 포함)
  - TASK-013: `ded6372`·`29e55da`의 [client](../../../crates/dot-logger-client/src/) `cancel`·`receiver`·`transport`·`codec`·`offline`, [CLI](../../../crates/dot-logger-cli/src/) `main`·`args`·`commands`·`model`·`output`
  - TASK-012 후속: `4f79119`(RV-036/037/038 조치)의 [server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs)·[receiver.rs](../../../crates/dot-logger/src/adapters/remote/receiver.rs) diff 및 서버↔클라이언트 wire 경계
- **TASK-012 범위 판단:** `4f79119..29e55da` 사이에 `crates/dot-logger` 변경은 없다. TASK-012 본 구현(`5f0ad83`)은 REV-019에서 독립 리뷰를 마쳤으므로 다시 검토하지 않았다. REV-019 이후 자가 검증만 거친 조치 diff `4f79119`와 TASK-013이 사용하는 계약 경계만 포함했다.
- **정적 확인:** `cargo fmt --check` Pass, `cargo clippy -p dot-logger-client -p dot-logger-cli --all-targets --all-features -- -D warnings` Pass. 소스가 EV-025와 같으므로 테스트·Miri는 다시 실행하지 않았다. 아래 반례는 코드 정독으로 도출했고 실행 재현은 처리담당 Red 시험에 맡긴다.
- **종합 판정:** Pass (P2 1, P3 6). P1 없음. 데이터 손실, ACK 선행, RT 침해 결함은 확인하지 못했다.
- **상태:** Open

### 확인한 긍정 사항

- **RT 격리:** TASK-013은 core/RT/unsafe 소스를 변경하지 않았다. workspace `unsafe_code = "forbid"`가 client/CLI에 적용된다.
- **취소 lost-wake:** [`Cancellation::guard`](../../../crates/dot-logger-client/src/cancel.rs#L34)는 `Notified`를 먼저 poll해 등록한 뒤 flag를 확인한다. 경합 시 Cancelled/TimedOut 우선은 보수적이며, 원격 IO는 `completion_unconfirmed`로 표시된다.
- **연결 식별:** `ConnectionId(Arc<()>)`는 old owner가 살아 있는 동안 할당이 유지되어 포인터 재사용 충돌이 없다. one-shot connector는 재연결 시 오류를 반환한다.
- **ACK 순서:** CLI tail은 stdout `write_all`+`flush`가 성공한 뒤에만 consumer true를 반환하고 ACK를 보낸다. 출력 cap/IO 실패 시 ACK를 보내지 않는다.
- **TASK-012 RV-037 조치:** `Credential::matches`는 고정 256B 비교와 길이 `ct_eq`를 사용하며 stored length를 노출하지 않는다. 여러 token 중 `find`의 조기 종료로 일치한 index가 시간에 드러날 수 있지만, 이는 조치 문서가 밝힌 "primitive 보호, 전체 RPC 시간 특성 미주장"에 포함된다.

---

## 2. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 발견된 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-046 | P2 | [`RemoteSubscription::handoff`](../../../crates/dot-logger-client/src/transport.rs#L590-L593) | 만료·세대 폐기·소비자 거절을 모두 `ConsumerRejected`로 합치고, CLI는 이를 `io_or_transport`/exit4로 표시함 | 사전 거절과 소비자 거절을 구분하고 CLI 분류를 명시 | — | Open |
| RV-047 | P3 | [`Context::validate` tail](../../../crates/dot-logger-cli/src/commands.rs#L48-L66) | `external-bytes` 2GiB 상한과 `tail_plan` 검증이 연결 뒤에 실행되어 RV-039 유형의 입력 오류 가림이 남음 | tail 입력 검사를 모두 preflight로 이동 | — | Open |
| RV-048 | P3 | [local replay limit](../../../crates/dot-logger-cli/src/commands.rs#L381-L384) | 정확히 `limit`개 기록을 재생해도 Limit/exit3. query는 후속 기록이 있을 때만 Limit | 후속 기록 존재 여부로 판정하거나 의미를 문서화 | — | Open |
| RV-049 | P3 | [`Incoming::poll_next`](../../../crates/dot-logger/src/adapters/remote/server.rs#L228-L252) (TASK-012 `4f79119`) | 포화 시 초과 연결을 10ms에 1개씩만 거절해, permit이 반환된 뒤 backlog 뒤의 정상 연결이 약 N×10ms 지연될 수 있음 | wake당 유한 batch 거절 또는 trade-off 문서화·측정 | — | Open |
| RV-050 | P3 | [token `MetadataValue`](../../../crates/dot-logger-client/src/transport.rs#L263) | bearer 값이 sensitive로 표시되지 않음(HPACK 색인 대상, Debug 출력 가능) | parse 후 `set_sensitive(true)` 적용 | — | Open |
| RV-051 | P3 | [`path`](../../../crates/dot-logger-client/src/transport.rs#L196-L207)·[`codec::method`](../../../crates/dot-logger-client/src/codec.rs#L58) | 문자열 method 대응이 누락되면 조용히 GetStatus로 라우팅됨 | variant별 경로를 한 곳의 exhaustive match로 관리 | — | Open |
| RV-052 | P3 | [`recover`](../../../crates/dot-logger-cli/src/commands.rs#L410-L420) | open한 descriptor를 재검사하지 않고 path metadata 길이로 safe_offset을 비교함. `open_snapshot`의 RV-043 조치와 일관되지 않음 | 열린 파일 `metadata()`로 종류·길이 재검사 | — | Open |

---

## 3. 상세 리뷰 내역

### RV-046: 구독 handoff 실패 원인 소실과 CLI 오분류 (P2)

#### [리뷰어]
- **위치:** [transport.rs#L590-L593](../../../crates/dot-logger-client/src/transport.rs#L590-L593), [output.rs#L320-L335](../../../crates/dot-logger-cli/src/output.rs#L76-L92)
- **발견한 문제:** core [`ReceivedRecord::handoff`](../../../crates/dot-logger/src/adapters/remote/receiver.rs#L54-L64)는 ① 구독 세대가 폐기된 경우, ② TTL이 만료된 경우, ③ consumer가 false를 반환한 경우에 `Err(self)`를 반환한다. ①②는 consumer를 호출하지 않는다. client는 `ClientReceiver::handoff`가 돌려준 `PendingRecord`를 버리고 세 경우 모두 `ClientError::ConsumerRejected`("consumer rejected the record")로 바꾼다.
- **반례:**
  1. 라이브러리: `next()`로 Record를 받은 뒤 `discard_generations(g+1)`를 호출하고 `handoff`를 호출하면 consumer는 실행되지 않았는데 결과가 `ConsumerRejected`다. host 화면 연동이 이 오류를 "UI가 거절함"으로 처리할 수 있다.
  2. CLI tail: consumer 실패는 `failure`로 먼저 반환하므로, CLI에 도달하는 `ConsumerRejected`는 항상 ①② 사전 거절이다. 그런데 `From<ClientError>`의 `_ => Io` 분기 때문에 `io_or_transport`/exit4로 표시된다. RV-045가 만료/부재를 `gone_or_expired`/exit3으로 분리한 원칙과 어긋난다.
- **처리 제안:**
  - `RemoteSubscription::handoff`에서 consumer 호출 여부를 기록해 사전 거절(만료·세대)과 소비자 거절을 구분한다. 예: consumer를 감싼 flag, 또는 `Remote(Gone)`/별도 variant. 공개 오류 variant를 추가하면 API §14.4·SDK README도 함께 갱신한다.
  - CLI `From<ClientError>`에 `ConsumerRejected`·`Disconnected`·`StaleConnection`을 명시적으로 대응시켜 `_` 분기에 기대지 않는다.
  - Red: (a) 세대 폐기 후 handoff 시 consumer 미호출과 `ConsumerRejected`가 아닌 결과, (b) 짧은 TTL 경과 후 handoff 시 같은 결과, (c) 기존 소비자 거절은 계속 `ConsumerRejected`·ACK 미전송.
- **심각도 근거:** ACK·데이터 안전성은 유지된다(어느 경우에도 ACK 없음). 그러나 공개 API 진단과 CLI exit 계약이 틀리므로 P2로 판단했다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-047: tail 입력 검사가 연결 뒤로 밀림 (P3)

#### [리뷰어]
- **위치:** [commands.rs#L48-L66](../../../crates/dot-logger-cli/src/commands.rs#L48-L66) vs [commands.rs#L568-L576](../../../crates/dot-logger-cli/src/commands.rs#L568-L576)
- **발견한 문제:** `validate`는 `external-bytes` ≤ 2GiB를 확인하지 않는다. `remote()`는 `RemoteClient::connect` 뒤에 이 상한과 `tail_plan(external)` SessionBuilder 검증을 수행한다.
- **반례:** `--external-bytes 3221225472`(3GiB)이고 endpoint에 연결할 수 없으면, 입력 오류(exit2) 대신 연결 실패 `io_or_transport`/exit4가 나온다. 연결에 성공하면 불필요한 연결을 연 뒤 exit2가 된다.
- **처리 제안:** 2GiB 상한과 `tail_plan` 검증을 `validate`의 tail 분기로 옮긴다. `remote()`의 중복 검사는 방어용으로 남겨도 된다. Red: 연결할 수 없는 endpoint와 3GiB 입력 → 기대 exit2.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-048: local replay의 정확한 limit 도달을 Limit으로 판정 (P3)

#### [리뷰어]
- **위치:** [commands.rs#L381-L384](../../../crates/dot-logger-cli/src/commands.rs#L381-L384), 비교 [commands.rs#L314](../../../crates/dot-logger-cli/src/commands.rs#L314)
- **발견한 문제:** query는 `limit` 도달 시 남은 행이나 다음 cursor가 있을 때만 Limit이다. replay는 `out.records == limit`이면 다음이 End여도 즉시 stop/Limit(exit3)이다. 기본 `--limit 1000`에서 정확히 1000건인 범위를 재생하면 완료가 아니라 limit으로 보고된다.
- **처리 제안:** 다음 `poll` 결과가 End/Stopped인지 확인한 뒤 판정하거나(대기 시간이 필요한 Ready면 Limit), 현재 의미를 ui_spec/README에 "limit 도달 = 부분 결과"로 명시한다. remote tail은 `stopped:"record_limit"`으로 성공 처리하므로 세 명령의 limit 의미를 함께 정리할 것을 권한다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-049: 포화 admission 냉각으로 거절 처리량이 100건/s에 고정됨 (P3, TASK-012 후속)

#### [리뷰어]
- **위치:** [server.rs#L228-L252](../../../crates/dot-logger/src/adapters/remote/server.rs#L228-L252)
- **발견한 문제:** RV-036 조치로 busy spin은 해소됐다. 다만 초과 연결 하나를 accept→drop할 때마다 10ms timer를 기다리므로 거절 처리량이 약 100건/s로 제한된다. 초과 유입이 그보다 많으면 kernel backlog가 차고, permit이 반환된 뒤 backlog 뒤에 있는 정상 연결은 앞선 N개 거절이 끝날 때까지(약 N×10ms, backlog 1024이면 약 10초) 기다린다.
- **영향 범위:** 기본 loopback/disabled 구성에서는 영향이 작다. 연결 한도·거절 의미는 유지되며 측정한 결과는 아니다.
- **처리 제안:** wake마다 유한 개수(예: 32~64)의 초과 연결을 거절한 뒤 냉각에 들어가도록 해서 CPU 상한과 backlog 회복을 함께 확보한다. 또는 이 trade-off를 문서에 명시하고 TASK-022 실측 항목에 포화 회복 시간을 추가한다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-050: bearer metadata의 sensitive 미표시 (P3)

#### [리뷰어]
- **위치:** [transport.rs#L257-L265](../../../crates/dot-logger-client/src/transport.rs#L257-L265)
- **발견한 문제:** `format!("Bearer {t}").parse::<MetadataValue<Ascii>>()` 결과에 `set_sensitive(true)`를 호출하지 않는다. h2는 sensitive가 아닌 header를 HPACK 동적 테이블에 색인할 수 있고, `MetadataMap`/`Request`의 Debug 출력에 값이 그대로 나타난다. 현재 client는 Debug를 출력하지 않으며 TLS가 필수(비 loopback)이므로 위험은 낮다.
- **처리 제안:** parse 직후 `set_sensitive(true)`를 적용한다. 단위 시험에서 `is_sensitive()`를 확인한다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-051: 문자열 method 라우팅의 조용한 fallback (P3)

#### [리뷰어]
- **위치:** [transport.rs#L196-L207](../../../crates/dot-logger-client/src/transport.rs#L196-L207), [codec.rs#L58-L69](../../../crates/dot-logger-client/src/codec.rs#L58-L69), [codec.rs#L194-L296](../../../crates/dot-logger-client/src/codec.rs#L194-L296)
- **발견한 문제:** `codec::method`는 exhaustive match지만 `path()`는 `_ => GetStatus`로 대응한다. 새 `ManagementRequest` variant를 추가하면서 `path()`를 갱신하지 않으면 요청이 GetStatus RPC로 전송된다. 응답 kind 불일치로 Corrupt가 되어 실패 쪽으로 닫히지만, 원인이 드러나지 않고 컴파일러도 누락을 잡지 못한다.
- **처리 제안:** method 이름·RPC 경로·기대 reply kind를 한 곳의 exhaustive match(또는 private enum)에서 관리한다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-052: recover의 열린 descriptor 재검사 누락 (P3)

#### [리뷰어]
- **위치:** [commands.rs#L410-L420](../../../crates/dot-logger-cli/src/commands.rs#L410-L420), 비교 [offline.rs#L53-L67](../../../crates/dot-logger-client/src/offline.rs#L53-L67)
- **발견한 문제:** RV-043 조치로 `open_snapshot`은 open 전 path 검사와 open 뒤 `file.metadata()` 재검사를 모두 수행한다. `recover`는 path `metadata`만 확인하고, `safe_offset == len` 비교도 path 기준으로 한다. 경로 불변성은 host 책임이라는 계약은 유지되지만, 두 진입점의 방어 수준이 다르다.
- **처리 제안:** open한 `file.metadata()`로 `is_file`·길이를 다시 확인하고 safe_offset을 비교한다. 참고로 open 자체가 FIFO 교체로 막히는 잔여 위험은 두 경로 모두 남아 있으며, CLI의 `spawn_blocking`은 취소할 수 없어 deadline을 넘겨 대기할 수 있다. Unix `O_NONBLOCK` open 후 정규 파일 확인 같은 보강을 선택 검토한다(필수 아님).

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

---

## 4. 파일 변경표 (리뷰 대상)

| 구분 | 파일 | 검토 내용 |
|---|---|---|
| 생성 | [client transport.rs](../../../crates/dot-logger-client/src/transport.rs)·[codec.rs](../../../crates/dot-logger-client/src/codec.rs)·[cancel.rs](../../../crates/dot-logger-client/src/cancel.rs)·[offline.rs](../../../crates/dot-logger-client/src/offline.rs)·wire.rs | 연결/TLS 검증, 취소, native 변환, read-only 파일 — RV-046/050/051 |
| 생성 | [client receiver.rs](../../../crates/dot-logger-client/src/receiver.rs) | 연결 식별·handoff/take_ack — 이상 없음 |
| 생성 | [CLI commands.rs](../../../crates/dot-logger-cli/src/commands.rs)·[main.rs](../../../crates/dot-logger-cli/src/main.rs)·[output.rs](../../../crates/dot-logger-cli/src/output.rs)·[model.rs](../../../crates/dot-logger-cli/src/model.rs)·args.rs | preflight·limit·출력 cap·종료 분류·복구 — RV-046/047/048/052 |
| 수정 | [server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs)·[receiver.rs](../../../crates/dot-logger/src/adapters/remote/receiver.rs) (`4f79119`) | 포화 admission 냉각·고정 길이 credential 비교·gate 수명 문서 — RV-049 |

## 5. 클래스(타입) 검토표

| 구분 | 소속 | 타입 | 검토 결과 |
|---|---|---|---|
| 생성 | client transport | RemoteClient·RemoteSubscription | one-shot 연결/close 전파 양호. handoff 오류 분류(RV-046), token sensitive(RV-050) |
| 생성 | client transport(private) | ConnectionState·ConnectionIo | close/waker 정리 양호 |
| 생성 | client cancel | Cancellation | 등록 후 flag 확인 순서 양호 |
| 생성 | client receiver | ClientReceiver·ConnectionId·PendingRecord·Acknowledgement | Arc 식별·stale 거절 양호 |
| 생성 | CLI | Context·Output·Failure | preflight 누락(RV-047), `_ => Io` fallback(RV-046) |
| 수정 | core remote(private) | Incoming·Credential | 냉각 처리량(RV-049), 비교 primitive 양호 |

## 6. 다음 액션

1. 후속 개발 세션에서 RV-046을 먼저 조치한다. Red → Green → 기존 transport7/CLI10 회귀 순서로 진행하며, 공개 오류 variant를 바꾸면 API §14.4를 함께 갱신한다.
2. RV-047~052는 같은 세션에서 묶어 처리하거나, 판정(수용/보류/기각)과 근거를 처리담당 영역에 기록한다. RV-049는 TASK-022 실측 항목과 연결할 수 있다.
3. 이 리뷰는 native·실앱·PC 실측의 수락을 뜻하지 않는다. TASK-013 상태(로컬 완료/native T3 이월)는 바꾸지 않는다.

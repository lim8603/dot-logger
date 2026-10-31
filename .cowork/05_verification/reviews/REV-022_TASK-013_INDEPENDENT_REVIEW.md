# [REV-022] TASK-013 클라이언트·CLI 및 TASK-012 후속 조치·서버 경계 독립 코드리뷰

## 1. 개요

- **일자 / 리뷰어:** 2026-10-31 #009 / AI 리뷰 전용 세션 ([code-review 스킬](../../../.agents/skills/code-review/SKILL.md), [§1E](../../01_cowork_protocol/session_protocol.md#1e-리뷰-세션-및-코드리뷰-절차-review-session-protocol))
- **성격:** 독립 Read-Only 리뷰. REV-020/021은 개발 세션 자가 점검이므로 같은 형상을 리뷰어 관점에서 다시 검토했다. 제품 코드는 수정하지 않았다.
- **대상 형상:** 소스 `29e55da` (HEAD `5f214ae`는 문서 변경만 포함)
  - TASK-013: `ded6372`·`29e55da`의 [client](../../../crates/dot-logger-client/src/) `cancel`·`receiver`·`transport`·`codec`·`offline`, [CLI](../../../crates/dot-logger-cli/src/) `main`·`args`·`commands`·`model`·`output`
  - TASK-012 후속: `4f79119`(RV-036/037/038 조치)의 [server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs)·[receiver.rs](../../../crates/dot-logger/src/adapters/remote/receiver.rs) diff 및 서버↔클라이언트 wire 경계
- **TASK-012 범위 판단:** `4f79119..29e55da` 사이에 `crates/dot-logger` 변경은 없다. TASK-012 본 구현(`5f0ad83`)은 REV-019에서 독립 리뷰를 마쳤으므로 전체를 다시 검토하지 않았다. REV-019 이후 자가 검증만 거친 조치 diff `4f79119`와 TASK-013이 사용하는 계약 경계만 포함했다.
- **경계 확장 (Human 선택 A, 같은 세션):** TASK-013이 실제로 호출하는 서버 경로를 클라이언트 사용 패턴과 대조했다. 대상은 [server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs) `RpcService`·`UnaryRpc`·`AckRpc`·`SubscribeRpc`·`SubscriptionStream`·`status`, [hub.rs](../../../crates/dot-logger/src/adapters/remote/hub.rs) `Subscription::next/ack/expire`·`offer` Gap·`request`/`Command`, [receiver.rs](../../../crates/dot-logger/src/adapters/remote/receiver.rs) `push`/`handoff`, core `Replay::seek` generation이다. RV-053~055가 여기서 나왔다. `types`·`wire` 변환 전체와 원격 통합 시험은 다시 검토하지 않았다.
- **정적 확인:** `cargo fmt --check` Pass, `cargo clippy -p dot-logger-client -p dot-logger-cli --all-targets --all-features -- -D warnings` Pass. 소스가 EV-025와 같으므로 테스트·Miri는 다시 실행하지 않았다. 아래 반례는 코드 정독으로 도출했고 실행 재현은 처리담당 Red 시험에 맡긴다.
- **종합 판정:** Pass (P2 2, P3 8). P1 없음. 데이터 손실, ACK 선행, RT 침해 결함은 확인하지 못했다.
- **상태:** Open

### 확인한 긍정 사항

- **RT 격리:** TASK-013은 core/RT/unsafe 소스를 변경하지 않았다. workspace `unsafe_code = "forbid"`가 client/CLI에 적용된다.
- **취소 lost-wake:** [`Cancellation::guard`](../../../crates/dot-logger-client/src/cancel.rs#L34)는 `Notified`를 먼저 poll해 등록한 뒤 flag를 확인한다. 경합 시 Cancelled/TimedOut 우선은 보수적이며, 원격 IO는 `completion_unconfirmed`로 표시된다.
- **연결 식별:** `ConnectionId(Arc<()>)`는 old owner가 살아 있는 동안 할당이 유지되어 포인터 재사용 충돌이 없다. one-shot connector는 재연결 시 오류를 반환한다.
- **ACK 순서:** CLI tail은 stdout `write_all`+`flush`가 성공한 뒤에만 consumer true를 반환하고 ACK를 보낸다. 출력 cap/IO 실패 시 ACK를 보내지 않는다.
- **TASK-012 RV-037 조치:** `Credential::matches`는 고정 256B 비교와 길이 `ct_eq`를 사용하며 stored length를 노출하지 않는다. 여러 token 중 `find`의 조기 종료로 일치한 index가 시간에 드러날 수 있지만, 이는 조치 문서가 밝힌 "primitive 보호, 전체 RPC 시간 특성 미주장"에 포함된다.
- **서버↔클라이언트 계약 일치 (경계 확장):**
  - replay generation: core는 `seek`만 generation을 +1 하며, 클라이언트 RV-042 precondition(seek=+1, 그 외 동일)과 맞는다.
  - `late` 플래그: 서버는 첫 미전송 기록이 다 보내질 때까지 다른 기록 조각을 끼워 넣지 않는다. 그래서 같은 기록의 조각들이 같은 `late` 값을 가지며, receiver의 조각 간 header 일치 검사와 충돌하지 않는다.
  - ACK: 서버 `peer` 일치 검사는 one-shot 단일 연결 ACK와 맞는다.
  - Gap: 큐에 있던 기록을 먼저 보낸 뒤 연속 범위로 Gap을 보내는 순서가 클라이언트 `gap_reason==1`·범위 검사와 맞는다.
  - 크기 상한: ReadBlob 길이는 host `respond`와 클라이언트가 모두 검사한다. Page 항목 1000개 상한도 양쪽이 같다.
  - 만료 시 ACK: 서버 만료 뒤 ACK가 Gone으로 거절되면 클라이언트는 `AcknowledgementUnknown`을 반환하고, 서버도 Unknown으로 처리한다. 양쪽 모두 성공을 주장하지 않는다.

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
| RV-053 | P2 | [`SubscriptionStream` idle lease](../../../crates/dot-logger/src/adapters/remote/server.rs#L446-L450)·[`RemoteSubscription::next`](../../../crates/dot-logger-client/src/transport.rs#L525-L538)·[`ReceiverConfig::ttl`](../../../crates/dot-logger-client/src/receiver.rs#L18-L19) | heartbeat가 없어 조용한 구독은 min(client timeout, ttl) 뒤 종료된다. 다시 구독하면 그 사이 기록이 Gap 없이 보이지 않는다. client 문서는 ttl이 서버 idle lease를 겸한다는 점을 밝히지 않는다 | idle 대기와 기록 deadline 분리, 또는 heartbeat/lease 문서화 (프로토콜 변경은 Human 판단) | — | Open |
| RV-054 | P3 | [server `status`](../../../crates/dot-logger/src/adapters/remote/server.rs#L95-L106)·[client `rpc_error`](../../../crates/dot-logger-client/src/transport.rs#L148-L161) | host Corrupt가 InvalidArgument를 거쳐 client InvalidInput(exit2)이 되고, tonic 크기 상한 OutOfRange는 Transport(exit4)가 됨. 대응표가 계약 문서에 없음 | 손실 없는 status 대응과 API 문서화 | — | Open |
| RV-055 | P3 | [`RemoteHub::request`](../../../crates/dot-logger/src/adapters/remote/hub.rs#L524-L527)·[CLI posted_host_io](../../../crates/dot-logger-cli/src/output.rs#L166-L172) | host가 응답 없이 Command를 Drop하면 Gone이 된다. 변경 명령(ApplyConfig/RunCheck)이 이 경우 `gone_or_expired`로 표시되고 `completion_unconfirmed` 표시가 빠짐 | 응답 없는 Drop을 Gone과 구분하거나, 변경 명령의 Gone에 불명 표시 추가 | — | Open |

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

### RV-053: 조용한 구독의 강제 종료와 재구독 사이의 보이지 않는 누락 (P2, 서버 경계)

#### [리뷰어]
- **위치:** [server.rs#L446-L450](../../../crates/dot-logger/src/adapters/remote/server.rs#L446-L450), [transport.rs#L525-L538](../../../crates/dot-logger-client/src/transport.rs#L525-L538), [transport.rs#L421-L436](../../../crates/dot-logger-client/src/transport.rs#L421-L436), [receiver.rs#L18-L19](../../../crates/dot-logger-client/src/receiver.rs#L18-L19)
- **발견한 문제:**
  1. 서버 `SubscriptionStream`은 event가 없는 상태로 `age`가 지나면 stream을 `TimedOut`으로 끝낸다. 이 동작은 API 계약에 "idle lease"로 적혀 있다. 클라이언트는 `ReceiverConfig::ttl`을 그대로 `age_ms`로 보내는데, `ttl` 문서는 "incomplete/unaccepted record age"라고만 설명한다. 그래서 수신 TTL을 짧게 잡으면 서버 idle lease도 함께 짧아진다는 사실을 호출자가 알 수 없다.
  2. 클라이언트 `next()`는 `RemoteConfig::timeout`(기본 5초)을 다음 기록 deadline으로 쓰고, 오류가 나면 구독을 닫는다. 기록이 드문 채널이라면 기록이 오지 않은 것만으로 구독이 끝난다.
  3. 프로토콜에 heartbeat frame이 없으므로, 지속 tail(dot-probe 실시간 화면)은 재구독 루프에 기댈 수밖에 없다. 새 구독은 transfer 번호가 1부터 새로 시작하고 이전 구독과 이어지지 않는다. 따라서 끊긴 사이 host `offer`가 `Gone`이 된 기록은 클라이언트에 Gap으로 나타나지 않는다. host는 실패를 알지만 수신 측 화면은 누락을 표시할 수 없다.
- **반례:** 기록 간격이 6초인 채널을 기본 설정으로 구독한다. `next()`가 5초에 TimedOut을 반환하고 구독이 닫힌다. 재구독하기 전에 offer된 기록은 클라이언트 쪽에 Gap 없이 사라진다.
- **처리 제안 (택1 또는 조합):**
  - (a) 클라이언트: `next()`의 TimedOut이 구독을 닫지 않게 한다. tonic `Streaming::message`의 cancel-safety를 확인한 뒤 시험으로 고정한다. 또는 idle 대기 상한을 기록 deadline과 분리한 설정으로 둔다.
  - (b) 문서: `ReceiverConfig::ttl`·README·API §14.4에 "ttl = 서버 idle lease 겸 미수락 기록 수명"과 "재구독 간 누락은 Gap으로 표시되지 않음"을 명시한다.
  - (c) 프로토콜: idle heartbeat frame 또는 lease 분리 필드를 추가한다. wire/계약 변경이므로 H/J 판단 사항이며, ADR 승격 기준을 검토한다.
- **심각도 근거:** ACK·데이터 무결성은 지켜진다. 다만 계획된 실시간 화면 사용(US-005)에서 누락이 표시되지 않을 수 있고, 공개 설정의 의미가 숨어 있으므로 P2로 판단했다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-054: gRPC status 대응의 정보 손실과 계약 문서 부재 (P3, 서버 경계)

#### [리뷰어]
- **위치:** [server.rs#L95-L106](../../../crates/dot-logger/src/adapters/remote/server.rs#L95-L106), [transport.rs#L148-L161](../../../crates/dot-logger-client/src/transport.rs#L148-L161)
- **발견한 문제:**
  - 서버는 `Corrupt`와 `InvalidInput`을 모두 `invalid_argument`로 보낸다. 클라이언트는 이를 `InvalidInput`으로 받으므로, host가 원본 손상을 `Corrupt`로 응답해도 CLI는 `invalid_input`/exit2(사용자 입력 오류)로 표시한다.
  - 메시지가 tonic decode/encode 상한을 넘으면 `OutOfRange`가 되고, 클라이언트 `_ => Transport`를 거쳐 `io_or_transport`/exit4가 된다. 예를 들어 서버 `message_bytes`가 클라이언트 `--message-bytes`보다 크면 큰 Page 응답이 Limit이 아니라 IO 실패로 보인다. Query 요청에는 클라이언트 상한을 서버에 알릴 필드가 없다.
  - status 대응표는 [interface_contract](../../03_design_artifacts/interface_contract.md)에 없다.
- **처리 제안:** 서버 `Corrupt → data_loss`, 클라이언트 `DataLoss → Corrupt`·`OutOfRange → Limit`로 대응시킨다. 기존 `invalid_argument`는 계속 `InvalidInput`으로 받아 하위 호환을 유지한다. 대응표를 API 계약에 적는다. 크기 협상은 후속 판단(P3 이하)으로 둔다.

#### [처리담당]
- 판정 결과:
- 처리 내용:
- 검증 근거:

### RV-055: 응답 없는 Command Drop과 변경 명령의 완료 불명 표시 누락 (P3, 서버 경계)

#### [리뷰어]
- **위치:** [hub.rs#L524-L527](../../../crates/dot-logger/src/adapters/remote/hub.rs#L524-L527), [output.rs#L166-L172](../../../crates/dot-logger-cli/src/output.rs#L166-L172)
- **발견한 문제:** host가 `respond` 없이 `Command`를 Drop하면 oneshot이 닫혀 `Gone`이 된다. host panic이나 계약 위반이 이런 경우다. 그런데 `Gone`은 만료된 snapshot/blob capability에도 쓰인다. CLI는 `Gone`을 `gone_or_expired`/exit3으로 표시하고 `posted_host_io: completion_unconfirmed`를 붙이지 않는다. 그래서 `config apply`/`check run`이 실제로 일부 적용됐을 수 있는데도 "적용 여부 불명"이라는 표시가 빠진다.
- **처리 제안:** hub가 응답 없는 Drop을 `Transport`(또는 별도 불명 의미)로 반환해 capability `Gone`과 구분한다. 또는 CLI가 변경 명령의 `Gone`에도 `completion_unconfirmed`를 붙인다. Red: 시험 host가 ApplyConfig Command를 respond 없이 Drop → 기대 불명 표시.

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
| 기존 (경계 확장) | [server.rs](../../../crates/dot-logger/src/adapters/remote/server.rs) RPC/stream·[hub.rs](../../../crates/dot-logger/src/adapters/remote/hub.rs) 구독/ACK/command | 클라이언트 사용 패턴과 대조 — RV-053/054/055 |

## 5. 클래스(타입) 검토표

| 구분 | 소속 | 타입 | 검토 결과 |
|---|---|---|---|
| 생성 | client transport | RemoteClient·RemoteSubscription | one-shot 연결/close 전파 양호. handoff 오류 분류(RV-046), token sensitive(RV-050) |
| 생성 | client transport(private) | ConnectionState·ConnectionIo | close/waker 정리 양호 |
| 생성 | client cancel | Cancellation | 등록 후 flag 확인 순서 양호 |
| 생성 | client receiver | ClientReceiver·ConnectionId·PendingRecord·Acknowledgement | Arc 식별·stale 거절 양호 |
| 생성 | CLI | Context·Output·Failure | preflight 누락(RV-047), `_ => Io` fallback(RV-046) |
| 수정 | core remote(private) | Incoming·Credential | 냉각 처리량(RV-049), 비교 primitive 양호 |
| 기존 | core remote(private) | SubscriptionStream·Subscription·RpcService | idle lease/heartbeat 부재(RV-053), status 대응(RV-054). late·Gap·ACK peer 계약은 일치 |
| 기존 | core remote | RemoteHub·Command | 응답 없는 Drop의 Gone 처리(RV-055) |

## 6. 다음 액션

1. 후속 개발 세션에서 RV-046을 먼저 조치한다. Red → Green → 기존 transport7/CLI10 회귀 순서로 진행하며, 공개 오류 variant를 바꾸면 API §14.4를 함께 갱신한다.
2. RV-053은 처리 방식에 따라 권한이 다르다. (a) 클라이언트 동작과 (b) 문서는 기존 위임 안에서 조치할 수 있다. (c) heartbeat/lease 프로토콜 변경은 Human 판단을 받은 뒤 진행한다.
3. RV-047~052·054·055는 같은 세션에서 묶어 처리하거나, 판정(수용/보류/기각)과 근거를 처리담당 영역에 기록한다. RV-054의 status 대응 변경은 API 계약 문서도 함께 갱신한다. RV-049는 TASK-022 실측 항목과 연결할 수 있다.
4. 이 리뷰는 native·실앱·PC 실측의 수락을 뜻하지 않는다. TASK-013 상태(로컬 완료/native T3 이월)는 바꾸지 않는다.

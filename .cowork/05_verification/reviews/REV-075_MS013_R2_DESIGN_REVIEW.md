# REV-075 — MS-013 r2 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-27 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r2](../../04_implementation/milestones/MS-013.md), [REV-075 요청서](REV-075_REQUEST.md), [REV-074](REV-074_MS013_OUTPUT_DESIGN_REVIEW.md) |
| 기준 | r2 `d9c2da71daf3b39439b3fdd1892078a1b4098110`, 검토 시 HEAD `b108c56`. r1 `da5f6c9` |
| 판정 / 상태 | **Conditional / Open** |

## 1. 요약정리

**P1 4건, P2 5건, P3 1건.** r2는 원본을 ACK 대기에서 분리하고, DB 수락 범위를 단계 6으로 분명히 하며, 설정 병합표와 기존 동작 기준선을 추가했다. 착수 전에 host의 완료·원본 회수 한도, 손실 회계의 최종 정산, 스위치 경계, 파일 성공과 네트워크 종료의 기한을 확정해야 한다. 코드·문서·기존 EV를 정적으로 대조했다. 시험과 성능 재측정은 요청 범위 밖이다.

## 2. REV-074 반영 대조

| 이전 지적 | r2 절 및 대조 결과 |
|---|---|
| RV-213 | §5.1에서 host가 네트워크 reader를 보유하고 복사하는 구조는 기존 소유권 API로 구성 가능하다. 완료 경합과 원본 재사용·파일 부하 격리는 RV-225. [Router::scope](../../../crates/dot-logger/src/routing/mod.rs#L707) |
| RV-214 | §5.4의 원본·링·구독 단위 분리는 다중 구독 곱셈 오류를 없앴다. 실패·스냅샷·최종 정산은 RV-226, 단절 감지와 ACK 창은 RV-229. |
| RV-215 | §5.6은 host 복사를 기준으로 잡고 opt-in wire 표시를 제안했다. 위치 토큰의 의미와 명령·표시 경합은 RV-227, lease와 호환 조합은 RV-229. |
| RV-216 | §5.9에서 ACK와 파일 sync 대상을 분리했다. outer shutdown 및 선택 출력 정리 기한은 RV-228. [core Closed](../../../crates/dot-logger/src/control/mod.rs#L847) |
| RV-217 | §5.3의 링 복사 완료는 target당 1회라 fanout 결과와 분리된다. 새 경계·정책 생략의 API와 wire 영향은 RV-230. |
| RV-218 | §5.10에 퇴역 상세 8개, 예산 경계 시험, 선택 손실 파일이 추가됐다. 예약 계산과 보장 범위는 RV-231. |
| RV-219 | §5.2가 단일 network, legacy tap 충돌, 파일 순번 및 기본 경로를 명시했다. 채택은 H4·H5. |
| RV-220 | FR-019a/b를 나누고 DB는 단계 6에서 판정한다. 파일 없는 `Plan` 및 이벤트 API 표현은 RV-234. |
| RV-221 | §5.5에 병합·오류·feature 시험, 출처를 분리한 `explain()`이 생겼다. 명시적 구성 교체와 인덱스 안정성은 RV-232, 공개 범위는 RV-230. |
| RV-222 | §8.2에 수신 고유 건수·바이트와 큐 배출 대조가 들어갔다. legacy tap 기준선에 필요한 관측점은 RV-233. |
| RV-223 | §7에서 레거시 골든, 코어 변경, facade Red를 분리했다. 단계 1에서 공개 enum이 바뀌는 점과 공개 표면 검사는 RV-233. |
| RV-224 | 기본 구독 바이트 하한, OFF 중 payload 복사·분배 0, `subscribers` 필드가 반영됐다. 구독 수를 어느 스레드의 관측으로 확정할지는 RV-227. |

## 3. 리뷰 포인트

| ID | 중요도 | 위치 | 문제 | 처리 제안 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-225 | **P1** | §5.1, §7~8 | 같은 턴에 보유를 끝내고 파일 수락을 동일하게 유지한다는 보장이 현재 API와 턴 구조에서 성립하지 않음 | 복사·완료 정산 한도와 자원 재사용 시험을 단계 0~1에 둠 | 대기 | Open |
| RV-226 | **P1** | §5.4, §5.9 | 중단 시 링과 구독 사이 유실이 보존식에서 빠지고 무ACK도 무손실로 보고될 수 있음 | 종결 상태·스냅샷·수신 확인 수준 정의 | 대기 | Open |
| RV-227 | **P1** | §5.6 | 최초 전송 seq를 토글 시각에 알 수 없고 명령·표시의 적용 순서가 비어 있음 | 컷오프 토큰, 명령 상태 전이, FIFO 제어 배리어 | 대기 | Open |
| RV-228 | **P1** | §5.9 | 파일을 마친 뒤 네트워크를 기다리면 파일 성공 또는 facade 기한을 지킬 수 없음 | 파일과 네트워크 종료 병행, 선택 출력 정리 최종화 규약 | 대기 | Open |
| RV-229 | P2 | §5.3~6 | heartbeat로 단절 상한을 보장할 수 없고 ACK 기본 모드의 기존 client 동작이 미정 | 호환 ACK/lease와 wire 협상 계약 | 대기 | Open |
| RV-230 | P2 | §5.3, §6 | Handed 관리 wire·CLI와 정책 생략의 손실 분류가 변경 목록에서 빠짐 | 숫자 호환 및 loss schema 영향 명시 | 대기 | Open |
| RV-231 | P2 | §1.3, §5.10 | 구독 예산 중복 곱셈, 가변 marker와 런타임 보장 범위 미정 | checked 산식과 예산 시험 범위 고정 | 대기 | Open |
| RV-232 | P2 | §5.5, H4~H5 | 옵션 파일로 기존 코드의 출력을 제거하거나 바꿀 규칙이 부족함 | 구성 교체 및 순번·경로 계약 | 대기 | Open |
| RV-233 | P2 | §6~8 | legacy 측정에 없는 관측값을 요구하고 공개 호환 게이트 추정이 빠듯함 | 공통 측정점, 표면 스냅샷 및 조기 게이트 | 대기 | Open |
| RV-234 | P3 | §5.8, §6~7 | 파일 없는 구성의 `Plan.sync_every` 및 이벤트 오류가 모호함 | 유지할 공개 타입의 의미를 명시 | 대기 | Open |

### RV-225 — 같은 턴 복사와 완료·파일 격리는 별도 보장

- **[리뷰어]** `Router::scope`는 dispatcher와 reader를 따로 빌려주므로 파일 reader만 IO 스레드로 보내고 host가 network reader를 남길 수 있다. 그러나 `TargetConsumer::try_next()`는 feedback 큐가 차면 읽지 않으며, `Delivery::report()`는 동일 receipt를 다른 파일 출력이 갱신 중일 때도 `Busy`를 반환할 수 있다. 기존 IO helper는 최대 1 s 동안 재시도한다. 이를 host에 쓰면 파일 분배까지 지연되며, guard를 바로 떨어뜨리면 Handed가 아닌 Unknown 경로로 간다. raw의 최종 guard drop과 RT 재획득 가능 시각도 구분해야 한다. 현재 raw reclaim은 turn 앞에서 수행한다. 경합이 없어도 파일 전용 다음 턴은 즉시 진행할 수 있는 시점에 혼합 모드는 바이트 복사를 더 수행하므로 포화점에서 `Capacity` 개수 동일을 일반 보장할 수 없다. [Delivery::report](../../../crates/dot-logger/src/routing/mod.rs#L181), [try_next](../../../crates/dot-logger/src/routing/mod.rs#L1180), [turn reclaim](../../../crates/dot-logger/src/routing/mod.rs#L1042), [IO retry](../../../crates/dot-logger/src/logger/host.rs#L1035)
- **처리 제안** — 반환과 완료 발행을 분리할 경우 payload 없는 bounded feedback의 예약·재시도·중복 방지·포화 정산을 설계한다. 다른 접근은 Router가 분배하는 자리에서 bounded copy를 하고 기존 dispatcher feedback으로 완료를 기록하는 방식이다. 어떤 경우든 비차단 링, 턴별 복사 건수·바이트·작업 한도, 파일 우선 순서와 raw reclaim 지점을 단계 0~1에서 확인한다. 느린 네트워크가 만드는 장기 보유와 host 복사 자체의 부하를 대조군으로 나눠, raw 2 슬롯·Important·completion Busy·파일 포화에서 검증한다. 파일 격리의 수치 허용치는 Human과 합의한다.
- **[처리담당]** 대기.

### RV-226 — 손실 보존식은 정상 fanout 뒤에도 닫혀야 함

- **[리뷰어]** 링 복사 1건 직후 worker가 실패하거나 drain 기한이 끝나면 해당 기록은 `handed=1`이면서 발행·구독 없음 어느 상태에도 속하지 않는다. 최종 보고에서 `ring_pending`을 0으로 만들면 L2의 등식이 깨진다. 부분 fanout에서 worker가 멈추면 아직 offer하지 못한 구독의 건수·바이트도 따로 정산해야 한다. 현재 hub의 `offer`는 `Gone`·`Transport` 등을 반환할 수 있다. 한편 마지막 chunk가 넘겨진 뒤 끊긴 무ACK 구독은 r2에서 `sent`만 증가해 `is_lossless()==true`가 가능하다. 일부 상태가 스레드별로 따로 갱신되면 읽는 중인 Health에선 보존식이 순간적으로 깨질 수 있다. [hub offer](../../../crates/dot-logger/src/adapters/remote/hub.rs#L382), [stream 인계](../../../crates/dot-logger/src/adapters/remote/server.rs#L462), [MS-013 회계·최종 판정](../../04_implementation/milestones/MS-013.md#L198)
- **처리 제안** — L2에 실패·기한에 따른 폐기와 처리 중 항목의 소유자를, L3에 offer 전 종료·adapter 실패·부분 fanout의 정산 지점을 둔다. 각 레코드의 구독 eligibility와 건수·바이트의 분모, overflow 표시, Health의 일관된 snapshot 또는 근사 관측 범위를 명시한다. 완료 보고에서 손실 0과 수신 확인 수준을 별도로 노출한다. 무ACK 모델에서 수신을 확인하고 싶으면 벤치마크처럼 client 관측과 대조해야 한다.
- **[처리담당]** 대기.

### RV-227 — 스위치 위치와 적용 결과를 한 시점에서 확정할 수 있는지 판단해야 함

- **[리뷰어]** 토글 턴이 시작될 때 producer의 route head가 파일 전용이면 다음 accepted seq는 알 수 있어도 첫 network payload seq는 알 수 없다. 이후 링 포화와 구독 필터로 첫 수신 seq는 한 번 더 달라진다. `taken`은 ingress에서 마지막으로 꺼낸 위치다. 여기에 표시를 네트워크 링 밖 별도 슬롯으로 게시하면 OFF가 이전 payload를 앞지르거나 ON 전에 새 payload가 나갈 수 있다. 현재 `Logger` 명령 송신은 `Sender` 기반이다. caller timeout과 host 적용을 원자적으로 중재하지 않으면, host가 기한 전에 적용하고 응답 전에 선점된 경우 caller가 `Timeout`을 받으면서 실제 상태는 바뀔 수 있다. [Router taken 갱신](../../../crates/dot-logger/src/routing/mod.rs#L1223), [Logger 명령 채널](../../../crates/dot-logger/src/logger/mod.rs#L1087), [현 구성 응답 대기](../../../crates/dot-logger/src/logger/mod.rs#L1327)
- **처리 제안** — 즉시 반환하는 것은 분배 커서 기반의 하한으로 정의하고, 정확한 첫 수신 위치가 필요하면 첫 payload의 epoch+receipt 또는 후속 관측으로 전달한다. cutoff 포함 규칙·빈 producer·off backlog·구독 스냅샷을 명시한다. 단계마다 이전 payload·부분 chunk·Gap·marker의 순서를 보장하는 FIFO fence를 두고, 새 구독이 이미 켜진 epoch에 붙는 경우의 초기 표시를 정한다. 명령 큐를 제한하고 commit/cancel 경쟁을 중재한다. 기한에 적용 여부를 확인할 수 없는 상태를 별도로 표시하거나 이미 커밋된 결과를 재조회할 수 있게 한다.
- **[처리담당]** 대기.

### RV-228 — 파일 성공과 선택 출력 정리의 기한을 구분해야 함

- **[리뷰어]** 파일 drain/sync가 3 s 기한의 끝에 완료되면 §5.9 순서상 네트워크 drain과 close/join에 남는 시간이 거의 없다. `min(남은 기한)`만으로는 hub mutex 대기·thread join·선택 손실 파일의 동기 IO를 취소할 수 없다. 현재 facade는 core Closed 후 `remote.stop()`에서 thread를 join하고 최종 Closed를 보낸다. 이 구조를 그대로 확장하면 core의 성공과 호출자에게 보이는 종료 지연이 어긋나거나, 선택 출력 때문에 파일 성공을 확인하지 못한다. [host 종료 보고](../../../crates/dot-logger/src/logger/host.rs#L639), [remote stop과 join](../../../crates/dot-logger/src/logger/remote.rs#L321)
- **처리 제안** — host에서 마지막 복사가 끝나면 네트워크 정리를 파일 최종 sync와 병행하고, 단일 절대 기한을 각 작업에 전달한다. 선택 IO가 미종결이면 파일의 완료 결과와 전체 자원 정리 상태를 따로 보고하고, Incomplete 이후 worker 수명·최종 계수의 발행 조건을 명시한다. 같은 기한으로 느린 파일 sync와 멈춘 구독, `loss_file` stall을 겹쳐 파일 성공·외부 응답 기한·자원 회수를 각각 판정한다.
- **[처리담당]** 대기.

### RV-229 — 유휴 heartbeat와 기존 구독 ACK의 호환 계약

- **[리뷰어]** 단방향 frame을 yield하는 사실은 peer가 살아 있다는 증거가 아니다. 전송 계층이 가득 차면 stream이 poll되지 않을 수 있어 `max_age` 타이머와 expiry도 독립적인 wall-clock 감지 상한을 주지 않는다. EV-123 F4에 조용한 단절이 재현돼 있다. 기존 client의 정상 `handoff`는 ACK를 보내고, hub는 ACK가 들어올 때 구독 창을 회수한다. ACK를 껐을 때 마지막 chunk 인계 직후 회수하면 기존 ACK가 `Gone`이 될 수 있고, 회수하지 않으면서 receiver ACK도 생략하면 창이 찬다. [stream poll·lease](../../../crates/dot-logger/src/adapters/remote/server.rs#L447), [hub ACK](../../../crates/dot-logger/src/adapters/remote/hub.rs#L248), [client handoff](../../../crates/dot-logger-client/src/transport.rs#L861), [EV-123](../verification_evidence.md#ev-123)
- **처리 제안** — H3 heartbeat의 보장은 건강한 연결의 유휴 lease 유지로 한정한다. 조용한 단절 상한은 응답을 확인하는 별도 메커니즘을 넣을 때 검증한다. H1·H2에서 기본 옵션을 기존 ACK의 flow control 유지 여부와 별도로 결정하고, 무ACK window 회수 규칙이 필요하면 wire에서 명시 협상한다. opt-in 표시를 필수로 요청한 client가 구형 서버를 만났을 때의 오류·fallback 및 `MarkerOverflow` 공개 상태 매핑을 정한다.
- **[처리담당]** 대기.

### RV-230 — 완료 경계의 관리 wire와 정책 생략의 손실 분류

- **[리뷰어]** 6비트는 기존 terminal bit 시작 위치 8과 겹치지 않는다. core 마스크 밖에도 관리 `GetCompletion` 요청/응답이 경계 0~4만 처리하며 CLI도 다섯 값을 열거한다. 현재 facade가 조회를 노출할지에 따라 공개 경계의 접근 범위를 명시해야 한다. 정책 생략을 새 `DropReason`으로만 추가하면 기존 completion loss 분류를 따라 `OutputDropped`가 된다. loss checkpoint는 scope별 cause 숫자를 검증하므로 새로운 persisted 분류의 호환성을 별도로 판단해야 한다. 현재 파일은 완료 경계 비트 자체 대신 손실 scope/cause를 기록한다. [wire 요청](../../../crates/dot-logger/src/adapters/remote/wire.rs#L372), [wire 응답](../../../crates/dot-logger/src/adapters/remote/wire.rs#L481), [CLI](../../../crates/dot-logger-cli/src/commands.rs#L920), [loss decode](../../../crates/dot-logger/src/loss.rs#L625)
- **처리 제안** — 기존 값 0~4를 유지하고 Handed의 관리 wire·client·CLI 지원 여부를 명시한다. 정책 생략은 `NotRouted`/정책 종결 중 의미가 맞는 쪽을 택하고 파일 손실에 섞이는지 단계 1에서 시험한다. persisted 분류를 늘린다면 구형 판독과 버전 규칙을 해당 단계에 넣는다.
- **[처리담당]** 대기.

### RV-231 — 메모리 상한을 검증할 산식이 일치하지 않음

- **[리뷰어]** `RemoteLimits::reservation()`은 단일 hub에 대해 이미 구독 수를 곱하고 공유 연결·명령 비용도 더한다. §5.10의 바깥 `subscriptions ×`를 적용하면 예산을 중복 예약한다. §5.1은 `ring_bytes`에 메타를 포함하고 §5.10은 다시 메타를 더한다. marker 1 frame은 생산자별 경계 배열 때문에 상수 크기로 볼 수 없다. FR-021의 실행 중 상한과 §5.10의 TLS/HTTP2 예약 제외는 동일한 검증 범위로 쓰여 있다. [RemoteLimits::reservation](../../../crates/dot-logger/src/adapters/remote/types.rs#L40), [MS-013 산식](../../04_implementation/milestones/MS-013.md#L306)
- **처리 제안** — payload·메타·구독 예약을 각각 한 번만 더하는 checked 산식, producer 수에 따른 marker·receipt·명령 크기와 worker 스택을 명시한다. 테스트는 관리되는 예약과 외부 런타임 여유를 별도로 계측한다. 선택 손실 파일의 메모리는 기록량이 많을 때 병합 한도 초과를 어떻게 계수하는지도 정의한다.
- **[처리담당]** 대기.

### RV-232 — 옵션 파일로 출력 조합을 실제로 바꿀 수 있는지 확인 필요

- **[리뷰어]** 이름별 append와 뒤 호출 덮어쓰기만으로는 기존 코드의 `disk` 출력을 제거해 네트워크 전용으로 바꿀 수 없다. 병합표는 같은 이름의 종류 불일치를 충돌로 선언한다. 코드가 타입·writer만 주고 파일이 출력을 전부 소유하는 패턴이라면 재빌드 없이 조합을 바꿀 수 있으므로 지원할 패턴을 명시할 필요가 있다. 교체한 출력의 위치, 이름 없는 기존 `file_output`과의 충돌, `from_toml_str`의 상대 경로 기준도 빠져 있다. [MS-013 병합표](../../04_implementation/milestones/MS-013.md#L215), [기존 순번 해석](../../../crates/dot-logger/src/logger/mod.rs#L1010)
- **처리 제안** — 같은 바이너리와 서로 다른 파일로 파일 전용·네트워크 전용·혼합을 실행하는 수락 시험을 둔다. 기존 builder 출력까지 파일로 전환할 필요가 있으면 전체 교체나 명시 제거를 승인받아 추가하고, 순번과 이름 충돌 및 문자열 로더의 기준 디렉터리를 정의한다.
- **[처리담당]** 대기.

### RV-233 — 기준선 계측과 공개 API 게이트를 단계별로 고정해야 함

- **[리뷰어]** §8.2의 L1/L3 및 queue 판정을 legacy tap에도 동일 적용한다고 했지만 tap은 `offer` 결과를 버린다. 기준선에 폐쇄된 구독·누락 등을 검출할 수 있는 공통 관측점이 필요하다. 공개 항목 35→약 50은 하위 설정 타입과 재노출을 확정하기 전의 예상이다. 검사 스크립트는 60개 상한 외에도 snapshot 전체 일치를 요구한다. 단계 1의 `Boundary` 추가는 완전 match에서 breaking이므로 「공개 동작 불변」은 기존 facade 사용을 대상으로 좁혀야 한다. [legacy tap](../../../crates/dot-logger/src/logger/remote.rs#L74), [표면 gate](../../../scripts/check-integration-surface.py#L393), [단계 계획](../../04_implementation/milestones/MS-013.md#L342)
- **처리 제안** — 기존 경로와 신규 경로 모두 고유 ID·건수·바이트의 송수신 대조로 기준선을 잡고 내부 큐 관측이 필요한 조건은 계측 범위를 명시한다. 실제 facade 이름·reexport·레시피·snapshot을 단계별 gate에 넣고 core·wire 호환 시험을 별도로 둔다. host 복사·완료·스위치·종료의 조기 확인을 반영해 16.5~22 인일을 다시 산정한다.
- **[처리담당]** 대기.

### RV-234 — 파일 없는 구성에서 기존 공개 타입의 의미

- **[리뷰어]** 네트워크 전용에서는 `sync_every` 불필요라고 했지만 `Plan.sync_every: Duration`은 유지된다. 또한 기존 이벤트 API의 파일 이벤트 미설정 오류는 `NotConfigured`이며 r2의 `Unsupported`를 새 변형으로 쓰면 breaking 목록에도 들어간다. [Plan](../../../crates/dot-logger/src/logger/mod.rs#L503), [EventError](../../../crates/dot-logger/src/logger/event.rs#L60)
- **처리 제안** — 파일 없는 Plan의 sync 표현을 고정하고, `mark_event`는 기존 `NotConfigured`를 재사용할지 명시한다.
- **[처리담당]** 대기.

### Human 결정 자료

| 항목 | 리뷰 권고 |
|---|---|
| H1·H2 / Q2·Q4 | 원본 완료는 Handed, 수신 ACK는 별도 추적으로 두는 방향을 지지한다. 파일의 성공, 송신 중 관측 손실, 수신 확인의 차이를 보고에 드러내고 기존 ACK 기반 흐름 제어를 보존한다. |
| H3 / Q6 | 표시를 요청한 구독의 유휴 lease 유지에 heartbeat를 사용할 수 있다. 단절 감지 상한은 별도 범위로 결정한다. |
| H4·H5 / Q5·Q8 | 단일 network, 명시적 legacy 충돌, 파일 순번 유지에 동의한다. r2 기본 경로에서는 혼합 설정의 새 network에 채널 이름 지정이 필요하다. 옵션 파일의 전체 교체 요구는 RV-232에서 확정한다. |
| Q1·Q3·Q7 | 기존 착수 순서와 측정·공개 수락을 유지한다. 공개 API 범위와 300 MB/s 내부 목표는 승인 시 확정한다. |

이번 회차로 예정된 독립 재리뷰를 마쳤다. 원래 P1의 잔여는 위 신규 지적과 연결한다. 후속 설계 반영과 H1~H5·Q1~Q8의 승인은 프로젝트 오너 판단 사항이다.

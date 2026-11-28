# REV-074 — MS-013 출력 구성 선택과 네트워크 전용 모드 설계 리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-27 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 Draft](../../04_implementation/milestones/MS-013.md), [REV-074 요청서](REV-074_REQUEST.md) |
| 기준 | `da5f6c9616a9e902f27a3a379049670be61e9997`. 검토 시 HEAD는 `4f7ec58`. 이후 MS-012 변경과 작업 트리의 TASK-047 변경은 검토 범위 밖 |
| 판정 / 상태 | **Conditional / Closed**(2026-11-27 처리담당: 12건 모두 수용, MS-013 r2 반영 — 2026-11-28 [REV-083](REV-083_MS013_R10_DESIGN_REVIEW.md) Pass로 반복 종료) |

## 1. 요약정리

**P1 4건, P2 7건, P3 1건.** 네트워크 출력을 Router target으로 두는 구조는 기존 소유권 모델과 맞는다. 설계 승인 전에는 BestEffort의 공유 자원 상한, 송신 쪽이 확인할 수 있는 손실, 스위치 경계, ACK와 종료의 순서를 확정해야 한다. 소스·계약·기존 검증 기록을 정적으로 대조했다. 시험·성능 재측정은 리뷰 요청 범위 밖이다.

## 2. 문서 절과 관련 코드 대조

| MS-013 절 | 대조 결과 및 근거 |
|---|---|
| §3, §5.1, §5.2 | 파일 출력 필수, target별 `required: true`·`pending_slots: 0`·`max_attempts: 1`, 파일 0의 쓰기 성공분만 tap, offer 결과 미사용, facade의 원격 손실 계수 부재를 확인했다. Router는 생산자 FIFO에서 읽고 target FIFO로 내보낸다. 유실을 제외한 새 어댑터의 순서는 후속 시험으로 고정한다. target 한도 초과는 `Capacity`, 큐 삽입 실패는 정책에 따라 `RetryExhausted` 또는 `PendingFull`이 될 수 있다. [builder](../../../crates/dot-logger/src/logger/mod.rs#L715), [budget](../../../crates/dot-logger/src/logger/budget.rs#L633), [host](../../../crates/dot-logger/src/logger/host.rs#L1089), [tap](../../../crates/dot-logger/src/logger/remote.rs#L78), [Router](../../../crates/dot-logger/src/routing/mod.rs#L1280), [Health](../../../crates/dot-logger/src/logger/health.rs#L56) |
| §3, §5.3, §5.4 | `PendingAck`·`RemoteDelivery`, 16 MiB **메시지** 상한, 기본 32 KiB 메시지와 10 s lease를 확인했다. 큰 레코드는 chunk로 나뉜다. 구독 바이트 기본값에는 최대 레코드 크기 하한도 있다. [remote limits](../../../crates/dot-logger/src/adapters/remote/types.rs#L40), [기본값](../../../crates/dot-logger/src/logger/remote.rs#L124), [hub](../../../crates/dot-logger/src/adapters/remote/hub.rs#L209) |
| §5.3, §5.8 | ADR-002의 RT 무대기와 ADR-008의 출력별 완료 기준을 함께 적용해야 한다. 현재 복사 경로는 원본과 구독 버퍼의 수명을 분리할 수 있다. core 종료 sync는 `Synced`를 지원하는 대상만 선택하며, facade의 주기 sync는 현재 파일 target 전체를 넘긴다. [ADR-002](../../03_design_artifacts/adrs/ADR-002_rt_logging_never_waits_for_storage.md), [ADR-008](../../03_design_artifacts/adrs/ADR-008_bounded_ownership_completion.md), [ADR-012](../../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md), [ADR-015](../../03_design_artifacts/adrs/ADR-015_batched_pipelined_remote_ack.md), [host](../../../crates/dot-logger/src/logger/host.rs#L748), [control](../../../crates/dot-logger/src/control/mod.rs#L777) |
| §5.5, §5.6, §6 | 기존 미지정 경로는 모든 파일, 명시 인덱스는 파일 선언 순서다. facade 운용 중 변경은 레벨·샘플링이며 원격 ApplyConfig도 Route 변경을 거절한다. 기존 파일 예약 토픽은 `.logger/meta`와 `.logger/loss`다. [route_targets](../../../crates/dot-logger/src/logger/mod.rs#L1010), [set_channel](../../../crates/dot-logger/src/logger/mod.rs#L1314), [API §18.11~12·§22.2](../../03_design_artifacts/interface_contract.md#1811-s3c-remote-선택-연결--2026-11-07), [codec](../../../crates/dot-logger/src/storage/codec.rs#L11), [ADR-013 개정](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md#예약-토픽과-파일-프로필-개정--2026-11-18) |
| §7~9 | EV-105의 루프백 784 MB/s는 8 MiB 고급 경로, 유선 약 116 MB/s는 대용량이다. EV-123 F2의 기존 tap 격리와 F4의 조용한 단절은 새 target에서 다시 검증할 근거다. EV-128은 Windows의 짧은 대기가 소형 처리량을 떨어뜨린 실측이다. [EV-105](../verification_evidence.md#ev-105), [EV-123](../verification_evidence.md#ev-123), [EV-128](../verification_evidence.md#ev-128) |

## 3. 리뷰 포인트

| ID | 중요도 | 위치 | 문제 | 처리 제안 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-213 | **P1** | §5.1, §8.1 | 출력별 절반 한도에서도 개별 writer의 raw 풀을 소진할 수 있음 | 생산자·종류별 예약과 모든 선택 출력의 합산 한도 정의 | 수용 | Resolved |
| RV-214 | **P1** | §5.3~4, §8.1 | 다중 구독 합산식이 성립하지 않고 실제 수신 여부를 관측할 수 없는 경우가 누락됨 | 원본·구독 시도·최종 상태를 각각 보존식으로 정의 | 수용 | Resolved |
| RV-215 | **P1** | §5.6, §8.1 | 토글 플래그와 일반 스트림 기록만으로는 스위치 경계와 완료 상태가 일치하지 않음 | 컷오프·제어 프레임·종료 경합 계약 정의 | 수용 | Resolved |
| RV-216 | **P1** | §5.3, §5.8, §8.1 | 선택 ACK가 원본을 붙드는 동안 core Closed와 remote 정지 순서가 종료 기한을 넘길 수 있음 | 선택 출력 정리 시점과 report 스냅샷 확정 | 수용 | Resolved |
| RV-217 | P2 | §5.2~3, §6 | 한 target의 완료 경계를 구독별 결과로부터 결정할 규칙과 공개 경계 전파 계획이 없음 | fanout 판정 및 완료 비트 마스크·API 포함 | 수용 | Resolved |
| RV-218 | P2 | §1.3, §5.4~5, §8.1 | 메모리 검증이 계산값 비교에 머물고 TOML 예제의 예약이 예산을 초과함 | 전체 예약 범위·이력 상한·실사용 예제 및 경계 시험 | 수용 | Resolved |
| RV-219 | P2 | §5.2, §6 | 한 listener에 복수 network target이 들어오면 구독 원천·토글 범위가 모호함 | 초기 cardinality 및 tap과 동시 지정 규칙 계약화 | 수용 | Resolved |
| RV-220 | P2 | §1.3, §5.7~8, §7~8 | DB 조합을 포함한 FR-019의 완료 시점과 단계 2 수락 범위가 충돌함 | 단계별 수락 범위 및 종료·내구성 표 보완 | 수용 | Resolved |
| RV-221 | P2 | §5.5, §6~8 | 설정 병합·feature·비밀 오류 처리와 공개 호환 범위가 충분히 정의되지 않음 | 병합표·오류 구조·호환/feature 시험 고정 | 수용 | Resolved |
| RV-222 | P2 | §8.2, §9 | Gap 0만으로 무손실 최대 처리량을 판정할 수 없음 | 송수신 대조, 큐 배출, 플랫폼/스레드 관측 추가 | 수용 | Resolved |
| RV-223 | P2 | §7~8 | 단계 1 동작 불변과 신규 장애 검증의 배치 및 사용성 게이트가 불명확함 | 기존 동작 기준선과 다음 단계 Red·Green을 분리 | 수용 | Resolved |
| RV-224 | P3 | §2.2, §3, §5.6 | 원격 기본 한도·꺼진 상태 비용·receipt 필드 서술이 구현 또는 문서 내부와 불일치 | 기준 사실 및 API 도식 정리 | 수용 | Resolved |

### 개별 지적

#### RV-213 — BestEffort 보유량이 파일과 RT 입구의 자원을 먼저 소진할 수 있음

- **[리뷰어]** writer A가 raw 슬롯 2개, writer B가 슬롯 62개라면 절반 상한인 32개 이하에서도 멈춘 네트워크가 A의 raw 슬롯 2개를 모두 붙들 수 있다. 파일이 두 기록을 완료해도 다음 A의 raw 획득이 거절된다. 여러 선택 출력이 각기 다른 원본을 붙들면 출력별 절반 제한의 합으로 공유 한도도 소진할 수 있다. Router의 target ledger는 그 target의 delivery가 살아 있는 동안만 세며 입력 풀·완료 창에 남겨 둘 예약을 제공하지 않는다. [Router 공유·대상 소유권](../../../crates/dot-logger/src/routing/mod.rs#L86), [budget](../../../crates/dot-logger/src/logger/budget.rs#L633)
- **처리 제안** — 엄격한 파일 격리가 목표라면 공유 원본을 넘겨서 ACK까지 붙드는 설계 대신 별도로 예산을 잡은 복사본과 경량 완료 추적을 검토한다. 공유 참조를 유지한다면 선택 출력의 총합 및 생산자별 small/raw/Important/완료 슬롯 한도와 허용되는 거절 증가를 계약화한다. 파일 단독 대조와 비교해 작은 writer quota, 다중 선택 출력, ACK 정지에서 raw 반환과 파일 수락·완료를 함께 시험한다.
- **[처리담당]** 수용. 출력별 큐 상한 대신 **host 턴 내 복사·원본 반환**으로 바꿨다(네트워크 target 소비자는 host, 분배 턴 끝에 별도 예산의 네트워크 링에 복사 후 `Handed` 정산). 원본 보유 ≤ 1턴이라 writer raw·Important·완료 창을 네트워크 상태와 무관하게 돌려준다. 링이 차면 `Dropped(Capacity)`·`ring_full`. 시험: 「파일만」 대비 「파일 + 멈춘 네트워크」에서 RT `Capacity`·파일 완료·`is_success` 동일(raw 2 슬롯, 다중 채널, 구독·스레드 정지). 복사 비용은 K8로 측정. MS-013 §5.1·§8.1·단계 1·2.

#### RV-214 — 송신 손실 집계의 단위와 관측 경계가 맞지 않음

- **[리뷰어]** 원본 1건을 구독자 둘에게 복사하면 `routed=1`, `Σhanded=2`다. 구독이 있어도 필터가 전부 제외하면 두 수의 관계는 다시 달라진다. `Gap`은 복사 전에 발생하며 `offer`가 `Ok(PendingAck)`이어도 Gap이나 Disconnect일 수 있다. `sent`는 마지막 chunk를 스트림에 넘긴 사실이다. ACK가 없는 상태에서 끊기면 이미 수신한 기록과 미수신 기록을 구별할 수 없다. Gap 프레임도 전달 전에 종료될 수 있고, 프레임에는 바이트 합이 없다. EV-123 F4의 조용한 단절에는 즉각 감지를 약속할 근거가 없다. [hub offer·overflow](../../../crates/dot-logger/src/adapters/remote/hub.rs#L382), [stream](../../../crates/dot-logger/src/adapters/remote/server.rs#L462), [EV-123 F4](../verification_evidence.md#ev-123)
- **처리 제안** — Router에 들어온 원본과 탈락 이유를 먼저 회계한다. 구독별로 대상이 된 사본을 기준으로 포화·크기 거절·부분 송신 뒤 종료·ACK 확인·미확인·진행 중을 배타적으로 정의하고 건수와 payload 바이트를 함께 센다. 필터 제외와 스위치 제외는 정책 생략으로 정의한다. gap 생성과 client 관측의 일치는 끝까지 전달된 구독에서 검증한다. 재연결은 새 구독 ID로 회계하고 조용한 단절의 감지 보장 범위를 명시한다. Handed 시점만 증명할 구독에서 실제 수신 확인을 어떻게 보고할지는 C1/C6에 따라 Human 판단에 올린다.
- **[처리담당]** 수용. 원본(L1)·링(L2)·구독(L3)·ACK 4개 보존식과 배타 상태, 건수·payload 바이트 동시 계수, 필터·스위치 제외는 정책 생략, 구독 0명은 `no_subscriber`, 재연결은 새 ID, 퇴역 구독은 최근 8개 상세 + 총계. 조용한 단절 감지 범위(버퍼 포화 시간 + `max_age`)를 명시하고, 무ACK `sent`는 「수신 미확인」으로 보고하는 안을 Human 결정 H2로 올렸다. client Gap = 서버 gap은 정상 종료 구독에서 대조. MS-013 §5.4.

#### RV-215 — 스위치 선형화와 수신 경계가 비어 있음

- **[리뷰어]** OFF 동안 수락된 ingress가 host에 쌓였다가 ON 후 분배될 수 있다. OFF 직전 큐·부분 송신·ACK 중 기록도 남는다. 현재 완료 셀과 Router의 대상 선택은 수락 revision의 route를 공유하므로 Router만 건너뛰면 선택된 셀이 미완료로 남을 수 있다. 일반 구독 기록을 경계로 쓰면 필터·포화 경로를 탄다. 별도 frame kind를 보내면 현재 클라이언트는 `Corrupt`로 종료한다. 시각과 revision만으로는 여러 생산자의 첫 위치를 재구성할 수 없다. 기존 스트림은 유휴 lease가 지나면 닫히므로 일찍 구독해 둔 사용자가 트리거 시점에도 연결돼 있으리라는 보장이 없다. [완료 셀 초기화](../../../crates/dot-logger/src/completion/mod.rs#L930), [Router 선택](../../../crates/dot-logger/src/routing/mod.rs#L1109), [CaptureReceipt](../../../crates/dot-logger/src/capture/types.rs#L111), [client frame 해석](../../../crates/dot-logger-client/src/transport.rs#L809)
- **처리 제안** — 수락과 분배 중 어느 시점을 스위치 기준으로 삼는지 결정한다. 이전 큐의 drain/discard, 이미 넘긴 바이트 처리, 이전 epoch의 완료 결과, 동시 요청·중복 토글·기한 초과와 명령 적용 여부를 계약화한다. 구독 준비 응답과 유휴 lease 만료 뒤 재구독 규약도 정한다. 새 epoch와 재사용 가능한 시작 토큰을 반환하고, 필터·포화에도 경계를 받거나 세션이 명시적으로 끝나도록 프로토콜 호환 및 예약 공간을 정한다. backlog·빠른 OFF/ON·포화·다중 생산자·종료 경합에서 최초 수신 payload와 receipt를 대조한다.
- **[처리담당]** 수용. 컷오프는 host 복사 시점, 경계는 생산자별 `next_seq`(`SwitchReceipt.boundary`)로 정확히 알린다. 명령은 host 큐로 직렬화, 같은 상태면 `changed: false`, 적용 전 기한 경과는 **미적용** `Timeout`. 꺼진 동안 라우팅된 기록은 `switched_off`로 즉시 정산(완료 셀 잔류 없음). 표시는 새 frame kind로 `epoch_markers`를 요청한 구독에만(기존 client 무영향), 필터 우회, 예약 슬롯 1개·불가 시 `MarkerOverflow` 명시 종료. 유휴 heartbeat(`max_age/2`)는 H3로 결정 요청. 시험 목록은 리뷰 제안 그대로. MS-013 §5.6·§6.

#### RV-216 — 선택 ACK의 종료가 파일 성공을 지연시킬 수 있음

- **[리뷰어]** core는 optional까지 포함한 `totals.pending`과 drain이 0이어야 Closed다. facade는 core Closed 이후 remote를 멈춘다. 이 순서에서 선택 ACK가 원본을 잡고 있으면 remote 종료의 2 s 유예가 시작되기 전에 ACK 만료를 기다릴 수 있다. 기존 구독 기본 상한은 10 s다. `deadline_elapsed`는 optional 손실의 직접 영향과 별개로 `is_success()`를 실패시킨다. 주기 sync도 network ID가 들어오면 현재 범용 목록을 그대로 사용할 수 없다. [core Closed 조건](../../../crates/dot-logger/src/control/mod.rs#L847), [종료 성공 조건](../../../crates/dot-logger/src/control/types.rs#L127), [facade join과 remote 정지](../../../crates/dot-logger/src/logger/host.rs#L616), [remote 유예](../../../crates/dot-logger/src/logger/remote.rs#L156)
- **처리 제안** — ingress 차단, 파일 drain·sync, 네트워크 접수 중단, 미확정 ACK 종결, delivery·raw 수명 회수, hub stop/join, 최종 계수 스냅샷의 순서와 예산을 명시한다. Incomplete 중간 계수와 Closed 최종 계수를 구분한다. BestEffort worker 실패는 기존 전역 `worker_failed`가 RT 수락을 닫는 경로에 전파되지 않도록 별도 계약을 둔다. 파일 단독·혼합·네트워크 전용에서 ACK 미발행, 출력 worker 실패, deadline 경합을 시험한다. [전역 실패 처리](../../../crates/dot-logger/src/logger/host.rs#L538)
- **[처리담당]** 수용. RV-213 모델로 네트워크는 core pending에 남지 않아 core Closed가 ACK를 기다리지 않는다(ACK는 복사본 수준 보고, H1). 종료 순서: 입구 → 분배 drain → 파일 drain·sync(파일 target만) → 링 송신 `min(남은 기한, network_drain)` → ACK 창 종료 → hub 정지 `min(2 s, 남은 기한)` → 최종 스냅샷. Incomplete 중간값/Closed 최종값 구분. 네트워크 스레드 실패는 그 출력만 Failed, 전역 `worker_failed` 미설정(RT 수락 유지). MS-013 §5.9.

#### RV-217 — 구독별 결과를 target 완료로 접는 규칙이 필요함

- **[리뷰어]** 기존 `offer_delivery`는 한 delivery와 한 구독의 ACK를 연결한다. 이번 target은 여러 구독으로 fanout한다. 하나는 ACK, 다른 하나는 Gap, 셋째는 필터 제외인 경우 어떤 순간에 Required가 완료되는지 정의가 없다. `Written`은 OS 쓰기 경계다. `Handed`를 별도 변형으로 채택하면 현재 `word & 31` 마스크가 새 비트를 버리므로 enum 추가만으로는 완료 조회와 정리가 동작하지 않는다. 복사가 끝난 뒤 원본 회수는 가능하고 ACK는 그 이후 별도 수명으로 추적할 수 있다. [offer_delivery](../../../crates/dot-logger/src/adapters/remote/hub.rs#L467), [Boundary 계약](../../../crates/dot-logger/src/completion/types.rs#L8), [완료 분류](../../../crates/dot-logger/src/completion/mod.rs#L1025)
- **처리 제안** — 대상 구독 집합을 고정하는 시점, all/any 또는 지정 구독 판정, 0명·필터 제외·부분 ACK와 Required 결과를 명시한다. Q4에서 core 경계를 고르면 마스크·조회·집계·abandon·공개 exhaustive match의 영향을 단계에 포함한다. 복사본의 독립 수명과 ACK 지연 상태는 별도 시험한다.
- **[처리담당]** 수용. 원본 완료는 복사 시점 `Handed` 1회로 고정하고 구독별 결과는 보고로만 둬 fanout 접기 규칙이 필요 없게 했다. `Boundary::Handed` core 추가와 마스크 6비트·조회·집계·abandon·exhaustive match 영향은 단계 1 범위로 명시. MS-013 §5.3·§6·§7.

#### RV-218 — 메모리 상한의 범위와 검증이 불충분함

- **[리뷰어]** 예제의 구독당 128 MiB에 기존 기본 구독 4개를 적용하면 payload 예약만 512 MiB로 총 256 MiB를 넘는다. 현 예약 산식은 런타임/TLS/HTTP2에 별도 여유가 필요하다고 명시한다. 종료된 구독을 매번 보고에 누적하면 동시 구독 제한이 있어도 장기 재연결에서 이력이 늘어난다. Plan 계산값을 Plan과 비교하는 시험으로는 이 경로를 잡을 수 없다. [RemoteLimits 예약](../../../crates/dot-logger/src/adapters/remote/types.rs#L40), [facade 메모리 분할](../../../crates/dot-logger/src/logger/budget.rs#L863)
- **처리 제안** — 고정 예약, 동적 runtime의 범위, 구독·이력·marker·계수·추가 스택의 한도를 명시한다. 종료된 구독의 상세에는 유한 정책을 두고 퇴역 총계를 유지한다. 실행 가능한 TOML, 예산 경계에서 시작 거절, 재연결·최대 동시 구독·포화 중 메모리 상한을 시험한다. `loss_file`은 파일 쓰기가 생기는 명시 옵션으로 구분하고 기간·크기·쓰는 스레드·오류 처리를 정의한다.
- **[처리담당]** 수용. 예약 목록(링·구독 버퍼·표시 슬롯·회계·손실 파일 버퍼·스택, runtime/TLS는 예약 밖 명시), 합계 초과 시작 거절, 예산 경계 ±1·재연결 1,000회·최대 구독·포화 heap 시험. TOML 예시를 1 GiB·구독 2 × 128 MiB·링 64 MiB로 고치고 단계 3에서 실제 시작 시험. `loss_file`은 디스크 쓰기 명시 옵션(1초 합침·`max_bytes` 회전·네트워크 스레드·오류는 Degraded). MS-013 §5.10·§5.5·§5.4.

#### RV-219 — 구독 소스의 수와 호환 규칙을 확정해야 함

- **[리뷰어]** 현재 구독은 채널·레벨·원본 포함 여부를 선택하며 출력 이름은 없다. 동일 hub에 두 network target이 한 원본을 제공하면 중복 수신과 행별 계수 귀속 문제가 생긴다. `remote_with(A, limitsA)`와 `network_output(B, limitsB)` 동시 지정 시 어느 서버·한도를 적용할지도 비어 있다. 또 network target 추가 시 인덱스 없는 기존 채널과 tracing의 라우팅 의미를 정해야 한다. [구독 필터](../../../crates/dot-logger/src/adapters/remote/server.rs#L398), [tracing 기본 route](../../../crates/dot-logger/src/logger/mod.rs#L679), [인덱스 해석](../../../crates/dot-logger/src/logger/mod.rs#L1010)
- **처리 제안** — 0.2의 단일 network target 허용과 충돌 구성의 명시적 거절을 우선 검토한다. 복수를 지원하면 구독이 출력 이름을 지정하는 wire 계약과 중복 정책을 정한다. legacy tap만 선언, 파일+명시 network, remote와 network 동시 선언, tap-only 스위치 지원 여부, 기본 route와 이름·인덱스 혼용을 기본 구성 시험으로 고정한다.
- **[처리담당]** 수용. 0.2 네트워크 출력 최대 1개, `remote()`+`network_output()` 동시 선언 `StartError::Conflict`(H4), tap 구성에는 스위치 없음, 기본 경로는 파일이 있으면 모든 파일·없으면 모든 출력(`tracing` 포함, H5), index·이름 혼용은 `InvalidOutputs`. 각 구성 시험을 단계 2에. MS-013 §5.2.

#### RV-220 — DB 포함 조합의 수락 기준이 단계 2에 걸려 있음

- **[리뷰어]** R1·FR-019는 DB 단독과 혼합 조합까지 요구한다. 단계 2는 세 가지 파일/네트워크 조합만 시험하고 FR-019를 완료로 적는다. DB 매핑·배치 완료 게이트를 단계 6에 두는 것은 타당하나 그 전까지 DB 수락은 보류다. §5.8에는 DB 조합과 legacy tap의 관찰 범위가 빠져 있고, 네트워크 전용에서 의미 없는 sync 설정 및 파일 관련 API 동작도 미정이다. [FR-019](../../04_implementation/milestones/MS-013.md#L43), [단계 2~6](../../04_implementation/milestones/MS-013.md#L269), [현재 파일 기반 필수값 검사](../../../crates/dot-logger/src/logger/budget.rs#L503)
- **처리 제안** — 단계 2의 부분 수락과 단계 6의 DB 단독·혼합 수락을 나눈다. 단계 6 게이트에 Record→DB 매핑·배치 거절/부분 ACK·출력 역할·파일 없는 종료를 포함한다. DB API ACK와 내구성을 구분하고 파일 없는 구성에서 `sync_every`, `directories`, 이벤트 보존, UPS 표의 의미를 정한다. 0.2 alpha 범위에도 해당 부분 완료를 표시한다.
- **[처리담당]** 수용. FR-019를 a(파일·네트워크, 단계 2)와 b(DB, 단계 6 게이트 뒤)로 나누고 alpha 범위에 「DB 조합 미포함」 명시. 게이트 항목(매핑·배치 거절/부분 ACK·역할·파일 없는 종료·API ACK ≠ 내구성) 추가. 보장표에 legacy tap·DB 행과 네트워크 전용의 `sync_every`(경고)·`drop_deadline`·`directories()`·`mark_event` 동작. MS-013 §1.3·§5.7·§5.8·§7.

#### RV-221 — 옵션 파일 병합과 공개 API의 변경 목록이 덜 닫혀 있음

- **[리뷰어]** 호출 순서만으로는 `config()`가 출력을 전체 교체하는지 이름별 병합하는지, preset과 writer별 quota가 어떤 단위로 우선하는지 알 수 없다. 현재 `Plan`은 공개 필드만 있고 출처가 없다. 출처를 담으면 코드와 파일 Plan은 해당 값에서 달라지고 공개 필드를 추가하면 리터럴 호출자도 영향을 받는다. TOML 오류를 문자열이나 source chain으로 전달하면 입력 조각이 노출될 수 있으므로 최상위 메시지 검사 하나로는 충분하지 않다. [Plan 공개 필드](../../../crates/dot-logger/src/logger/mod.rs#L503), [builder](../../../crates/dot-logger/src/logger/mod.rs#L577), [Cargo features](../../../crates/dot-logger/Cargo.toml#L9)
- **처리 제안** — 병합 단위·중복 이름·경로 기준·preset·오류 우선순위를 표로 고정하고 Plan의 동등성은 해석된 설정으로 시험한다. 줄·열과 안전한 오류 코드만 노출하고 원문·값의 `Display`·`Debug`·source·Plan 경로 노출을 시험한다. 파일 크기 상한은 읽는 도중에도 적용한다. TOML 의존을 도입할 때 MSRV 1.85 resolver·audit, default 없음·config-file만·remote/influxdb 조합, 미지원 출력 거절을 확인한다. §6에 Plan, Boundary, 필요 시 ConfigError, wire·client 영향과 외부 소비자의 리터럴/완전 match를 포함한다.
- **[처리담당]** 수용. 병합표(필드별·이름별·토픽별, preset은 빈 손실 값만, 경로는 설정 파일 디렉터리 기준, 오류 우선순위). `Plan` 공개 필드 불변, 출처는 `explain() -> PlanSources`. `ConfigFileError{kind,line,column}`, `source()` 없음, 노출 시험, 읽는 중 1 MiB 상한, toml 의존성 MSRV·audit·feature 조합 확인. §6에 core `Boundary`·client·wire·`PlanWarning` 포함. MS-013 §5.5·§6.

#### RV-222 — Gap 0은 무손실 처리량의 충분조건이 아님

- **[리뷰어]** ingress 거절·Router 포화·구독 없음·중도 단절·미배출 꼬리에도 client Gap은 0일 수 있다. EV-105의 8 MiB 결과는 4 KiB에서 300 MB/s를 뒷받침하지 않는다. 4 KiB에서 300 MB/s는 약 73,200건/s로 레코드당 작업 비용을 확인해야 한다. 새 스레드가 기존 polling 대기를 채택하면 Windows에서 EV-128의 정체가 재현될 수 있다. [EV-105](../verification_evidence.md#ev-105), [EV-128](../verification_evidence.md#ev-128)
- **처리 제안** — 부하·수락·구독 대상·고유 수신 건수/바이트·종료 배출을 대조한다. 정책 필터를 끈 프로필에서 거절·Gap·Unknown·누락·중복이 모두 0이고 큐가 지속 증가하지 않는 최고 속도를 공개한다. Handed/ACK, 구독 수, client ACK 창, payload 형태, 링크, TLS, 스레드 CPU·affinity를 명시한다. 단계 0에 스레드 깨우기의 Windows 검증과 macOS 비교를 넣는다. legacy tap 기준선은 파일 IO 조건을 함께 기록한다.
- **[처리담당]** 수용. 판정을 6점 대조(부하·수락·라우팅·구독 대상·client 고유 수신·종료 배출)와 모든 손실 0·큐 비증가로 바꾸고 조건(Handed/ACK·구독 수·ACK 창·payload·링크·TLS·CPU·affinity·커널·EV-130)을 명시. 4 KiB 300 MB/s ≈ 73,200건/s 기록당 비용 측정, 단계 0에 Windows·macOS 깨우기 측정과 legacy tap 기준선(파일 IO 조건). MS-013 §8.2·단계 0.

#### RV-223 — 동작 불변 단계에 고정할 회귀와 신규 Red의 범위를 구체화해야 함

- **[리뷰어]** 단계 1은 새 worker·큐 모델을 언급하지만 public network target은 단계 2에 들어온다. §8.1의 네트워크 소비 정지 시험을 단계 1의 공개 facade 시험으로 쓰기 어렵다. 기존 시험 통과만으로 Plan 산식, 출력 ID·sync/barrier 순서, 부분 쓰기 tap, 시작 실패 정리, 파일 내구성을 모두 보존했다고 판정하기 어렵다. NFR-008 수락에는 선택 연결당 +6줄 외에도 facade 공개 항목 ≤60이 있다. [단계 계획](../../04_implementation/milestones/MS-013.md#L263), [NFR-008](../../02_project_definition/requirement_spec.md#L52)
- **처리 제안** — 단계 0에서 위 레거시 동작을 고정하고 단계 1은 리팩터링 전부터 Green인 facade 회귀와 내부 Router 시험으로 닫는다. 단계 2에서 network-only와 혼합 장애의 facade Red를 만든다. 호환·wire·장기 계수 검증의 소속 단계를 표시한 뒤 인일을 갱신한다. NFR-008 공개 항목 수와 L3 레시피 계측도 완료 조건에 넣는다.
- **[처리담당]** 수용. 단계 0을 레거시 골든 고정(Plan 산식·출력 ID·sync/barrier 순서·부분 쓰기 tap·시작 실패 정리·내구성)으로, 단계 1은 골든 + 기존 회귀 + 내부 Router 시험으로 닫고, facade Red(네트워크 전용·혼합 장애)는 단계 2로. NFR-008 공개 항목(현재 35/60, 예상 약 50)·L3 「네트워크 전용」 레시피를 단계 2 완료 조건에. 인일 16.5~22로 갱신. MS-013 §7·§8.1·§6.

#### RV-224 — 기준선과 용어의 세부 정합성

- **[리뷰어]** §3의 기본 `subscription_bytes`에는 최대 레코드 크기까지 올리는 하한이 생략됐다. P6의 분배 비용 0과 §5.6의 host 원자 읽기 1회는 다르다. `SwitchReceipt` 선언과 구독 0명 예시의 필드가 일치하지 않는다. [기본 원격 한도](../../../crates/dot-logger/src/logger/remote.rs#L124), [스위치 초안](../../04_implementation/milestones/MS-013.md#L217)
- **처리 제안** — 한도 산식을 정확히 옮기고, OFF 보장은 새 사용자 payload 분배 0으로 표현한다. receipt 정의에 구독 수와 그 관측 시점을 포함한다.
- **[처리담당]** 수용. 기본 `subscription_bytes` = max(min(slot 합, 4 MiB), record_bytes)로 정정, OFF 보장은 「새 사용자 payload 분배·복사 0(기록당 원자 읽기 1회)」로, `SwitchReceipt`에 `subscribers`(적용 시점)·`boundary` 정의. MS-013 §3·§2.2 P6·§5.6.

### 열린 질문과 재리뷰

| 질문 | 리뷰 의견 |
|---|---|
| Q1 | 기존 MS-012 → TASK-041 → MS-013 구현 순서 유지. |
| Q2 | 기한 내 정리된 BestEffort 유실은 기존 required 중심 성공 판정과 분리한다. 네트워크 전용에서는 성공한 종료와 무손실을 각각 표기한다. |
| Q3 | 측정·공개를 수락 기준으로 두고 300 MB/s는 승인 시 내부 측정 목표로 사용한다. |
| Q4 | 독립 경계 `Handed` 사용을 권고한다. core에 공개할 때는 RV-217의 영향 범위를 반영한다. |
| Q5 | 호출 순서는 명시적인 병합표와 함께 채택할 수 있다. |
| Q6 | 오류 원인 구분과 keepalive는 별도 작업으로 진행할 수 있다. 조용한 단절의 감지 범위와 timeout 계약은 MS-013에 명시한다. |
| Q7 | 새 공개 보고 타입부터 확장성을 설계한다. 기존 타입의 `#[non_exhaustive]`는 0.2 소비자 이관 비용을 확인하고 결정한다. |
| Q8 | 파일 순서 인덱스를 유지하고 이름 선택을 추가한다. 미지정·혼용 동작도 RV-219에서 고정한다. |

후속 설계 세션에서 MS-013을 수정하고 지적별 반영 여부와 수락 기준을 기록한 뒤 REV-075에서 대조한다.

**[처리담당] 2026-11-27:** 12건 모두 수용해 MS-013 r2에 반영했다(반영표 MS-013 §11). 새 Human 결정 H1~H5(MS-013 §2.3)가 생겼다. 재리뷰 요청: [REV-075 요청](REV-075_REQUEST.md). 최종 설계 승인과 열린 질문의 채택은 프로젝트 오너 결정이다.

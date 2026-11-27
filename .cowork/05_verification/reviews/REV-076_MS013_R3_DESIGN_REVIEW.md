# REV-076 — MS-013 r3 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-27 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r3](../../04_implementation/milestones/MS-013.md), [REV-076 요청서](REV-076_REQUEST.md), [REV-075](REV-075_MS013_R2_DESIGN_REVIEW.md), [REV-074](REV-074_MS013_OUTPUT_DESIGN_REVIEW.md) |
| 기준 | r3 `cda0161e560733bcfd5c090cc183081864f023a7`, 검토 시 HEAD `d468a0a` |
| 판정 / 상태 | **Conditional / Open** |

## 1. 요약정리

**P1 5건, P2 5건.** r3는 원본 guard를 네트워크 스레드에 보내지 않고 기존 수신 ACK 흐름 제어를 유지한다. 스위치 receipt가 보장하는 위치를 생산자별 하한으로 좁혔고, 옵션 파일의 출력 전체 교체와 파일 없는 구성의 기존 API 의미도 구체화했다. 착수 전 복사 보류의 수명, 완료 셀 정산, 스위치와 표시의 커밋, 실패 시 최종 회계, 절대 기한의 최종화 권한을 정해야 한다. 코드·계약·기존 검증 근거를 정적으로 대조했다. 제품 시험과 성능 측정은 요청 범위 밖이다.

## 2. 이전 지적 대조

| 이전 지적 | r3 확인 결과 / 잔여 |
|---|---|
| RV-225, RV-213 | §5.1에서 IO worker의 `Delivery::report` 재시도를 host에서 제거한다. 턴 초과 기록 보류와 공용 feedback에 남는 경합은 RV-235·236. H6 측정은 RV-244. |
| RV-226, RV-214 | §5.4에 세 단계 폐기와 ACK 종결 항목이 있다. 상태를 누가 어디서 관측·인수하는지는 RV-238·239. |
| RV-227, RV-215 | §5.6의 생산자별 하한은 현재 ingress 커서로 표현할 수 있다. 커밋, 가득 찬 링, 구독 가입과 Gap 순서는 RV-237·240. |
| RV-228, RV-216 | §5.9에 병행 종료와 파일 성공 분리가 있다. 멈춘 network worker/runtime을 기한에 정산하는 권한과 `Closed`의 변경 의미는 RV-239. |
| RV-229 | §5.3~6에서 ACK를 유지하고 heartbeat 보장을 유휴 lease로 한정했다. 지원 확인·종료 사유의 호환 잔여는 RV-240·243. |
| RV-230, RV-217 | 관리 wire 경계 5와 CLI 범위가 추가됐다. OFF의 대상별 정책 종결은 RV-236, 공개 변경 목록의 잔여는 RV-243. |
| RV-231, RV-218 | §5.10은 hub 예약의 중복 곱셈을 해소했다. 스위치·종결 제어 경로에서 동시에 보유하는 최대치는 RV-242. |
| RV-232, RV-221 | §5.5에서 출력 전체 교체와 상대 경로 기준을 정했다. 기존 위치 참조가 교체된 파일을 조용히 가리키는 문제는 RV-241. |
| RV-233, RV-222·223 | §6~8의 공통 관측점과 표면 스냅샷 검토를 확인했다. H6의 조기 판정과 성공 기준은 RV-244. |
| RV-234, RV-220 | 파일 없는 구성은 `Plan.sync_every = Duration::ZERO`·`EventError::NotConfigured`. DB 조합 수락은 단계 6에서 하고 alpha 범위에는 미포함으로 표시한다. |

RV-219의 단일 네트워크 출력·legacy tap 충돌 규칙, RV-224의 구독 기본 바이트 하한과 OFF 중 사용자 payload 분배 0 문구는 유지된다. H4·H5의 최종 채택은 프로젝트 오너 판단이다.

## 3. 리뷰 포인트

| ID | 중요도 | 위치 | 문제 | 처리 제안 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-235 | **P1** | §5.1, §7 | 파일 분배 후 복사를 다음 턴으로 넘기면 payload의 소유권·순서·진행 조건이 빠짐 | 같은 dispatch에서 끝나는 정책과 최소 허용 용량 정의 | 대기 | Open |
| RV-236 | **P1** | §5.1, §5.3 | OFF 선택 셀이 정산되지 않고 공용 feedback의 Busy는 파일을 막을 수 있음 | 대상별 완료 전이·예약·공정 재시도 설계 | 대기 | Open |
| RV-237 | **P1** | §5.6 | CAS 승리, 링 표시 삽입, 영수증 응답이 하나의 기한 계약을 이루지 못함 | 커밋 전에 표시 자원 예약, bounded 응답·재조회 정의 | 대기 | Open |
| RV-238 | **P1** | §5.4 | 현재 ACK/Gap 관측만으로는 배타 정산을 복원할 수 없고 실패 시 소유자가 불명확 | hub 상태 전이에 근거한 계수와 복구 가능한 진행 상태 | 대기 | Open |
| RV-239 | **P1** | §5.9 | 기한을 집행할 스레드 자체가 멈추면 join과 최종 보고가 막힘 | 독립 기한 보고·finalize권·미회수 상태 정의 | 대기 | Open |
| RV-240 | P2 | §5.6, §6 | 구독 시작·전환 표시·Gap의 순서가 현재 hub 모델과 맞지 않음 | 동일 순서 기준과 호환 행렬 정의 | 대기 | Open |
| RV-241 | P2 | §5.5, Q8 | 전체 교체가 기존 숫자 index의 대상을 조용히 바꿈 | 교체 시 명시적 remap 또는 조합 거절 | 대기 | Open |
| RV-242 | P2 | §5.10 | 제어 프레임·완료 결과·회계 구조의 동시 보유 피크가 예약되지 않음 | 각 파티션의 고정 상한과 인코딩 경계 검증 | 대기 | Open |
| RV-243 | P2 | §6, §7 | 관리 응답의 추가 비트가 옛 클라이언트의 기존 조회도 깨뜨리고 방출 API 목록이 빠짐 | 요청/응답 호환과 public enum·오류 표현 명시 | 대기 | Open |
| RV-244 | P2 | H6, §7~8 | H6의 두 비교점과 판정 프로필이 불완전하고 첫 판정이 늦음 | 공통 부하·가드 정지 시점·조기 go/no-go 정의 | 대기 | Open |

### RV-235 — 파일 분배 뒤 다음 턴에서 복사할 원본이 없음

- **[리뷰어]** dispatcher는 ingress를 한 번 소비해 `SharedPayload`를 만든 뒤 모든 선택 target에 분배한다. H6의 복사 한도를 넘겨 파일에만 넘긴 원본은 다음 턴에서 다시 읽을 수 없다. 별도의 보류 guard를 만들면 정지한 네트워크 흐름에 원본을 붙드는 문제를 되살리고, 복사 완료까지 동일 생산자의 순서와 종료 범위도 새로 관리해야 한다. ingress를 건드리기 전에 턴을 멈추면 원본을 별도 보유하지 않지만 해당 파일 분배도 멈춘다. 복사 가능한 최대 레코드가 설정된 턴 예산보다 크면 매 턴 보류하는 경우도 생긴다. [Router 분배](../../../crates/dot-logger/src/routing/mod.rs#L1109), [공유 원본 획득](../../../crates/dot-logger/src/routing/mod.rs#L1223), [r3 턴별 한도](../../04_implementation/milestones/MS-013.md#L178)
- **처리 제안** — 파일 우선이 필수라면 복사 예산을 다 썼을 때 선택 출력의 해당 레코드를 같은 분배에서 명시적으로 폐기하고 별도 계수하는 방식이 단순하다. 다음 턴 처리가 필요하면 완전한 분배 전 보류와 파일에 가는 영향, 또는 별도 예약 복사본의 소유권을 선택한다. 최대 raw 크기, 한도보다 큰 첫 기록, 턴 말미 혼합 기록, FIFO·원본 회수·종료까지 단계 0에서 검증한다.
- **[처리담당]** 대기.

### RV-236 — feedback 경합과 OFF 정산은 완료 모델 변경이 필요함

- **[리뷰어]** 현재 dispatcher는 `Enqueued`, `Dropped`, 전체 `Unrouted`만 기록한다. 등록 시 이미 선택된 network 셀을 Router가 OFF에서 건너뛰기만 하면 해당 셀이 pending으로 남는다. 혼합 기록에 기존 `Unrouted`를 적용하면 파일 셀이 있으므로 `InvalidTransition`이다. `Cause::Policy`는 ingress 분류이며 `Bank::output`에 넘기면 저장할 output row가 없다. 또 dispatcher feedback도 동일 `Completion::report`를 거쳐 `Busy`가 나면 앞 항목에서 멈춘다. 버퍼에 항목을 더 잡아도 선점된 셀 뒤의 파일 feedback은 밀릴 수 있다. [예약 셀](../../../crates/dot-logger/src/completion/mod.rs#L932), [dispatch 적용](../../../crates/dot-logger/src/completion/mod.rs#L854), [feedback flush](../../../crates/dot-logger/src/routing/mod.rs#L303), [loss row](../../../crates/dot-logger/src/loss.rs#L317)
- **처리 제안** — 대상별 `Handed`/정책 생략 terminal 전이와 ABI를 명시한다. 정책 생략을 network 보고에만 둘지, 타깃 귀속을 버리고 기존 persisted 분류에 넣을지 결정한다. 링 publish 전에 대응하는 완료 슬롯을 예약하고, Busy 재시도와 stage 0의 실패 기준을 정한다. OFF-only·혼합 OFF, 파일 완료 셀 선점, feedback 포화/재개에서 중복 복사 0과 종료 pending 정리를 확인한다.
- **[처리담당]** 대기.

### RV-237 — 링 표시와 스위치 영수증의 커밋 경계가 다름

- **[리뷰어]** §5.6의 CAS가 host를 승자로 정한 뒤에도 바이트 링이 가득 차 있으면 ON/OFF 표시를 즉시 넣을 수 없다. 구독 큐의 제어 자리 1개는 host→network 링을 비워 주지 않는다. 상태를 바꾸고 표시를 뒤로 미루면 경계보다 새 payload가 앞서거나 표시가 빠질 수 있다. 더구나 receipt의 `subscribers`는 network 스레드가 표시를 관측한 뒤에 정해지는데 같은 절은 host가 커밋 직후 응답한다고 한다. 네트워크 스레드를 정지시키면 host 승리 뒤 caller 대기에 상한이 없다. host에도 절대 기한 확인이 없으면 caller가 늦게 스케줄되는 동안 만료된 명령을 적용할 수 있다. [스위치 응답·CAS·FIFO](../../04_implementation/milestones/MS-013.md#L279), [기존 host 명령 처리](../../../crates/dot-logger/src/logger/host.rs#L557)
- **처리 제안** — 커밋 전에 링 제어 공간을 확보하고, 만료·Closing 확인을 포함해 커밋과 표시 순서를 정한다. 즉시 receipt는 host가 소유하는 epoch/하한까지만 보장하고, 구독 관측은 후속 상태로 제공하거나 기한 후 적용 여부를 조회하는 bounded 결과를 둔다. 링 full, host 선점, network 정지, 중복 요청, 연속 토글, shutdown 표시 삽입을 결정적으로 시험한다.
- **[처리담당]** 대기.

### RV-238 — 복사본을 누가 최종 정산하는지 확정해야 함

- **[리뷰어]** 현재 `PendingAck`는 `Unknown`만으로 마지막 chunk 이전의 `cut`과 이후의 `unacked`를 구분할 수 없다. `offer`의 overflow도 `Ok(PendingAck)`로 돌아오며 Gap/Disconnect 분기가 서로 다르다. `sent` 전이와 ACK·close는 hub 상태에서 일어난다. 네트워크 스레드가 링 pop이나 부분 fanout 직후 실패하면 해당 스레드의 마지막 카운터 스냅샷과 살아 있는 구독·링만으로 이미 offer한 대상과 미시도 대상을 확정할 수 없다. seqlock 게시가 진행 중인 채 writer가 중단되는 경우, Health의 읽기 시도를 무제한 반복하면 관리 호출도 끝나지 않는다. [PendingAck](../../../crates/dot-logger/src/adapters/remote/hub.rs#L90), [chunk와 ACK 전이](../../../crates/dot-logger/src/adapters/remote/hub.rs#L209), [overflow](../../../crates/dot-logger/src/adapters/remote/hub.rs#L414)
- **처리 제안** — 구독 lock 안에서 전이를 계수하거나, 해당 전이를 손실 없이 꺼낼 수 있는 bounded 상태를 설계한다. 링 pop·eligible 고정·offer·ACK의 각 경계에서 중단돼도 다른 소유자가 한 번만 정산할 수 있게 한다. Health는 bounded 읽기와 마지막 정상 snapshot을 제공하고 counter overflow 및 snapshot 미완료를 표시한다. `is_lossless`는 최종 정산이 완료된 보고에서만 참이 될 수 있게 하고, ingress 거절 포함 여부와 단위(원본과 구독 사본)를 문서화한다.
- **[처리담당]** 대기.

### RV-239 — 절대 기한에 보고할 주체와 자원 정리 상태

- **[리뷰어]** §5.9의 (b)는 링 종료 표시·hub close·runtime 종료·thread join을 수행해야 한다. 실제 (b)가 hub 잠금이나 schedule 정지에 걸리면 기한에 자기 작업을 강제 정리하지 못한다. `shutdown_timeout(0)`은 runtime 소유 스레드가 그 호출에 도달해야 시작하며 외부 `JoinHandle::join`의 상한을 제공하지 않는다. 현재 facade는 core Closed 뒤 remote join을 하고, `Shutdown::Closed`는 cleanup 완료를 문서화한다. 파일만 성공했고 멈춘 스레드를 남겨도 즉시 `Closed`와 최종 등식을 함께 약속하는 계약은 미확정 자원을 누가 소유하고 계수를 동결하는지 정의해야 성립한다. [host Closed 발행](../../../crates/dot-logger/src/logger/host.rs#L639), [remote stop과 join](../../../crates/dot-logger/src/logger/remote.rs#L321), [Shutdown 공개 타입](../../../crates/dot-logger/src/logger/mod.rs#L342)
- **처리 제안** — 파일 완료와 facade의 종료 응답 기한을 독립적으로 관측하고, 별도 감독자가 기한에 응답하도록 한다. network worker/runtime이 미종결이면 `Incomplete`와 함께 파일 성공을 노출하거나, `Closed` 의미를 개정하고 report의 네트워크 부분을 Unknown/미최종으로 표시하는 대안 중 하나를 계약화한다. `Detached`의 남은 메모리·파일 핸들 소유권과 동시 logger 재시작 시 한도도 명시한다. host 또는 network 정지, ring 종료 표시 full, file sync와 종료 close 동시 지연을 구별해 시험한다.
- **[처리담당]** 대기.

### RV-240 — 초기 epoch와 이전 링 backlog의 순서를 고정해야 함

- **[리뷰어]** host가 epoch 2로 넘어간 뒤 링에 epoch 1 payload가 남았을 때 새 구독에 epoch 2 초기 표시를 즉시 보내면, 현재 L2의 pop 시점 eligible 규칙에서는 그 구독이 뒤이어 epoch 1 payload를 받을 수 있다. 현재 hub는 전송 레코드 FIFO와 별도의 `gap: Option<(first,last)>`를 쓰고 미전송 payload를 Gap보다 우선한다. marker 한 자리를 추가하는 것만으로 Gap이 경계를 넘겨 합쳐지지 않음을 보장하기 어렵다. 또 client는 첫 frame에서 kind 0 핸드셰이크를 요구하므로 초기 표시는 핸드셰이크 뒤 첫 이벤트로 계약해야 한다. [hub stream poll](../../../crates/dot-logger/src/adapters/remote/hub.rs#L209), [server kind 0](../../../crates/dot-logger/src/adapters/remote/server.rs#L449), [client handshake](../../../crates/dot-logger-client/src/transport.rs#L504)
- **처리 제안** — 구독 가입과 전환이 소비자의 FIFO 순서를 공유하게 하거나, 가입 시점 fence로 이전 payload를 제외한다. gap 구간을 epoch 경계에서 분리하고 marker 순서가 멀티 chunk와 ACK 창 회수 순서에 영향 주는지 명시한다. 새 구독이 backlog, 빠른 토글, 포화 Gap 사이에 들어오는 시험을 추가한다.
- **[처리담당]** 대기.

### RV-241 — 출력 교체 시 기존 index가 조용히 다른 파일을 가리킴

- **[리뷰어]** 코드가 `outputs(&[0])`로 파일 A를 선택했을 때 설정의 파일 B를 목록 앞에 넣으면 같은 코드가 B에 기록한다. 현재 `route_targets`는 번호가 유효한지만 확인하고 해당 번호를 선언된 파일 ID로 바꾼다. `explain()`은 관찰을 돕지만 잘못된 목적지에 쓰는 일을 막지 않는다. §6의 index 의미 유지 문구와 §5.5의 교체 후 새 순번 해석도 같은 조건에서 함께 읽힌다. [기존 index 검사](../../../crates/dot-logger/src/logger/mod.rs#L1010), [교체와 index](../../04_implementation/milestones/MS-013.md#L229)
- **처리 제안** — 요구 R1의 코드 타입/파일 정책 조합은 허용하되, 전체 교체가 기존 숫자 route를 건드리는 경우 명시적 이름 정책으로 덮도록 하거나 시작을 거절한다. 자동 이름은 참조 가능 여부 및 뒤따르는 코드 선언과의 충돌 규칙을 고정한다. 교체된 혼합 설정에서 파일 순번 재배치가 조용히 적용되지 않는 시험을 둔다.
- **[처리담당]** 대기.

### RV-242 — 관리 예약은 링 외부의 제어 데이터까지 포함해야 함

- **[리뷰어]** MS-013의 `P × 12`는 필드 크기의 합이다. 실제 `Vec<(u32,u64)>`의 요소 크기, 활성 명령 한 건과 응답 소유권, logger가 보관하는 현재 epoch/경계, ring 내 제어 항목 여유, network↔runtime 전이 상태를 각각 확인해야 한다. 현 서버는 `RemoteLimits.message_bytes`를 인코딩에도 적용하므로 생산자가 많은 구성의 표시 frame은 슬롯이 있어도 gRPC 한도를 넘을 수 있다. 관리 책임 범위의 상한과 runtime 측정 여유를 분리한 방향은 적절하다. [현재 예약](../../../crates/dot-logger/src/adapters/remote/types.rs#L40), [gRPC 송신 한도](../../../crates/dot-logger/src/adapters/remote/server.rs#L313), [r3 예약 목록](../../04_implementation/milestones/MS-013.md#L327)
- **처리 제안** — 큐와 처리 중 항목, 장기 보관 스냅샷, 정리 미완료 시 소유 상태를 포함하는 checked 피크 산식을 둔다. packed 표현 또는 실제 메모리 크기를 사용하고, 최악 protobuf 인코딩 길이를 시작 전 검증하거나 bounded 분할을 계약화한다. 최대 생산자·최소 message 한도와 여러 토글/정리를 겹친 예산 경계 시험을 추가한다.
- **[처리담당]** 대기.

### RV-243 — 공개 완료 이벤트와 오류 사유의 호환 목록

- **[리뷰어]** r3의 관리 wire 표는 옛 client가 5를 받으면 오류임을 적었다. 실제 기존 decoder는 요청한 경계와 별개로 응답 `confirmed`의 bit 5가 있으면 전체를 거절하므로, 옛 client가 기존 `Enqueued`를 조회해도 복사 target에 대한 응답이 깨질 수 있다. dispatcher feedback을 재사용하면서 `DispatchOutcome`에 변형을 추가하면 공개 enum 목록에도 포함해야 한다. 현재 client의 `rpc_error`는 `Limit`과 임의 메타데이터 사유를 함께 보존하지 않는다. [client 완료 decoder](../../../crates/dot-logger-client/src/codec.rs#L259), [DispatchOutcome](../../../crates/dot-logger/src/routing/types.rs#L114), [client 오류 해석](../../../crates/dot-logger-client/src/transport.rs#L208)
- **처리 제안** — 기존 경계 질의에 bit 5를 투영하지 않거나 확장 지원을 명시적으로 협상한다. 지원할 조합을 고급 경로에서 동일 major의 old↔new로 시험한다. 공개 dispatch/조회·클라이언트 오류 타입에 필요한 변경은 §6과 단계별 외부 소비자 검증에 적고, `MarkerOverflow`가 호출자에게 어떻게 구별되는지 확정한다.
- **[처리담당]** 대기.

### RV-244 — 파일 격리 기준의 실패를 초기에 검출

- **[리뷰어]** H6의 0.8P·10% 두 기준은 측정 가능하지만 P가 시도율, 수락률, 최종 파일 완료율 중 무엇인지와 지속 시간·기록 혼합·writer quota·sync 설정이 없다. 구독 정상과 정지의 raw 반환 「시각이 같다」는 문장은 스케줄러 시간 변화에 취약하다. 링 full 이후 실제 memcpy가 생략돼 정지 부하가 정상 복사 부하보다 가벼워질 수도 있다. 설계의 핵심은 단계 0에서 spike하는데 H6 계측은 단계 5에서만 판정한다. [H6](../../04_implementation/milestones/MS-013.md#L104), [단계 0·5](../../04_implementation/milestones/MS-013.md#L368), [측정 계획](../../04_implementation/milestones/MS-013.md#L399)
- **처리 제안** — P는 고정 구성의 지속 무손실 파일 완료율로 정의하고, 동일 입력 시퀀스·quota·tmpfs/sync와 동등한 복사량을 명시한다. guard 보유·재획득은 같은 시각을 요구하는 대신 턴 수와 장기 보유 상한으로 판정한다. 단계 0에서 복사 부담과 격리 기준의 조기 탈락 조건을 두고 단계 5에서 확정 측정한다. 수정된 완료·정리·wire 의존성이 확정되기 전 20~25 인일은 범위 추정으로 유지한다.
- **[처리담당]** 대기.

## 4. 결정 자료와 다음 회차

H1의 원본 완료와 수신 ACK 분리, H3의 건강한 연결 유휴 유지, H4·H5의 단일 network와 기본 route를 지지한다. H6은 RV-244의 동일 부하와 판정 시점을 포함해 결정하고, H7·Q8은 RV-241에서 전체 교체와 위치 기반 route의 결합을 함께 결정한다. Q2의 파일 성공과 네트워크 정리 결과 분리는 RV-239의 종료 타입 계약과 함께 승인 대상이다.

RV-235~244를 다음 개정에서 반영한 뒤 REV-077에서 대조한다. 이전 지적이 해소되고 새 지적이 0건인 회차까지 반복한다.

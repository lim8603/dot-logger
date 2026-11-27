# REV-077 — MS-013 r4 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-27 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r4](../../04_implementation/milestones/MS-013.md), [REV-077 요청서](REV-077_REQUEST.md), [REV-076](REV-076_MS013_R3_DESIGN_REVIEW.md) |
| 기준 | r4 `3901fa3b5b3952f656ef2c2cc83d1a1b13dbf8f7`, 검토 시 HEAD `6ae7314` |
| 판정 / 상태 | **Conditional / Open** |

## 1. 요약정리

**P1 3건, P2 3건.** 같은 분배 안에서 네트워크 복사를 시도하고 초과분을 버리는 계약으로 원본 보류 문제가 해소됐다. observer를 완료 레지스트리 밖에 두는 설계는 공유 완료 큐 및 관리 wire 영향도 제거한다. 다음 개정에서 스위치 기한, 장애 중 집계 소유권, 종료 보고의 시점을 닫아야 한다. 검증은 소스·기존 계약의 정적 대조로 진행했다. 시험과 성능 측정은 요청 범위 밖이다.

## 2. 이전 지적 대조

| 이전 지적 | r4 확인 결과 |
|---|---|
| RV-235 · 복사 한도 | 같은 분배에서 네트워크 몫만 버리고 원본을 별도로 보유하지 않는다. 한도와 파일 경로 영향은 단계 0·1의 계획으로 확인할 수 있다. [MS-013 §5.1](../../04_implementation/milestones/MS-013.md#L174) |
| RV-236 · 완료 경합 | observer에 완료 셀과 별도 feedback이 없어 혼합 OFF의 pending/Busy 반례를 해소한다. observer-only의 `NoRoute` 의미와 0 target 경로는 RV-248. |
| RV-237 · 커밋 | 제어 슬롯 선확인과 host-only 영수증으로 링 포화와 네트워크 스레드 응답 의존을 제거했다. host 선점에 대한 기한 계약은 RV-245. |
| RV-238 · 정산 | hub 잠금 내 상태 전이, bounded Health, `finalized` 조건을 추가했다. 공유 진행 상태와 외부 정산 간 전이 및 보존식은 RV-246. |
| RV-239 · 종료 | 호출자 기한 감시와 `Incomplete` 재사용 방향은 적절하다. 기한 시 파일 결과의 출처와 최종화 확인은 RV-247. |
| RV-240 · 구독 순서 | 공용 링 순번, 가입 fence, kind 0 이후 초기 표시, Gap fence를 지정했다. ACK와 종료 알림의 순서는 RV-249. |
| RV-241 · 출력 교체 | 숫자 index와 교체의 조합을 시작 시 거절하고 자동 이름의 참조를 막아 기존 번호의 조용한 재해석을 방지한다. |
| RV-242 · 예산 | 링 제어, packed 메모리, 진행 중 명령과 구독 상태를 별도로 예약한다. 표시의 실제 wire 길이 상한은 RV-250. |
| RV-243 · 호환 | `Boundary`와 `DispatchOutcome` 변경을 철회해 관리 응답 bit 5 문제가 해소됐다. RV-217·230을 위해 별도 완료 경계를 다시 열 근거는 없으며 수신 확인은 네트워크 보고에서 한다(H1). `End`의 종료 및 ACK 계약은 RV-249. |
| RV-244 · 조기 측정 | H6에 60 s 무손실 파일 완료율과 동일 부하 프로필, 단계 0 go/no-go 조건이 있다. 실제 포화율·복사 비용·깨우기 방식은 spike와 후속 측정으로 확인하면 된다. |

REV-074·075의 파일 tap 유지, 단일 네트워크 출력, DB 단계 6 수락, 출력 파일 구성 교체 시험과 레거시 측정 관측점이 유지된다.

## 3. 리뷰 포인트

| ID | 중요도 | 위치 | 계약 공백 | 처리 제안 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-245 | **P1** | §5.6 | CAS 성공과 상태·표시 게시 사이에 기한을 넘길 수 있음 | bounded 응답과 적용 여부의 공개 상태를 정의 | 대기 | Open |
| RV-246 | **P1** | §5.4, §5.9 | L3 최종 등식에 cut이 빠지고 offer 직후 중단을 정확히 한 번 정산할 근거가 없음 | 보존식과 복구 가능한 커밋 경계를 고정 | 대기 | Open |
| RV-247 | **P1** | §5.9 | core Closed를 파일 성공으로 발표하면 최종 파일 발행 실패를 놓치고, host 무응답 시 Incomplete에 담을 상태가 없음 | 파일 최종 발행 ACK와 기한 시 보고 원천 정의 | 대기 | Open |
| RV-248 | P2 | H8, §5.1, §7 | 기존 NoRoute가 관측 출력 성공에도 붙으며, 0 target 구성에는 명시적으로 바꿔야 하는 시작 계약이 있음 | Core와 네트워크 보고의 의미 정의 및 시작 gate | 대기 | Open |
| RV-249 | P2 | §5.6, §6 | End를 보내고 구독을 닫을 때 기존 ACK 확정·오류 계약이 열려 있음 | End/EOF/ACK의 종료 상태표와 호환 시험 | 대기 | Open |
| RV-250 | P2 | §5.6, §5.10 | 표시 protobuf의 상한 산식에 반복 항목의 인코딩 오버헤드가 빠질 수 있음 | 형식을 결정하고 실제 인코딩 길이로 상한 확인 | 대기 | Open |

### RV-245 — 「host 한 단계」는 응답 기한 상한이 아님

- **[리뷰어]** r4의 ④ `Pending → Committed` CAS 직후 host가 선점되면 호출자의 280행 설명과 달리 ⑤ 상태 변경과 표시 게시가 아직 되지 않았을 수 있다. 이때 취소 CAS는 실패하고 영수증 대기가 host 재개 시점까지 연장된다. 기한 확인과 CAS 사이에 선점되면 호출자가 먼저 취소하지 않은 경우 기한 뒤 CAS도 가능하다. 같은 상태 명령에서 CAS를 생략할 경우에도 취소와 응답 중 어느 것이 최종 결과인지 정해야 한다. [커밋과 호출자 계약](../../04_implementation/milestones/MS-013.md#L275)
- **처리 제안** — `Timeout`의 미적용 보장은 취소가 확정된 경우로 유지하고, 기한에 host가 이미 권리를 얻었으나 게시·영수증이 미확정이면 호출자가 반환할 상태와 명령별 재조회 방법을 정의한다. 다른 방식을 택한다면 상태·표시가 관측되는 단일 커밋과 기한 보장의 근거를 문서에 명시한다. 단계 0의 핵심 판정은 CAS 직후 host 정지 상태에서도 API가 정해진 기한에 반환하고 재개 후 한 번만 효과가 나타나는지다.
- **[처리담당]** 대기.

### RV-246 — 손실 보존식과 offer의 복구 가능한 전이

- **[리뷰어]** L3에서 `offered=1, queued=1`인 사본을 구독 close에서 `cut`으로 옮기면 현재 식의 우변이 0이 된다. `sent`를 누적값으로 유지하는지, ACK 후 다른 상태로 옮기는지도 식에 함께 표시해야 한다. 또한 hub에서 offer가 실제로 성공한 직후 네트워크 스레드가 `Progress.next_offer_index`를 갱신하기 전에 실패하면 감독자는 이미 큐에 넣은 구독까지 `not_offered`로 세게 된다. 링 pop과 Progress 등록 사이에도 같은 공백이 있다. 단일 완료 플래그는 감독자가 두 번 실행하는 경우를 막을 수 있으나 이 중간 경계의 완료 여부를 담지 않는다. [회계와 Progress](../../04_implementation/milestones/MS-013.md#L200), [hub offer의 상태 변경](../../../crates/dot-logger/src/adapters/remote/hub.rs#L414)
- **처리 제안** — 누적 수량과 순간 상태를 구분하고, 최소한 L3 우변에 `cut`을 포함하며 ACK 식을 명시한다. 예를 들어 누적 `sent = acked + unacked + ack_pending`으로 둔다. 링 소비권 이전과 구독별 offer가 감독자에게 복구 가능하도록 링 항목·구독 ID별 커밋 또는 동등한 원자적 소유권 계약을 제시한다. 취소 이후 늦게 재개한 worker가 확정 계수를 변경할 권한도 정한다. 경계 사이 중단, hub 성공 직후 중단 및 취소 후 재개에서 각 단위를 정확히 한 번 정산하는지 검증한다.
- **[처리담당]** 대기.

### RV-247 — 기한 보고가 실제 파일 완료와 같은 사실을 말해야 함

- **[리뷰어]** r4는 core Closed 직후 `FilesClosed`를 보낸다. 현재 host에서는 core Closed 이후에 IO 스레드가 `target.log.finish()`로 마지막 세그먼트를 발행하고, 실패하면 `io_failed`를 올린다. `FilesClosed`를 먼저 보낸 뒤 이 발행이 실패하면 호출자가 이미 파일 성공을 관측할 수 있다. 반대로 host가 최초 진행 메시지 전에 멈춘 경우 호출자는 채널 수신 기한은 지킬 수 있으나 현재 공개 `Incomplete`가 요구하는 `FacadeReport`를 채울 원천이 없다. 기존 API는 이 경우 `Unresponsive(PendingShutdown)`을 반환한다. 기한 이후 동기 감독 정산까지 호출자가 맡으면 잠금 재시도 외에 링·구독 순회 시간도 반환 기한에서 빠진다. [core 종료 후 파일 finish](../../../crates/dot-logger/src/logger/host.rs#L639), [최종 발행 실패 처리](../../../crates/dot-logger/src/logger/host.rs#L943), [PendingShutdown::wait](../../../crates/dot-logger/src/logger/mod.rs#L379)
- **처리 제안** — `FilesClosed`가 마지막 파일 발행 결과까지 포함하도록 순서를 명시하고 완료 시각을 함께 기록한다. 기한까지 보고가 하나도 없으면 기존 `Unresponsive`를 유지하거나 관측 가능한 불완전 상태를 별도로 정의한다. 호출자의 기한 경로에서는 확정 스냅샷만 읽고, 시간이 남지 않으면 `finalized=false`로 반환해 정산을 `PendingShutdown`에 맡긴다. 단계 0에서 무보고 host 정지, core Closed 뒤 파일 finish 실패, 정산 중 기한 만료를 각각 판정한다.
- **[처리담당]** 대기.

### RV-248 — observer-only의 공개 의미와 0 target 시작 기준

- **[리뷰어]** 현 `Cause::NoRoute`는 기록이 명시적으로 **어느 출력에도** 라우팅되지 않았다는 뜻이며 producer commit 때 채널 단위로 센다. 특정 파일만 피한 기록을 파일별로 센다는 H8 설명과 다르다. 기존 파일 A/B에서 B만 선택한 기록은 NoRoute가 아니지만, 파일+observer 구성에서 성공적으로 네트워크에 전달한 observer-only 기록은 r4대로라면 NoRoute다. 또 기존 `Router`와 completion Registry 생성부는 `targets.is_empty()`를 명시적으로 거절한다. 이는 지금 코드의 실행 결과가 불확실한 경우가 아니라 새 모드가 의도적으로 변경할 검증 경계다. [NoRoute 정의](../../../crates/dot-logger/src/loss.rs#L49), [입구 계수](../../../crates/dot-logger/src/completion/mod.rs#L934), [Router 검증](../../../crates/dot-logger/src/routing/mod.rs#L536), [Registry 검증](../../../crates/dot-logger/src/completion/mod.rs#L144)
- **처리 제안** — H8에서 observer-only를 기존 `NoRoute`로 보고할 경우 「tracked 출력 없음」이라는 공개 의미·혼합 파일 판독 안내·네트워크 계수와 대조하는 기준을 명시한다. 기존 의미를 그대로 보존하려면 observer 성공과 무출력을 구분한다. 0 target 허용은 observer가 실제 구성된 경우로 범위를 정하고, 시작·입구 수락·raw 회수·주기 sync 생략·네트워크 단절 중 종료 및 기존 빈 구성 거절을 spike의 합격 조건으로 명시한다. 구현 방식은 spike에 맡길 수 있다.
- **[처리담당]** 대기.

### RV-249 — End 알림과 미확정 ACK를 함께 처리해야 함

- **[리뷰어]** 기존 client는 EOF에서 남은 ACK RPC를 정리한 뒤에만 정상 종료한다. 기존 hub는 구독을 닫으면서 pending을 폐기하며, 닫힌 구독에 새 ACK를 받으면 Gone을 돌려준다. r4가 `End(MarkerOverflow)`를 보낼 때 어느 시점에 ACK 접수를 닫는지, client가 End를 보고 바로 루프를 끝내도 되는지 비어 있다. ACK RPC 오류와 End가 동시에 오면 기존 client의 `reap()`이 먼저 실패하며 End를 읽지 못할 수도 있다. [EOF와 ACK](../../../crates/dot-logger-client/src/transport.rs#L751), [hub close](../../../crates/dot-logger/src/adapters/remote/hub.rs#L170), [ACK에 대한 Gone](../../../crates/dot-logger/src/adapters/remote/hub.rs#L248)
- **처리 제안** — 정상 drain/강제 종료별로 End, 새 payload 차단, ACK 허용 종료, EOF의 순서를 명시한다. 확정되지 않은 ACK와 End 이유를 둘 다 사용자에게 보존하거나 우선순위를 공개한다. End는 살아 있는 스트림에서 전달할 수 있는 상태로 정의하고, 전달 불가 시 transport 오류가 어떻게 보이는지 명시한다. 마지막 레코드의 ACK가 남은 경우와 ACK RPC 오류가 먼저 도착한 경우를 wire 호환 시험에 더한다.
- **[처리담당]** 대기.

### RV-250 — 표시의 wire 길이는 12P 메모리 산식과 별도로 검증

- **[리뷰어]** §5.6의 「머리 + P × (5+10)」는 varint 값 길이만 합친다. 생산자별 항목을 protobuf repeated message로 인코딩하면 각 항목의 필드 태그와 바깥 길이 prefix도 반복된다. packed scalar 배열 또는 byte blob을 쓸 경우의 길이는 또 다르다. 표시가 실제 한도를 넘으면 기존 서버의 encoding 한도 때문에 시작은 통과해도 구독이 표시에서 끊길 수 있다. [표시 상한](../../04_implementation/milestones/MS-013.md#L275), [gRPC encoding 한도](../../../crates/dot-logger/src/adapters/remote/server.rs#L313)
- **처리 제안** — 표시 필드 표현을 먼저 고정하고 최악 값으로 만든 실제 frame의 encoded length나 그에 대한 검증된 checked 상한으로 시작 조건을 판정한다. 최대 producer 수와 최소 메시지 한도에서 경계값을 시험한다. 저장용 packed 메모리 크기 산식은 그대로 별도 검증한다.
- **[처리담당]** 대기.

## 4. 수렴 기준과 다음 회차

RV-245~250은 공개 계약·상태 전이·정산식의 결정 사항이다. 정확한 Rust 원자 순서, ring 구현 방식, 복사 처리량, 스레드 깨우기 방식 및 허용치 충족 여부는 골든과 단계 0 spike에서 판정하면 된다. H6의 조기 go/no-go와 기존 구성 회귀가 이미 있으므로 이 측정 결과를 설계 재리뷰의 선행 조건으로 요구하지 않는다.

RV-245~250을 다음 개정에서 반영한 뒤 REV-078에서 대조한다. 이전 지적이 해소되고 새 지적이 0건인 회차까지 반복한다.

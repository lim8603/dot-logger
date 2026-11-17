# REV-060 — TASK-034 runtime 설정·native/loss 통합(D) 개발 자가검토

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-060 / 2026-11-17 착수·2026-11-18 마감 |
| 대상 | `6628475` 이후, 보관 WIP36c1724를 현재 ADR-013 native/loss 계약에 맞춰 적용 |
| 종류 | 개발 세션 자가검토 — Human 「런타임 통합 진행하자」 |
| 판정 | **Pass** — [EV-087](../verification_evidence.md#ev-087), 제품 `874c27c` |
| 상태 | **Closed** — 2026-11-18, RV-153/154 조치·위임 수락 |

S1 계약 승인·기존 S2 중요도/시작 level/이벤트 및 S3를 유지하고 S2 runtime 잔여를 통합했다. `set_channel`·`refresh`·쉬는 held/tracing/반납 writer ack·원격 ApplyConfig를 제공하며 B의 단일 active writer revision 회전을 그대로 소비한다. 제품 형식 선택/legacy wrapper·새 저장 회전 경로를 만들지 않는다. E GUI/호출 비용/최종 커버리지와 Task 전체 완료는 별도다.

## 2. 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 수정 | `logger/mod.rs`·`writer.rs` | ConfigError·set_channel·refresh, held writer 공유 소유권 |
| 수정 | `logger/host.rs`·`sink.rs`·`remote.rs` | 단일 pending 설정 slot·기한/종료 처리·무대기 ack·remote ApplyConfig |
| 수정 | `control/config_update.rs`·`logger/budget.rs` | required() 공용 계산·두 live bank의 사전 예약 |
| 수정 | `telemetry/operations.rs` | facade의 route 사전 거절을 위한 crate 내부 parse_patch 재사용 |
| 수정 | core/client/allocator 시험·`tests/api-surface/logger.txt`·README | 실제 native revision/손실·remote 연속성·busy/lock 경계·RT0, 표면35/60 |

## 3. 클래스(타입) 변경표

| 구분 | 타입 | 내용 |
|---|---|---|
| 생성 | `logger::ConfigError` | Busy·Invalid·Closing·Failed 고정 공개 코드 |
| 생성 | `Held`·`HeldWriters`(내부 alias)·`Configure` | facade 보유 writer 공유·host의 단일 대기 요청 |
| 수정 | `Logger`·`Writer`·`ChannelOptions` | 일반 설정 게시·RT refresh·초기 옵션 문서 |
| 수정 | `Command`·`HostPlan`·`Links`·`Remote`·`Runtime`·`SinkState` | 설정 명령·borrowed update 전달·held ack·remote 관리 |
| 수정 | config update `State` | 레이아웃 불변, 예약 산식 required() 추출 |

## 4. 리뷰 항목

| ID | 중요도 | 위치·문제 | 판정/처리 | 상태 |
|---|---|---|---|---|
| RV-153 | P2 | WIP host의 `(result, queue.pop_front())`가 result=None에도 대기 요청을 버려 idle writer에서 Busy 대신 Failed. 동적 VecDeque와 기한 이후 게시 가능성 | 수용. pending Option 한 개를 완료/기한/종료 때만 take. 추가 로컬 대기는 Busy, 기한이 지난 요청은 게시 전에 Busy | Closed |
| RV-154 | P2 | WIP `Held::acknowledge(true)`/sink ack가 애플리케이션의 writer mutex를 기다려 set_channel 고정 기한을 침범 | 수용. caller/host/sink 모두 try_lock으로 건너뛰고 관리 turn에서 재시도. 독점 writer 접근 유지 | Closed |

### RV-153

**[리뷰어]** WIP를 현재 저장소에 적용한 뒤 `f40_an_idle_taken_writer_makes_the_next_change_busy_until_it_refreshes`가 `Err(Failed)`를 돌려 Red였다. 결과가 없는 turn에도 pop_front로 Sender가 사라졌다. 관리 pending의 동적 증가와 만료 요청의 늦은 게시도 같은 경계에서 정리해야 한다.

**[처리담당]** 단일 `Option<Configure>`로 상한을 고정했다. 종료/worker failure/core closing을 먼저 확인하고, 만료이면 게시 전에 Busy, 준비됐으면 현재 posted base에 채널 patch를 적용한다. None은 pending을 그대로 둔다. 추가 요청은 기존 pending을 바꾸지 않고 Busy다. f40의 Busy→refresh→성공·dropped/untaken writer, 내부 `a_waiting_configuration_is_retained_and_additional_requests_are_busy`의 추가 요청 Busy/원 대기 요청 성공/이미 만료된 요청 비게시를 대조했다. remote는 기존 일회성 core Busy/Conflict 의미를 유지한다.

### RV-154

**[리뷰어]** held mutex를 잡은 동안 별도 thread에서 set_channel을 부르면 500ms 안에 답하지 못하는 Red를 확인했다. sink 역시 `wait=true`이면 event encoder가 가진 lock을 기다릴 수 있었다.

**[처리담당]** 일반 관리 경로에서 try_lock으로 접근한다. poisoned mutex는 기존 방식으로 상태를 복구하되 WouldBlock은 건너뛴다. producer를 외부에서 대신 ack하지 않고 facade가 실제 소유하는 Writer만 refresh하므로 사용자 RT writer와 경쟁하지 않는다. `configuration_does_not_wait_for_the_held_writer_mutex` Green 및 idle tracing·반납 writer·host 실패/reaper 회귀로 확인한다.

## 5. 검증·리뷰 포인트

- TDD: 새 API 부재 compile Red → WIP 접목 후 busy/lock 의미 Red → 수정 Green. 오래된 Envelope fixture helper는 native reader로 대체했고 기존 minimum 옵션·이벤트/손실·종료 조치를 보존했다.
- native: 파일별 metadata revision과 각 사용자 RecordMeta revision 일치, r1→r2→r3 실제 payload 보존. runtime Policy 생략6건은 epoch1/원 session으로 누적되고 final tail을 확인했다. generic B의 r1→r2→r1→r2·fault/pin/보호·batch prefix 시험도 전체 회귀에 포함한다.
- remote: telemetry off/on 양쪽에서 route InvalidInput·memory RestartRequired·잘못된 base Conflict·미ack Busy·성공 Applied2, 이어지는 로컬 revision3 및 실제 파일 user0/r1·user2/r2·user3/r3를 대조한다.
- RT: refresh/새 revision에서의 Skipped를 allocator guard 안에서 실행한다. 새 unsafe·RT 할당/해제/I/O/lock/retry 없음. refresh의 sampling reset은 사전 등록한 유한 배열을 대상으로 한다.
- 최종 두 Rust workspace 각684 Pass·5 ignored, 각18 feature/consumer/default19·remote20 allocator·예제, 두 clippy·fmt·strict196 docs·표면35/60/레시피·Miri13 Pass([EV-087](../verification_evidence.md#ev-087)). Open→In Progress→Resolved→Closed로 위임 마감했다. namespace 이전 reserved topic 거절 f44와 공개 rustdoc까지 최종 전체에 포함한다. E GUI/호출 비용·TASK-033 최종 커버리지/UC·native/장기/실사용처는 이번 단위 밖이다.

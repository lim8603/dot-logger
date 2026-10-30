# TASK-006 — 완료 추적·명시 sync·종료

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-002 |
| Story | US-001·005 |
| 요구사항 | FR-001 FR-003 FR-013 NFR-003 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-31 |
| 다음 액션 | EV-009 최신 수정분 로컬 Pass·Human 승인; native는 공개 완료 후(T3) 실행 |
| 의존 | TASK-005 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

2026-10-28 Human 「다음 진행하자」로 착수. TASK-004/005 구현·로컬 Pass·Human 승인을 선행 근거로 CI 후속 검증 상태에서 개발을 진행한다. GAP-004와 native Not Run은 유지하고 계정 조회/차단 재시도를 하지 않는다.

## 변경 대상과 첫 검증

구현: [`completion/`](../../../crates/dot-logger/src/completion/), [`control/`](../../../crates/dot-logger/src/control/), tracked capture와 routing feedback 연결, `tc_003_completion.rs`·`tc_003_control.rs`·공통 fixture, 독립 consumer·할당 계측·CI recipe.

첫 실패/리뷰 조건: **seq1 pending·seq2 done이면 전체 완료 아님, window 포화 수락 거절**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-003`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] 대상별 partial/expired/unknown·deadline snapshot, shutdown incomplete에서 owner 유지.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] 최신 수정분 Human 리뷰 완료 — 2026-10-29 Human 「승인」으로 RV-001~005 최종 수정분 승인. native/실장비 이월은 유지한다.

테스트 정의: [TC-003](../../05_verification/test_case.md#tc-003). 제품 코드·테스트 실행 결과: [EV-005](../../05_verification/verification_evidence.md#ev-005--task-006-완료-추적명시-sync종료). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.


## 구현 결과 — 2026-10-28

- `CaptureOwner::new_tracked`가 producer별 고정 completion window를 preallocate한다. 같은 초기 route/revision을 사용하고 capture/후속 Router/ControlOwner의 승인 메모리 분할을 대조한다. 한 slot의 `Empty/Initializing/Active/Updating/Terminal` lifecycle에 단발 CAS로 예약 소유권을 얻고 pending slot은 재사용하지 않는다. 일반 보고 중이면 RT가 `CompletionCapacity`로 즉시 거절하며 거절 raw loan은 동일 identity/pointer/내용으로 반환한다. 필터 생략·입력 오류·입력 포화는 추적 슬롯/sequence를 소비하지 않는다.
- 초기화/상태/receipt sequence는 고정 atomics다. RT 수락/대여 call의 active 표시와 closing을 SeqCst로 순서화해 pre-close admission/loan publication 종료 후에만 shutdown 범위를 동결한다. 새 unsafe 경계·외부 의존성·OS IO/worker를 RT 경로에 추가하지 않았다.
- `Completion::poll`은 대상 capability와 receipt를 검증하고 Written/Synced/DB API/수신 앱 확인을 구분한다. 확인 경계는 실패로 지우지 않는다. 상세 이력은 완료 후 `history_retention` 또는 window 재사용 중 먼저 온 경계에서 Expired이며 pending을 시간만으로 축출하지 않는다. retired terminal 수량은 별도 고정 counter로 보존한다. 일반 `poll_at`으로 동일 단조 도메인의 시간 경계를 재현할 수 있다.
- producer별 first/last 고정 scope·유한 page·동시 query permit·deadline/cancel wait를 제공한다. 모든 page를 읽기 전 전체 완료를 반환하지 않고 Busy/Failed/Dropped/Unknown/Expired/NotRouted를 구분한다. 조회/대기는 sync/출력 취소를 실행하지 않는다.
- tracked Router는 dispatcher/대상 reader의 completion 결과를 유한 feedback에 보존한다. Busy 시 보존/백프레셔, 일반 delivery의 명시 보고 없이 Drop하면 Unknown이다. 같은 slot의 새 generation에 오래된 Drop이 Unknown 표시를 덮지 못하도록 sequence `fetch_max`를 사용한다. Written을 확인한 guard는 payload를 해제해도 Synced는 pending으로 남을 수 있다.
- `ControlOwner`는 ordinary sync slots와 별도 shutdown slot을 사전 예약한다. 한 capture에 하나의 controller만 연결하고, 재생성 epoch·session/slot/generation 토큰으로 stale/다른 세션 명령의 재사용을 차단한다. 연결 lease는 control allocations 해제 뒤 마지막에 반환한다.
- sync는 fixed scope/target/deadline 명시 요청이다. concrete dispatcher의 이전 입력/대상 IO/feedback 배출을 확인한 후 `SyncWork`를 writer 대역에 넘긴다. 같은 target의 sync IO를 직렬화하고 실제 확인 뒤 유한 metadata application을 수행한다. 기한 초과 중인 guard는 무효화하거나 slot 재사용하지 않는다. 이후 수락 기록을 과거 barrier로 Synced 처리하지 않고, 실패/Unknown이어도 Written을 보존한다.
- 정상 shutdown은 신규 입력 close → pre-close call quiescence → cutoff 동결 → drain/선언한 sync → 실제 guard/loan/회수·feedback 정리로 진행한다. deadline의 Incomplete는 owner를 소비하지 않으며 같은 owner로 이후 cleanup을 이어간다. Closed도 실패/폐기/불명과 deadline 준수 여부를 보고한다. borrowed producer/query/IO guard의 owner 수명은 유지되고 앱 thread join은 애플리케이션 scoped host 책임이다.
- 실제 파일 생성·OS sync/rename·writer thread 기동·장비 durability/RT/throughput은 TASK-007 이후다. 여기의 sync 확인은 테스트/예제 writer 대역이며 디스크 영속 성공 근거가 아니다. 후속 입력이 계속되면 범위보다 넓은 대상 drain 대기가 생길 수 있고 deadline으로 제한된다.

## 후속 자가 리뷰 — 2026-10-28

Human 요청으로 TASK-001~006 설계/구현을 대조했다. 현 source36개는 EV-005와 일치하나, [EV-006](../../05_verification/verification_evidence.md#ev-006)의 신규 반례가 Rust1.85/1.96 각각3개 실패했다. RV-001(writer 실패가 Enqueued feedback보다 빠르면 분배 정지), RV-002(Normal 완료창 점유로 Important 예약 우회), RV-003(필수 sync가 시작 전 만료되면 같은 owner의 cleanup 정체)는 P1 미수정이다. README 구현 단계 불일치 RV-004도 기록했다.

기존 로컬 Pass·Human 승인·게시 사실은 이력으로 유지하고, 현 구현의 설계 충족 판정은 보류한다. 제품 코드·기존 제품 테스트는 수정하지 않았으며 격리 복사본의 재현 코드/실행 로그를 EV-006에 보존했다. TASK-007보다 이 결함들의 수정안·회귀 검증을 먼저 진행하도록 권고한다. native/장비 미검증과 사용량 조회/차단 재시도 금지는 그대로다.

## 자가 리뷰 후속 수정·회귀 — 2026-10-28

Human 「지금 수정과 회기 검증 진행해도 되겠어?」로 수정/검증 착수. 위 EV-006의 판정은 수정 전 이력이며, 현 결과는 [EV-007](../../05_verification/verification_evidence.md#ev-007)을 따른다. RV-001~003의 정식 재현 시험을 먼저 실패시킨 뒤 구현했고 README RV-004도 정렬했다. 완료 예약은 기존 sequence mod window를 유지한 유한 lookahead·단발 CAS이며, 만료 shutdown sync는 실제 자원/선행 sync 종료 뒤 유한 cursor로 Unknown을 적용한다. 일반 sync 만료·진행 IO 수명·Written 확인은 유지한다.

신규9개(완료 private1·완료 통합3·제어 통합4·RT guard1), 두 Rust 각16 features/독립 consumer/예제/clippy/rustdoc/allocator·fmt Pass. Miri lib9+raw17+routing19+completion15+control18=78×8 seeds(624) Pass. Important small/raw 경로 allocation/deallocation0/0, query/sync128회 requested peak11586 / concrete14978 / 최종 잔여0. 전체 검증 중 기존 consumer의 Important16 대비 완료창8 오류를 발견해 window32/work_per_poll32로 정렬하고 재검증했다.

수정분 Human 구현 리뷰·native/실제 파일/worker/장비 검증은 잔여다. 기존 승인 이력과 별개이며 TASK-006/MS-002 In Progress, Gate4/5 Not Ready를 유지한다. commit/push·CI 조회/재시도·fresh audit는 하지 않았다. 새 source37개(README 포함)와 명령/로그는 EV-007에 고정한다.

후속 마감: Human 「커밋 푸쉬하고 세션 마무리」에 따라 수정 코드 `d57c2e3`을 `[skip ci]`로 커밋·origin/main push했고 원격 ref와 EV-007 source37개 blob 일치를 확인했다. 검증/상태/마감 문서는 별도 docs(cowork) 커밋이다. 커밋 요청을 수정분 구현 리뷰 승인으로 대신하지 않으며 native/장비 이월과 사용량 조회/재시도 금지는 유지한다. 다음은 EV-007 수정분 리뷰이며 TASK-007은 미착수다.

## 후속 개선 검토·종료 경합 리뷰 — 2026-10-29

Human 「리뷰 개선 검토부터 하고 TASK-006 검토 진행하자」에 따라 지난 제안과 수정 코드를 검토했다. EV-007 source37개는 현재 파일과 모두 같아 기존 전체 시험은 반복하지 않았다. RV-001~004 수정·과거 Pass/승인 이력은 유지한다.

[EV-008](../../05_verification/verification_evidence.md#ev-008)에서 **RV-005 / P1**을 확인했다. 다른 스레드의 일반 `request_sync`가 shutdown 열림 검사 뒤 정지하면 그 미게시 호출을 추적하지 않은 shutdown이 Closed 성공을 반환한다. 이후 요청이 게시되어 새 SyncWork를 발급할 수 있으며, IO가 진행 중인데도 Closed가 유지된다. 실제 공개 API를 두 scoped thread에서 호출하고 격리 복사본의 test-only hook으로 스케줄만 고정했다. Rust1.85/1.96 각각 새 계약 assertion1개 실패, 순차 거절·레이아웃 관측2개 Pass다. 원본 제품 코드는 수정하지 않았다.

최종 구현 승인 권고는 **Reject / 보류**다. 일반 sync 요청의 진입·게시와 shutdown 전환/최종 정리를 함께 순서화하고 이 경합의 정식 회귀를 먼저 추가해야 한다. 단순 사후 검사만 추가하면 검사와 게시 사이의 공백이 남는다. GAP-007 Open, TASK-006/MS-002 In Progress·TASK-007 미착수·native/장비 이월을 유지한다.

이전 제안은 다음과 같이 정리했다. 채널 선형 탐색은 capture/completion에 중복되지만 유한 탐색 자체는 무대기 위반이 아니므로 인덱스 재사용/LUT를 측정 후 비교한다. RecordSlot(size40/align8)의 정렬은 별도 TargetCell 배열·메모리 증가도 포함해 장비에서 비교한다. 미등록 채널은 이미 InvalidInput으로 거절하므로 그 사례에 근거한 CaptureError enum 확대는 보류한다. poll_shutdown은 bool이 아닌 ShutdownPoll이고 기존 문서에 진행 계약이 있으므로 반복 구동 예제 보강은 P3 후보다. 세부 근거는 EV-008에 둔다.

## RV-005 수정·회귀 및 수정 반영 총괄 — 2026-10-29

Human 「수정 진행하고, TASK-006 변경사항 보고할때 수정반영사항 같이 보고하자」에 따라 [EV-009](../../05_verification/verification_evidence.md#ev-009)를 실행했다. 신규 정식 반례2개를 두 Rust에서 먼저 실패시킨 뒤 수정했고, 오류 반환 보강까지 private3개를 추가했다. 제품 변경은 control/mod.rs와 새 control/tests.rs다.

- `SyncSubmission` RAII·단발 CAS로 일반 sync 진입~게시/오류 반환을 표시한다. SeqCst 진입 표시·열림 확인·shutdown close·정지 확인으로 pre-close 호출을 빠짐없이 종료 추적에 포함한다.
- 요청 게시 중에는 Closing/Incomplete를 유지한다. 게시 후 실제 sync가 끝나야 성공 Closed, 게시 전 기한 도달이면 만료/실패 Closed, 검증 오류면 표시가 풀려 IO 없이 Closed에 도달한다. 동시 요청은 기존 Busy, close 이후는 Closing이다.
- RT 경로·private unsafe·dependency/lock·공개 오류 enum은 유지했다. poll_shutdown rustdoc에 Closing/Incomplete 이후 반복 구동과 Closed 성공 여부의 별도 확인도 명시했다.
- 두 Rust 각16 features·독립 consumer·scoped 예제·clippy/rustdoc/fmt·기존 완료15/제어18·allocator6 Pass. Miri lib12+raw17+routing19+completion15+control18=81×8(648) Pass. tracked/Important RT0/0, query/sync128회 peak11586/concrete14978/최종 잔여0이다. F-11 소비자별 종단 검증과 명령/환경/source38개 hash는 EV-009에 연결했다.

| 수정 ID | 현재 반영사항 | 이번 작업에서의 구분 |
|---|---|---|
| RV-001 | writer 실패 뒤 늦은 Enqueued 확인을 병합하고 terminal 실패·분배 진행 보존 | EV-007 기존 수정 유지·회귀 |
| RV-002 | producer별 Important 완료 보호량 및 Normal cyclic runway 검사·W≥R 검증 | EV-007 기존 수정 유지·회귀 |
| RV-003 | 시작 전 만료 shutdown sync를 실제 IO/선행 sync 종료 뒤 유한 cursor로 Unknown 정리 | EV-007 기존 수정 유지·회귀 |
| RV-004 | README를 현재 코어/대역과 실제 파일·worker 미구현 범위에 정렬 | 기존 반영 유지 |
| RV-005 | 일반 sync 진입/게시와 shutdown 순서화로 Closed 후 새 sync IO 발급 차단 | 이번 수정·정식 회귀3개 |

자가 리뷰 **Conditional — 로컬 수정/회귀 Pass**, GAP-007 로컬 Closed다. TASK-006/MS-002는 최종 수정분 Human 리뷰·native 미검증으로 In Progress이며 TASK-007은 미착수다. LUT/정렬의 장비 측정과 공개 오류 enum 보류 판단은 유지한다. 구현 승인·native/장비 검증을 완료로 대신하지 않고 commit/push·CI 조회/재시도도 수행하지 않았다.

## 최종 AI 검토 — 2026-10-29

Human 「다음단계로」에 따라 앞서 안내한 최종 리뷰를 진행했다. EV-009의 source38개·검증 artifact·원본 압축 로그 hash가 현재 파일과 모두 일치하며, 기록된10개 검사 명령의 exit0을 확인했다. 소스 변경이 없어 같은 시험은 반복하지 않았다.

- RV-001의 terminal 보존/늦은 Enqueued, RV-002의 producer별 cyclic 보호, RV-003의 실제 자원/선행 sync 종료 후 정리 및 RV-004 README 범위를 재확인했다.
- RV-005는 진입 CAS→열림 확인→close→정지 확인의 SC 순서로 pre-close 일반 sync 게시를 추적한다. close 이후 호출은 게시 전에 거절하고, 요청 실패·포화도 RAII로 표시를 해제한다. 실제 guard/명령 metadata application은 기존 종료 조건이 계속 검사한다.
- test-only 중단은 스레드별 cfg(test)에 한정된다. RT 경로·private unsafe·dependency/lock은 그대로이며 공개 오류 enum 확대가 없다. 동시 요청 Busy는 일반 호출자가 재시도할 기존 오류다.
- 추가 P1/P2 결함은 확인하지 못했다. 판정은 **Conditional — 로컬 구현 범위의 승인 권고**, 최종 Human 승인/native·실장비 완료를 대신하지 않는다.

다음 개발 준비: TASK-007의 승인된 계획에서 u64 receipt 왕복·벽시계 역행·unknown schema·8 MiB 단일 record 경계를 첫 실패 시험으로 시작한다. 이후 독립 MCAP reader·CRC/index 및 encoder/index 메모리 예산을 대조한다. TASK-007은 아직 Planned이며, 최종 수정분 Human 승인과 native 이월 상태의 개발 진행 확인 뒤 착수한다.

## 파일 변경표

| 구분 | 파일 | 목적·변경 |
|---|---|---|
| 생성 | [completion/mod.rs](../../../crates/dot-logger/src/completion/mod.rs), [types.rs](../../../crates/dot-logger/src/completion/types.rs), [query.rs](../../../crates/dot-logger/src/completion/query.rs), [tests.rs](../../../crates/dot-logger/src/completion/tests.rs) | 추적 예약·대상별 결과·이력/집계·유한 조회/대기·publication 경계 검증 |
| 생성 | [control/mod.rs](../../../crates/dot-logger/src/control/mod.rs), [types.rs](../../../crates/dot-logger/src/control/types.rs) | 명시 barrier·기한·guard·정상 종료 coordinator |
| 생성 | [control/tests.rs](../../../crates/dot-logger/src/control/tests.rs) | RV-005 결정적 게시/종료 경합3개·스레드별 test-only scheduler |
| 생성 | [tc_003_completion.rs](../../../crates/dot-logger/tests/tc_003_completion.rs), [tc_003_control.rs](../../../crates/dot-logger/tests/tc_003_control.rs), [support/mod.rs](../../../crates/dot-logger/tests/support/mod.rs) | 완료/제어 통합26개와 공통 구성 |
| 수정 | [capture/mod.rs](../../../crates/dot-logger/src/capture/mod.rs), [types.rs](../../../crates/dot-logger/src/capture/types.rs), [tests.rs](../../../crates/dot-logger/src/capture/tests.rs) | completion 연결·정확한 거절/게시·close 경합·메모리 원본 분할 전달·기존 경계 fixture |
| 수정 | [routing/mod.rs](../../../crates/dot-logger/src/routing/mod.rs), [types.rs](../../../crates/dot-logger/src/routing/types.rs) | 유한 feedback·delivery 보고/Unknown·실제 drain 검사 |
| 수정 | [config.rs](../../../crates/dot-logger/src/config.rs), [lib.rs](../../../crates/dot-logger/src/lib.rs) | Completion 자원 코드·공개 모듈/현재 범위 |
| 수정 | [독립 consumer](../../../tests/api-consumer/src/main.rs), [allocator tests](../../../tests/rt-allocation/src/tests.rs) | tracked admission→routing→Written→명시 sync→shutdown·RT 0/0·반복 세션 계측 |
| 수정 | [CI](../../../.github/workflows/ci.yml) | 기존 raw-memory job에 완료·제어 Miri 대상 추가; 실행은 후속 검증 |
| 수정 | [도메인](../../03_design_artifacts/domain_model.md), [인터페이스](../../03_design_artifacts/interface_contract.md), [Task 목록](../task_registry.md), [MS 목록](../milestone_registry.md), [MS-002](../milestones/MS-002.md), 이 문서 | 구현/계약과 진행 상태 정렬 |
| 수정 | [전략](../../05_verification/test_strategy.md), [TC/추적표](../../05_verification/test_case.md), [EV](../../05_verification/verification_evidence.md), [공유 상태](../../06_evolution/project_state.md), [개인 상태](../../members/lim8603/workspace/my_state.md) | 현재 근거·미검증/승인·다음 시작점 |
| 생성 | [증거 폴더](../../06_evolution/imported_context/task_006_2026-10-28/), [세션 로그](../../members/lim8603/workspace/session_logs/session_2026-10-28_002.md) | Red/Green·환경/명령·hash와 작업 연속성 |

## 클래스·타입 변경표

| 구분 | 파일 | 타입 | 역할·변경 |
|---|---|---|---|
| 생성 | completion/mod.rs | Registry, ProducerState, RecordSlot, TargetCell, Retired, RoutePlan, Limits | 고정 등록/producer window·대상 atomics·retired counters·불변 계획/한도 |
| 생성 | completion/mod.rs | Completion, Reservation, ActiveCall, Publication, AbandonMarker | borrowed 조회/보고·단발 예약·call quiescence·일반 게시 소유권·최종 release의 sequence 표시 |
| 생성 | completion/query.rs | CompletionScope, CompletionQuery, QueryPermit | 고정 범위·유한 page/wait·동시 query 예약 |
| 생성 | completion/types.rs | Boundary, Boundaries, TargetContract, CompletionConfig | 지원/확인 경계·대상 완료 계약·명시 추적/조회/이력 제한 |
| 생성 | completion/types.rs | FailureReason, Update, QueryOutcome, RecordStatus, CompletionError | 확인 사실과 오류/폐기/불명/만료 분리 |
| 생성 | completion/types.rs | ProducerRange, TargetRequest, CompletionItem, QuerySummary, PageReport, WaitOutcome, WaitReport, CompletionTotals | bounded query DTO·wait snapshot·retired/live 대조 수량 |
| 생성 | control/mod.rs | ControlOwner, SyncWork, ControllerLease | 명시 제어 소유자·실제 IO 수명 guard·controller 연결/해제 소유권 |
| 생성 | control/mod.rs | SyncSubmission | RV-005 일반 sync 진입~게시/오류 반환의 단발 CAS·RAII 표시와 종료 정지 확인 |
| 생성 | control/tests.rs | SubmissionPause | test-only 스레드별 게시 중단/재개 barrier, cfg(test) 밖에서는 제거 |
| 생성 | control/mod.rs | AtomicRange, SyncTarget, SyncSlot, SlotPublication | 고정 sync cutoffs·target/operation lifecycle·일반 게시 소유권 |
| 생성 | control/types.rs | ControlConfig, SyncToken, SyncResult, SyncSnapshot, ControlProgress | 제한된 명령·session/epoch/generation·barrier 결과/진행 |
| 생성 | control/types.rs | ShutdownPhase, ShutdownPlan, ShutdownReport, ShutdownPoll, ControlError | 정상 종료 계획·불완전/실패/기한·구체 자원 보고 |
| 수정 | capture/mod.rs·types.rs | CaptureOwner, CaptureScope, CaptureControl, Producer, CaptureConsumer, CaptureRegistration, GeneralProducer, CaptureError, CaptureBuildError | tracked 생성·한도/정확한 loan 반환·call close 경합·registration 전달·추적 포화 일반 대기 |
| 수정 | routing/mod.rs·types.rs | Router, Dispatcher, Target, TargetWriter, TargetConsumer, Delivery, RoutingError | 초기 completion 계획 대조·feedback 보존/보고·구체 drain·guard 최종 결과 |
| 생성 | routing/mod.rs | Feedback, DrainState | dispatcher 결과 저장·coordinator 전용 비위조 자원 샘플 |
| 수정 | config.rs | Resource | control 하위 Completion 예약 부족 코드 |

## 리뷰 포인트

1. `CompletionCapacity`는 입력 공간과 별도의 추적 예약 실패다. Active/Updating 슬롯을 덮거나 accepted sequence를 증가시키지 않으며 raw 거절은 같은 loan이다.
2. 경계 확인·terminal 결과·이력 Expired·대기 timeout/cancel을 구분한다. Written+SyncFailure는 Written 확인을 지우지 않는다. page 일부·최고 sequence만으로 전체 완료를 판단하지 않는다.
3. 실제 IO guard/loan/feedback/reclaim가 남으면 Closed 성공을 반환하지 않는다. 기한 도달 뒤에도 owner를 유지하고 cleanup을 재개한다. 같은 target의 sync IO와 control 세대 재사용을 분리한다.
4. 로컬 대역 검증과 native/실제 파일·장비 검증을 구분한다. TASK-004/005 승인은 유지하며 TASK-006은 Human 리뷰 승인/native 대기다. TASK-007은 미착수다.


## 검증 결과

- 완료 통합12·제어14·private publication 경계2·신규 compile-fail2, 기존 소형/원본/분배 회귀 Pass.
- Rust1.85/1.96 각각16 feature·독립 consumer·RT guard·clippy/rustdoc/fmt Pass. Miri lib8+raw17+routing19+completion12+control14 =70×8 seeds(560) Pass.
- tracked RT allocation/deallocation0/0. 초기 구성→query/sync128회→shutdown/owner Drop의 요청 bytes peak11570 / concrete 예약14962 / 잔여0. allocator 내부/RSS/OS cache·실제 worker 비용은 범위 밖.
- [EV-005](../../05_verification/verification_evidence.md#ev-005--task-006-완료-추적명시-sync종료)에 source36개·환경/명령/로그 hash와 미검증을 보존한다. Human 구현 리뷰는 승인됐고 실제 writer/파일/OS sync·native·장비 검증은 잔여다.


## 승인·커밋·세션 마감 — 2026-10-28

Human이 구현 승인·commit/push·세션 종료를 요청했다. TASK-005/006 코드/테스트/CI를 `c98b7fa4ba8928137f95d40ac2aa693a541ea934`에 함께 커밋했고 EV-005의36 source hash와 commit blob이 일치한다. 기존 CI 사용량 소진 지시에 따라 커밋에 `[skip ci]`를 사용하고 CI를 조회/재시도하지 않는다. 초기 HTTPS 인증 차단은 gh 브라우저 인증과 Git credential helper 연결로 해소했다. pull --ff-only 후 코드·승인 문서 `136fe96`의 origin/main push를 완료했고 원격 ref 일치를 확인했다. 승인/검증/마감 문서는 별도 docs(cowork) 커밋으로 정리했다. TASK-004/005/006 native는 Not Run을 유지하며 세션을 마감한다.

## 최종 승인 — 2026-10-29

Human 「승인」은 직전 질문의 TASK-006 수정분 승인과 native 검증 이월 상태의 TASK-007 개발 진행 승인이다. EV-009 source38개/로컬 Pass를 유지하며 TASK-006은 native 후속 검증으로 In Progress다. commit/push 승인은 포함하지 않는다.

## 검증 환경·시점 변경 — 2026-10-31

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체 수락하며 UPS 관련 판정은 시뮬레이션을 사용한다. native GAP-004는 GitHub 공개 완료 후(T3) 진행하는 승인된 이월이다. 별도 장치/실물 UPS 확보를 대기 조건으로 요구하지 않는다. 현재 일반 커널 실행/RT 커널 설치를 구분하며 RT 실측은 해당 커널 부팅 확인 후 기록한다. 기존 로컬 Pass·Human 승인/미승인과 미실행 범위는 보존하고 이번 정책 변경만으로 Task를 Done 처리하지 않는다.

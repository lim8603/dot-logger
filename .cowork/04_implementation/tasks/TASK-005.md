# TASK-005 — 분배·출력 격리·예산

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-002 |
| Story | US-001·005 |
| 요구사항 | FR-007 FR-012 FR-013 NFR-005 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-004 |
| 예상 소요 | 1~2 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

2026-10-28 Human이 사용량 전량 소진에 따른 CI 차단을 확인하고 TASK-005 착수를 지시했다. TASK-004의 구현·로컬 Pass·Human 승인을 선행 근거로 개발을 진행하는 예외이며, TASK-004 native CI/GAP-004는 후속 검증으로 유지한다. TASK-004를 Done으로 바꾸거나 CI를 면제하지 않는다. 계정 재확인·동일 차단 재시도는 하지 않는다.

## 변경 대상과 첫 검증

구현: [`routing/`](../../../crates/dot-logger/src/routing/), `CaptureConsumer::peek`·등록 검증, `Resource::Output`, [`tc_003_routing.rs`](../../../crates/dot-logger/tests/tc_003_routing.rs), 독립 consumer·allocator 계측·CI recipe.

첫 실패/리뷰 조건: **파일 A 큐 정지에도 B enqueue 지속, 물리 bytes 중복 계상/누락 없음**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-003`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] record/byte 공정 순회, per-target 유한 큐·재시도·회수, 불명확한 손실 없음.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 리뷰 완료(2026-10-28 「승인」). 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-003](../../05_verification/test_case.md#tc-003). 제품 코드·테스트 실행 결과: [EV-004](../../05_verification/verification_evidence.md#ev-004--task-005-분배출력-격리예산). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 구현·검증 결과 — 2026-10-28

- Router가 owning output/retry endpoints를 보유하고 scope에서 dispatcher/대상별 reader를 분리한다. 원본 guard를 일반 경로에서 공유하고 마지막 guard 종료 후 기존 ReturnPending/한정 스캔으로 회수한다. private unsafe·RT 생산자/loan API·외부 dependency/lock은 변경하지 않았다.
- 명시된 record/byte quantum과 전체 turn 제한, 대상별 queue/retry/진행 IO slots·bytes, 시도/age/취소를 구현했다. Enqueued는 분배 경계이고 완료 성공이 아니다. event 버퍼 부족은 입력/재시도 유지, 실패 head는 report.fault와 함께 보존한다.
- 통합 19개·신규 compile-fail 3개, Rust 두 버전 각16 feature·독립 consumer·기존 RT allocation guard·clippy·rustdoc·fmt Pass. 원본 cross-thread IO guard 수명 포함 통합19 × Miri8 seeds Pass.
- allocator 계측: 초기 구성/Router 생성부터 256회 큐 포화(512 records)·scope/owner 해제까지 요청 bytes peak2368 / output 예약2816 / 잔여0. 시스템 allocator 내부 metadata/rounding·RSS·OS page cache·실제 worker 비용은 이 계측의 범위가 아니다.
- 수락 경계/완료 window·sync·실제 shutdown·파일 IO/OS별 native·장비 처리량/RT는 미검증 또는 후속 범위다. queue expiry/cancel은 reader/turn 구동이 필요하고 진행 IO guard는 강제 무효화하지 않는다. shared 전역 예산 포화는 입력을 지연할 수 있다.
- Human 구현 리뷰는 2026-10-28 「승인」으로 완료됐다. native CI 후속 검증이 남아 In Progress를 유지한다. 구현 검증 당시 공개 전환/과금 설정 변경/게시/커밋은 수행하지 않았다. 이후 승인된 TASK-005/006 소스는 2026-10-28 코드 `550185f`에 함께 커밋됐으며 gh/Git HTTPS 인증 연결 후 코드·승인 문서 `b3a0ca6`의 origin/main push와 원격 ref 일치 확인을 완료했다.

## 파일 변경표

| 구분 | 파일 | 목적·변경 |
|---|---|---|
| 생성 | [routing/mod.rs](../../../crates/dot-logger/src/routing/mod.rs), [routing/types.rs](../../../crates/dot-logger/src/routing/types.rs) | 일반 경로 공정 분배·대상별 큐/retry·guard·명시 결과·물리/보유 예산 |
| 생성 | [tc_003_routing.rs](../../../crates/dot-logger/tests/tc_003_routing.rs) | 분배/격리·예산·취소·수명 통합19개 |
| 수정 | [capture/mod.rs](../../../crates/dot-logger/src/capture/mod.rs) | 비소비 peek·세션/채널/생산자/실제 pool layout 등록 |
| 수정 | [pool/mod.rs](../../../crates/dot-logger/src/pool/mod.rs) | private raw layout 조회, 실제 풀과 다른 plan의 원본 예약 오집계 방지 |
| 수정 | [config.rs](../../../crates/dot-logger/src/config.rs), [lib.rs](../../../crates/dot-logger/src/lib.rs) | Output resource·routing 공개·구현 단계 설명 |
| 수정 | [독립 consumer](../../../tests/api-consumer/src/main.rs) | 외부 패키지 capture→routing→target guard 실행 |
| 수정 | [allocator tests](../../../tests/rt-allocation/src/tests.rs), [allocator lib](../../../tests/rt-allocation/src/lib.rs) | 요청 bytes 계측·반복 포화/최종 해제, 기존 RT 무할당/무해제 감시 유지 |
| 수정 | [CI](../../../.github/workflows/ci.yml) | 기존 raw-memory job에 routing Miri 대상 추가, 실행은 후속 검증 |
| 수정 | [도메인](../../03_design_artifacts/domain_model.md), [인터페이스](../../03_design_artifacts/interface_contract.md) | 실제 구현 범위·소유권·한계 연결 |
| 수정 | [MS 목록](../milestone_registry.md), [MS-002](../milestones/MS-002.md), [Task 목록](../task_registry.md), [TASK-004](TASK-004.md), 이 문서 | 선행 CI 예외·착수/로컬 Pass·리뷰 대기 정렬 |
| 수정 | [전략](../../05_verification/test_strategy.md), [TC/추적표](../../05_verification/test_case.md), [EV](../../05_verification/verification_evidence.md) | 실행 범위·미실행·근거·GAP-004 정렬 |
| 수정 | [공유 상태](../../06_evolution/project_state.md), [개인 상태](../../members/lim8603/workspace/my_state.md) | 다음 시작점·Human 확인 원인·T2 공개 준비 |
| 생성 | 증거 폴더(비공개 원시 근거: `imported_context/task_005_2026-10-28/`), 세션 로그(비공개 세션 로그: `session_logs/session_2026-10-28_001.md`) | source/log hash·환경/명령/Red/Green·결정/재개 기록(로그는 기존 gitignore 적용) |

## 클래스·타입 변경표

| 구분 | 파일 | 타입 | 역할·변경 |
|---|---|---|---|
| 생성 | routing/mod.rs | Router, Dispatcher, TargetConsumer, Delivery | owning 큐·일반 분배·독립 reader·capture 수명에 묶인 불변 IO guard |
| 생성 | routing/mod.rs | Input, Cursor, Limits | producer별 deficit·순회/시간/close·공유 및 work 한도 |
| 생성 | routing/mod.rs | Target, TargetWriter, Pending, PendingQueue | 대상별 owning/borrowed endpoints·유한 retry ring |
| 생성 | routing/mod.rs | Payload, SharedPayload | raw guard 또는 소형 복사 하나의 공유 불변 payload |
| 생성 | routing/mod.rs | SharedLedger, SharedPermit, TargetLedger, TargetPermit | distinct 물리/복사 예약과 대상별 보유 원인의 분리 집계·Drop 반환 |
| 생성 | routing/types.rs | Route, TargetConfig, Schedule, RoutingConfig | 초기 revision 대상 계획·명시 quota/시도/age/공정 순회 구성 |
| 생성 | routing/types.rs | RoutingError, DropReason, DispatchOutcome, DispatchEvent | startup/불변식 오류·대상별 enqueue/실패/정책 폐기 결과 |
| 생성 | routing/types.rs | TurnReport, RoutingSnapshot, TargetSnapshot | work/event prefix/fault와 물리/복사/큐/retry/진행 IO 샘플 |
| 수정 | capture/mod.rs | CaptureScope, CaptureConsumer | 소비자 등록 전달·FIFO peek·실제 pool layout 조회 |
| 생성 | capture/mod.rs | CaptureRegistration | 세션/채널/생산자/revision·concrete small/raw slots/slot bytes의 내부 등록 값 |
| 수정 | pool/mod.rs | RawPool | private layout 조회만 추가, payload/state/Safety 알고리즘은 유지 |
| 수정 | config.rs | Resource | concrete output 예약 부족을 나타내는 Output 추가 |
| 수정 | tests/rt-allocation/src/tests.rs | CountingAllocator | 기존 호출 횟수에 요청 bytes의 순증/peak 계측 추가(시험 전용) |

## 리뷰 포인트

1. Enqueued와 최종 완료/쓰기/영속 성공의 경계를 구분한다. future completion 소비자는 event prefix와 queued expiry/cancel event를 모두 수집해야 한다.
2. 취소 후에도 실제 진행 IO guard가 남으면 원본/대상 예산을 반환하지 않는다. queue 취소/age 배출은 target reader를 구동해야 한다.
3. 원본은 capture의 물리 예약으로 한 번만 집계하고 output별 retained bytes는 보유 원인이다. 일반 경로 공유 header/소형 복사는 output 예산, allocator 내부 비용은 worker headroom이다.
4. TASK-004 승인은 유지, TASK-005 Human 리뷰는 승인됐고 native는 대기다. TASK-006 및 실제 파일 IO·장비 성능은 미착수/미검증이다.

## 검증 환경·시점 변경 — 2026-10-31

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체 수락하며 UPS 관련 판정은 시뮬레이션을 사용한다. native GAP-004는 GitHub 공개 완료 후(T3) 진행하는 승인된 이월이다. 별도 장치/실물 UPS 확보를 대기 조건으로 요구하지 않는다. 현재 일반 커널 실행/RT 커널 설치를 구분하며 RT 실측은 해당 커널 부팅 확인 후 기록한다. 기존 로컬 Pass·Human 승인/미승인과 미실행 범위는 보존하고 이번 정책 변경만으로 Task를 Done 처리하지 않는다.

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- native·다른 OS: [TASK-024](TASK-024.md)(T3, GAP-004)
- 이 PC 정량 성능·RT: [TASK-023](TASK-023.md)에서 수행한 범위(EV-064~070)

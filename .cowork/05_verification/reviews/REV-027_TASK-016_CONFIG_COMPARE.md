# [REV-027] TASK-016 revision 설정 변경·전후 비교 개발 자가 점검

## 개요

- 회차: REV-027 / 2026-11-01 #002 / AI 개발 자가 점검. 독립 리뷰로 표현하지 않는다.
- 요청: Human 「TASK-016 진행하자」. 기존 구현·검증·리뷰 수락·분리 커밋/푸시 위임 적용.
- 대상: control/config_update.rs·telemetry/compare.rs, capture/completion/routing/pool/telemetry 연결·MCAP revision budget·외부 consumer·RT allocator.
- 상태: Closed — EV-030/source542c6e6·source102개·검사10개·Miri496·commit blob102개 일치, 기존 위임에 따른 AI 로컬 수락/마감.
- 최종 판정: Pass — 신규 미해결 P1/P2/P3 지적 없음. native/실측/실앱·운영 backend는 별도 후속이다.

## 요약표

| ID | 중요도 | 위치·계약 | 문제/제안 | 처리·검증 | 상태 |
|---|---|---|---|---|---|
| 신규 RV 없음 | — | TASK-016 로컬 범위 | 승인 계약을 TDD로 구현·개발 자가 점검 | EV-030 최종 회귀 Pass/위임 수락 | Closed |

## [리뷰어] 검토

| 영역 | 판단·근거 |
|---|---|
| RT 경계 | Producer small/raw 제출과 명시 poll은 고정 배열·atomic 접근만 한다. 새로운 mutex/clock/할당/해제/IO/콜백/무한 재시도 없음. 관리 Mutex 및 PlanGuard 증감은 일반 경로다. RT allocator 변경 적용/샘플링/raw/오류/Drop 관측 0/0 |
| 게시/동시성 | 관리 Mutex가 base 검사부터 게시까지 직렬화. 모든 bank 내용을 채운 뒤 revision/posted Release, producer Acquire 뒤 ack Release. 동일 base 동시 apply는 성공1/Conflict1. 전역 동시 적용으로 표현하지 않음 |
| 회수/수명 | 모든 producer applied ack와 이전 accepted_seq cutoff의 소비 watermark, old completion terminal, raw/공유 payload 실제 Drop가 필요. retain은 ingress 소비 Release 전에 생성하고, 관리 ready는 consumed Acquire 뒤 guard를 읽음. raw access/shared payload가 먼저 Drop되고 plan permit이 나중에 반환됨. idle/forget은 보수적 Busy |
| 기존 수락 불변 | CapturedRecord의 admission revision·Completion slot의 revision/target 선택을 고정하고 Router는 해당 revision route를 사용. 변경 중 기존 큐·active IO를 재분류/무효화하지 않음. target/required/boundary·schema·pool/worker는 고정 |
| 예산/오류 | 두 bank/유한 target matrix·progress와 sampling 배열을 checked 예약. control 부족은 enable 전 무변경 실패. 미등록/중복/0/빈 변경·feedback 초과·짧은 목적지·Conflict/Busy/최대 revision/닫힘/재시작을 구분. 최종 owner만 general Drop |
| 샘플링 | 생산자/채널별 유효 Normal 제출 시도 위상. Important 우회·level 필터는 명시 적용. invalid schema/길이/loan ownership은 skip으로 숨기지 않고 raw 실패/skip은 같은 loan 반환. scope 재대여와 이전 capture 후 enable watermark도 확인 |
| 계측/비교 | 전환 sample은 Unavailable(RevisionTransition), 혼합 revision Window 없음. 새 baseline 이후 비교. 원래 profile provenance/수치·실제 구간·skip/거절/출력 손실을 보존하며 입력/장비/workload/target/boundary/scope 차이는 비비교 가능. stale/누락/예측을 절감0으로 만들지 않음 |
| 저장/소비자 | 단일 revision MCAP 계약을 유지하고 명시 writer_budget_for_revision으로 새 writer를 생성. 실제 파일 record/metadata roundtrip 확인. API consumer의 Report→Snapshot 비교, 기존 query/replay/recovery/control·remote/client/CLI 종단 회귀를 EV-030에서 확인 |
| 범위/ADR | 승인된 두 세대/producer 경계 적용과 기존 target 범위의 구현 세부. 신규 dependency/unsafe/wire/핵심 기술·프로토콜 변경 없음. 새 ADR 승격 사유 없음 |

## [처리담당] 처리·검증

- 최초 publication 구현의 고정 Unavailable stub에서 Conflict/부분 ack/old Delivery Busy 3개가 의도한 assertion Red 후 Green.
- transition 표본이 Observed로 잘못 반환되는 재현 Red 후 RevisionTransition/동일 revision Window 조치·Green.
- before/after 비교는 API 부재 compile Red 후 관측 rate/입력 차이/잘못된 자원 Window·Stale 시나리오 Green.
- 자가 점검 중 scope 재대여 sampling 위상·이전 capture 후 enable의 소비 watermark를 보강했다. 최종 테스트에 두 경계와 실제 동시 게시/생산자 경합을 포함했다.
- 신규 설정 13개·비교 3개·실제 파일1개·RT allocation1개. 원래 완료/라우팅/원본/계측·기존 클라이언트/CLI 회귀는 EV-030 참조.

## 파일 변경표

| 구분 | 파일 | 목적 |
|---|---|---|
| 생성 | crates/dot-logger/src/control/config_update.rs | dry-run/preview·명시 게시·두 bank·ack/수명 |
| 생성 | crates/dot-logger/src/telemetry/compare.rs | 조건 차이·증거 보존·전후 rate/자원 비교 |
| 수정 | capture/{mod,tests}.rs·completion/mod.rs·pool/mod.rs·routing/mod.rs | 고정 sampling·revision route/완료·실제 guard·writer budget |
| 수정 | control/mod.rs·telemetry/{mod,aggregate,capacity}.rs | 공개 모듈·전환 관측·혼합 구간 거절·비교 사유 |
| 생성/수정 | tc_009_config_update.rs·tc_009_compare.rs·tc_004_storage.rs·rt-allocation/src/tests.rs·api-consumer/src/main.rs | TDD/동시성·실제 파일·외부 API·무할당 검증 |
| 수정 | API/도메인/데이터/UI·TASK/MS/추적/EV/project_state·본 리뷰 | 구현 계약·근거·미검증 정렬 |

## 클래스(타입) 변경표

- 생성 공개: ChannelPatch·ConfigPatch·Validation·ConfigUpdateError·ConfigUpdates, Resources·Snapshot·Differences·Direction·RateChange·Comparison.
- 생성 private: Policy·Bank·Progress·Management·State·PlanGuard.
- 수정: CaptureOwner/Scope/Producer/Consumer/Registration·WriteState/ReadState(고정 배열/소비 watermark), Completion Registry/RecordSlot/Reservation(수락 revision/target), Router/Dispatcher/SharedPayload·RawRecord(원래 bank 수명), Telemetry/History·Unavailable/Reason(전환/비교 상태).
- 기존 TargetContract·RecordMetadata·CaptureReceipt·wire·MemoryBudget·Report/Window의 공개 필드 구성은 그대로다. Resources는 별도 Window/measurement scope를 보존한다.

## 미검증/다음 행동

원격 ApplyConfig/Compare 운영 backend·CLI runtime 연결/자동 worker·실앱, 실제 PC/RT 호출 지연/계측 비용≤5%/오차≤20%·native(T3)/다른 OS·UPS/24h·fresh audit는 NotRun이다. 단일 revision 파일 전환은 호스트가 구동해야 한다. TASK-017은 다음 후보 Planned이며 자동 착수하지 않는다. TASK-016/MS-005·TC-009 전체·Gate4/5 미완료를 유지한다.

## 최종 검증·수락

[EV-030](../verification_evidence.md#ev-030): EV-030/source542c6e6·source102개·검사10개·Miri496·소스/commit blob102개 SHA-256 일치. Rust1.96.0/MSRV1.85.0 각각16feature·독립 core/remote consumer·RT allocator15개·workspace/all-features·clippy/rustdoc -D warnings와 fmt 검사10개 exit0. Miri strict provenance lib17+routing19+config13+compare3+telemetry10=62×8seeds=496 Pass. RT 신규 경로 할당/해제0. 원격 운영 backend/실앱·실측/native 미완료와 Gate4/5 Not Ready를 유지하며 개발 자가 점검/로컬 범위만 수락한다.

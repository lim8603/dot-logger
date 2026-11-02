# TASK-020 — tracing 어댑터와 dot-probe 통합 예제

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-006 |
| Story | US-001·002·003·004 |
| 요구사항 | FR-001 FR-002 FR-004 FR-006 FR-017 NFR-004 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-02 |
| 다음 액션 | S1~S3 EV-040~042 로컬 수락, 통합/환경 TASK-026/022·native T3 대기. 다음 구현 TASK-021 |
| 의존 | TASK-013·TASK-016·TASK-019 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

S1 구현: `crates/dot-logger/src/adapters/tracing.rs`, `examples/general_logs.rs`, `crates/dot-logger/tests/tc_007_tracing.rs`. S2는 `crates/dot-logger-client/examples/dot_probe_pipeline.rs`와 shared `examples/support/pipeline/`·`tests/pipeline.rs`에 구현했다. 기존 client의 단일 snapshot 예제와 구분한다.

첫 실패/리뷰 조건: **일반 tracing을 RT 경로로 사용하지 않음·화면 client 데이터 흐름**.

- [x] S1 API 부재 compile Red 확인(EV-040).
- [x] S1 TC-007 하위 신규8개 통과. S2/S3와 TC 전체는 별도 잔여.
- [x] S1 두 Rust workspace/16features·consumer/RT/lint/doc/fmt·예제 확인(EV-040).

## 완료 기준

- [x] S2 대표 합성 로그/수치/raw→실제 파일/DB/구독·명시 설정 비교(EV-041). 실제 dot-probe 수정은 TASK-026, S3는 아래 EV-042로 수락했다.
- [x] S1~S3 로컬 대상 revision·환경·명령·미실행 및 MS-006 하위 근거를 EV-040~042/추적표에 기록했다. 전체 MS 검증은 미완료다.
- [x] S1~S3 로컬 실패/포화·취소/종료·예산 경계 검증. 실앱/환경 검증은 별도 대기다.
- [x] 기존 2026-10-31 Human 위임으로 S1~S3 로컬 구현/조치 수락·REV-039~041 Closed. 실제 통합/환경 수락은 대기다.

테스트 정의: [TC-007](../../05_verification/test_case.md#tc-007). S1 제품 코드·검증 근거: [EV-040](../../05_verification/verification_evidence.md#ev-040). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## TASK-013 REV-022에서 넘긴 화면 수락 조건 — 2026-10-31

원격 구독의 ttl은 서버 idle lease를 겸하고 heartbeat가 없다. 로컬 next wait timeout은 같은 구독에서 재대기할 수 있지만 서버 lease/연결 종료 뒤 새 구독 사이에는 기록 복원/Gap 연속성을 보장하지 않는다. 화면은 종료·새 구독 경계를 불연속으로 표시해야 한다. SDK/EV-026의 제한을 소비하고 장기 tail·연속성 수락을 따로 검증한다. heartbeat/lease wire 확장은 필요성 판단 및 H/J 결정 후 진행한다.

## 단계별 실행 — 2026-11-01 #014

Human 「다음 작업 시작하자」로 REV-038 조치 후 착수했다. 선행 TASK-013/016/019 로컬 수락과 native/실측 이월 상태의 후속 개발 위임을 사용한다.

| 단계 | 업무 | 상태/남은 조건 |
|---|---|---|
| S1 | 일반 경로 tracing/log 이벤트 유한 인코딩·수집 연결 | 로컬 수락 — EV-040/REV-039 Closed |
| S2 | 대표 로그/수치/합성 원본의 파일·DB·구독/조절 비교 예제 | 로컬 수락 — EV-041/REV-040 Closed |
| S3 | 구독 종료/재연결 불연속 표시 예제·검증 | 로컬 수락 — EV-042/REV-041 Closed, RV-053 화면 예제 후속 소비 |
| 통합/환경 수락 | 지속 dot-probe/화면·PC/RT/native | 대기 — TASK-026/022·T3 |

TASK-018은 별도 FR-015 범위 승인 대기로 미착수다. S1 수락은 Task 전체 완료가 아니다.

## S1 로컬 수락

- API §14.9의 caller scratch·UTF-8 JSON·모든 값은 text·own fields 한정·명시 capture 계약을 구현했다. native severity만 변환하며 시각/schema/중요도·receipt 의미를 유지한다. Buffer overflow/formatter 오류는 전체 거절한다.
- `tracing_event`/`log_record`는 기존 application subscriber/sink에서 호출하는 훅이며 global Subscriber/Log 구현을 설치하지 않는다. 사용자 formatter에 RT/시간 보장을 적용하지 않는다. 새 worker/heap 큐·의존 버전/lock 변경 없음.
- 기능 부재 compile Red→신규8개 Green. Rust1.85의 예제/시험 format_args E0716은 RV-081에서 동일 호출 소비로 수정했다. 두 Rust workspace각409개/실제 DB4개 ignore, 각각16features/독립 consumer/RT allocator·fmt/clippy/doc/예제 Pass. EV-040·REV-039/RV-081 Closed, 기존 위임으로 S1만 수락했다.
- 다음은 S2 대표 로그/수치/합성 원본→파일·DB·구독/조절 비교 예제다. S3 재연결 불연속·TASK-026 실앱·PC/RT/native(T3)·Task/MS/TC 전체·Gate4/5는 미완료다.

## S2 로컬 수락 — 2026-11-01 #015

- `dot_probe_pipeline --plan`은 환경/파일/DB/소켓 접근 없는 순수 계획이며, 명시 `--run <fresh-exclusive-measurement>`만 유한 합성 입력을 실제 MCAP·DB·loopback gRPC로 연결한다. 원 receipt/metadata/bytes·수치 query의 receipt/Unix ns/값을 대조한다. 자기 소유 임시 파일·서버/IO를 정리하고 예약0을 검사한다.
- 같은 생성 입력30건에서 before 수락30/skip0·파일/구독30/DB20, after 수락15/의도적 skip15·파일/구독15/DB할당0을 관측했다. raw20건의 실제 회수·파일/수신45건·DBnumeric10건을 확인했다. 변경 후 DB off는 초기 revision DB 계약을 유지하기 위한 명시 route 변경이다.
- session/epoch·revision/구간·환경/workload/target/boundary/input ID를 JSON에 보존한다. 누락 Red→Green은 RV-082다. 가상 mono 시계의 기능 창이며 fanout 차이는 Comparison workload 차이/Unavailable로 남긴다. 실제 처리량·CPU/RSS/RT/≤5% 비용/≤20% 오차 수락이 아니다.
- 일반4개+실제 DB1개. 두 Rust workspace각413개/실제DB5개 일반회귀 ignore, 별도 고정 OSS2.9.1 harness각 core4+pipeline1·CLI Pass/cleanup0. default/remote/MSRV default client·독립 graph·fmt/clippy/doc·CLI0/3/4 확인. 제품 core/client src 변경 없음, lock 버전/checksum 동일. EV-041·REV-040/RV-082 Closed·기존 위임 수락.
- 다음은 S3 구독 종료/재연결 불연속 표시 예제·검증이다. Task/MS/TC 전체·실제 지속 앱/화면/PC·RT/UPS/장기/native(T3)·Gate4/5 미완료를 유지한다. TASK-018은 별도 FR-015 승인 대기로 미착수다.

## S3 로컬 수락 — 2026-11-02 #001

- `dot_probe_tail --plan`은 무부하 계획, 명시 `--run`은 유한 합성 source와 실제 loopback TCP/gRPC의 한국어 구분선·JSON 화면 이력이다. local/server TimedOut 출처를 단정하지 않고 Unconfirmed로 표시하며 같은 구독 재대기를 유지한다. lease 뒤 None, 명시 취소/오류는 종료 구분선, 명시 새 연결/구독은 새 segment·연결 금지·누락 건수 Unknown이다. 구독 내 Gap만 inclusive transfer 범위·새 span으로 표시한다.
- 고정 실행: 같은 구독 기록2건/새 구독1건 ACK, overflow transfer2 Gap, 취소 종료2개 구분선. 화면 이력 포화·부모 연결 종료에서는 미ACK→원 PendingAck Unknown, 실제 owner/runtime 종료 뒤 external0을 확인했다. 큐 인계와 렌더링 완료를 구분하며 ACK unknown은 이미 받은 행을 보존한다.
- 화면 최대64행/실행32행·payload64B·수신1슬롯·종료 전용1행, 시나리오 대기10초·서버 정리 관측2초·SDK250ms·lease1500ms다. 별도 JSON/runtime/OS headroom과 선언8MiB/external4MiB를 구분한다. 무제한 retry/재전송/복원·heartbeat는 없다.
- RV-083 큰 u128 schema JSON panic을 실제 Red→hex/큰 정수·시각 문자열/source clock 구조 보존 Green으로 조치했다. 기능/renderer 부재 Red 후 신규8개, 두 Rust workspace각421개(일반 회귀 DB5개 ignore)·default/remote·fmt/clippy/doc·CLI plan0/run0/invalid4/stdout-full4·default dependency graph Pass. source `0b02274`·검증파일134개 commit blob 일치, 제품 src/lock 불변. [EV-042](../../05_verification/verification_evidence.md#ev-042)·[REV-041](../../05_verification/reviews/REV-041_TASK-020_TAIL_SCREEN.md) Closed·기존 위임 로컬 수락.
- S1~S3 로컬 구현 잔여를 마쳤지만 TASK-020 In Progress·TC-007 Partial·MS-006/전체 Gate4/5 미완료다. 실제 지속 dot-probe/화면·장기 tail/재구독 복원·PC/RT/UPS/성능/native(T3)는 미수락이다. 새 Miri/실제DB 재실행은 하지 않았다. 다음 구현 단위는 TASK-021, TASK-018은 FR-015 별도 승인 대기로 미착수다.

# TASK-017 — 예산 제한 간편 점검

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-005 |
| Story | US-004 |
| 요구사항 | FR-016 NFR-005 NFR-006 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-01 |
| 다음 액션 | 고정 routed RunCheck 구현/기능 검증 EV-035; 지속 실앱/화면(TASK-020/026)·통합 실측(TASK-021~024)·공개 후 native(T3) 수락 대기 |
| 의존 | TASK-015·TASK-013 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: `crates/dot-logger/src/diagnostics/{mod,run,file}.rs` · `tests/tc_010_{diagnostics,file_check}.rs`. S1/S2 및 S3 로컬 host·CLI까지 수락. 추가 구현: `diagnostics/host.rs`·remote/native codec·CLI·`tc_010_check_host.rs`/CLI `check_host.rs`. 생산/실앱/실측 후속이다.

첫 실패/리뷰 조건: **dry-run 무부하·쓰기/시간 예산 선도달·수신자 없음·취소**.

- [x] S1의 무부하 계획·예산 판정을 API 부재 compile Red → 최소 구현 Green으로 확인했다.
- [x] S1 하위5개 통과. 실제 IO/실행/정리를 포함한 TC-010 전체는 Partial이다.
- [x] S1 workspace/all-features·MSRV1.85 관련22개·fmt/clippy/rustdoc와 API 문서를 확인했다.

## 완료 기준

- [x] 로컬 S2 파일 scope의 계획→명시 실행→해석, 전용 경로 정리·운영 기록 보존·Inconclusive 구분.
- [x] S2 대상 revision·환경·명령·결과/미실행을 EV-032/추적표에 기록했다. 전체 MS/TC 완료는 올리지 않는다.
- [x] S2 파일 scope 실패·취소·정리·메모리/시간/쓰기/reserve 경계 검증. 생산 부하/실측은 후속이다.
- [x] 기존 위임에 따른 AI S2 개발 자가 점검/로컬 수락. 직접 Human 추가 리뷰·전체 제품 수락을 뜻하지 않는다.

테스트 정의: [TC-010](../../05_verification/test_case.md#tc-010). S1 코드/검증: [EV-031](../../05_verification/verification_evidence.md#ev-031)·[REV-029](../../05_verification/reviews/REV-029_TASK-017_PLAN_START.md), 소스0622a90. S1 당시 계획 단위를 기존 위임으로 수락했으며 현재 S2 추가 수락은 아래 기록을 따른다. Task 전체 완료/실제 성능 인증은 아니다. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 작업 단계와 현재 계약 — 2026-11-01 #004

- [x] S1 — 무부하 `CheckPlan::build`·원본 provenance/명시 예산 보존·메모리 선언/경로 문법 검증·`write_allowance` 순수 계산. 신규5개 Red→Green, [API §15.3](../../03_design_artifacts/interface_contract.md#153-task-017-s1-로컬-계획예산-판정--2026-11-01).
- [x] S2 — plan 뒤 별도 명시 run, 배타적으로 소유한 전용 경로만 생성/정리, bounded workload·실제 physical write/reserve·운영 기록 불변, budget/cancel/IO 실패 뒤 drain/sync/close·최종 범위/원인·Pass/Fail/Aborted/Inconclusive.
- [ ] S3 — 실제 Report/측정 provenance·CLI/runtime/원격 backend 연결 및 실제 수신자·실앱/PC/RT/native 검증. 동일 scope에 필요한 구현 단위로 진행하고 FR-015/TASK-018은 별도 승인 전 착수하지 않는다.

S1의 `required_memory_bytes`는 host의 예약 계산 선언이지 실제 예약/측정 증명이 아니다. 예산 판정은 host 관측의 stateless 계산이며 reservation/IO 취소를 수행하지 않는다. final drain/index/footer/outstanding bytes까지 미리 예약하고 신규 IO를 직렬화하는 host 책임을 API에 명시했다. 네트워크는 미측정/loopback/remote 요청 선언만 구분한다. S2에서는 아래 수락 기록처럼 실제 종료·정리와 함께 검증했으며 운영 host 연결은 S3에 남아 있다.

TASK-013/015 로컬 수락을 선행 근거로 착수했다. 운영 backend/실앱/실측/native 잔여를 완료로 간주하지 않고 순수 로컬 S1에 한정했다. 새 H/J 결정·새 dependency/unsafe/wire/공유 문서 구조 변경 없음. Task/MS/TC 전체·Gate4/5 미완료 유지.

## S2 로컬 수락 — 2026-11-01 #005

- [EV-032](../../05_verification/verification_evidence.md#ev-032)·소스1cae3b9·[REV-030](../../05_verification/reviews/REV-030_TASK-017_FILE_RUN.md) Closed. [API §15.4](../../03_design_artifacts/interface_contract.md#154-task-017-s2-로컬-합성-파일-점검--2026-11-01).
- 단일 호출 worker·전용 synthetic MCAP file scope. 신규 파일15개+짧은 쓰기/seek 재쓰기/부분 OS 오류/상한 내부3개, API/settlement/identity Red→Green. 단조 실제 시계와 실제 임시 file IO·footer/sync/close/정리, 기존 파일 bytes 보존, writer 예약 회수 확인.
- workspace/all-features347개(문서 시험 포함), Rust1.96/MSRV1.85 관련40개 각각, fmt/clippy/rustdoc·Miri3종×8seeds=24 Pass. source4개 commit blob 일치.
- `required_memory_bytes`에 writer+payload/run/path 보수 예약이 맞는지 preflight하고 writer의 실제 공유 output 예약을 사용한다. 물리 예약 상한·실제 Write bytes/free-reserve backstop, footer/부분 범위·primary/settlement/cleanup 오류를 보존한다.
- Written/Synced 외 경계·loopback/remote는 Inconclusive/무실행. 임시 파일의 직접 writer 시험이며 capture/routing·생산자 완료 ACK/원격 성능 인증이 아니다. 기능 시험의 공간 probe는 제어된 fixture이며 실제 운영 OS probe는 host/S3가 공급해야 한다.
- 다음 S3: 실제 free probe/capability·Report/profile/CLI/원격 host 연결과 생산 부하를 별도 연결한다. 기존 임의 경로/무제한 remote 실행 금지·예산·Unsupported 계약을 유지한다. 실앱/PC/RT/UPS/장기/native(T3)와 Task/MS/TC 전체 미완료 유지. 새 dependency/unsafe/wire/ADR·H/J 결정 없음.

## S3 로컬 운영 연결 수락 — 2026-11-01 #006

- 소스 `5390a55194cf5bf80ecc74f461e24a07d8f6af10`·source101개 commit blob/hash 일치, 검사24개 exit0.

- [EV-033](../../05_verification/verification_evidence.md#ev-033)·[REV-031](../../05_verification/reviews/REV-031_TASK-017_CHECK_HOST.md) 개발 자가 점검, [API §15.5](../../03_design_artifacts/interface_contract.md#155-task-017-s3-로컬-운영-hostcli-연결--2026-11-01).
- [x] 고정 슬롯·checked 증가/단일 사용 capability, host만 경로/예산 등록, 무부하 PlanCheck/CLI preview, 별도 RunCheck Manage 실행.
- [x] 실제 Linux statvfs·원 OS 오류/checked available bytes, probe 뒤 시간/취소 재검사. 안전한 rustix1.1.4·lock 신규2개, 제품 unsafe/RT 변경 없음.
- [x] 원래 Report/profile·session/revision/구간/조건·만료를 native로 원문 보존하고 CLI 요약에 출처를 표시. 합성 결과를 capacity로 승격하지 않음.
- [x] 실제 CLI→TCP/gRPC→host→MCAP footer/sync/close/cleanup. read/manage·단일 사용·메시지 부족 pre-IO·timeout 중 pacing 취소·기존 경로 보존·Fail/Inconclusive exit 구분 검증.
- [ ] 생산/실앱 전체: #007 snapshot/ACK 및 #008 고정 routed backend(EV-035)는 수락. 지속 수집/화면·PC/RT 비용/오차/성능·UPS/장기/native(T3)/다른 OS 수락 잔여.

S3 전체 완료 체크는 위 후속 때문에 미완료로 유지한다. 기존 종료·수락 범위를 실제 장비 능력 검증으로 확대하지 않는다. 자동 worker/상시 서버·임의 remote 경로·무제한 실행·FR-015/TASK-018은 도입하지 않는다.

## S3 생산 모듈·실앱 snapshot 연결 첫 단위 — 2026-11-01 #007

- [EV-034](../../05_verification/verification_evidence.md#ev-034)·[REV-032](../../05_verification/reviews/REV-032_TASK-017_ROUTED_BRIDGE.md) Closed, 기존 위임으로 이 실행 단위만 로컬 수락. 다음은 고정 host의 routed RunCheck 등록/출처·실앱 지속 수집 및 화면 연결이다.
- [실행 예제](../../../crates/dot-logger-client/examples/dot_probe_bridge.rs)·[host 흐름](../../../crates/dot-logger-client/examples/support/dot_probe.rs)·[통합 시험](../../../crates/dot-logger-client/tests/routed_check.rs). core/RT/unsafe·dot-probe 제품 코드·wire 변경 없음. serde_json은 이미 고정된 workspace 버전의 예제/시험 dev 의존만 추가했다.
- [x] 실제 `CaptureOwner/Producer::try_record_raw` 수락 receipt·`Router` Delivery guard·`RemoteHub::offer_delivery`·실제 TCP/gRPC 수신/조립·1슬롯 소비 큐 인계/ACK·원 receipt ReceiverAcknowledged·raw 회수·실제 연결 종료/예약 반환을 연결했다. 수신/조립만으로 Pending이 바뀌지 않고 raw 슬롯이 유지된다.
- [x] 입력≤64KiB/128 thread, 슬롯1·logger budget16MiB·current-thread worker1·2초 신규 수집/인계 구간. 취소/수신자 없음/시간 부족 무수집, 큐 포화/연결 종료 Unknown, 소비 수락 후 ACK 불명도 원 데이터는 유지하되 완료 추정 없음. 명시 `--plan` 무부하, `--run` 별도 실행.
- [x] 실제 dot-probe `269d348`의 `robot-serve` 실행·기존 `PROBE_TICK_REPORT` export 1,618B 생성·SIGTERM 정상 종료(exit0) 후 해당 원문을 전달했다. SafetyThread120/ControlThread60/battery_main24 ticks는 입력 내용이며 logger 비용/RT 달성 근거로 쓰지 않는다. [원문·명령·결과](../../06_evolution/imported_context/task_017_routed_2026-11-01/results.json).
- [x] 신규7개 Red→Green, Rust1.96/MSRV1.85 workspace 각364·fmt/clippy/doc/protocol/core 의존 분리·실제 예제 실행 Pass. 개발 자가 점검 미해결 P1/P2/P3 없음.
- [x] #008의 `register_routed/run_routed`·`handle_command`로 고정 routed backend/원 Report·기존 CLI 관리 권한을 연결했다(EV-035). EV-033 file backend와 별도 bridge 예제도 유지하며 자동 worker는 도입하지 않는다.
- [ ] export snapshot 연결은 dot-probe 내장 상시 capture·기존 화면·off/on 비교·TASK-020/026 첫 적용 완료가 아니다. T1은 미도래다. source freshness/boot identity·capacity/network/RSS 측정은 Unavailable. LAN·실제 PC/RT 오차/비용·UPS/장기/native(T3)/다른 OS 및 Task/MS/TC 전체·Gate4/5 미완료 유지.


## S3 고정 routed RunCheck 구현·잔여 조건 정리 — 2026-11-01 #008

요청 「TASK-017 남은 업무 모두 끝내자」에 따라 미구현된 고정 routed backend를 연결하고 이 Task의 기능 시험을 완료했다. [EV-035](../../05_verification/verification_evidence.md#ev-035)·[REV-033](../../05_verification/reviews/REV-033_TASK-017_ROUTED_HOST.md)·[API §15.7](../../03_design_artifacts/interface_contract.md#157-task-017-고정-routed-runcheck--2026-11-01). **Task 상태는 In Progress**다. 아래 별도 선행조건을 이번 기능 구현의 완료로 대체하지 않는다.

- [x] 동일 고정 슬롯의 File/Routed 등록·서로 다른 scope preview·단일 실행, 실제 target/요구 메모리/source provenance 검증. file 호출자가 native `HostCheckEvidence::File`를 소비하도록 회귀 확인.
- [x] `SnapshotCheck`를 client 생산 API로 제공: host-selected≤64KiB snapshot·1슬롯 큐·실제 raw/routing/원 receipt ACK·raw 회수·actual close/예약 반환. 원 source Report/profile와 check session/revision/monotonic clock 보존.
- [x] Read preview/Manage run·작은 응답 예산 pre-IO 거절·remote waiter timeout 후 실제 무수집/정리·native evidence·재실행 Gone를 기존 CLI/TCP 경로에서 검증.
- [x] 메모리 부족·foreign source/target/network/worker/boundary·수신자 없음·취소·호스트 시계 deadline 및 초기 OS 오류·미관측 구간을 검증. deadline 초과 수집/초기 원인 손실을 Red→Green으로 조치했다.
- [x] 일반 PC 실제 파일 IO/space probe·운영 파일 보존 회귀와 실제 dot-probe exporter→`dot_probe_check` host 실행을 검증했다. logger 예산16MiB+host 입력/파서/큐/runtime 보수 headroom=backend24MiB, 예제 허용32MiB. file TC-010의16MiB 프로필과 구분하고 RSS/실측 capacity로 승격하지 않는다.

| 남은 수락 조건 | 담당·연결 Task | 실행 조건/현재 상태 |
|---|---|---|
| 지속 in-process 수집·기존 화면·off/on·실제 첫 적용 | AI / TASK-020·026 | TASK-019/021 등 통합 선행 작업 후 실제 dot-probe adapter/화면 범위 확인 및 실행. 단일 export 전달은 첫 적용 완료가 아니며 T1 미도래 |
| PC/RT 비용·오차·LAN/RSS·UPS·장기 성능 | AI / TASK-021~024·TASK-022 승인 프로필 | 제품 통합 harness와 유효 프로필 필요. 현재 generic 기능 IO 시험만 Pass. RT 부팅/capability 미확인, 재부팅/부트 설정 변경은 기존 보류 유지 |
| native/다른 OS 수락 | AI / GAP-004·T3 | 저장소 공개 완료 후 source hash 고정 실행. 현재 트리거 미도래; 공개 전환/CI·계정 조회·차단 재시도하지 않음 |

TASK-017의 로컬 기능 구현 잔여는 처리했으나 위 통합/환경 수락은 면제되지 않는다. S3 전체·Task/MS·TC 전체와 Gate4/5를 Done/Pass로 올리지 않는다. 선택 추천 FR-015/TASK-018·무제한/임의 원격 실행·상시 서버는 추가하지 않았다. 별도 Task의 명시 DoD는 현황의 담당 경계를 설명한 것이며 완료 기준을 삭제하거나 작업 범위를 변경한 것이 아니다.

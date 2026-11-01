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
| 다음 액션 | S3 로컬 host/free probe·Report/profile·CLI 연결 수락(EV-033); 다음 생산 capture/routing·실제 수신자/실앱·실측/native |
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
- [ ] 생산 capture/routing/실제 수신자 ACK·dot-probe 실앱·PC/RT 비용/오차/성능·UPS/장기/native(T3)/다른 OS.

S3 전체 완료 체크는 위 후속 때문에 미완료로 유지한다. 기존 종료·수락 범위를 실제 장비 능력 검증으로 확대하지 않는다. 자동 worker/상시 서버·임의 remote 경로·무제한 실행·FR-015/TASK-018은 도입하지 않는다.

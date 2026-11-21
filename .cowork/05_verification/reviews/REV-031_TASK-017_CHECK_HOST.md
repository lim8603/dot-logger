# [REV-031] TASK-017 S3 로컬 운영 host·CLI 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #006 / AI 개발 자가 점검. 독립 리뷰가 아니다.
- 요청: Human 「S3 시작하자」. 기존 TASK-017 구현/검증·수락/분리 커밋·푸시 위임 적용.
- 소스: `f4ff74d5c3e2cb7c0140e6d64567691541437ae5`·source101개 commit blob/hash 일치.
- 대상: diagnostics host/run·remote Command/native RPC·client codec·CLI·Linux probe 의존성 및 통합 시험. 정확한 파일/hash·명령은 [EV-033](../verification_evidence.md#ev-033)·결과(비공개 원시 근거: `imported_context/task_017_s3_2026-11-01/results.json`).
- 기준: [code-review 스킬](../../../.agents/skills/code-review/SKILL.md)·API §15.3~15.5·TC-010·coding convention.
- 상태/판정: Closed / Pass — 아래 개발 중 발견3건 조치, 신규 미해결 P1/P2/P3 없음. S3 로컬 운영 연결만 기존 위임으로 수락한다.

## 요약표

| ID | 중요도 | 위치·계약 | 문제 | 처리·검증 | 상태 |
|---|---|---|---|---|---|
| RV-066 | P2 | diagnostics/run.rs 초기/record admission | 공간 probe 도중 time/cancel이 도래해도 이전 관측으로 새 부하를 허용할 수 있음 | probe 뒤 now/cancel 재검사, 초기/진행 중×기한/취소 반례 Red→Green | Closed |
| RV-067 | P2 | CLI commands/output | 새 check 결과 Inconclusive가 출력돼도 일반 proxy 성공 exit0으로 끝남 | Pass0/Fail4/Aborted·Inconclusive3, 실제 결과 실패와 completion_unconfirmed 구분, CLI assertion Red→Green | Closed |
| RV-068 | P2 | host source 등록 | 운영 Report session과 전용 합성 check session이 다르면 원래 출처 등록이 거절됨 | 양쪽 session/clock을 별도 보존하고 내부 Report 일관성/동일 재현 조건만 검사, 별도 session 실제 CLI·native Red→Green | Closed |

## [리뷰어] 검토

| 영역 | 판단·근거 |
|---|---|
| RT/동시성 | 새 코드는 명시 일반 host worker다. RT 수집/원자 연산/unsafe 변경 없음. 실제 Command/permit은 IO·정리 반환까지 유지하고 timeout은 신규 부하 취소로만 합성. pacing은 10ms 구간 관측, OS IO 강제 취소 주장 없음 |
| capability/권한 | 고정1..1024 슬롯, checked 단조 ID/slot 재사용 새 ID·run preflight 전 단일 소비. read PlanCheck와 Manage RunCheck 분리. 원격 경로·예산 입력 없음, 임의 fallback/retry 없음. 다른 backend 요청은 Unsupported |
| 응답/예산 | message1024B 여유+고정4096B schema 상한이 없으면 IO/ID 소비 전 Limit. 반환 Vec는 boxed-slice 변환으로 capacity=len. 최소5120B server와 만료 profile 포함 JSON의 실제 CLI 종단 확인 |
| OS 관측 | Linux statvfs의 안전한 rustix1.1.4 wrapper, 원 OS 오류·available≤total·fragment>0·checked multiplication. own unsafe/명령 실행/새 thread 없음. probe가 부하 측정 자체나 예약·quota·처리량 증명이 아님 |
| 출처 | 원 Report/window/conditions/profile로 재계산한 결과의 일관성과 계획 identity를 등록에서 확인. native 원문·원래 clock/epoch·profile 만료·조건/rate 유지. 합성 결과에서 capacity/새 Profile/운영 counter를 만들지 않음 |
| IO/namespace | S2 전용 inode/부모 독점 전제·create_new·prefix/footer/sync/실제 close/정리를 재사용. probe 뒤 time/cancel 재검사 추가. 기존/foreign 파일 보존·실제 실패·partial 결과를 CLI에서 확인 |
| 호환 | PlanCheck는 기존 protobuf 메시지를 사용하는 새 메서드. 구 server는 Unsupported, RunCheck fallback 없음. 기존 DTO/major1·권한/기한은 유지. 알 수 없는 report schema는 Corrupt/completion_unconfirmed이며 성공 추정 없음 |
| 테스트/소유권 | 실제 CLI/TCP/host/MCAP·관리 권한·취소·응답 부족·재실행·기존 파일 보존·Report/profile 원문·슬롯 재사용을 검증. source/CLI API compile Red 및 RV-066/067/068 assertion Red 근거 보존. 기존 workspace 종단도 회귀 |
| 의존성 | Linux target만 rustix1.1.4(std/fs)·linux-raw-sys0.12.1 신규, 기존 버전 교체 없음. MSRV1.85 전체·audit 확인. 운영 probe의 되돌릴 수 있는 내부 구현으로 기록, 핵심 stack/아키텍처·새 ADR 변경 아님 |

## [처리담당] 조치·근거

- RV-066 수용: `next_record`의 공간 조회 직후 `Shared::now` 및 cancellation을 재관측하고 초기 preflight elapsed도 실제 probe 이후 시각으로 계산한다. 이미 admitted IO/마감은 중단하지 않으며 time/cancel만으로 기존 prefix의 footer/sync/정리를 생략하지 않는다. Red(비공개 원시 근거: `imported_context/task_017_s3_2026-11-01/red-probe-deadline.log`)·S2 실제16개 Pass.
- RV-067 수용: host schema/outcome 해석 뒤 실제 결과를 먼저 출력하고 전용 Failure로 종료 코드를 분리했다. 실제 결과가 있는 실패에는 불필요한 completion_unconfirmed가 없으며 transport timeout에는 유지한다. Red(비공개 원시 근거: `imported_context/task_017_s3_2026-11-01/red-cli-outcome.log`)·CLI 실제5개 Pass.
- RV-068 수용: source 운영 session/clock을 전용 check session과 일치시키지 않고 양쪽 identity와 원래 Window/profile를 별도 보존한다. 측정 시각 간 차분/갱신은 하지 않고 Report 내부 및 재현 revision/environment/workload/target/boundary 일관성은 유지한다. Red(비공개 원시 근거: `imported_context/task_017_s3_2026-11-01/red-source-session.log`)·native/source4242와 check42의 실제 CLI 결과 보존 Pass.
- Rust1.96 workspace357/core288·MSRV1.85 workspace357, fmt/clippy/rustdoc -D warnings·protobuf/prost 비교/protoc·audit Pass. strict-provenance Miri 관리 수명/공간 산술2종×8seeds=16 Pass. 신규 기능 시험10개(host3+CLI5+산술1+느린 probe1). 실제 OS IO를 Miri로 대체하지 않는다.
- F-11 소비자: 기존 S1/S2·수집/라우팅/완료/저장·remote/Command timeout/권한·client/CLI·RT allocator를 workspace 및 core 회귀로 확인했다. 세션 로그에 조인 소비자와 미검증 범위를 기록했다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | diagnostics/host.rs·tc_010_check_host.rs | 단일 사용 host capability·source 보존·실제 Linux probe·관리/산술 시험 |
| 생성 | dot-logger-cli/tests/check_host.rs | 실제 CLI/TCP→host→파일 생명주기5개 |
| 수정 | diagnostics/mod.rs·run.rs·tc_010_file_check.rs | API 공개·느린 probe time/cancel 조치 |
| 수정 | remote/{hub,types,server,wire}.rs·client codec·proto | PlanCheck additive method·reply headroom |
| 수정 | CLI args/commands/output | plan/run 구조화 출력·결과별 exit |
| 수정 | Cargo.toml·core Cargo.toml·Cargo.lock | Linux 한정 안전한 statvfs 의존성 pin |
| 생성/수정 | 본 리뷰·EV/TC·API/UI/tech stack·TASK/MS·project_state/실행 근거 | 로컬 수락·검증 범위·후속 동기화 |

## 클래스(타입) 변경표

| 구분 | 소속 | 타입 | 역할 |
|---|---|---|---|
| 생성 | host.rs | CheckHost·CheckSource·CheckEvidence·HostError | 고정 등록/소비·원래 Report/profile·실제 결과·오류 |
| 생성 private | host.rs | Registered·CommandRuntime | 소유한 plan/writer·Command 취소를 runtime에 합성 |
| 수정 | remote | ManagementRequest·Command | PlanCheck·응답 capacity 조회 |
| 수정 private | CLI output.rs | Failure | 실제 점검 Fail/Aborted/Inconclusive 종료 분리 |
| 수정 메서드 | diagnostics/run.rs | CheckPlan·Shared 사용 경로 | probe 이후 시각/취소로 admission 결정 |
| 생성/수정 시험 | CLI/core tests | Host·Runtime | 실제 서버/파일 fixture·느린 probe 재현 |

## 미검증/다음 행동

생산 capture/routing/수신 ACK·dot-probe 실앱·PC/RT 성능/비용/오차·UPS/장기·native(T3)/다른 OS가 남아 S3 전체·Task/MS/TC·Gate4/5 미완료다. source/report 저장·worker 구동은 host 책임이며 demo fixture를 운영 daemon으로 승격하지 않는다. trusted namespace 외부 교체·panic/kill 후 자동 정리·재시작 exactly-once를 보장하지 않는다. FR-015/TASK-018 미착수 유지.

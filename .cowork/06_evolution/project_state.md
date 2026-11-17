# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 범용 로보틱스/RTOS Rust 로그 라이브러리(첫 사용처 dot-probe) |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build/Verify — MS-002~006 Done, MS-008 재개(TASK-034), MS-007·009 In Progress |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-008 In Progress(재개) — TASK-034 facade 노출; MS-009 In Progress — 표준 호환·원격 시각화·TASK-035; MS-007 In Progress — 일정 판단(TASK-033)·장기/native(TASK-024)·실사용처(TASK-026) — 024·026은 마지막·일정 판단 밖 |
| 활성 Task | TASK-034 In Progress(기존 결과 확인 대기, 다음 D runtime 통합); TASK-030 In Progress(S1·S2 A/B/C 로컬 Done, D/E 후속); TASK-035 In Progress(S1·S2 C 로컬 Done, E 최종 마감 대기); TASK-033 In Progress(034·035 뒤 재측정) |
| 미처리 리뷰 | 없음. [REV-058](../05_verification/reviews/REV-058_LOSS_CHECKPOINT_C.md) RV-146~148 수정·회귀·위임 수락으로 Closed(2026-11-17, EV-085). REV-055~057도 Closed |
| 상태 | 로컬 회귀 Pass — 1.96.0/1.85.0 각665 Pass·5 ignored, 각18 feature·Miri17+control-only1 재검증(EV-085), REV-058 Closed. 라인 커버리지93.56%(EV-080, TASK-034 전; 재측정 대기). Done25/35 Task. D runtime·E GUI/호출 비용/통합 판정 미완료, native·장기·실사용처 Not Run(TASK-024·026), Gate4/5 미완료 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-11-17 / TASK-035 S2 C 로컬 단위 완료·REV-058 Closed·다음 TASK-034 runtime(D) / AI |
| 세션 상태 | 진행(2026-11-17 Human 「다음 이어서 작업하자」로 C 수행). TASK-030 S2 B(EV-084)·TASK-035 S2 C 로컬 작업 단위 마감(EV-085). 다음 TASK-034 runtime WIP(D) → E 통합·TASK-033 재측정. 승인된 ADR-013 범위는 재승인 없이 진행 |
| 최근 세션 | 2026-11-17 #001 — B revision 저장(EV-084)·C loss checkpoint(EV-085) 로컬 단위 완료, 다음 D runtime WIP |

## 현재 결정과 제약

- **2026-11-17 TASK-035 S2 C 로컬 단위 완료:** [API §23.3](../03_design_artifacts/interface_contract.md#233-손실-누적-checkpoint-c-구현--2026-11-17)·[EV-085](../05_verification/verification_evidence.md#ev-085)·[REV-058](../05_verification/reviews/REV-058_LOSS_CHECKPOINT_C.md) Closed(RV-146~148). 제품 `0270f03`. telemetry 독립 counter·native loss checkpoint·bounded loss-only API/CLI·원 report 복구·target identity를 구현했다. retry 시도/skip/assignment/Unknown·미확인 꼬리·포화/누락 part를 분리한다. 다음 D TASK-034 WIP36c1724 통합, 그 뒤 E GUI/호출 비용·전체 통합/커버리지 판정. TASK-030·035 전체 In Progress를 유지하며 이전 A/B 절의 unavailable은 당시 구현 기록이다. GitHub PRIVATE로 T2/T3 미도래.

- **2026-11-17 TASK-030 S2 B 완료:** [API §23.2](../03_design_artifacts/interface_contract.md#232-revision-회전source별-조회복구-b-구현--2026-11-17)·[EV-084](../05_verification/verification_evidence.md#ev-084)·[REV-057](../05_verification/reviews/REV-057_REVISION_STORAGE_B.md) Closed. r1→r2→r1→r2 single writer 회전·source별 revision/sequence query/recovery·manifest v2, event/pin·발행 prefix·sync 범위 검증. loss는 unavailable, runtime WIP는 미통합이다. GitHub PRIVATE로 T2/T3 미도래. 이번 로컬 검증의 실제 커널은 `7.0.0-43-realtime`; 재부팅·커널 변경·정량 성능 측정은 수행하지 않았다.

- **2026-11-16 Human 「승인」 — ADR-013 Accepted:** D1 배포 전 native 단일 형식(legacy 쓰기/판독·선택 API·호환 wrapper 제외), D2 누적 손실 checkpoint(시도·assignment·skip·Unknown·미관측 분리), D3 단일 active writer의 교차 revision 회전·source별 조회/복구를 승인했다. [ADR-013](../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) 범위 안의 구현·검증은 재승인 없이 진행한다. 설계 승인을 제품 구현/시험 Pass로 대신하지 않는다.

- **2026-11-15 Human 「권고대로」 — TASK-034 ① 범위 조정(API §22.8):** core 저장이 파일당 단일 revision이라 facade 실행 중 설정 변경은 파일 회전 설계가 필요하다. 시작 시 level(`ChannelOptions::minimum`)만 TASK-034에서 구현하고, 실행 중 변경은 TASK-030 S2 저장 ADR에 회전·다중 revision 조회를 포함해 승인받은 뒤 WIP 브랜치 `wip/task-034-s2b-runtime-config`(`36c1724`, 로컬)로 끝낸다.
- **2026-11-15 Human 합의 — 일정 목표:** 다음 주(2026-11-16 주) 목표는 핵심 범위 완료와 MS-007 일정 판정이다. 대상은 TASK-034, TASK-030 S2 + TASK-035, TASK-033 최종 재측정, TASK-025다. viewer·대시보드(TASK-031·032)는 그다음 주로 둔다(Human 「뷰어는 그렇게 해도 돼」). 인일은 추정이며 납기 확정이 아니다. 병목은 승인 지점 세 곳이다(TASK-034 결과, TASK-030 S2 ADR, 판정).
- **2026-11-15 Human 「승인」 — TASK-034 facade 노출 계약(API §22):** D1 쉬는 writer는 facade ack(미반출·tracing) + 사용자 writer `Busy`/`Writer::refresh`, D2 이벤트 trigger는 일반 경로 `mark_event`만(RT trigger B1), D3 중요 예약 기본 0 + `PlanWarning`, D4 레시피 추가 없음.
- **2026-11-15 Human 「승인」 — 일정 판단 목표치:** 커버리지 하한 라인 92·함수 92·리전 89%(회귀 방지), 85% 미만 파일 실제 동작 공백 미처리 0, 범위 안 UC 구현 동작 시험 대응 100%, TASK-034·035 뒤 재측정으로 MS-007 판정. [quality_gate](../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정).
- **2026-11-15 Human 결정 — TASK-033 S3 기능 공백(「모두 권고대로 진행하자」):** facade 노출 ①실행 중 level·샘플링 ②이벤트 전후 보존 trigger ③단순 경로 중요도 → [TASK-034](../04_implementation/tasks/TASK-034.md)(MS-008 재개). 파일 안 손실 기록·채널별 손실 수 → [TASK-035](../04_implementation/tasks/TASK-035.md)(MS-009, TASK-030 S2 ADR과 함께). 나머지 기능 공백 → B1(범위 밖, 일정 판단 제외). client `diagnostics`(dot-probe 전용 `SnapshotCheck`+bridge) → `examples/support/dot_probe` 이동(`4bd6d04`).
- **2026-11-15 Human 결정 — native·24h 일정 판단 제외·TASK-030 S1 권고안:** 「native·24h 장기 검증을 일정 판단에서 빼자. 나머지는 권고대로」. TASK-024도 TASK-026처럼 마지막 단계·일정 판단 밖, 일정 판단은 TASK-033만. descriptor 없는 채널은 export에서 원문·encoding 유지+한계 기록([API §21.2](../03_design_artifacts/interface_contract.md#212-s1-구현-반영--2026-11-12)). CLI summary `records:0`은 결함 아님(출력한 `record` 줄 수) — CLI README에 의미 추가. [quality_gate](../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)·[TASK-030](../04_implementation/tasks/TASK-030.md#s1-리뷰-기록--2026-11-15).
- **2026-11-13 Human 결정 — 완료 기준 변경:** dot-logger는 범용 라이브러리이고 dot-probe는 첫 사용처다. 「실사용처 테스트는 마지막에 진행은 하겠지만, 일정 판단에 두지 말고, 일정 판단은 유닛테스트 케이스 커버리지, 실제 로깅 사용 사례 조사 등을 통해서 마무리」. 이에 따라 Task 22건 Done, native 잔여는 TASK-024, 실사용처는 TASK-026(MS-007, 마지막), 커버리지·사용 사례는 TASK-033. 커버리지 도구는 AI가 설치(Human 「필요한건 알아서 설치」). [quality_gate](../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)·[EV-078](../05_verification/verification_evidence.md#ev-078).
- **TASK-030 매핑:** 사용자 schema/encoding·real 시각·전체 receipt의 RecordMeta 보존은 native A에 반영(API §23.1). ROS 2 encoding은 T1 유지. [API §21](../03_design_artifacts/interface_contract.md#21-task-030-mcap-표준-호환--2026-11-12).
- **2026-11-12 Human 결정 — TASK-029 S4 범위:** `dot_probe_pipeline`(InfluxDB·설정 비교)은 facade로 옮기지 않고 고급 경로 예제로 유지, facade 전용 `dot_probe_facade` 예제 추가. DB·설정 변경의 facade 편입은 TASK-026에서 판단([EV-075](../05_verification/verification_evidence.md#ev-075)).
- **2026-11-12 Human 결정 — TASK-029 착수·L1:** NFR-008 「facade 공개 항목」은 이름 있는 항목(타입·trait·자유 함수, variant·메서드 제외)으로 세고, 변경 감지는 rustdoc JSON(nightly-2026-10-26) + 스크립트·snapshot. L1 개념 초과는 `Shutdown::is_success()` 추가로 해소. [API §20](../03_design_artifacts/interface_contract.md#20-task-029-레시피접점-회귀-가드--2026-11-12).
- **TASK-028 auto preset:** Done. JSON profile·측정/추정 구분과 사용자 값 우선 계약 유지. 측정 도중 취소는 미실행([API §19.8·19.9](../03_design_artifacts/interface_contract.md#198-task-028-s2-auto-preset-계약--2026-11-11), [EV-071](../05_verification/verification_evidence.md#ev-071)).
- **이전 완료 결정:** TASK-023·027·028 및 T5 상세는 API §18~19·EV-054~069를 따른다. T4는 이월, T6는 native A로 해소(EV-083).

- **2026-11-05 Human 결정 — TASK-027 S1 facade 계약:** ADR-012 Accepted(공유 `CaptureCore`·소유 `Writer`·빌림 `Producer<'_>`, Writer Drop은 반납함 이동만), 손실 값 필수·성능 값 기본값, `Logger::drop` 기한 종료, NFR-008 수치(L1 ≤20줄·≤6개념 등), `writer.try_log(channel, &msg)`. [API §18](../03_design_artifacts/interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05).

- **MS-008/009:** TASK-027~029 Done, TASK-030~032·035와 TASK-034 재개는 각 Task/현재 순서를 따른다. FR-015 질문 프리셋은 별도 승인 범위다([MS-008](../04_implementation/milestones/MS-008.md)·[MS-009](../04_implementation/milestones/MS-009.md)).

- **probe 연동 순서:** dot-logger 자체 작업·TASK-025 인계 뒤 별도 dot-probe 세션에서 TASK-026을 진행한다. 원격 성능 잔여는 T7, native/장기는 TASK-024다. RT 재부팅은 ADR-011의 사전 승인·once/back 절차를 유지하며 이 세션에서 dot-probe 저장소를 수정하지 않는다.

- **2026-10-31 Human 승인·권한 위임:** TASK-012 기존 로컬 구현 수락 및 REV-019 개선·검증 후 마감 → TASK-013 착수 승인. 앞으로 승인된 목표의 구현·검증·리뷰 마감/작업 전환 판단과 작업 단위별 커밋·푸시는 AI가 수행·보고한다. 같은 범위 재승인은 받지 않는다. [위임 기준](../01_cowork_protocol/decision_authority_matrix.md#프로젝트-오너의-권한-위임--2026-10-31). 미실행 검증·native 공개 후 조건은 유지한다.

- **TASK-009~022 현재 계약:** 각 Task 문서와 [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안)을 따른다. 미수락 잔여(native T3·실앱·PC/RT 성능·장기)는 각 Task와 GAP-002/004에 유지된다. 이 문서에 있던 요약 원문은 [상태 아카이브](state_archive.md) #023으로 옮겼다.

- **2026-10-26 Human 승인:** 「권고 기준선 채택 — 장비·호환 검증은 계획대로 진행」. 상세설계·ADR-008/009·추가 시험 목표·MS 계획을 채택했다. 검증 이월은 면제가 아니다.

- **ADR-010 Accepted / 2026-10-26 Human 승인:** MCAP 0.26.0 채택. 0.25 선택 zstd 경로의 보안 공지를 해소했다. Tokio는 1.51.6 patch. 최종 audit 취약점/unsound 0, `paste` 유지보수 경고 1(GAP-005).

- Gate 1·2 Pass, ADR-003~006 기준선 유지. 1 GB/s 원본+10만 소형/s·32생산자/16파일/128채널·R1 2 GiB 및 512 MiB/4 GiB 비교는 조건부 목표이며 제품 기본값·달성 결과가 아니다.
- **ADR-007 Accepted:** dot-probe 내장형 Ports and Adapters + 제한된 생산자/소비자 파이프라인. 별도 데몬은 첫 범위에 넣지 않는다. 프로세스 장애까지 격리하지 않음.
- **클라이언트 형태 확정:** Rust API + CLI, dot-probe 기존 화면 연동. 명령/통신 상세도 ADR-009 기준선 채택 완료.
- **실제 장치 대체 환경 확정 — ADR-011 Accepted / 2026-10-31:** Human이 현재 Ubuntu PC 실측을 실제 장치 시험으로 대체 수락하고 실물 UPS 없이 시뮬레이션 결과로 판정하도록 승인했다. native는 GitHub 공개 완료 후 진행한다. 기본 부팅은 generic(7.0.0-43). RT 행이 필요하면 AI가 요청서를 내고 Human 승인 뒤 `scripts/rt-boot.sh once`(1회 RT 부팅)·확인·`back` 절차로 진행한다(2026-11-11 실행, EV-070). 상세 [TASK-022](../04_implementation/tasks/TASK-022.md)·[ADR-011](../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md).
- RT 무대기·별도 중요도·중요 우선 기본값, 보존 모드 옵션과 처리/저장 예산 분리 유지. FR-018은 선택 옵션·기본 off, 전체 약3초는 신호 발생→차단 예산.
- 플랫폼 Tier 1/2 유지. affinity는 간섭 감소이며 완전 격리/보편적 성능 보장 아님. Tier 2 고급 기능 미지원은 capability로 표시한다.
- FR-015는 장비 측정 auto preset만 2026-11-04 TASK-028 S2로 부분 채택(선택·기본 off). 질문 프리셋 TASK-018은 별도 범위 승인 전 착수하지 않고 Must 완료에 몰래 넣지 않는다.

## 활성 Task 요약

Done Task는 [Task 목록](../04_implementation/task_registry.md)을 따른다(25건: TASK-001~017·019~023·027~029).

| Task | 제목 | 담당 | 상태 | 갱신 | 다음 액션 |
|---|---|---|---|---|---|
| TASK-030 | MCAP 표준 호환(표준 도구 판독) | AI | In Progress | 2026-11-17 | S1 Done(EV-077·082), S2 A/B/C 로컬 Done(EV-083~085). 다음 D TASK-034 runtime → E GUI/비용·통합 판정 |
| TASK-033 | 로깅 사용 사례 조사와 커버리지 기반 완료 판정 | AI | In Progress | 2026-11-15 | S2 [UC 31개](imported_context/research_2026-11-15_logging_use_cases.md)·S3 대응표·공백 시험 16+1·작은 ring 결함 수정([EV-079](../05_verification/verification_evidence.md#ev-079)), 결정 4건 반영, S3 Done(라인 92.40%). S4 목표치 승인·기준 1~3 충족, TASK-034·035 뒤 최종 재측정 대기 |
| TASK-034 | facade에 core 기능 노출 | AI | In Progress | 2026-11-16 | S1 승인·S2/S3 Done(runtime 제외, EV-081·082). 기존 결과 Human 확인 대기 유지. A/B/C 선행 완료, 다음 WIP36c1724 runtime 통합(D) |
| TASK-035 | 손실 기록의 파일 내 표시 | AI | In Progress | 2026-11-17 | S1 Done, S2 C 로컬 Done(EV-085·REV-058). D 통합 뒤 E GUI/호출 비용·최종 마감 대기 |
| TASK-024 | 24시간·전원 차단·OS 교환 검증 | AI | Planned | 2026-11-15 | 마지막 단계(일정 판단 밖, 2026-11-15). P6/24h·UPS 모의 준비. 기능 Task의 native·다른 OS 잔여 통합(T3) |
| TASK-025 | 사용 문서·MS 추적표·인수 정리 | AI | Planned | 2026-11-15 | TASK-033·MS-009 뒤, TASK-024·026 앞(TASK-024 의존 해제) |
| TASK-026 | dot-probe 실제 첫 적용과 화면 연동 | AI | Planned | 2026-11-13 | 프로젝트 마지막 단계(일정 판단 밖) |
| TASK-031·032 | viewer·대시보드 | AI | Planned | 2026-11-04 | TASK-030 뒤 S1 조사·결정 |
| TASK-018 | 질문 프리셋(선택) | AI | Planned | 2026-11-04 | FR-015 범위 승인 대기, Must 아님 |

## 코드리뷰 현황 (Reviews)

| ID | 범위 | 상태 | 현재 처리/후속 |
|---|---|---|---|
| REV-001~054 | 완료된 core·파일·조회·운영·facade·preset·통합 가드 리뷰 | Closed | 회차별 근거는 [리뷰 문서](../05_verification/reviews/). native·실앱·성능 이월은 각 Task에 유지 |
| [REV-055](../05_verification/reviews/REV-055_FACADE_STANDARD_EXPORT_REVIEW.md) | TASK-030 S1 export·TASK-033 S3/S4 시험·ring 결함 수정·client diagnostics 이동·TASK-034 facade 중요도/최소 level/이벤트 보존 누적 리뷰(`64e9d69..949a74e`) | Closed | RV-136~140 전부 수용·조치, [EV-082](../05_verification/verification_evidence.md#ev-082) `5d7fe6a`. 2026-11-16 위임 수락 |
| [REV-056](../05_verification/reviews/REV-056_NATIVE_STORAGE_A.md) | TASK-030 S2 A native 단일 형식 개발·자가검토 | Closed | RV-141~144 전부 수용·수정, [EV-083](../05_verification/verification_evidence.md#ev-083) `048cdf3`. 두 Rust 각630 Pass·각18 feature, 2026-11-16 위임 수락 |
| [REV-057](../05_verification/reviews/REV-057_REVISION_STORAGE_B.md) | TASK-030 S2 B revision 회전·source별 조회/복구·manifest v2 개발 자가검토 | Closed | RV-145 Header-only identity 공백 수정·EV-084, 두 Rust 각642 Pass·각18 feature·Miri20, 2026-11-17 위임 수락 |
| [REV-058](../05_verification/reviews/REV-058_LOSS_CHECKPOINT_C.md) | TASK-035 S2 C counter·native checkpoint·별도 조회/복구/CLI 자가검토 | Closed | RV-146~148 수정·EV-085 각665/각18 feature·Miri17+1·RT 가드, 2026-11-17 위임 수락 |

이전 회차별 대상·판정·처리 이력은 [리뷰 문서](../05_verification/reviews/)와 [상태 아카이브](state_archive.md)의 #014·#024 이관을 따른다.

## 다음 시작점

REV-055 조치는 완료했다([EV-082](../05_verification/verification_evidence.md#ev-082)). 별도 Human 확인 대기는 TASK-034 기존 구현 결과(EV-081·082)다.

1. **TASK-034 runtime 통합(D):** WIP `wip/task-034-s2b-runtime-config`(`36c1724`)를 승인된 [ADR-013](../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md)·API §22·23.1~23.3의 현재 native 저장/loss 계약에 맞춰 적용한다. A/B/C 선행은 EV-083~085로 로컬 완료했다. `.set_channel`·Writer refresh·쉬는 writer ack·원격 ApplyConfig·update 전후 실제 파일 revision 회전과 loss session 누적을 시험한다. 단순 cherry-pick 성공으로 수락하지 않는다. TASK-034 기존 결과 Human 확인 대기는 유지하되 이미 승인된 runtime 구현을 재승인받지 않는다. 그 뒤 E native GUI user/meta/loss·counter 전후 호출 비용·전체 회귀와 TASK-033 커버리지/UC 판정으로 TASK-030·034·035를 마감한다.
2. **TASK-033 S4:** 승인 목표치 기준 1~3 충족(EV-080). TASK-034·035 뒤 최종 재측정(기준 4)으로 MS-007 일정 판단을 마무리한다.
3. **2026-11-16 주 목표:** TASK-034·TASK-030 S2·TASK-035 → TASK-033 판정 → TASK-025 문서·인수 정리. 납기 확정이 아닌 합의 목표다.
4. **2026-11-23 주 계획:** TASK-031 viewer → TASK-032 대시보드. viewer 실시간 경로 설계 시 T7 트리거를 확인한다.
5. **마지막 단계·일정 판단 밖:** TASK-024 장기/native T3·TASK-026 실사용처 적용. TASK-018은 별도 범위 승인 대기의 선택 기능이다.

## 승인·재개 제약

기존 구현·검증·수락·작업 전환·작업 단위별 커밋/푸시 위임은 유지한다. 새 범위·공개 전환·선택 기능 자동 착수로 확대하지 않는다. native 공개 후(T3)·PC/RT/장기/실앱·설정/비교·지속 실앱 통합·Gate4/5 미완료 조건은 유지한다. 상세 승인/이전 에디터 진단 정정·과거 재개 메모는 [상태 아카이브](state_archive.md)의 #014 이관과 관련 Task/API를 따른다. 개발1.96.0·MSRV1.85·Miri nightly-2026-10-26 고정, 전역 rustup default/사용자 에디터 설정은 변경하지 않는다.

## 선택 로딩

- [도메인](../03_design_artifacts/domain_model.md), [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안), [데이터](../03_design_artifacts/data_model.md), [CLI](../03_design_artifacts/ui_spec.md).
- [MS 목록](../04_implementation/milestone_registry.md): 9개 MS, [Task 목록](../04_implementation/task_registry.md): 35개 Task(1개 선택). 선행조건·변경 대상·첫 실패/DoD·상대 인일 추정(계획 승인, 납기 확정 아님).
- [요구 추적표](../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트): INT 1 + FR 18 + NFR 8 + CON 7 = 34행. MS별 구현/테스트 체크·코드/EV 연결. TC-001의 TASK-002 하위 EV-001 로컬/native, TC-002 소형 EV-002 로컬/native·원본 EV-003 로컬, TC-003 분배 EV-004·완료/제어 EV-005 로컬 Pass; 후속/native/장비 시험 잔여.
- [2026-10-26 조사](imported_context/research_2026-10-26_detailed_design.md): 공식 자료와 dot-probe 코드 관측, 기술 후보/사실/미검증 분리.
- 이전 결정 근거가 필요할 때만 ADR-003~006·요구 §12~17·Story 상세·개인 로그를 추가로 읽는다.

## 이월 백로그

다음 T = T8 / 다음 B = B2. 트리거 감시는 AI 책임이다.

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| T1 | ROS 2 파일 호환·토픽 연동 필요/범위 검토 | dot-probe 첫 적용 완료가 확인된 때 | INT-001 / OQ-008 / TASK-026 | 2026-10-25 Human Q5 |
| T2 | 공개 전환 전 라이선스·GitHub 운영정책·공개할 이력/로그 검토(vendor `mcap` MIT 전문 고지 보완 포함 — ADR-010 개정 1) | 공개 전환 또는 첫 외부 배포를 요청할 때 | INT-001 / 공개 준비 | 2026-10-28 Human 논의 |
| T3 | 이월 native 검증 실행·결과 반영 | GitHub 저장소 공개 완료가 확인된 때 | GAP-004 / ADR-011 / TASK-004~017·019~021 | 2026-10-31 Human 검증 시점 결정 |
| T4 | facade 소형 기록 slot 직접 encode 별도 설계(scratch encode + byte ring 복사 대체) | TASK-023 S2 P1(256B) 측정에서 encode·복사 비용이 의미 있게 나온 때 | NFR-001 / API §18.3 / TASK-027 S3d / TASK-023 S2 | 2026-11-07 Human 「승인」·백로그 기록 요청 |
| T7 | TASK-023 이월 성능 행: P4(TCP receiver ACK 대비)·P5, 원격 포화 회복 실측(REV-022 RV-049) | 원격 경로 성능 요구가 생길 때(TASK-031 viewer 실시간 경로 설계 포함), 또는 Human 요청 시 | NFR-001·002 / TASK-023·012 / EV-059 | 2026-11-08 Human P4·P5 이월, 2026-11-13 TASK-023 Done 처리 때 등록 |

범위 밖 백로그 B:

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| B1 | TASK-033 S3 기능 공백(범위 밖, 일정 판단 제외): UC-02 오래된 기록 덮어쓰기·정책 선택, UC-05 high-water 지표, UC-07 시간 기준 rate·출력/상태별 rate, UC-10 정규식·프로필 채널 선택, UC-11 시계 객체 주입·epoch 이전 표시, UC-12 사용자 파일 metadata·첨부, UC-14 `log` kv·span, UC-16/19 요청 시 분할·sync(facade ④), UC-18 backtrace, UC-21 panic hook·signal, UC-22 쓰기 오류 재열기, UC-23 압축, UC-24 반복·step·remap·real 순서, UC-25 병합·필터 export, UC-28 원격 일시정지·분할·snapshot, UC-29 상태 기반 기록, UC-31 텍스트 검색 | 사용처 요구 또는 Human 요청 시 | [TASK-033](../04_implementation/tasks/TASK-033.md#uctc-대응--s3-2026-11-15) | 2026-11-15 Human 「모두 권고대로」 |
**T5 해소**(EV-066~068), **T6 해소**(EV-083: Envelope scratch 복사 제거, 정량 효과 미측정). T1~T4는 미도래이며 T4의 복사 p50 21ns는 시계 오버헤드와 같은 수준이었다. T7은 viewer 실시간 설계 시 확인한다. native 실행을 공개 후로 정한 것은 공개 전환 요청이 아니므로 T2를 발동하지 않는다. OQ는 T/B에 중복 등록하지 않는다.

## 열린 결정과 검증 공백

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| OQ-002 | ADR-011로 현재 PC 실측·UPS 시뮬레이션 수락 확정. local-pc-v1 수치 Approved(EV-021), UPS 하위 EV-043 수락·성능/RT 실측 잔여 | 실앱/성능 harness·RT 부팅 확인 후 현재 PC 성능/TC-012·장기 통합 실행 |
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·실제 앱/장비 수명 수락은 미검증 | TASK-004~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬/native CI 확인(EV-001), 실제 DB·장비 호환 잔여 | TASK-019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증, TASK-018은 별도 범위 승인 후 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·004·005 | PC 환경 확정·프로필/후속 TC 실행 잔여, TASK-004~017·019~021 native 공개 후(T3) 이월·paste 경고. GAP-003 Closed | [검증 근거](../05_verification/verification_evidence.md) 참조 |
| Closed GAP·RV | GAP-006~010·RV-006~035 로컬 조치/수락 완료(native는 GAP-004 이월) | 원문 [상태 아카이브](state_archive.md) #023 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-11-17 #001 C: Human 「다음 이어서 작업하자」로 TASK-035 S2 C 로컬 단위 완료. 고정 counter·native loss·별도 조회/복구·CLI, RV-146~148 Red→Green·REV-058 Closed([EV-085](../05_verification/verification_evidence.md#ev-085), 두 Rust 각665·각18 feature·Miri17+1·RT 가드). 제품0270f03·source156 일치. 다음 TASK-034 runtime(D), E 전체 마감은 후속.

- 2026-11-17 #001: Human 「권고대로 시작하자」로 TASK-030 S2 B 재개·완료. 실제 교차 delivery·보호/pin·fault prefix·sync·v2 query/replay/blob/recovery 검증, RV-145 수정·REV-057 Closed([EV-084](../05_verification/verification_evidence.md#ev-084), 두 Rust 각642 Pass·각18 feature·Miri20). 다음 TASK-035 S2([로그](../members/lim8603/workspace/session_logs/session_2026-11-17_001.md)).

- 2026-11-16 #001: REV-055 조치·목록 정렬·ADR-013 승인·TASK-030 S2 A 완료·REV-056 Closed([EV-082](../05_verification/verification_evidence.md#ev-082)·[EV-083](../05_verification/verification_evidence.md#ev-083), 두 Rust 각630 Pass), 다음 B로 세션 종료([로그](../members/lim8603/workspace/session_logs/session_2026-11-16_001.md)).
- 2026-11-15 #001: TASK-030 S1 Done·사용 사례31개·목표치 승인·TASK-034 중요도/시작 level/이벤트 보존 구현, runtime은 저장 ADR 뒤([로그](../members/lim8603/workspace/session_logs/session_2026-11-15_001.md)).
- 2026-11-13 #001: 범용 라이브러리 완료 기준 변경·Task22건 Done·TASK-033 등록·커버리지 기준선 준비([EV-078](../05_verification/verification_evidence.md#ev-078), [로그](../members/lim8603/workspace/session_logs/session_2026-11-13_001.md)).

이전 이력과 상세 원문은 [상태 아카이브](state_archive.md)의 #028·#027·#026·#025·#023·#022, R1/R2 이관 원문은 #020/021을 따른다.

# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 범용 로보틱스/RTOS Rust 로그 라이브러리(첫 사용처 dot-probe) |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build/Verify — MS-001~006·008 Done, MS-007·009 In Progress |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-009 In Progress — 표준 호환·파일 손실·원격 viewer 완료, 운영 대시보드(TASK-032) 진행 중. MS-007 In Progress — 일정 판정 완료, 인수·장기/native·실사용처 잔여 |
| 활성 Task | [TASK-032](../04_implementation/tasks/TASK-032.md) In Progress — S1 Done([ADR-016](../03_design_artifacts/adrs/ADR-016_viewer_embedded_web_dashboard.md) 내장 웹·읽기 전용). 다음 S2 |
| 미처리 리뷰 | 없음 — REV-001~069 Closed. 최근 [REV-069](../05_verification/reviews/REV-069_T7_STORAGE_REMOTE_ACK_CRC_REVIEW.md) RV-163~165 조치·수락([EV-106](../05_verification/verification_evidence.md#ev-106)) |
| 상태 | 30/35 Task Done. Foxglove·T7 [EV-096~105](../05_verification/verification_evidence.md#ev-096), REV-069 조치 [EV-106](../05_verification/verification_evidence.md#ev-106) 두 Rust 각 711 Pass·5 ignored. [EV-089](../05_verification/verification_evidence.md#ev-089) 일정 판정 라인 93.69%·함수 92.67%·리전 91.08%, UC 28/28. Gate 4·5 Not Ready |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-11-21 / TASK-032 S1 결정(ADR-016) / AI |
| 세션 상태 | **진행 — 2026-11-21 #003.** REV-069 조치 마감(`5daf22b`), TASK-032 S1 Done(ADR-016). 다음 S2 |
| 최근 완료 세션 | 2026-11-21 #002 — bd02d49..078ca6c 독립 코드리뷰([REV-069](../05_verification/reviews/REV-069_T7_STORAGE_REMOTE_ACK_CRC_REVIEW.md), RV-163~165) |

## 현재 결정과 제약

- **2026-11-21 Human 「A. viewer 내장 웹」·「읽기 전용」 — [ADR-016](../03_design_artifacts/adrs/ADR-016_viewer_embedded_web_dashboard.md):** TASK-032 대시보드는 viewer가 loopback HTTP로 내장 정적 페이지와 JSON 경로를 제공한다. 새 crate·외부 CDN·빌드 도구는 쓰지 않는다. GetStatus(1초, 한 번에 하나)·Compare·PlanCheck만 조회하고 ApplyConfig·RunCheck는 CLI로 둔다. 값 상태를 보존하고 실패 시 마지막 값·시각과 사유를 구분한다. [화면 §7](../03_design_artifacts/ui_spec.md#7-task-032-운영-대시보드--2026-11-21)

- **2026-11-21 Human 「묶음·파이프라인 ACK (권장)」 — [ADR-015](../03_design_artifacts/adrs/ADR-015_batched_pipelined_remote_ack.md):** `handoff`는 ACK를 대기열에 넣고 바로 반환한다. 묶음 Ack RPC와 유한 동시 전송을 쓰고, 실패는 이후 호출에서 `AcknowledgementUnknown`으로 보고하며, `flush`로 확인한다. 구현은 `4ac9bf1`, LAN P4 소형 Pass는 [EV-104](../05_verification/verification_evidence.md#ev-104)다.

- **2026-11-21 Human 「0 행 생략 (권장)」 — ADR-013 D2 저장 표현 개정:** 손실 checkpoint는 `version=2` part에 0이 아니거나 overflow인 행만 등록 행 구간과 함께 싣는다. 예약은 dense 최악값을 유지하고 v1 판독도 유지한다. 구현·P3/P5 Pass는 [EV-103](../05_verification/verification_evidence.md#ev-103), 근거는 [EV-101](../05_verification/verification_evidence.md#ev-101)이다. [ADR-013 개정](../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md#손실-checkpoint-0-행-생략-개정--2026-11-21)

- **2026-11-18 Human 결정 — [ADR-014](../03_design_artifacts/adrs/ADR-014_viewer_live_websocket.md):** 운영 PC의 WebSocket v1과 로컬 스키마 목록을 승인했다. 「제안대로 진행」으로 Query/ReadBlob과 viewer 시간 이동·배속을 채택했다. S2/S3 로컬 근거 EV-092~094.

- **.logger 예약 토픽·호스트 예제 개정:** 호스트가 지정하는 예제 경로는 `.logger/`, CLI 예제 manifest는 `.logger/session.json`이다. native `.logger/meta`·`.logger/loss`는 `dot-logger/standard/2` 및 Metadata 형식 버전 2로 식별한다. 이전 개발 파일은 당시 검증 증거로 보존한다. [ADR-013](../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md#예약-토픽과-파일-프로필-개정--2026-11-18) · [EV-090](../05_verification/verification_evidence.md#ev-090)

- **ADR-013 A~E 완료:** 제품 `874c27c`의 native 표준 저장·revision 회전·손실·runtime 변경을 최종 수락했다. Foxglove 실제 값 판독·호출 비용·회귀는 [EV-088](../05_verification/verification_evidence.md#ev-088), tracing 경합/패닉 복구 회귀 `be20250`은 [REV-061](../05_verification/reviews/REV-061_ADR_013_E.md)에 있다. TASK-030·034·035와 MS-008은 Done. [EV-089](../05_verification/verification_evidence.md#ev-089)에서 TASK-033 S4와 MS-007 일정 판정도 마쳤다. GitHub PRIVATE 및 T2/T3 미도래를 유지한다.

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

Done Task는 [Task 목록](../04_implementation/task_registry.md)을 따른다(30건: TASK-001~017·019~023·027~031·033~035).

| Task | 제목 | 담당 | 상태 | 갱신 | 다음 액션 |
|---|---|---|---|---|---|
| TASK-024 | 24시간·전원 차단·OS 교환 검증 | AI | Planned | 2026-11-15 | 마지막 단계(일정 판단 밖, 2026-11-15). P6/24h·UPS 모의 준비. 기능 Task의 native·다른 OS 잔여 통합(T3) |
| TASK-025 | 사용 문서·MS 추적표·인수 정리 | AI | Planned | 2026-11-15 | TASK-033·MS-009 뒤, TASK-024·026 앞(TASK-024 의존 해제) |
| TASK-026 | dot-probe 실제 첫 적용과 화면 연동 | AI | Planned | 2026-11-13 | 프로젝트 마지막 단계(일정 판단 밖) |
| TASK-032 | viewer 대시보드 | AI | In Progress | 2026-11-21 | S1 Done(ADR-016). S2 상태 JSON 계약 실패 시험 → 내장 HTTP·상태 화면 → S3 비교·PlanCheck |
| TASK-018 | 질문 프리셋(선택) | AI | Planned | 2026-11-04 | FR-015 범위 승인 대기, Must 아님 |

## 코드리뷰 현황 (Reviews)

| ID | 범위 | 상태 | 현재 처리/후속 |
|---|---|---|---|
| REV-001~058 | 완료된 core·파일·조회·운영·facade·preset·통합 가드 및 ADR-013 A/B/C 개발 자가검토 | Closed | 회차별 근거는 [리뷰 문서](../05_verification/reviews/)(EV-082~085). native·실앱·성능 이월은 각 Task에 유지 |
| [REV-059](../05_verification/reviews/REV-059_NATIVE_REVISION_LOSS_REVIEW.md) | `949a74e..0270f03` 통합 코드리뷰·후속 `8b59aa6` 조치 | **Closed** | RV-149~152 전부 수용·수정·자가검토·위임 수락(EV-086). 복구 완결·손실 관측·종료 경계·등록 검증, 두 Rust 각672 Pass·5 ignored |
| [REV-060](../05_verification/reviews/REV-060_RUNTIME_CONFIG_D.md) | WIP36c1724 기반 runtime D 개발 자가검토·제품874c27c | **Closed** | RV-153 pending 보존/기한·RV-154 무대기 ack 조치·위임 수락(EV-087), 두 Rust 각684·18 feature·Miri13·표면35/60 Pass |
| [REV-061](../05_verification/reviews/REV-061_ADR_013_E.md) | ADR-013 E 최종 통합·tracing 경합/패닉 복구 회귀 | **Closed** | EV-088, RV-155 두 변이 Red·현행 Green, 기존 위임 수락 |
| [REV-062](../05_verification/reviews/REV-062_DOT_LOGGER_NAMESPACE.md) | .logger 예약 토픽·호스트 예제·standard/2 | **Closed** | EV-090, 두 Rust 각685 Pass·CLI/Foxglove 확인 |
| [REV-063](../05_verification/reviews/REV-063_TASK_031_LIVE_VIEWER.md) | TASK-031 S2 WS bridge·카탈로그·ACK/수명 | **Closed** | EV-092·위임 로컬 수락. Foxglove 실시간·T7 정량 잔여 |
| [REV-064](../05_verification/reviews/REV-064_TASK_031_MCAP_COMPATIBILITY.md) | TASK-031 S3 MCAP 등록 배치 | **Closed** | EV-093·위임 로컬 수락 |
| [REV-065](../05_verification/reviews/REV-065_TASK_031_QUERY_REPLAY.md) | TASK-031 S3 원격 기록 재생 | **Closed** | EV-094·위임 로컬 수락 |
| [REV-066](../05_verification/reviews/REV-066_RUNTIME_NAMESPACE_VIEWER_REVIEW.md) | Viewer 구독·재생 수명, MCAP 등록 검증, IO 작업자 실패 처리 | **Closed** | RV-156~159 수정·자가검토·위임 수락([EV-095](../05_verification/verification_evidence.md#ev-095)) |
| [REV-067](../05_verification/reviews/REV-067_TASK_031_T7_STORAGE.md) | T7 저장 시작·동시 부하 | **Closed** | RV-160·161 Closed. 손실 checkpoint 0 행 생략 뒤 8 GiB P3·P5 Pass(EV-101·103) |
| [REV-068](../05_verification/reviews/REV-068_TASK_031_T7_ACK_CRC.md) | T7 마감 변경(0 행 생략·묶음 ACK·CRC) | **Closed** | RV-162 ACK 기한 starvation 조치·위임 수락 |
| [REV-069](../05_verification/reviews/REV-069_T7_STORAGE_REMOTE_ACK_CRC_REVIEW.md) | `bd02d49..078ca6c` T7 저장·0 행 생략·묶음/파이프라인 ACK·CRC-32 독립 리뷰 | **Closed** | RV-163~165 수용(RV-164 일부 조정)·`5daf22b` 수정·위임 수락([EV-106](../05_verification/verification_evidence.md#ev-106)) |

이전 회차별 대상·판정·처리 이력은 [리뷰 문서](../05_verification/reviews/)와 [상태 아카이브](state_archive.md)의 #014·#024·#029 이관을 따른다.

## 다음 시작점

1. **TASK-032 S2.** S1은 2026-11-21 [ADR-016](../03_design_artifacts/adrs/ADR-016_viewer_embedded_web_dashboard.md)으로 끝났다(내장 웹·읽기 전용). 먼저 상태 JSON 계약의 실패 시험을 쓴다: Unavailable/Stale를 0·정상으로 바꾸지 않고, 끊김·권한 없음에서 마지막 값·시각과 사유를 구분한다. 그 뒤 viewer 내장 HTTP와 상태 화면, S3 비교·PlanCheck 순서로 진행한다. [TASK-032](../04_implementation/tasks/TASK-032.md) · [화면 §7](../03_design_artifacts/ui_spec.md#7-task-032-운영-대시보드--2026-11-21)
2. **TASK-025 인수 정리:** 상세 작업의 TASK-032 선행 조건에 따라 MS-009 뒤에 진행한다. 11월 16일 주 목표에도 포함됐던 일정 차이는 기존 선행 조건과 계획 날짜를 보존해 추적한다.
3. **마지막 단계:** TASK-024 장기/native T3·TASK-026 실사용처 적용은 일정 판단 밖이다. TASK-018은 별도 범위 승인 대기의 선택 기능이다.

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

범위 밖 백로그 B:

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| B1 | TASK-033 S3 기능 공백(범위 밖, 일정 판단 제외): UC-02 오래된 기록 덮어쓰기·정책 선택, UC-05 high-water 지표, UC-07 시간 기준 rate·출력/상태별 rate, UC-10 정규식·프로필 채널 선택, UC-11 시계 객체 주입·epoch 이전 표시, UC-12 사용자 파일 metadata·첨부, UC-14 `log` kv·span, UC-16/19 요청 시 분할·sync(facade ④), UC-18 backtrace, UC-21 panic hook·signal, UC-22 쓰기 오류 재열기, UC-23 압축, UC-24 반복·step·remap·real 순서, UC-25 병합·필터 export, UC-28 원격 일시정지·분할·snapshot, UC-29 상태 기반 기록, UC-31 텍스트 검색 | 사용처 요구 또는 Human 요청 시 | [TASK-033](../04_implementation/tasks/TASK-033.md#uctc-대응--s3-2026-11-15) | 2026-11-15 Human 「모두 권고대로」 |
**T5 해소**(EV-066~068), **T6 해소**(EV-083: Envelope scratch 복사 제거, 정량 효과 미측정). T1~T4는 미도래이며 EV-088 재측정에서도 256B 인코드·복사 p50은 시계 대조와 같은 21ns였다. **T7 해소**(포화 회복 EV-097, P5 EV-103, LAN P4 EV-104, 유선 P4-bulk EV-105). native 실행을 공개 후로 정한 것은 공개 전환 요청이 아니므로 T2를 발동하지 않는다. OQ는 T/B에 중복 등록하지 않는다.

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

- 2026-11-21 #003 — REV-069 반입·RV-163~165 조치(`5daf22b`)·위임 마감, TASK-032 S1 ADR-016 결정([EV-106](../05_verification/verification_evidence.md#ev-106), [로그](../members/lim8603/workspace/session_logs/session_2026-11-21_003.md)).
- 2026-11-21 #002 — `bd02d49..078ca6c` T7 저장·원격 변경 독립 코드리뷰 [REV-069](../05_verification/reviews/REV-069_T7_STORAGE_REMOTE_ACK_CRC_REVIEW.md) Open(`RV-163~165`)([로그](../members/lim8603/workspace/session_logs/session_2026-11-21_002.md)).
- 2026-11-21 #001 — RV-161 P0 대조·0 행 생략 개정·P3/P5 Pass, ADR-015 묶음·파이프라인 ACK로 P4 소형 Pass, 하드웨어 CRC로 유선 P4-bulk Pass, T7 해소 및 TASK-031 Done([EV-101~105](../05_verification/verification_evidence.md#ev-101), [로그](../members/lim8603/workspace/session_logs/session_2026-11-21_001.md)).
- 2026-11-19 #001(11-20 마감) — RV-161 저장 조치·P3/P5 재측정, Windows 직접 연결과 P4 루프백 대조를 기록했다([EV-098~100](../05_verification/verification_evidence.md#ev-098), [로그](../members/lim8603/workspace/session_logs/session_2026-11-19_001.md)).

완료 상세 원문과 이전 이력은 [상태 아카이브](state_archive.md) #030~031 및 관련 EV·세션 로그에 보존한다.

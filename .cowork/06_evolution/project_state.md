# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — TASK-021 로컬 구현·UPS 시뮬레이션 수락, 지속 실앱 후속 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — MS-008 In Progress(TASK-027 S2 facade·기본 worker EV-047 로컬 Pass), MS-006/007 진행 |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-008 In Progress — 소비처 통합 접점 경량화; MS-009 Planned; MS-006 In Progress; MS-002~005/007 검증 후속 |
| 활성 Task | TASK-027 S3 — 선택 연결·`health()`·CPU 고정·GAP-009. TASK-023 S2는 facade 경로로 착수 가능 |
| 미처리 리뷰 | 없음 — REV-046 자가 점검 Closed(64c4d23) |
| 상태 | Yellow — UPS 모의 수락, 운영/실앱·성능/RT/장기·native 후속 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-11-05 / #001 / AI |
| 세션 상태 | 진행 — TASK-027 S1 승인·S2 로컬 Pass(EV-047). 다음 S3 |
| 최근 세션 | [2026-11-05 #001](../members/lim8603/workspace/session_logs/session_2026-11-05_001.md) — TASK-027 S1 승인·S2 facade 구현 |

## 현재 결정과 제약

- **2026-11-05 Human 결정 — TASK-027 S1 facade 계약 승인:** 리뷰 포인트 5개를 하나씩 검토해 모두 권고안을 선택했다. ① [ADR-012](../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md) Accepted: 공유 `CaptureCore` + 소유 `Writer` + 빌림 `Producer<'_>` 뷰, Writer Drop은 반납함 이동만 하고 host/reaper가 최종 소유(ADR-008 borrowed core 유지). ② 성능 값(IO 1·CPU 고정 없음)은 facade 기본값, 손실 값(sync 주기·종료/Drop 기한·메모리·보존)은 `Budget`/preset 필수. ③ `Logger::drop`은 `drop_deadline` 기한 있는 종료. ④ NFR-008: L1 ≤20줄·≤6개념, L2 ≤40·≤10, L3 +6, facade 공개 ≤60. ⑤ `writer.try_log(channel, &msg)`. 계약은 [API §18](../03_design_artifacts/interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05), 시험은 TC-013 F1~F8.

- **2026-11-04 Human 결정 — 소비처 접점 경량화·시각화 범위 추가:** 사용성 평가(공개 항목 534개·기본 worker 없음·예산 명시 부담·Envelope 저장으로 표준 도구 판독 불가)에 동의하고 「권고대로」·「TASK-032 포함」·「MS 분리 가능」. [MS-008](../04_implementation/milestones/MS-008.md)(TASK-027 facade·기본 worker, TASK-028 preset·auto preset, TASK-029 레시피)와 [MS-009](../04_implementation/milestones/MS-009.md)(TASK-030 MCAP 표준 호환, TASK-031 viewer·Foxglove 계열 bridge, TASK-032 운영 대시보드) 등록. NFR-008 통합 용이성 추가, FR-006 viewer 확장, FR-015 장비 측정 auto preset만 TASK-028 S2로 부분 채택(질문 프리셋은 TASK-018 제안 유지). **순서:** TASK-027 → TASK-028 S1 → TASK-023 S2/S3(facade 경로)·RT 요청 → TASK-028 S2 → TASK-029 → MS-009 → probe 인계 → TASK-026. 11-03 결정의 "단독 작업 먼저, probe 연동 나중" 원칙 안에서 단독 작업 범위를 넓혔다. ADR 후보: facade 소유 모델(TASK-027 S1)·MCAP 표준 저장 모드(TASK-030 S2 전)·시각화 방식(TASK-031 S1).

- **2026-11-03 Human 결정 — probe 연동 전 dot-logger 단독 작업 선행:** 「이 프로젝트에서 할 수 있는걸 다 마치고 probe쪽에서 logger를 연동하는 세션을 진행」. ① 원격 ApplyConfig/Compare 운영 backend·CLI runtime metrics 구현(기존 API§15 범위), ② TASK-023 합성 행(P0 기준선·10ms 제어 off/on·P1~P3·P5·계측 비용)을 TASK-026 전에 generic 커널에서 실행 — 실앱 행은 TASK-026 뒤 유지, ③ 합성 측정 뒤 RT 준비/재시작 요청서를 제출하고 승인 후 RT 행 실행, ④ probe 연동 인계 패키지 작성 후 dot-probe 저장소 세션에서 TASK-026 진행. 24h soak(TASK-024)는 연동 뒤. dot-probe 저장소는 이 세션에서 수정하지 않는다.

- **2026-10-31 Human 승인·권한 위임:** TASK-012 기존 로컬 구현 수락 및 REV-019 개선·검증 후 마감 → TASK-013 착수 승인. 앞으로 승인된 목표의 구현·검증·리뷰 마감/작업 전환 판단과 작업 단위별 커밋·푸시는 AI가 수행·보고한다. 같은 범위 재승인은 받지 않는다. [위임 기준](../01_cowork_protocol/decision_authority_matrix.md#프로젝트-오너의-권한-위임--2026-10-31). 미실행 검증·native 공개 후 조건은 유지한다.

- **TASK-011 현재 계약:** frozen Snapshot·mono ns 동기 replay·prefetch≤1·checked generation/누적 읽기, 늦은 소비 시계/End 원본 수명·실제 reader close 후 반환을 유지한다. 상세 [TASK-011](../04_implementation/tasks/TASK-011.md)·[API§14.2](../03_design_artifacts/interface_contract.md#142-task-011-로컬-재생-api--2026-10-30).

- **TASK-010 현재 계약:** 고정 session/revision·독립 read-only source·완전 chunk prefix·유한 index/page/blob/read 예산·reader-owned pin은 실제 Drop 뒤 반환한다. closed bounded scan·pin 분실/forget 강제 회수 없음. [TASK-010](../04_implementation/tasks/TASK-010.md)·[API§14.1](../03_design_artifacts/interface_contract.md#141-task-010-로컬-api--2026-10-30).

- **TASK-009 현재 계약:** 원 session/revision/등록 순서·불변 read-only 원본·독립 빈 출력, 완전 검증 prefix만 복구한다. 자동 재동기화/과거 completion 복원/실패 파일 삭제·재시도 없음. [TASK-009](../04_implementation/tasks/TASK-009.md)·[API§13.5](../03_design_artifacts/interface_contract.md#135-task-009-중단-복구-api--2026-10-30).

- **2026-10-31 Human 구현 승인:** TASK-008/010/011 EV-012/013/016/017/020 및 REV-009/010/013/014/017 수락·Closed. 낮은 pin 유실/forget 강제 회수 없음·실제 reader close 뒤 반환과 실측/native 이월은 유지한다. 상세는 각 Task/API를 따른다.

- **TASK-012 계약:** remote 기본 disabled/loopback/read-only·외부 TLS/인증·bounded 구독/256KiB·handoff ACK/Gap/Unknown·세대 gate·host inbox. EV-023 소스4f79119는 이전 기준, server 오류 조치·구독 경계는 [EV-026](../05_verification/verification_evidence.md#ev-026)/b3c08bd·API§14.5를 따른다. 포화10ms 냉각 유지·실측은 TASK-022. native/실앱·LAN/RSS/보안 수락은 후속이며 OS keyring·CI/계정/과금 조회/차단 재시도 없음을 유지한다.

- **TASK-013:** EV-025/026 로컬 수락·최신 EV-029 회귀. local wait timeout은 같은 구독 유지, ttl은 record/idle lease 겸용·새 구독 간 Gap 연속성 미보장(S3 EV-042 화면 예제 수락). CLI runtime metrics Unavailable·config apply/compare Unsupported·실앱/native/성능 후속. [TASK-013](../04_implementation/tasks/TASK-013.md)·API§14.5.

- **TASK-014/015:** EV-029/REV-025 Closed. 제출·대여의 capacity/contract/lifecycle 분리·총계 보존, 과부하는 capacity만 사용한다. 부하율은 accepted+rejected_capacity 시도율(재시도 포함)이며 고유 입력/손실이 아니다. CounterCell128B/RT 할당·해제0, checked control 예약·Observation 이력/독점 reset 계약 유지. 원 session/revision/epoch/구간·장비/부하/target/boundary/clock을 대조하고 정확 terminal 차분·pending을 공급한다. 물리 bytes/slots·출력정지·filesystem/quota·보존을 분리하며 누락/foreign/Busy/overflow는 Unavailable이다. 실제 물리 gauge·운영 worker/CLI·실앱·PC/RT 비용≤5%/오차≤20%/native는 미검증. [TASK-014](../04_implementation/tasks/TASK-014.md)·[TASK-015](../04_implementation/tasks/TASK-015.md)·[API§15](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안).

- **TASK-016:** EV-030/031 로컬 수락·REV-027/028 Closed. startup enable·두 bank·producer posted/ack/cutoff/완료·실제 raw/Delivery guard 해제 뒤 회수, Busy/Conflict/checked revision을 유지한다. level/Normal sampling/기존 route만 live 변경하며 Important 우회·같은 loan·과거 수락 route 보존·혼합 revision Window 금지다. Comparison은 원 Report/profile/시계/입력/환경 차이와 skip/거절/출력 손실을 분리한다. revision별 파일은 별도 writer/예산이다. 운영 backend/CLI·실앱·PC/RT 비용/오차/native는 후속. [TASK-016](../04_implementation/tasks/TASK-016.md)·[API§15.2](../03_design_artifacts/interface_contract.md#152-task-016-로컬-설정-변경전후-비교--2026-11-01).

- **TASK-017 현재 계약:** EV-035·REV-034 조치 EV-036 로컬 수락. 고정 single-use File/Routed host·원 source/시계/권한·≤64KiB snapshot raw→TCP 수신 큐 ACK/원 receipt를 유지한다. 예제2초/요구24MiB·허용32MiB, capacity/freshness/LAN/RSS 미측정. 지속 앱/화면/off-on·실측/UPS/장기/native(T3) 미수락으로 Task/TC Partial 유지. [TASK-017](../04_implementation/tasks/TASK-017.md)·[API§15.7](../03_design_artifacts/interface_contract.md#157-task-017-고정-routed-runcheck--2026-11-01).

- **TASK-019:** S1~S3 EV-037~039 로컬 수락, REV-037/038 Closed. 새 독점 namespace·equal/older timestamp 거절·원 schema/receipt/real ns·actual IO/guard 수명을 유지한다. 통합/환경/정량 성능/native 잔여. [TASK-019](../04_implementation/tasks/TASK-019.md)·[API§14.8](../03_design_artifacts/interface_contract.md#148-task-019-s3-원-deliverydb-완료파일-격리--2026-11-01).

- **TASK-020:** S1~S3 EV-040~042 로컬 수락·REV-041/RV-083 Closed. TimedOut은 보류/같은 구독 확인, 종료/새 구독은 새 segment·연결 금지·누락 Unknown, 구독 내 Gap만 transfer 범위·새 span이다. 화면 포화 미ACK·종료행 보존·실제 owner 정리, 큰 ID/시각 문자열 계약 유지. 지속 앱/화면·장기 tail/복원·PC/RT/native는 후속. [전체 단계](../04_implementation/tasks/TASK-020.md#단계별-실행--2026-11-01-014).

- **TASK-021:** S1~S3 EV-043/REV-042 Closed·로컬 구현/UPS 시뮬레이션 수락. 기본 off·startup 계획·원 기한 최소값·rank/ID·동일 종료 coordinator·진행 IO 실제 수명·post-alarm snapshot0/사전 보호·partial 계약을 유지한다. 현재 PC ext4의8/64MiB×전달4조건×두 Rust Pass, 실제 sync/프로세스 중단/복구 oracle 일치. **실물 UPS는 필수 입력 아님.** native T3·장기/RT·실앱은 별도 후속. [전체 단계](../04_implementation/tasks/TASK-021.md#단계별-실행수락--2026-11-02-001).

- **TASK-022 승인 기준:** local-pc-v1의10ms 주기/기한·500µs 합성 연산·2GiB·P3 120초×3회·160GiB cap/64GiB reserve·UPS 모의3초/8·64MiB는 Human 승인 완료다. [EV-021](../05_verification/verification_evidence.md#ev-021)은 프로필 준비/승인 근거이며 실제 성능/RT는 NotRun, UPS 하위는 EV-043 시뮬레이션 수락이다.

- **2026-10-26 Human 승인:** 「권고 기준선 채택 — 장비·호환 검증은 계획대로 진행」. 상세설계·ADR-008/009·추가 시험 목표·MS 계획을 채택했다. 검증 이월은 면제가 아니다.

- **ADR-010 Accepted / 2026-10-26 Human 승인:** MCAP 0.26.0 채택. 0.25 선택 zstd 경로의 보안 공지를 해소했다. Tokio는 1.51.6 patch. 최종 audit 취약점/unsound 0, `paste` 유지보수 경고 1(GAP-005).

- Gate 1·2 Pass, ADR-003~006 기준선 유지. 1 GB/s 원본+10만 소형/s·32생산자/16파일/128채널·R1 2 GiB 및 512 MiB/4 GiB 비교는 조건부 목표이며 제품 기본값·달성 결과가 아니다.
- **ADR-007 Accepted:** dot-probe 내장형 Ports and Adapters + 제한된 생산자/소비자 파이프라인. 별도 데몬은 첫 범위에 넣지 않는다. 프로세스 장애까지 격리하지 않음.
- **클라이언트 형태 확정:** Rust API + CLI, dot-probe 기존 화면 연동. 명령/통신 상세도 ADR-009 기준선 채택 완료.
- **실제 장치 대체 환경 확정 — ADR-011 Accepted / 2026-10-31:** Human이 현재 Ubuntu PC 실측을 실제 장치 시험으로 대체 수락하고 실물 UPS 없이 시뮬레이션 결과로 판정하도록 승인했다. native는 GitHub 공개 완료 후 진행한다. **2026-11-02 Human 재확인: 현재는 일반 Ubuntu이며 RT 커널로 실행 중이 아니다**(uname:7.0.0-42-generic). 2026-10-31 RT 패키지 설치 관측은 과거 이력이며 현재 설치/부팅 가능 상태를 보장하지 않는다. RT 검증이 필요해지면 AI가 준비 상태·필요 설치/부팅 전환·재시작·시험 계획을 먼저 정리해 Human에게 요청하고 승인 후 진행한다. 부팅 뒤 실행 커널/CONFIG_PREEMPT_RT/capability를 확인해야 RT 행을 실행한다. 상세 [TASK-022](../04_implementation/tasks/TASK-022.md)·[ADR-011](../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md).
- RT 무대기·별도 중요도·중요 우선 기본값, 보존 모드 옵션과 처리/저장 예산 분리 유지. FR-018은 선택 옵션·기본 off, 전체 약3초는 신호 발생→차단 예산.
- 플랫폼 Tier 1/2 유지. affinity는 간섭 감소이며 완전 격리/보편적 성능 보장 아님. Tier 2 고급 기능 미지원은 capability로 표시한다.
- FR-015는 장비 측정 auto preset만 2026-11-04 TASK-028 S2로 부분 채택(선택·기본 off). 질문 프리셋 TASK-018은 별도 범위 승인 전 착수하지 않고 Must 완료에 몰래 넣지 않는다.

## 활성 Task 요약

| Task | 제목 | 담당 | 상태 | 갱신 | 다음 액션 |
|---|---|---|---|---|---|
| TASK-004 | 원본 풀·수명·무해제 반환 | AI | In Progress | 2026-10-31 | 로컬 Pass·Human 승인 유지; native GAP-004는 공개 완료 후(T3) 실행 |
| TASK-005 | 분배·출력 격리·예산 | AI | In Progress | 2026-10-31 | EV-004 로컬 Pass·Human 승인 유지; native는 공개 완료 후(T3) 실행 |
| TASK-006 | 완료 추적·명시 sync·종료 | AI | In Progress | 2026-10-31 | EV-009 최신 수정분 로컬 Pass·Human 승인; native는 공개 완료 후(T3) 실행 |
| TASK-007 | MCAP 봉투·writer·인덱스 | AI | In Progress | 2026-10-31 | EV-011 로컬 Pass·Human 승인·REV-008 Closed; native는 공개 완료 후(T3) 실행 |
| TASK-008 | 회전·순환·이벤트 보호 | AI | In Progress | 2026-10-31 | EV-012/013 Human 승인·현재 PC 실측, native 공개 후(T3) |
| TASK-009 | 중단 복구·IO 장애 | AI | In Progress | 2026-10-31 | EV-014/015 로컬 Pass·Human 승인; 현재 PC 대체 실측·모의 장애, native 공개 후 |
| TASK-010 | 시간·채널 조회/active snapshot | AI | In Progress | 2026-10-31 | EV-016/017 Human 승인·현재 PC 조회/IO 실측, native 공개 후(T3) |
| TASK-011 | 재생 시간과 취소 제어 | AI | In Progress | 2026-10-31 | EV-020 Human 승인·현재 PC 실제 시계/IO 실측, native 공개 후 |
| TASK-012 | gRPC 구독·ACK·관리 서버 | AI | In Progress | 2026-10-31 | EV-023 수락 유지·REV-022 Closed/EV-026, native/실측 잔여 |
| TASK-013 | Rust 클라이언트와 CLI | AI | In Progress | 2026-10-31 | EV-025 수락 유지·REV-021 Closed·REV-022 Closed/EV-026, native T3·실앱/성능 후속 |
| TASK-014 | 운영 계측·유한 집계 | AI | In Progress | 2026-10-31 | EV-029 거절 범주·이력 복구 조치 수락/REV-025 Closed; 비용≤5%·PC/RT/native 후속 |
| TASK-015 | 여유·병목·보존 시간 리포트 | AI | In Progress | 2026-10-31 | EV-029 압력/시도 부하·host 출력 정렬 조치 수락/REV-025 Closed; PC/RT 오차/native·실앱 후속 |
| TASK-016 | revision 설정 변경과 전후 비교 | AI | In Progress | 2026-11-03 | EV-030/031·운영 backend EV-044/REV-044 Closed; 실앱·PC/RT/native 후속 |
| TASK-017 | 예산 제한 간편 점검 | AI | In Progress | 2026-11-01 | 고정 routed RunCheck EV-035로 로컬 기능 잔여 처리; 지속 앱/화면·통합 실측/native 수락 대기 |
| TASK-019 | InfluxDB 어댑터와 충돌·장애 | AI | In Progress | 2026-11-01 | S1~S3 EV-037~039 로컬 수락; REV-038 Closed·통합/환경/native 후속 |
| TASK-020 | tracing 어댑터와 통합 예제 | AI | In Progress | 2026-11-02 | S1~S3 EV-040~042 로컬 수락·REV-041 Closed; 통합 TASK-026/환경/native 대기 |
| TASK-021 | UPS 긴급 계획·모의 시험 | AI | In Progress | 2026-11-02 | S1~S3 EV-043 로컬 구현/UPS 모의 수락·REV-042 Closed, native T3·통합 후속; 다음 TASK-026 |
| TASK-023 | P0~P5 성능·RT·자원 검증 | AI | In Progress | 2026-11-04 | S2 — TASK-027 facade 기본 worker 경로로 logger on 측정(S1 EV-046 완료) |
| TASK-027 | 기본 실행 계층(facade)과 기본 worker | AI | In Progress | 2026-11-05 | S2 [EV-047](../05_verification/verification_evidence.md#ev-047) 로컬 Pass(F1~F8, panic 시험 GAP-009); 다음 S3 |
| TASK-022 | 실제 장비·플랫폼 검증 프로필 | AI | In Progress | 2026-10-31 | local-pc-v1 수치 Approved·선행 harness 준비, RT 행 부팅/capability 확인 후 |

## 코드리뷰 현황 (Reviews)

| ID | 범위 | 상태 | 현재 처리/후속 |
|---|---|---|---|
| REV-001~017 | 코어·파일·조회·재생 | Closed | 로컬 조치·Human 수락 완료, native/PC 실측 이월은 유지 |
| REV-018~022 | 원격·client·CLI | Closed | EV-026까지 수락. RV-049→TASK-022, RV-053→TASK-020 |
| REV-023~028 | 계측·리포트·설정변경 자가/독립 리뷰 | Closed | EV-027~031 로컬 수락 마감, 비용·오차·실앱/native 후속 |
| REV-029~041 | 간편 점검·DB·tracing/화면 예제 | Closed | EV-031~042 로컬 수락, 실앱/환경 후속은 각 Task 참조 |
| [REV-042](../05_verification/reviews/REV-042_TASK-021_UPS_SIMULATION.md) | UPS 긴급 계획·모의 시험 자가 점검 | Closed | RV-084 기한 경합·RV-085 ext4 프로필 조치, EV-043 수락 |
| [REV-043](../05_verification/reviews/REV-043_TASK-021_INDEPENDENT_REVIEW.md) | TASK-021 UPS 긴급 계획·모의 시험 독립 코드리뷰 | Closed | RV-086/RV-087 rustdoc 선행조건·1회 관측 계약 보강(9cbbfb8), 동작 변경 없음 |
| [REV-044](../05_verification/reviews/REV-044_TASK-016_OPERATIONS_BACKEND.md) | 운영 backend(status·apply·compare) 자가 점검 | Closed | RV-088~090 P3 문서 처리, EV-044 로컬 수락. 독립 리뷰 미실시 |
| [REV-045](../05_verification/reviews/REV-045_TASK-016_OPERATIONS_BACKEND_INDEPENDENT_REVIEW.md) | TASK-016 운영 backend 독립 코드리뷰 | Closed | RV-091 `# Errors`·RV-092 Debug·RV-093 snapshots ID 오름차순(API§15.8) 조치(497149b) |
| [REV-046](../05_verification/reviews/REV-046_TASK-027_S2_FACADE.md) | TASK-027 S2 facade·기본 worker 자가 점검 | Closed | RV-094 panic 시험→GAP-009, RV-095~099 P3 문서화/S3. 독립 리뷰 미실시 |

이전 회차별 대상·판정·처리 이력은 [리뷰 문서](../05_verification/reviews/)와 [상태 아카이브](state_archive.md)의 #014 이관을 따른다.

## 다음 시작점

1. **단독 작업(2026-11-03 결정, 2026-11-04 MS-008/009로 확장):** ~~운영 backend~~ EV-044·~~REV-045~~·~~GAP-008~~ EV-045·TASK-023 S1 EV-046 완료 → ~~TASK-027 S1~~ 승인 → ~~S2~~ EV-047(`1af172c`·`64c4d23`) → **다음 TASK-027 S3**(slot 직접 encode·tracing/remote/계측 선택 연결·`health()`·CPU 고정·worker panic 시험 GAP-009·api-consumer 갱신) → TASK-028 S1 → TASK-023 S2(facade 경로; P1 256B·P2 8MiB·P3 혼합, 32생산자/16파일/128채널, P3 혼합은 `StorageWorkload` 확장 필요)/S3(120초×3·P5·계측 비용, P4 배정 확인) → RT 요청/승인 후 RT 행 → TASK-028 S2 auto preset → TASK-029 → MS-009(TASK-030~032) → probe 연동 인계 패키지.
2. **TASK-026 실제 첫 적용·화면 연동(dot-probe 저장소 세션):** 별도 저장소 지침과 기존 composition/화면을 확인하고 logger off/on 동일 제어 시나리오부터 TDD로 진행한다. 아직 미착수다. TASK-020 S1~S3·TASK-021 S1~S3는 로컬 수락, TASK-026은 [진입점](../04_implementation/tasks/TASK-026.md)을 따르며 MS-008/009 뒤 facade 경로로 연동한다. TASK-018은 질문 프리셋 부분만 FR-015 승인 대기다.
3. **UPS 수락 완료/환경 후속:** TC-011 현재 PC ext4의 승인된 모의3초·실제 sync/중단/복구는 EV-043으로 수락했다. 실물 UPS는 필요하지 않다. 지속 앱/화면/off-on·P0~P6/RT/24h·native는 TASK-026/022~024·공개 후 T3다. 첫 적용 전이라 T1 미도래다.
4. 설정/비교 backend·worker, RV-049/TASK-022, PC/RT 비용/오차·장기·native/OS·Gate4/5 미완료. RV-053의 유한 화면 예제는 EV-042로 소비했으나 장기 tail/구독 간 복원은 미수락이다. RT 부팅·T2/T3 조건 유지.

## 승인·재개 제약

기존 구현·검증·수락·작업 전환·작업 단위별 커밋/푸시 위임은 유지한다. 새 범위·공개 전환·선택 기능 자동 착수로 확대하지 않는다. native 공개 후(T3)·PC/RT/장기/실앱·설정/비교·지속 실앱 통합·Gate4/5 미완료 조건은 유지한다. 상세 승인/이전 에디터 진단 정정·과거 재개 메모는 [상태 아카이브](state_archive.md)의 #014 이관과 관련 Task/API를 따른다. 개발1.96.0·MSRV1.85·Miri nightly-2026-10-26 고정, 전역 rustup default/사용자 에디터 설정은 변경하지 않는다.

## 선택 로딩

- [도메인](../03_design_artifacts/domain_model.md), [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안), [데이터](../03_design_artifacts/data_model.md), [CLI](../03_design_artifacts/ui_spec.md).
- [MS 목록](../04_implementation/milestone_registry.md): 9개 MS, [Task 목록](../04_implementation/task_registry.md): 32개 Task(1개 선택). 선행조건·변경 대상·첫 실패/DoD·상대 인일 추정(계획 승인, 납기 확정 아님).
- [요구 추적표](../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트): INT 1 + FR 18 + NFR 8 + CON 7 = 34행. MS별 구현/테스트 체크·코드/EV 연결. TC-001의 TASK-002 하위 EV-001 로컬/native, TC-002 소형 EV-002 로컬/native·원본 EV-003 로컬, TC-003 분배 EV-004·완료/제어 EV-005 로컬 Pass; 후속/native/장비 시험 잔여.
- [2026-10-26 조사](imported_context/research_2026-10-26_detailed_design.md): 공식 자료와 dot-probe 코드 관측, 기술 후보/사실/미검증 분리.
- 이전 결정 근거가 필요할 때만 ADR-003~006·요구 §12~17·Story 상세·개인 로그를 추가로 읽는다.

## 이월 백로그

다음 T = T4 / 다음 B = B1. 트리거 감시는 AI 책임이다.

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| T1 | ROS 2 파일 호환·토픽 연동 필요/범위 검토 | dot-probe 첫 적용 완료가 확인된 때 | INT-001 / OQ-008 / TASK-026 | 2026-10-25 Human Q5 |
| T2 | 공개 전환 전 라이선스·GitHub 운영정책·공개할 이력/로그 검토 | 공개 전환 또는 첫 외부 배포를 요청할 때 | INT-001 / 공개 준비 | 2026-10-28 Human 논의 |
| T3 | 이월 native 검증 실행·결과 반영 | GitHub 저장소 공개 완료가 확인된 때 | GAP-004 / ADR-011 / TASK-004~017·019~021 | 2026-10-31 Human 검증 시점 결정 |

범위 밖 백로그 B: 없음. 현재 T1/T2/T3는 미도래. 공개 후 native 실행 시점 지정은 공개 전환 자체 요청이 아니므로 T2를 지금 발동하지 않는다. OQ를 T/B에 중복 등록하지 않는다.

## 열린 결정과 검증 공백

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| OQ-002 | ADR-011로 현재 PC 실측·UPS 시뮬레이션 수락 확정. local-pc-v1 수치 Approved(EV-021), UPS 하위 EV-043 수락·성능/RT 실측 잔여 | 실앱/성능 harness·RT 부팅 확인 후 현재 PC 성능/TC-012·장기 통합 실행 |
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·실제 앱/장비 수명 수락은 미검증 | TASK-004~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬/native CI 확인(EV-001), 실제 DB·장비 호환 잔여 | TASK-019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증, TASK-018은 별도 범위 승인 후 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·004·005 | PC 환경 확정·프로필/후속 TC 실행 잔여, TASK-004~017·019~021 native 공개 후(T3) 이월·paste 경고. GAP-003 Closed | [검증 근거](../05_verification/verification_evidence.md) 참조 |
| GAP-009 | Open — TC-013 F5 worker panic 자동 시험 없음(처리 코드는 구현, RV-094) | TASK-027 S3에서 주입점 검토 |
| GAP-010 | Open — Rust 1.85 clippy의 기존 `adapters/tracing.rs` needless_lifetimes 1건(변경 전부터) | 해당 파일 수정 시 정리 |
| GAP-008 | Closed — hyper stream task가 serve 반환 뒤 hub 예약을 보유하던 순서 문제. serve가 서버 측 handle 해제까지 대기(EV-045, 13523f1), 동시4 19/200→0/200 | 재발 시 같은 방법으로 재측정 |
| GAP-006 | EV-006 자가 리뷰 RV-001~004는 EV-007 수정·로컬 회귀 Pass로 Closed | Human 승인 완료·native GAP-004 후속, 기존 이력 보존 |
| GAP-007 | RV-005는 EV-009 로컬 수정/회귀 Pass·Human 승인으로 Closed | native는 후속, 수정 전 EV-008 이력 보존 |
| RV-015~017 | 최종 sync 취소는 EV-014, 방어적 슬라이싱/스택 포맷은 EV-015에서 반영·REV-011/012 Closed·Human 승인 | native·장비 검증 대기 |
| RV-024/025 | TASK-011 늦은 소비 중 배속 변경/오류 정리 반례를 EV-018에서 조치, REV-015 Closed·2026-10-31 Human 승인 | native·장비 검증 대기 |
| RV-026~028 | RV-026 현 불변식상 overflow 불가·방어 정리, RV-027/028 진단 API 조치·EV-019 Pass·REV-016 Closed·2026-10-31 Human 승인 | native·장비 검증 대기 |
| RV-032~035 | TASK-012 auth/세대/reply capacity/blob 요청 길이 반례 조치·EV-022 Pass·REV-018 Closed | 2026-10-31 Human 수락·REV-018 Closed, native/실앱·LAN/전체RSS 후속 |
| RV-029~031 | REV-017 반입·MSRV 유지/비율 비교/오류 진단·EV-020 Pass, Closed·Human 승인 | native·PC 실측 후속 |
| RV-021~023 | 동일 Query 정렬 재사용·Page inline/누적 읽기 예산 안내를 EV-017에서 조치, REV-014 Closed·Human 승인 | native·PC 실측 후속 |
| RV-018~020 | TASK-010 만료 복원·blob Gone·frame Limit을 EV-016에서 조치, REV-013 Closed·Human 승인 | native·PC 실측 후속 |
| RV-006~010 | EV-011 로컬 수정·Human 승인 완료, REV-008 Closed | native GAP-004·장비 성능은 잔여 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-11-05 #001: TASK-027 S2 — capture를 공유 core/endpoint로 분해(`1af172c`, 공개 시그니처 불변)하고 facade·기본 worker를 구현(`64c4d23`). TC-013 F1~F8·allocator F2/F3, workspace 1.96/1.85 각 460 Pass(EV-047), REV-046 Closed.
- 2026-11-05 #001: TASK-027 S1 — core 빌림 구조 조사 후 ADR-012·API §18·NFR-008 수치를 제안했고, Human이 리뷰 포인트 5개를 모두 권고안으로 승인했다(ADR-012 Accepted, MS-008 In Progress, TC-013 F1~F8 상세화). 제품 코드 변경 없음.
- 2026-11-04 #001: 객관 평가(엔진은 가벼운 설계·통합 접점은 무거움, Envelope 저장으로 표준 MCAP 도구 판독 불가) 후 MS-008/009·TASK-027~032·NFR-008·TC-013/014 계획 등록. 제품 코드 변경 없음.
- 2026-11-03 #003: TASK-023 S1 P0 기준선 harness·판정 규칙·합성 제어 루프(EV-046, 소스 3740671). `/data/build` 여유 부족은 Human 승인으로 build cache(target/debug·gate-target) 정리해 472.9GB 회복.
- 2026-11-03 #003: GAP-008 원인(hyper HTTP/2 stream task의 hub handle 지연 해제)을 확인하고 serve drain 대기로 수정(EV-045, 소스 13523f1), Closed.
- 2026-11-03 #003: REV-045 RV-091~093 수용·조치(소스 497149b: `# Errors` rustdoc·Debug·status snapshots ID 오름차순)·회귀 Pass, Closed.
- 2026-11-03 #002: TASK-016 운영 backend(status·apply·compare)에 대한 독립 코드리뷰([REV-045](../05_verification/reviews/REV-045_TASK-016_OPERATIONS_BACKEND_INDEPENDENT_REVIEW.md))를 수행하여 Open 상태로 등록했다. (P1/P2 0건, P3 권고 3건: rustdoc #Errors 명시, Debug derive, 스냅샷 정렬 계약)
- 2026-11-03 #001: 운영 backend(status 지표·ApplyConfig·Compare, 새 오류 3종) EV-044/REV-044 로컬 수락(소스 a738ebf). GAP-008 사전 존재 불안정 시험 등록.
- 2026-11-03 #001: REV-043 RV-086/087 rustdoc 보강(소스 9cbbfb8)·검증 후 Closed, 미처리 리뷰 없음 — [2026-11-03 #001](../members/lim8603/workspace/session_logs/session_2026-11-03_001.md).

- 2026-11-02 #002: TASK-021 UPS 긴급 계획·모의 시험에 대한 독립 코드리뷰([REV-043](../05_verification/reviews/REV-043_TASK-021_INDEPENDENT_REVIEW.md))를 수행하여 Open 상태로 등록했다. (P1/P2 0건, P3 문서 권고 2건)

- 2026-11-02 #001: TASK-020 S3 화면 예제·TASK-021 UPS 시뮬레이션을 EV-042/043·REV-041/042로 로컬 수락했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-02_001.md).

- 2026-11-01 #015: TASK-020 S2 실제 파일/DB/구독·명시 설정 비교 예제를 EV-041/REV-040으로 로컬 수락했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_015.md).

- 2026-11-01 #014: REV-038 문서 조치와 TASK-020 S1 일반 로그 입력을 EV-040/REV-039로 로컬 수락했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_014.md).




R1/R2 이관 원문은 [상태 아카이브](state_archive.md)의 #020/021을 따른다. 이전 완료 이력과 게시/승인 상세는 [상태 아카이브](state_archive.md)·관련 Task/EV·세션 로그를 따른다.

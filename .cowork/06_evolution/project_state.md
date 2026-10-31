# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 수집·분배·저장/복구·조회/재생·gRPC/ACK·계측/리포트·설정/비교 로컬 검증, 운영/worker·실앱 후속 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — TASK-016 EV-030 로컬 수락, MS-005 진행 |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-005 In Progress — 운영 계측, MS-004/007은 native·실앱/실측 후속 |
| 활성 Task | TASK-016 In Progress — EV-030 로컬 수락·운영 backend/실앱/PC·RT/native 후속; 다음 후보 TASK-017 |
| 미처리 리뷰 | 없음 — REV-001~027 Closed. REV-027 TASK-016 개발 자가 점검 Pass/위임 로컬 수락; RV-049/TASK-022·RV-053/TASK-020 후속 |
| 상태 | Yellow — 클라이언트/계측 로컬 수락, 운영 backend/실앱·비용/PC/RT/UPS·native 후속 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-11-01 / #002 TASK-016 로컬 수락 / AI |
| 세션 상태 | 진행 — TASK-016 구현/검증/로컬 수락 보고. TASK-017 Planned/미착수 |
| 최근 세션 | [2026-11-01 #002](../members/lim8603/workspace/session_logs/session_2026-11-01_002.md) — TASK-016 설정 변경/비교·EV-030·REV-027 Closed |

## 현재 결정과 제약

- **2026-10-31 Human 승인·권한 위임:** TASK-012 기존 로컬 구현 수락 및 REV-019 개선·검증 후 마감 → TASK-013 착수 승인. 앞으로 승인된 목표의 구현·검증·리뷰 마감/작업 전환 판단과 작업 단위별 커밋·푸시는 AI가 수행·보고한다. 같은 범위 재승인은 받지 않는다. [위임 기준](../01_cowork_protocol/decision_authority_matrix.md#프로젝트-오너의-권한-위임--2026-10-31). 미실행 검증·native 공개 후 조건은 유지한다.

- **TASK-011 현재 계약:** frozen Snapshot을 소유하는 mono ns 동기식 replay·최대1건 prefetch·양수 u32 유리수 배속·pause/resume/seek/stop·checked generation과 누적 읽기/원본 수명을 사용한다. 늦은 소비자의 재생 시계를 늦추고 원격 in-flight 취소를 추정하지 않는다. End는 마지막 blob read를 위해 snapshot을 유지하고 stop/Drop·terminal 오류에서 실제 reader close 뒤 반환한다. 상세 [TASK-011](../04_implementation/tasks/TASK-011.md)·[API §14.2](../03_design_artifacts/interface_contract.md#142-task-011-로컬-재생-api--2026-10-30).

- **TASK-010 현재 계약:** 고정 session/revision/원래 등록·독립 read-only source·완전 chunk safe offset, 유한 external/index/page/blob/read 예산을 사용한다. reader-owned permit은 active→closed 회전을 따라 보호하며 실제 reader Drop 뒤에만 회수 가능하다. 저수준 pin 분실/forget의 강제 회수·clock 변환은 도입하지 않는다. 현재 closed도 bounded scan으로 index를 재구성하며 summary scan 생략 최적화·자동 worker/CLI·실제 성능/OS 검증은 후속이며 local replay/원격 host 연결은 EV-018/022 범위다. 상세 [TASK-010](../04_implementation/tasks/TASK-010.md)·[API §14.1](../03_design_artifacts/interface_contract.md#141-task-010-로컬-api--2026-10-30).

- **TASK-009 현재 계약:** 불변 read-only 원본·원래 session/revision/생산자 수/동일 순서의 채널 등록·독립된 빈 출력이 필요하다. 완전 검증 chunk prefix만 복구하며 손상 뒤 재동기화·과거 completion 응답 복원·실패 파일 자동 삭제/재시도는 하지 않는다. 상세: [TASK-009](../04_implementation/tasks/TASK-009.md)·[API §13.5](../03_design_artifacts/interface_contract.md#135-task-009-중단-복구-api--2026-10-30).

- **2026-10-31 Human 구현 승인:** 「승인, 다음작업 진행하자」로 TASK-008 EV-012/013·TASK-010 EV-016/017·TASK-011 EV-020을 수락했다. REV-009/010/013/014/017 및 해당 RV Closed, TASK-008/010/011 Human 리뷰 완료. 기존 리뷰는 닫았으며 신규 TASK-012 리뷰는 별도다. 낮은 pin 유실/forget 강제 회수 없음·실제 reader close 뒤 반환 계약과 실측/native 이월은 유지한다.

- **TASK-012 계약:** remote 기본 disabled/loopback/read-only·외부 TLS/인증·bounded 구독/256KiB·handoff ACK/Gap/Unknown·세대 gate·host inbox. EV-023 소스4f79119는 이전 기준, server 오류 조치·구독 경계는 [EV-026](../05_verification/verification_evidence.md#ev-026)/b3c08bd·API§14.5를 따른다. 포화10ms 냉각 유지·실측은 TASK-022. native/실앱·LAN/RSS/보안 수락은 후속이며 OS keyring·CI/계정/과금 조회/차단 재시도 없음을 유지한다.

- **TASK-013 기능 기준:** EV-025/source29e55da 뒤 [EV-026](../05_verification/verification_evidence.md#ev-026)/b3c08bd로 handoff·진단/CLI·유휴 next를 보강했다(검사14개/Miri32). local wait timeout은 같은 구독 유지, server lease/명시 cancel은 종료한다. ttl은 record/idle lease 겸용, 별도 구독 간 Gap 연속성은 미보장이고 TASK-020 화면에서 불연속 표시한다. 현재 workspace는 REV-025 EV-029/eaf736c로 전체 회귀했다. CLI runtime metrics Unavailable·운영 backend Unsupported·실앱/native/성능 후속은 유지한다.

- **TASK-014/015 현재 계약·형상:** [EV-029](../05_verification/verification_evidence.md#ev-029) source101개/검사14개·Miri224·소스eaf736c commit blob일치, REV-025 Closed. EV-027/028의 계측·report 기반에 제출/대여 capacity/contract/lifecycle 범주를 추가하고 총계는 보존했다. 거절 중 capacity만 과부하로 사용하며 기존 출력 실패/자원 한계 판정은 유지한다. 부하율은 accepted+rejected_capacity **시도율**(재시도 포함·고유 입력/손실 아님)이다. acquire·계약/수명 거절·skip은 분자 제외. CounterCell128B/RT 할당·해제0, 추가 Sample 크기는 checked control 예약으로 검사한다. history_len은 Observation<usize>, 독점 reset은 이력/poison을 초기화한다.
- **리포트/호스트 경계:** 실제 Window session/revision/epoch/구간·장비/부하/target/boundary와 검증 능력의 session clock/유효성을 유지한다. 물리 bytes/slots·출력정지·filesystem/quota·순환 보존/retained span을 분리한다. host는 성공 sample과 target_totals의 유한 이력에서 실제 from/to를 대조하고 terminal checked 차분·최종 pending을 공급한다. 누락/foreign/Busy/overflow는 Unavailable이며 가까운 시점 혼합/배출 후 정확 계약이다. 물리 gauge/출력 출처·capacity evidence·자동 worker/CLI runtime backend·실앱과 실제 PC/RT 오차≤20%·계측 비용≤5%·native는 후속이다. 상세 [TASK-014](../04_implementation/tasks/TASK-014.md)·[TASK-015](../04_implementation/tasks/TASK-015.md)·[API§15](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안).

- **TASK-016 로컬 계약:** EV-030/source542c6e6·source102개·검사10개·Miri496·소스/commit blob102개 일치, REV-027 개발 자가 점검 Pass/Closed·위임 수락. 명시 enable/preview/validate/apply와 두 bank·posted/per-producer ack·old accepted cutoff/완료/실제 raw·Delivery guard 해제 뒤 회수, Busy/Conflict/checked revision을 사용한다. level/Normal sampling/기존 target route만 live 변경하며 Important sampling 우회·같은 raw loan·과거 수락 route를 보존한다. 전환 Sample은 RevisionTransition, 혼합 revision Window 금지. Comparison은 원래 Report/profile/revision/구간·입력/장비/boundary/측정 scope 차이와 skip/거절/출력 손실을 분리한다. 단일 revision 파일은 명시 budget/별도 writer다. 운영 backend/CLI runtime/자동 worker·실앱/실제 PC·RT 비용/오차/native는 후속. [TASK-016](../04_implementation/tasks/TASK-016.md)·[API §15.2](../03_design_artifacts/interface_contract.md#152-task-016-로컬-설정-변경전후-비교--2026-11-01).

- **TASK-022 승인 기준:** local-pc-v1의10ms 주기/기한·500µs 합성 연산·2GiB·P3 120초×3회·160GiB cap/64GiB reserve·UPS 모의3초/8·64MiB는 Human 승인 완료다. [EV-021](../05_verification/verification_evidence.md#ev-021)은 프로필 준비/승인 근거이며 실제 성능/RT/UPS는 NotRun이다.

- **2026-10-26 Human 승인:** 「권고 기준선 채택 — 장비·호환 검증은 계획대로 진행」. 상세설계·ADR-008/009·추가 시험 목표·MS 계획을 채택했다. 검증 이월은 면제가 아니다.

- **ADR-010 Accepted / 2026-10-26 Human 승인:** MCAP 0.26.0 채택. 0.25 선택 zstd 경로의 보안 공지를 해소했다. Tokio는 1.51.6 patch. 최종 audit 취약점/unsound 0, `paste` 유지보수 경고 1(GAP-005).

- Gate 1·2 Pass, ADR-003~006 기준선 유지. 1 GB/s 원본+10만 소형/s·32생산자/16파일/128채널·R1 2 GiB 및 512 MiB/4 GiB 비교는 조건부 목표이며 제품 기본값·달성 결과가 아니다.
- **ADR-007 Accepted:** dot-probe 내장형 Ports and Adapters + 제한된 생산자/소비자 파이프라인. 별도 데몬은 첫 범위에 넣지 않는다. 프로세스 장애까지 격리하지 않음.
- **클라이언트 형태 확정:** Rust API + CLI, dot-probe 기존 화면 연동. 명령/통신 상세도 ADR-009 기준선 채택 완료.
- **실제 장치 대체 환경 확정 — ADR-011 Accepted / 2026-10-31:** Human이 현재 Ubuntu PC 실측을 실제 장치 시험으로 대체 수락하고 실물 UPS 없이 시뮬레이션 결과로 판정하도록 승인했다. native는 GitHub 공개 완료 후 진행한다. 현재 실행7.0.0-42-generic(CONFIG_PREEMPT_RT 미설정), 설치7.0.0-38-realtime(CONFIG_PREEMPT_RT=y)을 확인했다. RT 행은 해당 커널 부팅 확인 후 실행하며 이번에 재부팅/설정 변경은 하지 않았다. 상세 [TASK-022](../04_implementation/tasks/TASK-022.md)·[ADR-011](../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md).
- RT 무대기·별도 중요도·중요 우선 기본값, 보존 모드 옵션과 처리/저장 예산 분리 유지. FR-018은 선택 옵션·기본 off, 전체 약3초는 신호 발생→차단 예산.
- 플랫폼 Tier 1/2 유지. affinity는 간섭 감소이며 완전 격리/보편적 성능 보장 아님. Tier 2 고급 기능 미지원은 capability로 표시한다.
- FR-015는 제안. TASK-018은 별도 범위 승인 전 착수하지 않고 Must 완료에 몰래 넣지 않는다.

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
| TASK-016 | revision 설정 변경과 전후 비교 | AI | In Progress | 2026-11-01 | EV-030 로컬 수락/REV-027 Closed; 운영 backend/실앱·PC/RT/native 후속, 다음 TASK-017 |
| TASK-022 | 실제 장비·플랫폼 검증 프로필 | AI | In Progress | 2026-10-31 | local-pc-v1 수치 Approved·선행 harness 준비, RT 행 부팅/capability 확인 후 |

## 코드리뷰 현황 (Reviews)

| ID | 범위 | 상태 | 현재 처리/후속 |
|---|---|---|---|
| REV-001~017 | 코어·파일·조회·재생 | Closed | 로컬 조치·Human 수락 완료, native/PC 실측 이월은 유지 |
| REV-018~022 | 원격·client·CLI | Closed | EV-026까지 수락. RV-049→TASK-022, RV-053→TASK-020 |
| [REV-023](../05_verification/reviews/REV-023_TASK-014_TELEMETRY.md)·[REV-024](../05_verification/reviews/REV-024_TASK-015_REPORT.md) | 계측·리포트 개발 자가 점검 | Closed | EV-027/028 로컬 수락, 비용·오차·실앱/native 후속 |
| [REV-025](../05_verification/reviews/REV-025_TASK-014-015_INDEPENDENT_REVIEW.md) | TASK-014/015 독립 리뷰 P2 1·P3 2 | Closed | EV-029/eaf736c·검사14개/Miri224·source101개 blob일치, 기존 위임 수락 |
| [REV-026](../05_verification/reviews/REV-026_TASK-015_RV-060-062_REVIEW.md) | TASK-015 RV-060~062 조치·후속 형상 검증 | Closed | Pass (P1/P2/P3 0건), RV-060~062 조치 및 계약·RT 안전성 확인 완료, 수락 마감 |
| [REV-027](../05_verification/reviews/REV-027_TASK-016_CONFIG_COMPARE.md) | TASK-016 설정/비교 개발 자가 점검 | Closed | EV-030/source542c6e6·검사10개/Miri496·source102개 blob일치·위임 로컬 수락 |

이전 회차별 대상·판정·처리 이력은 [리뷰 문서](../05_verification/reviews/)와 [상태 아카이브](state_archive.md)의 #014 이관을 따른다. 과거 Resolved 상태와 현재 Closed를 혼동하지 않는다.

## 다음 시작점

1. **TASK-016 로컬 구현·검증·자가 점검 수락 완료.** EV-030/source542c6e6·source102개·검사10개·Miri496·REV-027 Closed, native/운영 backend/실앱/실측 이월 유지. API §15.2의 ack/old plan/단일 revision 파일과 전환 계측·비교 조건을 다음 소비자에서 유지한다.
2. **다음 개발 후보 TASK-017(예산 제한 간편 점검)은 Planned/미착수.** 상세/API §15/TC-010에서 dry-run 무부하·쓰기/시간 예산 선도달·수신자 없음·취소부터 조사/TDD한다. 이번 요청은 TASK-016 작업 단위까지 수행했으며 TASK-017 제품 코드를 만들지 않았다. 새 H/J 결정이 생길 때만 판단을 받는다.
3. RV-049 포화 회복 실측은 TASK-022, RV-053 server lease/재구독 화면 불연속은 TASK-020 후속이다. ApplyConfig/Compare 원격 운영 backend·CLI runtime 연결/자동 worker도 실앱 통합과 함께 명시 구동한다.
4. native는 공개 후(T3), 실제 PC/RT 오차·비용/UPS/실앱·TC-009/010 전체/MS-005/Gate4·5는 미완료. 새 T1/T2/T3 도래 근거 없음.

## 승인·재개 제약

기존 구현·검증·수락·작업 전환·작업 단위별 커밋/푸시 위임은 유지한다. 새 범위·공개 전환·선택 기능 자동 착수로 확대하지 않는다. native 공개 후(T3)·PC/RT/UPS/실앱·운영 backend·Gate4/5 미완료 조건은 유지한다. 상세 승인/이전 에디터 진단 정정·과거 재개 메모는 [상태 아카이브](state_archive.md)의 #014 이관과 관련 Task/API를 따른다. 개발1.96.0·MSRV1.85·Miri nightly-2026-10-26 고정, 전역 rustup default/사용자 에디터 설정은 변경하지 않는다.

## 선택 로딩

- [도메인](../03_design_artifacts/domain_model.md), [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안), [데이터](../03_design_artifacts/data_model.md), [CLI](../03_design_artifacts/ui_spec.md).
- [MS 목록](../04_implementation/milestone_registry.md): 7개 MS, [Task 목록](../04_implementation/task_registry.md): 26개 Task(1개 선택). 선행조건·변경 대상·첫 실패/DoD·상대 인일 추정(계획 승인, 납기 확정 아님).
- [요구 추적표](../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트): INT 1 + FR 18 + NFR 7 + CON 7 = 33행. MS별 구현/테스트 체크·코드/EV 연결. TC-001의 TASK-002 하위 EV-001 로컬/native, TC-002 소형 EV-002 로컬/native·원본 EV-003 로컬, TC-003 분배 EV-004·완료/제어 EV-005 로컬 Pass; 후속/native/장비 시험 잔여.
- [2026-10-26 조사](imported_context/research_2026-10-26_detailed_design.md): 공식 자료와 dot-probe 코드 관측, 기술 후보/사실/미검증 분리.
- 이전 결정 근거가 필요할 때만 ADR-003~006·요구 §12~17·Story 상세·개인 로그를 추가로 읽는다.

## 이월 백로그

다음 T = T4 / 다음 B = B1. 트리거 감시는 AI 책임이다.

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| T1 | ROS 2 파일 호환·토픽 연동 필요/범위 검토 | dot-probe 첫 적용 완료가 확인된 때 | INT-001 / OQ-008 / TASK-026 | 2026-10-25 Human Q5 |
| T2 | 공개 전환 전 라이선스·GitHub 운영정책·공개할 이력/로그 검토 | 공개 전환 또는 첫 외부 배포를 요청할 때 | INT-001 / 공개 준비 | 2026-10-28 Human 논의 |
| T3 | 이월 native 검증 실행·결과 반영 | GitHub 저장소 공개 완료가 확인된 때 | GAP-004 / ADR-011 / TASK-004~015 | 2026-10-31 Human 검증 시점 결정 |

범위 밖 백로그 B: 없음. 현재 T1/T2/T3는 미도래. 공개 후 native 실행 시점 지정은 공개 전환 자체 요청이 아니므로 T2를 지금 발동하지 않는다. OQ를 T/B에 중복 등록하지 않는다.

## 열린 결정과 검증 공백

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| OQ-002 | ADR-011로 현재 PC 실측·UPS 시뮬레이션 수락 확정. local-pc-v1 수치 Approved(EV-021), 실행 준비/실측 잔여 | 선행 harness·실앱/UPS 구현/RT 부팅 확인 후 현재 PC 실측·TC-011 모의/TC-012 실행 |
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·실제 앱/장비 수명 수락은 미검증 | TASK-004~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬/native CI 확인(EV-001), 실제 DB·장비 호환 잔여 | TASK-019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증, TASK-018은 별도 범위 승인 후 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·004·005 | PC 환경 확정·프로필/후속 TC 실행 잔여, TASK-004~016 native 공개 후(T3) 이월·paste 경고. GAP-003 Closed | [검증 근거](../05_verification/verification_evidence.md) 참조 |
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

- 2026-11-01 #002: TASK-016 EV-030/source542c6e6·source102개·검사10개·Miri496·REV-027 Closed·기존 위임 로컬 수락. 다음 TASK-017 Planned/미착수 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_002.md)·[근거](../05_verification/verification_evidence.md#ev-030).


- 2026-10-31 #014: REV-025 거절 범주·압력/시도 부하·host 끝점 정렬·poison 복구 Red→Green, EV-029/eaf736c·검사14개/Miri224·source101개 blob일치 위임 수락/Closed, 2026-11-01 Human 요청으로 종료·TASK-016 Planned — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_014.md).

- 2026-10-31 #013: TASK-014/015(`b3c08bd..a247d01`) 독립 Read-Only 리뷰. fmt/clippy/workspace test Pass. REV-025 Open (P2 1·P3 2), 제품 코드 미수정, Human 요청으로 종료 — [리뷰](../05_verification/reviews/REV-025_TASK-014-015_INDEPENDENT_REVIEW.md)·[로그](../members/lim8603/workspace/session_logs/session_2026-10-31_013.md).
- 2026-10-31 #012: TASK-015 리포트 TDD14·실제 window 연계·할당0/공개 consumer·EV-028 source100개/검사13개·Miri168·소스a247d01 로컬 수락·REV-024 Closed·Human 요청으로 세션 종료 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_012.md).

이전 완료 이력과 게시/승인 상세는 [상태 아카이브](state_archive.md)·관련 Task/EV·세션 로그를 따른다.

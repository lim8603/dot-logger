# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 수집·분배·저장/복구·조회/재생·gRPC/ACK 로컬 검증, 클라이언트·운영/worker 후속 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — REV-022 EV-026·TASK-014 EV-027 로컬 수락, MS-005 진행 |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-005 In Progress — 운영 계측, MS-004/007은 native·실앱/실측 후속 |
| 활성 Task | TASK-014 In Progress — EV-027 로컬 수락/비용·실측/native 잔여 / 다음 후보 TASK-015 |
| 미처리 리뷰 | 없음 — REV-001~023 Closed. REV-022 EV-026·REV-023 EV-027 로컬 수락, RV-049 실측/TASK-022·RV-053 장기 tail/화면 TASK-020 후속 |
| 상태 | Yellow — 클라이언트/계측 로컬 수락, 운영 backend/실앱·비용/PC/RT/UPS·native 후속 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-10-31 #011 / AI |
| 세션 상태 | 요청 범위 완료 — REV-022 조치·TASK-014 로컬 수락/게시, TASK-015 미착수 |
| 최근 세션 | [2026-10-31 #011](../members/lim8603/workspace/session_logs/session_2026-10-31_011.md) — REV-022 조치·TASK-014 |

## 현재 결정과 제약

- **2026-10-31 Human 승인·권한 위임:** TASK-012 기존 로컬 구현 수락 및 REV-019 개선·검증 후 마감 → TASK-013 착수 승인. 앞으로 승인된 목표의 구현·검증·리뷰 마감/작업 전환 판단과 작업 단위별 커밋·푸시는 AI가 수행·보고한다. 같은 범위 재승인은 받지 않는다. [위임 기준](../01_cowork_protocol/decision_authority_matrix.md#프로젝트-오너의-권한-위임--2026-10-31). 미실행 검증·native 공개 후 조건은 유지한다.

- **TASK-011 현재 계약:** frozen Snapshot을 소유하는 mono ns 동기식 replay·최대1건 prefetch·양수 u32 유리수 배속·pause/resume/seek/stop·checked generation과 누적 읽기/원본 수명을 사용한다. 늦은 소비자의 재생 시계를 늦추고 원격 in-flight 취소를 추정하지 않는다. End는 마지막 blob read를 위해 snapshot을 유지하고 stop/Drop·terminal 오류에서 실제 reader close 뒤 반환한다. 상세 [TASK-011](../04_implementation/tasks/TASK-011.md)·[API §14.2](../03_design_artifacts/interface_contract.md#142-task-011-로컬-재생-api--2026-10-30).

- **TASK-010 현재 계약:** 고정 session/revision/원래 등록·독립 read-only source·완전 chunk safe offset, 유한 external/index/page/blob/read 예산을 사용한다. reader-owned permit은 active→closed 회전을 따라 보호하며 실제 reader Drop 뒤에만 회수 가능하다. 저수준 pin 분실/forget의 강제 회수·clock 변환은 도입하지 않는다. 현재 closed도 bounded scan으로 index를 재구성하며 summary scan 생략 최적화·자동 worker/CLI·실제 성능/OS 검증은 후속이며 local replay/원격 host 연결은 EV-018/022 범위다. 상세 [TASK-010](../04_implementation/tasks/TASK-010.md)·[API §14.1](../03_design_artifacts/interface_contract.md#141-task-010-로컬-api--2026-10-30).

- **TASK-009 현재 계약:** 불변 read-only 원본·원래 session/revision/생산자 수/동일 순서의 채널 등록·독립된 빈 출력이 필요하다. 완전 검증 chunk prefix만 복구하며 손상 뒤 재동기화·과거 completion 응답 복원·실패 파일 자동 삭제/재시도는 하지 않는다. 상세: [TASK-009](../04_implementation/tasks/TASK-009.md)·[API §13.5](../03_design_artifacts/interface_contract.md#135-task-009-중단-복구-api--2026-10-30).

- **2026-10-31 Human 구현 승인:** 「승인, 다음작업 진행하자」로 TASK-008 EV-012/013·TASK-010 EV-016/017·TASK-011 EV-020을 수락했다. REV-009/010/013/014/017 및 해당 RV Closed, TASK-008/010/011 Human 리뷰 완료. 기존 리뷰는 닫았으며 신규 TASK-012 리뷰는 별도다. 낮은 pin 유실/forget 강제 회수 없음·실제 reader close 뒤 반환 계약과 실측/native 이월은 유지한다.

- **TASK-012 계약:** remote 기본 disabled/loopback/read-only·외부 TLS/인증·bounded 구독/256KiB·handoff ACK/Gap/Unknown·세대 gate·host inbox. EV-023 소스4f79119는 이전 기준, server 오류 조치·구독 경계는 [EV-026](../05_verification/verification_evidence.md#ev-026)/b3c08bd·API§14.5를 따른다. 포화10ms 냉각 유지·실측은 TASK-022. native/실앱·LAN/RSS/보안 수락은 후속이며 OS keyring·CI/계정/과금 조회/차단 재시도 없음을 유지한다.

- **TASK-013 기능 기준:** EV-025/source29e55da 뒤 [EV-026](../05_verification/verification_evidence.md#ev-026)/b3c08bd로 handoff·진단/CLI·유휴 next를 보강했다(검사14개/Miri32). local wait timeout은 같은 구독 유지, server lease/명시 cancel은 종료한다. ttl은 record/idle lease 겸용, 별도 구독 간 Gap 연속성은 미보장이고 TASK-020 화면에서 불연속 표시한다. 현재 workspace는 TASK-014 EV-027/d99ba3b로 전체 회귀했다. CLI runtime metrics Unavailable·운영 backend Unsupported·실앱/native/성능 후속은 유지한다.

- **TASK-014 현재 계약/형상:** [EV-027](../05_verification/verification_evidence.md#ev-027) source97개/검사14개·Miri56·소스d99ba3b commit blob일치. 명시 enable/고정 producer counter·301개 이력/관리 sample·10/60/300초 구간·reset epoch·대상별 completion 합계·control 예산을 로컬 수락했다. 동시값은 근접 시점 혼합이며 배출 후 정확 대조한다. sample Mutex는 관리 경로에만 있다. overhead fixture 준비/128 debug smoke는 비용≤5%·RT/PC 성능 Pass가 아니며 TASK-022/023 후속이다. 자동 worker/CLI metrics backend·분모/예측/설정·비교는 후속 Task다.

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
| TASK-014 | 운영 계측·유한 집계 | AI | In Progress | 2026-10-31 | EV-027 로컬 수락/REV-023 Closed; 비용≤5%·PC/RT/native 후속, 다음 후보 TASK-015 |
| TASK-022 | 실제 장비·플랫폼 검증 프로필 | AI | In Progress | 2026-10-31 | local-pc-v1 수치 Approved·선행 harness 준비, RT 행 부팅/capability 확인 후 |

## 코드리뷰 현황 (Reviews)

| ID | 대상 | 일자 | 성격/판정 | 상태 | 요약 및 다음 액션 |
|---|---|---|---|---|---|
| [REV-001](../05_verification/reviews/REV-001_TASK-002.md) | TASK-002 | 2026-10-27 | 구성·파이프라인 구현 / Pass | Closed | EV-001 로컬/native CI 7개 Pass, Human 승인 완료 |
| [REV-002](../05_verification/reviews/REV-002_TASK-003.md) | TASK-003 | 2026-10-27 | 소형 수집·중요 예약 / Pass | Closed | EV-002 RT 무할당/무대기, native CI 7개 Pass, Human 승인 완료 |
| [REV-003](../05_verification/reviews/REV-003_TASK-004.md) | TASK-004 | 2026-10-27 | 원본 풀·수명·안전성 / Pass | Closed | EV-003 generation/state, Miri 8 seeds Pass, Human 승인 (native 대기) |
| [REV-004](../05_verification/reviews/REV-004_TASK-005.md) | TASK-005 | 2026-10-28 | 분배·출력 격리·예산 / Pass | Closed | EV-004 공정 순회/가드 공유, Miri 19×8 Pass, Human 승인 (native 대기) |
| [REV-005](../05_verification/reviews/REV-005_TASK-006_RV-001-004.md) | TASK-006 | 2026-10-28 | 설계 대조 자가리뷰 / 보류 (P1 3, P2 1) | Closed | RV-001~004 EV-007 수정·로컬 Pass, commit `d57c2e3`, Human 승인 완료 |
| [REV-006](../05_verification/reviews/REV-006_TASK-006_DEEP_REVIEW.md) | TASK-006 | 2026-10-28 | 제미나이 심층 코드리뷰 / Note/Suggestion | Closed | False Sharing·O(N) 선형탐색 등 5건 분석, rustdoc 보강/장비 과제 연결 |
| [REV-007](../05_verification/reviews/REV-007_TASK-006_RV-005.md) | TASK-006 | 2026-10-29 | 종료 경합 자가리뷰 / Conditional (P1 1) | Closed | RV-005 EV-009 수정·로컬 Pass, Human 승인 완료 |
| [REV-008](../05_verification/reviews/REV-008_TASK-007_RV-006-010.md) | TASK-007 | 2026-10-29 | MCAP 저장 구현 리뷰 / Conditional (P2 3, P3 2) | Closed | RV-006~010 EV-011 로컬 수정 Pass, commit `71b3e2c`, 2026-10-30 Human 승인 완료 |
| [REV-009](../05_verification/reviews/REV-009_TASK-008.md) | TASK-008 | 2026-10-30 | 회전·보존 자가 리뷰 / Conditional (P2 1, 로컬 해결) | Closed | RV-011 EV-012 수정/회귀 Pass, 2026-10-31 Human 승인·실측/native 후속 |
| [REV-010](../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md) | TASK-008 | 2026-10-30 | 회전·보존 심층 코드리뷰 / Conditional (P2 1, P3 2) | Closed | EV-013 로컬 Pass, RV-012 위험/후속 TASK-010 유지·2026-10-31 Human 승인 |
| [REV-011](../05_verification/reviews/REV-011_TASK-009.md) | TASK-009 | 2026-10-30 | 복구·IO 장애 자가 리뷰 / Conditional (P2 1, 로컬 해결) | Closed | RV-015 EV-014 수정·회귀 Pass, 2026-10-30 Human 승인·native 후속 |
| [REV-012](../05_verification/reviews/REV-012_TASK-009_DEEP_REVIEW.md) | TASK-009 | 2026-10-30 | 심층 코드리뷰 / Pass (P3 2) | Closed | RV-016/017 EV-015 로컬 Pass·소스fe03468·2026-10-30 Human 승인 |
| [REV-013](../05_verification/reviews/REV-013_TASK-010.md) | TASK-010 | 2026-10-30 | 조회/읽기 수명 자가 리뷰 / Conditional (P2 3) | Closed | RV-018~020 조치·EV-016 로컬 Pass, 2026-10-31 Human 승인·native/실측 후속 |
| [REV-014](../05_verification/reviews/REV-014_TASK-010_CODE_REVIEW.md) | TASK-010 | 2026-10-30 | 심층 코드리뷰 / Pass (P2 1, P3 2) | Closed | RV-021~023 정렬 캐시·rustdoc 조치·EV-017 로컬 Pass, 2026-10-31 Human 승인·native/실측 후속 |
| [REV-015](../05_verification/reviews/REV-015_TASK-011.md) | TASK-011 | 2026-10-30 | 재생/시간·자원 수명 자가 리뷰 / Conditional (P2 2, 로컬 해결) | Closed | RV-024/025 조치·EV-018/019 로컬 Pass·2026-10-31 Human 승인, native·장비 잔여 |
| [REV-016](../05_verification/reviews/REV-016_TASK-011_CODE_REVIEW.md) | TASK-011 | 2026-10-30 | 로컬 재생·시간 제어 심층 코드리뷰 / Pass (P2 1, P3 2) | Closed | 2026-10-31 RV-026 현 불변식상 overflow 불가·방어 정리, RV-027/028 진단 API 조치·EV-019 Pass·2026-10-31 Human 승인·native/장비 잔여 |
| [REV-017](../05_verification/reviews/REV-017_TASK-011_FINAL_REVIEW.md) | TASK-011 | 2026-10-31 | 로컬 재생 구현 조치·안전성 최종 리뷰 / Pass (P2 1, P3 2) | Closed | RV-029 국소 경고 억제/RV-030 비율 비교/RV-031 오류 진단 조치·EV-020 Pass·2026-10-31 Human 추가 조치 승인 |
| [REV-018](../05_verification/reviews/REV-018_TASK-012.md) | TASK-012 | 2026-10-31 | 원격 adapter 자가 리뷰 / Conditional (P2 4건 로컬 조치) | Closed | RV-032~035 조치·EV-022 Pass·2026-10-31 Human 로컬 구현 수락, native/실측 후속 |
| [REV-019](../05_verification/reviews/REV-019_TASK-012_PROJECT_CODE_REVIEW.md) | TASK-012 | 2026-10-31 | 원격 adapter·프로젝트 종합 심층 코드리뷰 / Pass (P2 2, P3 1) | Closed | RV-036~038 조치·EV-023 검사9개/Miri25 Pass·Human 위임에 따른 AI 마감 |
| [REV-020](../05_verification/reviews/REV-020_TASK-013_RECEIVER_START.md) | TASK-013 첫 단위 | 2026-10-31 | 연결 수명 API 개발 자가 점검 / Pass | Closed | EV-024 통합3개·검사12개·Miri24 Pass·위임된 첫 단위 수락, transport/CLI 후속 |
| [REV-021](../05_verification/reviews/REV-021_TASK-013_CLIENT_CLI.md) | TASK-013 로컬 구현 | 2026-10-31 | client/CLI 개발 자가 점검·조치 / Pass | Closed | RV-039~045 조치·EV-025 검사15개/Miri32·위임된 AI 로컬 수락, native/실앱 후속 |
| [REV-022](../05_verification/reviews/REV-022_TASK-013_INDEPENDENT_REVIEW.md) | TASK-013 `29e55da`·TASK-012 `4f79119` | 2026-10-31 | 독립 Read-Only 리뷰·서버 경계 확장 / Pass (P2 2, P3 8) | Closed | EV-026 검사14개/Miri32·위임 수락. RV-049 TASK-022 실측·RV-053 TASK-020 장기 tail/화면 후속, wire 확장 없음 |
| [REV-023](../05_verification/reviews/REV-023_TASK-014_TELEMETRY.md) | TASK-014 d99ba3b | 2026-10-31 | 개발 자가 점검 / 로컬 Pass | Closed | EV-027 검사14개/Miri56·위임 수락; 비용/RT/PC/native/실앱 후속 |

## 다음 시작점

1. 이번 승인 범위의 REV-022 개선/회귀·TASK-014 로컬 구현/검증/수락을 완료했다. REV-022/023 Closed, EV-026 sourceb3c08bd·EV-027 sourced99ba3b 근거를 따른다. 같은 범위 재승인은 불필요하다.
2. 다음 개발 후보 TASK-015(여유·병목·보존 시간 리포트)는 Planned/미착수다. TASK-014 비용≤5%/계측 포함 RT·PC·native/실앱은 미완료이며 TASK-022/023과 연결한다. 상위 MS-005·TC-009·Gate4/5 전체 완료가 아니다.
3. RV-049 최적화/포화 회복 실측은 TASK-022, RV-053 서버 lease 초과 유휴/재구독 연속성·화면 불연속은 TASK-020 후속이다. 현재 wire 유지로 수락했고 heartbeat/lease 확장이 필요하면 H/J 판단한다. 새 프로토콜 승인을 요구하며 현재 작업을 중단한 상태는 아니다.
4. native는 공개 완료 후(T3), PC/RT/UPS/실앱 및 Gate4/5는 후속이다. 새 T1/T2/T3 도래는 확인되지 않았다.

## 승인·권한 위임 상태 — 2026-10-31 #007

- Human의 기존 모든 권한/게시 위임과 #008 TASK-013 진행 지시를 유지한다. EV-025 검증 뒤 AI가 로컬 구현을 수락하고 REV-021을 마감했다. 같은 범위의 최종 승인 요청은 반복하지 않는다.
- 구현/검증 결과와 위험·미완료를 보고하며, 미실행 native/PC 시험이나 제품 전체 수락을 임의로 Pass로 바꾸지 않는다. 공개 전환/선택 기능 자동 착수 지시로 확대하지 않는다.

## 새 세션 재개 메모

- 저장 문제 원인 정정(2026-10-31 #006): Human이 이전 파일 저장 문제는 Antigravity 문제였다고 확인했다. #005의 VS Code pending edits 원인 확정은 당시 진단 이력으로 보존하며 현재 원인 판단으로 사용하지 않는다. 당시 설정 변경의 효과는 이번에 재검증하지 않았고 사용자 에디터 설정도 변경하지 않았다.
- REV-019 EV-023 조치·검증 후 Closed. 포화 대기/고정 credential 비교·receiver 지침 보강을 수락했으며 전체 인증 시간 특성은 미검증이다. gate 자동 삭제 미도입. TASK-013은 같은 수명 계약을 소비한다.
- TASK-022 수치 Approved·EV-021(프로필 준비/승인)과 EV-022(TASK-012 제품 로컬 Pass)을 구분한다. TASK-008/010/011/012 로컬 리뷰·TASK-022 수치 승인은 완료됐다. TASK-013 개발/후속 검증은 별도다.
- 개발1.96.0·MSRV1.85·nightly-2026-10-26 Miri 고정, dependency/lock·RT 무할당/무대기·private unsafe 경계를 유지한다. 전역 rustup default는 바꾸지 않았다.
- 완료/종료·writer 배치/재시도 의미는 API§13·TASK-006~008을 따른다. 낮은 ReadPin 유실/forget·종료 불명 background IO는 강제 회수하지 않고 reader-owned permit만 실제 reader 종료 후 반환한다.
- 자동 startup 발견/기존 namespace 재개·운영 worker/StorageDegraded·실앱 연동은 후속이다. remote/client/CLI는 EV-025 로컬 범위이며 자동 운영 worker를 시작하지 않는다. 실제 성능·RT·UPS·24h·다른 OS/native·TC 전체·MS-002/003/004·Gate4/5 완료를 선언하지 않는다. decoder/runtime/source/sink·host 메모리와 OS cache는 logger 예약 밖이다.

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
| T3 | 이월 native 검증 실행·결과 반영 | GitHub 저장소 공개 완료가 확인된 때 | GAP-004 / ADR-011 / TASK-004~014 | 2026-10-31 Human 검증 시점 결정 |

범위 밖 백로그 B: 없음. 현재 T1/T2/T3는 미도래. 공개 후 native 실행 시점 지정은 공개 전환 자체 요청이 아니므로 T2를 지금 발동하지 않는다. OQ를 T/B에 중복 등록하지 않는다.

## 열린 결정과 검증 공백

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| OQ-002 | ADR-011로 현재 PC 실측·UPS 시뮬레이션 수락 확정. local-pc-v1 수치 Approved(EV-021), 실행 준비/실측 잔여 | 선행 harness·실앱/UPS 구현/RT 부팅 확인 후 현재 PC 실측·TC-011 모의/TC-012 실행 |
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·실제 앱/장비 수명 수락은 미검증 | TASK-004~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬/native CI 확인(EV-001), 실제 DB·장비 호환 잔여 | TASK-019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증, TASK-018은 별도 범위 승인 후 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·004·005 | PC 환경 확정·프로필/후속 TC 실행 잔여, TASK-004~014 native 공개 후(T3) 이월·paste 경고. GAP-003 Closed | [검증 근거](../05_verification/verification_evidence.md) 참조 |
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

- 2026-10-31 #011: REV-022 EV-026 조치/수락·TASK-014 EV-027 로컬 구현/수락·단위별 게시 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_011.md).
- 2026-10-31 #010: 현황 파악·REV-022 반입·승인 경계 보고 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_010.md).
- 2026-10-31 #009: 독립 리뷰 REV-022 Open 등록 — [리뷰](../05_verification/reviews/REV-022_TASK-013_INDEPENDENT_REVIEW.md).

이전 완료 이력과 게시/승인 상세는 [상태 아카이브](state_archive.md)·관련 Task/EV·세션 로그를 따른다.

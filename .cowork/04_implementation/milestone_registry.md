# Milestone 목록과 승인된 실행 계획

- 상위 Intent: INT-001(Approved). 최신 완료 기준(2026-11-13, EV-078)에 따라 MS-001~006·008·009 Done(MS-009는 2026-11-21 마감), MS-007 In Progress. MS-008은 TASK-034 E 수락으로 마감했다(EV-088). MS-007 일정 판정은 EV-089를 따른다. TASK-026은 MS-007의 마지막 단계·일정 판단 밖이며 native·장기 검증은 TASK-024가 소유한다.
- 1인 순차 진행 기준의 상대 일정이다. 시작일·가용 시간·검증 장비가 미정이므로 달력 날짜/납기를 확정하지 않는다.
- 인일은 설계 추정치이며 TASK-004 수명·MCAP 복구·실제 장비 검증 후 재산정한다. 소요 범위에 리뷰/장비 대기·실제 24시간 경과 대기는 별도다.

| MS | 제목 | Phase | 상태 | Task | 종료 결과 | 인일 초안 | 상세 |
|---|---|---|---|---|---|---|---|
| MS-001 | 상세설계 합의 | Design | Done | TASK-001 | 검토 가능한 상세설계·추적표·구현 계획 확정 | 1~2 | [상세](milestones/MS-001.md) |
| MS-002 | 제한된 수집 코어 | Build | Done | TASK-002·TASK-003·TASK-004·TASK-005·TASK-006 | RT 수집·원본 수명·fanout·완료를 대역으로 증명 | 8~14 | [상세](milestones/MS-002.md) |
| MS-003 | 파일 영속성과 보존 | Build | Done | TASK-007·TASK-008·TASK-009 | 다중 파일 기록·회전·이벤트·복구 완성 | 6~9 | [상세](milestones/MS-003.md) |
| MS-004 | 조회·재생·클라이언트 | Build | Done | TASK-010·TASK-011·TASK-012·TASK-013 | Rust API/CLI로 실제 기록 활용 | 7~12 | [상세](milestones/MS-004.md) |
| MS-005 | 운영 관측·조절·간편 점검 | Build | Done | TASK-014·TASK-015·TASK-016·TASK-017·TASK-018 | 한계 확인→개발자 조절→전후 비교 완성 | 7~11 (+선택 1~2) | [상세](milestones/MS-005.md) |
| MS-006 | 외부 연동과 긴급 기록 | Build | Done | TASK-019·TASK-020·TASK-021 | DB·dot-probe 통합 예제·UPS 소프트웨어 경로 완성 | 8~14 | [상세](milestones/MS-006.md) |
| MS-007 | 장비 검증과 인수 | Verify | In Progress | TASK-022·TASK-023·TASK-024·TASK-025·TASK-026·TASK-033·TASK-038·TASK-039·TASK-040 | 현재 PC 프로필 준비·실측/24h·UPS 모의 수락·공개 후 OS 교환 | 8~14 | [상세](milestones/MS-007.md) |
| MS-008 | 소비처 통합 접점 경량화 | Build | Done | TASK-027·TASK-028·TASK-029·TASK-034 | facade 기본 worker·preset/auto preset·support 없는 레시피 | 8~13 | [상세](milestones/MS-008.md) |
| MS-009 | 표준 호환과 원격 시각화 | Build | Done(2026-11-21) | TASK-030·TASK-031·TASK-032·TASK-035 | 표준 MCAP 판독·분리된 viewer 시각화·운영 대시보드 | 8~13 | [상세](milestones/MS-009.md) |
| MS-010 | 릴리즈와 배포 | Verify→Deliver | In Progress(0·1단계 Done) | TASK-036·TASK-037·TASK-041 (2~4단계 Task는 착수 때 등록) | 외부 소비자 빌드·v0.1.0 미리보기·crates.io(TASK-026 뒤)·1.0 직전 재검증(24h·멀티 OS CI·게이트, T8)·1.0(Gate 5) | 미정 | [상세](milestones/MS-010.md) |
| MS-011 | RT 로거 비교 근거와 채택 편의 | Verify→Build | In Progress(2026-11-23 등록, TASK-042 Done 2026-11-26) | TASK-042·TASK-043·TASK-044 | `try_log` 지연 분포 공개, RT 미룬 서식 로그·`Record` derive/prost(0.2 후보, 기능 논의에서 채택) | 7.5~13.5 | [상세](milestones/MS-011.md) |
| MS-012 | InfluxDB 연동 재검증과 사용 안내 | Verify | **Done**(2026-11-27, EV-132~135) | TASK-045·TASK-046·TASK-047 | v2 실DB 재검증·`Writer` 예제·v3 Core 호환 범위·장시간 DB 단절(F4-DB). REV-073 조치 선행, v0.1.1 전 | 2.5~5+실행 | [상세](milestones/MS-012.md) |
| MS-013 | 출력 구성 선택과 네트워크 전용 모드 | Design→Build | **Draft r6**(2026-11-27, REV-074~078 반영, REV-079 재리뷰 대기 — 지적 0건까지 반복) | 승인 뒤 채번(단계 0~6) | 파일·네트워크·DB 출력 조합, 파일 없는 네트워크 전용 시작, 원격 손실 집계, 코드+옵션 파일 구성, 전송 스위치, 최대 처리량 공개. 0.2 | 20~25 + 단계 6 미정 | [상세](milestones/MS-013.md) |

필수 합계 **61~102 인일**(주 5일 전담 기준 약 12.2~20.4주; 2026-11-04 MS-008/009 16~26 인일 추가), FR-015 선택 작업 1~2 인일 별도. 가용 시간이 적거나 장비/리뷰 대기가 있으면 달력 기간은 늘어난다. 납기 약속이 아니다.

첫 구현 진입점은 **TASK-002 → TASK-003 → TASK-004**다. 원본 수명/메모리 안전성과 작은 종단 기록을 먼저 검증한다. 계측 TASK-014는 코어 이후 조기 구현할 수 있고, 장비 프로필 TASK-022는 설계 합의 직후 수집 가능하다. 각 의존의 단일 기준은 task_registry다.

Must 종료 전에 FR-018 소프트웨어 옵션과 실제 장비 검증을 모두 추적한다. 장비 미확정 상태를 완료로 체크하지 않는다. dot-probe 첫 적용 완료가 확인되면 project_state T1(ROS 2 검토)을 알린다.

2026-10-28 TASK-006: 고정 완료 window·유한 query/history·명시 sync barrier·deadline shutdown coordinator를 구현하고 로컬 검증했다. 실제 파일/worker/장비 보장은 후속이며 MS-002는 native 검증 잔여로 In Progress다.

후속 자가 리뷰 EV-006의 P1 3건과 README 불일치는 EV-007에서 수정·로컬 회귀 Pass했다. 수정분 Human 리뷰와 native/실장비 검증은 남아 MS-002 In Progress를 유지한다.

2026-10-29 후속 리뷰 [EV-008](../05_verification/verification_evidence.md#ev-008)의 RV-005 P1은 Human 수정 지시 후 [EV-009](../05_verification/verification_evidence.md#ev-009)에서 진입/게시·종료 순서화로 수정했다. 신규 private3개·두 Rust 각16 features/consumer/예제·Miri81×8·RT0/0 Pass, GAP-007 로컬 Closed다. 최종 수정분 Human 리뷰/native·실장비는 잔여여서 MS-002 In Progress를 유지한다.

2026-10-29 Human은 TASK-006 최신 수정분과 native 이월 상태의 TASK-007 진행을 승인했다. TASK-007 MCAP 봉투/writer/인덱스·실제 파일 sync는 [EV-010](../05_verification/verification_evidence.md#ev-010) 로컬 Pass 이후 RV-006~010을 [EV-011](../05_verification/verification_evidence.md#ev-011)에서 수정·회귀했고, 최신 수정분 Human 리뷰/native 대기다. MS-003 In Progress, 회전/복구·장비 검증은 잔여다.

2026-10-30 Human은 TASK-007 최종 구현·REV-008 Closed와 native 이월 상태의 TASK-008 착수를 승인했다. TASK-008 회전/보존/이벤트 보호는 EV-012 로컬 Pass(통합15·두 Rust16 features/lint/doc/fmt·allocator8·Miri776), REV-009 Resolved·Human 최종 구현 리뷰/native 대기다. MS-003 In Progress, TASK-009 복구는 미착수다.

2026-10-30 후속: Human 「커밋 푸쉬 후, 다음 테스크 진행」으로 TASK-008 조치를 게시하고 TASK-009를 구현했다. EV-014 복구19개·두 Rust16조합/consumer/예제/lint/doc/fmt·allocator9·Miri936 로컬 Pass, REV-011 Resolved다. Human 최종 구현 리뷰/native·장비 잔여로 MS-003 In Progress를 유지한다. 자동 startup 발견/재개·worker와 실물 장애 수락은 미완료다.

2026-10-30 TASK-009 최신 구현 승인·REV-011/012 Closed, 소스5c60399/문서74fc9ba 게시 후 TASK-010을 구현했다. EV-016 query·reader-owned pin 수명·두 Rust16조합·allocator10/lint/doc/fmt·Miri1024 로컬 Pass, REV-013 Resolved·Human 리뷰/native·장비 잔여다. MS-004 In Progress이며 재생/remote/CLI·실장비 성능은 후속이다.

2026-10-30 TASK-010 소스1972649/문서f92dd17 게시 후 승인된 TASK-011을 구현했다. EV-018 두 Rust16features·실제 파일 replay consumer·allocator10/lint/doc/fmt·영향 Miri304 로컬 Pass, REV-015 Resolved·Human 리뷰/native·장비 잔여로 MS-004 In Progress를 유지한다.

2026-10-31 TASK-011 REV-016 반입·RV-026 방어적 정리/RV-027~028 진단 API 조치, EV-019 두 Rust all-features·실제 파일 consumer/allocator/lint/doc/fmt·replay Miri96 로컬 Pass다. REV-015/016 Resolved·Human 리뷰/native·장비 대기로 MS-004 In Progress를 유지한다. remote/CLI는 후속이며 TASK-012는 자동 착수하지 않는다.

2026-10-31 구현 계약 확인 후 Human 「승인」으로 TASK-011 최종 구현 승인·REV-015/016 Closed·Human 리뷰 체크 완료를 반영했다. TASK-008/010 Human 리뷰와 native GAP-004·장비/실제 timer·remote/CLI는 잔여이며 TASK-011/MS-004 In Progress를 유지한다. TASK-012는 자동 착수하지 않는다.

2026-10-31 ADR-011: 현재 PC 실측/UPS 시뮬레이션 수락·native 공개 완료 후 이월 승인. TASK-022 환경/수락 프로필 준비로 MS-007 In Progress이며 Phase Build는 유지하고 종합 시험/Verify 진입/Gate4·5 Pass를 선언하지 않는다.

2026-10-31 TASK-013 EV-025: Rust native client/CLI·오프라인·취소/ACK·화면 API 계약 로컬 구현/리뷰 수락 완료(소스6863268, 검사15개·Miri32). MS-004의 로컬 조회·재생·원격·클라이언트 근거를 연결했으며 native/다른 OS·실앱/성능 후속 때문에 MS-004 In Progress·Gate4/5 미완료를 유지한다. 다음 개발 후보 TASK-014는 이번 요청 범위에서 착수하지 않았다.

2026-10-31 TASK-014 EV-027: 고정 계측·유한 이력/대상 합계·fixture 로컬 구현/검증·REV-023 위임 수락. MS-005 In Progress, 비용/RT/실앱/native·TASK-015~017은 미완료이며 다음 후보 TASK-015는 미착수다.

2026-10-31 #012 TASK-015 로컬 수락: EV-028 source100개/검사13개·Miri168·소스95a63a6, REV-024 Closed. Observed/Validated/Estimated/Stale·손실 안정 큐 Overloaded·분리된 bytes/slots/출력정지/filesystem/quota/순환 보존 산술과 합성 오차 시나리오를 검증했다. TASK-016/017·실제 PC/RT 오차/비용·native·실앱이 남아 MS-005 In Progress/TC-009 전체 미완료다.

2026-10-31 #014 REV-025 조치: [EV-029](../05_verification/verification_evidence.md#ev-029) sourceeaf736c·검사14개/Miri224·source101개 blob일치로 거절 범주/압력·시도 부하·host 출력 끝점 정렬·poison 복구를 로컬 수락, REV-025 Closed. TASK-016/017·실측 비용/오차·native/실앱은 남아 MS-005 In Progress/TC-009·Gate4/5 전체 미완료를 유지한다.

2026-11-01 #002 TASK-016 로컬 수락: EV-030/source542c6e6·source102개·검사10개·Miri496·REV-027 Closed. 명시 validate/apply·posted/per-producer ack·old plan Busy·filter/Normal sampling/기존 route·변경 전후의 관측/조건 차이를 연결했다. source102개 commit blob 일치. 운영 backend/CLI runtime·실앱/실제 PC/RT 비용/오차·native와 TASK-017이 남아 MS-005 In Progress·TC-009 전체 Partial·Gate4/5 Not Ready를 유지한다. 다음 TASK-017은 Planned/미착수다.

2026-11-01 #004: TASK-016 REV-028 RV-063/064 조치/Closed 뒤 TASK-017 S1 무부하 계획·시간/물리 쓰기/reserve/취소 예산 판정을 착수·로컬 수락했다. [EV-031](../05_verification/verification_evidence.md#ev-031)·REV-029 Closed·소스75c6096. 다음 S2 실제 run·배출/sync/정리·결과 분류 및 CLI/runtime/실앱/실측/native가 남아 Task/MS·TC-010 전체 Partial·Gate4/5 Not Ready를 유지한다.

2026-11-01 #005: TASK-017 S2 합성 파일 실제 실행·시간/쓰기/reserve/취소·footer/sync/close/소유 namespace 정리와 결과 분류를 [EV-032](../05_verification/verification_evidence.md#ev-032)·소스5d3349b·REV-030 Closed로 로컬 수락했다. workspace347·MSRV 관련40·Miri24 Pass. S3 Report/CLI/운영 host·실제 free probe/실앱/PC/RT/native 후속이 남아 Task/MS/TC 전체·Gate4/5 미완료를 유지한다.

2026-11-01 #006: TASK-017 S3 로컬 운영 host/PlanCheck·RunCheck/CLI·실제 Linux free probe·원 Report/profile 연결을 [EV-033](../05_verification/verification_evidence.md#ev-033)·REV-031 Closed로 수락했다. workspace357/core288·MSRV workspace357·Miri16·audit Pass. 생산 capture/routing/실제 수신자·dot-probe/실앱·PC/RT/UPS/장기/native가 남아 S3 전체·Task/MS/TC·Gate4/5 미완료 유지.

2026-11-01 #007: TASK-017 S3 실제 raw capture/routing·loopback TCP receiver 큐/ACK·dot-probe tick export snapshot을 [EV-034](../05_verification/verification_evidence.md#ev-034)·REV-032 Closed/소스b196f06로 첫 연결 수락했다. 신규7개·Rust1.96/MSRV workspace 각364 Pass. routed RunCheck 운영 backend·지속 실앱/화면·PC/RT/UPS/장기/native 후속으로 MS-005/Task/TC 전체·Gate4/5 미완료 유지.

2026-11-01 #010: REV-034 P3 조치 EV-036/Closed 뒤 TASK-019 S1 유한 InfluxDB v2 write를 EV-037/REV-035 Closed로 로컬 수락했다. MS-006 In Progress. 실제 DB/query·Delivery/파일 격리·tracing/실앱·UPS·실측/native·Gate4/5는 미완료다.

2026-11-01 #011: TASK-019 S2 유한 query와 OSS2.9.1 실제 DB 대조를 EV-038/REV-036 Closed로 로컬 수락. 다음 S3 Delivery/완료·파일 격리. MS-006 In Progress·실앱/UPS/실측/native·Gate4/5 미완료 유지.

2026-11-01 #012: TASK-019 S3 원 Delivery/완료·같은 session 예산·로컬 파일 영향 관측을 EV-039/REV-037 Closed로 수락. TASK-019 S1~S3 로컬 기능 완료, 통합/환경/native·MS-006/Gate4/5 미완료. 다음 후보 TASK-020, 선택 TASK-018은 별도 승인 대기다.

2026-11-01 #014: TASK-020 S1 일반 tracing/log 입력을 EV-040/REV-039로 로컬 수락했다. S2 파일·DB·구독/조절 비교 및 S3 화면 불연속 예제, TASK-021 UPS·TASK-026 실앱·PC/RT/native가 남아 MS-006 In Progress·Gate4/5 Not Ready를 유지한다. 선택 TASK-018은 별도 승인 대기다.

2026-11-01 #015: TASK-020 S2 합성 입력의 실제 파일/DB/구독·명시 설정 비교 EV-041/REV-040 수락. S3 화면 불연속·TASK-021 UPS·TASK-026 실제 지속 앱·PC/RT/native가 남아 MS-006 In Progress·Gate4/5 Not Ready를 유지한다.

2026-11-02 TASK-020 S1~S3 EV-040~042 로컬 수락·REV-039~041 Closed. 다음 구현은 TASK-021 UPS 소프트웨어 경로이며 TASK-026 실제 지속 앱/화면·PC/RT/UPS/정량 성능/native(T3)·전체 Gate4/5는 잔여라 MS-006 In Progress를 유지한다.

2026-11-02 TASK-021 S1~S3 로컬 구현·현재 PC ext4 UPS 시뮬레이션을 EV-043/REV-042로 수락했다. 실물 UPS 제공은 필수 조건이 아니다. 다음 구현은 TASK-026 실제 지속 dot-probe/화면, native 공개 후 T3·장기/RT/전체 Gate4/5는 잔여라 MS-006 In Progress 유지.

2026-11-04 #001: 사용성 평가(공개 항목 534개·기본 worker 없음·예산 명시 부담·Envelope 저장의 표준 도구 판독 불가)에 따라 Human이 MS-008 소비처 통합 접점 경량화(TASK-027~029)와 MS-009 표준 호환과 원격 시각화(TASK-030~032, TASK-032 포함)를 승인했다. 순서는 TASK-027 → TASK-028 S1 → TASK-023 S2/S3(facade 경로)·RT 요청 → TASK-028 S2 → TASK-029 → MS-009 → probe 인계 → TASK-026이다. FR-015 장비 측정 부분은 TASK-028 S2로 포함하고 질문 프리셋은 TASK-018에 남는다.

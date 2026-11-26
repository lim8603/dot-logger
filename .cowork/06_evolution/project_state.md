# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 범용 로보틱스/RTOS Rust 로그 라이브러리(첫 사용처 dot-probe) |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Verify — Gate 4 Pass(이 PC 로컬 범위, 2026-11-21 Human 수락). MS-001~006·008·009 Done, MS-007 In Progress |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-007 In Progress — 실사용처(TASK-026) 잔여(TASK-024·038·039 2026-11-24, TASK-040 2026-11-25 Done). [MS-011](../04_implementation/milestones/MS-011.md) RT 로거 비교 근거와 채택 편의 In Progress(2026-11-23 등록, TASK-042 2026-11-26 Done, 043·044 Planned). [MS-010](../04_implementation/milestones/MS-010.md) 릴리즈와 배포 In Progress — 0·1단계 Done(2026-11-22 [v0.1.0 미리보기](https://github.com/lim8603/dot-logger/releases/tag/v0.1.0)), 1a v0.1.1(TASK-041) Planned, 2단계부터 TASK-026 뒤이고 그 전에 추가 기능 논의(2026-11-23 Human). 3a 1.0.0 직전 재검증(24h·멀티 OS CI·게이트, T8) 2026-11-24 추가. [MS-012](../04_implementation/milestones/MS-012.md) InfluxDB 연동 재검증과 사용 안내 Planned(2026-11-26 설계·M1·M2 승인). MS-009 Done |
| 활성 Task | **진행 중: REV-073 반입·조치 → MS-012 [TASK-045](../04_implementation/tasks/TASK-045.md)(v2 실DB 재검증·`Writer` 예제)·[046](../04_implementation/tasks/TASK-046.md)(v3 Core 호환)·[047](../04_implementation/tasks/TASK-047.md)(장시간 DB 단절) → TASK-041**(2026-11-26 Human: 시험 3종·`Writer` 예제·v0.1.1 전·Docker) · [TASK-041](../04_implementation/tasks/TASK-041.md) v0.1.1 Planned(REV-073·MS-012 뒤, 태그는 Human 확인 뒤) · TASK-042 Done(2026-11-26, README 지연 표·[EV-129](../05_verification/verification_evidence.md#ev-129)) · TASK-041 Planned · TASK-043·044 Planned(0.2 후보, 기능 논의) · TASK-018 Cancelled(B2) |
| 미처리 리뷰 | **[REV-073](../05_verification/reviews/REV-073_HARNESS_AND_LATENCY_REVIEW.md) Open** — Conditional, P1 1건·P2 7건·P3 3건(RV-202~212). REV-001~072 Closed |
| 상태 | 39/47 Task Done(TASK-018 Cancelled), MS-009 Done. CI(3 OS × 2 Rust·exchange·audit·Miri) 최근 코드 커밋 모두 Pass(`2632b6c` run `45796420659` 19개 작업, 약 56분, 2026-11-26). 최신 회귀 [EV-115](../05_verification/verification_evidence.md#ev-115): 두 Rust 각 742 Pass·5 ignored, Clippy(Linux·macOS·Windows 교차)·외부 소비자·feature 조합·CI 범위 Miri 통과. T3 해소: `a0fcfcf`에서 CI 9개 작업 Pass, macOS ignore 없음([EV-116](../05_verification/verification_evidence.md#ev-116)), GAP-004 Closed. **Gate 4 Pass(로컬 범위)·Gate 5 Not Ready** |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-11-26 / #003 — InfluxDB 논의, MS-012·TASK-045~047 설계·승인, REV-073 반입 착수. 이전 세션은 [최근 작업·이력](#최근-작업이력) / AI |
| 세션 상태 | **마감 — 2026-11-26 #002(리뷰).** generic 커널 1회 부팅 중(다음 재부팅은 RT, governor `powersave` 원복 확인). REV-073 Open, CI `45796420659` 19/19 Pass. 다음 세션은 InfluxDB 연동 시험·샘플 코드 논의 |
| 최근 완료 세션 | [최근 작업·이력](#최근-작업이력) 참조 |

## 현재 결정과 제약

- **2026-11-23 Human — S2·MS-011 결정:** TASK-024 S2 D1=Docker 쓰기 제한(실행 중 `io.max` 전환에 권한 있는 보조 컨테이너 허용 「privileged 써도 돼」), D2=tc_008 갈음, D3=현재 동작 기록. 외부 관점 지적 3건(RT 수집 비용 숫자·RT 글줄 경로·`Record` derive)을 [MS-011](../04_implementation/milestones/MS-011.md) TASK-042~044로 등록, M1=하나로 묶음·M2=TASK-042는 v0.1.1 전, TASK-042 D1=`powersave`·`performance` 둘 다(전환은 sudo, 실행 때 Human)·D2=자체 숫자만. TASK-043·044 채택은 기능 논의.
- **2026-11-26 측정 환경 주의(EV-130):** 이 PC의 지연 측정은 AI 세션이 대기 중일 때(백그라운드 실행) 약 600 ms마다 약 0.5 ms 코어가 약 9배 느려지는 사건이 섞인다. 세션이 작업 중일 때는 없다. 지연 측정 조건에 실행 방식을 적고, 기록 0건 행의 응답 시간으로 사건 유무를 본다. 기전(SoC 펌웨어·전원 추정) 확인은 루트 실험이라 Human 판단.
- **2026-11-23 사고 — 하네스 채움이 호스트 루트에:** S2 하네스 첫 구현의 tmpfs 마운트 경로 오류로 F3 디버그 채움이 T500 루트에 들어가 디스크 경고, 이어 832 GB 파일로 루트 100%(즉시 삭제, `fallocate`라 데이터 쓰기 없음). 채우기 직전 같은 chroot 안에서 tmpfs·크기 확인을 넣었고 본 실행 동안 루트 여유 변화 없음(EV-123).

- **2026-11-21 Human 결정 — 버전·릴리즈 계획([MS-010](../04_implementation/milestones/MS-010.md)):** ① MCAP는 vendor를 별칭 crate로(ADR-010 개정 2) ② 라이브러리 의존성 `^` 범위(ADR-009 개정) ③ v0.1.0은 Gate 5 전 GitHub 미리보기(Gate 5 예외) ④ crates.io는 TASK-026 뒤 ⑤ SemVer·0.x 동일 버전·형식 버전 분리([릴리즈 노트 정책](../07_delivery/release_note.md#버전호환-정책)). 계획은 ADR이 아니라 MS-010에 기록하고(Human 「ADR은 아닌 것 같은데」), 기존 ADR 내용이 바뀌는 ①②만 개정 절로 남겼다. 계기: 워크스페이스 밖 소비자 프로젝트가 vendor 전용 API 때문에 컴파일되지 않음(`E0599`) → [TASK-036](../04_implementation/tasks/TASK-036.md).

- **2026-11-21 Human 「INT-002로 하고싶다」 — 후속 Intent 아이디어:** AI가 로그와 소스코드에 접근해 애플리케이션의 자기개선(로그 기반 오류 분석·코드 개선·필요한 로그 추가 제안)을 돕는 분석 도구를 INT-002 Draft로 등록했다. 목표·범위는 미정이고, INT-001 마무리 뒤 구체화한다. 논의 기록은 [INT-002](../02_project_definition/intents/INT-002_ai_assisted_log_analysis.md).

- **완료·문서화된 결정(아카이브 #034):** 2026-11-21 T3 native CI(EV-113~116, GAP-004 Closed)·공개 전환(T2)·Gate 4 수락(로컬 범위, Phase Verify)·`imported_context` 비공개(.gitignore).
- **완료·문서화된 결정(아카이브 #033):** #004 공개 준비(이력 유지·`imported_context`만 제거·`MIT OR Apache-2.0`·단일 저장소, 원본 번들 `~/backup/dot-logger-original-2026-11-21.bundle`), TASK-025 README 영어화, TASK-032·MS-009 마감, ADR-013~016과 그 개정(0 행 생략, `.logger` 예약 토픽), TASK-027·029·030·033·034 범위 결정, 2026-11-15 일정 목표·커버리지 하한. 현재 효력은 각 ADR·Task·[quality_gate](../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)를 따른다.
- **완료 기준(2026-11-13·15, quality_gate):** dot-logger는 범용 라이브러리이고 dot-probe는 첫 사용처다. 실사용처(TASK-026)·native·24h(TASK-024)는 마지막 단계이며 일정 판단에 넣지 않는다.

- **probe 연동 순서:** dot-logger 자체 작업·TASK-025 인계 뒤 별도 dot-probe 세션에서 TASK-026을 진행한다. 원격 성능 잔여는 T7, native/장기는 TASK-024다. RT 재부팅은 ADR-011의 사전 승인·once/back 절차를 유지하며 이 세션에서 dot-probe 저장소를 수정하지 않는다.

- **2026-10-31 Human 승인·권한 위임:** TASK-012 기존 로컬 구현 수락 및 REV-019 개선·검증 후 마감 → TASK-013 착수 승인. 앞으로 승인된 목표의 구현·검증·리뷰 마감/작업 전환 판단과 작업 단위별 커밋·푸시는 AI가 수행·보고한다. 같은 범위 재승인은 받지 않는다. [위임 기준](../01_cowork_protocol/decision_authority_matrix.md#프로젝트-오너의-권한-위임--2026-10-31). 미실행 검증·native 공개 후 조건은 유지한다.

- **TASK-009~022 현재 계약:** 각 Task 문서와 [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안)을 따른다. 미수락 잔여(native T3·실앱·PC/RT 성능·장기)는 각 Task와 GAP-002/004에 유지된다. 이 문서에 있던 요약 원문은 [상태 아카이브](state_archive.md) #023으로 옮겼다.

- **2026-10-26 Human 승인:** 「권고 기준선 채택 — 장비·호환 검증은 계획대로 진행」. 상세설계·ADR-008/009·추가 시험 목표·MS 계획을 채택했다. 검증 이월은 면제가 아니다.

- **ADR-010 Accepted / 2026-10-26 Human 승인:** MCAP 0.26.0 채택. 0.25 선택 zstd 경로의 보안 공지를 해소했다. Tokio는 1.51.6 patch. 최종 audit 취약점/unsound 0, `paste` 유지보수 경고 1(GAP-005).

- Gate 1·2 Pass, ADR-003~006 기준선 유지. 1 GB/s 원본+10만 소형/s·32생산자/16파일/128채널·R1 2 GiB 및 512 MiB/4 GiB 비교는 조건부 목표이며 제품 기본값·달성 결과가 아니다.
- **ADR-007 Accepted:** dot-probe 내장형 Ports and Adapters + 제한된 생산자/소비자 파이프라인. 별도 데몬은 첫 범위에 넣지 않는다. 프로세스 장애까지 격리하지 않음.
- **클라이언트 형태 확정:** Rust API + CLI, dot-probe 기존 화면 연동. 명령/통신 상세도 ADR-009 기준선 채택 완료.
- **실제 장치 대체 환경 확정 — ADR-011 Accepted / 2026-10-31:** Human이 현재 Ubuntu PC 실측을 실제 장치 시험으로 대체 수락하고 실물 UPS 없이 시뮬레이션 결과로 판정하도록 승인했다. native는 GitHub 공개 완료 후 진행한다. 기본 부팅은 2026-11-24 7.0.0-44 설치 뒤 **RT 커널**(같은 버전에서 GRUB이 realtime을 앞에 둠, 2026-11-25 확인). generic 행이 필요하면 Human 승인 뒤 `sudo scripts/rt-boot.sh generic`(1회 generic 부팅), RT 행은 `once`, 기본으로 돌리기는 `back`(2026-11-11 실행 EV-070, generic 명령 2026-11-25 추가). 상세 [TASK-022](../04_implementation/tasks/TASK-022.md)·[ADR-011](../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md).
- RT 무대기·별도 중요도·중요 우선 기본값, 보존 모드 옵션과 처리/저장 예산 분리 유지. FR-018은 선택 옵션·기본 off, 전체 약3초는 신호 발생→차단 예산.
- 플랫폼 Tier 1/2 유지. affinity는 간섭 감소이며 완전 격리/보편적 성능 보장 아님. Tier 2 고급 기능 미지원은 capability로 표시한다.
- FR-015는 장비 측정 auto preset만 2026-11-04 TASK-028 S2로 부분 채택(선택·기본 off). 질문 프리셋 TASK-018은 2026-11-22 Human 「하지 않음 — B2로 이관」(Cancelled).

## 활성 Task 요약

Done Task는 [Task 목록](../04_implementation/task_registry.md)을 따른다(39건: TASK-001~017·019~025·027~040·042).

| Task | 제목 | 담당 | 상태 | 갱신 | 다음 액션 |
|---|---|---|---|---|---|
| TASK-045 | v2 실DB 재검증과 `Writer` 예제 | AI | Planned | 2026-11-26 | REV-073 조치 뒤 S1 |
| TASK-046 | InfluxDB 3 Core 호환 확인 | AI | Planned | 2026-11-26 | TASK-045 뒤 |
| TASK-047 | 장시간 DB 단절 시험 | AI | Planned | 2026-11-26 | TASK-046 뒤 S0 결정 |
| TASK-041 | v0.1.1 패치 릴리즈 | AI | Planned | 2026-11-26 | REV-073·MS-012 뒤. 태그는 Human 확인 뒤 |
| TASK-043 | RT용 미룬 서식 로그 | AI | Planned | 2026-11-23 | MS-011, 0.2 후보 — 기능 논의 |
| TASK-044 | `Record` derive 또는 prost 연동 | AI | Planned | 2026-11-23 | MS-011, 0.2 후보 — 기능 논의 |
| TASK-026 | dot-probe 실제 첫 적용과 화면 연동 | AI | Planned | 2026-11-13 | 프로젝트 마지막 단계(일정 판단 밖) |

## 코드리뷰 현황 (Reviews)

| ID | 범위 | 상태 | 현재 처리/후속 |
|---|---|---|---|
| REV-001~058 | 완료된 core·파일·조회·운영·facade·preset·통합 가드 및 ADR-013 A/B/C 개발 자가검토 | Closed | 회차별 근거는 [리뷰 문서](../05_verification/reviews/)(EV-082~085). native·실앱·성능 이월은 각 Task에 유지 |
| [REV-059](../05_verification/reviews/REV-059_NATIVE_REVISION_LOSS_REVIEW.md) | `fa72a1a..5be17b5` 통합 코드리뷰·후속 `ca89b69` 조치 | **Closed** | RV-149~152 전부 수용·수정·자가검토·위임 수락(EV-086). 복구 완결·손실 관측·종료 경계·등록 검증, 두 Rust 각672 Pass·5 ignored |
| [REV-060](../05_verification/reviews/REV-060_RUNTIME_CONFIG_D.md) | WIP36c1724 기반 runtime D 개발 자가검토·제품3254b81 | **Closed** | RV-153 pending 보존/기한·RV-154 무대기 ack 조치·위임 수락(EV-087), 두 Rust 각684·18 feature·Miri13·표면35/60 Pass |
| [REV-061](../05_verification/reviews/REV-061_ADR_013_E.md) | ADR-013 E 최종 통합·tracing 경합/패닉 복구 회귀 | **Closed** | EV-088, RV-155 두 변이 Red·현행 Green, 기존 위임 수락 |
| [REV-062](../05_verification/reviews/REV-062_DOT_LOGGER_NAMESPACE.md) | .logger 예약 토픽·호스트 예제·standard/2 | **Closed** | EV-090, 두 Rust 각685 Pass·CLI/Foxglove 확인 |
| [REV-063](../05_verification/reviews/REV-063_TASK_031_LIVE_VIEWER.md) | TASK-031 S2 WS bridge·카탈로그·ACK/수명 | **Closed** | EV-092·위임 로컬 수락. Foxglove 실시간·T7 정량 잔여 |
| [REV-064](../05_verification/reviews/REV-064_TASK_031_MCAP_COMPATIBILITY.md) | TASK-031 S3 MCAP 등록 배치 | **Closed** | EV-093·위임 로컬 수락 |
| [REV-065](../05_verification/reviews/REV-065_TASK_031_QUERY_REPLAY.md) | TASK-031 S3 원격 기록 재생 | **Closed** | EV-094·위임 로컬 수락 |
| [REV-066](../05_verification/reviews/REV-066_RUNTIME_NAMESPACE_VIEWER_REVIEW.md) | Viewer 구독·재생 수명, MCAP 등록 검증, IO 작업자 실패 처리 | **Closed** | RV-156~159 수정·자가검토·위임 수락([EV-095](../05_verification/verification_evidence.md#ev-095)) |
| [REV-067](../05_verification/reviews/REV-067_TASK_031_T7_STORAGE.md) | T7 저장 시작·동시 부하 | **Closed** | RV-160·161 Closed. 손실 checkpoint 0 행 생략 뒤 8 GiB P3·P5 Pass(EV-101·103) |
| [REV-068](../05_verification/reviews/REV-068_TASK_031_T7_ACK_CRC.md) | T7 마감 변경(0 행 생략·묶음 ACK·CRC) | **Closed** | RV-162 ACK 기한 starvation 조치·위임 수락 |
| [REV-069](../05_verification/reviews/REV-069_T7_STORAGE_REMOTE_ACK_CRC_REVIEW.md) | `b016fe8..98aa5bd` T7 저장·0 행 생략·묶음/파이프라인 ACK·CRC-32 독립 리뷰 | **Closed** | RV-163~165 수용(RV-164 일부 조정)·`0a9bc4d` 수정·위임 수락([EV-106](../05_verification/verification_evidence.md#ev-106)) |
| [REV-070](../05_verification/reviews/REV-070_TASK_032_DASHBOARD.md) | TASK-032 viewer 내장 대시보드 S2·S3 (`0a9bc4d..97ece7f`) | **Closed** | RV-166~170 수용(RV-170 기준 조정)·`007b25c` 수정·위임 수락([EV-109](../05_verification/verification_evidence.md#ev-109)) |
| [REV-071](../05_verification/reviews/REV-071_WHOLE_CODEBASE_GATE4_REVIEW.md) | 코드 전체 독립 리뷰 — 많이 지적된 3개 영역·보안·신뢰성 및 조치 보충 리뷰 | **Closed** | 1차 `RV-171~188`([EV-111](../05_verification/verification_evidence.md#ev-111))·보충 `RV-189~192`(`9622e94`, [EV-112](../05_verification/verification_evidence.md#ev-112)) 조치. 2026-11-21 Human 수락 |
| [REV-072](../05_verification/reviews/REV-072_POST_GATE4_PLATFORM_SOAK_S2_REVIEW.md) | `ae22f9d..f30ae5a` 코드 변경 및 TASK-024 S2 설계 독립 리뷰(5대 중점) | **Closed** | 9건 수용(RV-196 범위 확대, RV-201 `version`은 MS-010 3단계로)·조치([EV-122](../05_verification/verification_evidence.md#ev-122)). 새로 켠 macOS·Windows 시험 중 플랫폼 차이 3건(f9·f11·f15)은 해당 플랫폼으로 한정. RV-199 후속 macOS viewer 회귀 수정(EV-124). 2026-11-24 CI run `45370553005` 16개 Pass로 위임 수락 Closed |
| [REV-073](../05_verification/reviews/REV-073_HARNESS_AND_LATENCY_REVIEW.md) | `f30ae5a..2632b6c` — 플랫폼 후속·시험 검출력·CI·하네스·README 지연 수치 | **Open** | Conditional. RV-202 F3 채움·삭제 경로(P1), RV-203~209 하네스 판정·종료·증거와 공개 해석(P2), RV-210~212 회귀·실행 확인(P3). CI 19/19 Pass |

이전 회차별 대상·판정·처리 이력은 [리뷰 문서](../05_verification/reviews/)와 [상태 아카이브](state_archive.md)의 #014·#024·#029 이관을 따른다.

## 다음 시작점

0. **InfluxDB 논의 결과(2026-11-26 #003):** 시험 3종(v2 실DB 재실행·v3 Core 호환·장시간 DB 단절)·`Writer` 직접 예제·v0.1.1 전·Docker 일회용 컨테이너 결정 → [MS-012](../04_implementation/milestones/MS-012.md)·TASK-045~047 설계·승인(M1=새 MS·M2=REV-073 선행·TASK-045 D1=crate 예제·D2=현재 API, TASK-047 D는 그 S0). 순서: **REV-073 반입·조치**(하네스 겹침·README RV-209) → TASK-045 → 046 → 047 → TASK-041. 이전 메모: 현재 InfluxDB 관련: `influxdb` feature, 시험 `tc_008_influxdb`·`_query`·`_routed`(가짜 서버)와 `tc_008_influx_live`(실제 DB, 기본 ignore — `scripts/check-influxdb.py`로 digest 고정 격리 DB에서 실행), 예제 없음, facade DB 출력 없음(기능 논의 안건), TASK-024 S2 F4-DB 미실행(tc_008 갈음). REV-073은 Open이며 F3 채움 재실행 전 RV-202 처리.
0-1. ~~CI 확인~~ — 2026-11-24 완료: run `45197725224`의 macOS viewer 실패는 RV-199 회귀였고 수정 뒤 run `45370553005` 16개 Pass → REV-072 Closed·TASK-038 Done(EV-124).
1. **순서(2026-11-23 Human):** ~~TASK-024 S5~~(2026-11-24 Done) → ~~TASK-039 할당 경로~~(2026-11-24 Done, EV-125) → ~~TASK-040 혼합 장시간~~(2026-11-25 Done, EV-126) → ~~TASK-042 `try_log` 지연 분포~~(2026-11-26 Done, EV-129) → **REV-073 반입·조치 → MS-012(TASK-045~047)**(2026-11-26) → **TASK-041** v0.1.1(태그 Human 확인) → **MS-010 2단계 전 추가 기능 개발 논의** → TASK-026.
   - 기능 논의 안건: facade DB 출력·재시작 자동 복구·ENOSPC 뒤 회복과 `IoPoisoned` 뒤 RT 수락 정책·Health 회복 표시(`RecordsFailed`가 `SyncOverdue`를 가림)·외부 공간 압박 시 링 최소 보존·**원격 구독 keepalive(조용한 단절 미감지, EV-123 F4)**·CLI `tail` 종료 결과 구분(`Gone`/초과 끊김이 `unknown`)·**원격 구독 실패 원인 구분(로컬 예산 부족과 서버 구독 한도가 같은 `ClientError::Remote(Limit)`, viewer는 둘 다 「upstream unavailable」로 알리고 재시도 — 설정 오류가 시간 초과로만 보임, EV-124. CLI `tail`도 출력 상한 도달과 서버 한도가 같은 `limit`·exit 3, EV-126)**·**Windows·macOS 짧은 대기 해상도(작은 슬롯 처리량 — Windows CI에서 작은 기록 약 1,300건/s, 링·이벤트 시험 5개가 조합당 약 6분, EV-128)**·**레벨 skip 호출의 시계 읽기 약 100 ns(레벨 확인을 앞당기면 거의 0, TASK-042 S1)**·**TASK-043 RT 미룬 서식 로그·TASK-044 derive/prost(0.2 후보)**·**출력 구성 선택·네트워크 전용 연구 모드(2026-11-26 Human R1~R3, 아래 [상세](#기능-논의-안건-상세--출력-구성-선택과-네트워크-전용-모드-2026-11-26) — 우려 검토 뒤 MS/Task화)**·B1/B2.
   - 장시간·대용량 실행은 시간·쓰기량을 먼저 알리고 승인받는다. 채움 시험은 같은 실행 맥락에서 대상 tmpfs를 확인한 뒤에만.
2. **마지막 단계:** TASK-026 실사용처 적용은 일정 판단 밖이다. TASK-024는 2026-11-24 Done이고, 24h·멀티 OS CI·게이트는 1.0.0 직전에 다시 한다(MS-010 3a, T8). TASK-018은 Cancelled(B2).

### 기능 논의 안건 상세 — 출력 구성 선택과 네트워크 전용 모드 (2026-11-26)

계기: InfluxDB 논의 중 Human이 연구개발용 구조를 제안. 상태: **안건(우려 검토 중)** — MS/Task·ADR 전. 다음 행동: 아래 우려 C1~C11을 Human과 검토 → 범위·순서 결정 → MS 등록·ADR.

**요구(Human):**
- **R1 출력 선택:** 시작 때 옵션 파일로 파일·네트워크·DB를 배타적으로 또는 동시에 선택.
- **R2 네트워크 전용 고부하 모드:** 초당 ~100 MB 단위동작 기록을 SSD에 쓰지 않고 정해진 메모리 안에서 네트워크로만 보내 원격(예: 원격 PC Redis)에서 분석.
- **R3 외부 전송 스위치:** 원하는 순간에 네트워크 전송을 켜고 끔.

**확인한 현재 동작(코드):**
- 흐름: 입력(`try_log`) → writer 링·raw 풀 → host 스레드 Router 분배 → 출력별 큐 → 파일 IO / 원격 tap.
- 파일 출력은 facade에서 모두 `required`·`pending_slots 0`·`max_attempts 1`, 공유 한도가 먼저 차 느린 파일이 입구 RT `Capacity` 거절로 역류(`logger/budget.rs` 주석). 거절은 모든 파일에서 빠지고 손실 보고에 남는다(EV-123 F1).
- 원격은 Router target이 아니라 **첫 번째 파일 출력이 쓴 것의 tap**(`logger/host.rs`)이고 `Tap::offer` 결과를 버린다. 구독별 예산 초과 시 그 구독만 Gap/Disconnect. **로거 쪽 원격 손실 집계 없음**(Health·종료 보고·`.logger` 모두), 구독자만 Gap을 앎. 고급 경로에는 `PendingAck`·`RemoteDelivery`(전송별 Acknowledged/Unknown/Dropped)가 있다.
- 파일 출력 없으면 `start()`가 `Missing::FileOutput`. DB는 facade에 없음. 옵션 파일 로딩 없음(코드 builder만), 실행 중 패치는 레벨·샘플링만. 원격 버퍼는 `remote_with(RemoteLimits)`로 키울 수 있음(기본 구독당 ≤ 4 MiB·`max_age` 10 s).
- 성능 근거: 유선 1 GbE gRPC 대용량 116 MB/s(TCP와 같은 수준, TLS), 256 B 1만 건/s ACK p99 4 ms(EV-105).
- 정확도: 파일(+종료 보고) > DB(`Acknowledged`만 확실) > 원격(구독자만 손실 인지).

**필요한 변경(초안):** 원격을 Router 독립 출력으로(파일 없는 시작 허용) · 필수가 아닌 출력과 출력별 버림 정책 · 원격 손실 집계(Health·종료 보고) · 옵션 파일 형식·로딩 · 외부 전송 스위치 · facade DB 출력(MS-012 결과 뒤).

**우려(초안, 검토 대상):**

| ID | 우려 | 내용 | 검토할 질문 |
|---|---|---|---|
| C1 | 손실 근거 상실 | 파일이 없으면 기준 기록이 없다. 지금 원격 손실은 로거에 남지 않아 「무엇이 빠졌는지」 답할 근거가 없다 | 로봇 쪽 원격 집계(보냄·Gap·끊김·Unknown)를 필수로 할지, 작은 로컬 손실 기록(파일 1개)을 남길지 |
| C2 | 링크 여유 | 1 GbE 실측 상한 ~117 MB/s에 100 MB/s는 85%. 버스트·재전송 여유가 거의 없다 | 2.5/10 GbE 전제인지, 채널 선택·압축으로 줄일지 |
| C3 | 메모리 상한과 버스트 | 로봇 쪽 버퍼 N MB는 100 MB/s에서 N/100 초의 정지만 흡수. 수신(Redis)이 같은 속도를 못 따라가면 Gap. Redis는 메모리 DB(100 MB/s ≈ 6 GB/분) | 버퍼 크기·정지 허용 시간 목표, 수신 쪽 상한·보관 정책 |
| C4 | 구독 전·스위치 전 데이터 | 구독자가 없을 때는 버려진다. 스위치를 켠 순간 구독이 아직 없으면 시작 부분을 잃는다 | 「구독 먼저 → 트리거」 순서 규약인지, 트리거 이전 일정 시간을 메모리에 들고 있다 보내는 pre-trigger 창이 필요한지 |
| C5 | 의미 변경 | 원격이 「파일이 쓴 것」의 tap에서 Router 출력으로 바뀌면 viewer 등 기존 구독자가 보는 순서·내용 보장이 달라진다 | 파일+원격 동시 구성에서 기존 tap 의미를 유지할지 |
| C6 | 완료·종료 판정 | 파일 없는 구성에서 「완료」와 종료 성공의 기준이 없다(지금은 sync) | 수신자 ACK(기존 ACK RPC)를 완료로 볼지, 보냄까지로 볼지 |
| C7 | 옵션 파일 소유와 비밀 | 라이브러리가 파일을 읽으면 형식·버전·검증·비밀(TLS 키·토큰·DB 토큰) 취급이 라이브러리 책임이 된다. 범용 라이브러리 방향과 충돌 가능 | 라이브러리가 읽을지, 직렬화 가능한 설정 타입만 주고 앱이 읽을지 |
| C8 | 외부 스위치 권한·경합 | 원격에서 켜고 끄려면 쓰기 권한(ApplyConfig와 같은 수준)·상태 보고가 필요. 끌 때 진행 중 기록 처리, 꺼진 동안 분배 비용 0 여부 | 스위치 주체(로봇 API / 원격 명령 / 구독 자체)와 끈 동안의 처리 |
| C9 | 호스트 CPU | 100 MB/s 복사·인코딩·TLS가 관리 스레드 CPU를 쓴다. RT 경로는 설계상 분리되지만 같은 PC의 코어 경합·affinity 영향 | 원격 전용 스레드·CPU 배치, 측정 기준 |
| C10 | UPS·내구 보장 범위 | UPS 경보 뒤 3 s 마감·sync 내구 보장은 파일 전제. 네트워크 전용에서는 적용되지 않는다 | 모드별 보장 범위를 문서·Health에 명시 |
| C11 | 버전·검증 비용 | 공개 API·출력 모델 변경이라 0.2. 100 MB/s 네트워크 전용은 새 성능·장애 시험(수신 정지·단절·장시간)이 필요하고 이 PC 네트워크가 1 GbE일 수 있다 | 시험 장비(링크 속도·수신 PC), MS-012·keepalive·facade 원격 재생 안건과 묶을지 |

**먼저 검토할 우려(AI 제안, 2026-11-26 Human 「기록해두고」):** 범위·설계를 가장 크게 바꾸는 순서.
1. **C1** — 네트워크 전용이면 로봇 쪽 원격 손실 집계는 사실상 필수(지금 로거 쪽 기록 0).
2. **C2·C3** — 링크 속도와 수신 정지 허용 시간을 먼저 정한다(1 GbE 실측 ~117 MB/s, 버퍼 N MB = 100 MB/s에서 N/100 s).
3. **C4** — 「구독 먼저 → 트리거」 규약으로 충분한지, pre-trigger 메모리 창이 필요한지가 R3 설계를 가른다.
4. **C7** — 범용 라이브러리라면 직렬화 가능한 설정 타입만 주고 파일은 앱이 읽는 쪽이 일반적.
5. **C10** — UPS 3 s 마감·sync 내구 보장은 파일 전제. 네트워크 전용에서는 없다는 것을 문서·Health에 드러낸다.

C5·C6·C8·C9·C11은 위 표의 질문으로 검토한다. 검토 시점: REV-073 조치 뒤 또는 Human 요청 시.

관련 기존 안건: keepalive(EV-123 F4) · facade 원격 재생(지금 Query/Replay는 facade에서 `Unsupported`) · 원격 실패 원인 구분(EV-124·126) · facade DB 출력.

## 승인·재개 제약

기존 구현·검증·수락·작업 전환·작업 단위별 커밋/푸시 위임은 유지한다. 새 범위·공개 전환·선택 기능 자동 착수로 확대하지 않는다. native 공개 후(T3)·PC/RT/장기/실앱·설정/비교·지속 실앱 통합·Gate4/5 미완료 조건은 유지한다. 상세 승인/이전 에디터 진단 정정·과거 재개 메모는 [상태 아카이브](state_archive.md)의 #014 이관과 관련 Task/API를 따른다. 개발1.96.0·MSRV1.85·Miri nightly-2026-10-26 고정, 전역 rustup default/사용자 에디터 설정은 변경하지 않는다.

## 선택 로딩

- [도메인](../03_design_artifacts/domain_model.md), [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안), [데이터](../03_design_artifacts/data_model.md), [CLI](../03_design_artifacts/ui_spec.md).
- [MS 목록](../04_implementation/milestone_registry.md): 12개 MS, [Task 목록](../04_implementation/task_registry.md): 47개 Task(TASK-018 Cancelled). 선행조건·변경 대상·첫 실패/DoD·상대 인일 추정(계획 승인, 납기 확정 아님).
- [요구 추적표](../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트): INT 1 + FR 18 + NFR 8 + CON 7 = 34행. MS별 구현/테스트 체크·코드/EV 연결. TC-001의 TASK-002 하위 EV-001 로컬/native, TC-002 소형 EV-002 로컬/native·원본 EV-003 로컬, TC-003 분배 EV-004·완료/제어 EV-005 로컬 Pass; 후속/native/장비 시험 잔여.
- 2026-10-26 조사(비공개 원시 근거: `imported_context/research_2026-10-26_detailed_design.md`): 공식 자료와 dot-probe 코드 관측, 기술 후보/사실/미검증 분리.
- 이전 결정 근거가 필요할 때만 ADR-003~006·요구 §12~17·Story 상세·개인 로그를 추가로 읽는다.

## 이월 백로그

다음 T = T9 / 다음 B = B3. 트리거 감시는 AI 책임이다.

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| T1 | ROS 2 파일 호환·토픽 연동 필요/범위 검토 | dot-probe 첫 적용 완료가 확인된 때 | INT-001 / OQ-008 / TASK-026 | 2026-10-25 Human Q5 |
| T8 | 1.0.0 직전 재검증 — 후보 revision으로 24h soak(TASK-040 혼합 경로 포함)·멀티 OS CI(3 OS × 2 Rust·exchange)·Gate 4/5 재판정([MS-010](../04_implementation/milestones/MS-010.md) 3a) | MS-010 4단계(v1.0.0) 착수 또는 1.0.0 후보 revision 확정 시 | MS-010 / TASK-024 / quality_gate | 2026-11-24 Human 「모두 승인, 그러나 24시간 테스트와 멀티OS CI, 게이트는 최종 릴리즈 1.0.0 직전 다시 할 수있게」 |
| T4 | facade 소형 기록 slot 직접 encode 별도 설계(scratch encode + byte ring 복사 대체) | TASK-023 S2 P1(256B) 측정에서 encode·복사 비용이 의미 있게 나온 때 | NFR-001 / API §18.3 / TASK-027 S3d / TASK-023 S2 | 2026-11-07 Human 「승인」·백로그 기록 요청 |

범위 밖 백로그 B:

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| B1 | TASK-033 S3 기능 공백(범위 밖, 일정 판단 제외): UC-02 오래된 기록 덮어쓰기·정책 선택, UC-05 high-water 지표, UC-07 시간 기준 rate·출력/상태별 rate, UC-10 정규식·프로필 채널 선택, UC-11 시계 객체 주입·epoch 이전 표시, UC-12 사용자 파일 metadata·첨부, UC-14 `log` kv·span, UC-16/19 요청 시 분할·sync(facade ④), UC-18 backtrace, UC-21 panic hook·signal, UC-22 쓰기 오류 재열기, UC-23 압축, UC-24 반복·step·remap·real 순서, UC-25 병합·필터 export, UC-28 원격 일시정지·분할·snapshot, UC-29 상태 기반 기록, UC-31 텍스트 검색 | 사용처 요구 또는 Human 요청 시 | [TASK-033](../04_implementation/tasks/TASK-033.md#uctc-대응--s3-2026-11-15) | 2026-11-15 Human 「모두 권고대로」 |
| B2 | FR-015 질문 프리셋 — 짧은 질문에 답하면 `Sizing`을 채워 주는 설정 추천 흐름(CLI 마법사 등). 과신 추천·자동 적용 금지는 TASK-028 규칙을 따른다 | 사용처 요구 또는 INT-002 구체화 시 | [TASK-018](../04_implementation/tasks/TASK-018.md) Cancelled / FR-015 | 2026-11-22 Human 「하지 않음 — B2로 이관」 |
**T2 해소**(2026-11-21 공개 전환), **T3 해소**(2026-11-22 EV-113~116, native CI 3 OS × 2 Rust Pass·GAP-004 Closed — 다른 OS 사이 파일 교환·24h는 TASK-024), **T5 해소**(EV-066~068), **T6 해소**(EV-083: Envelope scratch 복사 제거, 정량 효과 미측정). T1·T4는 미도래이며 EV-088 재측정에서도 256B 인코드·복사 p50은 시계 대조와 같은 21ns였다. **T7 해소**(포화 회복 EV-097, P5 EV-103, LAN P4 EV-104, 유선 P4-bulk EV-105). native 실행을 공개 후로 정한 것은 공개 전환 요청이 아니므로 T2를 발동하지 않는다. OQ는 T/B에 중복 등록하지 않는다.

## 열린 결정과 검증 공백

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| OQ-002 | ADR-011로 현재 PC 실측·UPS 시뮬레이션 수락 확정. local-pc-v1 수치 Approved(EV-021), UPS 하위 EV-043 수락·성능/RT 실측 잔여 | 실앱/성능 harness·RT 부팅 확인 후 현재 PC 성능/TC-012·장기 통합 실행 |
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·실제 앱/장비 수명 수락은 미검증 | TASK-004~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬/native CI 확인(EV-001), 실제 DB·장비 호환 잔여 | TASK-019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증. 질문 프리셋은 B2 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·005 | PC 환경 확정·프로필/후속 TC 실행 잔여·paste 경고. GAP-003 Closed, GAP-004 Closed(2026-11-22 native CI, EV-116) | [검증 근거](../05_verification/verification_evidence.md) 참조 |
| Closed GAP·RV | GAP-006~010·RV-006~035 로컬 조치/수락 완료(native는 GAP-004 이월) | 원문 [상태 아카이브](state_archive.md) #023 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-11-26 #002 — [REV-073](../05_verification/reviews/REV-073_HARNESS_AND_LATENCY_REVIEW.md) 독립 리뷰 마감. Conditional, RV-202~212 Open. CI 19/19 Pass, README 지연 5행 재계산 일치.

- 2026-11-26 #001 — REV-073 리뷰 요청서(`f30ae5a..2632b6c`)·세션 마감. TASK-042 S2 ③-b generic·`performance` 측정(RT 대비 p50~p99.99 같고 최대에서 차이, 1 kHz 깨어남 단일 729 µs), S3 README 지연 표·EV-129, TASK-042 Done. 50건 묶음 꼬리 원인 분석 — 이 PC가 쉬는 동안의 주기적 코어 감속(약 600 ms마다 0.5 ms·9배), dot-logger 무관(EV-130).
- 2026-11-25 #001 — 재부팅 뒤 TASK-042 S2 ③-a generic·`powersave` 측정(RT 대비 꼬리 2~3배, TASK-042 문서)(비공개 세션 로그: `session_logs/session_2026-11-25_001.md`).
- 2026-11-24 #001(~11-25) — CI 근본 수정(RV-199, EV-124)·REV-072 Closed, TASK-024 S5·TASK-038·039·040 Done(EV-125·126), CI 시간 의존 시험 5건 정리·CI 151 → 56분(EV-127·128), TASK-042 D3·S1 하네스·S2 RT 측정 ①②(비공개 세션 로그: `session_logs/session_2026-11-24_001.md`).
- 2026-11-23 #002 — REV-072 반입·조치(EV-122, CI에서 macOS·Windows 새 시험 플랫폼 차이 3건 정리), TASK-024 S2 하네스·F1~F5(EV-123, 하네스 채움 사고 포함), MS-011·TASK-042~044 등록(비공개 세션 로그: `session_logs/session_2026-11-23_002.md`).

완료 상세 원문과 이전 이력은 [상태 아카이브](state_archive.md) #030~031·#033·#034 및 관련 EV·세션 로그에 보존한다.

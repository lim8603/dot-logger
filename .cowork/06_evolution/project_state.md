# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 수집·분배·완료/제어·저장/보존/복구·조회 로컬 검증, 재생·worker 후속 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — TASK-011 Human 승인·EV-019 로컬 Pass, TASK-022 현재 PC 검증 프로필 준비 |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-004 In Progress — 조회·재생·클라이언트, MS-007은 TASK-022 프로필 준비 In Progress |
| 활성 Task | TASK-011 In Progress — Human 승인·현재 PC 실측 후속 / TASK-022 In Progress — PC/UPS 수락 확정·프로필 준비 |
| 미처리 리뷰 | [REV-014](../05_verification/reviews/REV-014_TASK-010_CODE_REVIEW.md) Resolved — RV-021~023 조치·EV-017 로컬 Pass·Human 리뷰 대기. [REV-013](../05_verification/reviews/REV-013_TASK-010.md) Resolved — RV-018~020 조치·EV-016 로컬 Pass·Human 리뷰 대기. TASK-008 REV-009/010 Resolved·Human 리뷰 대기, TASK-009 REV-011/012·TASK-011 REV-015/016 Closed |
| 상태 | Yellow — 현재 PC 프로필/실측·TASK-008/010 Human 리뷰 잔여, native는 공개 완료 후 승인 이월 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-10-31 / AI |
| 세션 상태 | 종료 — REV-016 조치/소스 게시·TASK-011 Human 승인·ADR-011 검증 기준/환경 프로필 기록, 문서 분리 게시 |
| 최근 세션 | [2026-10-31 #001](../members/lim8603/workspace/session_logs/session_2026-10-31_001.md) — 소스9660d7d 게시·EV-019/승인·ADR-011·TASK-022 프로필 준비 후 종료 |

## 현재 결정과 제약

- **TASK-011 현재 계약:** frozen Snapshot을 소유하는 mono ns 동기식 replay·최대1건 prefetch·양수 u32 유리수 배속·pause/resume/seek/stop·checked generation과 누적 읽기/원본 수명을 사용한다. 늦은 소비자의 재생 시계를 늦추고 원격 in-flight 취소를 추정하지 않는다. End는 마지막 blob read를 위해 snapshot을 유지하고 stop/Drop·terminal 오류에서 실제 reader close 뒤 반환한다. 상세 [TASK-011](../04_implementation/tasks/TASK-011.md)·[API §14.2](../03_design_artifacts/interface_contract.md#142-task-011-로컬-재생-api--2026-10-30).

- **TASK-010 현재 계약:** 고정 session/revision/원래 등록·독립 read-only source·완전 chunk safe offset, 유한 external/index/page/blob/read 예산을 사용한다. reader-owned permit은 active→closed 회전을 따라 보호하며 실제 reader Drop 뒤에만 회수 가능하다. 저수준 pin 분실/forget의 강제 회수·clock 변환은 도입하지 않는다. 현재 closed도 bounded scan으로 index를 재구성하며 summary 기반 scan 생략 최적화·live/replay/remote/CLI·실제 성능/OS 검증은 후속이다. 상세 [TASK-010](../04_implementation/tasks/TASK-010.md)·[API §14.1](../03_design_artifacts/interface_contract.md#141-task-010-로컬-api--2026-10-30).

- **TASK-009 현재 계약:** 불변 read-only 원본·원래 session/revision/생산자 수/동일 순서의 채널 등록·독립된 빈 출력이 필요하다. 완전 검증 chunk prefix만 복구하며 손상 뒤 재동기화·과거 completion 응답 복원·실패 파일 자동 삭제/재시도는 하지 않는다. 상세: [TASK-009](../04_implementation/tasks/TASK-009.md)·[API §13.5](../03_design_artifacts/interface_contract.md#135-task-009-중단-복구-api--2026-10-30).

- **승인·리뷰·검증 경계:** 2026-10-30 Human 「승인, 커밋 푸쉬하고 다음 작업 진행하자.」로 TASK-009 최신 구현·REV-012 조치 승인, REV-011/012 Closed 및 TASK-010 진행 승인. TASK-008 REV-009/010 Resolved·Human 리뷰 대기는 별도다. TASK-004~011 native GAP-004는 2026-10-31 ADR-011로 공개 완료 후 승인 이월, 실제 장치 시험은 현재 PC 실측/UPS 시뮬레이션 수락이며 CI/계정/과금 조회·차단 재시도는 하지 않는다.

- **2026-10-31 TASK-011 최종 구현 승인:** 구현 계약 확인 보고 후 Human 「승인」으로 EV-018/019의 현재 구현·RV-024~028 조치를 수락했다. REV-015/016 Closed·Human 리뷰 체크 완료. TASK-008/010 Human 리뷰 및 native GAP-004·장비 이월은 별도이며 커밋/푸시·TASK-012 착수는 수행하지 않았다.

- **형상관리 제약:** 소스와 `.cowork` 문서는 별도 커밋·[skip ci]로 관리한다. TASK-009 소스 `e5f3147`은 EV-014 source51개와 commit blob hash가 일치하며 검증/마감 문서212373c와 origin/main 게시 완료다. EV-015 source51개와 소스 `fe03468` commit blob이 일치한다. 소스fe03468·문서aee3830을 origin/main에 분리 게시했다([게시 근거](imported_context/task_009_review_fixes_2026-10-30/push.json)). TASK-010 소스 `8bbec13`를 로컬 커밋했으며 EV-016 source53개와 commit blob hash가 일치한다. REV-014 조치 소스 `439d9c5` 로컬 커밋·EV-017 source52개/commit blob 일치, 문서35e3ac3과 함께 origin/main 게시·원격 HEAD 일치를 확인했다. TASK-011 소스 `026999f`도 origin/main 게시·EV-018 source54개/commit blob 일치를 확인했다. 리뷰 조치 소스 `9660d7d`도 origin/main에 게시했고 EV-019 source54개/commit blob 일치를 확인했다. 검증/수락 기준/마감 문서는 별도 [skip ci] 커밋으로 게시한다. TASK-011 Human 승인은 완료이며 TASK-008/010 Human 리뷰만 별도 대기다. 인증 정보는 OS keyring에 두고 저장소에 넣지 않는다. 세션 종료 후 새 Task를 자동 착수하지 않는다.

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
| TASK-008 | 회전·순환·이벤트 보호 | AI | In Progress | 2026-10-31 | REV-009/010 Human 리뷰·현재 PC 실측, native는 공개 완료 후(T3) |
| TASK-009 | 중단 복구·IO 장애 | AI | In Progress | 2026-10-31 | EV-014/015 로컬 Pass·Human 승인; 현재 PC 대체 실측·모의 장애, native 공개 후 |
| TASK-010 | 시간·채널 조회/active snapshot | AI | In Progress | 2026-10-31 | REV-013/014 Human 리뷰·현재 PC 조회/IO 실측, native 공개 완료 후(T3) |
| TASK-011 | 재생 시간과 취소 제어 | AI | In Progress | 2026-10-31 | EV-019 로컬 Pass·Human 승인; 현재 PC 실제 시계/IO 실측, native 공개 후 |
| TASK-022 | 실제 장비·플랫폼 검증 프로필 | AI | In Progress | 2026-10-31 | PC/UPS 수락 확정·환경 수집 → RT 부팅/P0·제어 부하/기한/경로/예산 구체화 |

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
| [REV-009](../05_verification/reviews/REV-009_TASK-008.md) | TASK-008 | 2026-10-30 | 회전·보존 자가 리뷰 / Conditional (P2 1, 로컬 해결) | Resolved | RV-011 EV-012 수정/회귀 Pass, Human 최종 구현 승인 대기 |
| [REV-010](../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md) | TASK-008 | 2026-10-30 | 회전·보존 심층 코드리뷰 / Conditional (P2 1, P3 2) | Resolved | EV-013 로컬 Pass, RV-012 위험/후속 TASK-010 유지·Human 최종 리뷰 대기 |
| [REV-011](../05_verification/reviews/REV-011_TASK-009.md) | TASK-009 | 2026-10-30 | 복구·IO 장애 자가 리뷰 / Conditional (P2 1, 로컬 해결) | Closed | RV-015 EV-014 수정·회귀 Pass, 2026-10-30 Human 승인·native 후속 |
| [REV-012](../05_verification/reviews/REV-012_TASK-009_DEEP_REVIEW.md) | TASK-009 | 2026-10-30 | 심층 코드리뷰 / Pass (P3 2) | Closed | RV-016/017 EV-015 로컬 Pass·소스fe03468·2026-10-30 Human 승인 |
| [REV-013](../05_verification/reviews/REV-013_TASK-010.md) | TASK-010 | 2026-10-30 | 조회/읽기 수명 자가 리뷰 / Conditional (P2 3) | Resolved | RV-018~020 조치·EV-016 로컬 Pass, 최종 Human 리뷰/native·장비 대기 |
| [REV-014](../05_verification/reviews/REV-014_TASK-010_CODE_REVIEW.md) | TASK-010 | 2026-10-30 | 심층 코드리뷰 / Pass (P2 1, P3 2) | Resolved | RV-021~023 정렬 캐시·rustdoc 조치·EV-017 로컬 Pass, Human 리뷰/native·장비 대기 |
| [REV-015](../05_verification/reviews/REV-015_TASK-011.md) | TASK-011 | 2026-10-30 | 재생/시간·자원 수명 자가 리뷰 / Conditional (P2 2, 로컬 해결) | Closed | RV-024/025 조치·EV-018/019 로컬 Pass·2026-10-31 Human 승인, native·장비 잔여 |
| [REV-016](../05_verification/reviews/REV-016_TASK-011_CODE_REVIEW.md) | TASK-011 | 2026-10-30 | 로컬 재생·시간 제어 심층 코드리뷰 / Pass (P2 1, P3 2) | Closed | 2026-10-31 RV-026 현 불변식상 overflow 불가·방어 정리, RV-027/028 진단 API 조치·EV-019 Pass·2026-10-31 Human 승인·native/장비 잔여 |

## 다음 시작점

0. **현재 형상 기준:** TASK-011 리뷰 조치 소스 `9660d7d`를 origin/main 게시·원격 hash 일치 확인했다. [EV-019 source54개/검사10개·로그 hash](imported_context/task_011_review_fixes_2026-10-31/results.json)·[commit blob54개 대조](imported_context/task_011_review_fixes_2026-10-31/commit.json)·[소스 게시 근거](imported_context/task_011_review_fixes_2026-10-31/push.json)를 따른다. 검증/승인·ADR-011/마감 문서는 소스와 별도 [skip ci] 커밋으로 게시한다. 동일 소스의 성공 시험 반복·CI/계정/과금 조회·차단 재시도는 하지 않는다.

1. **TASK-011 Human 최종 구현 승인 완료:** [TASK-011](../04_implementation/tasks/TASK-011.md)·[REV-015](../05_verification/reviews/REV-015_TASK-011.md)/[REV-016](../05_verification/reviews/REV-016_TASK-011_CODE_REVIEW.md) Closed·Human 리뷰 체크 완료. EV-019/source54개 hash 기준 현재 구현 및 리뷰 조치를 2026-10-31 Human 「승인」으로 수락했다. 리뷰 조치 소스 `9660d7d` 게시·검증/승인/마감 문서 분리 게시로 세션을 종료했다. 장치 시험은 현재 PC 실측으로 대체하고 native GAP-004는 공개 완료 후 진행한다(ADR-011). TASK-012는 자동 착수하지 않는다.
2. **현재 PC 프로필/실측 준비:** [TASK-022](../04_implementation/tasks/TASK-022.md) In Progress. PC/UPS 수락 방식과 환경 관측 완료, P0·제어 부하/기한·시험 경로/예산을 구체화한다. 현재 generic 실행·RT 설치를 구분하며 RT 행은 해당 커널 부팅 뒤 실행한다. TASK-008/010 Human 리뷰는 별도 대기다. native는 T3(공개 완료 확인)에서 당시 source hash를 고정해 6 contracts+audit+raw-memory를 실행한다. TASK-012/021/023/024 제품 구현/장시간 시험은 자동 착수하지 않는다.

심층 제안은 EV-008 판단을 유지한다. 채널 탐색/LUT·RecordSlot 정렬은 장비 측정 후 비교하고, 미등록 채널은 이미 InvalidInput이므로 그 사례에 근거한 CaptureError 확대는 보류한다. poll_shutdown 반복 구동은 rustdoc에 보강했다.

## 새 세션 재개 메모

- TASK-011·REV-015/016 Closed·EV-019를 먼저 확인한다. 최신 [source54개/검사10개·로그 hash](imported_context/task_011_review_fixes_2026-10-31/results.json)가 같으면 성공 시험을 반복하지 않는다. 2026-10-31 Human 최종 구현 승인 완료이며 리뷰 조치 소스 `9660d7d` 게시·관련 문서 분리 커밋으로 종료했다. TASK-008/010 Human 리뷰는 별도 유지한다. ADR-011/TASK-022의 현재 PC 실측·UPS 시뮬레이션 수락과 native 공개 후(T3) 이월을 따르며 TASK-012는 자동 착수하지 않는다.
- 개발1.96.0·MSRV1.85·nightly-2026-10-26 Miri 고정, dependency/lock·RT 무할당/무대기·private unsafe 경계를 유지한다. 전역 rustup default는 바꾸지 않았다.
- 완료/종료·writer 배치/재시도 의미는 [API §13](../03_design_artifacts/interface_contract.md#13-상세-api-설계안)·TASK-006~008을 기준으로 보존한다. source/sink host 메모리와 OS cache는 logger 예약 밖이다.
- RV-012 후속: TASK-010 reader-owned permit은 실제 reader 종료 후만 반환한다. 낮은 ReadPin 유실/forget·종료 불명 background IO는 강제 회수하지 않으며 기한만으로 삭제를 허용하지 않는다.
- 자동 startup 발견/기존 namespace 재개·worker/StorageDegraded는 미구현 후속이다. active snapshot/query primitive는 EV-016/017, local offline replay는 EV-018 로컬 검증 범위다. 자동 worker/remote/live/CLI·실제 timer 성능은 후속이다. native 공개 후 이월·PC 실측/UPS 시뮬레이션 미실행·TC 전체·MS-002/003·Gate4/5 완료를 선언하지 않는다.

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
| T3 | 이월 native 검증 실행·결과 반영 | GitHub 저장소 공개 완료가 확인된 때 | GAP-004 / ADR-011 / TASK-004~011 | 2026-10-31 Human 검증 시점 결정 |

범위 밖 백로그 B: 없음. 현재 T1/T2/T3는 미도래. 공개 후 native 실행 시점 지정은 공개 전환 자체 요청이 아니므로 T2를 지금 발동하지 않는다. OQ를 T/B에 중복 등록하지 않는다.

## 열린 결정과 검증 공백

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| OQ-002 | ADR-011로 현재 PC 실측·UPS 시뮬레이션 수락 확정. P0·제어 주기/부하·수치 프로필 구체화는 잔여 | TASK-022에서 프로필 고정, 현재 PC 실측·TC-011 모의/TC-012 실행 |
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·실제 앱/장비 수명 수락은 미검증 | TASK-004~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬/native CI 확인(EV-001), 실제 DB·장비 호환 잔여 | TASK-019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증, TASK-018은 별도 범위 승인 후 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·004·005 | PC 환경 확정·프로필/후속 TC 실행 잔여, TASK-004~011 native 공개 후(T3) 이월·paste 경고. GAP-003 Closed | [검증 근거](../05_verification/verification_evidence.md) 참조 |
| GAP-006 | EV-006 자가 리뷰 RV-001~004는 EV-007 수정·로컬 회귀 Pass로 Closed | Human 승인 완료·native GAP-004 후속, 기존 이력 보존 |
| GAP-007 | RV-005는 EV-009 로컬 수정/회귀 Pass·Human 승인으로 Closed | native는 후속, 수정 전 EV-008 이력 보존 |
| RV-015~017 | 최종 sync 취소는 EV-014, 방어적 슬라이싱/스택 포맷은 EV-015에서 반영·REV-011/012 Closed·Human 승인 | native·장비 검증 대기 |
| RV-024/025 | TASK-011 늦은 소비 중 배속 변경/오류 정리 반례를 EV-018에서 조치, REV-015 Closed·2026-10-31 Human 승인 | native·장비 검증 대기 |
| RV-026~028 | RV-026 현 불변식상 overflow 불가·방어 정리, RV-027/028 진단 API 조치·EV-019 Pass·REV-016 Closed·2026-10-31 Human 승인 | native·장비 검증 대기 |
| RV-021~023 | 동일 Query 정렬 재사용·Page inline/누적 읽기 예산 안내를 EV-017에서 조치, REV-014 Resolved | Human 구현 리뷰/native·장비 대기 |
| RV-018~020 | TASK-010 만료 복원·blob Gone·frame Limit을 EV-016에서 조치, REV-013 Resolved | Human 구현 리뷰/native·장비 대기 |
| RV-006~010 | EV-011 로컬 수정·Human 승인 완료, REV-008 Closed | native GAP-004·장비 성능은 잔여 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-10-31 검증 기준 갱신: ADR-011 Accepted·현재 PC 실측/UPS 시뮬레이션 수락·native 공개 후(T3) 이월. TASK-022/MS-007 프로필 준비 In Progress, 실행 generic/설치 RT 확인. 제품 시험 미실행.

- 2026-10-31 #001: 프로젝트 브리핑·REV-016 반입/처리·EV-019 로컬 Pass·Human 최종 구현 승인·REV-015/016 Closed, 소스9660d7d 게시·ADR-011 현재 PC/UPS 수락·native 공개 후 이월·문서 분리 게시로 종료 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-31_001.md).

- 2026-10-30 #008: TASK-011 심층 리뷰·REV-016 Open 등록, 제품 코드 미수정으로 종료 — [리뷰 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_008.md).
- 2026-10-30 #007: TASK-010 게시 후 TASK-011 구현·EV-018 로컬 Pass·REV-015 Resolved, 소스026999f 게시와 검증/마감 문서 분리 커밋으로 종료 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_007.md).
- 2026-10-30 #006: REV-014 RV-021~023 정렬 재사용·API/예산 안내 조치·EV-017 로컬 Pass·REV-014 Resolved — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_006.md).

이전 완료 이력은 [상태 아카이브](state_archive.md)에 보존했다.

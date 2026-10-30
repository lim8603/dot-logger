# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 수집·분배·완료/제어·MCAP 저장·회전/보존/복구 로컬 검증, reader·worker 후속 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — TASK-009 중단 복구·IO 장애·EV-014 로컬 Pass |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-003 In Progress — 파일 저장(MS-002 native 검증 이월) |
| 활성 Task | TASK-009 In Progress — EV-014 로컬 Pass, Human 구현 리뷰/native 대기 |
| 미처리 리뷰 | [REV-012](../05_verification/reviews/REV-012_TASK-009_DEEP_REVIEW.md) Open — TASK-009 심층 리뷰(P3 2) / [REV-011](../05_verification/reviews/REV-011_TASK-009.md) Resolved / REV-009·010 Resolved 유지 |
| 상태 | Yellow — TASK-009 EV-014 로컬 Pass, REV-012 Open, TASK-008/009 Human 리뷰·native CI 차단 잔여 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-10-30 / AI |
| 세션 상태 | 리뷰 세션 — TASK-009 중단 복구 심층 코드리뷰 완료 (REV-012 Open) |
| 최근 세션 | [2026-10-30 #003](../members/lim8603/workspace/session_logs/session_2026-10-30_003.md) 종료 — TASK-008 조치 게시·TASK-009 복구 구현/검증·commit/push |

## 현재 결정과 제약

- **TASK-009 현재 계약:** 불변 read-only 원본·원래 session/revision/생산자 수/동일 순서의 채널 등록·독립된 빈 출력이 필요하다. 완전 검증 chunk prefix만 복구하며 손상 뒤 재동기화·과거 completion 응답 복원·실패 파일 자동 삭제/재시도는 하지 않는다. 상세: [TASK-009](../04_implementation/tasks/TASK-009.md)·[API §13.5](../03_design_artifacts/interface_contract.md#135-task-009-중단-복구-api--2026-10-30).

- **승인·리뷰·검증 경계:** Human이 TASK-008 게시와 TASK-009 후속 개발을 지시했고, 세션 종료와 TASK-009 commit/push도 추가 요청했다. 게시/마감은 최종 구현 승인으로 확대하지 않으며 REV-009/010/011 Resolved·TASK-008/009 Human 리뷰 대기다. TASK-004~009 native GAP-004는 검증 면제가 아니며 CI/계정/과금 조회·차단 재시도는 하지 않는다.

- **형상관리 제약:** 소스와 `.cowork` 문서는 별도 커밋·[skip ci]로 관리한다. TASK-009 소스 `e5f3147`은 EV-014 source51개와 commit blob hash가 일치하며 검증/마감 문서212373c와 origin/main 게시 완료다. 인증 정보는 OS keyring에 두고 저장소에 넣지 않는다. 세션 종료 후 새 Task를 자동 착수하지 않는다.

- **2026-10-26 Human 승인:** 「권고 기준선 채택 — 장비·호환 검증은 계획대로 진행」. 상세설계·ADR-008/009·추가 시험 목표·MS 계획을 채택했다. 검증 이월은 면제가 아니다.

- **ADR-010 Accepted / 2026-10-26 Human 승인:** MCAP 0.26.0 채택. 0.25 선택 zstd 경로의 보안 공지를 해소했다. Tokio는 1.51.6 patch. 최종 audit 취약점/unsound 0, `paste` 유지보수 경고 1(GAP-005).

- Gate 1·2 Pass, ADR-003~006 기준선 유지. 1 GB/s 원본+10만 소형/s·32생산자/16파일/128채널·R1 2 GiB 및 512 MiB/4 GiB 비교는 조건부 목표이며 제품 기본값·달성 결과가 아니다.
- **ADR-007 Accepted:** dot-probe 내장형 Ports and Adapters + 제한된 생산자/소비자 파이프라인. 별도 데몬은 첫 범위에 넣지 않는다. 프로세스 장애까지 격리하지 않음.
- **클라이언트 형태 확정:** Rust API + CLI, dot-probe 기존 화면 연동. 명령/통신 상세도 ADR-009 기준선 채택 완료.
- **실제 검증 장비 미확정:** Human이 잘못 클릭한 ‘확정’을 정정했다. CPU/SSD/OS/제어 주기/UPS는 TASK-022에서 수집한다. R1은 참조 조건이다.
- RT 무대기·별도 중요도·중요 우선 기본값, 보존 모드 옵션과 처리/저장 예산 분리 유지. FR-018은 선택 옵션·기본 off, 전체 약3초는 신호 발생→차단 예산.
- 플랫폼 Tier 1/2 유지. affinity는 간섭 감소이며 완전 격리/보편적 성능 보장 아님. Tier 2 고급 기능 미지원은 capability로 표시한다.
- FR-015는 제안. TASK-018은 별도 범위 승인 전 착수하지 않고 Must 완료에 몰래 넣지 않는다.

## 활성 Task 요약

| Task | 제목 | 담당 | 상태 | 갱신 | 다음 액션 |
|---|---|---|---|---|---|
| TASK-004 | 원본 풀·수명·무해제 반환 | AI | In Progress | 2026-10-28 | 로컬/승인 유지, 사용량 소진으로 native CI 후속 검증 |
| TASK-005 | 분배·출력 격리·예산 | AI | In Progress | 2026-10-28 | 선행 CI 대기 중 개발 진행 승인; EV-004 로컬 Pass·Human 리뷰 승인 |
| TASK-006 | 완료 추적·명시 sync·종료 | AI | In Progress | 2026-10-29 | EV-009 RV-001~005 Human 승인·로컬 Pass; native 후속 검증 |
| TASK-007 | MCAP 봉투·writer·인덱스 | AI | In Progress | 2026-10-30 | EV-011 로컬 Pass·Human 승인 완료, native 후속 검증 |
| TASK-008 | 회전·순환·이벤트 보호 | AI | In Progress | 2026-10-30 | EV-013 로컬 Pass·소스14fd1a4 게시, REV-009/010 Resolved → Human 리뷰/native |
| TASK-009 | 중단 복구·IO 장애 | AI | In Progress | 2026-10-30 | EV-014 로컬 Pass·REV-011 Resolved → Human 리뷰/native·장비 |

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
| [REV-011](../05_verification/reviews/REV-011_TASK-009.md) | TASK-009 | 2026-10-30 | 복구·IO 장애 자가 리뷰 / Conditional (P2 1, 로컬 해결) | Resolved | RV-015 EV-014 수정·회귀 Pass, 소스e5f3147·Human 리뷰 대기 |
| [REV-012](../05_verification/reviews/REV-012_TASK-009_DEEP_REVIEW.md) | TASK-009 | 2026-10-30 | 심층 코드리뷰 / Pass (P3 2) | Open | RV-016(방어적 슬라이싱)·RV-017(포맷팅 최적화) 제안, 제품 코드 수정 없이 Open 등록 |

## 다음 시작점

0. **현재 형상 기준:** TASK-008 조치 소스14fd1a4·문서1f2f44d를 origin/main에 게시했고 source49개/EV-013 blob hash를 확인했다([게시 근거](imported_context/task_008_review_fixes_2026-10-30/push.json)). TASK-009 소스 `e5f3147`은 EV-014 source51개와 일치하며 검증/마감 문서212373c와 origin/main 게시 완료다([게시 근거](imported_context/task_009_2026-10-30/push.json)). 최신 검증 hash/명령/로그는 EV-014다. CI/계정/과금 조회·차단 재시도는 하지 않는다.

1. **TASK-009 구현 리뷰:** [TASK-009](../04_implementation/tasks/TASK-009.md)·[REV-011](../05_verification/reviews/REV-011_TASK-009.md)·[REV-012](../05_verification/reviews/REV-012_TASK-009_DEEP_REVIEW.md)·[EV-014](../05_verification/verification_evidence.md#ev-014)를 Human이 최종 리뷰한다. TASK-008 REV-009/010도 최종 구현 리뷰 대기다. 구현 승인 전 Resolved/Open 리뷰를 Closed로 바꾸지 않는다. 동일 source의 성공 시험은 반복하지 않는다.
2. TASK-004~007은 구현·로컬 Pass·Human 승인 완료, TASK-008/009는 로컬 Pass·Human 리뷰 대기다. native GAP-004는 모두 잔여이며 후속 개발은 Human 지시로 진행했다. 여건 확보 후 current source로 6 contracts+audit+raw-memory(보존/복구 Miri 포함)를 확인한다. 계정 조회/차단 재시도는 하지 않는다.
3. 다음 신규 개발 후보는 TASK-010 시간·채널 조회/active snapshot(Planned·미착수)다. TASK-009 Human 리뷰 후 고정 safe offset·cursor/page bytes/count·원본 descriptor/read pin·취소/실제 IO 종료 후 반환과 RV-012 고립 회수 조건을 연결한다. 실제 장비 검증 TASK-022는 후속이다.

심층 제안은 EV-008 판단을 유지한다. 채널 탐색/LUT·RecordSlot 정렬은 장비 측정 후 비교하고, 미등록 채널은 이미 InvalidInput이므로 그 사례에 근거한 CaptureError 확대는 보류한다. poll_shutdown 반복 구동은 rustdoc에 보강했다.

## 새 세션 재개 메모

- TASK-009·REV-011·EV-014를 먼저 읽고 Human 구현 리뷰부터 재개한다. 검증 소스51개와 같으면 성공 시험을 반복하지 않는다.
- 개발1.96.0·MSRV1.85·nightly-2026-10-26 Miri 고정, dependency/lock·RT 무할당/무대기·private unsafe 경계를 유지한다. 전역 rustup default는 바꾸지 않았다.
- 완료/종료·writer 배치/재시도 의미는 [API §13](../03_design_artifacts/interface_contract.md#13-상세-api-설계안)·TASK-006~008을 기준으로 보존한다. source/sink host 메모리와 OS cache는 logger 예약 밖이다.
- RV-012 pin 유실 위험은 TASK-010 후속이다. 만료/Drop을 실제 읽기 종료로 추정하지 않으며 실제 IO 종료 확인 후 release한다. 자동 강제 회수는 도입하지 않았다.
- 자동 startup 발견/기존 namespace 재개·active reader·worker/StorageDegraded는 미구현 후속이다. native/장비·TC 전체·MS-002/003·Gate4/5 완료를 선언하지 않는다.

## 선택 로딩

- [도메인](../03_design_artifacts/domain_model.md), [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안), [데이터](../03_design_artifacts/data_model.md), [CLI](../03_design_artifacts/ui_spec.md).
- [MS 목록](../04_implementation/milestone_registry.md): 7개 MS, [Task 목록](../04_implementation/task_registry.md): 26개 Task(1개 선택). 선행조건·변경 대상·첫 실패/DoD·상대 인일 추정(계획 승인, 납기 확정 아님).
- [요구 추적표](../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트): INT 1 + FR 18 + NFR 7 + CON 7 = 33행. MS별 구현/테스트 체크·코드/EV 연결. TC-001의 TASK-002 하위 EV-001 로컬/native, TC-002 소형 EV-002 로컬/native·원본 EV-003 로컬, TC-003 분배 EV-004·완료/제어 EV-005 로컬 Pass; 후속/native/장비 시험 잔여.
- [2026-10-26 조사](imported_context/research_2026-10-26_detailed_design.md): 공식 자료와 dot-probe 코드 관측, 기술 후보/사실/미검증 분리.
- 이전 결정 근거가 필요할 때만 ADR-003~006·요구 §12~17·Story 상세·개인 로그를 추가로 읽는다.

## 이월 백로그

다음 T = T3 / 다음 B = B1. 트리거 감시는 AI 책임이다.

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| T1 | ROS 2 파일 호환·토픽 연동 필요/범위 검토 | dot-probe 첫 적용 완료가 확인된 때 | INT-001 / OQ-008 / TASK-026 | 2026-10-25 Human Q5 |
| T2 | 공개 전환 전 라이선스·GitHub 운영정책·공개할 이력/로그 검토 | 공개 전환 또는 첫 외부 배포를 요청할 때 | INT-001 / 공개 준비 | 2026-10-28 Human 논의 |

범위 밖 백로그 B: 없음. 현재 T1/T2는 미도래. OQ를 T/B에 중복 등록하지 않는다.

## 열린 결정과 검증 공백

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| OQ-002 | 실제 장비·제어 주기/부하·UPS 미정. 초기 정량 목표 승인 | TASK-022에서 환경·수락 프로필 확보, TC-011·012 실물 검증 |
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·실제 앱/장비 수명 수락은 미검증 | TASK-004~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬/native CI 확인(EV-001), 실제 DB·장비 호환 잔여 | TASK-019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증, TASK-018은 별도 범위 승인 후 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·004·005 | 장비·후속 제품 TC·TASK-004~009 native CI 사용량 소진에 따른 미검증·paste 경고. GAP-003 Closed, TASK-003 native Pass 이력 유지 | [검증 근거](../05_verification/verification_evidence.md) 참조 |
| GAP-006 | EV-006 자가 리뷰 RV-001~004는 EV-007 수정·로컬 회귀 Pass로 Closed | Human 승인 완료·native GAP-004 후속, 기존 이력 보존 |
| GAP-007 | RV-005는 EV-009 로컬 수정/회귀 Pass·Human 승인으로 Closed | native는 후속, 수정 전 EV-008 이력 보존 |
| RV-015 | TASK-009 최종 sync 중 취소 반례를 EV-014에서 수정·REV-011 Resolved | Human 최종 구현 리뷰/native·장비 대기 |
| RV-006~010 | EV-011 로컬 수정·Human 승인 완료, REV-008 Closed | native GAP-004·장비 성능은 잔여 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-10-30 #003: REV-010 조치·EV-013 소스/문서 게시 후 TASK-009 복구 구현·EV-014 Pass·REV-011 Resolved, pin/reader 후속 유지 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_003.md).
- 2026-10-30 #002: TASK-008 심층 리뷰·REV-010 Open 등록·과거 리뷰 안건 전수 감사 — [리뷰 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_002.md).
- 2026-10-30 #001: TASK-007 승인·REV-008 Closed, TASK-008 EV-012 Pass·REV-009 Resolved·소스 7388145 로컬 커밋·세션 종료 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_001.md).

이전 완료 이력은 [상태 아카이브](state_archive.md)에 보존했다.

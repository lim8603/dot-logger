# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 수집·분배·완료/제어 coordinator 구현·로컬 검증, 실제 파일/worker 미구현 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — TASK-006 자가 리뷰 결함 수정·EV-007 로컬 회귀 Pass |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-002 In Progress — 제한된 수집 코어(MS-001 Done) |
| 활성 Task | TASK-006 In Progress — RV-001~004 수정·로컬 회귀 Pass, 수정분 Human 리뷰/native 대기 |
| 상태 | Yellow — 로컬 수정/회귀 완료, 수정분 리뷰·사용량 소진에 따른 native CI 차단·장비 검증 잔여 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-10-28 / AI |
| 최근 세션 | [2026-10-28 #003](../members/lim8603/workspace/session_logs/session_2026-10-28_003.md) 종료 — 자가 리뷰 수정·회귀·코드 게시 완료, 수정분 리뷰/native 이월 |

## 현재 결정과 제약

- **2026-10-28 후속 수정·마감:** Human의 수정/회귀 지시 후 RV-001~004를 수정했다. [EV-007](../05_verification/verification_evidence.md#ev-007): 신규9개·두 Rust 각16 features/consumer/예제/clippy/rustdoc/fmt·RT0/0·Miri78×8 seeds Pass, source37개(README 포함) 고정. 이어 Human 「커밋 푸쉬하고 세션 마무리」로 코드 `d57c2e3`을 origin/main에 게시하고 원격 ref·검증37개 blob 일치를 확인했다. 세션 마감 문서는 별도 커밋으로 관리한다. GAP-006 로컬 Closed, 수정분 Human 구현 리뷰/native는 이월하며 CI/과금 조회는 하지 않았다. 커밋 요청을 구현 리뷰 승인으로 대신하지 않는다.

- **자가 리뷰 이력:** [EV-006](../05_verification/verification_evidence.md#ev-006)은 source36개가 EV-005와 일치하는 수정 전 대상에서 P1 반례3개를 확인한 기록이다. 과거 실패/승인 이력은 보존하고 현 구현의 근거는 EV-007을 사용한다.

- **2026-10-28 마감:** TASK-006 Human 승인·commit/push·세션 종료 요청. 코드 `c98b7fa`는 검증 source36개와 일치한다. source 변경 없는 승인/마감 문서는 분리 커밋했다. gh 브라우저 인증·Git credential helper 연결 후 pull --ff-only 및 push를 완료했고 원격 main `136fe96`과 일치함을 확인했다. 인증 정보는 OS keyring에 보관되며 저장소에 넣지 않는다.

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
| TASK-006 | 완료 추적·명시 sync·종료 | AI | In Progress | 2026-10-28 | EV-007 수정·로컬 회귀 Pass; 수정분 Human 리뷰/native 대기 |

## 다음 시작점

0. **게시 완료:** TASK-006 후속 수정 코드 `d57c2e3`을 origin/main에 push하고 원격 ref·EV-007 source37개 blob 일치를 확인했다. 코드와 `.cowork` 마감 기록은 분리 커밋한다. `[skip ci]`를 유지하고 CI/과금 상태는 재확인하지 않는다. 별도 추가된 `.agents/skills/code-review/SKILL.md`는 이번 작업 커밋에서 제외·보존했다.

1. **수정분 리뷰:** [EV-007](../05_verification/verification_evidence.md#ev-007)의 RV-001~004 수정과 local regression 결과를 Human이 리뷰한다. 정식 Red3개 후 수정·신규9개/기존 경로 Pass, 코드 `d57c2e3`의 source37개 hash가 검증 대상이다. EV-005/수정 전 EV-006은 이력으로 보존한다. 소스가 같으면 같은 시험은 반복하지 않는다.
2. TASK-004/005는 구현·로컬 Pass·Human 승인 완료, native 미검증으로 In Progress다. Human이 사용량 전량 소진·계정 재확인 불필요를 확인했고 TASK-005/006 개발 진행을 지시했다. 검증 면제가 아니며 CI 여건 확보 후 current source를 고정해 6 contracts+audit+raw-memory8 job을 확인한다. 계정 조회·차단 재시도는 하지 않는다.
3. 다음 신규 개발 후보는 TASK-007 MCAP 봉투·writer·인덱스(Planned·미착수)다. TASK-006 수정분 리뷰 후 진행할 수 있으며 writer 대역을 실제 파일 IO/명시 sync·회전/복구 단계와 연결한다. 공개 전환·라이선스·GitHub 운영정책은 T2로 별도 판단한다.

## 새 세션 재개 메모

- 이 문서 → TASK-006 상세·EV-007·TC-003 → 필요한 completion/control/capture 코드를 읽는다. RV-001~004 수정·로컬 회귀 완료, 수정분 Human 리뷰와 native가 남았다. TASK-004/005 및 이전 TASK-006 승인은 이력으로 유지한다.
- 개발1.96.0·MSRV1.85·dependency/lock 유지. private pool/storage unsafe 경계 유지. Miri nightly-2026-10-26은 검증 전용이며 전역 default는 변경하지 않았다. 현 source37개 hash·환경/명령/log 근거는 EV-007이다.
- producer별 완료 보호량 R=max(small/raw Important slots), W≥R다. Normal은 다음 R개 cyclic slot의 Empty/Terminal 상태를 확인해 보호하며 W=R은 Important 전용이다. 기존 consumer 예제는 Important16에 맞춰 window32/work_per_poll32로 정렬했다. 실제 RT p99는 미검증이다.
- `new_tracked`는 producer별 completion window를 입력 수락 전에 단발 예약한다. pending/Updating slot을 덮지 않고 CompletionCapacity로 무대기 거절한다. raw 거절/생략은 같은 loan이며 admission/loan call의 SeqCst 표시가 close/cutoff 동결을 순서화한다.
- Completion은 boundary 확인과 failed/dropped/unknown/expired를 분리한다. 고정 range/page/query permit·deadline/cancel wait와 완료 이력 age/window, retired terminal counters를 제공한다. query는 sync/출력 취소를 실행하지 않는다.
- Router는 유한 feedback으로 Busy 결과를 보존하고 source memory 분할·pool layout·초기 completion route를 대조한다. 원본은 기존 guard 하나를 공유하며 distinct raw 물리 예약·output 보유 원인/복사·completion/control metadata를 분리한다. 일반 guard 무보고 Drop은 Unknown, 오래된 Drop은 새 generation을 덮지 않는다.
- ControlOwner는 유일 controller lease·session/epoch/slot/generation·ordinary/전용 shutdown slots를 가진다. 명시 barrier만 writer 대역에 넘기고 같은 target의 sync IO를 직렬화한다. 만료 shutdown sync는 실제 IO/loan/feedback와 선행 일반 sync 종료 후 유한 cursor로 미확인 Synced를 Unknown으로 정리한다. 일반 요청 만료는 기록을 바꾸지 않는다. Closed의 is_success와 실패/불명/기한 보고를 별도로 확인한다.
- 실제 파일/OS sync·backend worker 기동은 TASK-007 이후다. 앱 thread join은 scoped host 책임이다. 실제 dot-probe static spawn 연결은 TASK-026, 장비 프로필은 TASK-022이며 T1은 미도래다. TC 전체·MS-002·Gate4/5 완료를 선언하지 않는다.

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
| GAP-001·002·004·005 | 장비·후속 제품 TC·TASK-004/005/006 CI 사용량 소진에 따른 미검증·paste 경고. GAP-003 Closed, TASK-003 native Pass 이력 유지 | [검증 근거](../05_verification/verification_evidence.md) 참조 |
| GAP-006 | EV-006 자가 리뷰 RV-001~004는 EV-007 수정·로컬 회귀 Pass로 Closed | 수정분 Human 리뷰·native GAP-004 후속, 기존 이력 보존 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-10-28 #003: 자가 리뷰 RV-001~004 수정·EV-007 회귀·코드 d57c2e3 게시·세션 종료, 수정분 Human 리뷰/native 이월 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-28_003.md).
- 2026-10-28 #002: TASK-006 구현·EV-005 로컬 Pass·Human 승인·커밋·push·세션 종료, native 이월 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-28_002.md).
- 2026-10-28 #001: Human 사용량 소진 확인·TASK-005 선행 CI 대기 중 착수 지시, 구현·EV-004 로컬 Pass·Human 리뷰 승인 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-28_001.md).

이전 완료 이력은 [상태 아카이브](state_archive.md)에 보존했다.

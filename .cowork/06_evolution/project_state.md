# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 수집·분배·완료/제어·MCAP 파일 writer 구현·로컬 검증, 회전/복구·worker 후속 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — TASK-007 리뷰 RV-006~010 수정·EV-011 로컬 Pass |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-003 In Progress — 파일 저장(MS-002 native 검증 이월) |
| 활성 Task | TASK-007 In Progress — RV-006~010 로컬 수정·EV-011 Pass, 최신 Human 구현 리뷰/native 대기 |
| 미처리 리뷰 | [REV-008](../05_verification/reviews/REV-008_TASK-007_RV-006-010.md) Resolved — P2 3건, P3 2건 로컬 수정/EV-011 Pass, 최신 구현 Human 승인 대기 |
| 상태 | Yellow — TASK-007 로컬 Pass, Human 리뷰·native CI 차단·실장비/후속 파일 기능 잔여 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-10-29 / AI |
| 세션 상태 | 종료 — 리뷰 절차 공식화·전체 리뷰 이력(REV-001~008) 복원 완료, REV-008 Human 승인 대기 |
| 최근 세션 | [2026-10-29 #004](../members/lim8603/workspace/session_logs/session_2026-10-29_004.md) 종료 — 코드리뷰 절차 공식화·REV-001~008 복원·문서 게시 |

## 현재 결정과 제약

- **2026-10-29 리뷰 절차 공식화:** Human 「프로젝트 리뷰처리 절차를 공식화 하려고해... 승인」에 따라 `05_verification/reviews/` 디렉터리를 신설하고 과거 리뷰 내역(REV-001~003)을 복원했다. 리뷰 세션의 코드 수정 금지(Read-Only), REV 문서 자산화, 리뷰 상태 추적(`Open` → `In Progress` → `Resolved` → `Closed`), 브리핑 시 미처리 리뷰 상시 보고 규칙을 프로토콜 및 진입점(AGENTS/GEMINI)에 공식 반영했다.

- **2026-10-29 게시·마감:** Human 「커밋 푸쉬하고 세션 마감해줘」에 따라 소스 `71b3e2c`를 origin/main에 게시했다. EV-011 source46개와 해당 commit blob의 SHA-256이 전부 일치한다. 소스/문서는 분리 커밋·[skip ci]로 관리하며 마감 문서를 별도 게시한다. 이 요청을 TASK-007 최신 구현 Human 리뷰 승인으로 확대하지 않는다.

- **2026-10-29 리뷰 수정:** Human 「코드리뷰 받은 내역을 확인하고 수정하자는 말이였어」로 RV-006~010 수정 범위·방향을 승인했다. 배치 Written·진행 prefix, pre-IO SyncWork 반환·공통 Busy 매핑, 예산 내 쓰기 버퍼와 report-only 재시도를 구현했다. [EV-011](../05_verification/verification_evidence.md#ev-011) 로컬 Pass이며 최신 구현 Human 리뷰/native·장비는 대기다.

- **2026-10-29 승인·착수:** Human 「승인」으로 TASK-006 RV-001~005 최신 수정분을 승인하고 native 이월 상태의 TASK-007 진행을 승인했다. TASK-006/EV-009 로컬 Pass·native GAP-004는 유지하며 MCAP 봉투/writer/인덱스를 구현하고 EV-010 로컬 검증을 마쳤다. TASK-007 Human 리뷰/native는 대기이며 당시 착수 승인은 commit/push 지시가 아니었다.

- **2026-10-29 후속 수정:** [EV-008](../05_verification/verification_evidence.md#ev-008)의 RV-005 P1을 Human 수정 지시 후 [EV-009](../05_verification/verification_evidence.md#ev-009)에서 해결했다. 일반 sync 진입/게시와 shutdown을 단발 원자적 표시·RAII·SeqCst로 순서화했다. 정식 반례2개 Red 후 수정·private3개, 두 Rust 각16 features/consumer/예제/clippy/rustdoc/fmt·Miri81×8·RT0/0 Pass, source38개 고정이다. GAP-007 로컬 Closed, 최신 수정분 Human 승인은 완료됐고 native는 잔여다. TASK-006 보고에 RV-001~005 수정 반영사항을 함께 정리한다. commit/push·CI/과금 조회는 하지 않았다.

- **이전 수정/게시 이력:** RV-001~004의 EV-007 로컬 회귀와 코드 d57c2e3 게시 이력은 [TASK-006](../04_implementation/tasks/TASK-006.md)에 보존했다. GAP-006/007은 로컬 수정 범위에서 Closed, TASK-006 최신 수정분 Human 승인은 완료됐고 native GAP-004는 잔여다. 현재 게시 소스 `71b3e2c`는 EV-011을 기준으로 검토한다.

- **자가 리뷰 이력:** [EV-006](../05_verification/verification_evidence.md#ev-006)은 source36개에서 P1 반례3개, EV-008은 EV-007과 같던 source37개에서 RV-005를 확인한 수정 전 기록이다. 실패/승인 이력은 보존하고 TASK-006 수정 근거는 EV-009, TASK-007 현재 근거는 EV-011을 사용한다.

- **형상관리 제약:** 소스와 `.cowork` 문서는 별도 커밋으로 관리한다. 이번 commit/push는 Human의 명시 지시로 수행했고, 세션 종료만으로 다른 게시나 구현 승인을 추정하지 않는다. 인증 정보는 OS keyring에 두고 저장소에 넣지 않는다.

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
| TASK-007 | MCAP 봉투·writer·인덱스 | AI | In Progress | 2026-10-29 | EV-011 RV-006~010 수정·로컬 Pass → 최신 구현 Human 리뷰/native 대기 |

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
| [REV-008](../05_verification/reviews/REV-008_TASK-007_RV-006-010.md) | TASK-007 | 2026-10-29 | MCAP 저장 구현 리뷰 / Conditional (P2 3, P3 2) | Resolved | RV-006~010 EV-011 로컬 수정 Pass, commit `71b3e2c`, 최신 구현 Human 승인 대기 |

## 다음 시작점

0. **현재 게시 기준:** TASK-006 RV-005와 TASK-007/리뷰 수정 소스는 `71b3e2c`로 origin/main에 게시했다. EV-011 source46개와 commit blob이 일치하며 마감/근거 문서는 별도 docs(cowork) 커밋으로 관리한다. native CI는 [skip ci]로 자동 실행하지 않았고 CI/계정/과금 조회·차단 재시도를 하지 않는다.

1. **TASK-007 최신 수정분 Human 구현 리뷰:** [TASK-007](../04_implementation/tasks/TASK-007.md)·[EV-011](../05_verification/verification_evidence.md#ev-011)·[#003 로그](../members/lim8603/workspace/session_logs/session_2026-10-29_003.md)를 확인한다. RV-006~010 로컬 수정은 완료했고 최신 구현 승인은 아직 받지 않았다. 저장 통합21개·private Busy2개, 두 Rust 각16 features/2-record consumer→finish→sync→shutdown/예제·clippy/rustdoc/fmt·allocator7, Miri83×8=664 Pass다. source46개 hash/명령/로그는 EV-011에 고정했으므로 소스가 같으면 같은 시험을 반복하지 않는다.

2. TASK-004~006은 구현·로컬 Pass·Human 승인 완료, native 미검증으로 In Progress다. TASK-007 개발 진행도 native 이월 상태로 Human 승인됐다. GAP-004는 면제가 아니며 여건 확보 후 current source로 6 contracts+audit+raw-memory를 확인한다. 계정 조회/차단 재시도는 하지 않는다.
3. 다음 신규 개발 후보는 TASK-008 회전·순환·이벤트 보호(Planned·미착수)다. TASK-007 리뷰 후 NeedRotation을 실제 회전/보존·다중 세그먼트 sync·directory metadata sync와 연결한다. TASK-009 복구/장애·TASK-010 active reader·TASK-022 장비 검증은 후속이다.

심층 제안은 EV-008 판단을 유지한다. 채널 탐색/LUT·RecordSlot 정렬은 장비 측정 후 비교하고, 미등록 채널은 이미 InvalidInput이므로 그 사례에 근거한 CaptureError 확대는 보류한다. poll_shutdown 반복 구동은 rustdoc에 보강했다.

## 새 세션 재개 메모

- 이 문서 → [#003 수정 로그](../members/lim8603/workspace/session_logs/session_2026-10-29_003.md) → TASK-007 상세·EV-011·TC-004를 읽는다. TASK-006 RV-001~005는 Human 승인, TASK-007 RV-006~010 로컬 수정은 완료됐고 최신 구현 리뷰/native·장비가 잔여다.
- 개발1.96.0·MSRV1.85·dependency/lock·private unsafe 경계를 유지한다. Miri nightly-2026-10-26은 검증 전용이며 전역 default는 변경하지 않았다. 현 source46개 hash·명령/log 근거는 EV-011이다.
- producer별 완료 보호량 R=max(small/raw Important slots), W≥R다. Normal은 다음 R개 cyclic slot의 Empty/Terminal 상태를 확인해 보호하며 W=R은 Important 전용이다. 기존 consumer 예제는 Important16에 맞춰 window32/work_per_poll32로 정렬했다. 실제 RT p99는 미검증이다.
- `new_tracked`는 producer별 completion window를 입력 수락 전에 단발 예약한다. pending/Updating slot을 덮지 않고 CompletionCapacity로 무대기 거절한다. raw 거절/생략은 같은 loan이며 admission/loan call의 SeqCst 표시가 close/cutoff 동결을 순서화한다.
- Completion은 boundary 확인과 failed/dropped/unknown/expired를 분리한다. 고정 range/page/query permit·deadline/cancel wait와 완료 이력 age/window, retired terminal counters를 제공한다. query는 sync/출력 취소를 실행하지 않는다.
- Router는 유한 feedback으로 Busy 결과를 보존하고 source memory 분할·pool layout·초기 completion route를 대조한다. 원본은 기존 guard 하나를 공유하며 distinct raw 물리 예약·output 보유 원인/복사·completion/control metadata를 분리한다. 일반 guard 무보고 Drop은 Unknown, 오래된 Drop은 새 generation을 덮지 않는다.
- ControlOwner는 유일 controller lease·session/epoch/slot/generation·ordinary/전용 shutdown slots를 가진다. 명시 barrier만 writer 대역에 넘기고 같은 target의 sync IO를 직렬화한다. 만료 shutdown sync는 실제 IO/loan/feedback와 선행 일반 sync 종료 후 유한 cursor로 미확인 Synced를 Unknown으로 정리한다. 일반 요청 만료는 기록을 바꾸지 않는다. Closed의 is_success와 실패/불명/기한 보고를 별도로 확인한다.
- SyncSubmission은 일반 sync의 진입~게시/오류 반환을 단발 CAS·RAII로 표시한다. close 전 열림을 본 요청이 끝나기 전에는 cutoff 동결/Closed를 확정하지 않는다. 동시 일반 요청 Busy·close 이후 Closing, 실제 guard/만료 정리 계약은 유지한다. test-only 스케줄러는 스레드별 cfg(test)다.
- TASK-007은 empty File/SyncSink를 일반 경로에서 구동한다. write_batch는 호출자 guard를 chunk target에 따라 묶고 완전 flush 뒤 Written을 보고한다. BatchWriteError의 appended/written/reported prefix로 중복 쓰기를 피하고 report만 재시도한다. write_buffer_bytes(0=direct)는 공유 예산에 포함하고 seek/flush 때 배출, Drop은 IO 없이 폐기한다. 인덱스 쌍은 min(records,chunks×channels)로 제한한다. sync 거절/Busy는 원래 work를 반환하고 실제 IO 실패만 Failed다. 다른 세그먼트의 pending/만료 범위를 새 파일 sync로 확인하지 않는다.
- worker 기동/앱 thread join은 scoped host·TASK-026 후속이다. 장비 프로필은 TASK-022이며 T1 미도래, 회전/복구/active reader·제품 압축·OS 교환은 후속이다. TC 전체·MS-002/003·Gate4/5 완료를 선언하지 않는다.

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
| GAP-006 | EV-006 자가 리뷰 RV-001~004는 EV-007 수정·로컬 회귀 Pass로 Closed | Human 승인 완료·native GAP-004 후속, 기존 이력 보존 |
| GAP-007 | RV-005는 EV-009 로컬 수정/회귀 Pass·Human 승인으로 Closed | native는 후속, 수정 전 EV-008 이력 보존 |
| RV-006~010 | EV-011 수정·로컬 회귀 Pass, 로컬 범위 해결 | [TASK-007](../04_implementation/tasks/TASK-007.md)의 최신 구현 Human 리뷰/native·TASK-022 장비 성능은 잔여 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-10-29 #004: 리뷰 절차 공식화·`reviews/` 신설·REV-001~003 복원·문서/진입점 갱신 및 커밋 푸시 마감 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-29_004.md).
- 2026-10-29 #003: RV-006~010 수정·EV-011 로컬 회귀 Pass·소스 71b3e2c 게시·세션 종료, 최신 구현 Human 리뷰/native·장비 이월 — [수정 로그](../members/lim8603/workspace/session_logs/session_2026-10-29_003.md).
- 2026-10-29 #002: 진행현황 점검·TASK-007 AI 코드리뷰 Conditional(P1 없음). RV-006~010 문제/해결책 제안을 기록했고 제품 코드는 미수정이다 — [리뷰 로그](../members/lim8603/workspace/session_logs/session_2026-10-29_002.md).

이전 완료 이력은 [상태 아카이브](state_archive.md)에 보존했다.

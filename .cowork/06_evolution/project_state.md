# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 소형·원본 ingress 구현·로컬 검증, 분배/완료/저장 미구현 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — TASK-004 구현·로컬 검증 완료, Human 리뷰 승인·native CI 대기 |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-002 In Progress — 제한된 수집 코어(MS-001 Done) |
| 활성 Task | TASK-004 In Progress — 원본 풀·수명·무해제 반환 |
| 상태 | Yellow — TASK-004 EV-003 로컬 Pass; Human 리뷰 승인·변경분 native CI·후속 런타임/실장비 검증 잔여 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-10-27 / AI |
| 최근 세션 | [2026-10-27 #003](../members/lim8603/workspace/session_logs/session_2026-10-27_003.md) — TASK-004 원본 풀·EV-003, Human 리뷰 승인·native CI 대기 |

## 현재 결정과 제약

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
| TASK-004 | 원본 풀·수명·무해제 반환 | AI | In Progress | 2026-10-27 | 구현·EV-003 로컬 Pass; Human 리뷰 승인·변경분 native CI 대기 |

## 다음 시작점

1. TASK-004 Human 구현 리뷰는 2026-10-27 「승인하고, 커밋 푸쉬하구 오늘 세션은 여기서 마무리하자」로 승인됐다. EV-003의 24개 source/manifest hash와 현재 파일이 일치한다.
2. 승인된 코드·문서를 분리 커밋·게시하고 변경분 native CI(기존6 OS/Rust + audit + raw-memory =8 job)를 확인한다. GAP-004는 해당 결과까지 Open이다.
3. CI 완료 결과·미완료를 동기화하고 세션을 마감한다. 다음 작업은 TASK-005이며 오늘 착수하지 않는다. 실제 dot-probe adapter는 TASK-026, 장비는 TASK-022, FR-015는 별도 범위다.

## 새 세션 재개 메모

- 이 문서 → TASK-004·EV-003을 읽는다. 구현 검토는 pool/mod.rs·storage.rs·capture/mod.rs, 수명 예제는 examples/scoped_capture.rs다. 공개 실패/소유권 계약은 인터페이스 §13·도메인 §4~5를 따른다.
- 개발 1.96.0·MSRV 1.85·외부 dependency/lock 유지. Miri nightly-2026-10-26은 검증 전용 추가이며 전역 default는 변경하지 않았다. Ubuntu 로컬·Miri는 PREEMPT_RT/실장비 판정이 아니다.
- CaptureOwner는 소형/원본 ingress만 구성한다. borrowed Loan·RawRecord는 owner보다 오래 살 수 없고 bytes borrow는 guard에 묶인다. generation/state atomic·슬롯 ReturnPending이 반환의 기준이며 capacity1 알림 포화에도 한정 스캔으로 회수한다. receipt는 입력 인수다. fanout·완료 window·파일·durability·실제 shutdown은 후속 Task다.
- workspace unsafe forbid 유지, dot-logger package만 deny/private pool-storage allow로 좁은 예외를 둔다. Sync·UnsafeCell Safety 경계와 Miri/allocator 검증은 Human 리뷰 대상이다.
- dot-probe의 현재 static spawn에는 borrowed 핸들 직접 주입이 불가하다. owner를 가진 관리 host+scoped RT worker 수명 prototype은 두 Rust에서 실행했다. 실제 run entry point/생성부 조정과 연결 검증은 TASK-026이며 dot-probe 파일은 수정하지 않았다. T1 미도래다.
- 기존 설계·TDD·MCAP 0.26·TASK-003 승인은 재질문하지 않는다. TASK-004는 리뷰/native 대기, TC-002 전체 미완료·실제 장비 미확정·FR-015 별도 제안을 유지한다.

## 선택 로딩

- [도메인](../03_design_artifacts/domain_model.md), [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안), [데이터](../03_design_artifacts/data_model.md), [CLI](../03_design_artifacts/ui_spec.md).
- [MS 목록](../04_implementation/milestone_registry.md): 7개 MS, [Task 목록](../04_implementation/task_registry.md): 26개 Task(1개 선택). 선행조건·변경 대상·첫 실패/DoD·상대 인일 추정(계획 승인, 납기 확정 아님).
- [요구 추적표](../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트): INT 1 + FR 18 + NFR 7 + CON 7 = 33행. MS별 구현/테스트 체크·코드/EV 연결. TC-001의 TASK-002 하위 EV-001 로컬/native, TC-002 소형 EV-002 로컬/native·원본 EV-003 로컬 Pass; 후속/장비 시험 잔여.
- [2026-10-26 조사](imported_context/research_2026-10-26_detailed_design.md): 공식 자료와 dot-probe 코드 관측, 기술 후보/사실/미검증 분리.
- 이전 결정 근거가 필요할 때만 ADR-003~006·요구 §12~17·Story 상세·개인 로그를 추가로 읽는다.

## 이월 백로그

다음 T = T2 / 다음 B = B1. 트리거 감시는 AI 책임이다.

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| T1 | ROS 2 파일 호환·토픽 연동 필요/범위 검토 | dot-probe 첫 적용 완료가 확인된 때 | INT-001 / OQ-008 / TASK-026 | 2026-10-25 Human Q5 |

범위 밖 백로그 B: 없음. 현재 T1은 미도래. OQ를 T/B에 중복 등록하지 않는다.

## 열린 결정과 검증 공백

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| OQ-002 | 실제 장비·제어 주기/부하·UPS 미정. 초기 정량 목표 승인 | TASK-022에서 환경·수락 프로필 확보, TC-011·012 실물 검증 |
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·실제 앱/장비 수명 수락은 미검증 | TASK-004~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬/native CI 확인(EV-001), 실제 DB·장비 호환 잔여 | TASK-019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증, TASK-018은 별도 범위 승인 후 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·004·005 | 장비·후속 제품 TC·TASK-004 native CI·paste 경고. GAP-003 Closed, TASK-003 native Pass 이력 유지 | [검증 근거](../05_verification/verification_evidence.md) 참조 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-10-27 #003: TASK-004 원본 풀·반환 구현·EV-003 로컬/Miri Pass·Human 리뷰 승인/native 대기·GAP-004 변경분 재개 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-27_003.md).
- 2026-10-27 #002: TASK-003 소형 ingress 구현·Human 리뷰 승인·EV-002 로컬/native CI Pass·Done·GAP-004 Closed — [로그](../members/lim8603/workspace/session_logs/session_2026-10-27_002.md).
- 2026-10-27 #001: Human 리뷰 승인, lim8603/dot-logger 비공개 생성·기존 main 게시, native CI 6조합+audit Pass·TASK-002 Done·GAP-004 Closed — [로그](../members/lim8603/workspace/session_logs/session_2026-10-27_001.md).

이전 완료 이력은 [상태 아카이브](state_archive.md)에 보존했다.

# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — 소형·원본 ingress 구현·로컬 검증, 분배/완료/저장 미구현 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — TASK-004 구현·로컬 Pass·Human 승인·게시 완료, native CI 계정 사유 대기 |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-002 In Progress — 제한된 수집 코어(MS-001 Done) |
| 활성 Task | TASK-004 In Progress — 원본 풀·수명·무해제 반환 |
| 상태 | Yellow — TASK-004 native CI는 GitHub 결제/한도로 실행 전 차단; 후속 런타임/실장비 검증 잔여 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-10-27 / AI |
| 최근 세션 | [2026-10-27 #003](../members/lim8603/workspace/session_logs/session_2026-10-27_003.md) 종료 — TASK-004 승인·게시 완료, native CI 계정 사유 대기 |

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
| TASK-004 | 원본 풀·수명·무해제 반환 | AI | In Progress | 2026-10-27 | 구현·Human 승인·게시 완료; 계정 조건 해결 후 native CI 재실행 |

## 다음 시작점

1. Human이 GitHub Billing & plans의 결제/지출 한도 조건을 확인한다. [TASK-004 CI run 40520558735](https://github.com/lim8603/dot-logger/actions/runs/40520558735)은 8개 job 모두 runner/steps 시작 전 차단됐다. 제품/native 시험은 Not Run이며 [EV-003](../05_verification/verification_evidence.md#ev-003--task-004-원본-풀수명반환)·GAP-004에 annotations와 재실행 조건을 보존했다.
2. 계정 조건 해결 후 동일 workflow를 재실행한다. 코드 `9a4f34c`·CI 대상 `f0ce6ee`와 EV-003의24개 source hash가 일치한다. 6 OS/Rust + audit + raw-memory 총8 job 확인 뒤 TASK-004 Done을 판단한다. 구현 리뷰는 2026-10-27 승인됐고 재승인받지 않는다.
3. TASK-004 종료 후 TASK-005 분배·출력 격리·예산으로 이어간다(Planned·미착수). 오늘은 세션 종료 요청에 따라 착수하지 않았다. 실제 앱 연결 TASK-026·장비 TASK-022·FR-015 별도 제안은 유지한다.

## 새 세션 재개 메모

- 이 문서 → TASK-004·EV-003 native 차단 기록을 읽는다. 코드/문서 게시와 세션 마감은 완료, 미완료는 GitHub 계정 조건 해결 후 CI 재실행이다. 소스/테스트를 변경하지 않으면 동일 로컬 시험을 반복하지 않는다.
- 개발 1.96.0·MSRV1.85·외부 dependency/lock 유지. Miri nightly-2026-10-26은 검증 전용이며 전역 default는 변경하지 않았다. 원본 통합17·경계4·신규 compile-fail7·원본 allocator guard1, Miri8 seeds·두 Rust 각16 feature/consumer/예제/clippy/rustdoc/fmt 로컬 Pass다. CI에서는 어떤 제품 시험도 실행되지 않았다.
- CaptureOwner는 소형/원본 ingress를 구성한다. Loan·RawRecord는 owner보다 오래 살 수 없고 bytes borrow는 guard에 묶인다. generation/state atomic·슬롯 ReturnPending이 반환의 기준이며 포화된 1슬롯 알림과 무관하게 한정 스캔으로 회수한다. receipt는 입력 인수다. 분배·완료·파일·실제 shutdown은 후속 Task다.
- workspace forbid·dot-logger package deny/private pool-storage allow와 Safety 경계는 TASK-004 Human 승인 범위다. 기존 설계·TDD·MCAP0.26·TASK-003·TASK-004 승인은 재질문하지 않는다.
- dot-probe의 static spawn에는 직접 borrowed 주입이 불가하다. 관리 host+scoped RT 수명 예제는 두 Rust에서 실행했고 실제 생성부/run entry point 연결은 TASK-026이다. dot-probe 파일은 수정하지 않았고 T1 미도래다. TC-002 전체·실장비·Gate4/5는 미완료다.

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
| GAP-001·002·004·005 | 장비·후속 제품 TC·TASK-004 CI 계정 사유 차단·paste 경고. GAP-003 Closed, TASK-003 native Pass 이력 유지 | [검증 근거](../05_verification/verification_evidence.md) 참조 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-10-27 #003: TASK-004 구현·EV-003 로컬/Miri Pass·Human 승인·게시 완료, CI 계정 사유 차단·세션 종료 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-27_003.md).
- 2026-10-27 #002: TASK-003 소형 ingress 구현·Human 리뷰 승인·EV-002 로컬/native CI Pass·Done·GAP-004 Closed — [로그](../members/lim8603/workspace/session_logs/session_2026-10-27_002.md).
- 2026-10-27 #001: Human 리뷰 승인, lim8603/dot-logger 비공개 생성·기존 main 게시, native CI 6조합+audit Pass·TASK-002 Done·GAP-004 Closed — [로그](../members/lim8603/workspace/session_logs/session_2026-10-27_001.md).

이전 완료 이력은 [상태 아카이브](state_archive.md)에 보존했다.

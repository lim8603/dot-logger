# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — TASK-002 구성 코드·테스트 구현, 수집/저장 런타임 미구현 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — TASK-002 구현·로컬 검증 완료, Human 리뷰 승인·CI 잔여 |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-002 In Progress — 제한된 수집 코어(MS-001 Done) |
| 활성 Task | TASK-002 In Progress / 담당 AI(TASK-001 Done) |
| 상태 | Yellow — TASK-002 로컬 검증 Pass. Human 구현 리뷰 승인(2026-10-27). native CI·런타임/장비 검증 잔여 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 갱신 | 2026-10-27 / AI |
| 최근 세션 | [2026-10-27 #001](../members/lim8603/workspace/session_logs/session_2026-10-27_001.md) 진행 — TASK-002 Human 리뷰 승인, GitHub 연결·게시 준비 |

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
| TASK-002 | workspace와 플랫폼·설정 계약 | AI | In Progress | 2026-10-27 | GitHub 연결·저장소 생성/push·native CI 확인(EV-001/GAP-004) |

## 다음 시작점

**2026-10-27 진행:** Human이 TASK-002 구현 리뷰를 승인하고 GitHub MCP 연결·본인 계정 저장소 생성·업로드를 요청했다. VS Code GitHub MCP 등록을 확인했고 Codex에도 `github` 서버를 등록했다. Codex OAuth는 GitHub 서버의 동적 클라이언트 등록 미지원으로 실패했다. 현재 세션 GitHub 도구·토큰·Git 인증 helper가 없어 Codex 측 인증 대기다. `dot-logger` 비공개 저장소로 준비한다.

1. [TASK-002](../04_implementation/tasks/TASK-002.md) Human 구현 리뷰는 승인 완료. [EV-001](../05_verification/verification_evidence.md)의 로컬 Pass는 유지하고 GitHub 연결 확인 → 본인 계정 저장소 생성 → 기존 main 이력 push → native CI 결과 확인을 진행한다. CI 실행 전까지 Task는 In Progress다.
2. `.github/workflows/ci.yml`은 Ubuntu 24.04·Windows Server 2025·macOS 14 및 두 Rust 버전으로 구성했다. 현재 Git remote가 없어 원격 CI를 실행하지 않았다(GAP-004). 사용자 연결 후 생성·게시가 승인된 상태다. 실행 환경과 실제 Windows 11/PREEMPT_RT 수락을 구분한다.
3. TASK-002 완료 조건 확인 후 TASK-003 소형 수집·중요 예약 → TASK-004 원본 수명 순서다. TASK-022 장비/OS/UPS 프로필은 계속 미확정이며 실제 성능·24h·차단·OS 교환은 MS-007에서 검증한다. FR-015/TASK-018은 별도 제안이다.

## 새 세션 재개 메모

- 먼저 이 문서 → TASK-002 → EV-001 결과·코드(`config.rs`, `platform.rs`, TC-001)와 GAP-004/005를 읽는다. TASK-003 진입 시 해당 Task·인터페이스 §13·도메인 §4~5를 추가로 확인한다.
- `rust-toolchain.toml`의 개발 1.96.0과 Cargo MSRV 1.85/lock을 유지한다. 로컬 환경은 Ubuntu 26.04.1 x86_64이며 실제 R1 장비가 아니다. 전역 rustup default는 계속 미설정으로 두고 프로젝트 pin을 사용한다.
- `SessionBuilder::validate`는 시작 전 구성 검사다. 실제 build·worker·파일·schema·durability·할당 예산·수집/종료는 후속 Task에서 구현한다. 문서상 설계 API를 구현 완료로 오인하지 않는다.
- 코드·테스트·CI는 `2f9015d`에 커밋했다. 설계/ADR-010·EV 원본·세션 마감 상태는 별도 `docs(cowork)` 커밋에 보존한다. EV-001의 소스 hash가 이 코드 커밋과 일치한다. 개인 세션 로그는 기존 .gitignore 정책으로 Git 제외다.
- 기존 설계·TDD 및 MCAP 0.26 승인은 재질문하지 않는다. 장비 미확정·native CI 미실행·나머지 제품 TC Not Run·FR-015 별도 제안·T1 미도래를 유지한다.

## 선택 로딩

- [도메인](../03_design_artifacts/domain_model.md), [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안), [데이터](../03_design_artifacts/data_model.md), [CLI](../03_design_artifacts/ui_spec.md).
- [MS 목록](../04_implementation/milestone_registry.md): 7개 MS, [Task 목록](../04_implementation/task_registry.md): 26개 Task(1개 선택). 선행조건·변경 대상·첫 실패/DoD·상대 인일 추정(계획 승인, 납기 확정 아님).
- [요구 추적표](../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트): INT 1 + FR 18 + NFR 7 + CON 7 = 33행. MS별 구현/테스트 체크·코드/EV 연결. TC-001의 TASK-002 하위 시험만 EV-001 로컬 Pass; 나머지 실행은 Not Run.
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
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·수명/동시성 증명은 미검증 | TASK-003~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬 확인(EV-001), native·실제 DB 호환 잔여 | TASK-002·019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증, TASK-018은 별도 범위 승인 후 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·004·005 | 장비·나머지 제품 TC·native CI·paste 유지보수 경고. toolchain GAP-003은 로컬 범위 Closed | [검증 근거](../05_verification/verification_evidence.md) 참조 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-10-26 #002: TASK-002 구성/API 구현·두 Rust 버전 각 16 feature·EV-001, MCAP 보안 수정 ADR-010 승인. 코드 커밋·세션 마감, Human 구현 리뷰·native CI는 다음 세션 이월 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-26_002.md).
- 2026-10-26 #001: 상세설계·33행 요구 추적·7MS/26Task·공식 조사 작성, ADR-007~009·클라이언트/상세 기준선·계획 승인, Gate 3 Pass — [로그](../members/lim8603/workspace/session_logs/session_2026-10-26_001.md).
- 2026-10-25 #003: Tier 1/2·코어 배치·6대 설계 고려사항·cpp-logger 생성·Git 초기화 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-25_003.md).

이전 공유 상태 원문은 [상태 아카이브](state_archive.md)에 이관했다. 현재 Git 저장소이며 2026-10-27 세션 시작 HEAD는 `37e4e9e`다. TASK-002 구성 구현/로컬 검증과 코드 커밋 `2f9015d`를 완료했다. 실제 성능·장비 검증·배포는 미실행이다. 2026-10-27 재개해 Human 구현 리뷰 승인을 기록했으며 GitHub 연결·게시를 진행한다.

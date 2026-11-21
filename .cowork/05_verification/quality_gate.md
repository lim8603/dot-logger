# 품질 게이트 (Quality Gate)

> 품질 관문 — 각 단계를 통과하기 위한 최소 기준

---

## 게이트 적용 원칙

- Gate 4/5는 대상 revision·환경·필수 시나리오가 일치하는 근거로 판단한다. `verification_evidence.md`의 유효 범위·미검증 관리 규칙을 따른다. Task Done·프로젝트 Green·과거 버전의 Pass를 현재 릴리즈 승인으로 대체하지 않는다.

- template 파일은 gate 통과 판단의 기준 문서가 아니다.
- 목록 문서는 상태와 링크의 기준선이고, 상세 근거는 기준 본문 또는 상세 문서에서 확인한다.
- `session_logs/`와 `imported_context/`는 보조 근거이며, 필요한 사실이 기준 문서에 반영되지 않았다면 게이트 충족으로 보지 않는다.
- `deliverable_plan.md`에서 `해당없음`으로 확정된 산출물은 gate 판단 대상에서 제외한다.
- `project_state.md`는 gate 통과 요약을 남길 수 있지만, 실제 판정은 각 gate의 증거 문서를 우선 확인한다.
- 게이트 1, 게이트 2에서 `deliverable_plan.md`의 수집 상태는 **현재 Phase에서 기준 문서를 직접 형성해야 하는 필수 산출물**만 차단 조건으로 본다.
- 최초 세션 최소 입력 누락은 즉시 게이트 미충족으로 판정하지 않지만, 게이트 1 또는 게이트 2로 넘어가기 전 `session_protocol.md`의 최초 세션 종료 체크에 따라 우선 보완하는 것을 기본값으로 본다.
- `verification_evidence.md`는 게이트 4, 게이트 5용 증거 인덱스다. 실제 판정은 이 문서와 원본 기준 문서를 교차 확인한다.

---

## 게이트 정의

### 게이트 1: Intent → Requirement

| 항목 | 기준 |
|------|------|
| 활성 Intent 승인 | `intent_registry.md`에 활성 Intent 1건 이상 Approved |
| 모호한 요구사항 | 0건 |
| 비목표 명시 | 작성 완료 |
| DEFINE 핵심 질문셋 정리 | `session_protocol.md` 기준 DEFINE 종료 전 필수 질문셋의 답 또는 열린 질문이 관련 기준 문서와 `project_state.md`에 보인다 |
| 산출물 계획 확정 | `deliverable_plan.md` Human Approved |
| 필수 산출물 수집 착수 | `deliverable_plan.md` 기준 현재 Define 기준 문서를 직접 사용하는 필수 산출물은 `미수집`으로 방치되지 않고, 필요한 데이터가 `미수집 데이터 목록`에 등록되어 있어야 한다 |

### 게이트 2: Requirement → Design

| 항목 | 기준 |
|------|------|
| 요구사항 ID 부여 | 전체 완료 |
| 우선순위 확정 | Must 항목 확정 |
| 수락 기준 작성 | `user_story_registry.md`에 등록된 승인 대상 Story 기준 충족 |
| 리스크 식별 | 초기 등록 완료 |
| DEFINE 핵심 질문셋 종료 | `session_protocol.md` 기준 DEFINE 종료 전 필수 질문셋이 기준 문서 기준으로 닫혀 있으며, 남은 항목은 Human 승인된 보류 사유와 다음 수집 계획이 기록되어 있어야 한다 |
| 필수 산출물 수집 상태 정렬 | `deliverable_plan.md` 기준 현재 Define 기준 문서를 직접 사용하는 필수 산출물은 `수집중` 이상이어야 하며, 요구사항 정의를 직접 기준 문서로 쓰는 핵심 산출물은 `완료` 또는 즉시 생성 가능한 수준이어야 한다 |

> 게이트 1, 게이트 2에서 후속 Phase 산출물까지 모두 `완료`일 필요는 없다.
> 단, 현재 Phase 기준 문서가 이미 존재해야 하는 필수 산출물이 `미수집`으로 남아 있거나, DEFINE 핵심 질문셋이 근거 없이 비어 있으면 게이트 미충족으로 본다.

### 게이트 3: Design → Implementation

| 항목 | 기준 |
|------|------|
| 도메인 모델 리뷰 | Human Approved |
| ADR 기록 | 핵심 결정 기록 완료 |
| 인터페이스 계약 정의 | 모듈 간 계약 확정 |
| 기술스택 확정 | `tech_stack.md` 등록 완료 |
| Milestone 구조화 | `milestone_registry.md` 초안 이상 작성 |
| Task 구조화 | `task_registry.md` 초안 이상 작성 |
| 팀 Task 할당 | `team_board.md` 역할별 Task 배정 완료 (팀 프로젝트 시) |

### 게이트 4: Implementation → Verification

| 항목 | 기준 | 핵심 증거 |
|------|------|-------------|
| 코드 리뷰 완료 | Review Checklist 통과 | `05_verification/verification_evidence.md`, `04_implementation/review_checklist.md`, 관련 `04_implementation/tasks/TASK-*.md` |
| 검증 대상 Milestone 정렬 | `milestone_registry.md` 기준 검증 대상 Milestone이 Review 이상 상태 | `05_verification/verification_evidence.md`, `04_implementation/milestone_registry.md`, 관련 `04_implementation/milestones/MS-*.md` |
| 핵심 Task 완료 기준 충족 | `task_registry.md` 기준 검증 대상 Task가 Review 이상 상태 | `05_verification/verification_evidence.md`, `04_implementation/task_registry.md`, 관련 `04_implementation/tasks/TASK-*.md` |
| 단위 테스트 통과 | 핵심 범위 기준 통과 | `05_verification/verification_evidence.md`, `05_verification/test_case.md`, 관련 `04_implementation/tasks/TASK-*.md` |
| 코딩 컨벤션 준수 | Lint 통과 | `05_verification/verification_evidence.md`, `04_implementation/review_checklist.md`, 관련 `04_implementation/tasks/TASK-*.md` |
| 게이트·불변식 변경 시 종단 재검증 (F-11) | 조인 판정의 소비자 목록이 작성되고 종단 스위트로 확인됨 | `05_verification/verification_evidence.md`, 최신 세션 로그(소비자 목록), `05_verification/test_case.md` |

#### 게이트 4 판정 규칙

- `verification_evidence.md`를 먼저 보고, 표에 적힌 원본 문서와 최신 상태가 서로 일치하는지 교차 확인한다.
- Milestone / Task / 리뷰 / 테스트 / lint 상태는 evidence와 목록 문서 / 상세 문서 / checklist가 같은 상태를 가리켜야 한다.
- 미충족이면 Verify 진입을 보류하고, 차단 사유와 열린 항목을 `project_state.md`에 기록한다.
- 예외는 Human 승인 후 `project_state.md`와 최신 세션 로그에 사유와 보완 계획을 남긴다.
- 테스트 예외나 일부 미실행 범위는 `test_case.md` 또는 `verification_evidence.md`에도 함께 남긴다.

### 게이트 5: Verification → Release

| 항목 | 기준 | 핵심 증거 |
|------|------|-------------|
| 통합 테스트 통과 | 전체 통과 | `05_verification/verification_evidence.md`, `05_verification/test_case.md`, 필요 시 관련 `04_implementation/tasks/TASK-*.md` |
| NFR 검증 | 목표값 충족 | `05_verification/verification_evidence.md`, `05_verification/test_strategy.md`, `05_verification/test_case.md` |
| 문서 최종 업데이트 | 완료 | `05_verification/verification_evidence.md`, `06_evolution/project_state.md`, release 대상 기준 본문 |
| 산출물 범위 충족 | `deliverable_plan.md` 기준 필수 산출물 생성 가능 | `05_verification/verification_evidence.md`, `02_project_definition/deliverable_plan.md`, `07_delivery/*` |

#### 게이트 5 판정 규칙

- `verification_evidence.md`를 기준 인덱스로 보고, Release 대상 범위의 최신 검증 결과와 문서 상태를 원본 기준 문서에서 다시 확인한다.
- Integration / E2E / NFR / 문서 / 산출물 준비 상태는 evidence, `test_case.md`, `test_strategy.md`, `project_state.md`, `deliverable_plan.md`, `07_delivery/*`가 서로 일치해야 한다.
- 미충족이면 Release를 보류하고 실패 범위, 누락 문서, 누락 산출물, 재검증 계획을 기록한다.
- 예외는 Human 승인 후 미충족 항목, 수용 사유, 후속 보완 계획을 남긴다.

---

## 게이트·불변식 변경의 종단 재검증 (F-11)

> 검증 규칙·게이트·불변식·계약을 **조이는** 변경은 자기 자신만 바꾸지 않는다. 그 관문을 이미 지나다니던 **기존 경로의 의미**를 바꾸므로, 조인 코드가 아니라 다른 곳에서 터진다.

### 왜 필요한가

- 조인 변경에는 대개 그것을 증명하는 새 테스트가 붙는다. 그 테스트는 통과한다 — 새 규칙이 새 규칙대로 동작하는지만 보기 때문이다.
- 정작 깨지는 것은 **그 관문을 지나던 기존 경로**다. 이전에는 통과하던 것이 이제 걸리고, 그 사실은 종단 경로를 실제로 밟아봐야 드러난다.
- 단위·통합 테스트가 전부 초록인 채로 기능 하나가 조용히 죽고, 다음 세션이 그것을 다시 밟기 전까지 아무도 모른다. **다른 세션이 남긴 결함이 이번 세션의 게이트 강화로 승격되는** 형태라 책임 소재도 흐려진다.

### 규칙

- **조인 시점에 소비자 목록을 적는다.** "이 판정을 통과해야 하는 것이 무엇인가" — 코드 경로, 화면, 계약/스키마, 시드/픽스처, 기존 데이터까지 포함해 세션 로그에 남긴다.
- **세션 종료 전 그 목록을 종단 스위트로 확인한다.** E2E·통합 등 실제 경로를 밟는 검증이어야 하며, 단위 테스트 통과는 이 항목을 대체하지 못한다.
- **미검증분은 침묵하지 않는다.** 실행하지 못했으면 소비자별로 무엇이 미검증인지 명시하고 `project_state.md` 이월과 세션 로그에 등재한다. Human이 사전 승인한 예외라도 **공지 없이 넘기는 것은 예외의 범위가 아니다.**
- 이 항목은 Phase 전환 게이트 4 판정에 포함된다(위 표).

### 적용 대상 예시

- 승격/배포 게이트의 판정 조건 강화, 스키마·API 계약 변경, 유일성·참조 무결성 제약 추가, 검증 함수의 허용 범위 축소, 권한 판정 강화, 기본값·폴백의 보수화.

---

## 플레이스홀더 / 구조 검증 (Placeholder / Structure Check)

아래 항목이 **현재 게이트 판단 또는 `deliverable_plan.md`에서 필수 / 권장 기준 문서로 사용되는 기준 본문 / 목록 문서 / 상세 문서**에 남아 있으면 게이트 미충족으로 본다.

- `templates/*_template.md` 안의 placeholder와 가이드 주석은 정상이다.
- 목록 문서의 `현재 등록 ... 없음` 문구는 허용하지만, 예시 행이나 더미 샘플 행은 남기지 않는다.
- `H1`에 `Template` 문자열이 남아 있다.
- `<!-- ... -->` 형태의 가이드 주석이 기준 본문에 남아 있다.
- `INT-000`, `US-000`, `MS-000`, `TASK-000`, `ADR-000` placeholder가 남아 있다.
- 목록 문서에 없는 활성 항목을 기준 문서처럼 사용한다.
- `deliverable_plan.md`의 기준 문서가 실제 목록 문서 / 상세 문서 구조와 불일치한다.
- imported context만 있고 기준 문서 반영이 없는 상태로 확정 판단한다.

---

## 예외 처리

Gate를 우회해야 하는 경우:

1. Human이 명시적으로 승인한다.
2. 사유를 ADR 또는 세션 로그에 기록한다.
3. 후속 보완 계획을 수립한다.

## dot-logger Gate 3 판정 — 2026-10-26

현재 판정: **Pass — 설계 기준선 확정, 구현 진입 가능**. 제품 구현은 미착수다.

승인 근거: 2026-10-26 Human 「권고 기준선 채택 — 장비·호환 검증은 계획대로 진행」. 직전 Partial 판정의 미승인 계약/기술 항목을 아래와 같이 해소했다.

| 항목 | 상태 | 근거/검증 경계 |
|---|---|---|
| 도메인 모델 리뷰 | Pass | 실행 구조 및 상세설계 Human Approved, ADR-007~009 |
| ADR 기록 | Pass | ADR-003~009 Accepted |
| 인터페이스 계약 | Pass | API §13~17·소유권/실패/완료·추가 AC 승인 |
| 기술스택 | Pass | tech_stack에 채택 등록. patch/MSRV 실제 호환 검증은 TASK-002 |
| Milestone/Task | Pass | INT-001 → MS-001~007 → TASK-001~026, 의존/DoD/시험/추정 일정 승인 |
| 팀 Task 할당 | 해당 없음 | solo, AI 실행/Human 판단 |

장비별 수락 조건·의존성 호환 확인을 해당 Task에서 수행하는 계획까지 Human이 승인했다. 이 이월은 성능/안전성/호환 시험 면제가 아니다. 실장비는 TASK-022, 제품 시험은 MS-002~007의 TC/EV로 검증한다. FR-015/TASK-018은 별도 제안이며 구현 승인 범위 밖이다. Gate 4/5는 Not Ready를 유지한다.

## 2026-10-31 승인된 검증 방법·시점 변경

Human 승인 [ADR-011](../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 현재 PC 실측, UPS는 시뮬레이션 결과로 대체 수락한다. native는 저장소 공개 완료 후 진행하는 승인된 이월로 GAP-004 Not Run을 유지하며 공개 전 구현/로컬 검증·준비를 진행할 수 있다. 이전 실물 UPS 필수/장비 미확정 조건은 이 범위에서 갱신한다. 미실행 실측·모의 시나리오를 Pass로 표시하지 않고 Task와 Gate는 유효 실행 근거로 개별 판단한다. 이번 정책/환경 기록만으로 Gate4/5를 통과시키지 않는다.

## 2026-11-13 Task·MS 완료 기준 변경 — Human 결정

Human 결정(2026-11-13): 「실사용처 테스트는 마지막에 진행은 하겠지만, 일정 판단에 두지 말고, 일정 판단은 유닛테스트 케이스 커버리지, 실제 로깅 사용 사례 조사 등을 통해서 마무리」. dot-logger는 범용 로보틱스/RTOS Rust 로그 라이브러리이고, dot-probe는 첫 사용처일 뿐이다. 설계·구현은 특정 사용처에 맞추지 않는다.

| 구분 | 기준 |
|---|---|
| **Task Done** | 해당 Task의 DoD 범위를 시험(유닛·통합·compile-fail·allocator guard·Miri 등)으로 확인했고, 이 PC의 로컬 회귀(1.96.0·MSRV 1.85.0)가 Pass이며, Human 리뷰나 위임 수락을 마쳤다. 알려진 시험 공백은 Task 안에서 메우거나, 이유와 함께 TASK-033 공백 목록에 넘긴다. |
| **Task 완료를 막지 않는 항목** | ① 실사용처 적용(TASK-026, 마지막에 진행) ② native·다른 OS(T3, 공개 후 TASK-024) ③ 장기 24h·전원 차단(TASK-024) ④ 이 PC의 정량 성능·RT 실측(TASK-023에서 수행한 범위, 나머지는 이월 항목). 이 항목들은 각 검증 Task가 소유하며, 기능 Task를 In Progress로 붙잡아 두지 않는다. |
| **MS Done** | 소속 Task가 모두 Done이다. 선택 기능(FR-015 질문 프리셋 TASK-018)은 Must 완료에 넣지 않는다. |
| **프로젝트 일정 판단(MS-007 마무리)** | ① 시험 커버리지: 라인·함수·리전 커버리지 기준선([EV-078](verification_evidence.md#ev-078))과 파일별 공백 분류. 목표치는 TASK-033 S4에서 확정한다. ② 로깅 사용 사례 조사: 로보틱스/RTOS 로깅 사용 사례 목록과 기존 TC의 대응표, 공백 시험 보완(TASK-033). |
| **Gate 4/5** | 위 기준과 별개다. 기존 판정 규칙을 유지하며 Task Done만으로 Gate를 통과시키지 않는다. |

**2026-11-15 Human 「승인」 — 일정 판단 목표치(TASK-033 S4):** ① workspace 커버리지 하한 라인 ≥ 92%·함수 ≥ 92%·리전 ≥ 89%(`cargo llvm-cov --workspace --all-features`, doctest·`#[ignore]` 제외, 회귀 방지) ② 라인 85% 미만 파일의 「실제 동작」 미실행 공백은 시험 또는 사유 기록으로 미처리 0 ③ 범위 안 UC의 구현 동작 시험 대응 100%(기능 공백은 B1 또는 소유 Task) ④ TASK-034·035 완료 뒤 재측정 Pass로 MS-007 일정 판단 마무리. [TASK-033](../04_implementation/tasks/TASK-033.md#s4-목표치--2026-11-15-human-승인).

**2026-11-15 Human 결정 추가:** 「native·24h 장기 검증을 일정 판단에서 빼자」. TASK-024(native·다른 OS T3, 24h·전원 차단)도 TASK-026처럼 마지막 단계로 진행하되 일정 판단에 넣지 않는다. 일정 판단은 위 ①②(TASK-033)만으로 한다.

**2026-11-18 일정 판정:** TASK-033 S4 [EV-089](verification_evidence.md#ev-089) Pass. 최종 라인 93.69%·함수 92.67%·리전 91.08%, 파일별 실제 동작 공백 미처리 0, 범위 내 UC 대응 28/28. Gate 4·5는 Not Ready.

이 결정으로 2026-10-31 기록의 "이번 정책 변경만으로 Task를 Done 처리하지 않는다"는 문구를 대체한다. 미실행 시험(native·장기·실사용처)을 Pass로 바꾸지 않으며, 미실행 상태는 GAP과 소유 Task에 그대로 남는다.

## dot-logger Gate 4·5 판정 — 2026-11-21

TASK-025 S4에서 판정했다. 대상은 main `987fdb9`와 이 판정에서 고친 공개 스키마·CI 설정이다. 근거 색인은 [EV-110](verification_evidence.md#ev-110)과 [MS별 추적 요약](test_case.md#ms별-추적-요약--2026-11-21-task-025-s4)이다. **Gate 4 판정은 AI 판정안이며 Human 수락 전에는 Phase를 바꾸지 않는다.**

### Gate 4 (Implementation → Verification) — AI 판정안: Pass (이 PC 로컬 범위)

| 항목 | 판정 | 근거 |
|---|---|---|
| 코드 리뷰 완료 | Pass | REV-001~070 모두 Closed. 각 리뷰는 review_checklist 기준으로 조치·수락했다 |
| 검증 대상 Milestone 정렬 | Pass | 구현 MS인 MS-002~006·008·009가 Done이다. MS-007은 검증·인수 MS 자체다 |
| 핵심 Task 완료 기준 | Pass | 구현 Task 31건 Done. 남은 TASK-024(native·24h)·TASK-026(실사용처)은 2026-11-13·15 Human 결정으로 마지막 단계의 검증·적용 Task다. TASK-018은 범위 미승인 선택 기능이다. TASK-025는 이 S4로 마무리한다 |
| 단위 테스트 통과 | Pass | Rust 1.96.0·1.85.0 각 724 Pass·실패 0·5 ignored(외부 DB), feature 조합 두 버전, Miri CI 범위([EV-110](verification_evidence.md#ev-110)). 커버리지 라인 93.69%·함수 92.67%·리전 91.08%([EV-089](verification_evidence.md#ev-089)) |
| 코딩 컨벤션 | Pass | 두 버전 Clippy `-D warnings`, fmt, strict rustdoc(workspace·core 전용), 통합 표면 검사 |
| F-11 종단 재검증 | Pass — 결함 2건 발견·조치 | 아래 소비자 목록을 현재 HEAD에서 확인했다. 공개 스키마 누락(GAP-011)과 CI Miri 설정 낡음(GAP-012)을 찾아 고쳤다 |

F-11 소비자 목록(최근 계약 강화와 영향 경로):

| 변경 | 소비자 | 확인 |
|---|---|---|
| ADR-015 묶음·파이프라인 ACK | 서버 Ack, client `handoff`/`flush`, CLI `tail`, viewer, `dot_probe_*`·`lan_probe` 예제, 공개 `.proto` | 종단 `tail` 10건 ACK, LAN [EV-104](verification_evidence.md#ev-104)·[EV-105](verification_evidence.md#ev-105). `.proto` 누락 발견 → 보완 |
| REV-069 구독 창 묶음 상한 | client 묶음 크기, 서버 거절 | 회귀 시험([EV-106](verification_evidence.md#ev-106)), 종단 `tail` |
| REV-070 endpoint·upstream 검사 | CLI, client `connect`, viewer, README 명령 | 종단 viewer·대시보드·CLI, S2 README 명령 실행 |
| ADR-013 `standard/2`·manifest v2 전용 | 오프라인 CLI, `recover`, viewer 파일 열기, Foxglove | [EV-088](verification_evidence.md#ev-088)·[EV-090](verification_evidence.md#ev-090)·[EV-096](verification_evidence.md#ev-096), 회귀 시험 |
| 손실 checkpoint 실제 시각 | Miri CI 작업 | CI 설정 낡음 발견 → 조치 |

범위 한계: 위 근거는 이 PC(Linux x86_64)의 로컬 실행이다. native·다른 OS CI는 GAP-004로 Not Run이며 공개 뒤 TASK-024(T3)가 소유한다. Gate 4 Pass는 이 범위를 넘어서 해석하지 않는다.

### Gate 5 (Verification → Release) — 판정: Not Ready

| 항목 | 판정 | 근거·남은 일 |
|---|---|---|
| 통합 테스트 전체 통과 | Not Ready | 로컬 통합·종단은 통과. TC-012의 P6 24시간·native·3 OS·실사용처, TC-011의 MS-007 부분이 Not Run |
| NFR 검증 | Not Ready | NFR-001·002·005·006은 이 PC 측정 근거가 있다([EV-064](verification_evidence.md#ev-064)~[EV-070](verification_evidence.md#ev-070), [EV-097](verification_evidence.md#ev-097)~[EV-105](verification_evidence.md#ev-105)). NFR-003 24h·NFR-007 3 OS는 Not Run |
| 문서 최종 업데이트 | Partial | 공개 README와 `07_delivery` 작업 문서(TASK-025 S2·S3) 완료. 릴리즈 노트는 미배포 초안이고 TASK-024·026 결과 반영 전 |
| 산출물 범위 충족 | Not Ready | deliverable_plan의 필수 산출물 다수가 수집중이고 공식 `docs/`는 만들지 않았다 |

Gate 5 재판정 경로: 저장소 공개 전환 → TASK-024(T3 native·3 OS, 24h·전원 차단) → TASK-026(첫 실사용처 적용) → 릴리즈 노트·운영서 갱신과 공식 산출물 생성 → Gate 5 재판정. 미실행 시험은 Pass로 바꾸지 않는다.

# 상태 아카이브 (State Archive)

> `project_state.md` / `my_state.md`에서 하베스트된 과거 완료 서사·핸드오프 이력의 **append-only 원문 저장소**(Log / Archive 역할).
> 매 세션 로드 대상이 아니다. `project_state.md`의 요약이 포인터로 가리키며, 특정 과거 세션 맥락이 필요할 때만 해당 섹션을 연다.

---

## 목적 / 운영 규칙

- 이 문서는 **다이어트 규칙 R1**(→ `session_protocol.md` §공유 상태 인덱스 관리, `project_state.md` §작성/유지 규칙)의 **하베스트 대상**이다.
- 세션 시작·마감에 R1/R2를 적용하여 완료 서사 상세를 이곳에 **원문 이관**한다. 상태 문서에는 최근 최대 3세션의 한 문장 요약과 근거 링크만 유지한다. 기존 로그/아카이브에 동일 원문이 있으면 링크를 재사용하고 복제하지 않는다.
- 이관은 **append-only**다 — 여기서는 요약·삭제하지 않고 원문을 그대로 누적한다. 정제·요약은 `project_state.md` 본문 쪽에서만 한다.
- 이관 단위는 세션(#NNN) 또는 이관 회차다. 아래 `## 이관분` 아래에 `### #NNN 이관분`(또는 회차) 섹션을 추가한다.
- 이 문서 자체가 과도하게 비대해져도 **압축하지 않는다** — 아카이브의 역할은 원문 보존이다. 필요하면 회차/기간 단위로 파일을 분리(`state_archive_YYYY.md` 등)하되, 이는 Human 승인 후 진행한다.

---

## 이관분 (project_state.md)

> `project_state.md`의 `다음 시작점`·`AI 핸드오프 메모`·`최근 완료 서사`에서 이관된 원문. 최신 이관분을 위에 쌓는다.

_아직 이관된 항목이 없다._

---

## 이관분 (my_state.md)

> `members/<이름>/workspace/my_state.md`의 `오늘의 세션 Intent`·`최근 결정/작업 메모`·`참조 세션 로그`에서 이관된 원문.

_아직 이관된 항목이 없다._

---

## 작성 / 유지 규칙

- 포인터 규율: `project_state.md`에서 이관분을 가리킬 때는 `[state_archive.md](state_archive.md) #NNN 이관분` 형태의 1줄 포인터를 남긴다.
- 이 문서는 기준 본문(Canonical)이 아니라 로그/아카이브(Log/Archive)다. 확정 사실의 근거로 삼을 때는 항상 승인된 기준 문서/ADR을 우선하고, 여기 원문은 보조 근거로만 쓴다.
- imported context와 달리 이 문서는 프로젝트가 스스로 생산한 이력이므로 VCS로 추적한다(세션 로그와 달리 gitignore 대상이 아니다).

## 2026-10-26 상세설계 착수 전 상태 원문

현재 상태 인덱스 R1/R2 정리로 이전 상세를 이관한다. 아래는 이력이며 현재 기준이 아니다.

# 프로젝트 상태 (Project State)

> 공유 재개 인덱스. 관련 상세 문서만 추가로 읽는다.

## 현재 상태 요약

### 핵심 필드

| 항목 | 내용 |
|------|------|
| 프로젝트 | dot-logger |
| 프로젝트 유형 | Greenfield — 구현 코드 없이 협업/설계 문서만 존재 |
| 팀 구성 | lim8603 — 프로젝트 오너/Master, AI 지원 |
| 팀 규모 | 1인 개발 |
| 협업 모드 | Active — 1인 Design 진행, 별도 Task 배정 전 |
| 협업 실행 모드 | solo — Human이 2026-10-25 확인 |
| 현재 Phase | Design |
| 개발 방식 | TDD — 실패 테스트 → 최소 구현 → 리팩토링, 2026-10-25 Human 최종 승인 |
| 활성 Intent | INT-001 — 재사용 가능한 실시간 로보틱스 로깅·텔레메트리(Approved) |
| 활성 Milestone | 없음 |
| 활성 Task | 없음 |
| 상태 | Yellow — Gate 1·2 Pass, 설계 기준선 승인. 핵심 API 소유권·포화·완료 상세 권고안 작성, Human 검토 전 |
| 대화 언어 | 한국어 |
| 작업 문서 언어 | 한국어 |
| 공식 산출물 문서 언어 | 미확정 |
| 마지막 갱신일 | 2026-10-25 |
| 마지막 갱신자 | AI |
| 최근 세션 상태 | 2026-10-25 #003 종료 — 6대 설계 반영·플랫폼 Tier 1/2 및 코어 격리 확정·C++ 쌍둥이 프로젝트 생성·Git 첫 커밋 완료. 다음 세션은 첫 구현 범위 실행 구조 결정부터 재개 |
| 참조 세션 로그 | [lim8603 / session_2026-10-25_003.md](../members/lim8603/workspace/session_logs/session_2026-10-25_003.md) — 6대 설계 반영·Tier 1/2 확정·cpp-logger 생성·Git 커밋 및 다음 시작점 |

### 한 줄 상태

ADR-005·006 기준선에 플랫폼 Tier 1/2 계층화 및 CPU 코어 격리를 확정했고 6대 설계 고려사항(역방향 회수·타임스탬프 삼원화·ENOSPC 페일세이프 등)을 반영했다. C++ 쌍둥이 프로젝트(`~/src/cpp-logger`) 생성 및 로컬 Git 첫 커밋을 완료했다. 다음 시작점은 첫 구현 범위 실행 구조 결정이다.

### 현재 작업 스트림

- 승인 기준선: [요구사항 §12·15](../02_project_definition/requirement_spec.md#15-설계-진입-준비-점검과-기준선-검토), [ADR-005](../03_design_artifacts/adrs/ADR-005_design_baseline_and_deferred_decisions.md). 원본1 GB/s급+소형10만건/s·32생산자/16파일·128채널·R1/P0~P6·R1 버퍼2 GiB와 용량 비교는 설계용 목표다. 제품 기본값/실측/모든 OS 보장과 구분한다.
- 크로스 플랫폼: [NFR-007·§16](../02_project_definition/requirement_spec.md#16-크로스-플랫폼-지원과-검증-범위), [ADR-006](../03_design_artifacts/adrs/ADR-006_cross_platform_linux_macos_windows.md), US-005. 라이브러리/클라이언트 모두 Linux·macOS·Windows 지원. Linux 일반/PREEMPT_RT 프로필을 구분하고 OS별 버전/CPU/계측·최적화·네이티브 실행 매트릭스는 Design에서 구체화한다.
- 설계 초안: [도메인/모듈](../03_design_artifacts/domain_model.md)·[핵심 인터페이스 §3~4·9~12](../03_design_artifacts/interface_contract.md#11-리뷰-시나리오승인-범위잔여-사항). 소유권/회수·예약/포화·완료와 FR-018 긴급 경로를 상세화했다. [요구사항 §17](../02_project_definition/requirement_spec.md#17-전원-이상-시-중요-파일-우선-긴급-기록)의 선택 옵션/기본 비활성·전체 시간 조건은 Human 확인, 상세안/추가 AC는 검토 전이다. 기술은 미채택이다.
- 핵심 가치/확정 정책: 사용 중 부하/한계 확인→개발자 기록량 조절→전후 효과 확인. [ADR-003](../03_design_artifacts/adrs/ADR-003_configurable_policies_and_defaults.md)의 RT 무대기·중요 기록 우선·별도 보존 중요도와 [ADR-004](../03_design_artifacts/adrs/ADR-004_processing_and_storage_budgets.md)의 처리/저장 예산 분리·SSD·보존 모드 옵션은 유지한다.
- 첫 적용은 dot-probe 자체 기록·조회·재생과 운영 관측/조절 지원. 센서 구성은 제품명이 없는 대표 설계 시나리오이며 과거 RealSense553~968 MB/s 계산을 현재 카메라4개의 확정 부하로 쓰지 않는다. ROS 2 검토는 T1, 실제 장비/기술/정량 검증은 미정이다.

## 활성 Task 요약

| Task ID | 제목 | 담당 | 상태 | 마지막 갱신일 | 다음 액션 |
|---------|------|------|------|---------------|-----------|
| 없음 | - | - | - | - | 첫 구현 범위의 실행 구조 대안/권고안과 핵심 API를 검토한 뒤 기술·구현 작업 구체화 |

## 다음 시작점

1. **[모듈 설계 §2·5](../03_design_artifacts/domain_model.md#2-책임-경계-초안)를 바탕으로 첫 구현 범위의 실행 구조 대안과 권고안을 작성·검토한다.** 라이브러리/별도 서비스 역할·프로세스 구성과 RT 수집/백그라운드 처리 경계부터 다룬다. 현재 구조는 Draft이며 이번 세션에서 채택하지 않았다.
2. 수집→제한 버퍼→파일→완료 확인을 기준으로 [핵심 API §11.2·12](../03_design_artifacts/interface_contract.md#112-이번에-검토할-계약-묶음)를 함께 리뷰한다. OQ-002~005에 따라 실제 dot-probe/OS·데이터 크기/발생률과 FR-018 활성 구성의 UPS/SSD·보호량·동기화/복구/계측 조건을 보완한다.
3. 합의 범위를 ADR로 기록하고 필요한 기술·Milestone/Task를 구성해 Gate 3를 평가한다. 이후 [승인된 TDD](../05_verification/test_strategy.md)로 구현하고 실제 환경 검증을 병행한다. TDD 승인·FR-018 선택 옵션/기본 비활성은 확정됐으며 API 상세 묶음/추가 AC의 승인은 아니다. 제품·시험 코드/종단 실행은 아직 없다.

### 이번 작업에서 다음에 읽을 문서

- 개인 담당/기록 맥락이 필요하면 [lim8603 프로필](../members/lim8603/profile.md)·[개인 상태](../members/lim8603/workspace/my_state.md)·[최신 로그](../members/lim8603/workspace/session_logs/session_2026-10-25_002.md)를 추가로 읽는다. 공유 상태를 개인 상태에 복제하지 않는다.

- 우선 [도메인/모듈](../03_design_artifacts/domain_model.md), [인터페이스 계약](../03_design_artifacts/interface_contract.md), [기술스택](../03_design_artifacts/tech_stack.md). 설계 문서는 Draft이며 승인된 요구/미결과 기술 채택을 구분한다.
- [요구사항 §15~16](../02_project_definition/requirement_spec.md#15-설계-진입-준비-점검과-기준선-검토), [승인 Story/AC](../02_project_definition/user_story_registry.md), [산출물 계획](../02_project_definition/deliverable_plan.md): Gate 1·2 Pass와 승인된 Design 보완 범위.
- [핵심 강점 §13.5](../02_project_definition/requirement_spec.md#135-핵심-강점과-개발자-조절-지원), [설정/간편 점검 §14](../02_project_definition/requirement_spec.md#14-질문-프리셋설정-추천간편-점검-검토안): 관측·검증·추정/손실과 개발자 조절의 구분. FR-015 질문/추천 상세는 제안이다.
- 근거가 필요할 때만 조사: 초기 후보(비공개 원시 근거), 공개 성능 비교(비공개 원시 근거)·수치 JSON(비공개 원시 근거), 실사용/TCP(비공개 원시 근거), 버퍼(비공개 원시 근거), 리포트(비공개 원시 근거).
- 센서/연동 근거가 필요할 때만 과거 센서 용량(비공개 원시 근거)·계산 JSON(비공개 원시 근거), 시각화 연동(비공개 원시 근거), ROS 2 검토(비공개 원시 근거). 과거 제품 가정/당시 미승인을 현재 기준으로 오해하지 않는다.

## 이월 백로그 (Carryover Backlog)

**채번:** 다음 `T` = `T2` · 다음 `B` = `B1`

### 트리거 대기 (T)

| ID | 항목 | 트리거 | 관련 | 출처 |
|----|------|--------|------|------|
| T1 | ROS 2 파일 호환·토픽 연동의 지원 필요성과 범위 검토 | dot-probe 첫 적용 완료가 확인된 때 | INT-001 / OQ-008 | 2026-10-25 Human Q5, requirement_spec.md §8 |

### 백로그 (B)

| ID | 항목 | 내용 | 구분 | 관련 | 출처 |
|----|------|------|------|------|------|
| 없음 | - | - | - | - | - |

이월의 단일 기준은 이 표다. 범위 밖으로 미룬 일은 B, 관측 가능한 선행 조건을 기다리는 일은 T로 채번하며 번호를 재사용하지 않는다. 트리거 도래 감시는 AI의 책임이다. 현재 설계 미결은 아래 OQ로 관리하며 이 표에 중복하지 않는다.

## AI 핸드오프 메모

- Human 「확정」은 ADR-005의 초기 목표·AC·문서 계획·설계 이월과 Design 진입 승인이다. 목표 수치와 R1은 승인됐지만 실제 장비 성능 달성/OS 실행 검증이 아니다. 초기 초안을 다시 승인받지 않는다.
- Human 후속 지시: Linux·macOS·Windows 지원은 라이브러리/클라이언트 공통(ADR-006). 대표 부하는 UHD급2D LiDAR1개·카메라4개·모터10개 이상이며 제품 종속성을 제거했다. ADR-001은 Superseded, 언어/기존 RT 목표는 ADR-006이 계승한다.
- 정책은 옵션으로 제공하고 적용 기본값과 지원 범위를 구분한다. 원본24시간 전량 보존을 요구하지 않으며 입력/처리와 하드웨어별 지속 저장·보존을 분리한다.
- 운영 계측/리포트·간편 점검·개발자 조절 지원은 핵심이다. 사용자가 조절하고 전후 효과를 확인하며 운영 중 고부하 보정·자동 로그 제한을 기본 동작으로 가정하지 않는다.
- Human이 lim8603·solo(1인 개발)를 확인했다. 프로필은 프로젝트 오너/Master/프로젝트 전반, 개인 상태와 첫 로그를 초기화했고 개인 로그 .gitignore 규칙을 추가했다. 기존 핵심 작업은 첫 로그에서 복원 기록으로 구분했다. 같은 식별자/모드를 다시 묻지 않는다.

## Human 확인 필요 사항

| ID | 항목 | 우선순위 | 관련 문서 | 상태 |
|----|------|---------|----------|------|
| OQ-002 | 실제 장비/제어·센서/OS별 성능과 FR-018 UPS/SSD·전달 지연·보호 미동기화량 확인 | High | requirement_spec.md §12·15.4·16~17 | Open(장비 검증 전) |
| OQ-003 | 소유권/회수·예약/포화·완료와 FR-018 긴급 파일 순위/신규 입력·동기화/복구 계약 리뷰 | High | interface_contract.md §3~4·9~12 / requirement_spec.md §15.4·17 | Open(권고안 작성, 상세 합의 전) |
| OQ-004·005 | 배치·OS 버전/CPU/MSRV·의존성/형식/DB/프로토콜·클라이언트·계측 오차/비용·FR-015 상세를 설계에서 결정 | High | requirement_spec.md §15.4·16 | Open(보완 계획 승인) |
| ONB-001 | lim8603·solo·프로필/개인 상태/로그 초기화 완료. 잔여: 공식 산출물 언어·생성 세부를 공식 생성 전에 확인 | Low | session_protocol.md §1A~1C | Open(일부 해소) |

## 주요 리스크 / 주의사항

| ID | 내용 | 대응 상태 | 비고 |
|----|------|-----------|------|
| RSK-001 | 비동기/lock-free와 호출 지연 보장의 혼동 | Open | 실제 전체 호출 경로 검증 필요 |
| RSK-002 | 큐/저장 장애와 RT 지연의 충돌, FR-018 전체 약 3초 내 중요 파일 영속 기록 실패 | Open | 보호량·신호 지연·진행 중 IO·동기화/차단 복구 검증 |
| RSK-003 | 다중 생산자·출력·조회·계측의 자원 경쟁 | Open | 혼합/편중/회전/조회·전송 동시 부하 |
| RSK-004 | 큰 원본의 복사·수명·미처리량과 RT/메모리 상한 | Open | 핸들·실제 원본 예산·정상/거절/회수 경로 |
| RSK-005 | 실제 저장량·지속 쓰기·보존/쓰기 수명 예산 초과 | Open | 배포 장치별 프로필/정책 |
| RSK-006 | 조건이 다른 기준·짧은 점검·폐기 후 안정 상태를 근거로 여유를 과신 | Open | 관측/검증/추정·기준 유효성·오차 검증 |
| RSK-007 | OS별 API/의미·계측 차이에 따른 호환/성능 오판 | Open | 플랫폼 경계·네이티브 실행·기록 교환·OS별 프로필 |

실제 구현·성능/장시간·OS 네이티브 실행·복구 검증은 아직 하지 않았다. 현재 폴더는 Git 저장소가 아니다. 조사는 당시 외부 주장/AI 분석과 현재 승인 기준을 구분한다.

## 최근 승인된 결정

| ID | 결정 | 근거 문서 | 날짜 |
|----|------|-----------|------|
| ADR-003 | 정책 옵션/기본값, RT 무대기 | [상세](../03_design_artifacts/adrs/ADR-003_configurable_policies_and_defaults.md) | 2026-10-25 |
| ADR-004 | 처리/저장 예산 분리·SSD·보존 옵션 | [상세](../03_design_artifacts/adrs/ADR-004_processing_and_storage_budgets.md) | 2026-10-25 |
| ADR-005 | 초기 성능/AC·문서 계획·미결 이월 승인, Design 진입 | [상세](../03_design_artifacts/adrs/ADR-005_design_baseline_and_deferred_decisions.md) | 2026-10-25 |
| ADR-006 | 라이브러리/클라이언트 Linux·macOS·Windows 지원, Linux RT 유지 | [상세](../03_design_artifacts/adrs/ADR-006_cross_platform_linux_macos_windows.md) | 2026-10-25 |

ADR-001은 ADR-006으로, ADR-002는 ADR-003으로 대체됐다. 초기 판단은 각 본문에 이력으로 보존했다.

## 최근 변경 파일 / 산출물

| 파일 | 변경 요약 | 관련 작업 |
|------|----------|----------|
| project_state / my_state / session_2026-10-25_003 | [세션 #003](../members/lim8603/workspace/session_logs/session_2026-10-25_003.md) 종료: 6대 설계 반영·Tier 1/2 및 코어 격리 확정·C++ 쌍둥이 프로젝트(`~/src/cpp-logger`) 생성·Git 첫 커밋(`4d6d39a`) 완료 및 다음 시작점(실행 구조 결정) 동기화 | INT-001 |
| requirement_spec / domain_model / interface_contract | 플랫폼 역할 계층화(Tier 1 Linux RT: Full SLA/성능 보장 vs Tier 2 Win/Mac: 기능 호환/Best-effort) 및 백그라운드 I/O 워커 CPU 코어 격리(Affinity/Pinning) 명문화 반영 | INT-001 |
| domain_model / interface_contract | 6대 설계 고려사항(RT 메모리 역방향 회수·타임스탬프 삼원화·스키마 진화·ENOSPC 페일세이프·OS별 영속/락 플래그·FR-018 3단계 긴급 차단 프로토콜)을 설계 초안 및 인터페이스 계약에 반영 | INT-001 |
| project_state / my_state / session_2026-10-25_002 | [세션 #002](../members/lim8603/workspace/session_logs/session_2026-10-25_002.md) 종료: API 상세안·FR-018 사용 정책·TDD 승인 반영을 기록하고 다음 시작점을 실행 구조 대안/권고안 검토로 동기화 | INT-001 |
| test_strategy / coding_convention / review_checklist / deliverable_plan / 상태·로그 | Human 최종 승인으로 TDD 개발 순서·실제 환경 검증 병행 명문화, 테스트 시나리오 수집중. 핵심 API 상세안은 계속 리뷰 전이며 Gate 3 미평가 | INT-001 |
| requirement_spec §17 / FUNC-002 / US-002·registry / INT-001 / interface_contract §12 / domain_model / risk_register / 상태·로그 | FR-018 선택 옵션·기본 비활성 확정. 활성 시 UPS 신호 발생→차단 전체 약 3초·중요 파일 우선 기록, 상세 계약/AC·실물 검증 미완료 | INT-001 |
| interface_contract / domain_model / project_state / my_state / session_2026-10-25_002 | 핵심 API 소유권·회수·중요도/포화·완료/실패 상세 권고안과 소비자별 리뷰 시나리오 작성. Human 검토·구현/실행 검증 전 | INT-001 |
| members/lim8603 / .gitignore / project_state·requirement_spec·deliverable_plan | lim8603·solo 기록 체계 초기화와 기존 작업 복원을 마치고 [세션 로그](../members/lim8603/workspace/session_logs/session_2026-10-25_001.md)에 종료 요약·다음 시작점을 남김 | INT-001 |
| INT-001 / requirement_spec §9·16 / NFR-007·US-005 / ADR-006·ADR-001·registry / tech_stack / risk_register / deliverable_plan | 3개 OS 공통 지원 추가, 대표 센서를 UHD급2D LiDAR1개·카메라4개로 일반화 | INT-001 |
| domain_model / interface_contract / functional_spec | 모듈·OS/실행/자원 경계·핵심 계약 초안. 실제 채택/구현 전 | INT-001 |
| ADR-005 / requirement_spec §12·15 / Story·AC / intent_registry / deliverable_plan / project_state | Human 「확정」 반영, Gate 1·2 Pass/Design. 승인과 성능 달성 구분 | INT-001 |
| 요구사항 §13~14 / FR-014~017·NFR-006 / 관련 조사 | 운영 관측·한계 확인·설정/간편 점검·개발자 조절·전후 비교 핵심화 | INT-001 |
| imported_context/research_2026-10-25_* | 공식 출처·버전/수치 스냅샷·센서/TCP/버퍼 계산·ROS/시각화/리포트 조사. 당시 조건을 보존 | INT-001 |

## 활성 산출물

| 유형 | ID | 제목 | 상태 | 비고 |
|------|----|------|------|------|
| Intent | INT-001 | 재사용 가능한 실시간 로보틱스 로깅·텔레메트리 | Approved | 초기 성능/AC·계획·설계 이월 및 추가 플랫폼 요구 반영 |
| Milestone | 없음 | - | - | - |
| User Story | US-001~005 | 수집·보존·활용·관측/조절·플랫폼 | Approved | 기존 설계 기준선/지원 방향. US-002의 FR-018 추가 AC-6~10은 Draft, 구현·검증 전 |
| Task | 없음 | - | - | - |

---

## 컨텍스트 로딩 가이드 (Context Loading Guide)

### 핵심 규칙

- 작업 시작 시 이 문서(`project_state.md`)에서 현재 제약과 다음 시작점을 확인한다. 산출물 범위가 관련되면 `02_project_definition/deliverable_plan.md`를 읽는다.
- 개인 담당이나 이전 맥락이 필요하면 `members/<이름>/workspace/my_state.md`와 최신 세션 로그를 읽는다. 팀 배정이 관련되면 `members/team_board.md`를 읽는다.
- 현재 작업과 관련된 목록 문서를 먼저 읽고 필요한 기준 본문과 상세 문서를 추가한다. 같은 Phase의 문서를 일괄 로드하지 않는다.
- `templates/`, `imported_context/`, `06_evolution/state_archive.md`, 오래된 세션 로그는 기본적으로 로드하지 않는다.
- `06_evolution/state_archive.md`는 과거 세션 완료 서사·핸드오프 이력 아카이브(Log/Archive)다. 이 문서의 요약이 포인터로 가리키며, 특정 과거 세션 맥락이 필요할 때만 해당 `#NNN 이관분` 섹션을 연다.
- `cowork.md`, `session_protocol.md` 등 규칙 문서는 현재 작업에서 관련 규칙이 필요할 때 해당 섹션을 참조한다.
- imported context는 필요한 사실을 추출해 기준 문서에 반영한 뒤 보조 근거로만 활용한다.

### 권장 로딩 순서

1. `project_state.md`에서 현재 상태와 다음 시작점 확인
2. 현재 작업과 관련된 목록 문서·기준 본문, 필요한 상세 문서 확인
3. 산출물 범위·개인 담당·과거 결정이 관련될 때만 `deliverable_plan.md`, `my_state.md`, 최신 로그, ADR 추가

### 단계별 로딩 맵 (Phase Map)

| Phase | 작업과 관련될 때 우선 참조 | 필요 시 참조 |
|-------|----------|-------------|
| **Define** | `02_project_definition/intent_registry.md`, `02_project_definition/user_story_registry.md`, `02_project_definition/requirement_spec.md`, `02_project_definition/functional_spec.md`, `02_project_definition/risk_register.md`, `02_project_definition/deliverable_plan.md` | `02_project_definition/intents/INT-*.md`, `02_project_definition/user_stories/US-*.md`, `02_project_definition/domain_glossary.md` |
| **Design** | `03_design_artifacts/adr_registry.md`, `03_design_artifacts/domain_model.md`, `03_design_artifacts/interface_contract.md`, `03_design_artifacts/data_model.md`, `03_design_artifacts/tech_stack.md` | `03_design_artifacts/adrs/ADR-*.md`, `02_project_definition/requirement_spec.md`, `02_project_definition/functional_spec.md`, `03_design_artifacts/ui_spec.md` |
| **Build** | `04_implementation/milestone_registry.md`, `04_implementation/task_registry.md`, `04_implementation/coding_convention.md`, `04_implementation/review_checklist.md` | `04_implementation/milestones/MS-*.md`, `04_implementation/tasks/TASK-*.md`, `03_design_artifacts/interface_contract.md`, `03_design_artifacts/data_model.md` |
| **Verify** | `05_verification/test_strategy.md`, `05_verification/test_case.md`, `05_verification/verification_evidence.md`, `04_implementation/task_registry.md`, `05_verification/quality_gate.md` | `04_implementation/tasks/TASK-*.md`, `02_project_definition/requirement_spec.md`, `03_design_artifacts/*` |
| **Evolve** | `06_evolution/*` | 필요에 따라 전체 프로젝트 문서 |
| **Deliver** | `07_delivery/*`, `05_verification/quality_gate.md`, `05_verification/verification_evidence.md` | `02_project_definition/deliverable_plan.md`, 전체 프로젝트 문서 |

---

## 작성 / 유지 규칙

- 이 문서는 세션 로그를 대체하지 않는 공유 재개 인덱스다.
- 표와 요약 섹션에는 현재 실제 값만 적고, 값이 없으면 `없음`으로 적는다.
- `INT-*`, `MS-*`, `TASK-*` 표기는 형식 안내일 뿐이며, 실제 값이 생기면 바로 교체한다.
- `한 줄 상태`, `현재 작업 스트림`, `다음 시작점`, `AI 핸드오프 메모`는 보통 3~5줄 이내로 유지한다.
- 같은 사실을 여러 섹션에 반복하지 말고, 요약 1회 + 관련 ID/문서 경로로 연결한다.
- 세션 로그의 raw 메모, 미확정 가설, 1회성 디버깅 흔적은 그대로 복사하지 않는다.
- R1: 핵심 필드는 현재 값으로 교체한다. 최근 완료 요약은 최대 3세션 × 한 문장 + 링크만 유지한다. 상세는 원문을 보존하여 `state_archive.md`로 이관하며 이미 보관된 내용은 재복제하지 않는다.
- **R2 — 현재 제약은 보존하고 상세는 분리.** 현재 작업에 필요한 내용도 긴 서사 그대로 유지하지 않는다. 각 활성 항목에는 ID, 상태, 현재 제약/블로커, 다음 행동, 근거 링크를 남기고 상세는 기존 Task/ADR/기준 문서로 연결한다. 단일 표 행 또는 문단이 800자를 넘으면 분리 검토한다. 미해결 조건·승인·검증 공백을 삭제하거나 완료로 바꾸지 않는다. 이관 후 대상과 링크를 확인한다.
- **크기 경고선 (F-05).** UTF-8 파일 전체 크기를 기준으로 `project_state.md` 24 KiB, 활성 `my_state.md` 각 12 KiB를 기본 경고선으로 삼는다. 상태 문서를 갱신하거나 인계할 때 확인하고 초과 시 R1/R2에 따라 정리한다. 크기만으로 작업·릴리즈를 차단하지 않는다.
- `최근 변경 파일 / 산출물`, `Human 확인 필요 사항`, `주요 리스크`는 항목이 없더라도 `없음` 상태를 명시한다.


## 2026-10-26 TASK-002 상태 동기화 시 이관

- 2026-10-25 #002: 핵심 API 초안·FR-018 기본 off·TDD 승인 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-25_002.md).

## 2026-10-28 #003 상태 인덱스 이관

- 2026-10-27 #003: TASK-004 구현·EV-003 로컬/Miri Pass·Human 승인·게시 완료, CI 계정 사유 차단·세션 종료 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-27_003.md).

## 2026-10-29 #001 세션 마감 상태 인덱스 이관

- 2026-10-28 #001: Human 사용량 소진 확인·TASK-005 선행 CI 대기 중 착수 지시, 구현·EV-004 로컬 Pass·Human 리뷰 승인 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-28_001.md).
- 이전 개인 상태의 TASK-005/006 목표·결과는 [2026-10-28 #001](../members/lim8603/workspace/session_logs/session_2026-10-28_001.md), [#002](../members/lim8603/workspace/session_logs/session_2026-10-28_002.md), [#003](../members/lim8603/workspace/session_logs/session_2026-10-28_003.md) 및 TASK-006/EV-007에 있다. 현재 my_state는 활성 MS-003/TASK-007과 최신 로그만 유지한다.

## 2026-10-29 #002 세션 마감 상태 인덱스 이관

- 2026-10-28 #002: TASK-006 구현·EV-005 로컬 Pass·Human 승인·커밋·push·세션 종료, native 이월 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-28_002.md).
- 이전 개인 상태의 2026-10-29 #001 세션 Intent: INT-001 / MS-002·003 / TASK-006·007 — 개선 검토·종료 경합 수정·MCAP 파일 구현/검증. 결과 TASK-006 Human 승인, TASK-007 EV-010 로컬 Pass — [로그](../members/lim8603/workspace/session_logs/session_2026-10-29_001.md).

## 2026-10-30 #003 상태 인덱스 이관

`project_state.md` 최근3세션을 #003/#002/#001로 정렬하며 기존 요약을 보존한다.

- 2026-10-29 #004: 리뷰 절차 공식화·`reviews/` 신설·REV-001~003 복원·문서/진입점 갱신 및 커밋 푸시 마감 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-29_004.md).
- 2026-10-29 #003: RV-006~010 수정·EV-011 로컬 회귀 Pass·소스 c932ee4 게시·세션 종료, 최신 구현 Human 리뷰/native·장비 이월 — [수정 로그](../members/lim8603/workspace/session_logs/session_2026-10-29_003.md).

## 2026-10-30 #003 세션 마감 — 완료 서사·상세 재개 메모 이관

공유 상태 R1/R2에 따라 원문을 보존한다. 현재 승인·검증 공백·다음 행동은 project_state.md에 유지한다.

- **2026-10-30 TASK-009 로컬 결과:** 불변 원본의 완전 검증 chunk prefix만 별도 파일로 복구·index 재생성/file sync·선택 namespace finalize/directory sync를 구현했다. [EV-014](../05_verification/verification_evidence.md#ev-014) source51개/검사10개 exit0, 복구19개·두 Rust 각16 features/consumer/예제/lint/doc/fmt·allocator9·Miri117×8=936 Pass다. [REV-011](../05_verification/reviews/REV-011_TASK-009.md) RV-015 최종 sync 중 취소는 Red→Green·Resolved다. 신규 구현은 로컬 리뷰 대상이며 Human 최종 리뷰/native·실물은 잔여다. 자동 startup 발견/기존 namespace 재개·reader·worker/StorageDegraded를 완료한 것으로 보지 않는다.

- **2026-10-30 게시·후속 착수:** Human 「커밋 푸쉬 후, 다음 테스크 진행」으로 EV-013 소스 `7102d93`/문서의 분리 커밋·게시와 TASK-009 착수를 승인했다. source49개 commit blob/EV-013 hash가 일치한다. TASK-008 REV-009/010 Resolved·최종 구현 리뷰 및 native GAP-004는 유지하며 후속 개발을 진행한다. CI 차단 재시도·계정 조회는 하지 않는다.

- **2026-10-30 개발 세션 #003:** Human의 브리핑·리뷰 지적사항 처리·TASK-008 요약 요청으로 RV-012~014를 조치했다. RV-013 중복 probe 제거·RV-014 symlink 의도 문서화, RV-012는 승인된 만료/Drop 후 실제 release 전 삭제 금지 계약을 유지하며 `must_use`/문서/회귀와 TASK-010 리더 수명 관리에 연결했다. 토큰 유실 위험은 잔여이며 기한만의 강제 회수를 도입하지 않는다. 최종 검사14개 exit0·보존 Miri128회 Pass이며 [EV-013](../05_verification/verification_evidence.md#ev-013)에 고정했다. 초기 브리핑/처리 요청 자체는 최종 구현 승인·commit/push·TASK-009 착수 지시가 아니었으며, 게시·후속 착수는 뒤이은 Human 요청으로 별도 승인됐다.

- **2026-10-30 리뷰 세션 #002:** Human 「리뷰세션 시작하자」 및 「그동안 리뷰 안건의 처리가 올바르게 되었는지도 리뷰해볼까?」에 따라 리뷰 전용 세션을 수행했다. 제품 코드 수정 없이 [REV-010](../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md)(Open, RV-012~014)을 생성하고, 과거 전체 리뷰 안건(RV-001~011)에 대한 전수 감사를 실시하여 단 하나의 회귀 없이 100% 완전 반영되어 있음을 교차 검증했다.

- **2026-10-30 승인·구현:** Human 「승인, TASK-008 진행하자.」로 TASK-007 최신 구현을 승인하고 REV-008을 Closed로 마감했다. TASK-008은 [EV-012](../05_verification/verification_evidence.md#ev-012) 로컬 Pass·REV-009 Resolved이며 Human 구현 리뷰/native GAP-004·장비는 잔여다.

- **2026-10-29 리뷰 절차 공식화:** Human 「프로젝트 리뷰처리 절차를 공식화 하려고해... 승인」에 따라 `05_verification/reviews/` 디렉터리를 신설하고 과거 리뷰 내역(REV-001~003)을 복원했다. 리뷰 세션의 코드 수정 금지(Read-Only), REV 문서 자산화, 리뷰 상태 추적(`Open` → `In Progress` → `Resolved` → `Closed`), 브리핑 시 미처리 리뷰 상시 보고 규칙을 프로토콜 및 진입점(AGENTS/GEMINI)에 공식 반영했다.

- **2026-10-29 게시·마감:** Human 「커밋 푸쉬하고 세션 마감해줘」에 따라 소스 `c932ee4`를 origin/main에 게시했다. EV-011 source46개와 해당 commit blob의 SHA-256이 전부 일치한다. 소스/문서는 분리 커밋·[skip ci]로 관리하며 마감 문서를 별도 게시한다. 이 게시 요청 자체는 구현 승인으로 확대하지 않았으며, 최종 구현 승인은 2026-10-30 별도로 받았다.

- **2026-10-29 리뷰 수정:** Human 「코드리뷰 받은 내역을 확인하고 수정하자는 말이였어」로 RV-006~010 수정 범위·방향을 승인했다. 배치 Written·진행 prefix, pre-IO SyncWork 반환·공통 Busy 매핑, 예산 내 쓰기 버퍼와 report-only 재시도를 구현했다. [EV-011](../05_verification/verification_evidence.md#ev-011) 로컬 Pass이며 최종 구현 Human 승인은 2026-10-30 완료됐다. native·장비는 잔여다.

- **2026-10-29 승인·착수:** Human 「승인」으로 TASK-006 RV-001~005 최신 수정분을 승인하고 native 이월 상태의 TASK-007 진행을 승인했다. TASK-006/EV-009 로컬 Pass·native GAP-004는 유지하며 MCAP 봉투/writer/인덱스를 구현하고 EV-010 로컬 검증을 마쳤다. 당시 TASK-007 Human 리뷰/native는 대기였으며 착수 승인은 commit/push 지시가 아니었다. 최종 구현 Human 승인은 2026-10-30 완료됐다.

- **2026-10-29 후속 수정:** [EV-008](../05_verification/verification_evidence.md#ev-008)의 RV-005 P1을 Human 수정 지시 후 [EV-009](../05_verification/verification_evidence.md#ev-009)에서 해결했다. 일반 sync 진입/게시와 shutdown을 단발 원자적 표시·RAII·SeqCst로 순서화했다. 정식 반례2개 Red 후 수정·private3개, 두 Rust 각16 features/consumer/예제/clippy/rustdoc/fmt·Miri81×8·RT0/0 Pass, source38개 고정이다. GAP-007 로컬 Closed, 최신 수정분 Human 승인은 완료됐고 native는 잔여다. TASK-006 보고에 RV-001~005 수정 반영사항을 함께 정리한다. commit/push·CI/과금 조회는 하지 않았다.

- **이전 수정/게시 이력:** RV-001~004의 EV-007 로컬 회귀와 코드 16f58de 게시 이력은 [TASK-006](../04_implementation/tasks/TASK-006.md)에 보존했다. GAP-006/007은 로컬 수정 범위에서 Closed, TASK-006 최신 수정분 Human 승인은 완료됐고 native GAP-004는 잔여다. 현재 게시 소스 `c932ee4`는 EV-011을 기준으로 검토한다.

- **자가 리뷰 이력:** [EV-006](../05_verification/verification_evidence.md#ev-006)은 source36개에서 P1 반례3개, EV-008은 EV-007과 같던 source37개에서 RV-005를 확인한 수정 전 기록이다. 실패/승인 이력은 보존하고 TASK-006 수정 근거는 EV-009, TASK-007 현재 근거는 EV-011을 사용한다.

- **형상관리 제약:** 소스와 `.cowork` 문서는 별도 커밋으로 관리한다. Human 「커밋 푸쉬 후, 다음 테스크 진행」에 따라 EV-013 소스/문서를 분리 커밋·게시하고 TASK-009 개발을 진행한다. 게시·후속 착수 지시를 TASK-008 최종 구현 승인으로 확대하지 않는다. 인증 정보는 OS keyring에 두고 저장소에 넣지 않는다.

## 새 세션 재개 메모

- 이 문서 → TASK-009 상세·REV-011·EV-014·TC-005를 읽는다. TASK-008/009 최종 Human 리뷰/native·장비가 잔여다.
- 개발1.96.0·MSRV1.85·dependency/lock·private unsafe 경계를 유지한다. Miri nightly-2026-10-26은 검증 전용이며 전역 default는 변경하지 않았다. 현재 source51개 hash·명령/log 근거는 EV-014다. EV-012/013은 TASK-008 검증 이력이다.
- producer별 완료 보호량 R=max(small/raw Important slots), W≥R다. Normal은 다음 R개 cyclic slot의 Empty/Terminal 상태를 확인해 보호하며 W=R은 Important 전용이다. 기존 consumer 예제는 Important16에 맞춰 window32/work_per_poll32로 정렬했다. 실제 RT p99는 미검증이다.
- `new_tracked`는 producer별 completion window를 입력 수락 전에 단발 예약한다. pending/Updating slot을 덮지 않고 CompletionCapacity로 무대기 거절한다. raw 거절/생략은 같은 loan이며 admission/loan call의 SeqCst 표시가 close/cutoff 동결을 순서화한다.
- Completion은 boundary 확인과 failed/dropped/unknown/expired를 분리한다. 고정 range/page/query permit·deadline/cancel wait와 완료 이력 age/window, retired terminal counters를 제공한다. query는 sync/출력 취소를 실행하지 않는다.
- Router는 유한 feedback으로 Busy 결과를 보존하고 source memory 분할·pool layout·초기 completion route를 대조한다. 원본은 기존 guard 하나를 공유하며 distinct raw 물리 예약·output 보유 원인/복사·completion/control metadata를 분리한다. 일반 guard 무보고 Drop은 Unknown, 오래된 Drop은 새 generation을 덮지 않는다.
- ControlOwner는 유일 controller lease·session/epoch/slot/generation·ordinary/전용 shutdown slots를 가진다. 명시 barrier만 writer 대역에 넘기고 같은 target의 sync IO를 직렬화한다. 만료 shutdown sync는 실제 IO/loan/feedback와 선행 일반 sync 종료 후 유한 cursor로 미확인 Synced를 Unknown으로 정리한다. 일반 요청 만료는 기록을 바꾸지 않는다. Closed의 is_success와 실패/불명/기한 보고를 별도로 확인한다.
- SyncSubmission은 일반 sync의 진입~게시/오류 반환을 단발 CAS·RAII로 표시한다. close 전 열림을 본 요청이 끝나기 전에는 cutoff 동결/Closed를 확정하지 않는다. 동시 일반 요청 Busy·close 이후 Closing, 실제 guard/만료 정리 계약은 유지한다. test-only 스케줄러는 스레드별 cfg(test)다.
- TASK-007은 empty File/SyncSink를 일반 경로에서 구동한다. write_batch는 호출자 guard를 chunk target에 따라 묶고 완전 flush 뒤 Written을 보고한다. BatchWriteError의 appended/written/reported prefix로 중복 쓰기를 피하고 report만 재시도한다. write_buffer_bytes(0=direct)는 공유 예산에 포함하고 seek/flush 때 배출, Drop은 IO 없이 폐기한다. 인덱스 쌍은 min(records,chunks×channels)로 제한한다. sync 거절/Busy는 원래 work를 반환하고 실제 IO 실패만 Failed다. 다른 세그먼트의 pending/만료 범위를 새 파일 sync로 확인하지 않는다.
- TASK-008 SegmentLog는 유한 catalog/event/pin·공유 출력 예약·file/index bytes 상한·세 모드·multi-segment sync를 구동한다. event 실제 범위는 coarse mono extents이고 gap-free 보장이 아니다. 핀 만료 후에도 실제 release 전에는 삭제하지 않는다. 표준 FileStore는 Unix directory sync만 구현하며 미지원은 거절한다. host free-space 실측/다른 OS adapter·장비·복구/active reader·worker 기동은 후속이고 TC 전체·MS-002/003·Gate4/5 완료를 선언하지 않는다.

- TASK-009 복구는 원래 session/revision/생산자 수·동일 순서의 등록과 read-only 불변 source를 요구한다. 현재 무압축/nonzero CRC 형식의 완전 chunk만 검증/복사하며 손상 뒤 재동기화하지 않는다. source/sink host 메모리는 별도, 새 파일 sync와 과거 completion 응답은 구분한다. 미지원 압축·한계·취소·IO cause/prefix를 보고하며 실패 파일은 삭제/자동 재시도하지 않는다. 자동 startup 발견/기존 namespace 재개·StorageDegraded 계측·worker는 후속이다.

### 개인 상태 완료 서사 이관 — 2026-10-30 #003 마감

## 최근 세션 이력

- #001: TASK-007 승인·REV-008 Closed, TASK-008 회전/보존/이벤트 구현·EV-012 Pass·REV-009 Resolved. 소스 e6669ac 로컬 커밋.
- #002: TASK-008 심층 코드리뷰([REV-010](../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md) Open, RV-012~014 지적) 및 과거 전체 리뷰 안건(RV-001~011) 전수 무회귀 검증 완료. 제품 코드 수정 없이 리뷰 세션 정상 마감.

- #003: REV-010 지적 처리·EV-013 로컬 Pass·REV-010 Resolved. 보존17개·두 Rust core/all-features/consumer/예제/allocator8/lint/doc/fmt·보존 Miri128회 Pass. pin 유실 위험은 TASK-010에 연결.

- #003 후속: Human 「커밋 푸쉬 후, 다음 테스크 진행」으로 7102d93/bf3acf0 게시 후 TASK-009 구현·EV-014 Pass·REV-011 Resolved. 복구19개·두 Rust16조합/consumer/예제/lint/doc/fmt·allocator9·Miri936 Pass. 신규 TASK-009는 로컬 리뷰 대상.

## 2026-10-30 #007 마감 — 이전 세션 요약 이관

- 2026-10-30 #004: REV-012 조치·TASK-009 승인/게시·REV-011/012 Closed 후 TASK-010 EV-016 Pass·REV-013 Resolved, 후속 소스4a54bcf 로컬 커밋·미게시/Human 리뷰 대기로 종료 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_004.md).
- 2026-10-30 #003: TASK-008 조치 게시·TASK-009 복구 구현/EV-014 Pass·REV-011 Resolved 및 소스5bbb45c 게시 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_003.md).
- 2026-10-30 #002: TASK-008 심층 리뷰·REV-010 Open 등록·과거 리뷰 안건 감사 — [리뷰 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_002.md).

## 2026-10-31 #006 상태 인덱스 이관

- 2026-10-31 #003: REV-017 조치/EV-020 Pass 후 TASK-008/010/011 승인·리뷰5건 Closed, TASK-022 10ms 수치 승인·EV-021 준비 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_003.md).
- 2026-10-31 #002: TASK-011 최종 안전성 코드리뷰·REV-017 Open 등록(제품 미수정) — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_002.md).

## 2026-10-31 #007 상태 인덱스 이관

- 2026-10-31 #004: TASK-012 구현·EV-022 Pass·d63fdea 소스/문서 분리 게시 후 종료·REV-018 Human 리뷰 대기 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_004.md).

## 2026-10-31 #008 상태 인덱스 이관

- 2026-10-31 #005: 프로젝트 종합 심층 코드리뷰 수행·REV-019 등록(RV-036~038 관측, 제품 코드 미수정)·저장 및 원격 동기화 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_005.md).


## 2026-10-31 #014 상태 문서 정리 — 이전 승인·재개 메모

## 승인·권한 위임 상태 — 2026-10-31 #007

- #012 종료: Human 「세션 마무리해줘」로 세션을 마감한다. TASK-015 로컬 수락/EV-028·REV-024 Closed와 기존 위임은 유지하며 이번 마감에서 TASK-016 구현이나 추가 시험을 시작하지 않는다.
- #012 Human 「프로젝트 진행현황 파악하고, TASK-015 시작하자」로 승인된 TASK-015를 재개했다. 로컬 구현/검증·리뷰 조치 수락과 작업 단위별 커밋/푸시는 기존 위임으로 진행하며 TASK-016은 아직 구현하지 않았다. #011 종료 지시의 미착수 표시는 당시 이력이다.
- Human의 기존 모든 권한/게시 위임과 #008 TASK-013 진행 지시를 유지한다. EV-025 검증 뒤 AI가 로컬 구현을 수락하고 REV-021을 마감했다. 같은 범위의 최종 승인 요청은 반복하지 않는다.
- 구현/검증 결과와 위험·미완료를 보고하며, 미실행 native/PC 시험이나 제품 전체 수락을 임의로 Pass로 바꾸지 않는다. 공개 전환/선택 기능 자동 착수 지시로 확대하지 않는다.

## 새 세션 재개 메모

- 저장 문제 원인 정정(2026-10-31 #006): Human이 이전 파일 저장 문제는 Antigravity 문제였다고 확인했다. #005의 VS Code pending edits 원인 확정은 당시 진단 이력으로 보존하며 현재 원인 판단으로 사용하지 않는다. 당시 설정 변경의 효과는 이번에 재검증하지 않았고 사용자 에디터 설정도 변경하지 않았다.
- REV-019 EV-023 조치·검증 후 Closed. 포화 대기/고정 credential 비교·receiver 지침 보강을 수락했으며 전체 인증 시간 특성은 미검증이다. gate 자동 삭제 미도입. TASK-013은 같은 수명 계약을 소비한다.
- TASK-022 수치 Approved·EV-021(프로필 준비/승인)과 EV-022(TASK-012 제품 로컬 Pass)을 구분한다. TASK-008/010/011/012 로컬 리뷰·TASK-022 수치 승인은 완료됐다. TASK-013 개발/후속 검증은 별도다.
- 개발1.96.0·MSRV1.85·nightly-2026-10-26 Miri 고정, dependency/lock·RT 무할당/무대기·private unsafe 경계를 유지한다. 전역 rustup default는 바꾸지 않았다.
- 완료/종료·writer 배치/재시도 의미는 API§13·TASK-006~008을 따른다. 낮은 ReadPin 유실/forget·종료 불명 background IO는 강제 회수하지 않고 reader-owned permit만 실제 reader 종료 후 반환한다.
- 자동 startup 발견/기존 namespace 재개·운영 worker/StorageDegraded·실앱 연동은 후속이다. remote/client/CLI는 EV-025 로컬 범위이며 자동 운영 worker를 시작하지 않는다. 실제 성능·RT·UPS·24h·다른 OS/native·TC 전체·MS-002/003/004·Gate4/5 완료를 선언하지 않는다. decoder/runtime/source/sink·host 메모리와 OS cache는 logger 예약 밖이다.



## 2026-10-31 #014 이관 — REV-001~025 직전 상세 목록

## 코드리뷰 현황 (Reviews)

| ID | 대상 | 일자 | 성격/판정 | 상태 | 요약 및 다음 액션 |
|---|---|---|---|---|---|
| [REV-001](../05_verification/reviews/REV-001_TASK-002.md) | TASK-002 | 2026-10-27 | 구성·파이프라인 구현 / Pass | Closed | EV-001 로컬/native CI 7개 Pass, Human 승인 완료 |
| [REV-002](../05_verification/reviews/REV-002_TASK-003.md) | TASK-003 | 2026-10-27 | 소형 수집·중요 예약 / Pass | Closed | EV-002 RT 무할당/무대기, native CI 7개 Pass, Human 승인 완료 |
| [REV-003](../05_verification/reviews/REV-003_TASK-004.md) | TASK-004 | 2026-10-27 | 원본 풀·수명·안전성 / Pass | Closed | EV-003 generation/state, Miri 8 seeds Pass, Human 승인 (native 대기) |
| [REV-004](../05_verification/reviews/REV-004_TASK-005.md) | TASK-005 | 2026-10-28 | 분배·출력 격리·예산 / Pass | Closed | EV-004 공정 순회/가드 공유, Miri 19×8 Pass, Human 승인 (native 대기) |
| [REV-005](../05_verification/reviews/REV-005_TASK-006_RV-001-004.md) | TASK-006 | 2026-10-28 | 설계 대조 자가리뷰 / 보류 (P1 3, P2 1) | Closed | RV-001~004 EV-007 수정·로컬 Pass, commit `16f58de`, Human 승인 완료 |
| [REV-006](../05_verification/reviews/REV-006_TASK-006_DEEP_REVIEW.md) | TASK-006 | 2026-10-28 | 제미나이 심층 코드리뷰 / Note/Suggestion | Closed | False Sharing·O(N) 선형탐색 등 5건 분석, rustdoc 보강/장비 과제 연결 |
| [REV-007](../05_verification/reviews/REV-007_TASK-006_RV-005.md) | TASK-006 | 2026-10-29 | 종료 경합 자가리뷰 / Conditional (P1 1) | Closed | RV-005 EV-009 수정·로컬 Pass, Human 승인 완료 |
| [REV-008](../05_verification/reviews/REV-008_TASK-007_RV-006-010.md) | TASK-007 | 2026-10-29 | MCAP 저장 구현 리뷰 / Conditional (P2 3, P3 2) | Closed | RV-006~010 EV-011 로컬 수정 Pass, commit `c932ee4`, 2026-10-30 Human 승인 완료 |
| [REV-009](../05_verification/reviews/REV-009_TASK-008.md) | TASK-008 | 2026-10-30 | 회전·보존 자가 리뷰 / Conditional (P2 1, 로컬 해결) | Closed | RV-011 EV-012 수정/회귀 Pass, 2026-10-31 Human 승인·실측/native 후속 |
| [REV-010](../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md) | TASK-008 | 2026-10-30 | 회전·보존 심층 코드리뷰 / Conditional (P2 1, P3 2) | Closed | EV-013 로컬 Pass, RV-012 위험/후속 TASK-010 유지·2026-10-31 Human 승인 |
| [REV-011](../05_verification/reviews/REV-011_TASK-009.md) | TASK-009 | 2026-10-30 | 복구·IO 장애 자가 리뷰 / Conditional (P2 1, 로컬 해결) | Closed | RV-015 EV-014 수정·회귀 Pass, 2026-10-30 Human 승인·native 후속 |
| [REV-012](../05_verification/reviews/REV-012_TASK-009_DEEP_REVIEW.md) | TASK-009 | 2026-10-30 | 심층 코드리뷰 / Pass (P3 2) | Closed | RV-016/017 EV-015 로컬 Pass·소스5c60399·2026-10-30 Human 승인 |
| [REV-013](../05_verification/reviews/REV-013_TASK-010.md) | TASK-010 | 2026-10-30 | 조회/읽기 수명 자가 리뷰 / Conditional (P2 3) | Closed | RV-018~020 조치·EV-016 로컬 Pass, 2026-10-31 Human 승인·native/실측 후속 |
| [REV-014](../05_verification/reviews/REV-014_TASK-010_CODE_REVIEW.md) | TASK-010 | 2026-10-30 | 심층 코드리뷰 / Pass (P2 1, P3 2) | Closed | RV-021~023 정렬 캐시·rustdoc 조치·EV-017 로컬 Pass, 2026-10-31 Human 승인·native/실측 후속 |
| [REV-015](../05_verification/reviews/REV-015_TASK-011.md) | TASK-011 | 2026-10-30 | 재생/시간·자원 수명 자가 리뷰 / Conditional (P2 2, 로컬 해결) | Closed | RV-024/025 조치·EV-018/019 로컬 Pass·2026-10-31 Human 승인, native·장비 잔여 |
| [REV-016](../05_verification/reviews/REV-016_TASK-011_CODE_REVIEW.md) | TASK-011 | 2026-10-30 | 로컬 재생·시간 제어 심층 코드리뷰 / Pass (P2 1, P3 2) | Closed | 2026-10-31 RV-026 현 불변식상 overflow 불가·방어 정리, RV-027/028 진단 API 조치·EV-019 Pass·2026-10-31 Human 승인·native/장비 잔여 |
| [REV-017](../05_verification/reviews/REV-017_TASK-011_FINAL_REVIEW.md) | TASK-011 | 2026-10-31 | 로컬 재생 구현 조치·안전성 최종 리뷰 / Pass (P2 1, P3 2) | Closed | RV-029 국소 경고 억제/RV-030 비율 비교/RV-031 오류 진단 조치·EV-020 Pass·2026-10-31 Human 추가 조치 승인 |
| [REV-018](../05_verification/reviews/REV-018_TASK-012.md) | TASK-012 | 2026-10-31 | 원격 adapter 자가 리뷰 / Conditional (P2 4건 로컬 조치) | Closed | RV-032~035 조치·EV-022 Pass·2026-10-31 Human 로컬 구현 수락, native/실측 후속 |
| [REV-019](../05_verification/reviews/REV-019_TASK-012_PROJECT_CODE_REVIEW.md) | TASK-012 | 2026-10-31 | 원격 adapter·프로젝트 종합 심층 코드리뷰 / Pass (P2 2, P3 1) | Closed | RV-036~038 조치·EV-023 검사9개/Miri25 Pass·Human 위임에 따른 AI 마감 |
| [REV-020](../05_verification/reviews/REV-020_TASK-013_RECEIVER_START.md) | TASK-013 첫 단위 | 2026-10-31 | 연결 수명 API 개발 자가 점검 / Pass | Closed | EV-024 통합3개·검사12개·Miri24 Pass·위임된 첫 단위 수락, transport/CLI 후속 |
| [REV-021](../05_verification/reviews/REV-021_TASK-013_CLIENT_CLI.md) | TASK-013 로컬 구현 | 2026-10-31 | client/CLI 개발 자가 점검·조치 / Pass | Closed | RV-039~045 조치·EV-025 검사15개/Miri32·위임된 AI 로컬 수락, native/실앱 후속 |
| [REV-022](../05_verification/reviews/REV-022_TASK-013_INDEPENDENT_REVIEW.md) | TASK-013 `6863268`·TASK-012 `ca1694a` | 2026-10-31 | 독립 Read-Only 리뷰·서버 경계 확장 / Pass (P2 2, P3 8) | Closed | EV-026 검사14개/Miri32·위임 수락. RV-049 TASK-022 실측·RV-053 TASK-020 장기 tail/화면 후속, wire 확장 없음 |
| [REV-023](../05_verification/reviews/REV-023_TASK-014_TELEMETRY.md) | TASK-014 3164f0d | 2026-10-31 | 개발 자가 점검 / 로컬 Pass | Closed | EV-027 검사14개/Miri56·위임 수락; 비용/RT/PC/native/실앱 후속 |
| [REV-024](../05_verification/reviews/REV-024_TASK-015_REPORT.md) | TASK-015 95a63a6 | 2026-10-31 | 개발 자가 점검 / 로컬 Pass (P2 3·P3 1 조치) | Closed | RV-056~059 Red→Green·EV-028 source100개/검사13개·Miri168·소스95a63a6 위임 수락; PC/RT/native/실앱 후속 |
| [REV-025](../05_verification/reviews/REV-025_TASK-014-015_INDEPENDENT_REVIEW.md) | TASK-014 3164f0d·TASK-015 95a63a6 | 2026-10-31 | 독립 Read-Only 리뷰 / Conditional (P2 1, P3 2) | Open | RV-060 거절 범주 혼합→허위 Overloaded·부하율 과대(API§15.1 문구 변경 동반, 권한 수준 확인), RV-061 대상별 OutputDelta 정렬 수단 부재, RV-062 이력 poison 미복구. TASK-016 착수 전 처리 권고 |



## #014 최근 이력 이관

- 2026-10-31 #011: REV-022 EV-026·TASK-014 EV-027 로컬 수락/게시·세션 종료, 다음 세션 TASK-015 착수 승인 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_011.md).

## 2026-11-01 #002 종료 — 최근 이력 포인터 정리

- 2026-10-31 #012 TASK-015 완료 상세는 [당시 세션 로그](../members/lim8603/workspace/session_logs/session_2026-10-31_012.md)와 [EV-028](../05_verification/verification_evidence.md#ev-028)에 보존한다. 공유 상태의 최근 이력은 #002·#014·#013 세 회차로 유지한다.

## 2026-11-01 #006 상태 인덱스 이관

- 2026-11-01 #003: TASK-016 독립 리뷰 Pass/P3 2건을 REV-028 Open으로 남기고 Human 요청으로 종료했다(후속 #004에서 조치/Closed) — [리뷰](../05_verification/reviews/REV-028_TASK-016_INDEPENDENT_REVIEW.md)·[로그](../members/lim8603/workspace/session_logs/session_2026-11-01_003.md).


## #015 — 2026-11-01 #007 상태 갱신의 이전 완료 요약 이관

- 2026-11-01 #004: REV-028 RV-063/064 조치/Closed 및 TASK-017 S1 계획·예산을 로컬 수락했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_004.md)·[EV-031](../05_verification/verification_evidence.md#ev-031).


## #015 — 2026-11-01 #010 상태 인덱스 R2 계약 설명 이관

현재 동작·제약은 관련 Task/API 기준 본문에 유지하며 아래는 압축 전 원문이다.

- **TASK-011 현재 계약:** frozen Snapshot을 소유하는 mono ns 동기식 replay·최대1건 prefetch·양수 u32 유리수 배속·pause/resume/seek/stop·checked generation과 누적 읽기/원본 수명을 사용한다. 늦은 소비자의 재생 시계를 늦추고 원격 in-flight 취소를 추정하지 않는다. End는 마지막 blob read를 위해 snapshot을 유지하고 stop/Drop·terminal 오류에서 실제 reader close 뒤 반환한다. 상세 [TASK-011](../04_implementation/tasks/TASK-011.md)·[API §14.2](../03_design_artifacts/interface_contract.md#142-task-011-로컬-재생-api--2026-10-30).

- **TASK-010 현재 계약:** 고정 session/revision/원래 등록·독립 read-only source·완전 chunk safe offset, 유한 external/index/page/blob/read 예산을 사용한다. reader-owned permit은 active→closed 회전을 따라 보호하며 실제 reader Drop 뒤에만 회수 가능하다. 저수준 pin 분실/forget의 강제 회수·clock 변환은 도입하지 않는다. 현재 closed도 bounded scan으로 index를 재구성하며 summary scan 생략 최적화·자동 worker/CLI·실제 성능/OS 검증은 후속이며 local replay/원격 host 연결은 EV-018/022 범위다. 상세 [TASK-010](../04_implementation/tasks/TASK-010.md)·[API §14.1](../03_design_artifacts/interface_contract.md#141-task-010-로컬-api--2026-10-30).

- **TASK-009 현재 계약:** 불변 read-only 원본·원래 session/revision/생산자 수/동일 순서의 채널 등록·독립된 빈 출력이 필요하다. 완전 검증 chunk prefix만 복구하며 손상 뒤 재동기화·과거 completion 응답 복원·실패 파일 자동 삭제/재시도는 하지 않는다. 상세: [TASK-009](../04_implementation/tasks/TASK-009.md)·[API §13.5](../03_design_artifacts/interface_contract.md#135-task-009-중단-복구-api--2026-10-30).


## #016 — 2026-11-01 #011 R1 최근 완료 요약 이관

- 2026-11-01 #007: S3 raw/routing·실제 ACK·실앱 snapshot 첫 단위를 수락하고 Human 요청으로 종료했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_007.md)·[EV-034](../05_verification/verification_evidence.md#ev-034).


## #017 — 2026-11-01 #012 R1/R2 상태 인덱스 이관

원문을 보존하며 현재 계약/잔여는 해당 Task/API에 유지한다.

- 2026-11-01 #008: 고정 routed RunCheck·원 source/CLI 권한·호스트 시계 경계를 수락하고 잔여 조건 동기화 후 종료했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_008.md)·[EV-035](../05_verification/verification_evidence.md#ev-035).

- **TASK-017 현재 계약:** EV-031~034·REV-029~032 수락에 [EV-035](../05_verification/verification_evidence.md#ev-035)/[REV-033](../05_verification/reviews/REV-033_TASK-017_ROUTED_HOST.md)를 추가했다. `CheckHost`의 고정 File/Routed 슬롯·단일 사용·원 source 검증·Read/Manage/응답 preflight·native evidence를 유지한다. client `SnapshotCheck`가 host-selected≤64KiB 한 건을 실제 raw/routing→TCP 수신 큐/ACK→원 receipt 완료·원본/예약 회수로 연결한다. host 원 session/revision/mono clock과 source Report/profile는 분리 보존한다. 예제2초/요구24MiB·허용32MiB, 물리 쓰기0이며 capacity/freshness/LAN/RSS 미측정이다. 로컬 기능 구현 잔여는 처리했으나 지속 수집/기존 화면/off-on(TASK-020/026)·통합 PC/RT/UPS/장기(TASK-021~024)·공개 후 native(T3)/다른 OS는 미수락이다. 전체 Task In Progress·TC Partial 유지. [TASK-017](../04_implementation/tasks/TASK-017.md)·[API §15.7](../03_design_artifacts/interface_contract.md#157-task-017-고정-routed-runcheck--2026-11-01).


## #018 — 2026-11-01 #014 R1/R2 상태 인덱스 이관

- 2026-11-01 #011: TASK-019 S2 query·OSS2.9.1 실제 DB를 EV-038/REV-036으로 수락하고 단계 전체 표 브리핑을 반영했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_011.md).

- **TASK-013 기능 기준:** EV-025/source29e55da 뒤 [EV-026](../05_verification/verification_evidence.md#ev-026)/eb39cb2로 handoff·진단/CLI·유휴 next를 보강했다(검사14개/Miri32). local wait timeout은 같은 구독 유지, server lease/명시 cancel은 종료한다. ttl은 record/idle lease 겸용, 별도 구독 간 Gap 연속성은 미보장이고 TASK-020 화면에서 불연속 표시한다. 현재 workspace는 REV-025 EV-029/9fecaae로 전체 회귀했다. CLI runtime metrics Unavailable·config apply/compare backend Unsupported·실앱/native/성능 후속은 유지한다. 점검 host/CLI는 EV-033 범위다.


## #019 — 2026-11-01 #015 R1/R2 상태 인덱스 이관

- 2026-11-01 #012: TASK-019 S3 원 Delivery/완료·로컬 파일 격리를 EV-039/REV-037로 수락했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_012.md).

- **TASK-019 현재 계약:** S1/S2/S3 [EV-037~039](../05_verification/verification_evidence.md#ev-039)/소스`86b1f67`·[REV-037](../05_verification/reviews/REV-037_TASK-019_ROUTED_COMPLETION.md) Closed. 새 독점 namespace·equal/older timestamp 거절·typed write/query·원 schema/receipt/real ns·실제 로컬 IO 종료와 원 guard/게시 수명을 유지한다. DB 정지·포화 중 MCAP6건/3508B·원문 보존 관측, 정량 성능 수락 아님. 실앱/환경/native 잔여. 전체 단계 표는 [TASK-019](../04_implementation/tasks/TASK-019.md)·[API§14.8](../03_design_artifacts/interface_contract.md#148-task-019-s3-원-deliverydb-완료파일-격리--2026-11-01).

## #020 — 2026-11-02 #001 R1/R2 상태 인덱스 이관

- **TASK-014/015 현재 계약·형상:** [EV-029](../05_verification/verification_evidence.md#ev-029) source101개/검사14개·Miri224·소스9fecaae commit blob일치, REV-025 Closed. EV-027/028의 계측·report 기반에 제출/대여 capacity/contract/lifecycle 범주를 추가하고 총계는 보존했다. 거절 중 capacity만 과부하로 사용하며 기존 출력 실패/자원 한계 판정은 유지한다. 부하율은 accepted+rejected_capacity **시도율**(재시도 포함·고유 입력/손실 아님)이다. acquire·계약/수명 거절·skip은 분자 제외. CounterCell128B/RT 할당·해제0, 추가 Sample 크기는 checked control 예약으로 검사한다. history_len은 Observation<usize>, 독점 reset은 이력/poison을 초기화한다.
- **리포트/호스트 경계:** 실제 Window session/revision/epoch/구간·장비/부하/target/boundary와 검증 능력의 session clock/유효성을 유지한다. 물리 bytes/slots·출력정지·filesystem/quota·순환 보존/retained span을 분리한다. host는 성공 sample과 target_totals의 유한 이력에서 실제 from/to를 대조하고 terminal checked 차분·최종 pending을 공급한다. 누락/foreign/Busy/overflow는 Unavailable이며 가까운 시점 혼합/배출 후 정확 계약이다. 물리 gauge/출력 출처·capacity evidence·자동 worker/CLI runtime backend·실앱과 실제 PC/RT 오차≤20%·계측 비용≤5%·native는 후속이다. 상세 [TASK-014](../04_implementation/tasks/TASK-014.md)·[TASK-015](../04_implementation/tasks/TASK-015.md)·[API§15](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안).

- 2026-11-01 #013: TASK-019 InfluxDB v2 어댑터 독립 코드리뷰를 수행하여 Pass 판정 및 REV-038(Open)을 등록했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_013.md).

## #021 — 2026-11-02 #001 TASK-021 수락 시 R2 이관

- **TASK-016 로컬 계약:** EV-030/source542c6e6·source102개·검사10개·Miri496·소스/commit blob102개 일치, REV-027 개발 자가 점검 Pass/Closed·위임 수락. 명시 enable/preview/validate/apply와 두 bank·posted/per-producer ack·old accepted cutoff/완료/실제 raw·Delivery guard 해제 뒤 회수, Busy/Conflict/checked revision을 사용한다. level/Normal sampling/기존 target route만 live 변경하며 Important sampling 우회·같은 raw loan·과거 수락 route를 보존한다. 전환 Sample은 RevisionTransition, 혼합 revision Window 금지. Comparison은 원래 Report/profile/revision/구간·입력/장비/boundary/측정 scope 차이와 skip/거절/출력 손실을 분리한다. 단일 revision 파일은 명시 budget/별도 writer다. 운영 backend/CLI runtime/자동 worker·실앱/실제 PC·RT 비용/오차/native는 후속. [TASK-016](../04_implementation/tasks/TASK-016.md)·[API §15.2](../03_design_artifacts/interface_contract.md#152-task-016-로컬-설정-변경전후-비교--2026-11-01).

- **2026-10-31 Human 구현 승인:** 「승인, 다음작업 진행하자」로 TASK-008 EV-012/013·TASK-010 EV-016/017·TASK-011 EV-020을 수락했다. REV-009/010/013/014/017 및 해당 RV Closed, TASK-008/010/011 Human 리뷰 완료. 기존 리뷰는 닫았으며 신규 TASK-012 리뷰는 별도다. 낮은 pin 유실/forget 강제 회수 없음·실제 reader close 뒤 반환 계약과 실측/native 이월은 유지한다.

### TASK-021 수락 시 이전 Closed 리뷰 행 이관

| [REV-029](../05_verification/reviews/REV-029_TASK-017_PLAN_START.md) | TASK-017 S1 계획/예산 개발 자가 점검 | Closed | Pass·S1 계획/예산 로컬 수락(EV-031/75c6096), S2는 REV-030 참조 |
| [REV-030](../05_verification/reviews/REV-030_TASK-017_FILE_RUN.md) | TASK-017 S2 합성 파일 실행/정리 개발 자가 점검 | Closed | Pass·신규 미해결 RV 없음, EV-032/5d3349b·S2 로컬 수락·운영/생산/실측 후속 |
| [REV-031](../05_verification/reviews/REV-031_TASK-017_CHECK_HOST.md) | TASK-017 S3 로컬 운영 host·CLI 개발 자가 점검 | Closed | RV-066~068 probe 기한/CLI exit/원 source session 조치, EV-033·로컬 연결 수락 |
| [REV-032](../05_verification/reviews/REV-032_TASK-017_ROUTED_BRIDGE.md) | S3 생산 모듈/실제 ACK·실앱 snapshot 개발 자가 점검 | Closed | 신규7개·두 Rust workspace 각364/EV-034, 미해결 지적 없음 |
| [REV-033](../05_verification/reviews/REV-033_TASK-017_ROUTED_HOST.md) | 고정 routed host·원 source/CLI 권한·시계 개발 자가 점검 | Closed | RV-069 호스트 deadline·RV-070 초기 OS 원인 보존 Red→Green, EV-035·기능 수락 |
| [REV-034](../05_verification/reviews/REV-034_TASK-017_INDEPENDENT_REVIEW.md) | TASK-017 독립 리뷰 조치 | Closed | RV-071/072 상수 공개·24MiB rustdoc·EV-036 수락 |
| [REV-035](../05_verification/reviews/REV-035_TASK-019_INFLUX_WRITE.md) | TASK-019 S1 개발 자가 점검 | Closed | RV-073 타입별 field·RV-074 HTTP body 예약 수명·EV-037 로컬 수락 |
| [REV-036](../05_verification/reviews/REV-036_TASK-019_NUMERIC_QUERY.md) | TASK-019 S2 개발 자가 점검 | Closed | RV-075 query dialect·실제 DB/typed query EV-038 수락 |
| [REV-037](../05_verification/reviews/REV-037_TASK-019_ROUTED_COMPLETION.md) | TASK-019 S3 개발 자가 점검 | Closed | RV-076 terminal 복구·RV-077 초기 cancel·RV-078 proxy 자식 합류·EV-039 수락 |
| [REV-038](../05_verification/reviews/REV-038_TASK-019_INDEPENDENT_REVIEW.md) | TASK-019 독립 리뷰 문서 조치 | Closed | RV-079/080 rustdoc·fmt 확인, 소스73edb7a |
| [REV-039](../05_verification/reviews/REV-039_TASK-020_TRACING_INPUT.md) | TASK-020 S1 일반 로그 입력 자가 점검 | Closed | RV-081 MSRV 예제 수명 수정·EV-040 수락 |
| [REV-040](../05_verification/reviews/REV-040_TASK-020_PIPELINE.md) | TASK-020 S2 통합 비교 자가 점검 | Closed | RV-082 JSON 출처 식별자 보강·EV-041 수락 |
| [REV-041](../05_verification/reviews/REV-041_TASK-020_TAIL_SCREEN.md) | TASK-020 S3 화면 구독 불연속 자가 점검 | Closed | RV-083 큰 ID/clock 표기 조치·EV-042 수락 |

## #022 — 2026-11-05 #001 최근 이력 하베스트(최대 3세션 유지)

- 2026-11-03 #002: TASK-016 운영 backend(status·apply·compare)에 대한 독립 코드리뷰([REV-045](../05_verification/reviews/REV-045_TASK-016_OPERATIONS_BACKEND_INDEPENDENT_REVIEW.md))를 수행하여 Open 상태로 등록했다. (P1/P2 0건, P3 권고 3건: rustdoc #Errors 명시, Debug derive, 스냅샷 정렬 계약)
- 2026-11-03 #001: 운영 backend(status 지표·ApplyConfig·Compare, 새 오류 3종) EV-044/REV-044 로컬 수락(소스 7f9fbd5). GAP-008 사전 존재 불안정 시험 등록.
- 2026-11-03 #001: REV-043 RV-086/087 rustdoc 보강(소스 dd7aaaf)·검증 후 Closed, 미처리 리뷰 없음 — [2026-11-03 #001](../members/lim8603/workspace/session_logs/session_2026-11-03_001.md).
- 2026-11-02 #002: TASK-021 UPS 긴급 계획·모의 시험에 대한 독립 코드리뷰([REV-043](../05_verification/reviews/REV-043_TASK-021_INDEPENDENT_REVIEW.md))를 수행하여 Open 상태로 등록했다. (P1/P2 0건, P3 문서 권고 2건)
- 2026-11-02 #001: TASK-020 S3 화면 예제·TASK-021 UPS 시뮬레이션을 EV-042/043·REV-041/042로 로컬 수락했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-02_001.md).
- 2026-11-01 #015: TASK-020 S2 실제 파일/DB/구독·명시 설정 비교 예제를 EV-041/REV-040으로 로컬 수락했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_015.md).
- 2026-11-01 #014: REV-038 문서 조치와 TASK-020 S1 일반 로그 입력을 EV-040/REV-039로 로컬 수락했다 — [로그](../members/lim8603/workspace/session_logs/session_2026-11-01_014.md).

## #023 — 2026-11-07 #001 세션 종료 R1/R2 하베스트

`project_state.md`가 34 KiB로 24 KiB 경고선을 넘어 원문을 그대로 옮겼다. 현재 제약은 Task·API 문서와 GAP-002/004에 유지된다.

### 현재 결정과 제약 — TASK-009~022 계약 요약 원문

- **TASK-011 현재 계약:** frozen Snapshot·mono ns 동기 replay·prefetch≤1·checked generation/누적 읽기, 늦은 소비 시계/End 원본 수명·실제 reader close 후 반환을 유지한다. 상세 [TASK-011](../04_implementation/tasks/TASK-011.md)·[API§14.2](../03_design_artifacts/interface_contract.md#142-task-011-로컬-재생-api--2026-10-30).

- **TASK-010 현재 계약:** 고정 session/revision·독립 read-only source·완전 chunk prefix·유한 index/page/blob/read 예산·reader-owned pin은 실제 Drop 뒤 반환한다. closed bounded scan·pin 분실/forget 강제 회수 없음. [TASK-010](../04_implementation/tasks/TASK-010.md)·[API§14.1](../03_design_artifacts/interface_contract.md#141-task-010-로컬-api--2026-10-30).

- **TASK-009 현재 계약:** 원 session/revision/등록 순서·불변 read-only 원본·독립 빈 출력, 완전 검증 prefix만 복구한다. 자동 재동기화/과거 completion 복원/실패 파일 삭제·재시도 없음. [TASK-009](../04_implementation/tasks/TASK-009.md)·[API§13.5](../03_design_artifacts/interface_contract.md#135-task-009-중단-복구-api--2026-10-30).

- **2026-10-31 Human 구현 승인:** TASK-008/010/011 EV-012/013/016/017/020 및 REV-009/010/013/014/017 수락·Closed. 낮은 pin 유실/forget 강제 회수 없음·실제 reader close 뒤 반환과 실측/native 이월은 유지한다. 상세는 각 Task/API를 따른다.

- **TASK-012 계약:** remote 기본 disabled/loopback/read-only·외부 TLS/인증·bounded 구독/256KiB·handoff ACK/Gap/Unknown·세대 gate·host inbox. EV-023 소스ca1694a는 이전 기준, server 오류 조치·구독 경계는 [EV-026](../05_verification/verification_evidence.md#ev-026)/eb39cb2·API§14.5를 따른다. 포화10ms 냉각 유지·실측은 TASK-022. native/실앱·LAN/RSS/보안 수락은 후속이며 OS keyring·CI/계정/과금 조회/차단 재시도 없음을 유지한다.

- **TASK-013:** EV-025/026 로컬 수락·최신 EV-029 회귀. local wait timeout은 같은 구독 유지, ttl은 record/idle lease 겸용·새 구독 간 Gap 연속성 미보장(S3 EV-042 화면 예제 수락). CLI runtime metrics Unavailable·config apply/compare Unsupported·실앱/native/성능 후속. [TASK-013](../04_implementation/tasks/TASK-013.md)·API§14.5.

- **TASK-014/015:** EV-029/REV-025 Closed. 제출·대여의 capacity/contract/lifecycle 분리·총계 보존, 과부하는 capacity만 사용한다. 부하율은 accepted+rejected_capacity 시도율(재시도 포함)이며 고유 입력/손실이 아니다. CounterCell128B/RT 할당·해제0, checked control 예약·Observation 이력/독점 reset 계약 유지. 원 session/revision/epoch/구간·장비/부하/target/boundary/clock을 대조하고 정확 terminal 차분·pending을 공급한다. 물리 bytes/slots·출력정지·filesystem/quota·보존을 분리하며 누락/foreign/Busy/overflow는 Unavailable이다. 실제 물리 gauge·운영 worker/CLI·실앱·PC/RT 비용≤5%/오차≤20%/native는 미검증. [TASK-014](../04_implementation/tasks/TASK-014.md)·[TASK-015](../04_implementation/tasks/TASK-015.md)·[API§15](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안).

- **TASK-016:** EV-030/031 로컬 수락·REV-027/028 Closed. startup enable·두 bank·producer posted/ack/cutoff/완료·실제 raw/Delivery guard 해제 뒤 회수, Busy/Conflict/checked revision을 유지한다. level/Normal sampling/기존 route만 live 변경하며 Important 우회·같은 loan·과거 수락 route 보존·혼합 revision Window 금지다. Comparison은 원 Report/profile/시계/입력/환경 차이와 skip/거절/출력 손실을 분리한다. revision별 파일은 별도 writer/예산이다. 운영 backend/CLI·실앱·PC/RT 비용/오차/native는 후속. [TASK-016](../04_implementation/tasks/TASK-016.md)·[API§15.2](../03_design_artifacts/interface_contract.md#152-task-016-로컬-설정-변경전후-비교--2026-11-01).

- **TASK-017 현재 계약:** EV-035·REV-034 조치 EV-036 로컬 수락. 고정 single-use File/Routed host·원 source/시계/권한·≤64KiB snapshot raw→TCP 수신 큐 ACK/원 receipt를 유지한다. 예제2초/요구24MiB·허용32MiB, capacity/freshness/LAN/RSS 미측정. 지속 앱/화면/off-on·실측/UPS/장기/native(T3) 미수락으로 Task/TC Partial 유지. [TASK-017](../04_implementation/tasks/TASK-017.md)·[API§15.7](../03_design_artifacts/interface_contract.md#157-task-017-고정-routed-runcheck--2026-11-01).

- **TASK-019:** S1~S3 EV-037~039 로컬 수락, REV-037/038 Closed. 새 독점 namespace·equal/older timestamp 거절·원 schema/receipt/real ns·actual IO/guard 수명을 유지한다. 통합/환경/정량 성능/native 잔여. [TASK-019](../04_implementation/tasks/TASK-019.md)·[API§14.8](../03_design_artifacts/interface_contract.md#148-task-019-s3-원-deliverydb-완료파일-격리--2026-11-01).

- **TASK-020:** S1~S3 EV-040~042 로컬 수락·REV-041/RV-083 Closed. TimedOut은 보류/같은 구독 확인, 종료/새 구독은 새 segment·연결 금지·누락 Unknown, 구독 내 Gap만 transfer 범위·새 span이다. 화면 포화 미ACK·종료행 보존·실제 owner 정리, 큰 ID/시각 문자열 계약 유지. 지속 앱/화면·장기 tail/복원·PC/RT/native는 후속. [전체 단계](../04_implementation/tasks/TASK-020.md#단계별-실행--2026-11-01-014).

- **TASK-021:** S1~S3 EV-043/REV-042 Closed·로컬 구현/UPS 시뮬레이션 수락. 기본 off·startup 계획·원 기한 최소값·rank/ID·동일 종료 coordinator·진행 IO 실제 수명·post-alarm snapshot0/사전 보호·partial 계약을 유지한다. 현재 PC ext4의8/64MiB×전달4조건×두 Rust Pass, 실제 sync/프로세스 중단/복구 oracle 일치. **실물 UPS는 필수 입력 아님.** native T3·장기/RT·실앱은 별도 후속. [전체 단계](../04_implementation/tasks/TASK-021.md#단계별-실행수락--2026-11-02-001).

- **TASK-022 승인 기준:** local-pc-v1의10ms 주기/기한·500µs 합성 연산·2GiB·P3 120초×3회·160GiB cap/64GiB reserve·UPS 모의3초/8·64MiB는 Human 승인 완료다. [EV-021](../05_verification/verification_evidence.md#ev-021)은 프로필 준비/승인 근거이며 실제 성능/RT는 NotRun, UPS 하위는 EV-043 시뮬레이션 수락이다.

### 열린 결정과 검증 공백 — Closed 행 원문

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| GAP-009 | Closed — crate 내부 `#[cfg(test)]` 주입으로 host·IO panic 시험(EV-048, 691f937) | 공개 hook 없음 유지 |
| GAP-010 | Closed — tracing 어댑터 lifetime 생략, 1.85/1.96 clippy exit0(EV-048) | — |
| GAP-008 | Closed — hyper stream task가 serve 반환 뒤 hub 예약을 보유하던 순서 문제. serve가 서버 측 handle 해제까지 대기(EV-045, d001931), 동시4 19/200→0/200 | 재발 시 같은 방법으로 재측정 |
| GAP-006 | EV-006 자가 리뷰 RV-001~004는 EV-007 수정·로컬 회귀 Pass로 Closed | Human 승인 완료·native GAP-004 후속, 기존 이력 보존 |
| GAP-007 | RV-005는 EV-009 로컬 수정/회귀 Pass·Human 승인으로 Closed | native는 후속, 수정 전 EV-008 이력 보존 |
| RV-015~017 | 최종 sync 취소는 EV-014, 방어적 슬라이싱/스택 포맷은 EV-015에서 반영·REV-011/012 Closed·Human 승인 | native·장비 검증 대기 |
| RV-024/025 | TASK-011 늦은 소비 중 배속 변경/오류 정리 반례를 EV-018에서 조치, REV-015 Closed·2026-10-31 Human 승인 | native·장비 검증 대기 |
| RV-026~028 | RV-026 현 불변식상 overflow 불가·방어 정리, RV-027/028 진단 API 조치·EV-019 Pass·REV-016 Closed·2026-10-31 Human 승인 | native·장비 검증 대기 |
| RV-032~035 | TASK-012 auth/세대/reply capacity/blob 요청 길이 반례 조치·EV-022 Pass·REV-018 Closed | 2026-10-31 Human 수락·REV-018 Closed, native/실앱·LAN/전체RSS 후속 |
| RV-029~031 | REV-017 반입·MSRV 유지/비율 비교/오류 진단·EV-020 Pass, Closed·Human 승인 | native·PC 실측 후속 |
| RV-021~023 | 동일 Query 정렬 재사용·Page inline/누적 읽기 예산 안내를 EV-017에서 조치, REV-014 Closed·Human 승인 | native·PC 실측 후속 |
| RV-018~020 | TASK-010 만료 복원·blob Gone·frame Limit을 EV-016에서 조치, REV-013 Closed·Human 승인 | native·PC 실측 후속 |
| RV-006~010 | EV-011 로컬 수정·Human 승인 완료, REV-008 Closed | native GAP-004·장비 성능은 잔여 |

### 최근 작업·이력 원문

- 2026-11-07 #001: TASK-028 S1 — S1a core 예약 공식을 생성자와 공유하는 순수 함수로 추출해 facade 정확 배정(RV-097 해소, F14), S1b `Preset`·`Sizing`·`.preset()`·`plan()`(F15). 소스 `c92fb98`·`d23de85`, 1.96/1.85 각 483 Pass·matrix 두 toolchain(EV-052), REV-050 Closed.
- 2026-11-07 #001: TASK-027 S3e — api-consumer에 facade 경로(일반 thread 기록·health·shutdown, remote 한 줄) 추가, `scripts/check-feature-matrix.py` 1.96/1.85 exit0(EV-051, 소스 `46e833a`). TASK-027 S1~S3 로컬 완료.
- 2026-11-07 #001: TASK-027 S3c — `LoggerBuilder::remote/remote_with`·`Logger::remote_addr`, 첫 output 기록 사본을 구독자에 전달(완료 대상 아님), GetStatus만 응답, remote 버퍼를 memory_bytes external partition으로 차감, Closed 전 listener 정지(멈춘 클라이언트는 2초 유예 후 중단, RV-110). 소스 `d595122`, 1.96/1.85 각 477 Pass(EV-050), REV-049 Closed.
- 2026-11-07 #001: TASK-027 S3b — `LoggerBuilder::tracing(topic, min, quota)`·`Logger::tracing()`→`TracingSink`(`tracing::Subscriber`+`log::Log`), 유실 계수 `Health::tracing_dropped`, formatting 중 재진입 교착 방지(RV-105)·shutdown이 global sink의 writer 회수(RV-106). 소스 `bbc0b28`, 1.96/1.85 각 473 Pass(EV-049), REV-048 Closed. 같은 날 S3d 제외 Human 승인·이월 T4 기록(`797ecc8`).
- 2026-11-07 #001: TASK-027 S3a — `Logger::health()`(출력별 sync 경과·실패 기록·IO 정지, RV-095)·`LoggerBuilder::telemetry()`·`Budget::worker_cpus`(RV-096)·worker panic 주입 시험(GAP-009)·host 사망 시 shutdown 즉시 실패(RV-100)·MSRV lint 정리(GAP-010). 소스 `691f937`, 1.96/1.85 각 470 Pass(EV-048), REV-047 Closed. slot 직접 encode는 byte ring 감김·0 채우기·unsafe 필요로 S3 제외를 권고 → Human 승인, 재설계 조건은 이월 T4로 기록.
- 2026-11-05 #001: TASK-027 S2 — capture를 공유 core/endpoint로 분해(`04a7cfa`, 공개 시그니처 불변)하고 facade·기본 worker를 구현(`1e651b4`). TC-013 F1~F8·allocator F2/F3, workspace 1.96/1.85 각 460 Pass(EV-047), REV-046 Closed.
- 2026-11-05 #001: TASK-027 S1 — core 빌림 구조 조사 후 ADR-012·API §18·NFR-008 수치를 제안했고, Human이 리뷰 포인트 5개를 모두 권고안으로 승인했다(ADR-012 Accepted, MS-008 In Progress, TC-013 F1~F8 상세화). 제품 코드 변경 없음.
- 2026-11-04 #001: 객관 평가(엔진은 가벼운 설계·통합 접점은 무거움, Envelope 저장으로 표준 MCAP 도구 판독 불가) 후 MS-008/009·TASK-027~032·NFR-008·TC-013/014 계획 등록. 제품 코드 변경 없음.
- 2026-11-03 #003: TASK-023 S1 P0 기준선 harness·판정 규칙·합성 제어 루프(EV-046, 소스 c9ea04a). `/data/build` 여유 부족은 Human 승인으로 build cache(target/debug·gate-target) 정리해 472.9GB 회복.
- 2026-11-03 #003: GAP-008 원인(hyper HTTP/2 stream task의 hub handle 지연 해제)을 확인하고 serve drain 대기로 수정(EV-045, 소스 d001931), Closed.
- 2026-11-03 #003: REV-045 RV-091~093 수용·조치(소스 ee57f44: `# Errors` rustdoc·Debug·status snapshots ID 오름차순)·회귀 Pass, Closed.


---

## #024 — 2026-11-07 #002 리뷰 세션 종료 하베스트 (Closed 리뷰 행 이관)

### 코드리뷰 현황 — Closed 행 원문 (REV-042~050)

| ID | 범위 | 상태 | 현재 처리/후속 |
|---|---|---|---|
| [REV-042](../05_verification/reviews/REV-042_TASK-021_UPS_SIMULATION.md) | UPS 긴급 계획·모의 시험 자가 점검 | Closed | RV-084 기한 경합·RV-085 ext4 프로필 조치, EV-043 수락 |
| [REV-043](../05_verification/reviews/REV-043_TASK-021_INDEPENDENT_REVIEW.md) | TASK-021 UPS 긴급 계획·모의 시험 독립 코드리뷰 | Closed | RV-086/RV-087 rustdoc 선행조건·1회 관측 계약 보강(dd7aaaf), 동작 변경 없음 |
| [REV-044](../05_verification/reviews/REV-044_TASK-016_OPERATIONS_BACKEND.md) | 운영 backend(status·apply·compare) 자가 점검 | Closed | RV-088~090 P3 문서 처리, EV-044 로컬 수락. 독립 리뷰 미실시 |
| [REV-045](../05_verification/reviews/REV-045_TASK-016_OPERATIONS_BACKEND_INDEPENDENT_REVIEW.md) | TASK-016 운영 backend 독립 코드리뷰 | Closed | RV-091 `# Errors`·RV-092 Debug·RV-093 snapshots ID 오름차순(API§15.8) 조치(ee57f44) |
| [REV-046](../05_verification/reviews/REV-046_TASK-027_S2_FACADE.md) | TASK-027 S2 facade·기본 worker 자가 점검 | Closed | RV-094→GAP-009(EV-048 Closed), RV-095/096 S3a 조치, RV-097~099 문서화. 독립 리뷰 미실시 |
| [REV-047](../05_verification/reviews/REV-047_TASK-027_S3A_HEALTH.md) | TASK-027 S3a health·계측·CPU 고정 자가 점검 | Closed | RV-100 host 사망 시 shutdown 즉시 WorkerFailed 조치, RV-101~104 문서화/미실행(Linux 외). 독립 리뷰 미실시 |
| [REV-048](../05_verification/reviews/REV-048_TASK-027_S3B_TRACING.md) | TASK-027 S3b tracing 연결 자가 점검 | Closed | RV-105 재진입 교착 방지·RV-106 shutdown writer 회수 조치, RV-107~109 문서화. 독립 리뷰 미실시 |
| [REV-049](../05_verification/reviews/REV-049_TASK-027_S3C_REMOTE.md) | TASK-027 S3c remote 연결 자가 점검 | Closed | RV-110 멈춘 클라이언트가 shutdown을 막던 문제 2초 유예 후 중단으로 조치, RV-111~114 문서화. 독립 리뷰 미실시 |
| [REV-050](../05_verification/reviews/REV-050_TASK-028_S1_PRESET.md) | TASK-028 S1 정확 배정·preset·plan 자가 점검 | Closed | RV-115 공식 공유로 이중화 방지(F14), RV-116~119 문서화/후속. 독립 리뷰 미실시 |

## #025 — 2026-11-07 #003 세션 종료 R1/R2 하베스트

`project_state.md` 24 KiB 초과(24,926 B)로 이관한 원문이다. 현재 요약은 상태 문서, 근거는 링크된 EV·API·세션 로그를 따른다.

- **2026-11-07 Human 결정 — TASK-023 S2b 발견 ⑥:** 「권고대로」. facade 보존 segment 개수 상한을 "크기·기록 수 회전이 만들 수 있는 가장 작은 segment" 기준으로 계산하고(storage 공용 공식 `smallest_rotated_segment`), 늘어난 catalog 예약은 예산·`plan()`에 반영한다. 시간 회전(60초)·종료로 닫히는 더 작은 segment는 이 계산 밖이다.

- **2026-11-07 Human 결정 — TASK-023 S2b 측정 발견 조치(3건 모두 권고안):** ③ IO batch 실제 크기 = min(사용자 값, target별 기록 예산/2), start/plan에서 결정. ④ 주기 sync 발급 조건을 "target 큐가 빔"에서 "cutoff 이하로 이 target에 라우팅된 기록이 모두 기록됨"으로 core 수정(routing/control, TDD·Miri). ⑤ 측정 도구 P0에 파일당 최대 기록 수 회전을 추가해 facade(segment당 16,384건)와 같은 회전 조건으로 비교, 작은 기록의 파일 수 증가는 TASK-028/029 제품 사이징 과제로 기록. 발견 ①(부분 라우팅 sync coverage)은 `b2584c6` 수정, ②(`Delivery` drop 순서 → 필수 target 폐기)는 결함 수정으로 진행.

- **2026-11-07 Human 결정 — TASK-023 S2a 채널별 출력 선택:** 「출력 순서 지정」. `.channel_with(topic, ChannelOptions::new().outputs(&[..]))`, `file_output` 선언 순서 0부터, 기본 전체 출력, 잘못된 선택은 `StartError::InvalidOutputs`. 16파일 측정을 P0와 같은 workload로 만들기 위함. [API §18.12](../03_design_artifacts/interface_contract.md#1812-채널별-출력-선택--2026-11-07-task-023-s2a).

- **2026-11-05 Human 결정 — TASK-027 S1 facade 계약 승인:** 리뷰 포인트 5개를 하나씩 검토해 모두 권고안을 선택했다. ① [ADR-012](../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md) Accepted: 공유 `CaptureCore` + 소유 `Writer` + 빌림 `Producer<'_>` 뷰, Writer Drop은 반납함 이동만 하고 host/reaper가 최종 소유(ADR-008 borrowed core 유지). ② 성능 값(IO 1·CPU 고정 없음)은 facade 기본값, 손실 값(sync 주기·종료/Drop 기한·메모리·보존)은 `Budget`/preset 필수. ③ `Logger::drop`은 `drop_deadline` 기한 있는 종료. ④ NFR-008: L1 ≤20줄·≤6개념, L2 ≤40·≤10, L3 +6, facade 공개 ≤60. ⑤ `writer.try_log(channel, &msg)`. 계약은 [API §18](../03_design_artifacts/interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05), 시험은 TC-013 F1~F8.

- **2026-11-04 Human 결정 — 소비처 접점 경량화·시각화 범위 추가:** 사용성 평가(공개 항목 534개·기본 worker 없음·예산 명시 부담·Envelope 저장으로 표준 도구 판독 불가)에 동의하고 「권고대로」·「TASK-032 포함」·「MS 분리 가능」. [MS-008](../04_implementation/milestones/MS-008.md)(TASK-027 facade·기본 worker, TASK-028 preset·auto preset, TASK-029 레시피)와 [MS-009](../04_implementation/milestones/MS-009.md)(TASK-030 MCAP 표준 호환, TASK-031 viewer·Foxglove 계열 bridge, TASK-032 운영 대시보드) 등록. NFR-008 통합 용이성 추가, FR-006 viewer 확장, FR-015 장비 측정 auto preset만 TASK-028 S2로 부분 채택(질문 프리셋은 TASK-018 제안 유지). **순서:** TASK-027 → TASK-028 S1 → TASK-023 S2/S3(facade 경로)·RT 요청 → TASK-028 S2 → TASK-029 → MS-009 → probe 인계 → TASK-026. 11-03 결정의 "단독 작업 먼저, probe 연동 나중" 원칙 안에서 단독 작업 범위를 넓혔다. ADR 후보: facade 소유 모델(TASK-027 S1)·MCAP 표준 저장 모드(TASK-030 S2 전)·시각화 방식(TASK-031 S1).

- 2026-11-07 #003(이어서): TASK-023 S2a 채널별 출력 선택(Human 결정)·S2b facade 경로 측정 도구. 측정 중 제품 결함 ①~⑤를 TDD로 조치(부분 라우팅 sync coverage·Delivery drop 순서 폐기·batch 예산 붕괴·연속 부하 주기 sync 미발급·기록 수 회전 대응), ⑥ 보존 segment 개수(Human 「권고대로」)·⑦ Busy 보고·⑧ `.active` 잔존까지 조치 — [EV-054](../05_verification/verification_evidence.md#ev-054).

- 2026-11-07 #003: REV-051 RV-120~127을 TDD로 조치(Writer Drop reaper 경쟁·유휴 spin·시작 실패 정리·오버플로·facade 메모리 예산·IO 실패 분류·공개 표면·perf CLI). 1.96/1.85 각 493 Pass — [EV-053](../05_verification/verification_evidence.md#ev-053), [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-07_003.md).

- 2026-11-05 #001: TASK-027 S1 ADR-012·API §18 승인과 S2 facade·기본 worker 구현(EV-047) — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-05_001.md).

1. **단독 작업(2026-11-03 결정, 2026-11-04 MS-008/009로 확장):** ~~운영 backend~~ EV-044·~~REV-045~~·~~GAP-008~~ EV-045·TASK-023 S1 EV-046 완료 → ~~TASK-027 S1~~ 승인 → ~~S2~~ EV-047(`04a7cfa`·`1e651b4`) → ~~S3a~~ EV-048(`691f937`: `health()`·계측·CPU 고정·GAP-009/010) → ~~S3b~~ EV-049(`bbc0b28`: tracing/log 연결 +2줄) → ~~S3c~~ EV-050(`d595122`: remote 관측 사본·상태 조회) → ~~S3e~~ EV-051(`46e833a`: api-consumer·feature matrix) → ~~TASK-028 S1~~ EV-052(`c92fb98` 정확 배정·`d23de85` preset/plan) → ~~[REV-051](../05_verification/reviews/REV-051_FACADE_PRESET_PERF_INDEPENDENT_REVIEW.md) 조치~~ EV-053(`559f2b9`, Closed) → **다음 TASK-023 S2**(S3d slot 직접 encode는 2026-11-07 Human 승인으로 S3 제외, 재설계 조건은 이월 T4; facade 경로 P1 256B·P2 8MiB·P3 혼합, 32생산자/16파일/128채널, P3 혼합은 `StorageWorkload` 확장 필요)/S3(120초×3·P5·계측 비용, P4 배정 확인) → RT 요청/승인 후 RT 행 → TASK-028 S2 auto preset → TASK-029 → MS-009(TASK-030~032) → probe 연동 인계 패키지.

## #026 — 2026-11-11 #002 세션 종료 R1/R2 하베스트

`project_state.md` 24 KiB 초과(31,247 B)로 이관한 원문이다. 현재 요약은 상태 문서, 근거는 링크된 EV·API·Task·세션 로그를 따른다.

### 완료된 Human 결정

- **2026-11-11 Human 결정 — 「올려올려」:** rustix 고정을 1.1.4 → 1.1.6(RUSTSEC-2026-0486, 같은 계열 수정판). [EV-069](../05_verification/verification_evidence.md#ev-069).
- **2026-11-11 Human 결정 — 「권고대로」:** ② 중복 CRC는 mcap 0.26.0을 `vendor/`에 두고 `[patch.crates-io]`로 무압축 chunk CRC를 1회 계산(ADR-010 개정 1, 파일 바이트 동일, +4~6%, `6187768`). `cargo audit`은 path 패키지를 검사하지 않으므로 mcap 공지는 수동 확인. ③ Envelope 복사 제거는 착수하지 않고 이월 T6. [EV-068](../05_verification/verification_evidence.md#ev-068).
- **2026-11-11 Human 결정 — T5 후속(「1. 진행 2. 아직 배포된게 아니니 제거 3. 효과 측정」):** ① ⑫ 조치 → `236ef59` Pass ② 중복 CRC 제거 — 복구가 chunk CRC를 필수로 쓰고 mcap 0.26 옵션으로는 1회화가 안 되어 mcap 포크가 필요(파일 바이트 동일, +4~9%), 진행 방식 확인 대기 ③ Envelope 복사 — 측정 완료(1회 −21~24%, 제거 상한 추정 +37~45%). [EV-067](../05_verification/verification_evidence.md#ev-067).
- **2026-11-10 Human 결정 — TASK-023 S3 마감(「a」):** 추가 쓰기 없이 EV-064·065로 판정(P3 효율 합산 89% Pass·장치 표류 주석, raw 획득 거절은 harness 사이징 → 스크립트 preset 크기 수정). T5는 Human 「그래도 한번 조사는 하고 넘어가자」로 조사 결정(다음 세션, tmpfs·SSD 쓰기 0, 시작 시 시간 상한 합의). 10GbE 1줄 ≈ 1.25 GB/s < 상한 2.6 GB/s, 원격 전송(P4)은 미측정. [EV-065](../05_verification/verification_evidence.md#ev-065).
- **2026-11-10 Human 결정 — 발견 ⑪ 비차단(「1」):** host 분배 상한(tmpfs logger P3 약 2.6 GB/s, 원인 미규명)은 「알려진 상한·비차단」. 선조치 없이 축소 S3 실행, 재개 조건은 T5. [EV-063](../05_verification/verification_evidence.md#ev-063).
- **2026-11-07 Human 결정 — TASK-023 S2(4건, 모두 권고안):** S2a 채널별 출력 선택(`ChannelOptions::outputs`, 선언 순서), S2b ③ batch = min(값, 기록 예산/2) ④ 주기 sync 발급 = cutoff까지 이 target 기록이 Written ⑤ P0를 facade 16,384건 회전에 맞춤 ⑥ 보존 segment 개수를 가장 작은 회전 segment 기준으로 계산(시간 회전·종료 segment는 밖). [API §18.12·§19.5](../03_design_artifacts/interface_contract.md#195-task-023-s2b-조치-반영--2026-11-07), [EV-054](../05_verification/verification_evidence.md#ev-054)·[EV-055](../05_verification/verification_evidence.md#ev-055).
- **2026-11-07 Human 결정 — TASK-028 S1 preset/사이징:** preset 3종(`control`·`sensor`·`black_box`) + `Sizing`, 흡수 1초 × slot 1.25·메모리 +10%(`black_box` 흡수 2초), start 시 계산 + `plan()`. preset은 빈 손실 값만 채우고 사용자 값을 덮어쓰지 않는다. [API §19](../03_design_artifacts/interface_contract.md#19-예산-preset사이징-계약--2026-11-07).
- **2026-11-07 Human 결정 — TASK-027 S3c remote 연결:** ① 구독 기록은 관측용 사본(완료 대상 아님, 포화는 그 구독의 Gap/Disconnect) ② 원격 관리는 상태 조회만(나머지 Unsupported, 이력 조회·재생은 MS-009 TASK-031) ③ 한도는 writer quota 기반 기본값 + `remote_with` 명시. 상세 [API §18.11](../03_design_artifacts/interface_contract.md#1811-s3c-remote-선택-연결--2026-11-07).
- **2026-11-07 Human 결정 — TASK-027 S3d 제외:** 「승인」으로 slot 직접 encode를 S3에서 빼고 scratch encode + byte ring 복사 경로를 유지한다(byte ring 감김·rtrb 0 채우기·unsafe/형식 변경 필요). TASK-023 S2 P1(256B)에서 encode·복사 비용이 의미 있게 나오면 별도 설계한다 — Human 요청으로 이월 백로그 T4에 기록. API §18.3 문구 정정.
- **2026-11-11 Human 결정 — TASK-028 S2 착수(질문 3건 모두 권고안):** 짧은 측정 + 보수 추정(지속 = 순간 × 0.5 Estimated, 수락 추정 ≤ 1.2 × S3 S), 프로필 JSON(feature `calibrate`), 반영 범위 = 흡수 시간·메모리·부하 경고(사용자 값 불변, CPU·RT는 기록만). [API §19.8](../03_design_artifacts/interface_contract.md#198-task-028-s2-auto-preset-계약--2026-11-11), [EV-071](../05_verification/verification_evidence.md#ev-071).

### 이전 다음 시작점 1

1. **단독 작업(2026-11-03 결정, 2026-11-04 MS-008/009로 확장):** 완료 사슬(운영 backend~TASK-028 S1~REV-051, EV-044~053)은 아카이브 #025. **TASK-023 S3 완료(2026-11-10 Human 「a」). T5 조사 완료(2026-11-11, [EV-066](../05_verification/verification_evidence.md#ev-066)): ⑪ = IO 경로 바이트당 비용(encode 복사·CRC 2회) + 공유 자원 경합. ⑫(종료 drain 장부 훑기) 조치 `236ef59`([EV-067](../05_verification/verification_evidence.md#ev-067), 8 스트림 0.92 → 3.00 GB/s). 무압축 chunk CRC 1회 `6187768`([EV-068](../05_verification/verification_evidence.md#ev-068), vendor mcap·ADR-010 개정 1), Envelope 복사는 T6, rustix 1.1.6([EV-069](../05_verification/verification_evidence.md#ev-069)). **RT 행 완료(2026-11-11, [EV-070](../05_verification/verification_evidence.md#ev-070)): generic·PREEMPT_RT FIFO 모두 control·safety miss 0, RT는 부하 중 wake 꼬리 감소, 유휴 wake 약 1 ms는 C-state 추정. 21:39 generic 복귀 완료.** **TASK-028 S2 Human 승인([EV-071](../05_verification/verification_evidence.md#ev-071)) → 다음 TASK-029.** ⑪은 P2 격차 없음(EV-060), P3는 생산자 바쁜 대기(도구 `RETRY_BACKOFF` 조치)와 단일 host 스레드 분배 상한 약 17.5만 기록/s(P3 혼합 약 1.75 GB/s, 빠른 장치 P0의 55~60%, 요구 P3는 충족) — EV-061. host 장부 훑기 제거·게이트 정정(EV-062·063) 뒤 tmpfs logger P3 약 2.6 GB/s — 장치 S(약 1.67 GB/s)·요구보다 높아 다음은 축소 S3 정식 실행. **S3 축소(2026-11-09 Human 「1」·「구간 60초」):** cgroup M 8 GiB로 RAM 2배 대체·무거운 행 1회·효율 ABBA·구간 60초, 약 0.7 TB(T500)·약 40분, 실행 전 쓰기량 고지. 요구 §12.4 장시간 기준 개정(2026-11-08 Human) → S3(120초×3·P5·계측 비용, P4 배정 확인, T4 판정; 공통 `Gate` 시작·`LoadWindow` 중첩 판정 사용) → RT 요청/승인 후 RT 행 → TASK-028 S2 → TASK-029 → MS-009(TASK-030~032) → probe 연동 인계 패키지. REV-052 Closed.

### 이전 최근 작업·이력

- 2026-11-11 #002(계속): TASK-028 S2 — 결정 3건 권고안, calibrate·프로필·`Sizing::profile`·plan 경고·CLI(TDD, 신규 시험 13), 1.96.0/1.85.0 각 559 Pass, T500 정확도 3회 Pass·SSD 12.89 GB — [EV-071](../05_verification/verification_evidence.md#ev-071).
- 2026-11-11 #002: RT 행 — RT 부팅 확인(첫 RT 세션은 Human 재시작으로 무측정, 재예약), `run-perf-rt.sh` RT 실행 miss 0, generic/RT 대조 [EV-070](../05_verification/verification_evidence.md#ev-070), SSD 약 70.5 GB — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-11_002.md).
- 2026-11-11 #001: T5 해소 — ⑪ 원인 규명([EV-066](../05_verification/verification_evidence.md#ev-066)), ⑫ 종료 drain 조치 `236ef59`([EV-067](../05_verification/verification_evidence.md#ev-067)), vendor mcap 무압축 chunk CRC 1회 `6187768`·ADR-010 개정 1·T6([EV-068](../05_verification/verification_evidence.md#ev-068)), rustix 1.1.6([EV-069](../05_verification/verification_evidence.md#ev-069)). RT 요청 승인·도구 FIFO 모드·부팅 스크립트(`a6dcc72`·`9764df3`), 1단계 generic FIFO 행 miss 0(SSD 70.4 GB) — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-11_001.md).
- 2026-11-10: ⑪ 비차단·T5 등록(Human 「1」), 축소 S3 1회차 실행(584 GB, 40분) — P3 효율 미달로 T5 도래 [EV-064](../05_verification/verification_evidence.md#ev-064). 후속 진단 149 GB: P3 미달 재현 안 됨·raw 거절은 풀 사이징·계측 비용 2.5% [EV-065](../05_verification/verification_evidence.md#ev-065).
- 2026-11-09 #001(계속): ⑪ 4차 — reconcile 게이트 정정(실제 abandon만 세대 증가, EV-062 정정), tmpfs 측정 +4%·거부 절반, 대기 설정 영향 ≤3%, 남은 logger 상한 tmpfs P3 2.6 GB/s(장치 S·요구보다 높음) — [EV-063](../05_verification/verification_evidence.md#ev-063).
- 2026-11-09 #001(계속): ⑪ 3차 — host 장부 훑기(reconcile·sync 적용) 제거 TDD, host R 99% → 52~66%, 처리량 개선은 잡음 안, 남은 한계는 파이프라인 대기(IO 배치 10 ms·유휴 1 ms) — [EV-062](../05_verification/verification_evidence.md#ev-062). 11-09 쓰기 누적 약 585 GB.
- 2026-11-09 #001(계속): ⑪ 2차 — P3 격차 원인(생산자 `yield_now` 바쁜 대기 → host 스레드 CPU 박탈, 도구 backoff 조치)·단일 host 분배 상한 Open — [EV-061](../05_verification/verification_evidence.md#ev-061). 요구 §12.4 축소 기준 개정·`run-perf-s3.sh` 반영(dry-run 통과). 11-09 쓰기 누적 약 495 GB.
- 2026-11-09 #001: TASK-023 S3 발견 ⑪ 1차 분석 — 회전 sync 가설 기각, 소진 후 MemoryMax 4G A/B logger ≈ P0, 장치 상태 표류 2배 관측, 쓰기 약 370 GB(Human 승인) — [EV-060](../05_verification/verification_evidence.md#ev-060). `run-perf-s3.sh` 장치 기록을 측정 경로의 디스크로 정정.
- 2026-11-08 #001(계속): TASK-023 S3 — 범위(P4·P5 이월, P1 3조합)·장시간 기준(S·0.6×S·효율 0.8·sync 판정) Human 결정, 발견 ⑩(출력 예산 CAS `Busy`) 조치 `5eacfa3`, 발견 ⑪ Open — [EV-059](../05_verification/verification_evidence.md#ev-059).
- 2026-11-08 #001(계속): TASK-023 S2d 발견 ⑨ — facade가 turn마다 완료 장부 전체(window × writers)를 sweep → writer 1개 window로(`5511eb3`), P3 20초 smoke 성공 — [EV-058](../05_verification/verification_evidence.md#ev-058).
- 2026-11-08 #001(계속): TASK-023 S2c — `Channel::metadata`(Human 「메타데이터 헬퍼」)·혼합 P2/P3 도구, RV-128 정정(`11f44d9`, 부분 라우팅 종료 sync 거절 회귀), smoke에서 발견 ⑨ — [EV-057](../05_verification/verification_evidence.md#ev-057).
- 2026-11-08 #001: REV-052 RV-128(target별 파일 장부로 다른 파일의 미동기화 기록 증명 요구)·RV-129(연속 구간만 watermark 전진, Enqueued 기준 target 분리)·RV-130(공통 `Gate` 시작·`LoadWindow` 중첩 판정) TDD 조치, 1.96/1.85 각 522 Pass — [EV-056](../05_verification/verification_evidence.md#ev-056), [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-08_001.md).
- 2026-11-07 #004: REV-051 조치·TASK-023 S2a/S2b 누적 변경을 독립 리뷰하여 [REV-052](../05_verification/reviews/REV-052_SYNC_BOUNDARY_PERF_REVIEW.md) Open(Conditional: P1 2·P2 1), 제품 무수정·공개 API 재현 2건 Red·1.96/1.85 각509 Pass — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-07_004.md).
- 2026-11-07 #003: REV-051 RV-120~127 조치·Closed(EV-053), TASK-023 S2a 채널별 출력 선택·S2b facade 측정 도구와 측정 중 제품 결함 ①~⑧ 조치(EV-054·055) — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-07_003.md).
- 2026-11-07 #002: 직전 독립 리뷰(REV-045) 이후 누적 변경분(`b330818..8eecc90`, 32파일) 전수 독립 코드리뷰 수행, [REV-051](../05_verification/reviews/REV-051_FACADE_PRESET_PERF_INDEPENDENT_REVIEW.md) Open 등록(Conditional Pass: P1 1·P2 5·P3 2, 제품 코드 무수정) — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-07_002.md).

## #027 — 2026-11-12 #001 세션 종료 최근 이력 하베스트(최대 3세션 유지)

`project_state.md` 최근 작업·이력을 3세션으로 줄이며 옮긴 원문이다.

- 2026-11-12: TASK-029 S4 [EV-075](../05_verification/verification_evidence.md#ev-075) `975858a` — `dot_probe_facade`(facade만, support 0, 68줄 vs pipeline 37 + support 889, 범위 차이 기록). S3 Human 승인.
- 2026-11-12: TASK-029 S3 [EV-074](../05_verification/verification_evidence.md#ev-074) `a2f0dce` — README 250→136줄(quickstart·레시피 중심), 계약 상세 11개 모듈 rustdoc 이동, crate quickstart doctest·README 동기화 가드. S1·S2 Human 승인.
- 2026-11-12: TASK-029 S1·S2 [EV-073](../05_verification/verification_evidence.md#ev-073) `681acd7` — 표면 가드(이름 있는 항목 30/60·snapshot), 레시피 L1 20줄·6개념/L2 40·7/L3 +3·+5, `Shutdown::is_success()`.
- 2026-11-12: REV-053 RV-131~133 조치 [EV-072](../05_verification/verification_evidence.md#ev-072) `71e5a72` — reconcile 진입 claim(Red `CompletionCapacity` → Green), 비 Linux calibrate 타입 오류(nightly build-std 교차 검사 E0308 8 → 0), core-only rustdoc CI 단계.
- 2026-11-11 #001: T5 해소(EV-066~068)·rustix 1.1.6(EV-069)·RT 요청 1단계 — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-11_001.md).
- 2026-11-10: 축소 S3 1회차·후속 진단([EV-064](../05_verification/verification_evidence.md#ev-064)·[EV-065](../05_verification/verification_evidence.md#ev-065)).


## #028 — 2026-11-16 #001 세션 종료 R1/R2 상태 인덱스 이관

공유 상태24KiB 경고선 초과(28,486B)를 정리했다. 현재 제약·미완료와 최신 REV-055/056은 공유 상태에 유지하며 아래 완료 상세와 과거 개인 상태는 원문으로 보존한다. 당일 개발 상세는 기존 세션 로그·EV-082/083에 있어 재복제하지 않는다.

### 완료 리뷰 행 원문

| REV-001~017 | 코어·파일·조회·재생 | Closed | 로컬 조치·Human 수락 완료, native/PC 실측 이월은 유지 |
| REV-018~022 | 원격·client·CLI | Closed | EV-026까지 수락. RV-049→TASK-022, RV-053→TASK-020 |
| REV-023~028 | 계측·리포트·설정변경 자가/독립 리뷰 | Closed | EV-027~031 로컬 수락 마감, 비용·오차·실앱/native 후속 |
| REV-029~041 | 간편 점검·DB·tracing/화면 예제 | Closed | EV-031~042 로컬 수락, 실앱/환경 후속은 각 Task 참조 |
| REV-042~050 | UPS·운영 backend·facade S2~S3·preset 자가/독립 리뷰 | Closed | EV-043~052 로컬 조치·마감(독립 REV-043·045 포함). 원문 [상태 아카이브](state_archive.md) #024 |
| [REV-051](../05_verification/reviews/REV-051_FACADE_PRESET_PERF_INDEPENDENT_REVIEW.md) | GAP-008·TASK-023 S1·TASK-027 S2~S3e·TASK-028 S1 누적 독립 코드리뷰 | Closed | Conditional Pass 8건(P1 1·P2 5·P3 2) 전부 수용·조치 [EV-053](../05_verification/verification_evidence.md#ev-053) `559f2b9`, 2026-11-07 Human 「승인」 |
| [REV-052](../05_verification/reviews/REV-052_SYNC_BOUNDARY_PERF_REVIEW.md) | REV-051 조치·TASK-023 S2a/S2b 누적 독립 코드리뷰(`8eecc90..f807b58`) | Closed | Conditional, P1 2·P2 1(RV-128~130) 전부 수용·조치 [EV-056](../05_verification/verification_evidence.md#ev-056) `a4b9bcb`·`2af2dce`, 정정 `11f44d9`(EV-057). 2026-11-08 Human 「승인」 |
| [REV-053](../05_verification/reviews/REV-053_COMPLETION_CALIBRATION_REVIEW.md) | REV-052 조치·TASK-023 S2/S3·T5·RT harness·TASK-028 S2 누적 리뷰(`f807b58..da908a0`) | Closed | Conditional, P1 1·P2 1·P3 1(RV-131~133) 전부 수용·조치 [EV-072](../05_verification/verification_evidence.md#ev-072) `71e5a72` — 훑기 단일 소유 claim, 비 Linux 대체 경로·시험 분리, core-only rustdoc CI. 2026-11-12 Human 「승인」 |
| [REV-054](../05_verification/reviews/REV-054_INTEGRATION_GUARD_REVIEW.md) | REV-053 조치·TASK-029 S1~S4 누적 리뷰(`da908a0..00ecb78`) | Closed | Conditional, P2 2건(RV-134~135) 수용·조치 [EV-076](../05_verification/verification_evidence.md#ev-076) `a536486` — snapshot 계약 조건(trait 제약·연관 항목 타입·impl 대상/제약) 직렬화, 개념 계수 import 해석·glob/미해석 실패. 2026-11-12 Human 「승인」 |

### 완료 결정 상세 원문

- **2026-11-12 Human 결정 — TASK-030 착수 4건 + 구현 중 2건(권고안):** facade `Schema::descriptor`(선택 descriptor), 타입 이름은 채널 metadata `payload_schema_name`. MCAP `sequence` = Envelope 하위 32비트, `log_time`·`publish_time` = real, mono·u64 sequence·producer·level·importance·revision·source는 보조 메타 채널 1개(protobuf `RecordMeta`), ROS 2 encoding T1 유지. [API §21.1](../03_design_artifacts/interface_contract.md#211-human-결정--2026-11-12).

- **2026-11-11 Human 결정 — TASK-028 S2(착수 3건 권고안·「승인」):** 짧은 측정 + 보수 추정(지속 = 순간 × 0.5, 수락 ≤ 1.2 × S3 S), JSON 프로필(feature `calibrate`), 흡수·메모리·부하 경고. 측정 도중 취소 시험 미실시. [API §19.8·19.9](../03_design_artifacts/interface_contract.md#198-task-028-s2-auto-preset-계약--2026-11-11), [EV-071](../05_verification/verification_evidence.md#ev-071).

### 이전 개인 상태 원문

원래 위치: `.cowork/members/lim8603/workspace/my_state.md`(아래 상대 링크도 원문 그대로 보존).

```markdown
# 작업자 상태 — lim8603

| 항목 | 내용 |
|---|---|
| 역할/실행 모드 | 프로젝트 오너/Master, solo |
| 활성 Intent/Milestone | INT-001 Approved / MS-009 착수 대기(MS-008 TASK-029 로컬 완료) |
| 현재 담당 | TASK-030 착수 전 — 직전 TASK-029 S1~S4 Human 승인([EV-073](../../../05_verification/verification_evidence.md#ev-073)~[EV-075](../../../05_verification/verification_evidence.md#ev-075)) |
| 리뷰 | 미처리 없음 — [REV-053](../../../05_verification/reviews/REV-053_COMPLETION_CALIBRATION_REVIEW.md) Closed(2026-11-12 Human 「승인」) |
| 갱신/세션 | 2026-11-12 / #001 종료 — Human 「세션 마무리하고 다음 세션에서 시작하자」 |
| 로그 | [2026-11-12 #001](session_logs/session_2026-11-12_001.md) |

다음 개발 시작점: TASK-030(MCAP 표준 호환) 착수 — Task 문서의 Human 확인 4건(MCAP sequence u32↔Envelope u64 매핑, `log_time` mono/real, 메시지별 level·importance·producer 보존 방식, ROS 2 encoding T1 유지)부터. S1 export는 ADR 없이, S2 native 표준 모드는 ADR 채번·승인 후. 확인 필요(11-11부터 이월): TASK-027 「Human 리뷰 대기」 표기가 REV-051(11-07 승인) 뒤 남은 것인지. 이후 순서·승인·검증 공백은 [공유 상태](../../../06_evolution/project_state.md). 개인 이월 없음.
```

### 이전 작업 순서 결정 상세 원문

- **2026-11-04 Human 결정 — MS-008/009:** 소비처 접점 경량화(TASK-027~029)와 MCAP 표준·viewer·대시보드(TASK-030~032) 등록, NFR-008 추가, FR-015는 auto preset만 TASK-028 S2로 부분 채택. **순서:** TASK-027 → TASK-028 S1 → TASK-023 S2/S3·RT 요청 → TASK-028 S2 → TASK-029 → MS-009 → probe 인계 → TASK-026. [MS-008](../04_implementation/milestones/MS-008.md)·[MS-009](../04_implementation/milestones/MS-009.md).

- **2026-11-03 Human 결정 — probe 연동 전 dot-logger 단독 작업 선행:** 「이 프로젝트에서 할 수 있는걸 다 마치고 probe쪽에서 logger를 연동하는 세션을 진행」. ① 원격 ApplyConfig/Compare 운영 backend·CLI runtime metrics 구현(기존 API§15 범위), ② TASK-023 합성 행(P0 기준선·10ms 제어 off/on·P1~P3·P5·계측 비용)을 TASK-026 전에 generic 커널에서 실행 — 실앱 행은 TASK-026 뒤 유지, ③ 합성 측정 뒤 RT 준비/재시작 요청서를 제출하고 승인 후 RT 행 실행, ④ probe 연동 인계 패키지 작성 후 dot-probe 저장소 세션에서 TASK-026 진행. 24h soak(TASK-024)는 연동 뒤. dot-probe 저장소는 이 세션에서 수정하지 않는다.

### 완료 결정·이월 판정 상세 원문

- **완료된 Human 결정(11-07~11-11: TASK-023 S2·S3 마감·⑪·T5·CRC vendor·rustix·TASK-027 S3c/S3d·TASK-028 S1):** 원문은 [상태 아카이브](state_archive.md) #026, 계약은 API §18~19·ADR-010 개정 1·EV-054~069. T4 이월은 아래 백로그, T6는 native 전환으로 해소(EV-083).

 **T5 → 해소(2026-11-11, EV-066~068): 원인 규명, ⑫ 조치 `236ef59`, 무압축 chunk CRC 1회 `6187768`, 잔여 개선은 T6.** **T6 → 해소(2026-11-16, EV-083): native 사용자 payload 직접 쓰기로 Envelope scratch 복사를 제거. 정량 효과는 재측정하지 않음.** 트리거 ①은 EV-065에서 재현되지 않음. T1~T4는 미도래(T4: 복사 p50 21 ns = 시계 오버헤드). T4 입력: S2b smoke(측정 아님)에서 256B `try_log` p50 143 ns 중 복사 p50 21 ns(시계 오버헤드와 같은 수준) — 정식 판정은 TASK-023 S3. 공개 후 native 실행 시점 지정은 공개 전환 자체 요청이 아니므로 T2를 지금 발동하지 않는다. OQ를 T/B에 중복 등록하지 않는다.


## #029 — 2026-11-17 #001 세션 종료 R1/R2 하베스트

공유 상태의 24KiB 경고선 초과(27,249B)와 완료 서사를 정리했다. 아래는 B/C 마감 당시 원문이며 현재 상태/제약은 project_state·API·Task·EV를 따른다. 최근 이력은 최대3세션의 한 문장으로 축약했고 상세는 EV-084·085와 기존 세션 로그에 보존한다.

- **2026-11-17 TASK-035 S2 C 로컬 단위 완료:** [API §23.3](../03_design_artifacts/interface_contract.md#233-손실-누적-checkpoint-c-구현--2026-11-17)·[EV-085](../05_verification/verification_evidence.md#ev-085)·[REV-058](../05_verification/reviews/REV-058_LOSS_CHECKPOINT_C.md) Closed(RV-146~148). 제품 `5be17b5`. telemetry 독립 counter·native loss checkpoint·bounded loss-only API/CLI·원 report 복구·target identity를 구현했다. retry 시도/skip/assignment/Unknown·미확인 꼬리·포화/누락 part를 분리한다. 다음 D TASK-034 WIP36c1724 통합, 그 뒤 E GUI/호출 비용·전체 통합/커버리지 판정. TASK-030·035 전체 In Progress를 유지하며 이전 A/B 절의 unavailable은 당시 구현 기록이다. GitHub PRIVATE로 T2/T3 미도래.

- **2026-11-17 TASK-030 S2 B 완료:** [API §23.2](../03_design_artifacts/interface_contract.md#232-revision-회전source별-조회복구-b-구현--2026-11-17)·[EV-084](../05_verification/verification_evidence.md#ev-084)·[REV-057](../05_verification/reviews/REV-057_REVISION_STORAGE_B.md) Closed. r1→r2→r1→r2 single writer 회전·source별 revision/sequence query/recovery·manifest v2, event/pin·발행 prefix·sync 범위 검증. loss는 unavailable, runtime WIP는 미통합이다. GitHub PRIVATE로 T2/T3 미도래. 이번 로컬 검증의 실제 커널은 `7.0.0-43-realtime`; 재부팅·커널 변경·정량 성능 측정은 수행하지 않았다.


### 완료된 facade 가드·preset 결정 원문

- **TASK-028 auto preset:** Done. JSON profile·측정/추정 구분과 사용자 값 우선 계약 유지. 측정 도중 취소는 미실행([API §19.8·19.9](../03_design_artifacts/interface_contract.md#198-task-028-s2-auto-preset-계약--2026-11-11), [EV-071](../05_verification/verification_evidence.md#ev-071)).

- **2026-11-12 Human 결정 — TASK-029 착수·L1:** NFR-008 「facade 공개 항목」은 이름 있는 항목(타입·trait·자유 함수, variant·메서드 제외)으로 세고, 변경 감지는 rustdoc JSON(nightly-2026-10-26) + 스크립트·snapshot. L1 개념 초과는 `Shutdown::is_success()` 추가로 해소. [API §20](../03_design_artifacts/interface_contract.md#20-task-029-레시피접점-회귀-가드--2026-11-12).

## #030 — 2026-11-18 #002 세션 종료 최근 이력 이관

- 2026-11-17 #003: REV-059 RV-149~152 조치·자가검토·위임 마감(제품 `ca89b69`, [EV-086](../05_verification/verification_evidence.md#ev-086), [로그](../members/lim8603/workspace/session_logs/session_2026-11-17_003.md)).
- 2026-11-17 #002: `fa72a1a..5be17b5` 독립 리뷰 REV-059 Open 생성 후 #003에서 Closed([로그](../members/lim8603/workspace/session_logs/session_2026-11-17_002.md)).

## #031 — 2026-11-18 #005 세션 종료 최근 이력 이관

- 2026-11-18 #001~002: ADR-013 E와 TASK-033 S4 수락 및 .logger 예약 토픽·standard/2 적용([EV-088](../05_verification/verification_evidence.md#ev-088)~[EV-090](../05_verification/verification_evidence.md#ev-090), [REV-061](../05_verification/reviews/REV-061_ADR_013_E.md)·[REV-062](../05_verification/reviews/REV-062_DOT_LOGGER_NAMESPACE.md), [로그 #001](../members/lim8603/workspace/session_logs/session_2026-11-18_001.md)·[#002](../members/lim8603/workspace/session_logs/session_2026-11-18_002.md)).

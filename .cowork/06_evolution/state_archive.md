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
- 근거가 필요할 때만 조사: [초기 후보](imported_context/research_2026-10-25_logging_telemetry.md), [공개 성능 비교](imported_context/research_2026-10-25_public_benchmarks.md)·[수치 JSON](imported_context/research_2026-10-25_benchmark_data.json), [실사용/TCP](imported_context/research_2026-10-25_end_to_end_tcp.md), [버퍼](imported_context/research_2026-10-25_buffer_budget.md), [리포트](imported_context/research_2026-10-25_capacity_reporting.md).
- 센서/연동 근거가 필요할 때만 [과거 센서 용량](imported_context/research_2026-10-25_sensor_volume.md)·[계산 JSON](imported_context/research_2026-10-25_sensor_volume_calculations.json), [시각화 연동](imported_context/research_2026-10-25_observability_integrations.md), [ROS 2 검토](imported_context/research_2026-10-25_rosbag2_interop.md). 과거 제품 가정/당시 미승인을 현재 기준으로 오해하지 않는다.

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
| project_state / my_state / session_2026-10-25_003 | [세션 #003](../members/lim8603/workspace/session_logs/session_2026-10-25_003.md) 종료: 6대 설계 반영·Tier 1/2 및 코어 격리 확정·C++ 쌍둥이 프로젝트(`~/src/cpp-logger`) 생성·Git 첫 커밋(`916d550`) 완료 및 다음 시작점(실행 구조 결정) 동기화 | INT-001 |
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
- 2026-10-29 #003: RV-006~010 수정·EV-011 로컬 회귀 Pass·소스 71b3e2c 게시·세션 종료, 최신 구현 Human 리뷰/native·장비 이월 — [수정 로그](../members/lim8603/workspace/session_logs/session_2026-10-29_003.md).

## 2026-10-30 #003 세션 마감 — 완료 서사·상세 재개 메모 이관

공유 상태 R1/R2에 따라 원문을 보존한다. 현재 승인·검증 공백·다음 행동은 project_state.md에 유지한다.

- **2026-10-30 TASK-009 로컬 결과:** 불변 원본의 완전 검증 chunk prefix만 별도 파일로 복구·index 재생성/file sync·선택 namespace finalize/directory sync를 구현했다. [EV-014](../05_verification/verification_evidence.md#ev-014) source51개/검사10개 exit0, 복구19개·두 Rust 각16 features/consumer/예제/lint/doc/fmt·allocator9·Miri117×8=936 Pass다. [REV-011](../05_verification/reviews/REV-011_TASK-009.md) RV-015 최종 sync 중 취소는 Red→Green·Resolved다. 신규 구현은 로컬 리뷰 대상이며 Human 최종 리뷰/native·실물은 잔여다. 자동 startup 발견/기존 namespace 재개·reader·worker/StorageDegraded를 완료한 것으로 보지 않는다.

- **2026-10-30 게시·후속 착수:** Human 「커밋 푸쉬 후, 다음 테스크 진행」으로 EV-013 소스 `14fd1a4`/문서의 분리 커밋·게시와 TASK-009 착수를 승인했다. source49개 commit blob/EV-013 hash가 일치한다. TASK-008 REV-009/010 Resolved·최종 구현 리뷰 및 native GAP-004는 유지하며 후속 개발을 진행한다. CI 차단 재시도·계정 조회는 하지 않는다.

- **2026-10-30 개발 세션 #003:** Human의 브리핑·리뷰 지적사항 처리·TASK-008 요약 요청으로 RV-012~014를 조치했다. RV-013 중복 probe 제거·RV-014 symlink 의도 문서화, RV-012는 승인된 만료/Drop 후 실제 release 전 삭제 금지 계약을 유지하며 `must_use`/문서/회귀와 TASK-010 리더 수명 관리에 연결했다. 토큰 유실 위험은 잔여이며 기한만의 강제 회수를 도입하지 않는다. 최종 검사14개 exit0·보존 Miri128회 Pass이며 [EV-013](../05_verification/verification_evidence.md#ev-013)에 고정했다. 초기 브리핑/처리 요청 자체는 최종 구현 승인·commit/push·TASK-009 착수 지시가 아니었으며, 게시·후속 착수는 뒤이은 Human 요청으로 별도 승인됐다.

- **2026-10-30 리뷰 세션 #002:** Human 「리뷰세션 시작하자」 및 「그동안 리뷰 안건의 처리가 올바르게 되었는지도 리뷰해볼까?」에 따라 리뷰 전용 세션을 수행했다. 제품 코드 수정 없이 [REV-010](../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md)(Open, RV-012~014)을 생성하고, 과거 전체 리뷰 안건(RV-001~011)에 대한 전수 감사를 실시하여 단 하나의 회귀 없이 100% 완전 반영되어 있음을 교차 검증했다.

- **2026-10-30 승인·구현:** Human 「승인, TASK-008 진행하자.」로 TASK-007 최신 구현을 승인하고 REV-008을 Closed로 마감했다. TASK-008은 [EV-012](../05_verification/verification_evidence.md#ev-012) 로컬 Pass·REV-009 Resolved이며 Human 구현 리뷰/native GAP-004·장비는 잔여다.

- **2026-10-29 리뷰 절차 공식화:** Human 「프로젝트 리뷰처리 절차를 공식화 하려고해... 승인」에 따라 `05_verification/reviews/` 디렉터리를 신설하고 과거 리뷰 내역(REV-001~003)을 복원했다. 리뷰 세션의 코드 수정 금지(Read-Only), REV 문서 자산화, 리뷰 상태 추적(`Open` → `In Progress` → `Resolved` → `Closed`), 브리핑 시 미처리 리뷰 상시 보고 규칙을 프로토콜 및 진입점(AGENTS/GEMINI)에 공식 반영했다.

- **2026-10-29 게시·마감:** Human 「커밋 푸쉬하고 세션 마감해줘」에 따라 소스 `71b3e2c`를 origin/main에 게시했다. EV-011 source46개와 해당 commit blob의 SHA-256이 전부 일치한다. 소스/문서는 분리 커밋·[skip ci]로 관리하며 마감 문서를 별도 게시한다. 이 게시 요청 자체는 구현 승인으로 확대하지 않았으며, 최종 구현 승인은 2026-10-30 별도로 받았다.

- **2026-10-29 리뷰 수정:** Human 「코드리뷰 받은 내역을 확인하고 수정하자는 말이였어」로 RV-006~010 수정 범위·방향을 승인했다. 배치 Written·진행 prefix, pre-IO SyncWork 반환·공통 Busy 매핑, 예산 내 쓰기 버퍼와 report-only 재시도를 구현했다. [EV-011](../05_verification/verification_evidence.md#ev-011) 로컬 Pass이며 최종 구현 Human 승인은 2026-10-30 완료됐다. native·장비는 잔여다.

- **2026-10-29 승인·착수:** Human 「승인」으로 TASK-006 RV-001~005 최신 수정분을 승인하고 native 이월 상태의 TASK-007 진행을 승인했다. TASK-006/EV-009 로컬 Pass·native GAP-004는 유지하며 MCAP 봉투/writer/인덱스를 구현하고 EV-010 로컬 검증을 마쳤다. 당시 TASK-007 Human 리뷰/native는 대기였으며 착수 승인은 commit/push 지시가 아니었다. 최종 구현 Human 승인은 2026-10-30 완료됐다.

- **2026-10-29 후속 수정:** [EV-008](../05_verification/verification_evidence.md#ev-008)의 RV-005 P1을 Human 수정 지시 후 [EV-009](../05_verification/verification_evidence.md#ev-009)에서 해결했다. 일반 sync 진입/게시와 shutdown을 단발 원자적 표시·RAII·SeqCst로 순서화했다. 정식 반례2개 Red 후 수정·private3개, 두 Rust 각16 features/consumer/예제/clippy/rustdoc/fmt·Miri81×8·RT0/0 Pass, source38개 고정이다. GAP-007 로컬 Closed, 최신 수정분 Human 승인은 완료됐고 native는 잔여다. TASK-006 보고에 RV-001~005 수정 반영사항을 함께 정리한다. commit/push·CI/과금 조회는 하지 않았다.

- **이전 수정/게시 이력:** RV-001~004의 EV-007 로컬 회귀와 코드 d57c2e3 게시 이력은 [TASK-006](../04_implementation/tasks/TASK-006.md)에 보존했다. GAP-006/007은 로컬 수정 범위에서 Closed, TASK-006 최신 수정분 Human 승인은 완료됐고 native GAP-004는 잔여다. 현재 게시 소스 `71b3e2c`는 EV-011을 기준으로 검토한다.

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

- #001: TASK-007 승인·REV-008 Closed, TASK-008 회전/보존/이벤트 구현·EV-012 Pass·REV-009 Resolved. 소스 7388145 로컬 커밋.
- #002: TASK-008 심층 코드리뷰([REV-010](../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md) Open, RV-012~014 지적) 및 과거 전체 리뷰 안건(RV-001~011) 전수 무회귀 검증 완료. 제품 코드 수정 없이 리뷰 세션 정상 마감.

- #003: REV-010 지적 처리·EV-013 로컬 Pass·REV-010 Resolved. 보존17개·두 Rust core/all-features/consumer/예제/allocator8/lint/doc/fmt·보존 Miri128회 Pass. pin 유실 위험은 TASK-010에 연결.

- #003 후속: Human 「커밋 푸쉬 후, 다음 테스크 진행」으로 14fd1a4/1f2f44d 게시 후 TASK-009 구현·EV-014 Pass·REV-011 Resolved. 복구19개·두 Rust16조합/consumer/예제/lint/doc/fmt·allocator9·Miri936 Pass. 신규 TASK-009는 로컬 리뷰 대상.

## 2026-10-30 #007 마감 — 이전 세션 요약 이관

- 2026-10-30 #004: REV-012 조치·TASK-009 승인/게시·REV-011/012 Closed 후 TASK-010 EV-016 Pass·REV-013 Resolved, 후속 소스8bbec13 로컬 커밋·미게시/Human 리뷰 대기로 종료 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_004.md).
- 2026-10-30 #003: TASK-008 조치 게시·TASK-009 복구 구현/EV-014 Pass·REV-011 Resolved 및 소스e5f3147 게시 — [작업 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_003.md).
- 2026-10-30 #002: TASK-008 심층 리뷰·REV-010 Open 등록·과거 리뷰 안건 감사 — [리뷰 로그](../members/lim8603/workspace/session_logs/session_2026-10-30_002.md).

## 2026-10-31 #006 상태 인덱스 이관

- 2026-10-31 #003: REV-017 조치/EV-020 Pass 후 TASK-008/010/011 승인·리뷰5건 Closed, TASK-022 10ms 수치 승인·EV-021 준비 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_003.md).
- 2026-10-31 #002: TASK-011 최종 안전성 코드리뷰·REV-017 Open 등록(제품 미수정) — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_002.md).

## 2026-10-31 #007 상태 인덱스 이관

- 2026-10-31 #004: TASK-012 구현·EV-022 Pass·5f0ad83 소스/문서 분리 게시 후 종료·REV-018 Human 리뷰 대기 — [로그](../members/lim8603/workspace/session_logs/session_2026-10-31_004.md).

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
| [REV-005](../05_verification/reviews/REV-005_TASK-006_RV-001-004.md) | TASK-006 | 2026-10-28 | 설계 대조 자가리뷰 / 보류 (P1 3, P2 1) | Closed | RV-001~004 EV-007 수정·로컬 Pass, commit `d57c2e3`, Human 승인 완료 |
| [REV-006](../05_verification/reviews/REV-006_TASK-006_DEEP_REVIEW.md) | TASK-006 | 2026-10-28 | 제미나이 심층 코드리뷰 / Note/Suggestion | Closed | False Sharing·O(N) 선형탐색 등 5건 분석, rustdoc 보강/장비 과제 연결 |
| [REV-007](../05_verification/reviews/REV-007_TASK-006_RV-005.md) | TASK-006 | 2026-10-29 | 종료 경합 자가리뷰 / Conditional (P1 1) | Closed | RV-005 EV-009 수정·로컬 Pass, Human 승인 완료 |
| [REV-008](../05_verification/reviews/REV-008_TASK-007_RV-006-010.md) | TASK-007 | 2026-10-29 | MCAP 저장 구현 리뷰 / Conditional (P2 3, P3 2) | Closed | RV-006~010 EV-011 로컬 수정 Pass, commit `71b3e2c`, 2026-10-30 Human 승인 완료 |
| [REV-009](../05_verification/reviews/REV-009_TASK-008.md) | TASK-008 | 2026-10-30 | 회전·보존 자가 리뷰 / Conditional (P2 1, 로컬 해결) | Closed | RV-011 EV-012 수정/회귀 Pass, 2026-10-31 Human 승인·실측/native 후속 |
| [REV-010](../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md) | TASK-008 | 2026-10-30 | 회전·보존 심층 코드리뷰 / Conditional (P2 1, P3 2) | Closed | EV-013 로컬 Pass, RV-012 위험/후속 TASK-010 유지·2026-10-31 Human 승인 |
| [REV-011](../05_verification/reviews/REV-011_TASK-009.md) | TASK-009 | 2026-10-30 | 복구·IO 장애 자가 리뷰 / Conditional (P2 1, 로컬 해결) | Closed | RV-015 EV-014 수정·회귀 Pass, 2026-10-30 Human 승인·native 후속 |
| [REV-012](../05_verification/reviews/REV-012_TASK-009_DEEP_REVIEW.md) | TASK-009 | 2026-10-30 | 심층 코드리뷰 / Pass (P3 2) | Closed | RV-016/017 EV-015 로컬 Pass·소스fe03468·2026-10-30 Human 승인 |
| [REV-013](../05_verification/reviews/REV-013_TASK-010.md) | TASK-010 | 2026-10-30 | 조회/읽기 수명 자가 리뷰 / Conditional (P2 3) | Closed | RV-018~020 조치·EV-016 로컬 Pass, 2026-10-31 Human 승인·native/실측 후속 |
| [REV-014](../05_verification/reviews/REV-014_TASK-010_CODE_REVIEW.md) | TASK-010 | 2026-10-30 | 심층 코드리뷰 / Pass (P2 1, P3 2) | Closed | RV-021~023 정렬 캐시·rustdoc 조치·EV-017 로컬 Pass, 2026-10-31 Human 승인·native/실측 후속 |
| [REV-015](../05_verification/reviews/REV-015_TASK-011.md) | TASK-011 | 2026-10-30 | 재생/시간·자원 수명 자가 리뷰 / Conditional (P2 2, 로컬 해결) | Closed | RV-024/025 조치·EV-018/019 로컬 Pass·2026-10-31 Human 승인, native·장비 잔여 |
| [REV-016](../05_verification/reviews/REV-016_TASK-011_CODE_REVIEW.md) | TASK-011 | 2026-10-30 | 로컬 재생·시간 제어 심층 코드리뷰 / Pass (P2 1, P3 2) | Closed | 2026-10-31 RV-026 현 불변식상 overflow 불가·방어 정리, RV-027/028 진단 API 조치·EV-019 Pass·2026-10-31 Human 승인·native/장비 잔여 |
| [REV-017](../05_verification/reviews/REV-017_TASK-011_FINAL_REVIEW.md) | TASK-011 | 2026-10-31 | 로컬 재생 구현 조치·안전성 최종 리뷰 / Pass (P2 1, P3 2) | Closed | RV-029 국소 경고 억제/RV-030 비율 비교/RV-031 오류 진단 조치·EV-020 Pass·2026-10-31 Human 추가 조치 승인 |
| [REV-018](../05_verification/reviews/REV-018_TASK-012.md) | TASK-012 | 2026-10-31 | 원격 adapter 자가 리뷰 / Conditional (P2 4건 로컬 조치) | Closed | RV-032~035 조치·EV-022 Pass·2026-10-31 Human 로컬 구현 수락, native/실측 후속 |
| [REV-019](../05_verification/reviews/REV-019_TASK-012_PROJECT_CODE_REVIEW.md) | TASK-012 | 2026-10-31 | 원격 adapter·프로젝트 종합 심층 코드리뷰 / Pass (P2 2, P3 1) | Closed | RV-036~038 조치·EV-023 검사9개/Miri25 Pass·Human 위임에 따른 AI 마감 |
| [REV-020](../05_verification/reviews/REV-020_TASK-013_RECEIVER_START.md) | TASK-013 첫 단위 | 2026-10-31 | 연결 수명 API 개발 자가 점검 / Pass | Closed | EV-024 통합3개·검사12개·Miri24 Pass·위임된 첫 단위 수락, transport/CLI 후속 |
| [REV-021](../05_verification/reviews/REV-021_TASK-013_CLIENT_CLI.md) | TASK-013 로컬 구현 | 2026-10-31 | client/CLI 개발 자가 점검·조치 / Pass | Closed | RV-039~045 조치·EV-025 검사15개/Miri32·위임된 AI 로컬 수락, native/실앱 후속 |
| [REV-022](../05_verification/reviews/REV-022_TASK-013_INDEPENDENT_REVIEW.md) | TASK-013 `29e55da`·TASK-012 `4f79119` | 2026-10-31 | 독립 Read-Only 리뷰·서버 경계 확장 / Pass (P2 2, P3 8) | Closed | EV-026 검사14개/Miri32·위임 수락. RV-049 TASK-022 실측·RV-053 TASK-020 장기 tail/화면 후속, wire 확장 없음 |
| [REV-023](../05_verification/reviews/REV-023_TASK-014_TELEMETRY.md) | TASK-014 d99ba3b | 2026-10-31 | 개발 자가 점검 / 로컬 Pass | Closed | EV-027 검사14개/Miri56·위임 수락; 비용/RT/PC/native/실앱 후속 |
| [REV-024](../05_verification/reviews/REV-024_TASK-015_REPORT.md) | TASK-015 a247d01 | 2026-10-31 | 개발 자가 점검 / 로컬 Pass (P2 3·P3 1 조치) | Closed | RV-056~059 Red→Green·EV-028 source100개/검사13개·Miri168·소스a247d01 위임 수락; PC/RT/native/실앱 후속 |
| [REV-025](../05_verification/reviews/REV-025_TASK-014-015_INDEPENDENT_REVIEW.md) | TASK-014 d99ba3b·TASK-015 a247d01 | 2026-10-31 | 독립 Read-Only 리뷰 / Conditional (P2 1, P3 2) | Open | RV-060 거절 범주 혼합→허위 Overloaded·부하율 과대(API§15.1 문구 변경 동반, 권한 수준 확인), RV-061 대상별 OutputDelta 정렬 수단 부재, RV-062 이력 poison 미복구. TASK-016 착수 전 처리 권고 |



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

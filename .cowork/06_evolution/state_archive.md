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

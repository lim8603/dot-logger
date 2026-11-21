# INT-001 — 재사용 가능한 실시간 로보틱스 로깅·텔레메트리 제공

## 상태

`Approved` — 프로젝트 목표·핵심 가치와 2026-10-25 Human 「확정」에 따른 설계용 초기 성능/Story AC·문서 계획·미결 이월 승인. **2026-11-21 Gate 4 Pass(Human 수락)로 현재 초점은 Verify다.** MS-001~006·008·009 Done, MS-007 In Progress. 아래 본문의 "미정" 표현은 2026-10-25 당시 기록이며, 이후 정해진 내용은 맨 아래 [현재 상태](#현재-상태--2026-11-21)를 따른다.

## 배경

Linux·macOS·Windows의 로보틱스 프로젝트에서 재사용할 Rust 로깅·텔레메트리 기반이 필요하다. Linux + PREEMPT_RT의 실시간 최적화·검증 목표도 유지한다. 첫 적용 대상은 `dot-probe`다.

## 핵심 가치

높은 기록 성능과 함께, **사용하면서 실제 부하와 처리 한계를 확인하고 개발자가 로그의 양·주기·대상을 조절한 효과를 다시 확인할 수 있게 한다.** Human이 지정한 dot-logger의 핵심 강점이다. 운영 계측·리포트·간편 점검·개발자 조절 지원을 연결하며 관측/검증/추정과 의도적 생략/예상치 못한 손실을 구분한다.

## 목표

- Linux·macOS·Windows에서 다른 프로젝트가 사용할 Rust 로깅 서버 라이브러리와 클라이언트.
- 사용 중 부하·병목·검증된 처리 범위를 제공하고 개발자가 기록량·주기·대상을 조절한 전후 효과를 확인할 수 있는 운영 계측과 점검 지원.
- 첫 적용에서 상태·오류 로그, 위치·속도·배터리 수치, 이미지·점군 센서 원본을 기록. 구체 부하와 저장 매핑은 후속 확인.
- 대표 설계 부하로 UHD급 2D LiDAR 센서 1개·카메라 센서 4개·모터 10개 이상·기타 센서와 3D solid-state 센서 가능성을 고려하고 UHD급 이상 처리 규모를 지향한다. 정확한 장비·포맷·빈도·실제 장비별 수락 조건은 설계에서 구체화한다.
- 높은 순간 입력·처리 성능과 24시간 동작 신뢰성. 모든 원본을 24시간 전량 보존하는 요구와 구분한다.
- RAM 수락·호출 비용뿐 아니라 OS I/O와 선택한 저장·전송·조회·재생 경로에서 데이터가 활용 가능해지기까지의 실사용 성능을 목표로 한다. 기존 조사에 근거한 정량 목표는 후속 구체화한다.
- 여러 스레드의 안전한 동시 로깅과 목적별 여러 파일 기록. 스레드와 파일을 일대일로 강제하지 않으며, 구체 설정·순서·자원 상한은 후속 설계.
- 로깅 파일 관리와 시계열 DB 연동.
- 미니 UPS 등의 전원 이상 시 중요 파일 우선 기록 옵션(FR-018, Human 요청·기본 비활성 확정). 활성화 시 신호 발생부터 실제 전원 차단까지 전체 약 3초 조건을 적용하며 신호 전달·저장/동기화·종료 예산과 보호량은 장비별로 설계/검증한다.
- 기록 데이터 조회에 응답하고 시간 기반으로 재생하는 기능. 재생 출력과 제어 범위는 미정.
- 모니터링 클라이언트. 라이브러리 또는 서비스 형태는 미정.

## 진행 방식

최신 대표 프로젝트·개발 방향을 공식 자료로 조사하고 `.cowork/`에 별도 보관한다. 조사에 근거하여 Human에게 필요한 사항을 **한 번에 하나씩** 질문한다.

## 성공 기준과 제약

- Rust와 Linux·macOS·Windows 지원은 Human 지정 조건이다(ADR-006). Linux + PREEMPT_RT의 실시간 최적화·검증 목표를 유지하고 각 OS의 실제 기능/성능 지원 범위를 검증한다.
- 입력·처리 성능과 하드웨어별 저장 성능·보존량을 분리한다. SSD를 기본으로 가정하고 연속·순환·이벤트 전후 구간 기록을 옵션으로 제공한다(ADR-004). 모드·용량·기간·SSD 상세와 정량 기준은 미정이다.
- 대기·보존·중요도 분류를 설정 옵션으로 제공한다. 기본값은 무대기·중요 기록 우선·로그 레벨과 별도의 보존 중요도 지정이다(ADR-003). dot-probe도 이 기본안을 사용한다.
- RT용 호출은 버퍼 포화·저장 장애에서도 공간 확보·저장 완료를 기다리지 않고 기록 수락 실패를 감지할 수 있어야 한다. 제한 시간 대기는 일반 호출용 옵션이다.
- 중요도 단계·기본 중요도·레벨 매핑 병행 규칙·구체 손실·내구성 정책은 미정이다.
- lock-free 등 실시간 성능 특성을 지향하되, 큐·알고리즘·프로세스 구성은 아직 선택하지 않았다.
- 지연·처리량·손실·장애 복구·장시간 검증의 수치와 수락 기준은 미정이다.
- 클라이언트 범위, 파일 형식, DB, 조회 프로토콜, 비목표와 공식 산출물 범위도 후속 확인한다.
- 기능 구현보다 조사와 순차 요구 확인을 먼저 수행한다.

## 관련 문서

- [크로스 플랫폼 ADR-006](../../03_design_artifacts/adrs/ADR-006_cross_platform_linux_macos_windows.md): 라이브러리·클라이언트 공통 지원, OS별 검증과 R1 구분.

- [초기 통합 사양안 제안0.1](../requirement_spec.md#12-초기-통합-사양안--제안-01): 수치·검증 프로필은 설계 기준선으로 승인. 기술 후보/미정 세부는 별도 검토하고 실측 달성과 구분한다.

- [요구사항](../requirement_spec.md)
- [정책 옵션과 기본값 ADR-003](../../03_design_artifacts/adrs/ADR-003_configurable_policies_and_defaults.md)
- [처리·저장 예산 분리 ADR-004](../../03_design_artifacts/adrs/ADR-004_processing_and_storage_budgets.md)
- 초기 조사(비공개 원시 근거: `imported_context/research_2026-10-25_logging_telemetry.md`)
- [공유 상태](../../06_evolution/project_state.md)

## 추가 검토

사용 중 한계 확인·간편 점검·개발자 조절 지원은 핵심 방향으로 지정됐다(FR-014·016·017). 질문 프리셋·PC별 기본 설정 추천은 관련 상세화 제안이다(FR-015). [요구사항 §14](../requirement_spec.md#14-질문-프리셋설정-추천간편-점검-검토안)에 시작 설정·일상 관측·내장 점검·해석/추천의 흐름을 작성했다. 구체 설정·명령·점검 예산과 하위 기능 우선순위는 검토 대상이다.

Human이 장비 사양·실제 로그 양/발생 간격에 따른 사용률·추가 처리 여유·잔여 시간 리포트를 제안했다(FR-014). [요구사항 §13](../requirement_spec.md#13-장비별-사용률처리-여유잔여-시간-리포트-검토안)에 가능성·관측/실측 기준·조건부 예측을 정리했다. 핵심 방향은 위에 반영했고 상세 계약·오차·실행 비용은 검토 대상이다. 개발자 조절과 비교는 [§13.5](../requirement_spec.md#135-핵심-강점과-개발자-조절-지원)를 따른다.

Q5에서 첫 적용은 자체 기록·조회·재생 기능으로 정했다. ROS 2 파일 호환·토픽 연동은 dot-probe 첫 적용 완료 후 검토한다(project_state.md T1). 파일 형식과 구체 재생 API는 미정이다. 조사 자료(비공개 원시 근거: `imported_context/research_2026-10-25_rosbag2_interop.md`).

## 2026-10-26 상세설계 실행 구조

Human 선택으로 첫 클라이언트는 Rust API + CLI와 dot-probe 기존 화면 연동, 실행 구조는 내장형 Ports and Adapters + 제한된 파이프라인(ADR-007)이다. 이 범위에서 위 초기 미정 표현을 해소한다. 실제 장비는 미확정이며 정량 수치 자체는 ADR-005 설계 기준선이다.

목표를 추가 Intent로 쪼개지 않고 INT-001 아래 [MS-001~007](../../04_implementation/milestone_registry.md)와 [실행 Task](../../04_implementation/task_registry.md)로 분해한다. [요구 추적표](../../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트)에서 MS별 구현/검증을 관리한다. 현재 상세 계약/기술/계획은 2026-10-26 Human 승인됐고 Gate 3 Pass다. MS-001/TASK-001 완료, 제품 구현/시험은 미완료다.

## 현재 상태 — 2026-11-21

위 본문의 목표 문장은 승인 당시 그대로 둔다. 그 뒤 정해진 사항과 완료 기준 변경은 다음과 같다.

### 성격과 완료 기준

- **범용 라이브러리:** dot-logger는 범용 로보틱스/RTOS Rust 로그 라이브러리이고 dot-probe는 첫 사용처일 뿐이다. 설계·구현을 특정 사용처에 맞추지 않는다(2026-11-13 Human 결정).
- **일정 판단:** 시험 커버리지와 로깅 사용 사례 대응으로 한다. 실사용처 적용(TASK-026), native·다른 OS·24시간·전원 차단(TASK-024)은 마지막 단계로 진행하되 일정 판단에 넣지 않는다(2026-11-13·15). 일정 판정은 [EV-089](../../05_verification/verification_evidence.md#ev-089)에서 Pass했다.
- **실제 장치 시험:** 현재 PC 실측으로 대체하고, UPS는 시뮬레이션으로 판정하며, native는 저장소 공개 뒤 진행한다([ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)).

### 당시 "미정"에서 정해진 것

| 당시 미정 항목 | 결정 | 근거 |
|---|---|---|
| 프로세스 구성·큐 | 내장형 Ports and Adapters, 제한된 생산자/소비자 파이프라인(별도 데몬 없음) | [ADR-007](../../03_design_artifacts/adrs/ADR-007_embedded_ports_adapters_pipeline.md), [ADR-008](../../03_design_artifacts/adrs/ADR-008_bounded_ownership_completion.md) |
| 파일 형식 | native 표준 MCAP(`dot-logger/standard/2`), 손실 기록 내장, 설정 revision별 파일 회전 | [ADR-010](../../03_design_artifacts/adrs/ADR-010_mcap_security_update.md), [ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) |
| 시계열 DB | InfluxDB v2(선택 feature, 유한 쓰기·수치 조회) | [ADR-009](../../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md), TASK-019 |
| 조회·재생 | 시간·채널 조회, mono 시간 재생(배속·이동), gRPC 원격 조회·재생 | TASK-010·011·012, [ADR-014](../../03_design_artifacts/adrs/ADR-014_viewer_live_websocket.md) |
| 모니터링 클라이언트 형태 | Rust 클라이언트 라이브러리 + `dot-logger` CLI + `dot-logger-viewer`(Foxglove WebSocket bridge, 읽기 전용 운영 대시보드) | TASK-013, [ADR-014](../../03_design_artifacts/adrs/ADR-014_viewer_live_websocket.md), [ADR-016](../../03_design_artifacts/adrs/ADR-016_viewer_embedded_web_dashboard.md) |
| 통합 방식 | 기본 경로 facade(기본 worker, 소유 writer, preset) + 고급 경로 모듈 | [ADR-012](../../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md), MS-008 |
| 수치 기준 | 설계 기준선은 ADR-005, 이 PC 수락 프로필은 local-pc-v1(TASK-022), 일정 판정 목표치는 커버리지 하한 라인·함수 92%, 리전 89% | [ADR-005](../../03_design_artifacts/adrs/ADR-005_design_baseline_and_deferred_decisions.md), [quality_gate](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정) |
| 전원 이상 기록 | 고급 경로 옵션, 기본 꺼짐. 신호→차단 3초·여유 200 ms 프로필을 시뮬레이션으로 수락 | TASK-021, [EV-043](../../05_verification/verification_evidence.md#ev-043) |
| ROS 2 호환 | 첫 사용처 적용 뒤 검토(T1, 미도래) | project_state 이월 T1 |

### 남은 것

- 3개 OS native 검증·24시간·전원 차단(TASK-024, 공개 뒤 T3), dot-probe 첫 적용(TASK-026), 질문 프리셋(TASK-018, 선택·범위 미승인).
- Gate 4는 Pass(이 PC 로컬, 2026-11-21 Human 수락, Phase Verify), Gate 5는 Not Ready([판정](../../05_verification/quality_gate.md#dot-logger-gate-45-판정--2026-11-21)).

# [REV-033] TASK-017 고정 routed RunCheck 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #008 / AI 개발 자가 점검. 독립 리뷰가 아니다.
- 대상: core 고정 host File/Routed dispatch·source 검증, client SnapshotCheck/공유 bridge·실행 예제, 기존/신규 CLI 및 통합 시험.
- 기준: [code-review 스킬](../../../.agents/skills/code-review/SKILL.md)·API §15.7·TC-010·coding convention.
- 상태/판정: Closed / Pass(로컬 기능 범위). Open → In Progress → Resolved → Closed를 이번 개발 및 기존 위임 수락으로 처리했다. 전체 제품/실앱/성능 수락을 뜻하지 않는다.
- 근거: [EV-035](../verification_evidence.md#ev-035)·형상/명령/원문(비공개 원시 근거: `imported_context/task_017_host_2026-11-01/results.json`). 초기 검사와 RV-070 이후 최종 검사를 분리 기록한다.

## 요약표

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리·검증 | 상태 |
|---|---|---|---|---|---|
| RV-069 | P2 | SnapshotCheck/host monotonic deadline | host 시계가2초 넘게 진행했는데 bridge 자체 Instant만 검사하여 수집1건 수락 | 실제 host 시계로 capture/handoff 직전 admission 재검사, 초과 무수집 Red→Green | Closed |
| RV-070 | P3 | 최초 clock 오류/evidence | clock의 원 OS 원인을 Start(InvalidWorkload)로 소실 | 원 io::Error·Observation/Fail·None 구간/수집0 보존, errno13 재현 Red→Green | Closed |

## [리뷰어] 검토

| 영역 | 판단·근거 |
|---|---|
| RT/unsafe | RT/원자 연산/unsafe 변경 없음. backend 등록·일반 worker 실행·정리는 비RT 경로. 새 Miri 시험은 없음 |
| 고정 권한/예산 | 동일 슬롯/checked ID·단일 사용, target/요구 메모리·network/boundary/worker 제한과 source 재구성 검증. Manage/Read는 기존 remote gate를 소비, reply headroom 부족은 capability를 소모하기 전 거절 |
| 실제 ACK/원본 수명 | 원 raw capture receipt·실제 Router Delivery 사용. 수신 큐 try_send 뒤 서버 ACK/원 receipt 완료. 기존 거절/단절/수락 뒤 ACK 불명 시험 유지. raw회수1/external0, 완전 조립만으로 ACK하지 않음 |
| 실패/종료 | late waiter close는 신규 capture를 막으며 native evidence를 남김. 실제 hub/server 종료/permit 수명 유지. operation 오류 후 이미 admitted receipt/카운터를 지우지 않음. 실패/미관측 구간·원 OS 오류를 유지 |
| 시계/출처 | source Report/profile 불변. 전용 check session/revision과 host mono ns로 capture. host와 wall admission 모두 검사, settlement 경과 시간을 부하 초과 원인으로 승격하지 않음. source freshness/boot ID 추정 없음 |
| API/의존 | native handle_command 반환은 HostCheckEvidence::{File,Routed}; 기존 file fixture 소비자를 함께 변경. trait은 trusted host 확장점, wire/runtime 타입을 core에 강제 노출하지 않음. 기존 고정 serde_json을 remote 선택 의존으로 사용, core graph 분리 |
| 실제 앱/범위 | dot-probe269d348의 exporter1627B를 새 고정 host 예제로 전달, 앱 코드 변경 없음. 단일 snapshot과 지속 앱/화면/off-on·최대 capacity/LAN/RSS/RT/UPS/장기/native를 구분 |

## [처리담당] 판정과 조치

- RV-069 수용: `bridge::Identity`의 원 session/revision/clock/admit 경로로 capture 전과 수신자 handoff 전을 검사한다. `budget_expired`는 실제 신규 작업이 막혔을 때만 기록한다. 호스트 시계가2초씩 전진하는 반례에서 accepted1→0, Inconclusive/TimeBudget를 확인했다.
- RV-070 수용: `SnapshotCheck::run`에서 최초 시계 오류를 실행 evidence로 반환한다. 원 errno13·Fail·Observation·from/to None·accepted0를 확인했다. 후속 clock failure도 마지막 유효 구간/이미 수락한 receipt와 분리하며 원 io::Error를 가능한 그대로 유지한다.
- 최종 client 신규7개·CLI 신규3개, 기존 bridge7개·file/CLI 회귀와 전체 workspace/MSRV 검사를 EV-035로 기록했다. 신규 미해결 P1/P2/P3 없음.
- 위임 수락은 로컬 기능 구현 단위다. TASK-017 In Progress·TC-010 Partial·MS-005/전체 Gate4/5 미완료 유지. 통합/환경 대기는 Task의 담당/조건 표를 따른다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | core diagnostics/routed.rs | trusted backend trait·typed routed/native evidence·bounded summary |
| 수정 | core diagnostics/{host,mod,run}.rs | 고정 File/Routed 등록/dispatch·공통 source 검증·Transport stage |
| 생성/이동 | client src/diagnostics/{mod,bridge}.rs | SnapshotCheck·기존 bridge를 제품 API와 fault test가 공유 |
| 생성/수정 | client examples/dot_probe_check.rs·dot_probe_bridge.rs·support/dot_probe.rs | 고정 host 무부하 plan/명시 run·기존 예제 소비자/오류 보존 |
| 생성/수정 | client tests/routed_host.rs·CLI tests/{routed_check_host,check_host}.rs | provenance/예산/시계/수락·권한/timeout·기존 File variant 소비 |
| 수정 | client Cargo.toml·README.md·lib.rs | remote 선택 parser·API 등록/예제·실행/메모리/범위 문서 |

## 타입 변경표

| 구분 | 타입 | 역할 |
|---|---|---|
| 생성 | RoutedCheckBackend | 단일 사용의 신뢰된 host backend·target/요구 메모리/실행 경계 |
| 생성 | RoutedCheckReport·RoutedCheckEvidence | 원 receipt/완료·원 source·실제 IO 종료 뒤 부분 결과 |
| 생성 | HostCheckEvidence·Runner | native/file/routed scope dispatch |
| 생성 | SnapshotCheck·Identity | bounded export 보관 및 원 session/revision/clock admission 연결 |
| 수정 | CheckHost·CheckStage | routed 등록/run·공통 관리 명령·Transport 오류 |
| 이동/수정 | Action·Options·BridgeOutcome·BridgeReport | 공유 생산 모듈로 이동, Fail/원 오류/실제 deadline stop·부분 receipt 유지 |

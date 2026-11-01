# [REV-032] TASK-017 생산 모듈·실앱 export snapshot 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #007 / AI 개발 자가 점검. 독립 리뷰가 아니다.
- 대상: client 실행 예제/host 흐름·통합 시험·manifest/lock/사용법, TASK-017 S3 첫 생산 모듈 연결 단위.
- 기준: [code-review 스킬](../../../.agents/skills/code-review/SKILL.md)·API §15.6·TC-010·coding convention. 신규 기능/불명 경계 시험을 먼저 작성해 compile Red를 확인하고 구현했다.
- 상태/판정: Closed / Pass. Open → In Progress → Resolved → Closed를 개발 자가 점검 및 기존 위임 수락으로 이 회차에 처리했다. 신규 미해결 P1/P2/P3 없음.
- 근거: [EV-034](../verification_evidence.md#ev-034)·[결과/형상](../../06_evolution/imported_context/task_017_routed_2026-11-01/results.json). source commit은 결과의 `source_commit`/`sources` 대조를 따른다.

## 요약표

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리·검증 | 상태 |
|---|---|---|---|---|---|
| 해당 없음 | — | 변경 범위 | 신규 미해결 지적 없음 | 아래 경계/검증 확인 | Closed |

## [리뷰어] 검토

| 영역 | 판단·근거 |
|---|---|
| RT/원자성 | core/RT/unsafe/원자 연산 변경 없음. app의 기존 export 파일을 일반 host에서 한 건 읽고 전용 session으로 수집. dot-probe 제어/안전 스레드를 수정하지 않음 |
| 실제 수집/라우팅 | synthetic CapturedRecord를 직접 만들어 offer하지 않음. 실제 raw loan/try_record_raw receipt와 Router Delivery를 사용. routed bytes/receipt 동일 여부 확인 |
| ACK 경계 | 완전 조립 뒤에도 Pending/raw 슬롯 유지. 큐 try_send 성공 이후 handoff ACK와 RemoteDelivery::poll을 거쳐 실제 완료 게시. 큐 포화/연결 종료 Unknown. 큐 수락 후 ACK 불명은 데이터 인계를 취소하거나 완료로 바꾸지 않음 |
| 소유권/종료 | 실제 Delivery를 ACK/Unknown 게시까지 유지, dispatcher 반환 회수. stream/client·hub/server 실제 close를 기다리고 공유 external 예약0 관측. primary operation 오류도 server shutdown 후 전파. panic/kill의 정리까지 보장하지 않음 |
| 예산/입력 | 일반 파일64KiB+1 bounded read·최종 길이 재검사/형태/thread 수 제한. raw/route/큐 각1. logger budget16MiB/단일 current-thread runtime. 신규 capture/인계2초 및 취소 재검사·settlement 최대3초. RSS·OS IO hard deadline 증명은 아님 |
| 출처/시계 | 원 JSON byte 불변, app 누적 tick 수와 별도 logger mono ns 구분. boot ID/freshness 없음은 Unavailable. Report/profile/capacity·RT/성능 근거를 새로 만들지 않음 |
| 의존성/권한 | 기존 serde_json 고정 버전의 dev edge만 추가. wire·관리 권한·CheckHost/RunCheck backend 의미 변경 없음. dot-probe는 빌드/실행/기존 exporter 읽기만, 저장소 변경 없음. 공개/CI/사용량 조회 없음 |
| 수락 범위 | 별도 명시 실행 snapshot 예제만 수락. in-process 상시 수집/화면·off/on·LAN/PC/RT/UPS/장기/native 및 routed 운영 backend 등록은 미검증/미구현으로 유지. T1 미도래 |

## [처리담당] 결과

- 신규7개 통합 시험: 실제 큐·원 receipt 완료/원본 회수, 거절/연결 종료·수락 후 ACK 불명, 무수신자/사전취소/시간 부족, 형태/입력 한계·원본 불변, multi-chunk 원문 동일.
- Rust1.96/MSRV1.85 workspace 각각364개·fmt/clippy/rustdoc/protocol·client core 의존 분리·실제 app source 실행 Pass. native/새 Miri 실행은 이 근거에 포함하지 않음(unsafe 변경 없음).
- 코드·사용법·API/Task/EV/추적표·상태를 동기화하고 소스/문서를 분리 커밋·게시한다. 새 ADR/H/J 결정 없음.

## 파일·타입 변경

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | client/examples/dot_probe_bridge.rs | 무부하 preview·명시 run·bounded 소비 큐·관측 JSON/exit |
| 생성 | client/examples/support/dot_probe.rs | exporter bounded read·원 raw 수집/라우팅·실제 socket/ACK·정리 |
| 생성 | client/tests/routed_check.rs | 신규 통합7개 |
| 수정 | client/Cargo.toml·Cargo.lock·client/README.md | 예제 등록·기존 serde_json dev edge·재현/수락 경계 |

| 타입 | 구분 | 역할 |
|---|---|---|
| Action | 생성/예제 내부 | 큐 수락/거절, 시험에서 연결 종료 주입 |
| Options | 생성/예제 내부 | 수신자 존재·유한 admission 시간 |
| BridgeOutcome | 생성/예제 내부 | Pass/Aborted/Inconclusive |
| BridgeReport | 생성/예제 내부 | 실제 receipt·ACK/Unknown·raw/예약 반환 관측 |

다음 시작점은 고정 host의 routed RunCheck 등록/출처 연결과 dot-probe 지속 수집·화면 통합 범위다. TASK-017/MS-005·TC-010 전체·Gate4/5 미완료 및 Yellow를 유지한다.

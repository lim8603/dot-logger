# [REV-023] TASK-014 운영 계측·유한 집계 개발 자가 점검

## 개요

- 2026-10-31 #011 / AI 개발 세션 자가 점검. 독립 리뷰 세션으로 가장하지 않는다. Human 「권고대로 진행하자」와 기존 수락/게시 위임 범위.
- 대상: telemetry/{mod,counters,aggregate}.rs, CaptureOwner/Scope/Producer, Completion의 session/revision/target_totals, TC-009·RT allocator 추가2개·overhead fixture.
- 상태: Closed — EV-027/소스d99ba3b·source97개 commit blob 확인 후 위임된 AI 로컬 수락.
- 판정: 로컬 정적 점검에서 신규 P1/P2 지적 없음. 기존 성능/실앱/OS·RT/native 공백은 유지한다.

## 검토 결과

| 영역 | 판단과 근거 |
|---|---|
| RT | small/raw/acquire는 결과별 producer-local atomic load/store만 추가. 1writer는 nonclone Producer·Scope iterator로 보장하고 reset은 &mut owner로 모든 borrow 종료 뒤만 허용. Mutex/sample/할당/clock/반복은 관리 경로에만 위치. raw 거절 owner·ACK·receipt 의미 불변 |
| 원자성/수량 | 카운터는 payload publish에 사용하지 않으므로 Relaxed. 동시 전체 시점 일관성을 주장하지 않고 quiescent exact 대조. 포화는 clamp+flag, delta는 overflow/reset/missing 구간을 성공 수치로 만들지 않음 |
| 예산/수명 | enable 전에 capture+completion+counter+301이력의 control 합계 검사. idempotent enable·소유 owner general Drop, scope/telemetry borrow 수명 유지. 관리 ring 회전/조회는 유한301개이며 샘플마다 재할당하지 않음 |
| 완료 권위 | target_totals는 기존 live/retired 집계를 target ID로 선택. 기존 totals(required_only) 경로 유지. records는 session ingress, assigned는 fanout이며 의미를 문서화 |
| 관측 의미 | session/revision/epoch/실제 시각 보존, MissingCompletion/ForeignSession/IncompleteWindow/Saturated/Stale 구분. 기본 disabled의 None은 미계측이고 runtime backend의 0으로 바꾸지 않음 |
| 시험 | API 부재 compile Red→TC-009 로컬6개 Green·포화 private1·allocator2추가. 동시 조회·raw/skip/reject·retired target·reset/301ring·누락/foreign/control 부족. 두 Rust/full matrix/Miri는 EV-027로 확정 |

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | telemetry/mod.rs·counters.rs·aggregate.rs | 고정 producer 카운터·표본·이력/구간 |
| 수정 | capture/mod.rs·capture/tests.rs | 명시 enable/reset, owner/scope 관측, RT 결과 계측 |
| 수정 | completion/mod.rs·lib.rs·Cargo.toml | 고정 target 집계·모듈·fixture 등록 |
| 생성/수정 | tests/tc_009_telemetry.rs·tests/rt-allocation/src/tests.rs·examples/telemetry_overhead.rs | 수량/예산/수명/비용 fixture |

## 타입 변경표

- 생성: CaptureCounts·Observation·Unavailable·Telemetry·Sample·Window·TelemetryError. private CounterCell·Bank·History.
- 수정: CaptureOwner·CaptureScope·Producer(빌린 counter), Completion(관측 accessor/대상 합계). Entry/WriteState·unsafe 수명 구현·wire/외부 dependency는 수정하지 않음.

## 미검증/다음 행동

기본 계측 처리량 비용≤5%·실제 RT 지연·PC/LAN/RSS/OS/native·장기 부하/실앱은 NotRun. fixture debug128회는 실제 성능 수락이 아니다. 원격 CLI metrics backend는 여전히 Unavailable이며 자동 sample worker·용량 분모/추정/설정 변경은 TASK-015~017/관련 호스트 후속이다. TASK-014 로컬 범위 이후 TASK-015는 다음 개발 후보이며 이번 작업에 자동 포함하지 않는다.

## 최종 검증·수락

[EV-027](../verification_evidence.md#ev-027) source97개/검사14개 exit0·두 Rust 각16feature/consumer/workspace·SDK/CLI/allocator 회귀·Miri56·디스크/commit blob 일치. 비용≤5%·실제 RT/PC/native·실앱 NotRun과 status backend Unavailable은 유지한다. 신규 미해결 P1/P2 없음, 기존 위임으로 로컬 구현과 리뷰를 수락/마감한다. 계측/이력 선택은 승인된 API§15의 내부 구현이며 새 핵심 기술/장기 불가역 프로토콜 결정이 아니므로 ADR을 추가하지 않았다.

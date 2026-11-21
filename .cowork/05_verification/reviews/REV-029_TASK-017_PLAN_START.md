# [REV-029] TASK-017 S1 무부하 계획·예산 판정 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #004 / AI 개발 자가 점검. 독립 리뷰로 표현하지 않는다.
- 요청: Human 「프로젝트 진행 파악하고, 리뷰 반입사항이 있다면 수정 반영 후, 다음 작업 시작」. 기존 구현·검증·리뷰 마감·분리 커밋/푸시 위임 적용.
- 대상 형상: 75c6096, `diagnostics/mod.rs`·`lib.rs`·`tc_010_diagnostics.rs`. S1 계획/예산 순수 판정만 대상이며 RunCheck 전체 수락이 아니다.
- 스킬: [code-review](../../../.agents/skills/code-review/SKILL.md) 체크리스트 적용.
- 상태/판정: Closed / Pass — 신규 미해결 P1/P2/P3 없음. [EV-031](../verification_evidence.md#ev-031), 기존 위임 S1 로컬 수락.

## 요약표

| ID | 중요도 | 위치·계약 | 문제/제안 | 처리·검증 | 상태 |
|---|---|---|---|---|---|
| 신규 RV 없음 | — | TASK-017 S1 | 계획/admission 범위 구현·개발 자가 점검 | EV-031 Red→Green·회귀, S2 실제 IO 후속 명시 | Closed |

## [리뷰어] 검토

| 영역 | 판단/근거 |
|---|---|
| RT/동시성 | 새 API는 일반 경로이며 capture/원자 연산/unsafe/lock/worker를 바꾸지 않는다. RT/allocator 기존 회귀 Pass. stateless admission을 실제 예약이나 직렬화로 표현하지 않음 |
| 무부하 | build는 소유 CheckSpec의 값/경로 문법만 확인. FS/clock/thread/network 호출 없음. temp fixture의 운영 파일 bytes 불변·새 경로 미생성 확인 |
| 예산 | 0 한도/worker/필요 메모리 및 memory 초과를 거절. checked 경계 판정에서 포화 차감으로 u64 wrap 없이 신규 쓰기 중단. physical bytes+outstanding/finalization과 reserve를 구분하고 payload를 물리 쓰기로 치환하지 않음 |
| 경로/수명 | 절대 경로·루트/상위 탐색 거절은 문법 검증뿐이다. 기존 경로 삭제/소유권/symlink 안전을 보장하지 않으며 실제 IO owner의 수명/정리는 S2 후속 |
| provenance/측정 | session/revision·환경/부하·boundary·선언 예산을 그대로 유지. required_memory는 host 계산 선언, capability/실제 예약/성능 수락 아님. 수신자 부재/loopback/remote 요청은 미측정 선언만 유지 |
| 오류/공개 API | 고정 PlanError·StopReason, 공개 문서 있음. 외부 dependency/runtime/wire 타입 신규 노출·unwrap/expect/panic 없음. 새 ADR/기술/공유 문서 구조 결정 없음 |
| 범위 | actual run/IO/drain/sync/close/cleanup·최종 결과·CLI/runtime/backend·실앱/PC/RT/native를 구현/검증 완료로 올리지 않음. Task/MS/TC 전체 미완료 |

## [처리담당] 처리·검증

- API 부재 compile Red(E0432, exit101) 확인 뒤 최소 구현, S1 신규5개 Green.
- 설정/비교17개와 합계22개를 Rust1.96·MSRV1.85에서 확인. workspace/all-features329개(문서 시험 포함), fmt·clippy/rustdoc -D warnings Pass. 근거/log/hash는 EV-031.
- S1에는 unsafe/생산자 동시성 변경이 없어 Miri를 추가 실행하지 않았다. 과거 Miri Pass를 현재 S1의 실행 결과로 재사용하지 않는다.
- 새 finding 없음. 기존 REV-028은 별도 리뷰 파일의 처리담당 영역에서 RV-063/064 수용·Closed로 마감했다.

## 파일 변경표

| 구분 | 파일 | 목적 |
|---|---|---|
| 생성 | crates/dot-logger/src/diagnostics/mod.rs | 무부하 계획/선언 예산과 신규 쓰기 allowance |
| 수정 | crates/dot-logger/src/lib.rs | diagnostics 공개 |
| 생성 | crates/dot-logger/tests/tc_010_diagnostics.rs | 무부하·예산·reserve·취소·미측정/경로 경계5개 |
| 생성/수정 | 본 리뷰·API §15.3·TASK-017/Task·MS·TC/EV/project_state | S1 수락 범위와 S2 다음 시작점 |

## 클래스(타입) 변경표

| 구분 | 소속 | 타입 | 역할 |
|---|---|---|---|
| 생성 | diagnostics/mod.rs | CheckBudgets·CheckSpec·CheckPlan | 명시 한도/원본 식별자·유효 선언 |
| 생성 | diagnostics/mod.rs | NetworkPlan·PlanError | 미측정 네트워크 계획 범위·선언 거절 |
| 생성 | diagnostics/mod.rs | Progress·StopReason | host의 실제 진행 관측·신규 부하 중단 사유 |

## 다음 행동

TASK-017 S2 실제 명시 run·전용 경로 소유/운영 파일 보존·물리 쓰기/outstanding/finalization 예산·취소/IO 오류 뒤 실제 종료/정리·결과 분류 TDD. 운영 backend/CLI 연결과 실제 성능/실앱/native 후속. FR-015/TASK-018 미승인 범위는 착수하지 않는다.

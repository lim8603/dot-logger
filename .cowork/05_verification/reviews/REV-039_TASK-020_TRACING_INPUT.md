# REV-039 — TASK-020 S1 tracing/log 일반 입력 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #014 / AI 개발 세션 자가 점검. [code-review 스킬](../../../.agents/skills/code-review/SKILL.md) 적용, 독립 리뷰 세션과 구분한다.
- 대상: 선택 `tracing-adapter` 입력 모듈·TC-007 하위 시험·general_logs 예제·Cargo/README. 소스 hash는 최종 [결과](../../06_evolution/imported_context/task_020_s1_2026-11-01/results.json)에서 고정한다.
- 종합 판정: **Pass — S1 로컬 기능 범위**. 미해결 지적 없음.
- 상태: **Closed** — Open → In Progress → 검증/조치로 Resolved → 기존 위임으로 S1 로컬 수락. 소스 `c08ad25`.

## 리뷰 항목 요약표

| ID | 중요도 | 위치/계약 | 문제/처리 | 판정 | 상태 |
|---|---|---|---|---|---|
| RV-081 | P2 | general_logs 예제·TC-007 / MSRV1.85 | placeholder가 있는 format_args 임시 인자를 log Record에 저장하면 1.85에서 E0716. 같은 log_record 호출 안에서 생성/소비하도록 수정 | 수용·초기 MSRV 실패→수정 후 전체 회귀 Pass | Closed |

## [리뷰어] 점검

- RT/동시성: 일반 경로 함수만 추가했다. 새 unsafe·원자 변수·락·worker는 없다. 기존 RT capture/routing/completion 구현은 불변이다. 사용자 Debug/Display는 할당·대기·재귀할 수 있어 bounded bytes와 RT/시간 보장을 구분했다.
- 소유권/예산: 결과는 caller scratch의 불변 borrow다. 재사용/해제는 Rust borrow로 제한된다. 별도 heap·큐·외부 예약을 만들지 않으며 scratch 및 사용자 formatter 비용은 host worker 예산이다. `Buffer.len <= bytes.len`을 유지한 차이 검사 후 copy하므로 usize 가산/인덱스가 넘지 않는다.
- 인코딩: JSON target/field name/value를 escape, 모든 C0 control·quote/backslash·Unicode를 처리한다. primitive 문자열은 원문, 그 외 Debug 문자열이다. typed 수치와 span context 복원은 제공하지 않는다. 초과/formatter 실패는 전체 거절하며 partial prefix를 반환하지 않는다. overflow는 sticky라 formatter가 오류를 무시해도 수락하지 않는다.
- API: 외부 public Event/Record는 선택 어댑터 입력에만 등장하고 core API·runtime/wire 의존을 바꾸지 않는다. native small admission 한 번만 호출하며 severity 외 host identity/time/importance·필터/sampling/오류·입력 receipt 의미를 유지한다. 전역 설치·암묵적 경로 변경·자동 완료·새 retry는 없다.
- 범위: S2 파일/DB/구독·조절 비교, S3 화면 구독 불연속 예제, TASK-026 실제 앱·PC/RT/UPS/장기/native는 별도다. S1 수락을 Task/MS/TC 전체 완료로 표시하지 않는다.

## [처리담당] 판정·검증

- RV-081 수용. [초기 MSRV 컴파일 실패](../../06_evolution/imported_context/task_020_s1_2026-11-01/msrv-initial-failure.log)의 E0716 두 곳을 수정했다. 라이브러리 API·capture 동작 변경은 없다.
- API 부재 compile Red(exit101) 뒤 최소 인코딩/수집 연결로 Green. 신규8개 시험: scoped subscriber 실제 trace/debug/info/warn/error와 own fields, JSON target/문자열/C0·Unicode, 숫자/bool text, exact-size/short 모든 prefix, Debug/Display 실패·swallowed overflow, capture identity/time/importance·Capacity/Closing·Skipped/schema/slot 크기를 확인했다.
- 최종 검증: Rust1.96/MSRV1.85 workspace 각각392개+doctest17개 Pass, 기존 실제 DB4개 ignore. 두 Rust 각각16 feature 조합·독립 core/default client graph·RT allocator15개/remote15개·예제, fmt/clippy/rustdoc(-D warnings), 새 예제 두 Rust 실행 Pass. [EV-040](../verification_evidence.md#ev-040).
- 새 unsafe/동시성 제어 변경이 없어 Miri 새 실행은 하지 않는다. native(T3)·다른 OS/RT/성능 수락도 미실행이다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | `crates/dot-logger/src/adapters/tracing.rs` | 일반 경로 인코딩 및 명시 small admission 훅 |
| 생성 | `crates/dot-logger/tests/tc_007_tracing.rs` | 실제 subscriber·오류/한계·capture 통합8개 |
| 생성 | `examples/general_logs.rs` | 일반 log→입력 수락/소비 예제 |
| 수정 | `crates/dot-logger/src/adapters/mod.rs`, `crates/dot-logger/Cargo.toml` | 선택 feature 연결·예제 등록·기존 serde_json의 검증용 dev 의존 |
| 수정 | `README.md` | 호출/표현/RT·S1 범위와 실행 명령 |

## 클래스(타입) 변경표

| 구분 | 타입 | 역할 |
|---|---|---|
| 생성 | `EncodedLog` | caller scratch borrow·source Level·단일 capture 제출 |
| 생성 | `EncodeError` | TooLarge/Formatting 전체 거절 |
| 생성(내부) | `Buffer`, `Escaped`, `Fields` | 길이/overflow·JSON escape·tracing 방문 |
| 생성(검증용) | `Hook`, `BadFormat`, `BadDebug`, `SwallowsOverflow` | scoped subscriber와 formatter 오류 경계 검증 |

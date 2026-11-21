# REV-041 — TASK-020 S3 화면 구독 불연속 개발 자가 점검

## 개요

- 일자/담당: 2026-11-02 #001 / AI 개발 세션 자가 점검. [code-review 스킬](../../../.agents/skills/code-review/SKILL.md) 적용, 독립 리뷰 세션과 구분한다.
- 대상: `dot_probe_tail`·shared `support/tail`·`tail_screen` 시험·client Cargo 예제 등록·README. core/client 제품 src·wire·의존 버전·lock 변경 없음.
- 상태: **Closed** — Open → In Progress → RV-083 조치/검증으로 Resolved → 기존 2026-10-31 위임에 따라 S3 로컬 수락/마감.
- 종합 판정: **Pass — S3 로컬 화면 예제 범위**, 새 미해결 지적 없음. 실제 지속 앱/정량 성능/native는 범위 밖이다.
- 근거: [EV-042](../verification_evidence.md#ev-042)·명령/로그(비공개 원시 근거).

## 리뷰 요약

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-083 | P2 | `support/tail/mod.rs` Screen::record·화면 ID/clock | u128 schema를 JSON 숫자로 인코딩하면 큰 ID에서 panic, 큰 정수/시각·source clock도 화면 계약의 정확 표기 필요 | ID hex·큰 정수 문자열·구조화 source clock | 수용·u128::MAX/i64::MIN/u64::MAX 반례 Red→Green | Closed |

## [리뷰어] 점검

- **RT/원자성:** 일반 경로 예제에만 JSON/문자열/Vec/IO를 추가했다. RT 호출/Drop, unsafe/원자성·제품 src를 바꾸지 않는다.
- **구독 수명:** local/server TimedOut의 동일 오류를 보류 표시하고 같은 구독에서 재대기한다. None/취소/기타 오류는 종료 구분선, 새 subscribe 성공은 새 segment다. 자동 재연결/재전송/heartbeat·Gap 복원 없음.
- **표시/ACK:** 종료/재구독의 unknown 누락과 구독 내 inclusive transfer Gap을 분리한다. 화면 queue admission만 consumer 수락이며 렌더링은 별도다. ACK unknown은 이미 인계된 행을 지우지 않는다. 이력이 차면 마지막 종료행을 보존하고 기록을 거절해 ACK하지 않는다.
- **한도/정리:** 최대64행·payload64B/행·종료 전용1행, 고정 실행32행·수신1슬롯·두 연결/구독·유한 poll/timeout이다. 오류/취소에서도 hub close·서버 종료 신호/join(기한 초과시 abort+await)·runtime 실제 Drop을 수행한다. shared 예약과 JSON/runtime/OS RSS를 구분한다.
- **API/격리:** SDK/wire/remote 오류 의미·라이브러리 feature와 core를 변경하지 않는다. 새 example만 remote를 요구한다. default dependency graph를 대조한다.

## [처리담당] RV-083

- 판정: 수용. 큰 schema ID에서 serde_json json! 내부 number out of range panic을 실제 재현했다(Red(비공개 원시 근거)).
- 조치: schema/session은 32자리 hex, sequence/revision/mono/real/source ticks/domain·transfer 위치는 문자열로 출력한다. source_time은 ticks/domain 구조 또는 null이며 원래 시각을 재해석하지 않는다.
- 검증: `large_native_identifiers_and_signed_source_clocks_are_exact_in_screen_json` Green. 신규8개에는 종료/재구독·local wait·Gap/span·stale 화면 호출·history full·실제 lease/새 TCP/취소·실제 미ACK/Unknown·예약0·한글 출력·취소/중첩 runtime 거절이 포함된다.
- 최종 검증: source `a48b80d`·검증파일134개 commit blob 일치, 두 Rust workspace각421개(DB5개 ignore)·default/remote·fmt/clippy/doc·CLI0/4·default graph Pass. EV-042로 S3 로컬 수락/Closed. 실제 지속 dot-probe/화면·장기 무손실 tail/재구독 복원·LAN/RSS/PC/RT/UPS/native는 후속이다.

## 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | `examples/dot_probe_tail.rs`·`support/tail/{mod,scenario}.rs` | 유한 한국어 화면/JSON·구독 경계·실제 loopback 수명 시나리오 |
| 생성 | `tests/tail_screen.rs` | 신규8개: 표시/ID·실제 lease/재연결/미ACK/정리 경계 |
| 수정 | client Cargo.toml·root/client README | remote 전용 예제 등록·계약/실행/예산/미수락 범위 |

| 구분 | 타입 | 역할 |
|---|---|---|
| 생성 | `Screen`·`EndReason`·`Step` | 유한 표시 이력·종료 원인·수신/ACK/대기/종료 구분 |
| 생성 | `TailReport`·`scenario::Mode` | 고정 기능 근거·정상/이력 포화/접속 종료 시나리오 |

독립 리뷰/제품 전체 Pass가 아니다. 실제 화면 적용은 TASK-026, 다음 구현 단위는 TASK-021이다.

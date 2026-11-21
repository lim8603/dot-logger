# REV-050 — TASK-028 S1 정확 배정·preset·plan 개발 자가 점검

## 개요

- 일자/담당: 2026-11-07 #001 / AI 개발 세션 자가 점검. 독립 리뷰가 아니다.
- 대상: 소스 `c92fb98`(S1a: capture·completion·control·routing·storage 예약 공식 추출, facade `derive` 정확 배정), `d23de85`(S1b: `logger/preset.rs`의 `Sizing`·`Preset`, `Budget::apply`, `LoggerBuilder::{preset, plan, resolve}`, `Plan`), TC-013 F14·F15.
- 상태: **Closed** — 결함 수정이 필요한 항목은 없다.
- 판정: **Pass — 로컬 기능 범위.** 근거: [EV-052](../verification_evidence.md#ev-052).

## 리뷰 요약

| ID | 중요도 | 위치·계약 | 문제 | 처리 | 상태 |
|---|---|---|---|---|---|
| RV-115 | P2 | 공식 이중화 위험 | 사이징이 core 예약 공식을 따로 구현하면 core 변경 때 어긋난다 | 생성자가 같은 함수를 호출하도록 추출했다. F14가 여러 형태(다중 writer·raw·출력 2개·telemetry)에서 보고값 = 실제 필요량을 1바이트 단위로 확인 | 조치 |
| RV-116 | P3 | 공유되지 않는 예약 | remote(external)·worker stack은 별도 공식이고 query/replay 예약은 facade가 쓰지 않는다 | remote는 `RemoteLimits::reservation()` 그대로, stack은 facade 상수. 사용하지 않는 core 경로는 계산하지 않음을 §19.3에 명시 | 문서화 |
| RV-117 | P3 | 부족 표기 | 명시 메모리 부족이 이제 partition별이 아니라 `Total`로 보인다(이전 S2는 Control/Output이 보일 수 있었다) | 총 요구량이 정확하고 충분하므로 사용자가 바로 쓸 수 있는 값을 준다. partition별 값은 `plan()`의 `memory`로 확인 | 문서화 |
| RV-118 | P3 | 흡수 시간 의미 | 흡수는 "IO 정지 중 RT 거절이 없는 시간"의 계획값이며 실제 IO 지연 분포를 보장하지 않는다 | 실측 근거는 TASK-023 S2/S3, 장비별 보정은 S2 auto preset | 후속 |
| RV-119 | P3 | 공개 표면 증가 | 이번 세션 facade 공개 타입 9개 추가 | NFR-008 ≤60 계수는 TASK-029 가드에서 확인 | 후속 |

## [리뷰어] 점검

- **범위/권한:** API §19.1 Human 결정 범위 안. 새 의존성·Cargo.lock 변경·unsafe 없음. core 공개 시그니처 변경 없음(추가된 공식 함수는 crate 내부).
- **사용자 값 우선:** `Budget::apply`는 `None`인 값만 채운다. F15가 사용자 메모리·sync 주기 유지와 `preset_filled` 목록을 확인한다.
- **부족 설명:** preset 부하에 비해 사용자 메모리가 작으면 plan·start 모두 같은 `required`로 거절하고 파일을 만들지 않는다(F15).
- **시험 신뢰도:** F14는 구현 전 Red(보고값 4,209,633B로도 control 0으로 거절). F15는 컴파일 Red 뒤 Green. slot 여유 1.25→1.0 돌연변이에서 F15 5개 중 4개 실패를 확인했다.

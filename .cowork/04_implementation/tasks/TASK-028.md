# TASK-028 — 예산 preset·사이징 도우미·장비 측정 auto preset

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-008 |
| Story | US-002·004·005 |
| 요구사항 | NFR-005 NFR-006 NFR-008 FR-015(장비 측정 부분) FR-016 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-004·005·011 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-027(S1 계약)·TASK-017·TASK-023(S2의 정확도 oracle) |
| 예상 소요 | S1 1~2 + S2 2~3 인일, 측정/리뷰 대기 제외 |

## 목표

메모리 7분할·풀 slot·보존 예산을 직접 계산하지 않도록, 채널별 주기·payload 크기·중요도·보존 시간·디스크 같은 상위 입력을 `SessionConfig`/`RetentionConfig`로 바꾸는 결정적 계산과 이름 붙은 preset을 제공한다(S1). 동작 PC를 짧게 측정해 preset 초안을 만드는 auto preset을 선택 기능으로 제공한다(S2).

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 결정적 사이징·preset | `Sizing` 입력→설정·계산 근거, preset 3~4종(제어 경량·센서 원본·블랙박스 순환·혼합; 수치 근거는 TASK-022 local-pc-v1), 부족 항목·부족량 설명 오류 | Done(로컬) — 2026-11-07 Human 결정 3건, [API §19](../../03_design_artifacts/interface_contract.md#19-예산-preset사이징-계약--2026-11-07), [EV-052](../../05_verification/verification_evidence.md#ev-052) |
| S2 | 장비 측정 auto preset | 아래 설계로 측정 프로필 파일 생성→`Sizing::profile` 적용, TASK-023 정식 결과 대비 오차 기록 | Done(로컬)·Human 승인 2026-11-11 — [API §19.8·19.9](../../03_design_artifacts/interface_contract.md#198-task-028-s2-auto-preset-계약--2026-11-11), [EV-071](../../05_verification/verification_evidence.md#ev-071) |

## S1 Human 결정과 계획 — 2026-11-07

Human이 리뷰 포인트 3건을 모두 권고안으로 정했다. preset은 `control`·`sensor`·`black_box` 3종 + `Sizing`, 흡수 1초 × slot 여유 1.25·메모리 +10%(`black_box` 흡수 2초), start 시 계산 + `plan()`. preset은 비어 있는 손실 값만 채우고 사용자 값을 덮어쓰지 않는다. 상세 [API §19](../../03_design_artifacts/interface_contract.md#19-예산-preset사이징-계약--2026-11-07).

| 하위 | 범위 | 상태 |
|---|---|---|
| S1a | core 예약 공식을 생성자와 공유하는 순수 함수로 추출하고 facade `derive`를 정확 요구량 배정으로 교체(RV-097) | 로컬 Pass — 소스 `21cc198`, F14 |
| S1b | `Sizing`·`Preset` 3종·builder `.preset`·`plan()`과 근거, 사용자 값 우선, 부족 항목·부족량 | 로컬 Pass — 소스 `4465be3`, F15, [EV-052](../../05_verification/verification_evidence.md#ev-052), [REV-050](../../05_verification/reviews/REV-050_TASK-028_S1_PRESET.md) Closed, API §19.3 |

## S2 Human 결정과 계획 — 2026-11-11

착수 시 질문 3건을 Human이 모두 권고안으로 정했다(「짧은 측정 + 보수 추정」·「JSON」·「흡수 시간·메모리·부하 경고」). 상세 [API §19.8](../../03_design_artifacts/interface_contract.md#198-task-028-s2-auto-preset-계약--2026-11-11).

- **수락 오차:** 추정 지속 처리량 ≤ 1.2 × TASK-023 S3 S(1.544 GB/s → ≤ 1.853 GB/s). 짧은 측정은 SSD 캐시 때문에 지속값을 과대평가하므로(이 T500 순간 약 2.65 GB/s) 지속값은 순간 × 0.5인 Estimated로만 둔다. 장기 모드는 없다.
- **이름 정정:** 설계의 `Preset::from_profile`은 부하(hz·bytes)가 사용자 입력이므로 `Sizing::profile(&Profile)`(어느 preset에도 이어 붙임)로 구체화한다.

| 하위 | 범위 | 상태 |
|---|---|---|
| S2a | feature `calibrate`: `Calibration` 한도·취소·정리, host 사실(Observed/Unavailable), `Profile` JSON 저장/읽기 — F19 | 로컬 Pass — EV-071 |
| S2b | `Sizing::profile` 흡수 반영(사용자 값 유지), `Plan::warnings` 메모리·부하 경고 — F20 | 로컬 Pass — EV-071 |
| S2c | CLI `dot-logger calibrate`, T500(`/home/lim8603/s3-data`) 3회 정확도 확인(쓰기 약 12 GB) | 로컬 Pass — CLI 시험, 추정 지속 0.78·1.47·1.42 GB/s(≤ 1.853), SSD 12.89 GB, EV-071 |

## S2 auto preset 설계 — 2026-11-04 Human 요청

- **실행 시점:** 설치·커미셔닝 때 1회 `dot-logger calibrate`(CLI)/API로 실행하고 버전 붙은 프로필 파일로 저장한다. 매 시작 시 측정은 기본 off(시작 지연·디스크 마모·비결정성).
- **측정 항목:** CPU 수·online/isolated CPU, governor(기록만), 커널·PREEMPT_RT, RAM total/available·cgroup 한도, filesystem 종류·여유, 유한 지속 쓰기+fdatasync 처리량·sync 지연(TASK-017 RunCheck·TASK-023 P0 writer 재사용), 선택 짧은 제어 루프 wake 지터.
- **산출:** preset 초안·측정 근거(Observed/Estimated/Unavailable 구분)·안전 여유 계수·사용자 명시 값과의 diff.
- **금지:** 사용자 고정 값 덮어쓰기, 명시 opt-in 없는 자동 적용, 미측정 값의 과신 추천(TASK-018 규칙 계승). Tier 2/미지원 항목은 Unavailable과 보수적 기본값·대체 근거를 남긴다.
- **한계 표시:** page cache·온도·governor 변동, 측정 시점 snapshot이며 보장 아님. 측정 디스크 사용은 RunCheck 예산으로 제한한다.
- **정확도:** 같은 PC의 TASK-023 S3 정식 측정과 비교해 추정 오차를 기록한다. 수락 오차는 S2 착수 시 제안(기준 후보 NFR-006 추정 오차 ≤20%).

## 변경 대상과 첫 검증

구현 경로: `crates/dot-logger/src/logger/preset.rs`, `crates/dot-logger/src/calibrate.rs`, `crates/dot-logger-cli`(2026-11-13 정정: 초기 계획 문구가 남아 있었다).

첫 실패/리뷰 조건: **입력 부하 대비 예산 부족을 항목·부족량과 함께 거절하고, 사용자 고정 값을 덮어쓰지 않으며, 미측정 장비 값은 Unavailable과 대체 근거로 표시**.

- [x] (EV-052·071 Red) 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] (TC-013 F12·F13·F19·F20) 최소 구현 후 관련 `TC-013`의 preset/auto preset 하위 시나리오를 통과시킨다.
- [x] (EV-072 REV-053 조치 포함) 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## Human 확인 필요 사항

- preset 종류·이름과 안전 여유 계수 기본값.
- ~~auto preset 수락 오차와 프로필 파일 형식(TOML/JSON) — S2 착수 시.~~ 2026-11-11 결정(위 S2 절).
- FR-015의 질문 프리셋 부분은 [TASK-018](TASK-018.md)에 남는다(제안 유지).

## 완료 기준

- [x] (EV-052·071) S1 결정적 사이징·preset, S2 auto preset 측정·저장·적용과 금지 규칙 검증.
- [x] (EV-052·071·078) 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] (사전 취소·reserve EV-071, 측정 중 취소 2026-11-13 EV-078) 실패/포화·취소/종료·예산 경계(측정 중 취소·디스크 reserve 포함)를 해당 범위에서 검증한다.
- [x] Human 리뷰 완료 — S1 REV-051(2026-11-07), S2 2026-11-11 「승인」·REV-053(2026-11-12 「승인」). 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-013](../../05_verification/test_case.md#tc-013). 실행 결과: S1 [EV-052](../../05_verification/verification_evidence.md#ev-052), S2 [EV-071](../../05_verification/verification_evidence.md#ev-071).

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- native·다른 OS: [TASK-024](TASK-024.md)(T3, GAP-004)
- 측정 도중 취소 시험: 2026-11-13 추가·변이 확인(EV-078)
- 실사용처 적용: [TASK-026](TASK-026.md)(마지막 단계, 일정 판단 밖)

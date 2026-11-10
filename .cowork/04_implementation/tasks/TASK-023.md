# TASK-023 — P0~P5 성능·RT·자원 검증

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-001·002·003·004·005 |
| 요구사항 | NFR-001 NFR-002 NFR-005 NFR-006 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-08 |
| 다음 액션 | S3 1회차 [EV-064](../../05_verification/verification_evidence.md#ev-064)·후속 [EV-065](../../05_verification/verification_evidence.md#ev-065) — P1·제어·P2 효율·계측 비용(2.5%) Pass, 정상 부하 raw 거절은 harness 풀 사이징(preset 35칸이면 0), P3 효율은 장치 표류로 미판정. 다음: preset 크기 재판정·P3 효율 반복 수 Human 판단. P4·P5 이월 |
| 의존 | TASK-027(S2)·TASK-026(실앱 행)·TASK-017·TASK-021·TASK-022 |
| 예상 소요 | 3~5 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `benches/; tests/profiles/; 검증 evidence`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **P0 기준선 없는 비율·RAM 수락만 잰 처리량을 결과에서 거절**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다. — S1 판정 규칙 Red→Green(EV-046)
- [ ] 최소 구현 후 관련 `TC-012`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] P1~P5 무결성·호출 p99/p99.9·동기화 포함 처리량·TCP 대비·메모리 3종·계측 비용.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-012](../../05_verification/test_case.md#tc-012). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 2026-11-03 의존 조정 — 합성 행 선행

Human 결정(2026-11-03)으로 실제 앱이 필요 없는 합성 행(P0 simple-file 기준선·local-pc-v1 10ms 제어 off/on·P1~P3·P5·계측 비용≤5%)을 TASK-026 전에 현재 PC generic 커널에서 실행한다. RT 행은 합성 generic 측정 뒤 TASK-022 요청 규칙에 따라 Human 승인/부팅 확인 후 실행한다. 실앱 부하·화면 연동 행은 TASK-026 의존을 유지하며 합성 결과로 대체하지 않는다.

## 2026-11-04 순서 조정 — facade 경로 측정

Human 승인(MS-008)으로 S2는 [TASK-027](TASK-027.md) 뒤에 진행한다. S2 harness가 기본 worker를 따로 배선하면 같은 배선을 두 번 만들고 제품 경로가 아닌 것을 재게 되므로, S2/S3는 사용자가 실제 쓰는 facade 기본 worker 경로를 측정한다. S1 P0 harness·판정 규칙(EV-046)은 그대로 사용한다. S3 정식 결과는 [TASK-028](TASK-028.md) S2 auto preset의 정확도 oracle로 쓴다. P4(TCP receiver ACK 대비)는 완료 기준에 있으나 S2/S3 범위 배정이 없으므로 S3 계획 때 포함 여부를 확인한다.

## 2026-10-31 수락 방식·검증 시점 갱신

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체하고 UPS는 시뮬레이션으로 판정한다. native는 GitHub 공개 완료 후 진행한다. 일반 Linux/RT는 실제 실행 커널을 기록하며 물리 전원 소실 내구성의 관측 결과로 표시하지 않는다. 기존 수치/예산/24h/소프트웨어 통합 목표와 선행 의존은 유지하며 이번 문서 갱신만으로 제품 시험을 Pass 처리하지 않는다.

## 단계별 실행 — 2026-11-03 #003

| 단계 | 범위 | 상태 |
|---|---|---|
| S1 | P0 기준선 harness·결과 판정 규칙·합성 제어 루프 | 로컬 구현 완료 — [EV-046](../../05_verification/verification_evidence.md#ev-046), 소스 `3740671`. 정식 측정 NotRun |
| S2 | logger on 경로(P1 256B·P2 8MiB·P3 혼합, 32생산자/16파일/128채널)와 같은 P0 workload 대조·무결성 oracle. TASK-027 facade 기본 worker 경로로 측정. P1에서 `try_log`의 encode·복사 비용을 분리해 기록한다(이월 T4 판정 입력) | TASK-027 뒤 |
| S2a | facade 채널별 출력 선택(`channel_with`·`ChannelOptions::outputs`) — 16파일 workload를 P0와 같게 만드는 제품 변경(Human 2026-11-07 「출력 순서 지정」) | 로컬 완료 — 소스 `1602d5e`, API §18.12, TC-013 F16, 1.96/1.85 각 496 Pass |
| S2b | facade 경로 logger on 측정 도구: P1 `try_log`, 같은 workload의 `StorageRun{tool: Logger}`, MCAP 판독 무결성 oracle, 호출 지연 ns 분포(p99/p99.9), encode·복사 비용 분리(T4 입력). P0 회전 256 MiB·16,384건 | 완료 — [EV-054](../../05_verification/verification_evidence.md#ev-054)·[EV-055](../../05_verification/verification_evidence.md#ev-055). 제품 발견 ①~⑧ 조치(⑥ Human 「권고대로」). smoke: P1 무제한 logger 73.7 MB/s·판독 일치, P0 대비 7.7% |
| S2c | P2 raw(`producer()` loan 8 MiB), P3 혼합(`StorageWorkload` 크기 혼합 확장, 32생산자·16파일·128채널) | 완료 — [EV-057](../../05_verification/verification_evidence.md#ev-057). `Channel::metadata`(Human 「메타데이터 헬퍼」) `e5a005a`, 도구 `145dc4a`, RV-128 정정 `b2899a3`. smoke: P2 logger 성공, P3 logger Unsuccessful(발견 ⑨ Open) |
| S2d | 짧은 시험 실행(측정 아님)·EV 기록. 정식 측정은 S3 | 완료 — [EV-058](../../05_verification/verification_evidence.md#ev-058). 발견 ⑨(완료 장부 전체 sweep) 조치 `a7fd4e1`. smoke P3 20초 logger 969 MB/s(P0 1,023 MB/s), 판독 일치 |
| S3 | 정식 측정(예열10초·120초×3, off/on 각3회)·계측 비용≤5%. 2026-11-08 Human: 장시간 기준(S·0.6×S 정상 부하·무제한 효율 ≥0.8), P4·P5 이월, P1 대표 3조합 | 진행 — 도구 완료(2026-11-09 축소 기준 반영), 발견 ⑩ 조치, ⑪ 비차단(T5), 1회차 [EV-064](../../05_verification/verification_evidence.md#ev-064)·후속 [EV-065](../../05_verification/verification_evidence.md#ev-065) P3 효율 미판정·T5 미확정([EV-059](../../05_verification/verification_evidence.md#ev-059)·[EV-060](../../05_verification/verification_evidence.md#ev-060)·[EV-061](../../05_verification/verification_evidence.md#ev-061)) |
| RT | 합성 generic 측정 뒤 TASK-022 요청 규칙으로 RT 준비/재시작 요청 | S3 뒤 |

S1 구현 위치는 UPS harness 선례를 따른다: [support/perf](../../../crates/dot-logger/examples/support/perf/mod.rs)·[perf_baseline](../../../crates/dot-logger/examples/perf_baseline.rs)·[TC-012 시험](../../../crates/dot-logger/tests/tc_012_performance.rs). 제품 코드 변경은 없다.

- **판정 규칙(첫 실패 조건):** 처리량 분모는 최종 file+directory sync 완료 시각이다. RAM 수락만 있으면 `NoFinalSync`, admission 중단은 `Incomplete`로 거절한다. 효율 비율은 같은 workload의 P0 run에 대해서만 계산하며 `MissingBaseline`·`BaselineNotP0`·`WorkloadMismatch`·`PayloadMismatch`를 거절한다. 추가 deadline miss는 off/on 각 3회·CPU/주기/기한/작업량/커널/release 수가 같을 때만 계산한다.
- **P0 writer:** 16B 헤더(seq·len)와 고정 fill payload, 1MiB 블록, 1GiB 회전(create_new), 1초 flush+fdatasync, 최종 sync_all과 디렉터리 sync. 시작·회전 때 free가 reserve+file 한도보다 작으면 admission을 멈추고 이미 받은 것은 sync한다. paced/unpaced, writer별 CPU 고정, 파일별 등차 seq prefix·길이·내용 read-back oracle, 소유 파일만 정리.
- **제어 루프:** 단조 시계 절대 release, 기한이 지난 release는 skipped이자 miss. wake/response는 2의 거듭제곱 µs histogram과 정확한 max, 다른 CPU 관측 수를 기록한다. 보정은 powersave governor 아래 busy-loop 보정이 2.15M~4.87M으로 흔들려, 실제 10ms 주기 안의 중앙값 보정(±5%)으로 바꿨다. governor는 기록만 하며 변경하지 않는다.
- **smoke(측정 아님):** logger off control/safety 각 500 release miss0. P0 실제 쓰기 5초 5,033,164,800B read-back 일치, 수락 5.35초·durable 20.69초(243MB/s) — RAM 수락 분모였다면 약4배 과대. 대용량 삭제 직후라 성능 수치로 쓰지 않는다.
- **환경:** `/data/build` 여유 부족(68.3GB)은 Human 결정으로 build cache를 정리해 472.9GB로 회복했다. 측정 run 전에 다시 cap+reserve를 검사한다.

2026-11-07 독립 리뷰: [REV-052](../../05_verification/reviews/REV-052_SYNC_BOUNDARY_PERF_REVIEW.md) Open(Conditional, P1 2·P2 1). 공개 L4 sync coverage·부분 scope watermark와 P0/logger 측정 구간을 후속 개발 세션에서 조치·검증한 뒤 S2c/S3로 진행한다. EV-054·055의 로컬 구현·과거 실행 결과와 정식 측정 NotRun은 유지한다.

2026-11-08 #001 조치: RV-128·129(공개 L4 sync 경계) `97e2b12`, RV-130(측정 구간) `f8f2859` — [EV-056](../../05_verification/verification_evidence.md#ev-056). **판정 규칙 추가:** 부하와 control/safety는 공통 `Gate` 시작 시각부터 계시하고(setup 제외), `LoadWindow`가 skew 0·control 구간 99% 이상 중첩일 때만 off/on 비교에 쓴다(아니면 `NotConcurrent`). 최종 내구화 tail은 부하 구간과 분리해 보고한다. S3의 예열 10초 분리는 S3 구현에서 이 공통 시작 기준으로 둔다(아직 미구현).

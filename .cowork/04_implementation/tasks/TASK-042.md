# TASK-042 — `try_log` 지연 분포 측정과 README 공개

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-011 |
| Story | US-001·004 |
| 요구사항 | NFR-001 NFR-002 |
| 설계 | [MS-011](../milestones/MS-011.md) · [API §18](../../03_design_artifacts/interface_contract.md#13-상세-api-설계안) · [TASK-022 local-pc-v1](TASK-022.md) |
| ADR | ADR-011(현재 PC 실측·RT 부팅 절차) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-23 |
| 다음 액션 | S1 하네스(S0 D1=C·D2=A 2026-11-23, D3 대기). 순서: TASK-024 S2·TASK-039 뒤, TASK-041(v0.1.1) 전 |
| 의존 | TASK-024 S2(같은 PC·RT 측정 창 겹치지 않게), TASK-039와 하네스 공유 |
| 예상 소요 | 1.5~2.5 인일 + 실행(각 10~15분 이하) |

## 배경

RT 로거는 호출 한 번의 지연 분포로 비교된다(NanoLog·Quill·LTTng). dot-logger의 공개 문서에는 이 숫자가 없다. 내부 근거는 [EV-088](../../05_verification/verification_evidence.md#ev-088) 하나다 — facade `try_log` 256 B p50 159 ns·p99 191 ns(RT 커널, CPU 2, `powersave`, 따뜻한 반복 루프, 실행별 분위수의 중앙값). 최대값, 캐시가 식은 호출, 거절 경로, 제어 루프 지터에 주는 영향은 측정한 적이 없다. 24h 시험(EV-120)의 「제어 주기 놓침 0」은 10 ms 주기 응답이 기한 안이었다는 증거이지 호출 비용 분포가 아니다.

## 측정 항목(초안)

| 행 | 내용 | 보고 |
|---|---|---|
| H | 따뜻한 반복: RT FIFO·고정 CPU·mlock, 크기 16·64·256 B, 호출 10⁶~10⁷회 | p50·p90·p99·p99.9·p99.99·최대, 시계 대조 비용 별도 |
| C1 | 식은 캐시: 호출마다 LLC 이상 크기 버퍼를 훑은 뒤 1회 호출 | 같은 분위수·최대 |
| C2 | 드문 호출: 1 kHz 루프에서 다른 일을 한 뒤 1회(현실적인 RT 글줄 빈도) | 같은 분위수·최대 |
| R | 거절 경로(버퍼 가득, `Capacity`)와 레벨 필터 skip | p50·p99·최대 |
| J | 제어 루프 지터: local-pc-v1 10 ms 루프(및 1 kHz)에서 기록 없음 / 주기당 50건 / 50건 + 파일 IO·원격 부하 | 깨어남 지연·응답 시간 분포, 최대, 놓침 |
| M | 다중 생산자: 다른 코어의 writer 1·4·8개가 함께 기록할 때 H | p99·최대 |

모두 PREEMPT_RT 커널(현재 기본 부팅이 아니면 ADR-011 once/back 절차)과 generic 커널 대조를 둔다. CPU·커널·governor·SMT·IRQ 배치·전원 상태를 결과와 함께 기록한다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S0 | 결정 | 아래 D1~D3 Human 결정 | D1·D2 결정(2026-11-23), D3 대기 |
| S1 | 하네스 | `examples/support/perf`의 `call_cost`·`LatencyHistogram` 확장(최대·꼬리 분위수·식은 캐시·지터 행), JSON 원시 결과 | Planned |
| S2 | 실행 | 행 H~M, 각 10~15분 이하, 반복 3회, 원시 결과를 `05_verification/perf_runs/`에 | Planned |
| S3 | 공개 | README 「Verification」에 조건·방법이 붙은 표(비교 대상 수치는 출처·환경이 다르면 같은 표에 섞지 않음), EV, 추적표 NFR-001 | Planned |

## 결정(Human, 2026-11-23 「D1은 둘 다, D2는 A」)

| ID | 질문 | 선택지 | 권장 |
|---|---|---|---|
| D1 | CPU governor | A. 현재 `powersave` 그대로 측정하고 명시 / B. 측정 동안 `performance`(sudo, 측정 뒤 원복) / C. 둘 다 | **C** — 둘 다. `performance` 전환·원복은 sudo가 필요해 실행 때 Human이 하거나 그 시점에 방법을 정한다 |
| D2 | 비교 대상 동일 PC 측정 | A. 하지 않음(자체 숫자와 방법만 공개) / B. Quill·NanoLog 공개 벤치마크를 같은 PC에서 돌려 나란히 | **A** — 자체 숫자와 방법만 공개. B는 공개 뒤 요청 시 |
| D3 | README 표 범위 | A. H·C1·J 요약 3~5행 / B. 전 행 | 대기 — 권장 A(전 행은 EV·perf_runs 링크) |

## 완료 기준

- [ ] 조건(하드웨어·커널·governor·크기·반복)과 방법이 함께 적힌 분포 표가 README에 있다.
- [ ] 최대값과 식은 캐시 행이 포함된다.
- [ ] 제어 루프 지터에서 기록 유무 차이가 분포로 보고된다.
- [ ] 원시 결과·EV·재현 명령이 있다.

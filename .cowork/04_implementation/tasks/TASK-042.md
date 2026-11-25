# TASK-042 — `try_log` 지연 분포 측정과 README 공개

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-011 |
| Story | US-001·004 |
| 요구사항 | NFR-001 NFR-002 |
| 설계 | [MS-011](../milestones/MS-011.md) · [API §18](../../03_design_artifacts/interface_contract.md#13-상세-api-설계안) · [TASK-022 local-pc-v1](TASK-022.md) |
| ADR | ADR-011(현재 PC 실측·RT 부팅 절차) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-25 |
| 다음 액션 | S2 ③ generic 커널 측정(Human 재부팅 뒤, 아래 「③ 실행 절차」) → S3 README·EV. 순서: TASK-041(v0.1.1) 전 |
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
| S0 | 결정 | 아래 D1~D3 Human 결정 | Done — D1·D2 2026-11-23, D3 2026-11-25 |
| S1 | 하네스 | `examples/support/perf`의 `call_cost`·`LatencyHistogram` 확장(최대·꼬리 분위수·식은 캐시·지터 행), JSON 원시 결과 | Done — `b94964b`·`0012ef8`(아래) |
| S2 | 실행 | 행 H~M, 각 10~15분 이하, 반복 3회, 원시 결과를 `05_verification/perf_runs/`에 | In Progress — ①·② Done(2026-11-25), ③ generic 대기 |
| S3 | 공개 | README 「Verification」에 조건·방법이 붙은 표(비교 대상 수치는 출처·환경이 다르면 같은 표에 섞지 않음), EV, 추적표 NFR-001 | Planned |

## 결정(Human, 2026-11-23 「D1은 둘 다, D2는 A」, 2026-11-25 D3 「A. 요약 3~5행」)

| ID | 질문 | 선택지 | 권장 |
|---|---|---|---|
| D1 | CPU governor | A. 현재 `powersave` 그대로 측정하고 명시 / B. 측정 동안 `performance`(sudo, 측정 뒤 원복) / C. 둘 다 | **C** — 둘 다. `performance` 전환·원복은 sudo가 필요해 실행 때 Human이 하거나 그 시점에 방법을 정한다 |
| D2 | 비교 대상 동일 PC 측정 | A. 하지 않음(자체 숫자와 방법만 공개) / B. Quill·NanoLog 공개 벤치마크를 같은 PC에서 돌려 나란히 | **A** — 자체 숫자와 방법만 공개. B는 공개 뒤 요청 시 |
| D3 | README 표 범위 | A. H·C1·J 요약 3~5행 / B. 전 행 | **A** — README는 H·C1·J 요약 3~5행, 전 행은 EV·perf_runs 링크(2026-11-25 Human) |

## 완료 기준

- [ ] 조건(하드웨어·커널·governor·크기·반복)과 방법이 함께 적힌 분포 표가 README에 있다.
- [ ] 최대값과 식은 캐시 행이 포함된다.
- [ ] 제어 루프 지터에서 기록 유무 차이가 분포로 보고된다.
- [ ] 원시 결과·EV·재현 명령이 있다.

## S1 하네스 — 2026-11-25

- `perf_baseline --latency <tmpfs_parent> <행> …`(`examples/support/perf/latency.rs`): 호출 하나를 시계 두 번으로 감싸 재고, 같은 스레드에서 시계 한 쌍 비용을 따로 잰다. 행마다 JSON 한 줄(p50·p90·p99·p99.9·p99.99·정확한 최대).
  - H `warm`: 1,024회 묶음 + 500 µs 비움(worker가 비우는 사이 거절은 세기만 하고 재지 않음). C1 `cold`: 호출마다 LLC의 2배(이 PC 32 MiB)를 훑은 뒤 1회. C2 `sparse`: 1 kHz 주기마다 256 KiB를 건드린 뒤 1회. R `reject`: 레벨로 걸러지는 호출(`Skipped`)과 64 슬롯을 비우지 않고 채워 생긴 `Capacity` 거절. J `jitter`: local-pc-v1 10 ms(일 500 µs)·1 kHz(일 50 µs) 루프에서 주기당 0·50건, 50건 + IO 부하(64 KiB × 100건/s를 SSD 출력에). M `multi`: writer 1·4·8개를 서로 다른 CPU(2·3·4·5·6·7·0·1)에서 동시에 H.
  - 기록은 tmpfs 링(1 GiB)에만 쓰고, 쓰는 프로세스에서 tmpfs·여유(링의 2배)를 확인한다. 시험 `tc_012` 2개(비 tmpfs·공간 부족 거부, 행마다 요청한 호출 수·결과 분류·정상 종료).
  - 분위수 히스토그램 정밀도를 1/8 → 1/32로 올렸다(Red→Green). p50 약 160 ns에서 20 ns 오차는 공개 숫자로 크다.
  - 하네스는 구현 뒤 시험을 붙였다(측정 도구라 TDD 순서 예외).
- `scripts/run-latency.sh`: 커널(`RT_EXPECT`)·governor(`GOVERNOR_EXPECT`)가 맞지 않으면 멈추고, 머리줄에 커널·cmdline·governor·boost·SMT·CPU·git을 남긴 뒤 행을 `reps`회 돈다. 1회 약 10분, IO 행 SSD 쓰기 약 0.8 GB.
- RT 시운전(1회, 지터 5 s, 207 s): 18개 행 성공. H 256 B p50 251 ns·p99 543 ns, C1 p50 863 ns, R `Capacity` p50 91 ns·`Skipped` 101 ns, M 4·8개 p50 375 ns, 10 ms 루프 깨어남 p50 43 µs(1 kHz는 5 µs — `powersave`의 깊은 절전 상태 추정).
- **관찰(제품 개선 후보):** 레벨로 걸러지는 호출도 약 100 ns(시계 한 쌍 20 ns 포함)가 든다. `try_log`가 레벨을 보기 전에 `SystemTime::now()`와 `Instant` 경과를 먼저 읽는다(`writer.rs` `try_log`). 레벨 확인을 앞당기면 거의 0이 될 수 있다 — S2 결과와 함께 판단.

## S2 결과 ①·② — 2026-11-25 (RT 커널)

이 PC(Ryzen 7 H 255, `7.0.0-44-realtime`, SMT on, boost on), 측정 스레드 FIFO 70·CPU 2·mlock, 기록은 tmpfs 링 1 GiB, 3회 반복. 표는 3회 중앙값(최대는 3회 중 최대), 단위 ns. 히스토그램 오차 ≤ 1/32.

| 행 | ① `powersave`(EPP `balance_performance`) p50 / p99 / p99.99 / 최대 | ② `performance`(EPP `performance`) p50 / p99 / p99.99 / 최대 |
|---|---|---|
| H 16 B | 295 / 527 / 5,631 / 43,851 | 223 / 367 / 3,903 / 41,857 |
| H 64 B | 287 / 527 / 5,631 / 41,246 | 251 / 367 / 3,967 / 42,338 |
| H 256 B | 287 / 543 / 5,631 / 53,950 | 251 / 375 / 4,031 / 49,361 |
| C1 식은 캐시 256 B | 751 / 1,695 / 7,039 / 17,201 | 767 / 1,407 / 6,655 / 19,466 |
| C2 드문 호출 1 kHz | 863 / 1,119 / 5,375 / 17,993 | 543 / 783 / 6,655 / 18,654 |
| R `Capacity` 거절 | 91 / 111 / 815 / 4,629 | 91 / 101 / 847 / 5,029 |
| R 레벨 skip | 101 / 101 / 1,247 / 14,206 | 101 / 101 / 1,183 / 4,679 |
| M writer 1 / 4 / 8개 p50 | 295 / 375 / 383 | 211 / 263 / 263 |
| M writer 8개 p99 / 최대 | 559 / 36,217 | 391 / 38,180 |
| J 10 ms 루프 50건 묶음(IO 부하) | 14,847 / 23,551 / 40,274 / 42,107 | 12,031 / 14,591 / 23,744 / 26,318 |
| J 10 ms 깨어남 0건 / 50건 p50 | 44,031 / 45,055 | 23,039 / 23,039 |
| J 1 kHz 깨어남 0건 / 50건 p50 | 4,607 / 4,735 | 3,007 / 3,135 |

- 45개 행 × 2 모두 성공(종료 보고 success), 제어 루프 놓침 0(IO 부하 포함).
- `performance`에서 p50 약 10%, p99·p99.99 약 30% 감소. 최대(40~50 µs)는 governor와 무관 — 인터럽트·SMT 등 시스템 요인 추정.
- 기록 유무로 깨어남 분포가 변하지 않는다(0건 = 50건). 10 ms 루프 깨어남(23~45 µs)이 1 kHz(3~5 µs)보다 긴 것은 긴 idle 동안의 깊은 C-state로 보인다(governor가 아니라 cpuidle 영역).
- J의 응답 시간(일 500 µs → p50 0.7~1.5 ms)은 보정한 합성 일의 주파수 변동이 지배해 기록 영향 판단에 쓰지 않는다. 기록 영향은 「50건 묶음」 분포로 본다.
- 머리줄 `dirty: true`는 추적하지 않는 `scripts/__pycache__/` 때문이다(측정 코드는 `0012ef8` 그대로). `.gitignore`에 추가했다. ①·② 머리줄에는 EPP가 없어 위 표에 손으로 적었다(②는 governor `performance` 전환 때 드라이버가 EPP도 `performance`로 바꿨고, 원복은 governor와 EPP를 따로 했다). 스크립트 머리줄에 EPP를 추가했다.
- 원시 결과: [perf_runs/task042-2026-11-25](../../05_verification/perf_runs/task042-2026-11-25/) `rt-powersave.jsonl`·`rt-performance.jsonl`. IO 부하 출력 4.5 GB는 삭제.

### ③ 실행 절차(generic 커널, Human 재부팅 뒤)

1. Human이 generic 커널로 부팅한다(ADR-011 절차, `scripts/rt-boot.sh` 참조). AI는 `uname -r`과 `/sys/kernel/realtime` 없음으로 확인한다.
2. `powersave`(EPP `balance_performance`) 상태에서: `CARGO_TARGET_DIR=/data/build/target RT_EXPECT=generic GOVERNOR_EXPECT=powersave scripts/run-latency.sh /dev/shm <SSD>/load <out>/generic-powersave 3 60`(약 30분, SSD 약 2.4 GB). 빌드는 측정 전에 `cargo build --release -p dot-logger --example perf_baseline`.
3. Human이 `sudo cpupower frequency-set -g performance` → AI가 governor·EPP 확인 → `GOVERNOR_EXPECT=performance …/generic-performance`(약 30분).
4. Human이 `sudo cpupower frequency-set -g powersave`와 `echo balance_performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference`로 원복 → AI 확인.
5. 결과를 perf_runs에 복사, 부하 출력 삭제, RT 커널로 되돌릴지는 Human 결정.

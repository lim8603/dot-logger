# TASK-042 — `try_log` 지연 분포 측정과 README 공개

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-011 |
| Story | US-001·004 |
| 요구사항 | NFR-001 NFR-002 |
| 설계 | [MS-011](../milestones/MS-011.md) · [API §18](../../03_design_artifacts/interface_contract.md#13-상세-api-설계안) · [TASK-022 local-pc-v1](TASK-022.md) |
| ADR | ADR-011(현재 PC 실측·RT 부팅 절차) |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-26 |
| 다음 액션 | 없음 — Done(2026-11-26, [EV-129](../../05_verification/verification_evidence.md#ev-129)·[EV-130](../../05_verification/verification_evidence.md#ev-130)). 다음은 TASK-041(v0.1.1). 레벨 skip 비용은 기능 논의 안건 |
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
| S2 | 실행 | 행 H~M, 각 10~15분 이하, 반복 3회, 원시 결과를 `05_verification/perf_runs/`에 | Done — ①·② RT(2026-11-25), ③-a generic·`powersave`(2026-11-25), ③-b generic·`performance`(2026-11-26) |
| S3 | 공개 | README 「Verification」에 조건·방법이 붙은 표(비교 대상 수치는 출처·환경이 다르면 같은 표에 섞지 않음), EV, 추적표 NFR-001 | Done — 2026-11-26, 아래 |

## 결정(Human, 2026-11-23 「D1은 둘 다, D2는 A」, 2026-11-25 D3 「A. 요약 3~5행」)

| ID | 질문 | 선택지 | 권장 |
|---|---|---|---|
| D1 | CPU governor | A. 현재 `powersave` 그대로 측정하고 명시 / B. 측정 동안 `performance`(sudo, 측정 뒤 원복) / C. 둘 다 | **C** — 둘 다. `performance` 전환·원복은 sudo가 필요해 실행 때 Human이 하거나 그 시점에 방법을 정한다 |
| D2 | 비교 대상 동일 PC 측정 | A. 하지 않음(자체 숫자와 방법만 공개) / B. Quill·NanoLog 공개 벤치마크를 같은 PC에서 돌려 나란히 | **A** — 자체 숫자와 방법만 공개. B는 공개 뒤 요청 시 |
| D3 | README 표 범위 | A. H·C1·J 요약 3~5행 / B. 전 행 | **A** — README는 H·C1·J 요약 3~5행, 전 행은 EV·perf_runs 링크(2026-11-25 Human) |

## 완료 기준

- [x] 조건(하드웨어·커널·governor·크기·반복)과 방법이 함께 적힌 분포 표가 README에 있다.
- [x] 최대값과 식은 캐시 행이 포함된다.
- [x] 제어 루프 지터에서 기록 유무 차이가 분포로 보고된다(README는 결론 문장, 0건·50건 분포는 EV-129).
- [x] 원시 결과·EV·재현 명령이 있다([EV-129](../../05_verification/verification_evidence.md#ev-129)).

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

1. Human이 `sudo scripts/rt-boot.sh generic`으로 다음 한 번만 generic 커널로 재부팅한다(`5f06969` 이후, 기본 항목은 그대로). 7.0.0-44부터 GRUB 기본 항목이 RT 커널이라 그냥 재부팅하면 RT로 뜬다. AI는 `scripts/rt-boot.sh status`·`uname -r`·`/sys/kernel/realtime` 없음으로 확인한다.
2. `powersave`(EPP `balance_performance`) 상태에서: `CARGO_TARGET_DIR=/data/build/target RT_EXPECT=generic GOVERNOR_EXPECT=powersave scripts/run-latency.sh /dev/shm <SSD>/load <out>/generic-powersave 3 60`(약 30분, SSD 약 2.4 GB). 빌드는 측정 전에 `cargo build --release -p dot-logger --example perf_baseline`.
3. Human이 `sudo cpupower frequency-set -g performance` → AI가 governor·EPP 확인 → `GOVERNOR_EXPECT=performance …/generic-performance`(약 30분).
4. Human이 `sudo cpupower frequency-set -g powersave`와 `echo balance_performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference`로 원복 → AI 확인.
5. 결과를 perf_runs에 복사, 부하 출력 삭제. 다음 재부팅은 기본 항목(RT 커널)으로 돌아간다.

## S2 결과 ③-a — 2026-11-25 (generic 커널, `powersave`)

`7.0.0-44-generic`(`sudo scripts/rt-boot.sh generic`으로 1회 부팅), governor `powersave`·EPP `balance_performance`, 그 밖의 조건은 ①과 같다. `826fb47`, `dirty: false`. 45개 행 성공, 놓침 0.

| 행 | ① RT p50 / p99 / p99.99 / 최대 | ③-a generic p50 / p99 / p99.99 / 최대 |
|---|---|---|
| H 256 B | 287 / 543 / 5,631 / 53,950 | 327 / 527 / 5,375 / 154,576 |
| C1 식은 캐시 | 751 / 1,695 / 7,039 / 17,201 | 735 / 1,503 / 9,983 / 41,145 |
| C2 드문 호출 | 863 / 1,119 / 5,375 / 17,993 | 863 / 1,087 / 17,407 / 19,846 |
| M writer 8개 | 383 / 559 / 6,015 / 36,217 | 375 / 543 / 5,119 / 42,789 |
| J 10 ms 50건 묶음(IO) | 14,847 / 23,551 / 40,274 / 42,107 | 14,847 / 33,791 / 100,475 / 108,871 |
| J 10 ms 깨어남(IO) | 45,055 / 135,167 / 138,093 / 139,605 | 46,079 / 135,167 / 184,930 / 217,256 |
| J 1 kHz 깨어남(0건) | 4,607 / 6,271 / 27,135 / 88,466 | 4,479 / 6,783 / 34,815 / 209,699 |

- p50·p99는 두 커널이 거의 같고, 차이는 꼬리·최대에서 난다(generic 최대 2~3배). PREEMPT_RT의 이점이 최악값에 있다는 것을 이 PC에서 확인.
- 원시 결과 `perf_runs/task042-2026-11-25/generic-powersave.jsonl`. IO 부하 출력은 삭제.
- ③-b는 2026-11-26 같은 절차로 진행했다(아래).

## S2 결과 ③-b — 2026-11-26 (generic 커널, `performance`)

`7.0.0-44-generic`(`sudo scripts/rt-boot.sh generic`으로 1회 부팅, `status`에서 `realtime=absent`·`next_entry` 비어 있음), governor·EPP 16개 모두 `performance`(머리줄 기록), 그 밖의 조건은 ①과 같다. `cc52ff2`, `dirty: false`. 17:30:48~17:57:14, 45개 행 성공, 놓침 0(IO 부하 포함). 집계는 ①~③-a와 같은 방식(3회 중앙값, 최대는 3회 중 최대)이고, 같은 집계로 ①·②·③-a 표 숫자가 그대로 재현됨을 확인했다.

| 행 | ② RT·`performance` p50 / p99 / p99.99 / 최대 | ③-b generic·`performance` p50 / p99 / p99.99 / 최대 |
|---|---|---|
| H 16 B | 223 / 367 / 3,903 / 41,857 | 231 / 351 / 3,519 / 109,401 |
| H 256 B | 251 / 375 / 4,031 / 49,361 | 263 / 375 / 3,711 / 122,739 |
| C1 식은 캐시 | 767 / 1,407 / 6,655 / 19,466 | 831 / 1,311 / 6,271 / 8,476 |
| C2 드문 호출 | 543 / 783 / 6,655 / 18,654 | 511 / 735 / 15,871 / 19,566 |
| R `Capacity` 거절 | 91 / 101 / 847 / 5,029 | 91 / 111 / 703 / 79,636 |
| M writer 8개 | 263 / 383 / 4,159 / 38,180 | 263 / 383 / 3,487 / 136,099 |
| J 10 ms 50건 묶음(IO) | 12,031 / 14,591 / 23,744 / 26,318 | 11,775 / 14,079 / 29,945 / 30,238 |
| J 10 ms 깨어남(IO) | 23,039 / 112,639 / 116,732 / 117,332 | 23,039 / 112,639 / 116,669 / 195,844 |
| J 1 kHz 깨어남(0건) | 3,007 / 4,479 / 32,767 / 63,070 | 2,943 / 4,479 / 35,839 / 91,174 |
| J 1 kHz 깨어남(IO) | 3,135 / 5,503 / 28,671 / 71,709 | 2,943 / 4,735 / 28,159 / 728,929 |

- `performance`에서도 ③-a와 같은 모양이다 — p50·p99·p99.99는 두 커널이 거의 같고(일부 행은 generic이 조금 낮음), 차이는 최대에서 난다. 호출 최대는 RT 약 40~50 µs, generic 약 80~140 µs(2~3배, R 거절은 5 µs 대 80 µs).
- 가장 큰 차이는 1 kHz 루프 깨어남(IO 부하) 최대 729 µs — 주기 1 ms·일 50 µs라 놓침은 없었지만 RT(72 µs)의 약 10배다. 3회 중 2회째의 단일 사건이다(나머지 42·91 µs, p99.99는 세 회 모두 약 28 µs).
- 같은 generic 커널에서 `powersave`(③-a) → `performance`로 J 10 ms 50건 묶음(IO)의 p99.99·최대가 100 µs대에서 30 µs로 줄었다. 반면 호출 최대는 governor와 무관하게 100 µs대로 남는다 — 호출 최대를 정하는 것은 커널(선점)이고 governor가 아니다.
- 측정 중 외부 접속 없음: sshd 기록이 측정 구간에 없다(192.168.0.135 접속은 측정 전 17:28·17:29). 유선 NIC(`enp4s0`) 인터럽트는 모두 CPU 15라 측정 CPU 2·SMT 짝 10과 겹치지 않는다. 큰 최대는 1·2·3회 모두에 고르게 나와 단일 사건이 아니다(Human 질문으로 확인).
- 원시 결과 `perf_runs/task042-2026-11-25/generic-performance.jsonl`(같은 디렉터리에 4개 조건). IO 부하 출력 2.3 GB(`/data/db/task042-load`)는 삭제.
- 원복: 측정 뒤 Human이 governor `powersave`·EPP `balance_performance`로 되돌렸고 AI가 16개 모두 확인했다(「③ 실행 절차」 4단계). 다음 재부팅은 기본 항목(RT 커널).

## S3 공개 — 2026-11-26

- README 「Verification」에 요약 5행(D3: H·C1·J), 두 커널 열, `performance` 기준. 조건(CPU·SMT·boost·governor·FIFO 70·고정 CPU·mlock·tmpfs 링)·집계(3회 중앙값, 최대는 3회 중 최대)·시계 두 번 포함을 표 위에 적었다. 비교 대상 수치는 넣지 않았다(D2).
- 행 선택: J는 처음에 SSD 부하가 있는 10 ms 묶음 행을 골랐으나, 부하 없는 같은 행의 꼬리가 더 크다(RT p99.99 102 µs 대 24 µs)는 것을 보고 유리한 행만 싣지 않도록 바꿨다 — 1 kHz(60,000주기)·10 ms(6,000주기) 묶음을 부하 없이 싣고, 부하의 영향(1 kHz p99 최대 +20 %, 최대 약 145 µs)은 문장으로, 깨어남 행은 50건 + SSD 부하로 실었다.
- 문장으로 적은 것: 단일 호출의 커널 차이는 최대뿐(H에서 generic 6개 중 5개가 85 µs 초과, RT 최대 54 µs), 약 1,000주기에 한 번 50건 묶음이 75~115 µs(두 커널, 원인 미확인), 기록은 깨어남 지연을 바꾸지 않음, `powersave`의 p50 15~45 %·p99 약 40 % 증가. 수치는 집계 스크립트 결과와 대조했다.
- 전 행·4개 조건·관찰·재현 명령은 [EV-129](../../05_verification/verification_evidence.md#ev-129). 추적표 NFR-001에 MS-011 행을 더했다.
- 기능 논의 안건으로 올린 것: ① ~~50건 묶음의 드문 75~115 µs 꼬리 원인~~ → 아래 원인 분석으로 해소 ② 레벨 skip 호출의 시계 읽기 약 100 ns.

## S3 후속 — 50건 묶음 꼬리 원인 분석(2026-11-26, Human 「원인 분석 먼저」)

- 하네스에 진단을 더했다: `DOT_LOGGER_PERF_SLOW_US`를 주면 J 행이 호출마다 시간을 재고 느린 묶음의 주기·seq·가장 느린 호출·깨어남·계산 시간을 미리 할당한 배열에 남긴다(시험 보강, 기본 동작은 그대로).
- 결과([EV-130](../../05_verification/verification_evidence.md#ev-130)): 꼬리는 dot-logger 호출 비용이 아니다. AI 세션이 대기 중(백그라운드 실행 — 원래 측정과 같음)일 때만 약 600 ms마다 약 0.5 ms 동안 측정 코어가 약 9배 느려지고(같은 주기의 50 µs 연산이 약 470 µs), 그 주기에 걸린 묶음이 75~115 µs가 된다. 기록 0건 행의 응답 p99.9도 475~507 µs로 같은 사건을 보인다. 포그라운드 10회에서는 사건이 없고 묶음 p99.9 31~44 µs.
- 기각: 압축(kcompactd)·열·SMT 짝·캐시 훑기·앞선 행의 상태·낮은 사용률. 남은 기전 추정은 SoC 펌웨어·전원 관리(루트 실험 필요, Human 판단).
- README 문장을 「원인 미확인」에서 위 결과로 고쳤다. 표 숫자는 그대로(측정한 그대로이고, 문장이 해석을 붙인다).
- 측정 절차: 이 PC 지연 측정은 실행 방식에 따라 꼬리가 달라지므로 조건에 실행 방식을 적고, 기록 0건 행의 응답으로 감속 사건을 함께 본다.

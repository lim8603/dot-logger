# TASK-042 50건 묶음 꼬리 진단 — 2026-11-26

EV-129 관찰(1 kHz 50건 묶음이 약 1,000주기에 한 번 75~115 µs)의 원인 분석 원시 결과. 모두 `7.0.0-44-generic`, governor `powersave`(EPP `balance_performance`), 측정 스레드 FIFO 70·CPU 2·mlock, `/dev/shm` 링 1 GiB, `jitter 256 1000 <s> 50 50`(1 kHz, 일 50 µs, 주기당 50건). `slow` 배열은 `DOT_LOGGER_PERF_SLOW_US` 기준을 넘은 묶음(주기·seq·묶음·가장 느린 호출과 위치·2 µs 넘는 호출 수, `wk1`·`idle1`은 깨어남·계산 시간 포함). 판정은 [EV-130](../../verification_evidence.md#ev-130).

| 파일 | 실행 방식 | 조건 |
|---|---|---|
| `a1`·`a2` | 포그라운드 | 진단 켬, 30 s·60 s |
| `a3`·`c1-old` | 포그라운드 | 진단 끔, 60 s(`c1-old`는 `cc52ff2` 바이너리) |
| `b2` | 포그라운드 | 10 ms + SSD 부하 행·1 kHz 0건 행 바로 뒤 |
| `e1-sib`·`e1-nosib` | 포그라운드 | 프로세스를 CPU 2·10(SMT 짝)에만 / CPU 10 제외 |
| `h1` | 포그라운드 | busy loop 8개로 Tctl 77 °C까지 데운 직후 |
| `sw1`·`sw2` | 포그라운드 | 600 ms마다 32 MiB(CPU 6) / 4 MiB(CPU 2) 훑기 |
| `full-rep` | 백그라운드 | `run-latency.sh` 1회차 전체(진단 켬) |
| `bg1`·`bg2`·`wk1` | 백그라운드 | 단독 30 s |
| `aw1` | 백그라운드 | CPU 6 busy loop |
| `idle1` | 백그라운드 | CPU 2에 SCHED_IDLE busy loop |

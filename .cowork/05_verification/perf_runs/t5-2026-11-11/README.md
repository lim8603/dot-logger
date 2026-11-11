# T5 조사 원자료 — 2026-11-11 (tmpfs, SSD 쓰기 0)

- 바이너리: `/data/build/target-diag{,2,3,4}` — 모두 `RESERVE = 0`·생산자/worker CPU 환경변수(`T5_PRODUCER_CPUS`·`T5_WORKER_CPUS`, diag만).
  - diag2: + chunk/data section CRC off. diag3: + `poll_shutdown`이 drain 중 totals 집계 생략. diag4: diag3 + CRC off. 소스는 각 빌드 직후 원복.
- 부하: `--p1-mix-max /dev/shm/... 12 GiB 256 100000 8MiB 120 <streams> 16 8 32 128 4096 <raw> <mem> 0`, P0는 `--p0-mix-max` 같은 형태.
- `cpu.sh`: 실행 중 0.2 s마다 `/proc/<pid>/task/*/stat` 표본(tid·이름·상태·user/sys tick). `hot.py`·`window.py`로 IO/host 스레드별 집계.
- `runs.jsonl` 실행 요약, `threads.txt` 스레드별 비용, `gdb-host-8streams.txt` drain 구간 host 스택, `memcpy-bw.txt` 8 MiB memcpy 대역폭.
- diag5 = ⑫ 수정 소스 기준, diag6 = + mcap CRC 1회 패치(`mcap-single-crc.diff`), diag7 = + encode_record 복사 1회 추가. `runs-ev067.jsonl`.

# TASK-024 S2 장애 주입 — 2026-11-23 23:17~23:43 KST

명령(저장소 루트, release 빌드 뒤):

    python3 -B scripts/soak-faults.py f1 f2 f3a f3b f4 f5 --out <dir>

기본값: 장애 전·후 60 s, F1 1 MiB/s 180 s, F2a 8 s 정지 3회·F2b 120 s, F3 120 s·tmpfs 512 MiB(F3b `nr_inodes=64`), F4 15 s, F5 지연 7·13·19·23·29·37 s, 표본 200 ms, RT(`DOT_LOGGER_PERF_RT=fifo`).
여기에는 시나리오별 `result.json`(판정·단계 시각·종료 이벤트·inspect)과 요약만 있다. 전체 soak·구독자 JSON lines(6.3 MB)는 이 PC의 `~/backup/task024-s2-2026-11-23-raw.tar.zst`에 보관했다.
`rootfree.txt`: 실행 동안 루트 파일 시스템 여유(2 s 간격) 최소·최대 — 호스트 디스크가 채움 대상이 아니었음을 보인다(F1의 의도된 링 쓰기 약 450 MiB만).

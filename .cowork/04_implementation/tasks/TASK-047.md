# TASK-047 — 장시간 DB 단절 시험

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-012](../milestones/MS-012.md) |
| Story | US-002·003 |
| 요구사항 | FR-004 NFR-003 NFR-005 — TC-008·TC-012 |
| 설계 | [TASK-024 S2 F4-DB·D2](TASK-024.md) · [`adapters::influxdb`](../../../crates/dot-logger/src/adapters/influxdb/mod.rs) rustdoc · `scripts/soak-faults.py` · `crates/dot-logger/examples/soak.rs` |
| ADR | ADR-011(현재 PC 실측) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-27 |
| 다음 액션 | S3 실행 중(f6a 짧은 재실행 → f6a/b/c 각 10분 장애, RT) |
| 의존 | REV-073 RV-202~204 조치(같은 하네스), TASK-045(v2 기준선) |
| 예상 소요 | 1.5~2.5 인일 + 실행 |

## 배경

TASK-024 S2에서 DB 단절(F4-DB)은 D2로 tc_008 단기 가짜 서버 시험(응답 지연·멈춤·포화·타임아웃)으로 갈음하고 미실행으로 남겼다. soak 하네스(`examples/soak.rs`, facade, 제어 256 B 5,000건/s + 프레임 64 KiB 10건/s)에는 DB 출력이 없다. facade에 DB 출력이 없으므로 DB 경로는 고급 경로(`Writer`)로 붙인다. 원격 구독 F4는 keepalive가 없어 조용한 단절을 감지하지 못했다(Fail, 기능 논의 안건). DB 쓰기는 요청마다 `request_timeout`·`ttl`이 있어 조용한 단절도 시간 초과로 드러나야 한다 — 이것을 실제 DB로 확인한다.

## 결정

| ID | 질문 | 선택지 | 권장 |
|---|---|---|---|
| D1 | DB 경로 연결 | A. soak에 일반 스레드 DB 작업자 추가: 제어 값 일부(예: 채널 2개 100 Hz)를 `Writer`로 배치 전송, facade 수집·파일과 같은 프로세스 / B. 고급 경로 `RoutedWriter`로 수집 기록 → DB 완료 보고(soak 재작성, pipeline support 재사용) / C. 원격 구독 → DB 브리지 별도 프로세스 | **A**(2026-11-27 Human 「권고대로」) — 앱이 직접 DB에 쓰는 흔한 형태이고, DB 장애가 RT 수집·파일 출력에 번지지 않는지와 `Writer` 상한·회복을 함께 본다. B는 작업량이 크고 C는 F4 원격 문제와 섞인다 |
| D2 | 장애 종류·길이 | 종류: ① `docker pause`(조용한 단절) ② `docker stop`→`start`(연결 거부·재기동) ③ `docker network disconnect`(경로 단절). 길이: A. 각 10분 장애(앞뒤 각 5분, 총 약 1 h) / B. ① 1 h + ②③ 각 10분(총 약 2 h) | **A**(2026-11-27 Human 「권고대로」) — 세 종류 각 10분 장애, 앞뒤 각 5분, 총 약 1 h. 파일 출력은 `/dev/shm`, DB는 tmpfs라 SSD 쓰기 약 0. S3 시작 직전에 다시 알림 |
| D3 | 판정 | 아래 판정 초안 | **초안대로**(2026-11-27 Human 「권고대로」) |

판정 초안:
- RT 쪽: 장애 전후와 장애 중 제어 주기 놓침 0, 수락 실패 0, 파일 출력 무손실(`verify`).
- DB 작업자: 메모리 예약 상한 유지(`Busy`로 다음 배치 거절, 무한 적체 없음), 장애 종류별 결과 분류가 계약대로(① 시간 초과 → `Unknown`, ② 연결 거부 → 실패·재시도 소진, ③ 시간 초과 또는 거부), 손실·불명 건수를 보고.
- 회복: DB가 돌아온 뒤 정해진 시간 안에 `Acknowledged` 재개. 회복 뒤 DB 조회로 확인 — `Acknowledged`로 보고된 점은 모두 존재하고, 존재하는 점은 보고된 시도(`Acknowledged`·`Unknown`)에만 속한다.
- ② 재기동 시 tmpfs 데이터가 남는지(stop/start는 컨테이너 유지)와 high-water mark 동작을 기록.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S0 | 결정 | D1~D3 Human 결정, 실행 시간·쓰기량 확정 | **Done** 2026-11-27 — D1=A·D2=A(약 1 h, SSD 쓰기 약 0)·D3=초안 |
| S1 | DB 작업자 | soak `run --influx`(feature `influxdb`): 일반 스레드 + 자체 current-thread runtime, 결과별 누적 건수·마지막 성공 시각을 표본 JSON에. 가짜 서버로 짧은 Red→Green(멈춤 → `Unknown` 집계) | **Done** 2026-11-27 — `examples/support/soak_influx.rs`(채널 2개 100 Hz 격자, 100 ms 배치, 밀린 점은 건너뛰고 집계, 배치 로그 JSONL). 가짜 서버 15 s: 204 → 150회 모두 Acknowledged·건너뜀 0 / 무응답 → 4회 Unknown(2회×2 s+0.2 s=4.2 s)·건너뜀 2,222점, 두 경우 RT 놓침 0·거절 0·종료 성공 |
| S2 | 하네스 | `soak-faults.py` f6a/b/c: v2 일회용 DB 컨테이너 생성(check-influxdb 격리 조건 재사용)·장애 주입·회복 뒤 DB 조회 대조·판정. 각 2분 장애의 짧은 실행으로 확인 | **Done** 2026-11-27 — `soak-faults.py` f6a/b/c(DB 데이터 `/dev/shm` 바인드, 고정 루프백 포트, 회복 뒤 Flux 대조)·판정 순수 함수 자가 시험 12개. 짧은 실행(장애 2분, `--no-rt`): f6b·f6c Pass, f6a는 **판정 오류**로 Fail — 장애 중 시작한 재시도를 재개된 DB가 0.045 s 뒤 확인한 배치를 「장애 중 확인」으로 셈. 「장애 중 *완료된* 배치는 Acknowledged 아님」으로 고치고 걸친 배치는 따로 기록(Red→Green), 저장 결과 재판정 Pass. 사전 확인: stop/start 뒤 데이터 유지, `network disconnect`는 즉시 연결 거부 |
| S3 | 장시간 실행 | D2 길이로 실행(1 h 초과면 시작 전 승인). 원시 결과 비공개 증거 폴더 | Planned |
| S4 | 마감 | EV, TASK-024 F4-DB "tc_008 갈음" 기록 갱신, 추적표, 자가검토 | Planned |

## 완료 기준

- [ ] f6 세 장애 판정 Pass(또는 Fail을 원인과 함께 기록하고 Human 판단).
- [ ] TASK-024 F4-DB 갱신, EV.

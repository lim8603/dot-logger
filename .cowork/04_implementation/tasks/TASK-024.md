# TASK-024 — 24시간·전원 차단·OS 교환 검증

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-001·002·003·004·005 |
| 요구사항 | FR-018 NFR-003 NFR-007 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-23 |
| 다음 액션 | S2 장애 주입(TASK-038 뒤) → S5 |
| 의존 | TASK-023 |
| 예상 소요 | 3~5 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `tests/soak/; tests/platform/; 검증 evidence`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **P6 각 장애의 기대 손실/복구 기준과 UPS 시뮬레이션 oracle 준비**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-012`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] 현재 PC 24h 결과·UPS 모의 차단/프로세스 중단·재개/복구를 시뮬레이션 수락. native/3 OS 생성·읽기 교환은 공개 완료 후 실행하고 미실행 GAP 유지(ADR-011).
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-012](../../05_verification/test_case.md#tc-012). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 2026-10-31 수락 방식·검증 시점 갱신

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체하고 UPS는 시뮬레이션으로 판정한다. native는 GitHub 공개 완료 후 진행한다. 일반 Linux/RT는 실제 실행 커널을 기록하며 물리 전원 소실 내구성의 관측 결과로 표시하지 않는다. 기존 수치/예산/24h/소프트웨어 통합 목표와 선행 의존은 유지하며 이번 문서 갱신만으로 제품 시험을 Pass 처리하지 않는다.

## native·다른 OS 잔여 통합 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)으로 기능 Task(TASK-004~017·019~023·027~029)를 Done으로 닫으면서, 각 Task에 흩어져 있던 native·다른 OS 잔여를 이 Task가 소유한다. 트리거는 그대로 T3(GitHub 공개 완료 확인)이다.

- GitHub CI 6 contracts·audit·raw-memory·`integration-surface` job을 공개 시점 source hash로 실행·확인(GAP-004).
- Windows/macOS 등 Tier 2 대상의 생성·읽기 교환(기존 범위).
- 공개 전에는 CI 재시도나 계정·과금 조회를 하지 않는다(기존 결정 유지).

## 2026-11-22 착수와 결정

Human 「TASK-024로 넘어가자」로 착수했다. 결정(같은 날 Human 선택, 모두 권고안):

- **24h 부하:** 대표 부하 약 2 MB/s — 256 B 기록 5,000건/s + 64 KiB 원본 10건/s, 링 보존 상한 16 GiB, 10 ms 제어 루프 동시 실행. 24h SSD 쓰기 약 170 GB.
- **커널:** 지금 부팅된 RT 커널(`7.0.0-43-realtime`)에서 실행. generic 장기 행은 미실행으로 남긴다.
- **정전 모의:** 2026-11-22 Human 「실제 UPS가 없어서 외부에서 신호 보내고 지정 초(3초?) 뒤에 프로세스 죽이는 테스트로 대신해」 → S2-P. EV-043(프로세스 안에서 경보를 만들고 sync 뒤 스스로 exit 17)과 달리, 경보와 종료가 모두 프로세스 밖에서 온다.
- **순서(2026-11-22 Human):** 「정전테스트 먼저 진행하고 24시간 테스트는 오늘 오후 5시쯤 시작하자」.
- **디스크:** T500(루트 파일 시스템, `/home/lim8603/s3-data`) — 이전 성능 측정(EV-064·071)과 같은 디스크. 빌드 디스크 `/data/build`(HP 512 GB)와 분리.

native CI(3 OS × 2 Rust)는 T3로 끝났다([EV-116](../../05_verification/verification_evidence.md#ev-116), GAP-004 Closed). UPS 모의 차단·복구 하위 행은 EV-043으로 수락됐다.

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | OS 간 파일 교환 | 각 OS(Linux·Windows·macOS)가 결정적 기록 세트(여러 채널·중요 기록·이벤트·회전 2개 이상)를 쓰고, 세 OS 각각이 세 OS의 파일을 모두 읽어 내용·개수·손실 0을 대조하는 CI 작업 | Done — 9/9 Pass([EV-119](../../05_verification/verification_evidence.md#ev-119), run `44987267000`). 도중 macOS·Windows 파일 출력 결함을 찾아 ADR-009 개정 2로 수정(`9f687dd`·`ad11ba3`) |
| S2 | P6 장애 주입(단기) | 대표 부하 아래 느린 파일·느린 구독자·공간 부족·DB/네트워크 단절·프로세스 강제 종료 후 `recover`를 각 몇 분씩 주입. 버퍼 상한 유지, 정책별 손실 보고, 복구 뒤 정상 경로 회복 | Planned |
| S2-P | 정전 모의(외부 경보 → 3초 뒤 강제 종료) | 2026-11-22 Human 지시: 실물 UPS 대신, 대표 부하로 기록 중인 프로세스에 외부 구동기가 경보를 보내고(호스트 앱이 받아 생산자를 멈추고 facade `Logger::shutdown`을 남은 예산 3초 − 200 ms로 호출. local-pc-v1·EV-043과 같은 3초. 고급 경로 `notify_power`는 EV-043 범위) 정확히 3초 뒤 SIGKILL한다. 종료 협조 없음. 판정: 경보 전에 수락된 보호 대상(중요) 기록이 강제 종료 뒤 파일에 모두 남고, 파일은 `recover`로 읽히며 잘린 꼬리만 제외되고, 손실 보고가 실제 누락과 맞는다. 경보 시점을 바꿔 여러 회 반복 | Done — 6/6 Pass([EV-118](../../05_verification/verification_evidence.md#ev-118)), harness `soak.rs` |
| S3 | soak harness와 리허설 | 대표 부하·제어 루프·주기 표본(RSS·fd·예약 바이트·손실/거절 카운터·디스크 사용·deadline miss)을 JSON으로 남기는 harness, 10분 리허설 | Done — 2026-11-22 10:50 KST 10분 리허설(RT·T500·링 16 GiB·60초 표본): 제어 2,980,000건·프레임 6,000건, 거절 0·miss 0·최악 1,426 µs, RSS 31.7 MB(10분간 +44 KiB, 링이 아직 안 참)·fd 5 고정, 1.53 GB·파일 183개, `verify` 전량 Pass(누락·중복·손상·손실 0). 링이 차는 약 1.9시간까지는 예열로 본다 |
| S4 | 24h 실행(RT, T500) | 24h 완주. 판정: 정상 부하 손실 0, 제어 루프 추가 deadline miss 0, RSS·fd·예약 바이트가 예열 뒤 증가 추세 없음, 디스크 사용 ≤ 링 상한, 회전 파일 전부 읽기 대조 Pass | Done — Pass(AI 판정, S5 Human 리뷰), [EV-120](../../05_verification/verification_evidence.md#ev-120). 2026-11-22 16:57 → 11-23 16:57 KST 완주: 제어 431,970,050·프레임 864,001건, 거절·miss 0, 최악 1,940 µs, 디스크 ≤ 15.72 GiB, fd 5, `verify --ring` Pass. RSS +328 KiB/22h는 1시간 heap 진단에서 살아 있는 heap이 바이트 단위로 평평해 누수 아님(glibc 할당기 보유). 조회·전송 동시 장시간은 TASK-040 |
| S5 | 근거·리뷰 | EV, TC-012 P6 행·추적표 갱신, Human 리뷰 | Planned |

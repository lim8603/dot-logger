# TASK-021 — UPS 긴급 계획·상태·모의 시험

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-006 |
| Story | US-001·002·003·004 |
| 요구사항 | FR-018 NFR-001 NFR-003 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress — 로컬 구현/UPS 시뮬레이션 수락, native T3 이월 |
| 마지막 갱신일 | 2026-11-02 |
| 다음 액션 | S1~S3 EV-043 로컬 수락·REV-042 Closed, 다음 구현 TASK-026. native T3·장기 통합 TASK-024 별도 |
| 의존 | TASK-009·TASK-006 |
| 예상 소요 | 2~4 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현: `crates/dot-logger/src/control/emergency.rs`·`platform/power.rs`, 기존 ControlOwner 통합. `examples/ups_simulation.rs`·shared `support/ups`·`tests/tc_011_emergency.rs`가 실제 파일/프로세스 중단 수락을 검증한다.

첫 실패/리뷰 조건: **off에서는 전이/예약 없음·중복 신호로 deadline 연장 없음·종료 경합**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-011`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] rank·snapshot 한도·선택 작업 중지·진행 IO 수명·partial 결과·TC-011의 UPS 시뮬레이션 수락(ADR-011). 실물 UPS 제공은 필수 조건에서 제외.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] 기존 Human 위임으로 로컬 구현/UPS 시뮬레이션 수락·REV-042/RV-084/085 Closed. native/통합 후속은 아래 별도 이월이다.

테스트 정의: [TC-011](../../05_verification/test_case.md#tc-011). 제품 코드·테스트 실행 결과: [EV-043](../../05_verification/verification_evidence.md#ev-043)·[REV-042](../../05_verification/reviews/REV-042_TASK-021_UPS_SIMULATION.md). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 2026-10-31 수락 방식·검증 시점 갱신

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체하고 UPS는 시뮬레이션으로 판정한다. native는 GitHub 공개 완료 후 진행한다. 일반 Linux/RT는 실제 실행 커널을 기록하며 물리 전원 소실 내구성의 관측 결과로 표시하지 않는다. 기존 수치/예산/24h/소프트웨어 통합 목표와 선행 의존은 유지하며 이번 문서 갱신만으로 제품 시험을 Pass 처리하지 않는다.

## 단계별 실행·수락 — 2026-11-02 #001

Human 「TASK-021 UPS 긴급 계획·모의 시험까지 진행하자」 및 실물 UPS 없이 알람 이벤트를 주입하는 수락 방식을 재확인했다. 몇 분의 여유도 같은 절대 기한으로 표현하며 승인3초 시험을 유지한다.

| 단계 | 업무 | 상태/남은 조건 |
|---|---|---|
| S1 | off/시작 계획·atomic mailbox·원 기한/중복·같은 정상 종료 coordinator | EV-043 로컬 수락 |
| S2 | rank/동일 rank ID·보호량·선택 출력 중지·실제 IO/guard·오류/부분 결과 | EV-043 로컬 수락 |
| S3 | 현재 PC ext4 8/64MiB×전달 지연·실제 sync·프로세스 중단/복구 oracle | UPS 시뮬레이션 합격 — 승인된 대체 판정 |
| 통합/플랫폼 후속 | 지속 dot-probe/장기 부하·RT·다른 OS/native | TASK-026/024/022·공개 후 T3. 실물 UPS 제공은 필수 입력 아님 |

- 구현은 `enable_emergency` startup 고정 계획·추가 heap 예약 없는 off·`notify_power` 고정 원자 mailbox다. post-alarm 추가 snapshot quota는0이며 기존 풀의 사전 수락 범위만 frozen cutoff로 보존한다. 파일 protected payload 합계/checked control metadata·필수 Synced target 포함을 검사하고 writer framing/output 예산은 유지한다. host가 frozen 실제 payload 양을 한 번 공급해야 하며 missing/초과/실패/만료를 보호 성공으로 만들지 않는다.
- cutoff는 가장 이른 값만 유지하고 원 신호 발생→전달 지연→안전 여유를 포함한다. 일반 sync 신규 수락/queued 작업은 닫거나 Expired로 끝내고 이미 시작한 sync/Delivery/raw IO guard는 기한 뒤에도 실제 종료까지 보존한다. rank/target ID 순으로 barrier를 시작하며 실패한 상위 파일 뒤에도 하위 파일을 처리할 수 있다. 종료 뒤 자동 Running 복귀 없음.
- 신규 TC-011 통합11개+control 경합1개+할당1개. RV-084는 report 중3초→500ms 단축을 Barrier로 실제 Red→최종 기한 재관측 Green으로 수정했다. RV-085는 초기 `/tmp` tmpfs 근거를 SSD 수락에서 제외하고 trusted parent를 명시해 `/data/build` ext4로 재실행했다.
- source `a5fafcd`·검증파일139개 commit blob 일치. 두 Rust1.96/MSRV1.85 workspace각434 Pass(DB5개 일반회귀 ignore), 두 Rust16 feature/독립 consumer·RT allocator·fmt/clippy/doc Pass. Miri strict provenance seeds0..7: TC-011 non-IO10×8 + control4×8=112 Pass, 알람/RT 거절 할당·해제0/0. default/core에 새 network dependency 없음, Cargo.lock/의존 버전/unsafe/wire 불변.
- **현재 PC ext4 수락:** 8/64MiB×전달0/50/250/1000ms·원점→차단3초·여유200ms를 두 Rust로 각각8조건 실행했다. rank1/2의64MiB 분배48/16MiB, 각8 receipt/metadata/source clock/바이트 복구 일치·sync 순서10→20·actual close 뒤 output 예약0. 두 도구체인16조건의 관측 최대1.803017545초로 work 기한2.8초 이내다. 각 child의 exit17/destructor 없는 중단·잘린 꼬리·parent 사전 oracle/새 reader/출력에서도8개를 대조했고 이전 completion은 복원하지 않았다.
- ENOSPC/sync 실패·늦은 기한·더 이른 기한은 partial, 진행 중 큰 원본은 실제 guard Drop까지 보존했다. 단일 순차 합성 보호 부하 결과이며 P3 동시 부하/장기/RT/native·물리 전원 소실 내구성 수락으로 확대하지 않는다. **실물 UPS는 완료 차단 조건이 아니다.** 로컬 구현/UPS 모의 수락은 완료, Task 상태는 기존 정책대로 native 이월을 명시해 In Progress로 유지한다. 다음 구현 TASK-026, TASK-018은 별도 FR-015 승인 대기다.

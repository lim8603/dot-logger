# TASK-022 — 실제 장비·플랫폼 검증 프로필

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-001·002·003·004·005 |
| 요구사항 | NFR-001 NFR-002 NFR-003 NFR-007 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-02 |
| 다음 액션 | 선행 시험 harness 준비. RT 필요 시 설치 상태/실행 계획 확인→Human에게 설치·전환/재시작 요청→승인·RT 부팅 확인 후 시험 |
| 의존 | TASK-001 |
| 예상 소요 | 1~2 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

준비 경로: `test-profiles/local-pc.toml`·`scripts/check-test-profile.py`·`.cowork/05_verification/test_strategy.md`. 프로필/읽기 전용 점검만 생성했으며 제품 성능 harness는 후속이다.

첫 실패/리뷰 조건: **문서 검토: 장비/OS/CPU/SSD/FS/NIC/UPS·제어 주기·deadline 누락 탐지**.

- [x] 문서/환경 검토: 장비 미확정과 RT 설치/실행 혼동을 확인하고 ADR-011로 수락 기준·관측 사실을 구분했다.
- [ ] 최소 구현 후 관련 `TC-012`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] Human 검증 장비·UPS 방법 확정 — 현재 PC 실측을 실제 장치 시험으로 대체, UPS 시뮬레이션 수락(ADR-011).
- [x] P0 도구 계획/부하·제어 주기/deadline·저장 경로/예산·수치 수락 프로필 고정 — local-pc-v1 Human 승인.
- [ ] 실행 커널별 환경/capability 확인 — generic 관측 완료, RT 행은 해당 커널 부팅 후. GitHub native는 공개 완료 후 이월.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 수치 프로필 리뷰 완료 — 2026-10-31 「승인」. 실제 실행/제품 검증은 별도이며 새 H/J 결정이 필요하면 기록한다.

테스트 정의: [TC-012](../../05_verification/test_case.md#tc-012). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 확정 장비·수락 방법 — 2026-10-31

Human 결정 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)으로 현재 PC의 실측을 실제 장치 테스트 대체 근거로 수락한다. UPS는 실물을 제공받지 않고 시뮬레이션 결과로 판정하며 GitHub native는 저장소 공개 완료 후 수행한다. 본 Task는 환경/수락 프로필 준비로 착수하며 종합 성능 시험은 아직 실행하지 않았다.

| 항목 | 관측·수락 조건 |
|---|---|
| PC/OS | AMD Ryzen7 H255, 8코어16스레드 / Ubuntu26.04.1 LTS x86_64 |
| 메모리 | OS 인식59,750,248,448B(약55.6GiB). logger 예산과 OS cache/host 앱 메모리는 분리 |
| 실행 커널 | 7.0.0-42-generic / CONFIG_PREEMPT_DYNAMIC=y, CONFIG_PREEMPT_RT 미설정 |
| RT 설치 과거 관측 | 2026-10-31에7.0.0-38-realtime/CONFIG_PREEMPT_RT=y를 관측한 이력. 현재 설치/부팅 가능 여부는 RT 준비 때 재확인하며 실행 중인 커널로 간주하지 않음 |
| SSD/FS | CT1000T500SSD8: 루트 ext4(relatime). HP EX950512GB 2개: /data/build ext4(noatime), /data/db ext4. 시험별 실제 경로 고정 필요 |
| UPS | 시뮬레이션 수락: 모의 원점·전달 지연·약3초 기한·최대 보호량·진행 IO·실제 sync/중단·복구/receipt 대조 |
| native | 공개 완료 후 source hash 고정·6 contracts+audit+raw-memory. 현재 미실행 이월 |
| 남은 프로필 | 제어 주기/deadline·대표 입력/보호량·P0 도구·NIC/원격 범위·CPU affinity/우선순위·시험 경로/예산을 실행 전에 선언 |

[환경 관측 원문](../../06_evolution/imported_context/verification_policy_2026-10-31/environment.json). 이 기록은 환경 확인이며 제품 성능·RT 지연·UPS 합격 증거가 아니다. 실물 UPS/별도 장치 미제공을 착수 차단 사유로 사용하지 않는다. 커널/부트 설정 변경·재부팅은 수행하지 않았다.

<a id="현재-pc-수치-프로필-초안--2026-10-31"></a>
## 현재 PC 수치 프로필 — 2026-10-31 Approved

Human 「승인, 다음작업 진행하자」에 따라 TASK-008/010/011 리뷰를 닫고 본 Task의 수치 프로필을 구체화했다. **local-pc-v1은 제어 주기/기한10ms 조정 뒤 2026-10-31 Human 「승인」으로 Approved다.** 장비/UPS 방식·native 시점은 ADR-011의 기존 승인을 그대로 사용한다. 제품 코드·P0~P6 부하 실행·재부팅·권한/설정 변경·도구 설치는 수행하지 않았다.

실행 입력은 [local-pc.toml](../../../test-profiles/local-pc.toml), 읽기 전용 점검은 [check-test-profile.py](../../../scripts/check-test-profile.py)다. 선언과 관측 환경을 비교할 뿐 workload/파일 생성/CPU pinning을 시작하지 않는다. `python3 scripts/check-test-profile.py test-profiles/local-pc.toml`은 선언 유효성을 출력하고, `--require-ready`는 승인/RT 부팅/구현 준비가 남으면 exit2다. 선언 유효를 제품 성능 합격으로 사용하지 않는다.

| 항목 | local-pc-v1 승인 기준·판정 경계 |
|---|---|
| 환경 | 현재 Ubuntu generic / 실행7.0.0-42, RT 행은 설치 상태 재확인·Human 요청/승인·필요 설치/부팅 전환·재시작 및 실행 커널/sysfs/권한 확인 후. generic 결과와 RT 결과 분리 |
| 저장 | `/data/build` ext4(noatime), 매 run 새로운 `dot-logger-verification-*` 전용 디렉터리. run cap160GiB·별도 free reserve64GiB·파일1GiB. 원본/운영 파일 덮어쓰기 없음. run 전후 실제 mount/여유·원본/출력 경로 기록 |
| 예산 | 2GiB: input64/raw1408/output256/external128/control64/worker128MiB, 기존 설계 배분. 512MiB/4GiB 비교는 별도 concrete quota/최대 record/startup 검증 뒤 사용하며 단순 배율로 실행하지 않음 |
| CPU | control2·safety3, SMT10/11은 workload 배치에서 제외, worker4~7/12~15, housekeeping0/1/8/9. 실제 affinity 기록·동일 설정 off/on 비교. 현재 시스템 전체 격리/IRQ 이동은 수행하지 않으며 affinity만으로 OS 간섭 제거를 주장하지 않음 |
| 제어 | 합성 control10ms/기한10ms·고정 작업량500µs, safety10ms/기한10ms·200µs. logger off에서 반복 수를 먼저 보정한 뒤 동일 연산량을 고정. generic SCHED_OTHER, RT control FIFO70/safety80은 capability/메모리 잠금 확인 후. **합성 시험의 승인 조건이며 실제 앱의 WCET 보장은 아님** |
| 제어 대조 | 동일 CPU/입력/커널·off/on 각3회, 절대 단조 시각 release와 완료를 측정. 건너뛴 release도 miss에 포함. miss는 각 명목 release+deadline 이전에 작업 완료 못한 주기, 추가 miss0 목표. paired counts/raw 최대·histogram·wake 지연/실행시간 분리 |
| P0 | TASK-023의 simple-file Rust writer와 합성 제어 loop를 짝으로 구성(아직 미구현). 동일 입력/출력 bytes·파일 수/회전·쓰기 블록/배치·1초 sync·최종 file/directory sync를 맞추고 encoding/framing bytes와 실제 syscall 크기도 별도 기록. 기준선이 다르면 효율 비율을 판정하지 않음. cyclictest는 scheduler 보조 진단이며 앱 제어 시험 대체 아님 |
| P1/P2/P3 | 기존 목표 유지: P1 ≤256B×10만/s·생산자1/4/8/16/32·파일1/4/16. P2 4×8MiB×30/s. P3 합산1,032,232,960 payload B/s, 32생산자/16파일/128등록채널·압축off·index/CRC on. 원본 pool/입력 거절·정책 생략·target별 receipt/bytes 무결성 대조 |
| 시간/시험량 | 예열10초·독립3회·본 측정120초/회. 본 측정 payload123,867,955,200B ≥ 실행 시 MemTotal×2를 둘 다 확인. 예열+metadata16GiB+active파일16GiB 허용량 포함168,550,023,168B ≤ cap160GiB. 측정 뒤 backlog/실제 sync/디렉터리 sync까지 처리량 분모에 포함. raw 통계와 반복별 percentile을 보존 |
| 디스크 재사용 | 반복별 oracle/receipt·환경·hash 보존 및 reader/writer/진행 IO 종료 확인 뒤 해당 run이 소유한 파일만 정리. 3회 데이터를 동시에 유지하지 않아도 되며 여유 reserve 위협 시 admission 중단·미완료 보고·같은 owner로 IO 정리. global drop_caches/기존 파일 삭제는 계획에 넣지 않음 |
| 합격 | 기존 조건부 R1목표·호출 p99≤5µs/p99.9≤20µs·정상 부하 손실/손상0·구성 메모리 상한·추가 deadline miss0 유지. SSD P0가1.5GB/s 미달이면 현재 PC 별도 행으로 보고하며 R1 달성을 선언하지 않음. 비율80%만으로 배포 입력 요구 충족을 대신하지 않음 |
| P5 | P3와 동시, 1client·초당1회·≤1000건/256KiB page·p95≤200ms. 조회/재생 예산·read-only source와 실제 reader 종료를 유지. 실제 timer 재생은 가상시계 회귀와 별도 측정 |
| UPS 모의 | 기본off/on, 원점→차단3000ms·전달0/50/250/1000ms·안전여유200ms·보호8/64MiB. 최대64MiB는 rank1 48/rank2 16MiB, 원본 풀 내 사전 보호 quota에 포함. 중복/더 이른1500ms 기한·진행 IO·sync 오류/부분 결과·중단 후 복구/외부 receipt oracle 대조. 물리 전원 소실 내구성으로 표시하지 않음 |
| 후속 | P4는 원격/두 host 경로·TCP 기준선 준비 후, P6 24h는 선언 배포 부하/순환 용량·선행 Task 준비 후, native/OS 교환은 공개 후T3. 이 초안 승인만으로 TASK-021/023/024/026 구현 완료·시험 실행을 선언하지 않음 |

### 관측과 실행 준비

[현재 환경 관측](../../06_evolution/imported_context/task_022_profile_2026-10-31/preflight.json): `/data/build` 여유341,256,384,512B, cap+reserve240,518,168,576B보다 큼. 실제 run 시 다시 검사한다. CPU2↔10·3↔11 SMT 대응을 확인했다. cyclictest/perf/rtla/stress-ng/iostat 설치, fio/iperf3 부재. 파일 P0는 동기화/회전 조건을 맞출 수 있는 Rust harness를 예정하므로 fio 설치를 선행 차단으로 사용하지 않는다. 원격 iperf3 대안/설치는 해당 Task에서 결정한다.

원참조 제어 주기20ms/안전10ms와 priority는 dot-probe `control_thread.rs:327`/`safety_thread.rs:91`/`realtime.rs:92`의 로컬 코드 관측이며 실제 배포 설정·승인된 deadline은 아니다. 참조 HEAD와 파일 작업 트리 SHA-256은 [점검 근거](../../06_evolution/imported_context/task_022_profile_2026-10-31/checks.json)에 고정했다. 다른 저장소는 읽기만 했다. 본 합성 행은 dot-probe 실사용 통합(TASK-026) 수락을 대체하지 않는다.

[EV-021](../../05_verification/verification_evidence.md#ev-021): 선언 유효성·실행 준비 미충족 분리, 5개 부정 입력 거절·기존 source54개 불변 확인. 제품 성능/RT/UPS 시험은 Not Run이다. 수치 프로필 승인/approval 필드 갱신을 완료했다. 실제 실행 revision·환경·선행 harness/capability를 고정한 뒤 실행한다. TASK-022 In Progress·Gate4/5 Not Ready 유지.

### 제어 주기 조정 — 2026-10-31

Human 「20ms를 10ms로 줄일 수있을까?」에 따라 합성 제어 주기와 완료 기한을 모두10ms(100Hz)로 반영했다. 주기당 목표 연산500µs는 유지하므로 합성 연산만의 시간 비중은2.5%→5%, 초당 실행 횟수는50→100회다. calibration 뒤 고정 연산량을 사용하며 OS/로깅 오버헤드는 실제 off/on 시험으로 별도 판정한다. 실제 dot-probe 코드의20ms 기본값은 수정하지 않았다. 이 변경을 전체 프로필 수치의 일괄 승인으로 확대하지 않으며 나머지 초안/미실행 경계를 유지한다.

[10ms 선언 점검](../../06_evolution/imported_context/task_022_profile_2026-10-31/control-10ms-checks.json)은 수정된 프로필 hash/선언 Valid·성능 NotRun을 기록한다. 기존 checks.json의20ms 프로필은 [당시 스냅샷](../../06_evolution/imported_context/task_022_profile_2026-10-31/profile-control20ms.toml)으로 보존한다.

### Human 수치 프로필 승인 — 2026-10-31

10ms 조정 보고 뒤 Human 「승인」을 local-pc-v1 수치 프로필 수락으로 반영했다. 주기/기한10ms·주기당 합성500µs, 안전 루프10ms·200µs, 메모리2GiB·P3 120초×3회·cap160GiB/reserve64GiB·UPS 모의3초/8·64MiB 및 위 표의 조건을 확정했다. 앞선 Draft/승인 대기는 당시 이력이다. approval=Approved·execution=NotRun이며 구현/부팅/실행 결과를 승인으로 대신하지 않는다.

[승인 프로필 스냅샷/사전 점검](../../06_evolution/imported_context/task_022_profile_2026-10-31/approval-checks.json)에서 선언 Valid exit0·준비 점검 exit2, 승인 대기 이유 제거를 확인했다. 선행 성능 harness·실앱 통합/UPS adapter 및 RT 부팅/capability 확인은 잔여다. TASK-022 In Progress·TC-011/012 NotRun·Gate4/5 Not Ready 유지. 기존 조건을 재승인받지 않고 다음 준비에서 사용한다.

## REV-022 RV-049 포화 회복 실측 후속 — 2026-10-31

일반 네트워크 프로필에서 포화 중 초과 연결 거절률/CPU, permit 반환 후 다음 accept 및 backlog 회복 지연을 구분해 측정한다. 현재 10ms 냉각/연결 상한은 유지한다. permit 반환 뒤에도 N개를 반드시 거절한다는 N×10ms 일반식은 사용하지 않는다. 유한 batch 최적화는 이 측정 뒤 판단하며 이번 EV-026 기능 회귀를 처리량/지연 실측으로 승격하지 않는다.

## TASK-014 계측 비용 실측 후속 — 2026-10-31

examples/telemetry_overhead.rs의 enabled/disabled·순서 반전 fixture를 실제 검증 프로필에서 release/CPU·작업 부하·반복 조건을 고정해 실행한다. 기본 계측 완료 처리량 비용≤5%와 계측 포함 RT 지연은 미측정 목표다. EV-027의 128회 debug smoke/할당0은 비용 또는 RT 목표 Pass가 아니다. raw acquisition 거절과 제출 결과, GeneralProducer 재시도 판정 수를 구분해 조건/카운터를 기록한다.

### TASK-021 UPS 프로필 실행 — 2026-11-02

[EV-043](../../05_verification/verification_evidence.md#ev-043)으로 local-pc-v1의 UPS 하위 행을 실행·수락했다. 현재 generic PC·`/data/build` ext4에서8/64MiB×전달0/50/250/1000ms·원점3초/안전여유200ms·64MiB rank48/16MiB, 두 Rust 각각8조건의 실제 MCAP sync/복구·child 중단 후 oracle 대조 Pass. 관측 최대1.803017545초, output 예약0. 초기 tmpfs 측정은 SSD 수락 근거에서 제외했다. 실물 UPS는 필수 입력이 아니다. 앞선 EV-021의 UPS NotRun은 당시 준비 이력이며 현재 UPS 하위 행은 Simulation Accepted다. P0~P6/10ms 제어 부하·RT·24h·통합/native 전체 검증은 미완료다.

### RT 준비·재시작 요청 규칙 — 2026-11-02 Human 명시

현재 PC는 Ubuntu 일반 커널로 실행 중이며 RT 커널을 올려 실행한 상태가 아니다. 이번 읽기 전용 `uname -r` 확인도 `7.0.0-42-generic`이다. 이전 설치 기록을 현재 RT 실행/설치 완료의 증거로 사용하지 않는다.

RT 시험이 필요한 시점에는 AI가 먼저 시험 harness·측정 항목/예상 시간·현재 설치 상태를 확인하고, 필요한 RT 커널 설치/부팅 선택·재시작과 작업 재개 절차를 구체적으로 정리해 Human에게 요청한다. Human 승인 전에 커널 설치/부트 설정 변경/재시작을 실행하지 않는다. 재시작 후에는 실행 커널·CONFIG_PREEMPT_RT·sysfs/capability를 실제 확인하고 동일 조건의 generic/RT 결과를 분리한다. 현재까지 EV-043 UPS 시뮬레이션을 포함한 PC 실행 결과는 일반 커널 결과이며 RT 시험은 Not Run이다.

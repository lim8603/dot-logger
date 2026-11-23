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
| 다음 액션 | S5 근거·리뷰(EV-118~120·123, TC-012 P6 행·추적표, Human 리뷰). S2 Done(EV-123) |
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
| S2 | P6 장애 주입(단기) | 대표 부하 아래 느린 파일·느린 구독자·공간 부족·DB/네트워크 단절·프로세스 강제 종료 후 `recover`를 각 몇 분씩 주입. 버퍼 상한 유지, 정책별 손실 보고, 복구 뒤 정상 경로 회복 | Done(AI 판정, S5 Human 리뷰) — F1·F2a·F2b·F3a·F3b·F5 Pass, F4 조용한 단절 미감지 1항목 Fail(keepalive 없음, 기능 논의 안건), F4-DB는 D2로 tc_008 갈음([EV-123](../../05_verification/verification_evidence.md#ev-123)) |
| S2-P | 정전 모의(외부 경보 → 3초 뒤 강제 종료) | 2026-11-22 Human 지시: 실물 UPS 대신, 대표 부하로 기록 중인 프로세스에 외부 구동기가 경보를 보내고(호스트 앱이 받아 생산자를 멈추고 facade `Logger::shutdown`을 남은 예산 3초 − 200 ms로 호출. local-pc-v1·EV-043과 같은 3초. 고급 경로 `notify_power`는 EV-043 범위) 정확히 3초 뒤 SIGKILL한다. 종료 협조 없음. 판정: 경보 전에 수락된 보호 대상(중요) 기록이 강제 종료 뒤 파일에 모두 남고, 파일은 `recover`로 읽히며 잘린 꼬리만 제외되고, 손실 보고가 실제 누락과 맞는다. 경보 시점을 바꿔 여러 회 반복 | Done — 6/6 Pass([EV-118](../../05_verification/verification_evidence.md#ev-118)), harness `soak.rs` |
| S3 | soak harness와 리허설 | 대표 부하·제어 루프·주기 표본(RSS·fd·예약 바이트·손실/거절 카운터·디스크 사용·deadline miss)을 JSON으로 남기는 harness, 10분 리허설 | Done — 2026-11-22 10:50 KST 10분 리허설(RT·T500·링 16 GiB·60초 표본): 제어 2,980,000건·프레임 6,000건, 거절 0·miss 0·최악 1,426 µs, RSS 31.7 MB(10분간 +44 KiB, 링이 아직 안 참)·fd 5 고정, 1.53 GB·파일 183개, `verify` 전량 Pass(누락·중복·손상·손실 0). 링이 차는 약 1.9시간까지는 예열로 본다 |
| S4 | 24h 실행(RT, T500) | 24h 완주. 판정: 정상 부하 손실 0, 제어 루프 추가 deadline miss 0, RSS·fd·예약 바이트가 예열 뒤 증가 추세 없음, 디스크 사용 ≤ 링 상한, 회전 파일 전부 읽기 대조 Pass | Done — Pass(AI 판정, S5 Human 리뷰), [EV-120](../../05_verification/verification_evidence.md#ev-120). 2026-11-22 16:57 → 11-23 16:57 KST 완주: 제어 431,970,050·프레임 864,001건, 거절·miss 0, 최악 1,940 µs, 디스크 ≤ 15.72 GiB, fd 5, `verify --ring` Pass. RSS +328 KiB/22h는 1시간 heap 진단에서 살아 있는 heap이 바이트 단위로 평평해 누수 아님(glibc 할당기 보유). 조회·전송 동시 장시간은 TASK-040 |
| S5 | 근거·리뷰 | EV, TC-012 P6 행·추적표 갱신, Human 리뷰 | Planned |

## S2 설계 — 2026-11-23

2026-11-23 Human 「S2 설계까지 진행」. 실행은 결정 D1~D3과 REV-072 리뷰 뒤.

### 코드로 확인한 전제

1. facade 파일 저장소는 주입 불가 — `logger/host.rs`가 `FileStore`와 실제 `statvfs` probe를 고정한다(tc_013_facade의 주석도 같음). 느린 저장·공간 부족은 OS 수준으로 만든다.
2. 공간 부족은 두 경로다(`storage/retention.rs` `room()`, `logger/host.rs` `settle`·`failure`).
   - probe 경로: `free < reserve + max_segment`이면 ring은 보호되지 않은 자기 오래된 닫힌 파일부터 지우고, 지울 것이 없으면 `RetentionError::Capacity`. facade는 그 배치 기록을 실패로 보고하고 출력은 유지한다(RV-125). 공간이 돌아오면 다시 기록한다. 손실 보고는 `Scope::OutputFailed`·`Cause::Adapter`다(`host.rs` `failure()`가 `Capacity`를 `AdapterFailure`로 바꾼다. REV-072 RV-193 ①).
   - 실제 쓰기 ENOSPC: log poisoned → `io_failed`·출력 `failed` → `Failed(IoPoisoned)`. 그 세션에서 회복하지 않는다. RT admission은 닫히지 않아 `try_log`는 계속 `Accepted`이고 이후 배치는 모두 `IoFailure`로 실패하며, poisoned log는 새 손실 checkpoint를 파일에 쓰지 못한다(RV-193 ③).
3. `HealthState::Degraded(RecordsFailed)`는 고착된다(`failed_records` 누적, 초기화 없음). 회복 판정은 `failed_records` 증가가 멈추는지로 한다. `RecordsFailed`가 `SyncOverdue`보다 우선이라 한 번 실패하면 상태에서 `SyncOverdue`가 가려진다 — sync 지연은 `OutputHealth.since_sync`로 본다.
4. facade에는 DB(InfluxDB) 출력이 없다. 고급 경로 `adapters::influxdb::{Writer, RoutedWriter}`와 가짜 DB 시험(tc_008: 응답 지연·멈춤·포화)이 있다.
5. facade는 시작할 때 이전 세션의 `.active`를 복구하지 않는다(매 `start()`가 새 세션). 공개 `storage::recover_segment`·`recover_to_store`는 세그먼트 단위이고 원래 채널 등록·`WriterBudget`이 필요하다. CLI `recover`는 손으로 쓴 manifest가 필요하다.
6. 원격 구독자는 파일 출력 0이 쓴 것의 best-effort 사본을 받으며 완료 대상이 아니다. 구독 한도는 facade 기본 `subscription_bytes ≤ 4 MiB`, `max_age` 10 s, 정책 `Gap`/`Disconnect`는 구독자가 고른다. 미ACK 기록이 `max_age`를 넘기면 정책과 무관하게 서버가 구독을 닫는다(`hub.rs` `expire`, 스트림 `Gone`). `Gap`은 한도 초과 기록을 건너뛰고 이어 보내는 것이지 무기한 정지를 견디는 것이 아니다(RV-193 ②).

2·3·5는 S2에서 관측해 기록하는 현재 동작이다. 바꿀지는 MS-010 2단계 전 기능 논의 안건으로 올린다(권장, D3).

### 환경(root 없이 확인)

- 공간 부족: `unshare -rm` 안에서 크기 제한 tmpfs 마운트·실제 ENOSPC 확인(2026-11-23).
- 네트워크: `socat` 프록시. SIGSTOP = 조용한 단절(패킷 정지), kill = 연결 끊김.
- 느린 디스크: 사용자 cgroup에 io 컨트롤러 위임 없음(`cpu memory pids`), FUSE 라이브러리 없음. **Docker는 사용 가능**(29.1.3, cgroup v2, 사용자 docker 그룹) — 2026-11-23 Human 제안 「도커 쓰면 안되나?」·「디스크 사용량도 도커로 지정할 수 있잖아?」로 확인:
  - `--device-write-bps /dev/nvme0n1:5mb`에서 40 MiB fsync 쓰기가 1.1 GB/s → **5.2 MB/s**(8.03초)로 제한됐다. 버퍼 쓰기의 writeback도 cgroup v2에서 컨테이너에 귀속된다.
  - `--tmpfs /out:size=64m`에서 80 MiB 쓰기가 64 MiB에서 ENOSPC로 멈췄다.
  - 로컬 이미지 `influxdb`·`postgres:16`이 있다. 이미지 glibc(Debian 2.36)가 호스트(2.43)보다 낮아 호스트 빌드 soak은 컨테이너 안에서 호스트 루트를 `chroot --userspec=1000:1000`으로 실행하는 방식을 하네스 작업 때 확인한다. RT는 `--cap-add SYS_NICE --ulimit rtprio=99 --ulimit memlock=-1`.
- 출력은 `/dev/shm` 또는 작은 tmpfs → SSD 쓰기 약 0. F1만 T500에 제한 속도로 쓴다(10분 × 수 MB/s, 수 GB 이하).

### 시나리오

모두 대표 부하(EV-120과 같음)·RT 설정, 장애 전·중·후 구간을 나눠 판정한다. 시나리오당 약 10분, 합계 약 1~1.5시간.

| # | 장애 | 주입 | 기대 결과 |
|---|---|---|---|
| F1 | 느린 파일 | D1 — Docker 쓰기 속도 제한(예: 대표 부하 2.56 MB/s보다 낮은 1 MB/s를 3분) | 버퍼 상한 유지, 차면 `try_log` `Capacity` 거절·손실 보고 일치, `SyncOverdue` 관측, 해제 뒤 거절 중지 |
| F2a | 느린 구독자(짧은 정지·느린 소비) | facade `.remote()` + CLI `tail`을 `max_age`(10 s)보다 짧게 SIGSTOP(3~5 s) 반복, 또는 소비 속도 제한. 정책 Gap·Disconnect 각각 | 파일 기록 손실 0, 서버 구독 메모리 ≤ 4 MiB. Gap: 같은 스트림에서 gap 이벤트 뒤 계속 수신. Disconnect: 한도 초과 시 끊김 |
| F2b | 느린 구독자(긴 정지) | 같은 구성에서 2분 SIGSTOP → SIGCONT | 파일 기록 손실 0, 10 s 뒤 서버가 구독을 닫음(두 정책 모두 `Gone`), 구독자 재접속 뒤 이어 받음 |
| F3a | 공간 부족(probe) | Docker `--tmpfs size=512m`(또는 `unshare -rm`)·링 256 MiB(세그먼트 32 MiB, `reserve` 0), 채움 파일로 여유 축소 → 삭제. `Capacity`에 닿으려면 자기 닫힌 파일을 다 지운 뒤에도 `free < reserve + 32 MiB`여야 하므로 채움 > 512 MiB − reserve − 32 MiB − active(reserve 0이면 약 450~480 MiB), 또는 `.reserve(...)`를 크게 둔다 | 자기 오래된 파일 삭제 → `Capacity` 실패 → 손실 보고 `OutputFailed`·`Cause::Adapter` = `failed_records` → 채움 삭제 뒤 `failed_records` 증가 멈춤·이후 기록 전부 존재. 종료 보고 `is_success() == false`(실패 기록이 있으므로 정상) |
| F3b | 공간 부족(실제 ENOSPC) | probe 뒤 쓰기 전에 채우는 경합 또는 reserve 0·세그먼트보다 작은 여유 | `IoPoisoned` 고착, 세션 안 회복 없음, RT `try_log`는 계속 수락 — 현재 설계 동작 기록(D3). 대조는 메모리 값만: `OutputHealth.failed_records` = `ShutdownReport` 필수 출력 실패 수(파일 손실 checkpoint는 쓰이지 않음) |
| F4 | 네트워크 단절 | F2a 구성 + socat 프록시 SIGSTOP / kill | 파일 손실 0, 구독자 끊김 감지·재접속 뒤 계속 |
| F4-DB | DB 단절 | D2 | — |
| F5 | 강제 종료 → 복구 | 경보 없이 SIGKILL 6회(시점 변경) → `.active`를 `recover_segment`로 복구 → 같은 디렉터리에서 재시작 | 닫힌 파일 전부 온전. `.active`: (마지막으로 완료된 sync 전에 수락된 수) ≤ 복구된 수 ≤ (SIGKILL 시점 수락 수), 복구분 안에 중간 빈칸 0, 잘린 꼬리만 제외. SIGKILL은 페이지 캐시를 버리지 않으므로 마지막 `flush_chunk`까지 남을 수 있다. 재시작 시 새 세션 디렉터리 정상 생성 |

공통 Pass: 제어 루프 deadline miss 0(장애가 RT 스레드를 막지 않음), 살아 있는 heap·RSS 상한 안, 장애 구간 밖 거절·실패 0, 회복 뒤 기록 전부 존재. **실패 기록 수 = 파일 손실 보고 합계**는 파일이 마지막 checkpoint를 쓸 수 있는 F1·F2·F3a·F4에만 적용한다. F3b는 메모리 대조(위), F5는 sync 하한·수락 상한 대조(위)로 판정한다. sync 지연은 상태가 아니라 `since_sync`로 본다(전제 3).

하네스(약 1인일): soak에 remote 옵션, 표본에 `failed_records`·`since_sync`·출력별 상태·**마지막 완료 sync 시점의 수락 수**(F5 하한 오라클), `.active` 복구·대조 추가. 시나리오 구동 스크립트.

REV-072 RV-193(P1, 2026-11-23 반입)으로 F2를 F2a·F2b로 나누고 F3a 손실 원인·채움 크기·종료 판정, F3b·F5 오라클, 공통 Pass 적용 범위를 코드 계약에 맞췄다.

### 하네스 구현(2026-11-23) — 설계와 달라진 점

- **구성:** `examples/soak.rs` `run`에 `--sample-ms`·`--reserve-mib`·`--remote <port>`(remote feature), 표본에 출력별 `failed_records`·`failed`·`since_sync_ms`, 종료 전 실패 수 안정화 대기(종료 보고와 대조용). `inspect`는 닫힌 파일과(`--active`) 미완료 파일의 기록 범위·빈칸과 마지막 누적 손실 checkpoint 합계(scope/cause별)를 낸다. 구동·판정은 `scripts/soak-faults.py`.
- **실행 위치:** F1·F3a·F3b는 Docker 컨테이너(`sleep`으로 띄운 뒤 `docker exec` + `chroot /host`로 호스트 바이너리 실행, RT ulimit), F2·F4·F5는 호스트 `/dev/shm`.
- **F1 쓰기 제한 전환:** `--device-write-bps`는 실행 중 바꿀 수 없어, 권한 있는 보조 컨테이너가 시험 컨테이너 cgroup의 `io.max`를 쓰고 푼다(2026-11-23 Human 「privileged 써도 돼」).
- **F2·F4 구독자:** CLI `tail --channel 2 --originals`(프레임 채널, 초당 10건 × 64 KiB), 수신 창 4 × 1 MiB(구독 한도 4 MiB 안), `--timeout-ms 3600000`(기본 5 s는 명령 전체 기한). 제어 채널(초당 5,000건)은 평상시에도 CLI가 다 받지 못해 gap이 생기므로 정지 판정에 쓰지 않는다. F2a 정지는 8 s(프레임 속도로 4 MiB가 약 6.4 s에 참).
- **F3b 주입:** 바이트 채움으로 probe와 쓰기 사이를 노리는 경합은 디버그 60 s 동안 probe 경로 실패만 냈다. 대신 tmpfs `nr_inodes=64`로 두고 장애 시점에 남은 inode를 모두 쓰면, 다음 세그먼트 생성이 바이트 여유(약 480 MiB)가 있는데도 실제 ENOSPC로 실패한다(probe는 바이트만 본다).
- **F5 읽기:** facade가 원래 `WriterBudget`·채널 등록을 내주지 않아 `recover_segment` 대신 같은 chunk 검증을 하는 `NativeReader`로 `.active`의 유효한 앞부분을 읽는다(찢어진 꼬리는 멈춤). sync 하한은 표본의 `since_sync`로 얻은 sync 완료 시각 중 직전 것보다 앞선 표본의 수락 수.
- **안전장치(2026-11-23 사고 뒤):** 첫 구현이 tmpfs를 컨테이너 `/home/...`에 마운트해 chroot된 soak·채움이 호스트 루트 디스크(T500)에 썼다 — F3a 디버그가 여유 16 MiB까지 채웠고(Human 디스크 경고), 경합 채움이 강제 종료되며 832 GB 파일이 남아 루트가 100%가 됐다(즉시 삭제, `fallocate`라 데이터 쓰기는 없음). tmpfs는 `/host<경로>`에 마운트하고, 채우기 직전마다 같은 chroot 안에서 `stat -f`로 tmpfs·크기를 확인해 아니면 중단한다. 재실행에서 루트 여유 변화 0 확인.

### 결정(Human, 2026-11-23 「D1~D3 권장대로 진행하자」)

| ID | 질문 | 선택지 | 결정 |
|---|---|---|---|
| D1 | F1 느린 디스크 방법 | **D. Docker `--device-write-bps`·`--device-write-iops`(의존성 추가 없음, 속도 정확, 확인됨)** / A. 하네스 전용 FUSE 지연 파일 시스템(`fuser` dev-dependency) / B. 같은 SSD 경쟁 쓰기(재현성 낮음, 수십 GB) / C. sudo dm-delay | **D** — Docker 쓰기 속도 제한 |
| D2 | DB 단절 | A. 기존 tc_008 가짜 DB 시험(멈춤·포화·타임아웃)으로 갈음하고 "facade DB 출력 없음"을 기능 논의 안건으로 / B. soak에 고급 경로 DB 출력을 붙이고 로컬 `influxdb` 컨테이너를 멈추거나(`docker pause`) 끊어 장시간 주입(+약 1인일) | **A** — tc_008로 갈음. EV에 "DB 장시간 단절 미실행, tc_008 단기 가짜 서버 시험으로 대체" 명시. facade DB 출력은 기능 논의 안건 |
| D3 | F3b·F5 판정 | A. 현재 설계 동작으로 기록, 변경은 기능 논의 안건 / B. 결함으로 보고 S2 안에서 수정 | **A** — RV-193 오라클로 관측·기록. `IoPoisoned` 뒤 RT 수락 지속·`.active` 자동 복구 없음은 기능 논의 안건 |

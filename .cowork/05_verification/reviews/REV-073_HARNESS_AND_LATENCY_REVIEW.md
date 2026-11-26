# REV-073 — 플랫폼 후속 조치·하네스·공개 지연 수치 독립 리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-26 / Codex |
| 대상 | [REV-073 요청](REV-073_REQUEST.md), `f30ae5a..2632b6c`. TASK-024 S2, TASK-038~040, TASK-042 및 CI |
| 종합 판정 | **Conditional** |
| 상태 | **Open** |
| 범위 | `.cowork/` 밖 44개 파일(+3,277/−188). EV·Task 문서는 대조 근거로 사용 |

## 1. 요약정리

**P1 1건, P2 7건, P3 3건.** F3 채움·정리의 경로 처리에 로컬 파일 손상 위험이 있다. 장애·혼합 soak는 일부 실패를 통과로 판정할 수 있고, 지연 하네스에는 초기화 실패 시 종료하지 못하는 경로가 있다. 제품 RT 호출 경로에서 새 할당·블로킹 변경은 확인되지 않았다. macOS viewer는 receiver의 예약 함수로 예산과 CLI 상한을 함께 계산한다. README의 지연 표 5행은 원시 결과와 일치하며, 깨어남과 꼬리의 인과 설명은 관측 범위에 맞춰 조정이 필요하다.

| 확인 | 결과 |
|---|---|
| [CI run 45796420659](https://github.com/lim8603/dot-logger/actions/runs/45796420659), `2632b6c` | **19/19 Pass**. Contracts 10개, 외부 소비자 6회, OS 교환 6개 작업, Miri·audit·integration-surface |
| 로컬 | Rust 1.96 fmt·workspace Clippy(all targets/features), Rust 1.85 dot-logger Clippy(all targets/features) Pass. client 61 Pass·1 ignored, 성능 계약 37 Pass, facade 44 Pass |
| 읽기 전용 확인 | 리뷰 범위 diff whitespace 검사와 Bash 구문 검사 Pass. 전체 범위의 `.cowork` 원시 부팅 로그에는 기존 후행 공백 6줄. shard mock 검사, README 집계, 혼합 하네스의 상태 timeout·보존 판정 반례 확인 |
| 실행 범위 | 장시간 soak, Docker 채움, 재부팅, 지연 재측정, 별도 Miri 변이는 실행 범위 밖. Miri 변이 검출력은 EV-128 기존 기록 대조 |

## 2. 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | 이 리뷰 문서 | 지적·검증 근거와 후속 처리 항목 등록 |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | REV-073 상태와 CI 확인 결과 기록 |
| 검토 대상 | [core](../../../crates/dot-logger/src/), [client](../../../crates/dot-logger-client/src/), [viewer](../../../crates/dot-logger-viewer/src/), [tests](../../../crates/dot-logger/tests/) | 플랫폼 계약·예산·시험 범위·시간 의존 수정 |
| 검토 대상 | [soak.rs](../../../crates/dot-logger/examples/soak.rs), [soak-faults.py](../../../scripts/soak-faults.py), [soak-mixed.py](../../../scripts/soak-mixed.py), [latency.rs](../../../crates/dot-logger/examples/support/perf/latency.rs) | 장애 주입·혼합 실행·힙 집계·지연 계측 |
| 검토 대상 | [ci.yml](../../../.github/workflows/ci.yml), [check-feature-matrix.py](../../../scripts/check-feature-matrix.py), [rt-boot.sh](../../../scripts/rt-boot.sh), [run-latency.sh](../../../scripts/run-latency.sh), [README.md](../../../README.md) | CI 구성, 1회 부팅, 측정 기록과 공개 수치 |

## 3. 클래스(타입) 변경표

이 세션의 타입 변경은 없다. 대상 diff의 주요 타입은 아래와 같다.

| 대상 diff | 파일 | 타입 | 역할 |
|---|---|---|---|
| 수정 | [receiver.rs](../../../crates/dot-logger/src/adapters/remote/receiver.rs), [types.rs](../../../crates/dot-logger/src/adapters/remote/types.rs) | `Reassembler`, `RemoteLimits` | Apple 잠금 회계와 예약량 계산 |
| 수정 | [client receiver.rs](../../../crates/dot-logger-client/src/receiver.rs), [live.rs](../../../crates/dot-logger-client/src/live.rs), [viewer main.rs](../../../crates/dot-logger-viewer/src/main.rs), [bridge.rs](../../../crates/dot-logger-viewer/src/bridge.rs) | `ReceiverConfig`, `LiveBudget`, `Options`, `Bridge` | 라이브 수신 예산 계산과 연결 수명 |
| 수정 | [retention.rs](../../../crates/dot-logger/src/storage/retention.rs) | `SegmentStore`, `FileStore` | OS별 디렉터리 동작과 내구성 계약 |
| 수정·추가 | [soak.rs](../../../crates/dot-logger/examples/soak.rs) | `Counting`, `Options`, `Found`, `Events`, `Pass` | 스레드별 힙, 이벤트, 파일 확인 |
| 추가·수정 | [latency.rs](../../../crates/dot-logger/examples/support/perf/latency.rs), [perf/mod.rs](../../../crates/dot-logger/examples/support/perf/mod.rs) | `Target`, `Row`, `Rt`, `Session`, `Jitter`, `SlowBatch`, `IoLoad`, `LatencyHistogram` | 계측과 분위수 |
| 추가 | [soak-faults.py](../../../scripts/soak-faults.py), [soak-mixed.py](../../../scripts/soak-mixed.py) | `Lines`, `Soak`, `Tail`, `Child`, `Subscriber`, `Status` | 자식 프로세스 구동과 판정 입력 |

## 4. 리뷰 포인트

### 중점별 확인 결과

| 중점 | 확인 결과 |
|---|---|
| REV-072 RV-193~201 | RV-193의 F3a 손실 종류·F3b 미회복·F2a/b 분리 반영을 확인했다. F2a의 Gap 필수 판정과 F5의 내구성 근거는 RV-203·204 참고. RV-194는 Windows NTFS 원자성과 flush 한계를 문서화했고, RV-195는 없는 경로를 metadata로 거절하고 native CI로 확인했다. RV-196의 플랫폼 확대와 f9·f11의 범위 제한은 각각 디렉터리 sync·CPU 배치 계약에 맞는다. f15는 Linux 시간 단정을 유지하고 macOS/Windows에서도 무손실과 종료를 확인한다. RV-197의 glibc FFI는 GNU 가드 아래 있다. RV-198은 외부 소비자 6회와 README resolver 안내로 보완했다. RV-199는 viewer의 검증·예약에 같은 checked 계산을 쓰고 이전 작업 종료를 기다려 예약을 회수한다. core/client 공개 메서드는 기존 타입에 값을 추가로 조회하는 형태이며 Apple에서 과거 산식을 하드코딩한 호출자는 상향된 예산이 필요하다. RV-200의 뺄셈 방어와 RV-201의 정수 검사·게시 전 버전 지정 이월을 확인했다. |
| 시간 의존 시험 | ACK는 확인된 개수로 정상 종료를 제한한다. 고정 타이밍 단위 시험은 대기·진행 중 ACK의 `next`와 대기 중 ACK의 `flush`를 직접 검증한다. `handoff`의 미확인 분기는 통합 시험에서 ACK가 먼저 확정되면 실행되지 않는다(RV-210). f17은 0.5 s 뒤 성공 sync와 1.2 s 이내 경과를 검증한다. EV-127의 sync 주기 10 s Red는 부하 중 완료가 없는 회귀에 유효하다. f32 등은 정상 출력의 `Failed`만 최대 5회 시도하고, 최종 에러와 보호·해제·quota 단정은 유지한다. |
| CI·Miri | 각 OS·Rust에 feature 18조합, Windows 6개씩 3 shard. 호출을 mock한 비교에서 단일과 분할의 26개 호출이 일치했다(프로토콜 검사·독립 소비자·RT 할당 가드 포함). 잘못된 shard 순서·분모·형식은 실행 전에 실패하며 형식 오류 3종은 `ValueError` traceback으로 나온다. Miri 300회는 native 20,000회를 유지하며 EV-128에서 기존 CAS 결함이 8개 seed 모두 Red였다. |
| 하네스 | `GlobalAlloc`은 요청을 `System`에 그대로 넘기고 실패한 할당은 세지 않는다. const/drop-free TLS와 고정 슬롯을 사용하며 realloc은 성공한 호출을 센다. RT 힙은 두 등록 스레드의 정상 루프 구간을 읽으므로 시작·종료 비용은 범위 밖이고, 포화 공유 슬롯은 스레드 구분이 약해진다. 히스토그램의 실제 측정 범위는 버킷 상한과 상대 오차 1/32 조건을 충족한다. 기존 산식은 마지막 `u64` 버킷 상한을 0으로 계산하지만 이 행들의 관측 범위에서는 도달하지 않았다. `SlowBatch`는 루프 전에 용량 4,096개를 잡고 초과분을 카운트한다. tmpfs 검사는 쓰는 프로세스 문맥에서 수행한다. F3의 shell 경로 처리와 하네스 판정·종료 문제는 아래에 기록했다. |
| README | 아래 재계산은 두 `performance` 파일에서 분위수마다 3회 중앙값, 최대는 3회 최댓값이다. 두 governor를 합친 generic warm 6개 최대 중 5개가 85 µs를 넘고 RT 최대는 53.950 µs다. 각 파일에 총 45개 행이 있으며 원시 결과의 `success`는 모두 true다. EV-130의 무기록 대조와 고정 연산 자료는 호스트 감속 설명을 뒷받침한다(RV-209). |

| README 행 | RT p50 / p99 / p99.99 / max (ns) | generic p50 / p99 / p99.99 / max (ns) |
|---|---|---|
| Warm 256 B | 251 / 375 / 4,031 / 49,361 | 263 / 375 / 3,711 / 122,739 |
| Cold 256 B | 767 / 1,407 / 6,655 / 19,466 | 831 / 1,311 / 6,271 / 8,476 |
| J 1 kHz 50건 | 11,007 / 15,103 / 92,159 / 116,675 | 11,007 / 14,591 / 86,015 / 115,582 |
| J 10 ms 50건 | 11,519 / 17,919 / 102,389 / 106,216 | 11,775 / 22,015 / 31,107 / 34,657 |
| J 10 ms 50건+IO, 깨어남 | 23,039 / 112,639 / 116,732 / 117,332 | 23,039 / 112,639 / 116,669 / 195,844 |

### 지적 요약

| ID | 중요도 | 위치 | 문제 | 처리담당 판정 | 상태 |
|---|---|---|---|---|---|
| RV-202 | **P1** | [soak-faults.py:488](../../../scripts/soak-faults.py#L488), [534](../../../scripts/soak-faults.py#L534), [547](../../../scripts/soak-faults.py#L547) | tmpfs 확인을 거친 경로를 shell에서 다시 분리·확장해 채움·삭제 대상이 달라질 수 있음 | 대기 | Open |
| RV-203 | P2 | [soak-faults.py:241](../../../scripts/soak-faults.py#L241), [387](../../../scripts/soak-faults.py#L387), [784](../../../scripts/soak-faults.py#L784) | F2a Gap 누락, 힙 증거 누락, producer 오류가 Pass 가능 | 대기 | Open |
| RV-204 | P2 | [soak-faults.py:674](../../../scripts/soak-faults.py#L674), [698](../../../scripts/soak-faults.py#L698) | F5가 sync 완료 시점의 내구성 하한과 실제 SIGKILL 종료를 확인하지 못함 | 대기 | Open |
| RV-205 | P2 | [soak-mixed.py:127](../../../scripts/soak-mixed.py#L127), [193](../../../scripts/soak-mixed.py#L193), [323](../../../scripts/soak-mixed.py#L323) | 조회·구독 워커가 중도 종료돼도 누적 성공 표본으로 통과 | 대기 | Open |
| RV-206 | P2 | [soak-mixed.py:215](../../../scripts/soak-mixed.py#L215), [294](../../../scripts/soak-mixed.py#L294), [321](../../../scripts/soak-mixed.py#L321) | 보존·외부 파일 읽기의 실제 커버리지와 손실 판독 실패를 판정하지 않음 | 대기 | Open |
| RV-207 | P2 | [latency.rs:602](../../../crates/dot-logger/examples/support/perf/latency.rs#L602), [630](../../../crates/dot-logger/examples/support/perf/latency.rs#L630) | M 설정 실패 시 barrier 정지, J 측정 패닉 시 IO 스레드 미종료 | 대기 | Open |
| RV-208 | P2 | [run-latency.sh:26](../../../scripts/run-latency.sh#L26), [47](../../../scripts/run-latency.sh#L47), [60](../../../scripts/run-latency.sh#L60) | 확인 불가인 커널, 오래된 바이너리, 실패·빈 행도 정상 실행 기록으로 남음 | 대기 | Open |
| RV-209 | P2 | [README.md:307](../../../README.md#L307), [320](../../../README.md#L320) | 측정된 분포를 무영향·단일 원인으로 단정 | 대기 | Open |
| RV-210 | P3 | [client transport.rs:938](../../../crates/dot-logger-client/src/transport.rs#L938) | 미확인 상태의 `handoff`를 고정하는 단위 회귀가 없음 | 대기 | Open |
| RV-211 | P3 | [rt-boot.sh:80](../../../scripts/rt-boot.sh#L80) | generic 확인에서 설정 읽기 실패를 허용 | 대기 | Open |
| RV-212 | P3 | [latency.rs:383](../../../crates/dot-logger/examples/support/perf/latency.rs#L383) | R 시계 대조는 호출과 다른 스레드에서 측정하고 첫 번째 종료 결과를 덮어씀 | 대기 | Open |

### 개별 지적

#### RV-202 — F3 채움·삭제의 경로 해석 (P1)

**[리뷰어]** `assert_tmpfs`의 `stat`과 F3a의 `fallocate`는 경로를 argv 한 개로 받는다. F3a 정리와 F3b의 inode 채움·정리는 같은 경로를 따옴표 없이 `sh -c`에 삽입한다. `--disk-dir`에 공백이 있으면 확인한 경로와 실제 `touch`/`rm`의 인수가 달라진다. 앞쪽 단어가 호스트에서 쓰기 가능한 일반 파일이면 `rm -f`가 그 파일을 삭제할 수 있다. chroot가 호스트 루트를 포함하므로 같은 chroot에서 한 tmpfs 검사가 해당 인수를 보호하지 못한다. 경로를 위치 인수로 넘겨 온전히 인용하거나 전용 디렉터리 안에서 argv로 생성·정리한다. 공백과 glob 문자 경로의 인수 처리부터 임시 디렉터리에서 검증한다. 실제 채움 명령은 실행하지 않았다.

**[처리담당]** 대기.

#### RV-203 — 장애 하네스 판정이 필수 실패를 건너뜀 (P2)

**[리뷰어]** F2a는 Gap 개수를 관측값으로만 넣어 0이어도 stream 생존·이후 기록 수신으로 통과한다. TASK-024의 Gap 통지 계약에 대해 양수 단정이 필요하다. `common_checks`는 heap 표본이 비면 두 필수 검사 자체를 생략하고, 정상 종료가 필요한 F1/F2/F4도 `code`와 end의 `failures`를 검사하지 않는다. Rust 하네스는 RT 힙 사용·producer 오류를 `failures`에 남기지만 `success`에는 logger shutdown 결과만 넣는다. F3a/b의 예상 출력 실패와 producer 실패도 따로 판정해야 한다. 필수 bool 키가 모두 존재할 때만 Pass로 두고, 공통 검사에 프로세스 결과·힙 표본·producer 실패를 포함한다. F5는 강제 종료라는 특성에 맞춰 종료 전 확보할 수 있는 공통 지표를 명시한다.

**[처리담당]** 대기.

#### RV-204 — F5 내구성 하한과 종료 원인 (P2)

**[리뷰어]** stdout 수신 시각에서 `since_sync_ms`를 빼고 300 ms 간격으로 묶으면 전달 지연이 sync 시각을 움직인다. 하나의 완료가 여럿으로 세어지거나, 마지막 완료 전 이미 sync된 기록 일부가 유실돼도 직전 완료 이전 표본만 만족하면 통과한다. 수락 수는 아직 큐에 있는 기록도 포함한다. 또한 `soak.wait(30)`의 종료 코드를 확인하지 않아 먼저 종료한 프로세스도 검사 대상으로 처리된다. sync 완료 시점의 파일별/채널별 확정 수나 시퀀스를 기록하고, 의도한 SIGKILL을 확인한다. SIGKILL은 페이지 캐시를 버리지 않으므로 이 증거의 범위는 프로세스 종료 뒤 읽을 수 있는 데이터다. EV-123에 보고한 6회 판독은 관측으로 유효하며 마지막 완료 sync 내구성의 단정은 별도 근거가 필요하다.

**[처리담당]** 대기.

#### RV-205 — 혼합 하네스의 워커 중도 종료 (P2)

**[리뷰어]** 상태 프로세스가 timeout이면 `Status.run`이 예외로 끝나고 실패 표본을 남기지 않는다. 성공 1회 뒤 timeout을 mock으로 주입하자 `status_all_answered` 조건이 true였다. `Subscriber`도 첫 기록 이후 예외가 나면 같은 문제가 있고, `finish`는 stdout이 조용한 자식에 stop을 전달하지 못한 채 join 제한을 넘겨 파일을 닫는다. 종료 제외 경계도 실제 서버 종료가 아닌 드라이버의 추정 시각이다. 워커 예외·종료 원인·생존 기간을 판정에 올리고, 자식 종료는 stdout 읽기와 독립적으로 수행한다. 정상 cap 재접속은 계속 허용한다.

**[처리담당]** 대기.

#### RV-206 — 혼합 하네스의 보존·파일 조회 판정 (P2)

**[리뷰어]** `judge_runs([[4999,4999]],5000,[{"ok":True,"control":2500}])`가 true다. 최종 제어 1건만 있어도 통과하며, frame suffix는 호출하지 않는다. 따라서 마지막 frame이나 아직 보호 중이어야 할 창이 통째로 사라져도 판정이 잡지 못한다. `reader`의 회차 성공은 파일을 하나도 열지 않았어도 가능하고, 읽을 파일에서 `LossReader::new`나 iterator가 에러를 내도 [soak.rs:1119](../../../crates/dot-logger/examples/soak.rs#L1119)의 `if let Ok`/`flatten`이 이를 빼 버린다. 두 채널의 tail과 현재 유효한 보호 창을 검증하고, 조회 레코드·파일 수와 손실 디코드 에러를 판정에 올린다. 링 정상 삭제로 파일을 놓친 경우는 이미 있는 `vanished` 지표로 계속 구분한다.

**[처리담당]** 대기.

#### RV-207 — M·J 측정 스레드 종료 경로 (P2)

**[리뷰어]** M의 한 스레드가 CPU affinity나 FIFO 설정을 실패하면 `start.wait()`에 오지 않는다. 다른 스레드가 고정 인원 barrier를 기다리고 scope가 끝나지 않는다. CPU 일부가 제외된 cpuset에서 성립하는 경로다. 기존 perf의 실패 전파형 `Gate::after`를 재사용할 수 있다. J는 측정 스레드의 `join`이 패닉을 돌려주면 `?`가 `stop.store`를 건너뛴다. IO 부하를 켜면 loader가 종료 신호를 받지 못한 채 scope가 대기한다. 정상·오류·패닉 모두에서 먼저 stop을 게시한 뒤 결과를 반환하고, 작은 오류 주입 시험으로 유한 종료를 확인한다.

**[처리담당]** 대기.

#### RV-208 — 지연 러너의 입력과 결과 증거 (P2)

**[리뷰어]** 커널 확인은 sysfs와 config를 모두 읽었을 때만 realtime으로 보고, 파일이 없거나 읽기 실패면 generic으로 기록한다. 러너는 현재 checkout의 HEAD와 `crates scripts` 변경 상태를 쓰지만 실행 파일의 출처는 기록하지 않는다. 이전 release 바이너리, 또는 `Cargo.lock` 변경도 clean 출력과 공존할 수 있다. `reps=0`이면 헤더만 쓰고 끝나며, 하네스가 JSON으로 내는 `success=false`도 러너 종료 값에 반영되지 않는다. 미확인 커널은 실패시키고 양수 행 수·행별 성공과 표본 수를 확인하며, 바이너리 hash/빌드 revision과 잠금 파일까지 기록한다. CPU 2를 확인하는 governor 가드는 M 행에서 사용하는 다른 CPU까지 확인하지 않으므로 전체 governor가 다를 때는 행 조건을 검증하거나 메타데이터 범위를 명시한다.

**[처리담당]** 대기.

#### RV-209 — README의 지연 인과 해석 (P2)

**[리뷰어]** 10 ms 깨어남 p99는 네 조건 모두에서 기록 0건·50건·SSD 부하가 같은 버킷이다. 1 kHz RT `performance`에서 p99 중앙값은 각각 4.479 / 5.247 / 5.503 µs이며, 최대값도 변한다. 이 정도 차이가 기록에 따른 인과 효과인지 판정할 자료는 없다. `Logging does not change`는 10 ms에서 관측한 p50/p99의 유사성으로 좁히고, EV-130은 해당 PC에서 감속과 동반된 큰 꼬리의 설명으로 쓴다. 10 ms p99.99는 6,000개 중 6,000번째 표본이므로 현 집계에서는 각 회차의 최대다. Warm 최대에도 감속이 겹쳤을 가능성과 RT 두 원시 헤더의 `dirty=true`를 근거에 표시한다. 표 숫자와 generic 5/6 집계는 일치한다.

**[처리담당]** 대기.

#### RV-210 — 시간에 무관한 handoff 회귀 (P3)

**[리뷰어]** 새 단위 시험은 `next`의 queued/in-flight/confirmed 세 조건을 고정한다. `handoff` 통합 시험은 ACK가 확정돼 `Disconnected`가 맞는 흐름도 허용하므로 CI 한 번으로 미확인 ACK 분기의 실행을 보장할 수 없다. 기존 preflight fixture나 고정 ACK RPC 상태로 연결을 닫고, `AcknowledgementUnknown`과 미확정 개수를 함께 단정하는 회귀가 RV-191 범위를 완결한다.

**[처리담당]** 대기.

#### RV-211 — generic 설정 검증 (P3)

**[리뷰어]** `rt-boot.sh generic`은 config에 RT 설정이 있는지만 검사한다. config 누락이나 grep 읽기 실패도 통과해 `next_entry` 변경까지 갈 수 있다. config 읽기 성공과 허용한 커널 이름을 먼저 확인하고 GRUB ID는 리터럴로 대조한다. 정상 경로는 boot log와 `next_entry` 대조가 있고, 기본 GRUB 설정은 쓰지 않는다.

**[처리담당]** 대기.

#### RV-212 — R 행의 시계 대조와 종료 결과 (P3)

**[리뷰어]** R 행의 `clock_pairs`는 측정 스레드를 join한 뒤 호출자에서 실행돼 헤더가 말하는 같은 CPU·동일 FIFO 조건의 대조가 아니다. 시계 대조를 측정 스레드로 옮기고, R 두 세션의 shutdown 성공을 각각 보존한다(`close`가 현재 공유 `row.success`를 두 번 덮는다).

**[처리담당]** 대기.

### 후속

F3 경로가 안전하게 처리되기 전에는 이 하네스로 채움 시험을 실행하지 않는다. 후속 개발 세션에서 장애·혼합 판정에 짧은 실패 fixture를 추가해 검출력을 먼저 확인한다. T8의 24시간 재검증에는 보완된 혼합 판정을 반영한다.

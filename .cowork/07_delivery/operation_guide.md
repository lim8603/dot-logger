# Operation Guide

> 운영서 — dot-logger를 쓰는 호스트와 운영 PC의 배포, 운영, 모니터링, 장애 대응, 복구 절차

---

## 문서 정보

| 항목 | 내용 |
| --- | --- |
| 프로젝트 | dot-logger |
| 대상 독자 | 로봇 호스트에 dot-logger를 넣어 배포·운영하는 개발자와 운영자 |
| 버전 | 0.1.0 (pre-release) |
| 기준 | 2026-11-21 main. 수치는 단일 Linux PC 실측이며 다른 장비의 보장이 아니다 |
| 관련 | 사용법은 [사용자 매뉴얼](user_manual.md), 검증 근거는 [verification_evidence](../05_verification/verification_evidence.md) |

---

## Placeholder 운용 기준

- 실측·승인된 값만 적는다. 정해지지 않은 운영 기준은 `미확정`으로 두고 §8에 올린다.
- dot-logger는 독립 서비스가 아니라 **호스트 프로세스에 들어가는 라이브러리**다. 서비스 기동·중지, 프로세스 감시, 로그 디렉터리 관리는 호스트 애플리케이션의 책임이다. 이 문서는 호스트가 지켜야 할 dot-logger 쪽 계약을 정리한다.

---

## 1. 배포 구성

### 시스템 요구사항

| 항목 | 최소 | 검증된 환경 | 비고 |
| --- | --- | --- | --- |
| OS | Linux (Tier 1) | Ubuntu, 커널 generic 7.0.0-43 / PREEMPT_RT | Windows·macOS는 Tier 2이며 미검증(CI 구성만 있음) |
| CPU | 미확정 | x86_64 PC 1대 | 장비별 측정은 `calibrate`로 프로필을 만든다 |
| RAM | 선언한 `Budget` + 호스트 여유 | 미확정 | 라이브러리 메모리는 시작 때 선언한 분할 안에서만 쓴다. Tokio·TLS·HTTP/2 버퍼는 별도 여유가 필요하다 |
| Disk | `Retention::ring` 상한 + 여유 | ext4 SSD | tmpfs는 영속 판정에서 제외한다 |
| Network | 원격 기능을 쓸 때만 | 유선 1 GbE LAN | Wi-Fi는 대용량 판정이 불안정했다(EV-104) |

### 의존성

| 소프트웨어 | 버전 | 용도 |
| --- | --- | --- |
| Rust | 1.85 이상(MSRV), 개발 1.96.0 | 빌드. `rust-toolchain.toml`이 1.96.0을 고정 |
| `Cargo.lock` | 저장소 고정값 | `--locked`로 빌드한다 |
| MCAP | 0.26.0 (vendor, 패치 포함) | 파일 형식. 패치 내용은 `vendor/mcap-0.26.0/VENDORED.md` |
| tonic / Tokio | 0.13.1 / 1.51.6 | `remote` feature의 gRPC |
| InfluxDB | v2 | `influxdb` feature를 쓸 때만 |
| Foxglove 또는 Lichtblick | 미확정 | 운영 PC의 시각화 |

---

## 2. 설치 및 배포

### 설치 순서

1. 호스트 `Cargo.toml`에 git 의존성을 추가하고 필요한 feature만 켠다([사용자 매뉴얼 §2](user_manual.md#2-시작하기)).
2. 대상 장비에서 출력 파일 시스템을 한 번 측정해 preset 프로필을 만든다(선택, `calibrate`).
   - `dot-logger calibrate`는 `--write-limit`까지 실제로 쓴다. 운영 디스크의 여유를 확인한 뒤 실행한다.
3. 호스트 프로젝트 아래에 로그 디렉터리(예: `.logger/`)를 만들고 `file_output`에 넘긴다.
4. 원격 조회가 필요하면 `LoggerBuilder::remote`로 서버를 켠다. loopback이 아닌 접근은 HTTPS 인증서와 토큰을 준비한다.
5. 운영 PC에는 CLI와 viewer를 빌드한다(`cargo build --locked -p dot-logger-cli -p dot-logger-viewer --release`).

### 환경 설정

| 설정 항목 | 기본값 | 설명 |
| --- | --- | --- |
| `Preset::control(hz)` / `Budget` | 없음(필수) | 메모리 분할, 동기화 간격, 종료 기한, writer 할당 |
| `Retention::ring(bytes)` | 없음(필수) | 출력별 디스크 상한 |
| `EventWindow` | 꺼짐 | 이벤트 전후 보존 구간, hold, 개수·바이트 한도 |
| `ChannelOptions::minimum` / `important_from` | 모두 기록 / 없음 | 채널별 최소 level, Important 기준 |
| `WriterQuota::important(n)` | 0 | Important 예약 slot. 0이면 포화 보호 없음 |
| `LoggerBuilder::telemetry()` | 꺼짐 | 1초 표본과 10/60/300초 창. 꺼져 있으면 지표는 `Unavailable` |
| 원격 서버 주소 | `127.0.0.1:0`(임의 포트) | 실제 주소는 `logger.remote_addr()` |
| `DOT_LOGGER_TOKEN` | 없음 | 원격 토큰(CLI·client·viewer). 인자로 넘기지 않는다 |
| 전원 경보 대응 | 꺼짐 | 고급 경로 `ControlOwner::enable_emergency`로 시작 전에 선언 |

### 배포 체크리스트

- [ ] `builder.plan()` 결과에 경고가 없거나, 경고를 알고 수용했다(예: `NoImportantReserve`)
- [ ] 로그 디렉터리가 영속 파일 시스템(tmpfs 아님)에 있고, ring 상한보다 여유가 있다
- [ ] 원격 서버를 켰다면 loopback 밖 접근에 TLS·토큰을 설정했다
- [ ] 종료 경로에서 `logger.shutdown(deadline)` 결과를 확인하고 실패를 보고한다
- [ ] 운영 PC의 viewer catalog가 호스트 채널 등록과 맞는다

---

## 3. 운영 절차

### 서비스 시작/중지

dot-logger는 호스트 프로세스 안에서 동작하므로 시작·중지는 호스트 코드가 한다.

```text
# 시작
let logger = Logger::builder()...start()?;     // 실패 시 StartError로 즉시 알 수 있다

# 중지
match logger.shutdown(Duration::from_secs(5))? { s if s.is_success() => ..., s => /* 미동기화 보고 */ }

# 상태 확인
logger.health()                                  // Running / Degraded / Failed
dot-logger status --endpoint http://127.0.0.1:PORT --json   // 원격 서버를 켰을 때
```

운영 PC 도구:

```text
# viewer + 대시보드 (loopback에서만 듣는다)
dot-logger-viewer --upstream https://ROBOT:PORT --ca robot-ca.pem --catalog catalog.json --dashboard 127.0.0.1:0
# 실행 중 level·샘플링 변경 (Manage 권한)
dot-logger config apply --endpoint ... --base-revision N --patch patch.json
```

### 로그(기록 파일) 관리

| 종류 | 위치 | 보존 | 비고 |
| --- | --- | --- | --- |
| 기록 파일 | `file_output` 디렉터리 아래 시스템이 이름 붙인 세션 디렉터리. 쓰는 중 `<sequence>.active`, 동기화·닫힘 후 `<sequence>.mcap` | `Retention::ring` 상한 안에서 닫힌 파일을 오래된 것부터 삭제. 상한은 segment 2개(최소 16 MiB) 이상이어야 하며 아니면 `StartError::Retention` | 설정 revision이 바뀌면 파일이 회전한다. logger는 자기가 게시한 닫힌 파일만 지운다 |
| 이벤트 보호 구간 | 같은 위치 | `hold` 동안 또는 `release_event`까지 삭제 보호 | 개수·바이트 한도가 있다 |
| 메타·손실 | 각 파일 안 `.logger/meta`, `.logger/loss` 채널 | 파일과 같다 | 별도 파일 없음 |
| 복구 결과 | `recover --output`에 지정한 새 경로 | 호스트가 관리 | 기존 경로를 덮어쓰지 않는다 |

---

## 4. 모니터링

### 볼 곳

| 수단 | 내용 |
| --- | --- |
| `logger.health()` | 전체 상태, 출력별 상태, CPU 배치 결과, 최근 1초 표본(2초 넘으면 Stale), tracing 손실 수 |
| `dot-logger status` / 대시보드 | `dot_logger.status.v1`: 최신 표본, 수집 개수, 출력별 complete/failed/dropped/Unknown/pending, 10/60/300초 창 |
| `dot-logger loss` | 파일에 저장된 누적 손실 카운터와 미관측 구간 |
| `dot-logger compare` | 설정 변경 전후 snapshot 비교(고급 경로 호스트) |

### 핵심 지표

| 지표 | 정상 | 경고 | 위험 |
| --- | --- | --- | --- |
| `HealthState` | `Running` | `Degraded` | `Failed` |
| 수집 거절(`try_log` 실패) 수 | 0 또는 예상 범위 | 미확정 | 미확정 |
| Important 거절 | 0 | — | 1 이상 |
| 출력 failed / Unknown | 0 | — | 1 이상 |
| 디스크 사용률 | ring 상한 이하 | 미확정 | 미확정 |

- 관측하지 않은 값은 0이 아니라 `Unavailable`로 나온다. 대시보드에서 빈 값을 정상으로 읽지 않는다.
- 경고 임계값은 장비와 사용처마다 다르다. 첫 사용처 적용(TASK-026)에서 정한다(§8 OQ-OPS-1).

---

## 5. 장애 대응

### 장애 등급

| 등급 | 정의 | 대응 시간 |
| --- | --- | --- |
| Critical | 필수 출력이 `Failed`, 또는 종료 결과가 `WorkerFailed`/`Unresponsive` | 미확정 |
| Major | `Degraded`가 지속되거나 Important 기록 거절 발생 | 미확정 |
| Minor | Normal 기록 일부 거절, 원격 구독 Gap | 미확정 |

### 장애 대응 절차

1. **감지** — `health()`, `status`, 대시보드, 종료 결과
2. **분류** — 위 등급 판정. 손실 범위는 `loss`로 확인
3. **대응** — 아래 시나리오별 조치. 실패를 성공으로 바꾸는 재시도는 하지 않는다
4. **보존** — 원본 파일은 건드리지 않고 `recover`로 새 사본을 만든다
5. **기록** — 장애 시각, revision, 손실 카운터, 복구 결과 행을 남긴다

### 주요 장애 시나리오

| 시나리오 | 증상 | 원인 | 대응 방법 |
| --- | --- | --- | --- |
| 수집 포화 | `try_log` 거절 증가, `Degraded` | 입력이 예산·디스크 속도를 넘음 | 거절 수 확인 → preset/Budget 조정, 샘플링(`set_channel`), Important 예약 확인 |
| 디스크 가득(`ENOSPC`)·쓰기 실패 | 해당 출력 `IoFailure`, `Failed` | 공간 부족, 파일 시스템 오류 | 실시간 호출에는 전파되지 않는다. 공간을 확보한 뒤 호스트 재시작. 자동 재열기는 없다(B1) |
| worker 중단 | `worker_failed()` true, 종료 결과 `WorkerFailed` | panic 또는 치명 오류 | 새 기록은 거절된다. 호스트 재시작 후 `recover`로 마지막 파일 확인 |
| 프로세스 비정상 종료 | 닫히지 않은 마지막 파일 | crash, kill | `recover --manifest ... --output NEW`로 복구. 잘린 꼬리는 제외 구간으로 보고된다 |
| 종료 기한 초과 | `Unresponsive` | 느린 IO | worker는 정리를 계속한다. 결과를 성공으로 보지 않는다 |
| 원격 연결 끊김 | CLI exit 4, 구독 종료, `AcknowledgementUnknown` | 네트워크, lease 만료 | 자동 재연결·재전송 없음. 새로 구독하고 두 구독 사이를 불연속으로 다룬다 |
| 설정 변경 실패 | `revision_conflict` / `revision_busy` / `restart_required` | 낡은 기준 / 이전 변경 배출 중 / 메모리 변경 | 재조회 후 재시도 / 대기 후 재시도 / 재시작 |
| 전원 경보 | `PowerAlarm` | UPS 예고 | 선언된 순위대로 중요 파일을 동기화한다. 신호 발생부터 차단까지 3초, 여유 200 ms 프로필이 시뮬레이션으로 검증됐다(EV-043). 결과가 없으면 재기동 때 완료 불명으로 다룬다 |

---

## 6. 백업 및 복구

| 대상 | 주기 | 보존 기간 | 복구 방법 |
| --- | --- | --- | --- |
| 기록 파일 | 미확정(호스트 정책) | ring 상한 | 닫힌 파일은 그대로 복사. 닫히지 않은 파일은 `recover`로 새 파일 생성 |
| 설정(preset 프로필, manifest) | 변경 시 | 미확정 | 호스트 저장소에서 관리 |

복구 원칙:

- 원본은 읽기 전용으로 열고 바꾸지 않는다. 출력은 새 경로에만 쓴다.
- 활성 파일은 완전한 chunk 경계의 `safe_offset`이 있어야 한다. 손상 뒤를 추측하지 않는다.
- 출력은 파일 동기화되지만 상위 디렉터리는 동기화하지 않는다(`namespace_synced:false`).
- 예산(`max_segments`, `read_bytes`, `max_records` 등)을 넘으면 성공으로 보고하지 않는다. 앞선 출력은 점검용으로 남는다.

---

## 7. 가정 (Assumptions)

| ID | 가정 | 영향 |
| --- | --- | --- |
| A-1 | 호스트가 로그 디렉터리를 영속 파일 시스템에 두고 다른 프로세스가 기록 중인 파일을 바꾸지 않는다 | 복구·조회의 전제 |
| A-2 | 실시간 스레드는 `try_log`만 호출하고 `mark_event`·설정 변경은 일반 스레드에서 한다 | 실시간 무대기 보장의 전제 |
| A-3 | 측정 수치는 단일 PC 기준이다 | 다른 장비는 `calibrate`와 자체 측정 필요 |
| A-4 | 전원 경보 판정은 실물 UPS가 아닌 시뮬레이션 신호와 실제 파일 IO로 했다(ADR-011) | 물리 전원 소실 내구성은 주장하지 않는다 |

---

## 8. 미확정 운영 항목 (Open Questions)

| ID | 항목 | 질문 | 상태 |
| --- | --- | --- | --- |
| OQ-OPS-1 | 경고·위험 임계값 | 거절 수, 디스크 사용률, 대응 시간 기준 | TASK-026 첫 사용처에서 결정 |
| OQ-OPS-2 | 장기 운영 | 24시간 이상 연속 운영의 자원 추이 | TASK-024 미실행 |
| OQ-OPS-3 | 다른 OS | Windows·macOS 동작과 성능 | T3(공개 후 native 검증) 미실행 |
| OQ-OPS-4 | 쓰기 오류 후 재열기 | 디스크 복구 뒤 재시작 없이 이어 쓰기 | B1 범위 밖 |
| OQ-OPS-5 | 백업 주기 | 기록 파일 외부 보관 정책 | 호스트 정책, 미확정 |

---

## 9. 관련 근거 / 출처 (Evidence & Sources)

| ID | 근거 | 출처 문서 | 비고 |
| --- | --- | --- | --- |
| E-1 | 두 Rust 각 724 Pass, 5 ignored | [EV-109](../05_verification/verification_evidence.md#ev-109) | 2026-11-21 |
| E-2 | 커버리지 라인 93.69%, 함수 92.67%, 리전 91.08% | [EV-089](../05_verification/verification_evidence.md#ev-089) | 일정 판정 |
| E-3 | UPS 시뮬레이션: 신호→차단 3초, 여유 200 ms, 16조건 보호 완료 | [EV-043](../05_verification/verification_evidence.md#ev-043) | ext4 실측 |
| E-4 | P3 저장 부하(8 GiB 한도)와 P5 동시 조회 p95 5.03 ms | [EV-103](../05_verification/verification_evidence.md#ev-103) | 단일 PC |
| E-5 | 유선 LAN: 256 B × 10,000/s ACK p99 4.12 ms, 8 MiB 115.9–116.4 MB/s | [EV-105](../05_verification/verification_evidence.md#ev-105) | 1 GbE |
| E-6 | ENOSPC·IO 장애 격리 계약 | [API §4.4](../03_design_artifacts/interface_contract.md#44-디스크-공간-부족enospc-및-io-장애의-fail-safe-격리) | 설계 |
| E-7 | CLI·viewer·client 사용과 한도 | 공개 README 3종 | 2026-11-21 S2 실행 확인 |

# Release Note

> 릴리즈 노트 — 각 릴리즈의 변경 사항, 신규 기능, 버그 수정, 알려진 이슈를 기록한다

---

## 문서 정보

| 항목 | 내용 |
| --- | --- |
| 프로젝트 | dot-logger |
| 버전 | 0.1.1 (패치 미리보기). 이전 0.1.0 (첫 릴리즈, 미리보기) |
| 릴리즈 일자 | 0.1.1: 2026-11-28 예정 — 주석 태그 `v0.1.1`은 CI Pass와 Human 확인 뒤([TASK-041](../04_implementation/tasks/TASK-041.md)). 0.1.0: 2026-11-22 — 주석 태그 `v0.1.0`, GitHub pre-release([TASK-037](../04_implementation/tasks/TASK-037.md), Human 「CI 통과하면 진행해」) |
| 상태 | **초안.** 2026-11-22 TASK-036·T3 결과로 배포 대상·알려진 이슈를 갱신했다. 외부 공개본은 저장소의 `CHANGELOG.md`(영어, FR 번호 없음) |

---

## 작성 기준

- 실제 배포된 변경을 기준으로 짧게 적는다. 내부 작업 메모는 옮기지 않는다.
- 내용이 없으면 `없음`으로 적는다.
- 사용자 영향, 운영 영향, Breaking Change, 후속 조치를 드러낸다.

---

## 버전·호환 정책

2026-11-21 Human 결정(1 A·2 권장대로·3 미리보기·4 TASK-026 뒤·5 권장대로). 계획 단계와 조건은 [MS-010](../04_implementation/milestones/MS-010.md)에 있다.

- **버전 규칙:** SemVer. 1.0 전까지 워크스페이스의 공개 crate(`dot-logger`·`-client`·`-cli`·`-viewer`, 그리고 vendor 별칭 crate)는 같은 버전(`workspace.package.version`)을 쓴다. 0.x에서 호환이 깨지는 변경은 minor(0.1 → 0.2), 그 밖은 patch.
- **형식·프로토콜 버전은 crate 버전과 따로 둔다:** 파일 `dot-logger/standard/2`·manifest v2·손실 보고 v2, 원격 gRPC `PROTOCOL_MAJOR`, viewer WebSocket v1·대시보드 v1. 형식이 바뀌면 형식 버전과 crate minor를 함께 올리고, 각 릴리즈에 호환표를 적는다.
- **MSRV:** 현재 1.85. 올리면 minor로 취급한다.
- **배포 형태:** v0.1.x는 GitHub의 미리보기(pre-release)와 주석 태그(`v0.1.0`·`v0.1.1`)로 내고, 사용자는 git `tag = "v0.1.1"`처럼 의존한다. crates.io 게시는 TASK-026 뒤. 1.0은 Gate 5 Pass 뒤.
- **저장소 안 변경 기록:** 공개용 `CHANGELOG.md`(영어)를 두고, 이 문서는 한국어 작업 문서로 유지한다.

---

## 릴리즈 이력

### v0.1.1 — 2026-11-28 (패치 미리보기)

#### 핵심 요약

- v0.1.0 알려진 이슈 K1(macOS·Windows에서 facade 파일 출력이 `start()`부터 `Unsupported`)을 고친 패치다(ADR-009 개정 2, `9f687dd`·`ad11ba3`).
- 검증 범위에 24h soak(EV-120), 4시간 혼합 실행(EV-126), 장애 주입 F1~F5(EV-123), 세 OS 간 파일 교환(EV-119)을 더했다.
- API 호환이 깨지는 변경은 없다(patch). 형식·프로토콜 버전은 0.1.0과 같다.

#### 릴리즈 범위

| 항목 | 내용 |
| --- | --- |
| 관련 Milestone / Intent | INT-001 / MS-010 1a(TASK-041). 포함 작업: TASK-024·038·039·040·042·045~047, REV-072·073 조치 |
| 배포 대상 | GitHub pre-release와 주석 태그 `v0.1.1`, git 의존성 `tag = "v0.1.1"`. crates.io 미게시 |
| 버전 | 워크스페이스 공개 crate와 vendor 별칭 crate `dot-logger-mcap` 모두 0.1.1. MCAP 헤더 `library`는 `dot-logger/<crate 버전>`(이전 `0.1.0` 고정 문자열) |
| 외부 공개본 | `CHANGELOG.md` `[0.1.1]` 절(영어) |

#### 신규 기능 (Features)

| ID | 기능 | 사용자/운영 영향 | 관련 근거 |
| --- | --- | --- | --- |
| R12 | `diagnostics::available_bytes` — Linux·macOS·Windows 여유 공간, 없는 경로는 모든 OS에서 오류 | 세 OS에서 같은 방식으로 공간을 본다 | ADR-009 개정 2 / REV-072 RV-195 |
| R13 | `ReceiverConfig::reservation_bytes`(client)·`Reassembler::reservation_bytes`(core) | 구독 수 × 이 값으로 `LiveBudget`·`QueryBudget`을 잡는다(macOS 잠금 바이트 포함) | REV-072 RV-199 / `e20aef4` |
| R14 | `influxdb_write` 예제, `scripts/check-influxdb.py --example`·`--server v3` | 고급 경로 InfluxDB 쓰기·조회 사용법과 InfluxDB 3 Core 확인 | TASK-045·046 / EV-133·134 |

#### 개선 사항 (Improvements)

| ID | 내용 | 기대 효과 | 관련 근거 |
| --- | --- | --- | --- |
| I5 | README·`adapters::influxdb`: InfluxDB 출력은 고급 경로이며 기본 `Logger`에는 DB 출력이 없음, 서버 버전 표(3 Core는 쓰기 동일·Flux 조회 없음·형 충돌 묶음 전체 거절) | 지원 범위를 사실대로 안다 | EV-134 |
| I6 | README 지연 분포 표(`try_log`, governor 2종) | RT 수집 비용을 숫자로 본다 | TASK-042 / EV-129·130 |

#### 버그 수정 (Bug Fixes)

| ID | 내용 | 영향 범위 | 관련 이슈 |
| --- | --- | --- | --- |
| F1 | macOS·Windows facade 파일 출력 `start()` `Unsupported` — 여유 공간을 Unix `statvfs`·Windows `fs4`로, Windows는 디렉터리 sync 생략(NTFS 저널, 로컬 NTFS 권장) | macOS·Windows 파일 출력 | v0.1.0 K1 / ADR-009 개정 2 / EV-119 |
| F2 | macOS 메모리 회계 — Apple std가 첫 잠금 때 할당하는 잠금 저장소(64 B)를 facade·원격 hub·receiver 예약에 포함 | macOS에서 예약이 실제보다 작던 문제. 0.1.0 공식에 딱 맞춘 예산은 조금 늘려야 할 수 있다 | REV-072 RV-199 / `156bd68`·`e20aef4` |
| F3 | MCAP 헤더 `library`가 버전과 무관하게 `dot-logger/0.1.0`이던 것 | 파일 출처 표시 | TASK-041 |
| S1 | 보안: `Cargo.lock` `url` 2.5.9(RUSTSEC-2026-0592, `reqwest` 경유, `influxdb` feature). 의존성은 범위라 자체 lock이 있는 사용자는 `cargo update -p url` | `influxdb` 사용 시 | `f30ae5a` |

#### Breaking Changes

- 없음. 단 F2로 macOS 예약 바이트가 늘어 경계에 딱 맞춘 예산은 `Limit`이 될 수 있다.

#### 알려진 이슈 (Known Issues)

| ID | 내용 | 심각도 | 우회 방법 |
| --- | --- | --- | --- |
| K1' | Windows·macOS 성능 실측 없음(CI·파일 교환만) | 낮음 | 운영 전 대상 OS에서 자체 확인 |
| K2' | 「회전·조회·전송 포함 24h」는 1.0.0 직전 재검증(T8)에서 충족 예정. 현재 근거는 24h 기록·회전(EV-120)과 4시간 혼합(EV-126) | 낮음 | — |
| K3 | 실사용처 적용 전 | 중간 | 예제로 통합 지점 확인 |
| K4 | 전원 경보는 시뮬레이션 신호 검증 | 중간 | 장비별 확인 |
| K5 | `paste` 유지보수 중단 경고 | 낮음 | 상위 갱신 시 재확인 |
| K6 | 이전 `standard/2` 일부 파일 Lichtblick 1970 시작 | 낮음 | `recover` |
| K7 | 재열기·압축·원격 일시정지/분할 범위 밖(B1) | 낮음 | 재시작 |
| K9 | 원격 구독 조용한 단절 미감지(keepalive 없음, EV-123 F4) | 중간 | 명령 기한(`--timeout-ms`) |
| K10 | 실제 ENOSPC 뒤 `IoPoisoned` 고착, RT 수락은 계속하고 실패로 집계(EV-123 F3b) | 중간 | 공간 회복 뒤 재시작 |
| K11 | InfluxDB 3 Core에서 `query_numeric` 불가(Flux 없음, EV-134) | 낮음 | 조회는 InfluxDB 2.x |

#### 업그레이드 / 배포 메모

- `tag = "v0.1.0"` → `tag = "v0.1.1"`. 코드 변경은 필요 없다(macOS 예산 경계는 Breaking Changes 참조).

### v0.1.0 — 2026-11-22 (미리보기)

#### 핵심 요약

- 로보틱스용 Rust 로그·텔레메트리 라이브러리의 첫 릴리즈 후보다.
- 실시간 스레드는 기다리거나 할당하지 않고 기록을 넘긴다. 기록은 표준 MCAP 파일로 남고, 모든 자원에 한도가 있으며, 남지 못한 기록은 개수로 보고된다.
- 라이브러리, `dot-logger` CLI, Rust 클라이언트, Foxglove bridge와 운영 대시보드(viewer)를 함께 낸다.
- pre-release다. API가 바뀔 수 있다. 실측은 Linux 한 대뿐이고, Windows·macOS는 CI 시험만 통과했다.

#### 릴리즈 범위

| 항목 | 내용 |
| --- | --- |
| 관련 Milestone / Intent | INT-001 / MS-001~006·008·009 Done, MS-007 진행 중(장기·OS 간 파일 교환·실사용처 잔여), MS-010 1단계 |
| 배포 대상 | GitHub pre-release와 주석 태그 `v0.1.0`, git 의존성 `tag = "v0.1.0"`(공개 저장소 `lim8603/dot-logger`, 2026-11-21 공개). crates.io 미게시(`publish = false`, TASK-026 뒤) |
| 라이선스 | `MIT OR Apache-2.0`. vendor `mcap` 0.26.0은 MIT(Foxglove) |
| 도구 체인 | MSRV Rust 1.85, 개발 1.96.0 |
| 비고 | 성능 수치는 단일 Linux PC 실측이며 보장이 아니다 |

#### 신규 기능 (Features)

| ID | 기능 | 사용자/운영 영향 | 관련 US/FR |
| --- | --- | --- | --- |
| R1 | 실시간 안전 수집 — `try_log` 무대기·무할당, 포화 시 개수 집계, Important 예약 slot | 제어 루프에 로깅을 넣어도 지연이 늘지 않는다 | US-001 / FR-001·008·009·012 |
| R2 | 메모리·큐·파일 예산 선언, 제어 주기 preset, `plan()` 미리보기, 장비 측정 프로필(`calibrate`) | 자원 사용이 시작 전에 정해진다 | US-002 / FR-007·014·015(측정 preset만) |
| R3 | native MCAP 저장(`dot-logger/standard/2`) — 사용자 schema·encoding 유지, `.logger/meta`·`.logger/loss` 채널 | Foxglove·Lichtblick·`mcap` 도구로 변환 없이 읽는다 | US-003 / FR-002·003·013 |
| R4 | 보존 — ring 상한, 이벤트 전후 보존(`mark_event`), 설정 revision 경계 회전 | 디스크 상한 안에서 사건 구간을 지킨다 | US-002 / FR-003·011 |
| R5 | 실행 중 level·샘플링 변경(`set_channel`, `config apply`) | 재시작 없이 기록량을 조절한다 | US-004 / FR-017 |
| R6 | 중단 파일 복구(`recover`)와 누적 손실 카운터(`loss`) | 비정상 종료 뒤 남은 기록과 손실 범위를 확인한다 | US-002 / FR-003 |
| R7 | gRPC 서버(구독·조회·재생·상태·완료), CLI, Rust 클라이언트, 묶음·파이프라인 ACK | 실행 중 기록을 원격으로 보고 받는다 | US-003 / FR-005·006·010 |
| R8 | viewer — Foxglove WebSocket v1 bridge(실시간·기록 재생), 읽기 전용 운영 대시보드(상태·비교·점검 계획) | 운영 PC에서 시각화와 상태 확인 | US-003·004 / FR-006 |
| R9 | 운영 — 텔레메트리 창(10/60/300초), 전후 비교, 파일 시스템 점검(`check`) | 설정 변경 효과와 장비 상태를 수치로 본다 | US-004 / FR-014·016·017 |
| R10 | 전원 경보 긴급 기록(고급 경로, 기본 꺼짐) | UPS 예고 시 중요 파일을 순위대로 동기화한다 | FR-018 |
| R11 | `tracing`/`log` 입력, InfluxDB v2 출력 | 기존 로깅·DB와 연결한다 | FR-002·004 |

> 관련 FR 번호는 작업 추적용이다. 릴리즈 시 외부 공개본에서는 뺀다.

#### 개선 사항 (Improvements)

| ID | 내용 | 기대 효과 | 관련 근거 |
| --- | --- | --- | --- |
| I1 | 손실 checkpoint에서 0인 행을 생략 | 대용량 부하에서 framing 25.90 GB → 0.94 GB | EV-103 |
| I2 | 묶음·파이프라인 ACK(64건 또는 5 ms, 동시 4 RPC) | LAN 소형 기록 10,000건/s 전량 ACK, p99 4.12 ms | EV-104·105 |
| I3 | 하드웨어 가속 CRC-32(crc32fast) | 8 MiB gRPC 115.9–116.4 MB/s, 원시 TCP의 99–100 % | EV-105 |
| I4 | Lichtblick 호환 — 등록을 메시지 chunk 앞에 배치 | 새 파일은 Lichtblick 색인 재생이 바로 된다 | TASK-031 |

#### 버그 수정 (Bug Fixes)

| ID | 내용 | 영향 범위 | 관련 이슈 |
| --- | --- | --- | --- |
| 없음 | 첫 릴리즈라 이전 릴리즈 대비 수정 사항이 없다 | - | - |

#### Breaking Changes

- 없음(첫 릴리즈). 단, 개발 중 형식은 지원하지 않는다.
  - 파일 profile `dot-logger/standard/1`은 입력으로 받지 않는다.
  - 오프라인 manifest는 `schema_version: 2`만 받는다(v1·전역 `revision` 미지원).

#### 알려진 이슈 (Known Issues)

| ID | 내용 | 심각도 | 우회 방법 |
| --- | --- | --- | --- |
| K1 | Windows·macOS는 CI 시험(3 OS × 2 Rust, RT 할당 시험 포함, EV-116)만 통과했다. **v0.1.0은 두 OS에서 facade 파일 출력이 `start()`부터 `Unsupported`로 실패한다** — main(`9f687dd`·`ad11ba3`, ADR-009 개정 2)에서 고쳤고 v0.1.1(TASK-041)로 낸다. 수정 뒤 세 OS 간 파일 생성·읽기 교환 9/9 Pass(EV-119). 성능 실측은 미실행 | 중간(해당 OS 사용 시) | v0.1.1 또는 main 사용. 성능은 운영 전 대상 OS에서 자체 확인 |
| K2 | 24시간 연속 운영은 이 PC(RT 커널)에서 기록·회전 부하(약 2.56 MB/s)로만 검증했다(EV-120: 손실·거절·deadline miss 0, 디스크 ≤ 링, fd·heap 증가 없음). 조회·전송을 섞은 장시간과 generic 커널 장기 실행은 미실행 | 낮음 | 운영 부하가 다르면 자체 장시간 시험. TASK-040 예정 |
| K3 | 실제 애플리케이션 적용 전(첫 사용처 dot-probe 적용은 마지막 단계) | 중간 | 예제(`dot_probe_*`)로 통합 지점 확인 |
| K4 | 전원 경보는 실물 UPS가 아닌 시뮬레이션 신호로 검증. 물리 전원 소실 내구성은 주장하지 않음 | 중간 | 장비별 확인 |
| K5 | 간접 의존성 `paste`가 유지보수 중단으로 보고됨(RustSec 경고 1건, 취약점 0) | 낮음 | 상위 의존성 갱신 시 재확인 |
| K6 | 일부 이전 `standard/2` 파일은 Lichtblick이 1970년 시작 범위로 보여 주며 미리 읽기 실패 | 낮음 | `recover`로 호환 사본 생성 |
| K7 | 쓰기 오류 후 자동 재열기, 압축 쓰기, 원격 일시정지·분할 등은 범위 밖(B1) | 낮음 | 재시작으로 대응 |
| K8 | crates.io 미게시 | — | git 의존성 `tag = "v0.1.0"` 사용 |

#### 업그레이드 / 배포 메모

- 첫 릴리즈라 업그레이드 절차는 없다.
- 라이브러리 의존성은 `^` 범위다(하한 = 검증한 lock 버전). 저장소 안 개발은 `Cargo.lock`과 `--locked`로 빌드한다. vendor MCAP은 별칭 crate `dot-logger-mcap`(lib 이름 `mcap`)으로 들어가 외부 프로젝트에서도 패치가 적용된다(`vendor/mcap-0.26.0/VENDORED.md`, TASK-036).
- `cargo audit`은 path 패키지를 검사하지 않는다. MCAP 보안 공지는 수동으로 확인한다.
- 릴리즈 시 남은 일: 태그와 날짜(TASK-037 S3). 공개 전환은 2026-11-21 완료, 외부 공개본은 `CHANGELOG.md`로 작성(2026-11-22).

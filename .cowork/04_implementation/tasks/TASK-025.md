# TASK-025 — 사용 문서·MS 추적표·인수 정리

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-001·002·003·004·005 |
| 요구사항 | FR-001 NFR-004 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-21 |
| 다음 액션 | S3 — `07_delivery` 사용자 매뉴얼·운영서·릴리즈 노트 작업 문서 |
| 의존 | TASK-033·TASK-032(MS-009). TASK-024 의존은 2026-11-15 해제 |
| 예상 소요 | 1~2 인일, 장비/리뷰 대기 제외 |

## 순서 변경 — 2026-11-15

Human 「권고대로」: TASK-024가 일정 판단에서 빠졌으므로(2026-11-15) 문서·추적표·인수 정리는 TASK-024를 기다리지 않는다. 순서는 TASK-033·MS-009 → **TASK-025** → 마지막 단계 TASK-024·026이다. TASK-024·026 결과(native·장기·실사용처)는 그때 이 문서들에 덧붙인다. TASK-025는 일정 판단 범위에 남는다.

## 작업 단계와 결정 — 2026-11-21

Human 결정(2026-11-21):
- README는 GitHub 공개를 고려해 새로 쓴다. 루트 README와 crate README 3개(cli·client·viewer)를 **영어로 통일**한다(「영어 통일 (Recommended)」).
- 라이선스·공개 범위는 이월 항목 T2에서 실제 공개 전환을 요청할 때 다룬다(「README만, T2는 나중에 (Recommended)」). README의 라이선스 절은 미정으로 두고, `.cowork/` 내부 문서 링크와 내부 요구 번호에 의존하지 않는다.

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S1 | 문서 검토 | README·crate README·예제에 적힌 명령을 실행하고, 실제 기능과 다른 곳의 목록을 만든다 | Done — 2026-11-21 아래 F1~F10 |
| S2 | 공개용 README | 루트·cli·client·viewer README를 영어로 새로 쓴다. S1 불일치를 고치고, 명령이 그대로 실행되는지 확인한다 | Done — 2026-11-21 `d4e0624`·`dca35d2` |
| S3 | 사용·운영 작업 문서 | `07_delivery`의 사용자 매뉴얼·운영서·릴리즈 노트(작업 문서, 한국어)에 재현 가능한 사용 예와 운영 한계를 정리한다 | Planned — 다음 |
| S4 | 추적표·게이트 | MS별 구현/TC/EV 링크를 정리하고 Gate 4·5를 따로 판정해 기록한다 | Planned |

공식 산출물(`docs/`) 생성은 이 Task 범위 밖이다. 필요해지면 언어·형식을 확인한 뒤 생성한다.

## S1 문서 검토 결과 — 2026-11-21

외부 장비 없이 실행할 수 있는 문서 명령 9개(레시피 L1·L2·L3 tracing·L3 remote, `dot-logger-api-consumer`, CLI `--help`, `dot_probe_bridge --plan`, `dot_probe_check --plan`, feature 목록)는 `c876395` 이후 HEAD에서 모두 exit 0이었다. 내용 불일치는 다음과 같다.

| ID | 위치 | 불일치 | S2 처리 |
|---|---|---|---|
| F1 | 루트·cli·client README | 한국어, viewer README만 영어 | 4개 모두 영어로 다시 쓴다 |
| F2 | 루트·cli·client README | `.cowork/` 내부 문서 링크와 내부 번호(NFR-008, API §, ADR, REV-022 보강 등)가 본문에 있음 | 링크와 번호를 빼고 내용만 남긴다 |
| F3 | 루트·client README, 예제 이름 | 외부 독자가 모르는 내부 사용처 이름(dot-probe)을 설명 없이 씀 | "첫 통합 대상인 로봇 애플리케이션을 본뜬 예제"로 설명한다. 예제 파일 이름은 유지한다 |
| F4 | 루트 README | 문제·구성 요소(라이브러리·CLI·client·viewer)·설치·지원 플랫폼·검증 수준·라이선스 절이 없음 | 공개 README 구조로 추가한다. 라이선스는 미정으로 둔다 |
| F5 | cli README | 예시 endpoint가 `127.0.0.1:50051`이지만 서버 기본 주소는 `127.0.0.1:0`(임의 포트) | 호스트가 출력한 주소를 쓰도록 `PORT` 자리표시로 바꾼다 |
| F6 | `examples/recipes/l3_remote.rs`, 루트 README | README는 "`dot-logger tail`로 구독"이라 하지만 레시피가 기록 직후 종료해 구독할 틈이 없음. 주소를 `Some(127.0.0.1:…)` 형태로 출력 | 구독 데모는 `live_host`로 안내하고, 레시피 출력은 주소만 나오게 고친다(줄 수 한도 안에서) |
| F7 | CLI `--help` | `config`, `check` 하위 명령에 설명이 비어 있음 | 설명을 추가한다 |
| F8 | rustdoc(공개 시 docs.rs) | 내부 번호가 문서 주석에 61곳(core 46) | 공개 표면의 주석에서 내부 번호를 걷어낸다 |
| F9 | 루트 README | viewer·운영 대시보드(MS-009)와 CLI 명령 개요가 없음 | 구성 요소 절에 넣는다 |
| F10 | 루트 README | 검증 범위가 내부 ADR 번호로 설명됨. CI는 구성만 있고 원격 실행 결과는 없음 | 검증 수준을 영어로 사실대로 적는다(로컬 Linux 실측, 다른 OS·장기·실사용처 미검증) |

## S2 결과 — 2026-11-21

- **코드(`d4e0624`):**
  - 공개 rustdoc과 예제 머리말 22곳에서 내부 번호를 걷어냈다(F8). 시험·private 주석 35곳은 `.cowork` 공개 여부(T2)가 정해질 때까지 남긴다.
  - CLI `config`·`check`에 도움말 설명을 넣었다(F7).
  - L3 remote 레시피는 `http://주소`를 출력한다(F6). 레시피 줄 수 +5로 한도(+6) 안이다.
- **README(`dca35d2`):** 루트·cli·client·viewer README를 영어 공개용으로 다시 썼다(F1~F5·F9·F10).
  - 내부 링크와 번호를 뺐다.
  - 루트 README는 기능, 구성 요소, 시작하기(git 의존성), 레시피, 선택 옵션, 실시간 조회와 대시보드, 검증 수준(단일 Linux PC, 측정값은 보장 아님), 라이선스 미정으로 구성했다.
  - CLI README는 기존 계약을 모두 옮겨 연결, 명령, 변경 결과, 점검, 오프라인·복구·손실, 출력, 종료 코드, 한도, 취소 순으로 정리했다.
  - client README는 일반 클라이언트 설명을 앞에 두고, dot-probe 예제는 "첫 사용처를 본뜬 예제"로 묶었다.
- **실행 확인:**
  - README 명령: L1·L2·L3 레시피, `live_host` + `dot-logger tail --record-bytes 65536 --limit 10`(10건 ACK), `dot_probe_pipeline --plan`, `dot_probe_tail --plan`이 모두 exit 0.
  - 검사 중 `tail` 기본 `--record-bytes`(8 MiB)가 데모 호스트 한도(64 KiB)를 넘어 `invalid_input`이 되는 것을 찾아, 두 README에 한도 안의 값을 지정하도록 적었다.
  - 상대 링크·앵커 검사 통과, README의 내부 번호·한국어·`.cowork` 링크 0개.
  - 두 Rust 각 724 Pass, Clippy·fmt·rustdoc·레시피 검사 Pass.
- `interface_contract.md`의 client README 링크를 새 절(`#dot_probe_tail`)로 고쳤다.

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `README.md; examples/; .cowork/07_delivery/; 추적표`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **문서 검토: 설치/통합/관측/조절/장애/보존 예제와 실제 기능의 불일치**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-001`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] 재현 가능한 사용 예·운영 한계·MS별 구현/TC/EV 링크·Gate 4/5 별도 판정.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-001](../../05_verification/test_case.md#tc-001). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

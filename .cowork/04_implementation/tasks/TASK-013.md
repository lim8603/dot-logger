# TASK-013 — Rust 클라이언트와 CLI

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-004 |
| Story | US-003·005 |
| 요구사항 | FR-001 FR-005 FR-006 FR-010 NFR-004 NFR-007 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-31 |
| 다음 액션 | EV-025 로컬 구현/리뷰 완료; native 공개 후(T3)·실앱/성능 관련 Task 후속. TASK-014는 별도 지시 후 |
| 의존 | TASK-011·TASK-012 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

실제 생성: `crates/dot-logger-client/`·`crates/dot-logger-cli/`. Rust native transport/수신/파일 API, 명시 CLI/예제/화면 계약까지 EV-025 로컬 검증했다.

첫 실패/리뷰 조건: **CLI JSON schema·exit code·query→replay 사용자 흐름과 Ctrl-C**.

- [x] 요구·설계와 연결한 공개 API/CLI 및 RV-039~045의 의도한 Red를 확인했다.
- [x] TC-007의 이 Task 로컬 하위 client/server·CLI·파일·취소/ACK 경로를 통과했다. native/실앱/성능은 별도다.
- [x] 두 Rust/16feature·실제 consumer/CLI·예제·schema·lint/doc/fmt와 영향 Miri32를 확인했다.

## 완료 기준

- [x] 로컬 native Rust client/server 왕복·read-only 파일·예제 및 dot-probe 화면 API 계약. 실제 화면 적용은 TASK-020/026, 다른 OS/native CI는 T3 잔여다.
- [x] EV-025/source29e55da·환경/명령/결과/미실행·MS 추적을 기록했다.
- [x] TLS/권한·모순/불량 응답·Gap/Unknown·실제 host IO 수명·stdout/출력·Ctrl-C/reader 정리를 검증했다.
- [x] Human 착수·수락/마감 판단 위임에 따른 AI 로컬 검증 수락, REV-021 Closed. 미실행 시험의 수락은 아니다.

테스트 정의: [TC-007](../../05_verification/test_case.md#tc-007). 제품 코드·현재 실행 결과: [EV-025](../../05_verification/verification_evidence.md#ev-025), 첫 단위 이력 EV-024. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 승인된 착수·첫 수신 수명 단위 — 2026-10-31 #007

Human 「승인」·모든 권한/작업 단위별 커밋·푸시 위임·「다음 작업 착수까지 진행」에 따라 TASK-012 EV-023 조치/마감 후 착수했다. 선행 TASK-011/012의 로컬 기능·최종 리뷰 수락을 사용하며 native/실측 잔여는 면제하지 않는다.

첫 단위는 RV-038 수명 지침을 클라이언트가 소비하도록 `ClientReceiver`를 구현했다. 실제 연결을 연 호스트가 `begin_connection`으로 opaque ConnectionId와 receiver를 만들고, push/세대 갱신/end/ACK 추출이 현재 identity를 검사한다. 서버 구독 ID를 재사용해도 오래된 연결의 조각/미인계 owner/ACK가 새 연결로 들어가지 않는다. consumer handoff는 기존 core의 generation/TTL/CRC·예산 경계를 사용하고 거절 시 원래 owner를 반환한다. disconnect는 미완성 버퍼를 정리하되 외부 보관 owner의 예약을 강제 회수하지 않는다.

- 기본 feature는 core-only, remote는 opt-in이다. client identity/receipt metadata 및 transport/consumer buffer는 host headroom이며 전체 RSS 상한을 주장하지 않는다.
- 공개 API 부재 compile Red → 통합3개 Green(재연결·세대/거절·잘못된/다른 연결), 두 Rust workspace/core consumer/lint/doc/fmt 및 Miri3×8=24 Pass. [EV-024](../../05_verification/verification_evidence.md#ev-024) source74개/검사12개·commit blob일치, 소스 `ded6372`.
- [REV-020](../../05_verification/reviews/REV-020_TASK-013_RECEIVER_START.md)는 이 첫 단위의 개발 자가 점검/위임된 수락으로 Closed다. Task 전체 완료/최종 사용자 경로 수락은 아니다. 위 DoD·CLI 사용자 흐름 체크는 미완료로 유지한다.

후속: 실제 tonic transport 연결·endpoint/TLS/token/버전·bounded 요청/stream decode, ACK를 검사한 바로 그 연결로 전송하는 수명, CLI JSON/exit/Ctrl-C, query→replay/오프라인 파일·예제/dot-probe API를 연결한다. 현재 begin/end는 소켓을 열고 닫는 API가 아니며 transport 재연결 시험이나 CLI 완료로 보고하지 않는다. 이미 추출/전송된 ACK·수락된 consumer 데이터는 취소했다고 추정하지 않는다. 실제 연결에 generation gate가 찼다면 종료 후 새 연결/receiver를 만들고 gate만 몰래 초기화하지 않는다.

## TASK-013 로컬 구현·검증 수락 — 2026-10-31 #008

Human 「TASK-013까지 진행할래?」·「계속 이어서 진행해줘」에 따라 남은 구현을 수행했다. 기존 권한 위임을 유지해 검증 뒤 AI가 로컬 구현을 수락했다. 이전 “transport/CLI 미구현” 문구는 #007 당시 이력이다.

- RemoteClient는 finite endpoint/CA/token·native request/reply·bounded codec·명시 TCP/TLS를 사용한다. one-shot connector/실제 IO guard로 자동 reconnect/retry를 제한하고 RemoteSubscription의 소비 인계 뒤 같은 연결로 ACK한다. 실패 ACK는 Unknown, 원격 취소는 host 실제 IO 종료가 아니다.
- open_snapshot은 원래 등록/예산·read-only 일반 파일·frozen prefix로 Snapshot<File>을 반환한다. 실제 Query/ReadBlob→Replay 제어와 offline timed replay를 연결했다.
- CLI는 schema_version1 NDJSON/사람용 출력·exit0/2/3/4·전역/개별 기한/출력 cap·권한/만료/미지원·협력 Ctrl-C를 제공한다. stdout write/flush 수락 전에는 ACK하지 않는다. status 미구현 지표는 Unavailable이며 ApplyConfig/RunCheck/Compare backend는 호스트의 Unsupported를 보존한다. config validate는 offline 선언/예약 검증이고 source scan/runtime 검증을 가장하지 않는다.
- 복구는 독립 create_new 파일·원본 bytes 보존·file sync·명시 namespace_synced:false, 오류 출력 보존이다. 단일 전체 immutable source만 지원하며 부분 active prefix는 Unsupported다. 기존 파일/원격 API 의미는 변경하지 않는다.
- [SDK/화면 계약](../../../crates/dot-logger-client/README.md)·[native 예제](../../../crates/dot-logger-client/examples/remote_query_replay.rs)·[CLI/선언 예제](../../../crates/dot-logger-cli/README.md)를 제공했다. 실제 dot-probe UI 적용은 후속 Task다.
- [EV-025](../../05_verification/verification_evidence.md#ev-025) source92개/검사15개 exit0·commit blob92개 일치. 두 Rust각16feature/workspace/default client/독립 consumer/allocator·clippy/rustdoc/fmt/proto 두 사본·CLI 선언 예제·cached audit·Miri32 Pass. transport7·CLI10·codec1·기존 수신 수명3을 포함한다. [REV-021](../../05_verification/reviews/REV-021_TASK-013_CLIENT_CLI.md) RV-039~045 조치·Closed다.

로컬 구현 DoD는 완료했다. native/다른 OS는 GitHub 공개 후(T3), 실제 앱 적용·LAN/RSS/PC 성능/RT/UPS는 각 Task의 잔여로 보존하여 Task 상태는 In Progress를 유지한다. MS-004/Gate4·5 전체 완료로 올리지 않는다. 다음 개발 후보는 TASK-014이며 이번 범위에서 자동 착수하지 않는다. 작업 단위별 소스/문서 분리 커밋·푸시는 위임에 따라 처리한다.

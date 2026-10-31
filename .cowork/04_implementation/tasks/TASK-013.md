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
| 다음 액션 | EV-024 수신 수명 첫 단위 Pass; 실제 transport/인증·CLI JSON/exit·query→replay/Ctrl-C 연결 |
| 의존 | TASK-011·TASK-012 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

실제 생성: `crates/dot-logger-client/`(첫 수신 수명 API). 후속 계획: `crates/dot-logger-cli/`(아직 미생성).

첫 실패/리뷰 조건: **CLI JSON schema·exit code·query→replay 사용자 흐름과 Ctrl-C**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-007`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] native client/server 왕복·오프라인 파일 경로·예제, dot-probe 화면 연동 API 계약.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-007](../../05_verification/test_case.md#tc-007). 제품 코드·첫 단위 실행 결과: [EV-024](../../05_verification/verification_evidence.md#ev-024). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 승인된 착수·첫 수신 수명 단위 — 2026-10-31 #007

Human 「승인」·모든 권한/작업 단위별 커밋·푸시 위임·「다음 작업 착수까지 진행」에 따라 TASK-012 EV-023 조치/마감 후 착수했다. 선행 TASK-011/012의 로컬 기능·최종 리뷰 수락을 사용하며 native/실측 잔여는 면제하지 않는다.

첫 단위는 RV-038 수명 지침을 클라이언트가 소비하도록 `ClientReceiver`를 구현했다. 실제 연결을 연 호스트가 `begin_connection`으로 opaque ConnectionId와 receiver를 만들고, push/세대 갱신/end/ACK 추출이 현재 identity를 검사한다. 서버 구독 ID를 재사용해도 오래된 연결의 조각/미인계 owner/ACK가 새 연결로 들어가지 않는다. consumer handoff는 기존 core의 generation/TTL/CRC·예산 경계를 사용하고 거절 시 원래 owner를 반환한다. disconnect는 미완성 버퍼를 정리하되 외부 보관 owner의 예약을 강제 회수하지 않는다.

- 기본 feature는 core-only, remote는 opt-in이다. client identity/receipt metadata 및 transport/consumer buffer는 host headroom이며 전체 RSS 상한을 주장하지 않는다.
- 공개 API 부재 compile Red → 통합3개 Green(재연결·세대/거절·잘못된/다른 연결), 두 Rust workspace/core consumer/lint/doc/fmt 및 Miri3×8=24 Pass. [EV-024](../../05_verification/verification_evidence.md#ev-024) source74개/검사12개·commit blob일치, 소스 `ded6372`.
- [REV-020](../../05_verification/reviews/REV-020_TASK-013_RECEIVER_START.md)는 이 첫 단위의 개발 자가 점검/위임된 수락으로 Closed다. Task 전체 완료/최종 사용자 경로 수락은 아니다. 위 DoD·CLI 사용자 흐름 체크는 미완료로 유지한다.

후속: 실제 tonic transport 연결·endpoint/TLS/token/버전·bounded 요청/stream decode, ACK를 검사한 바로 그 연결로 전송하는 수명, CLI JSON/exit/Ctrl-C, query→replay/오프라인 파일·예제/dot-probe API를 연결한다. 현재 begin/end는 소켓을 열고 닫는 API가 아니며 transport 재연결 시험이나 CLI 완료로 보고하지 않는다. 이미 추출/전송된 ACK·수락된 consumer 데이터는 취소했다고 추정하지 않는다. 실제 연결에 generation gate가 찼다면 종료 후 새 연결/receiver를 만들고 gate만 몰래 초기화하지 않는다.

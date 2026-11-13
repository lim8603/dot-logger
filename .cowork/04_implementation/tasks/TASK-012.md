# TASK-012 — gRPC 구독·ACK·관리 서버

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-004 |
| Story | US-003·005 |
| 요구사항 | FR-005 FR-006 FR-013 NFR-002 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-010·TASK-006 |
| 예상 소요 | 2~4 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: `proto/logger.proto`·`crates/dot-logger/src/adapters/remote/`, `tc_007_remote.rs`·독립 consumer/allocator·proto/feature 검사.

첫 실패/리뷰 조건: **느린 구독자·잘못된 조각·ACK 전 disconnect·인증 없는 비-loopback**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-007`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] 로컬 구현 범위: 제한 reassembly·Gap/Unknown·버전/권한·취소·receiver ACK, 독립 파일 경로 Written 유지(EV-022).
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 로컬 구현 수락·추가 조치 검증/마감 위임 완료. native/실측 수락은 별도다.

테스트 정의: [TC-007](../../05_verification/test_case.md#tc-007). 제품 코드·테스트 실행 결과: [EV-022](../../05_verification/verification_evidence.md#ev-022). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 착수 — 2026-10-31

Human 「권장대로 진행」으로 TASK-012 구현을 시작했다. TASK-006/010 로컬 구현·최종 리뷰 승인을 선행 기능 근거로 사용하며 native/PC 실측 잔여는 유지한다. §14의 bounded gRPC·수신 ACK·원본 opt-in·Gap/disconnect·version/권한을 구현한다. 기존 런타임/OS 네트워크를 RT 입력에 연결하지 않으며 원격 서버는 명시 bind/serve로 시작한다. 관리 명령은 유한 host inbox로 전달하고 실제 기능/권한 거절을 명시한다. 내부 query external 예약 authority를 공유해 adapter 합계를 제한한다. TLS는 채택된 tonic 계열의 TLS feature로 활성화하며 기술 계열/MSRV를 변경하지 않는다.

F-11 소비자: remote feature 서버/수신 재조립/관리 요청·query/replay host bridge·completion ReceiverAcknowledged/Unknown·routing 독립 파일 대상·독립 core consumer/allocator·16feature/MSRV. 전송 send와 ACK를 구별하고 실제 handler/reader 종료 전 요청 자원을 재사용하지 않는 경계를 시험한다. TASK-013 CLI·후속 관리 기능 자체·실제 LAN 성능/OS/native 수락은 별도다. 커밋/푸시는 요청하지 않았다.

## 구현·검증 결과 — 2026-10-31

- 원격 서버는 기본 disabled·loopback/read-only이며 명시 bind/serve로만 시작한다. tonic0.13.1의 tls-ring feature를 활성화하고 외부 주소는 실제 TLS+인증, read/manage 권한을 구분한다. 새 package/version 변경은 없다.
- RemoteHub/Subscription은 연결·stream·command·records/bytes·encoded message·age를 제한하고 QueryBudget의 external authority를 공유한다. max256KiB 조각·full identity/CRC·원본 opt-in·Gap(연속 transfer 범위/overflow 이유)·disconnect/Unknown·mono late 표시를 제공한다.
- Reassembler/ReceivedRecord는 동시 records/bytes/TTL·세대 gate·연속 offset/전체CRC를 검증하고 성공한 소비 인계에서만 Ack를 만든다. RemoteDelivery는 실제 guard를 보관해 수신 ACK 뒤 ReceiverAcknowledged, 종료/미확인은 Unknown을 게시한다. 네트워크 in-flight 또는 이미 인계된 데이터를 취소했다고 추정하지 않는다.
- 관리 unary는 host-driven CommandInbox와 native request/reply를 사용한다. Query/ReadBlob/Replay/GetCompletion은 host의 실제 primitive에 연결하며 host handles/권한/reader lifetime은 host가 보존한다. ApplyConfig/RunCheck/Compare의 후속 backend는 Unsupported로 명시한다. timeout/cancel로 host 소유 command/IO permit을 조기 반환하지 않고 reply type·encoded bytes·할당 capacity·요청 blob 길이를 검사한다.
- 공개 API 부재 Red 후 구현, RV-032~035의 malformed auth·늦은 이전 generation·reply capacity·overlong blob 행동 Red→Green을 기록했다. [REV-018](../../05_verification/reviews/REV-018_TASK-012.md) Resolved·Human 최종 리뷰 대기다.
- [EV-022](../../05_verification/verification_evidence.md#ev-022): source70개/검사11개 exit0. 두 Rust각16features/독립 core·remote consumer/allocator11·clippy/rustdoc/fmt·proto9message83field, 영향 Miri48 Pass. 원격13개 전용 계약/소켓 시험·실제 File Query→ReadBlob/Replay/Completion 및 ACK/Unknown·독립 파일 Written을 확인했다. hub 초기화+receiver256B/8MiB peak5744/8655984 ≤ 예약2668288/27833344B·잔여0, 기존 RT0/0 유지.

현재 구현은 host runtime/관리 primitive를 연결하는 원격 adapter다. 실제 backend worker/운영 보안 정책·CLI·dot-probe 통합·LAN/전체 runtime RSS/실제 성능·native/다른 OS 검증은 후속이다. TASK-012/MS-004 In Progress·Gate4/5 Not Ready, 소스/문서는 미커밋·미게시다. 최종 구현 리뷰 승인 뒤 다음 개발은 TASK-013이며 승인 전 Task를 바꾸지 않는다.

## 세션 마감·게시 — 2026-10-31

Human 「세션 마감해줘」·「커밋 푸쉬도 해줘야해」에 따라 EV-022 source70개/검사11개·Miri48·원본/뷰 로그16쌍의 hash 일치를 확인하고 동일 성공 시험은 반복하지 않았다. 소스 `5f0ad83`를 source/docs 분리·[skip ci] 커밋으로 origin/main 게시했으며 EV-022 commit blob70개와 원격 hash가 일치한다. [commit 대조](../../06_evolution/imported_context/task_012_2026-10-31/commit.json)·[게시 근거](../../06_evolution/imported_context/task_012_2026-10-31/source-push.json). 검증/승인/마감 문서는 별도 docs(cowork) 커밋으로 게시한다. 앞선 미커밋/미게시 본문은 검증 당시 이력이다.

공유/개인 상태와 세션 로그를 종료로 동기화했다. 마감/게시 요청을 별도 최종 구현 승인으로 확대하지 않아 REV-018 Resolved·TASK-012/MS-004 In Progress·Human 리뷰·native/실측 이월·Gate4/5 Not Ready를 유지한다. 다음 시작점은 TASK-012 최종 구현 리뷰이며 승인 뒤 TASK-013으로 진행한다. 새 Task는 자동 착수하지 않는다.

## REV-019 반입 — 2026-10-31

Human 「프로젝트 브리핑하고, 리뷰 반입하고, 승인 해야 하는 업무 보고해줘」에 따라 [REV-019](../../05_verification/reviews/REV-019_TASK-012_PROJECT_CODE_REVIEW.md)를 In Progress로 반입했다. RV-036은 기존 초과 연결 거절/한도를 유지하는 재폴링 대기 개선으로 수용, RV-037은 방어 보강으로 부분 수용하되 실제 공격·constant-time 보장은 미확정, RV-038은 연결별 receiver 수명 지침 보강으로 수용하고 gate 자동 삭제는 채택하지 않았다. 상세 근거·첫 검증은 REV-019 처리담당 영역을 따른다.

이번 요청에서 개선 제품 코드는 수정하지 않았다. 기존 EV-022 source70개 hash 일치·검사11개 exit0·Miri48 Pass를 확인했고 동일 시험은 반복하지 않았다. 개선 구현/회귀·문서 조치 후 REV-019를 Resolved로 올리고 REV-018과 함께 Human 최종 구현 수락을 받는 순서를 권고한다. 승인된 계약 안의 수정 세부사항은 A이며 별도 재승인 대상이 아니다. TASK-013 착수는 TASK-012 수락 및 다음 작업 지시 후다.

## REV-019 조치·수락 및 TASK-013 전환 — 2026-10-31 #007

Human 기존 로컬 구현 승인·모든 권한 위임·다음 착수 요청으로 REV-018 Closed를 반영하고, RV-036 timer 대기·RV-037 고정 credential 비교·RV-038 연결 수명 지침을 조치했다. [EV-023](../../05_verification/verification_evidence.md#ev-023) source70개/검사9개 exit0·두 Rust16feature·Miri25 Pass·commit blob70개 일치(`4f79119`) 후 AI의 위임된 마감 판단으로 REV-019도 Closed다. 이전 승인 대기/미구현 문구는 당시 이력이다. 로컬 Human 리뷰 완료를 전체 Task/TC·native/PC 수락 완료로 확대하지 않는다. 다음 TASK-013에 착수한다.

## REV-022 반입 — 2026-10-31 #010

[REV-022](../../05_verification/reviews/REV-022_TASK-013_INDEPENDENT_REVIEW.md)을 코드와 대조하여 Open→In Progress로 반입했다. RV-046~055 수용9건·부분 수용1건(RV-049 영향 조건 보정, 최적화 실측 후 판단)이다. 처리담당 영역에 방향·첫 Red/회귀·승인 경계를 기록했으며 구현/새 시험은 수행하지 않았다. 기존 EV-025 source92개와 현재 파일 hash 불일치0, EV-023/025의 기존 로컬 수락은 유지한다. 이번 반입으로 새 지적을 해결/마감하거나 native·실앱·성능을 수락하지 않는다.

다음 개발 순서는 RV-046/053(P2) 우선 조치 후 나머지 개선/회귀를 권고한다. RV-053 문서/클라이언트 세부는 기존 위임 안이며 heartbeat/lease wire 변경만 H/J 판단 후 진행한다. 자동 reconnect·재전송/구독 간 복원은 도입하지 않았다. 상세와 권한 근거는 REV-022 §7을 따른다.

## REV-022 조치 수락 — 2026-10-31 #011

Human 「권고대로 진행하자」에 따라 EV-026 source92개/검사14개·Miri32·소스b3c08bd를 로컬 검증/위임 수락했다. REV-022 Closed이며 상세는 REV §8과 API§14.5를 따른다. RV-049 포화 회복 실측은 TASK-022, RV-053 서버 lease 초과 유휴/재구독 연속성과 화면 불연속 표시는 TASK-020 후속이다. heartbeat wire 변경은 하지 않았다. 기존 EV-023/025는 이전 형상 근거로 보존하고 현재 조치는 EV-026을 사용한다. native T3·실앱/성능·Gate4/5 미완료는 유지한다. TASK-014는 이번 지시에 따라 착수한다.

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- native·다른 OS: [TASK-024](TASK-024.md)(T3, GAP-004)
- 원격 포화 회복 실측(REV-022 RV-049): 이월 T7
- 실사용처 적용: [TASK-026](TASK-026.md)(마지막 단계, 일정 판단 밖)

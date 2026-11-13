# TASK-010 — 시간·채널 조회와 active snapshot

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-004 |
| Story | US-003·005 |
| 요구사항 | FR-005 NFR-002 NFR-005 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-009 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: [`query/mod.rs`](../../../crates/dot-logger/src/query/mod.rs)·[`tc_006_query.rs`](../../../crates/dot-logger/tests/tc_006_query.rs), storage의 reader-owned pin 수명 연동.

첫 실패/리뷰 조건: **쓰는 중 incomplete chunk 비노출·cursor 만료·페이지 bytes/count 이중 제한**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-006`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] mono/real/source domain·다중 segment·원본 descriptor/읽기 핀·잘못된 파일 bounds.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] REV-010 RV-012: 오류/취소/리더 중단 경로에서도 `ReadPin`을 보관하고, 실제 읽기 종료 확인 후 반환하는 수명 관리를 검증한다. 고립 회수 조건은 기한만으로 삭제를 허용하지 않는 기존 계약 안에서 검토한다.
- [x] Human 리뷰 완료 — 2026-10-31 최종 구현 승인. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-006](../../05_verification/test_case.md#tc-006). 제품 코드·테스트 실행 결과: [EV-016](../../05_verification/verification_evidence.md#ev-016). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 리뷰 후속 — REV-010 RV-012 (2026-10-30)

TASK-008의 저수준 `ReadPin`은 만료/Drop으로 삭제 보호를 해제하지 않는다. 토큰을 유실하면 catalog 생존 기간 동안 pin slot과 파일 보호가 남아 `PinLimit`/`Capacity`로 이어질 수 있다. RV-012는 현 계약 유지·`must_use`/문서/회귀 보강으로 처리했으며, 위험 자체를 제거한 것으로 보지 않는다. 본 Task에서 리더가 토큰을 소유하고 오류/취소에도 실제 IO 종료를 확인한 뒤 반환하는 경로를 구현한다. 비정상 리더의 종료 확인·안전한 회수 조건을 함께 검토하고, 공개 소유권/오류 의미를 바꾸는 정책이 필요하면 별도 H/J 결정으로 제시한다. 기한 경과만으로 강제 해제하거나 진행 중인 읽기를 종료한 것으로 간주하지 않는다. 이 문장의 최초 기록 시에는 미착수였으며 2026-10-30 후속 진행 지시로 착수한다. 근거: [REV-010](../../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md), [인터페이스 §13.4](../../03_design_artifacts/interface_contract.md#134-task-008-회전보존-api--2026-10-30).

## 착수 — 2026-10-30

Human 「승인, 커밋 푸쉬하고 다음 작업 진행하자.」로 TASK-009 최신 구현 승인·소스fe03468/문서aee3830 게시 후 착수한다. native GAP-004·TASK-008 최종 리뷰는 별도 유지한다. 원본 불변 범위/complete chunk·시간/채널/레벨 필터·결정적 정렬·페이지/cursor/blob 한계와 RAII 읽기 수명을 TDD로 구현한다.

## 구현 결과 — 2026-10-30

- `QueryBudget`은 external partition의 snapshot 예약 합계를 원자적으로 제한한다. QueryConfig는 frame/source/segment/record/producer/metadata·page count/bytes·inline/blob·누적 read 한도를 명시하며 모든 산술/길이를 검사한다. OS cache·호스트 source adapter/allocator 내부는 별도다.
- `Snapshot`은 고정 session/revision·동일 등록·독립 read-only descriptor들의 complete safe offset만 스캔한다. recovery와 Fields/Validation/CRC/frame 검사를 crate-private로 공유하여 chunk 전체 검증 뒤 metadata와 체크된 payload 파일 위치만 index한다. 손상/한도 실패 시 부분 snapshot을 반환하지 않는다.
- mono/real/source domain의 native ticks와 `[from,to)`·채널/최소 level 필터 및 결정적 tie 순서를 지원한다. 도메인 누락/불일치·암묵적 UTC 변환은 거절한다. cursor는 snapshot/filter/위치를 고정하며 page는 metadata+inline bytes/count 이중 한도를 적용한다. 큰 payload는 snapshot-local BlobHandle로 반환하고 제한된 원본 range만 읽는다. wire encoding 상한/재생/live는 후속이다.
- `SegmentLog::active_query_source/query_source`는 reader에 permit을 이전한다. 실제 reader Drop 뒤 Release 게시·catalog Acquire 확인을 사용하며 active pin은 회전된 같은 파일을 계속 보호한다. 오류/취소/만료는 진행 IO를 강제로 종료하거나 pin을 일찍 해제하지 않는다. 낮은 ReadPin 분실/forget은 여전히 보호를 남긴다. catalog보다 permit이 오래 살아도 reservation은 유지한다.
- 최초 미구현 API에서 행동 Red6개 후 구현했고 현재 query9개·기존 retention에 reader close/error/cancel/expiry 및 active 회전 수명2개를 추가했다. RV-018은 만료 후 과거 now로 재개되는 반례 Red 뒤 최대 관측 시각으로 수정했고 RV-019는 blob 만료의 Gone 계약, RV-020은 설정 frame 상한의 Limit 분류를 각각 반례 Red 뒤 정렬했다. 초기 fixture의 source 축 범위/clock 인수·Cursor 이름 충돌·missing docs/scan tuple·ReadPin Debug/clippy 정리는 harness/구현 정리이며 별도 제품 결함으로 세지 않는다.
- 현재 closed도 bounded scan으로 index를 만든다. 표준 summary를 이용한 scan 생략 최적화는 미구현이며 실제 p95/IO 간섭 목표 달성을 주장하지 않는다. 호스트가 source prefix 불변성·독립 descriptor와 원래 등록을 보장한다. 자동 파일 발견/worker/CLI·replay/subscribe는 후속 Task다.

검증 결과는 EV-016 및 [REV-013](../../05_verification/reviews/REV-013_TASK-010.md)에 기록한다. TASK-010/MS-004 In Progress·Human 최종 리뷰/native·장비 잔여, Gate4/5 Not Ready를 유지한다. 새 public clock 변환·취소 강제 회수 정책을 도입하지 않았으며 승인된 동기식/유한 조회와 reader 수명 방향 안의 구현이므로 별도 ADR을 생성하지 않았다.

| 구분 | 파일 | 타입·내용 |
|---|---|---|
| 생성 | query/mod.rs | QueryBudget/QueryConfig/QuerySource/Snapshot·Query/TimeBasis·Cursor/Page/QueryRecord/Payload/BlobHandle/QueryError, private Ledger/Reservation/Row |
| 수정 | storage catalog/retention | private ReadPermit·Pin, ReadPin의 예약 authority/종료 flag, SegmentLog의 active boundary/reader-owned source 생성 |
| 수정 | storage mod/recovery·lib | query export, 기존 검증 helper의 crate-private 공유, 공개 wire 타입 추가 없음 |
| 생성/수정 | query/retention tests·독립 consumer·allocator·CI | 정렬/경계/오류·실제 reader 종료/회전·실제 파일 public API·8MiB 메모리와 향후 native recipe |

## 최종 로컬 검증

EV-016 최종 source53개/검사10개 exit0·두 Rust16조합/독립 실제 파일 query consumer·예제·clippy/rustdoc/fmt·allocator10·Miri128×8=1024 Pass다. query256B/8MiB peak11143/25176199 ≤ 예약110776/67217592 bytes, 잔여0·RT 할당/해제0/0 유지. REV-013 RV-018~020은 Resolved, Human 최종 리뷰/native GAP-004·장비·실제 IO 간섭/성능은 대기다. 현재 TASK-010 변경은 미커밋·미게시이며 TASK-011은 자동 착수하지 않는다.

세션 마감 — 2026-10-30: Human 「세션마무리해줘」에 따라 EV-016 source53개·검사10개 exit0·로그42개 원본/뷰 hash의 현재 일치를 확인하고 성공 시험을 반복하지 않았다. REV-013 Resolved·Human 리뷰/native·장비 잔여, TASK-010/MS-004 In Progress와 미커밋/미게시 상태를 유지한다. 다음 세션은 TASK-010·REV-013·EV-016의 최종 Human 리뷰부터 재개하고 TASK-011은 자동 착수하지 않는다.

후속 커밋 — 2026-10-30: Human 「커밋해줘」에 따라 TASK-010 소스 `8bbec13`를 로컬 커밋했다. EV-016 source53개와 commit blob hash가 일치하여 기존 성공 시험을 반복하지 않았다. [커밋 대조](../../06_evolution/imported_context/task_010_2026-10-30/commit.json). 문서는 소스와 분리 커밋하며 푸시는 하지 않는다. 커밋 요청을 최종 구현 승인으로 해석하지 않고 REV-013 Resolved·Human 리뷰/native·장비 대기와 세션 종료 상태를 유지한다. 위 미커밋/commit 미실행 문장은 검증·마감 당시 이력이다.

## REV-014 조치 — 2026-10-30

Human 「프로젝트 진행현황 확인하고, 새 리뷰 지적사항 처리하고, TASK-010 짧게 요약보고해줘.」에 따라 RV-021~023을 처리했다. 동일 Query의 정렬 index 재사용(불일치 시 사전 예약 버퍼에서 재구성·실패 전 무효화/완료 후 게시), Page 전용 Inline 좌표와 `Page::inline` 사용 안내, 초기 모든 safe_offset 합계+후속 inline/blob 재읽기 누적 예산을 반영했다. 공개 소유권·오류·데이터/정렬 계약을 보존하는 내부 개선으로 별도 ADR은 만들지 않았다.

[EV-017](../../05_verification/verification_evidence.md#ev-017) source52개·검사14개 exit0, 두 Rust 각 core/all-features workspace·실제 파일 API consumer·allocator10·clippy/rustdoc/fmt·영향 Miri(query10+retention18)×8=224 Pass다. 교차 Query/이전 cursor 재개·동일 Query 새 cursor·취소/만료 회귀를 리팩토링 전후 확인했다. RV-022/023은 문서 개선으로 형식적 신규 실패 테스트를 만들지 않았다. query256B/8MiB 메모리 peak/예약은 EV-016과 동일하며 잔여0다. REV-014 Resolved, Human 최종 리뷰/native GAP-004·장비 대기, TASK-010/MS-004 In Progress를 유지한다. 이번 조치는 미커밋·미게시이며 TASK-011은 착수하지 않았다.

2026-10-30 후속 지시 「커밋 푸시하고 다음 작업 시작하자」: EV-017 소스 `439d9c5` 로컬 커밋·source52개 commit blob hash 일치 확인, 동일 성공 시험은 반복하지 않았다. 문서는 별도 커밋·[skip ci] 게시하며 TASK-011 개발 진행은 승인됐다. 이 지시를 별도 최종 구현 리뷰 승인으로 확대하지 않아 REV-013/014 Resolved·native/장비 대기는 유지한다.

게시 확인 — 2026-10-30: 소스439d9c5·문서35e3ac3 origin/main push 성공·ls-remote HEAD 일치를 확인했다. [게시 근거](../../06_evolution/imported_context/task_010_review_fixes_2026-10-30/push.json). TASK-011 후속 구현/EV-018 로컬 Pass는 별도 작업 트리다.

## 검증 환경·시점 변경 — 2026-10-31

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체 수락하며 UPS 관련 판정은 시뮬레이션을 사용한다. native GAP-004는 GitHub 공개 완료 후(T3) 진행하는 승인된 이월이다. 별도 장치/실물 UPS 확보를 대기 조건으로 요구하지 않는다. 현재 일반 커널 실행/RT 커널 설치를 구분하며 RT 실측은 해당 커널 부팅 확인 후 기록한다. 기존 로컬 Pass·Human 승인/미승인과 미실행 범위는 보존하고 이번 정책 변경만으로 Task를 Done 처리하지 않는다.

## 후속 Human 승인 — 2026-10-31

Human 「승인, 다음작업 진행하자」로 EV-016/017의 구현/조치 최종 리뷰를 승인했다. 관련 리뷰 Closed·Human 리뷰 체크 완료. 앞선 승인 대기 본문은 이력이며 현재 PC 실측/UPS 시뮬레이션·native 공개 후(T3) 잔여로 In Progress를 유지한다. 다음 작업은 TASK-022 프로필 구체화다.

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- native·다른 OS: [TASK-024](TASK-024.md)(T3, GAP-004)
- 이 PC 정량 성능·RT: [TASK-023](TASK-023.md)에서 수행한 범위(EV-064~070)

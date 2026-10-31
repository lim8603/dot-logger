# TASK-008 — 회전·순환·이벤트 보호

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-003 |
| Story | US-002·005 |
| 요구사항 | FR-003 FR-011 NFR-003 NFR-005 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~010 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-31 |
| 다음 액션 | EV-012/013 Human 승인·현재 PC 실측 준비, native 공개 후(T3) |
| 의존 | TASK-007 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: [`retention.rs`](../../../crates/dot-logger/src/storage/retention.rs)·[`catalog.rs`](../../../crates/dot-logger/src/storage/catalog.rs)와 `tc_005_retention.rs`다.

첫 실패/리뷰 조건: **bytes/time/count/index 한계별 회전·겹친 이벤트와 읽기 핀으로 삭제 불가**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-005`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] 세 보존 모드·pre/post 실제 창·디스크 reserve·보호 quota·동기화 순서 검증.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 리뷰 완료 — 2026-10-31 최종 구현 승인. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-005](../../05_verification/test_case.md#tc-005). 제품 코드·테스트 실행 결과: [EV-012](../../05_verification/verification_evidence.md#ev-012). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 착수 — 2026-10-30

Human 「승인, TASK-008 진행하자.」로 TASK-007 최종 구현 승인·native 검증 이월 상태의 TASK-008 착수를 승인했다. 승인된 데이터 설계의 회전 순서·보존 모드·quota·reserve·핀 계약을 TDD로 구현한다. commit/push·계정/과금 조회·CI 재시도는 요청하지 않았다.

## 구현 결과 — 2026-10-30

- `SegmentLog`는 bytes·host mono 시간·record 수·index bytes/chunk 한도에 따라 회전한다. 명시 구성만 사용하고 file/index 판정은 MCAP0.26의 보수적 상한이다. 단일 record가 구성에 맞지 않으면 시작 전에 거절한다.
- chunk/footer 마감→file sync→`.active`에서 `.mcap` 이름 확정→directory sync→closed catalog 공개 순서를 지킨다. IO 실패는 Poisoned로 멈추고 미확인 파일은 catalog에 올리지 않는다. 단일 writer API를 유지하며 대상 lease는 회전 사이에도 끊기지 않는다.
- 연속 모드는 용량/기간에서 정지하며 암묵 삭제하지 않는다. 순환/이벤트 모드는 oldest unprotected closed만 삭제한다. active·event·read pin 때문에 확보가 안 되면 Capacity다. fresh filesystem 여유는 host port가 관측하며 매 admission 전에 full active file과 별도 disk reserve를 보수적으로 확보한다.
- pre/post는 session mono 창이며 실제 retained file record extents/부족한 과거 창/post 대기를 보고한다. overlap은 unique bytes로 차감하고 active는 최대 file bytes 전량 예약한다. 보호 quota 부족은 원인을 보존해 거절한다. event release/expiry는 명시 작업이다.
- read pin은 closed file 한 개의 count/bytes/deadline을 제한하며 만료는 새 읽기를 막는다. 실제 IO가 끝나 `release_pin`하기 전에는 삭제하지 않는다. catalog/event/pin·채널·범위·scratch는 공유 output 예약에 포함한다.
- `write_batch`는 회전·최종 flush 후 Written을 보고하며 오류의 accepted/flushed/reported prefix와 원래 retention 원인을 보존한다. multi-segment `sync_work`는 이전 파일 sync 사실과 current active·directory sync를 함께 구동한다. pre-IO guard 반환·IO Failed·Written 보존 계약을 유지한다.
- TDD 첫 API 부재/컴파일되는 append 미구현 Red를 확인했고 rotation/protection/IO/고갈 행동을 Green으로 구현했다. 추가로 unlink 후 directory sync 실패가 stale catalog를 남기는 반례 Red를 확인하고 수정했다. 중요도 누락 fixture·QueryOutcome payload·poll_shutdown 가변 참조·Written 계약의 shutdown 기대값은 test harness 정정이며 제품 결함으로 세지 않는다.

[EV-012](../../05_verification/verification_evidence.md#ev-012) 로컬 Pass: 저장 보존 통합15개(실제 Unix 파일1개 포함), 두 Rust 각16 features·독립 core consumer·예제·clippy/rustdoc/fmt·allocator8, Miri97×8=776회다. 신규 보존 메모리 시험256B/8MiB peak14451/16791504 ≤ 예약105696/25270752 bytes, 최종 잔여0이며 기존 RT0/0을 유지한다. source49개/명령/원본 로그 hash는 results.json에 고정했다. [REV-009](../../05_verification/reviews/REV-009_TASK-008.md)는 RV-011 로컬 해결·Resolved, Human 최신 구현 리뷰는 대기다. native CI GAP-004·실제 장비/전원·OS 교환·호스트 free-space adapter 실측은 미검증이다. 표준 FileStore는 Unix directory sync를 구현하고 다른 OS의 해당 primitive는 Unsupported로 거절한다. custom SegmentStore의 OS별 구현/장비 성능·복구/active reader는 이번 결과로 승격하지 않는다.

## 파일·타입 변경

| 구분 | 파일 | 타입·변경 내용 |
|---|---|---|
| 생성 | storage/retention.rs | SegmentStore/FileStore·SegmentLog/Active·RetentionError/RetentionBatchError·RotationReason/AppendReport — 회전/IO·예산·배치/sync |
| 생성 | storage/catalog.rs | RetentionMode/RetentionConfig·SegmentInfo·EventReport/ReadPin·Catalog/Event/Pin — 유한 catalog/이벤트/조회 보호 |
| 수정 | storage/mod.rs·writer.rs | 공개 export·TargetLease/TargetClaim, SegmentWriter의 private target claim·index 상한·동기화 공통 경로 |
| 생성/수정 | tc_005_retention.rs·rt-allocation tests·README | 실제 파일·메모리 port·control/shutdown·독립 MCAP 판독·예산/실패·일반 경로/OS 지원 범위 |

AI 자가 리뷰 **Conditional** — 현재 검토에서 미해결 P1/P2는 확인하지 못했다. Human 구현 리뷰/native 및 장비 검증을 대신하지 않는다. TASK-008/MS-003 In Progress·TC-005 부분 실행·Gate4/5 Not Ready다. commit/push·계정/과금 조회·CI 차단 재시도·fresh audit는 수행하지 않았다.

## 로컬 커밋·세션 마감 — 2026-10-30

Human 「커밋하고 세션 마무리하자」에 따라 소스 `73881452f9442e4cc38167d1a83e7fcf59a68110`를 로컬 커밋했다. EV-012 source49개와 commit blob의 SHA-256이 전부 일치하며 시험은 반복하지 않았다. [커밋 근거](../../06_evolution/imported_context/task_008_2026-10-30/commit.json)를 보존하고 마감/근거 문서는 소스와 별도 커밋한다. 푸시는 요청되지 않아 수행하지 않았다. TASK-008 Human 최종 구현 리뷰·REV-009 Resolved·native GAP-004·장비 이월을 유지한다. 다음 시작점은 TASK-008/REV-009/EV-012 구현 리뷰이며 TASK-009는 미착수다.

## REV-010 처리 — 2026-10-30

Human 「프로젝트 브리핑해주고, 리뷰 지적사항 처리하고, TASK-008 요약 보고해줘.」에 따라 개발 세션에서 RV-012~014를 처리했다. 이번 요청은 최종 구현 승인이나 commit/push 지시가 아니다.

- RV-012: 호출자 토큰 유실 위험은 유효하나, 만료를 실제 IO 종료로 간주하는 강제 회수는 승인된 삭제 보호 계약과 충돌한다. `ReadPin`에 `must_use`와 유실 시 `PinLimit`/`Capacity` 제약을 명시하고 `pin`/`release_pin`·인터페이스 문서를 보강했다. 유실·만료 후 삭제 금지/슬롯 점유 회귀는 기존 계약 확인용 Green이다. 실제 리더의 오류/취소/종료 후 반환과 안전한 고립 회수 조건은 TASK-010 완료 체크에 연결했다. 위험 자체를 제거한 것으로 보지 않는다.
- RV-013: 회전 없는 append만 별도 `room`을 호출한다. 회전의 검사 2회는 유지하며 중복 host probe 1회를 제거했다. 기존 3회/기대 2회 반례 Red 후 Green을 확인했고 이후 일반 append의 reserve 부족 거절·동일 record 재시도·독립 MCAP 판독을 검증했다.
- RV-014: `FileStore::new` rustdoc에 session 경로의 symlink 거절 의도를 명시했다. 동시 경로 교체 방지는 제공하지 않으므로 trusted base/session의 배타적 관리 전제를 함께 기술했다.

변경 파일/타입은 `catalog.rs`(`ReadPin`), `retention.rs`(`SegmentLog`, `FileStore`), `tc_005_retention.rs`(test `State`·신규 행동2개)다. 보존 통합17개·두 Rust core/all-features workspace/consumer/예제/allocator8/clippy/rustdoc/fmt·보존 Miri16×8=128회 Pass다. REV-010은 Resolved로 갱신했다. 최종 명령·source49개 hash·원본 로그·결과/미실행 범위는 [EV-013](../../05_verification/verification_evidence.md#ev-013)에 기록했다. dependency/lock·RT 입력·unsafe 경계·worker 기동은 유지하며 TASK-008/MS-003 In Progress, TC-005 부분 실행·Gate4/5 Not Ready다.

2026-10-30 Human 「커밋 푸쉬 후, 다음 테스크 진행」에 따라 EV-013 소스 `14fd1a4`를 커밋했다(source49개/commit blob hash 일치). `.cowork` 문서와 분리해 게시하며 TASK-009 후속 착수는 승인됐다. TASK-008 최종 구현 리뷰·REV-009/010 Resolved·native GAP-004는 유지한다.

## 검증 환경·시점 변경 — 2026-10-31

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체 수락하며 UPS 관련 판정은 시뮬레이션을 사용한다. native GAP-004는 GitHub 공개 완료 후(T3) 진행하는 승인된 이월이다. 별도 장치/실물 UPS 확보를 대기 조건으로 요구하지 않는다. 현재 일반 커널 실행/RT 커널 설치를 구분하며 RT 실측은 해당 커널 부팅 확인 후 기록한다. 기존 로컬 Pass·Human 승인/미승인과 미실행 범위는 보존하고 이번 정책 변경만으로 Task를 Done 처리하지 않는다.

## 후속 Human 승인 — 2026-10-31

Human 「승인, 다음작업 진행하자」로 EV-012/013의 구현/조치 최종 리뷰를 승인했다. 관련 리뷰 Closed·Human 리뷰 체크 완료. 앞선 승인 대기 본문은 이력이며 현재 PC 실측/UPS 시뮬레이션·native 공개 후(T3) 잔여로 In Progress를 유지한다. 다음 작업은 TASK-022 프로필 구체화다.

# TASK-009 — 중단 복구와 I/O 장애

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-003 |
| Story | US-002·005 |
| 요구사항 | FR-003 NFR-001 NFR-003 NFR-007 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~010 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-30 |
| 다음 액션 | EV-014/015 로컬 Pass·Human 승인·REV-011/012 Closed → native·장비 검증 |
| 의존 | TASK-008 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: [`storage/recovery.rs`](../../../crates/dot-logger/src/storage/recovery.rs)·[`fault_storage.rs`](../../../crates/dot-logger/tests/fault_storage.rs). writer private sync helper·allocator/CI recipe·README를 함께 갱신했다.

첫 실패/리뷰 조건: **chunk 중간 절단·CRC 손상·ENOSPC·sync/rename 실패**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-005`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] 원본 보존 복구·온전한 prefix/제외 범위·RT 경로 유지·OS 오류/primitive 의미를 구현하고 로컬 시험했다. 실제 OS별 내구성/장비 판정은 native/TASK-022로 이월한다.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-005](../../05_verification/test_case.md#tc-005). 제품 코드·테스트 실행 결과: [EV-014](../../05_verification/verification_evidence.md#ev-014). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 착수 — 2026-10-30

Human 「커밋 푸쉬 후, 다음 테스크 진행」에 따라 TASK-008 EV-013 소스/문서 게시 후 착수한다. TASK-008 최종 구현 리뷰·native GAP-004는 남겨두며 후속 개발 진행을 승인받았다. 승인된 별도 복구 파일·원본 무수정·완전 검증 chunk prefix·손상/제외 범위·IO 오류 보존 계약을 TDD로 구현한다.

## 구현 결과 — 2026-10-30

- `recover_segment`는 원래 session/revision·동일 순서의 등록 channel/schema와 frozen source를 입력받고 독립된 빈 sink에 복구한다. source에는 Write bound가 없고 호스트가 read-only open·불변성을 보장한다. 현재 `dot-logger/1` 무압축·nonzero chunk CRC 형식만 허용한다.
- fixed body buffer와 source bytes/record/chunk 한도, 추가 output 예약을 사용한다. 원본 길이/overflow/opcode·CRC·schema/channel 선행 참조·채널 metadata·봉투/시각·producer sequence를 검사하고 chunk 전체 검증 뒤에만 출력한다. 한 chunk 안의 마지막 메시지가 손상되어도 앞선 메시지를 살리지 않으며 손상 이후 재동기화하지 않는다.
- report는 마지막 완전 검증/복사 source chunk 끝과 제외 suffix/원인·chunk/record 수를 반환한다. 옛 index/summary도 재생성하므로 제외 suffix 전체를 메시지 손실로 해석하지 않는다. 새 summary/보조 인덱스와 opaque payload/full-width receipt를 보존하되 현재 봉투의 알려진 필드를 재인코딩한다. 과거 Written/Synced 응답은 추정하거나 registry에 복원하지 않는다.
- `recover_to_store`는 capability 확인→새 파일 create→복구/file sync→close→finalize→directory sync이며 성공 catalog를 자동 생성하지 않는다. 실패 파일은 조사용으로 남고 자동 retry/delete하지 않는다. 실제 IO/용량 오류는 원 cause/선행 prefix를 반환하고 source는 보존한다. custom port 계약 위반까지 방어했다고 주장하지 않는다.
- TDD: 컴파일되는 미구현 API에서 행동 Red(8실패/1기존 입력 오류 Pass) 후 Green으로 구현했다. 자가 리뷰 RV-015는 final sync 중 취소 게시의 성공 반환 반례 Red 후 최종 sync 뒤 재검사로 해결했다. 초기 unused import·fixture let-and-return lint는 harness 정리이며 별도 제품 결함으로 세지 않는다.
- 보존/복구 통합19개(실제 Unix FileStore/read-only source1개 포함), 두 Rust 각16 features/독립 consumer/예제/clippy/rustdoc/fmt·allocator9 Pass다. 복구 256B/8MiB payload peak21386/33572214 ≤ 전체 예약213776/50543888 bytes, 최종 잔여0이다. 기존 RT allocation/deallocation0/0을 유지한다. 확장 Miri117×8=936회 Pass, 최종 검사10개/source51개 hash는 EV-014로 고정했다.

원본 불변·빈 독립 출력·완전 chunk prefix·실제 IO/오류 보존의 승인된 방향 안의 구현 세부사항이므로 새 ADR은 생성하지 않았다. 최신 리뷰 [REV-011](../../05_verification/reviews/REV-011_TASK-009.md)은 RV-015 로컬 해결·Resolved이며 Human 최종 리뷰 전 Conditional이다. TASK-009/MS-003 In Progress·TC-005 부분 실행·Gate4/5 Not Ready다. native GAP-004·전원/실장비·OS 교환·host probe 실측은 잔여다. 자동 시작의 directory 발견/기존 namespace 재개·active reader TASK-010·worker·StorageDegraded 계측 TASK-014는 이번 primitive 구현으로 완료하지 않는다.

## 파일·타입 변경

| 구분 | 파일 | 타입·내용 |
|---|---|---|
| 생성 | storage/recovery.rs | RecoveryConfig/RecoveryOptions·RecoveryStop/RecoveryReport/RecoveryError·RecoveredSegment, private Fields/Validation — 유한 스캔/검증·재작성·동기화·보고 |
| 수정 | storage/mod.rs·writer.rs | 공개 export, FinishedSegment의 private 복구 전용 sync helper |
| 생성 | tests/fault_storage.rs | State/Buffer/Store·BrokenReader/FailAtReader/CancelOnSync — 파일/OS 오류·취소·삭제 금지·prefix·독립 판독 |
| 수정 | allocator tests·CI recipe·README | 복구 예산/최종 반환, future native Miri의 retention/fault_storage 추가, 호출자 책임/지원 범위 |

TASK-008 REV-010 조치까지는 소스14fd1a4·문서1f2f44d로 origin/main 게시 완료했다. TASK-009는 후속 세션 마감/게시 요청으로 소스 e5f3147을 커밋했으며, Human 최종 구현 리뷰는 별도 대기다.

세션 마감 — 2026-10-30: Human 「세션 마무리해줘」 및 「커밋 푸쉬도 해줘」에 따라 TASK-009 소스 `e5f3147`를 커밋했다. EV-014 source51개/commit blob 및 검사10개 근거가 일치하여 시험을 반복하지 않았다. `.cowork` 검증/마감 문서212373c와 분리해 origin/main 게시 완료했으며, 구현 Human 승인·Resolved 리뷰·native/장비 이월 상태는 유지한다. [커밋 근거](../../06_evolution/imported_context/task_009_2026-10-30/commit.json).

## REV-012 조치 — 2026-10-30

Human의 새 리뷰 지적사항 처리 요청으로 RV-016/017(P3)을 수용했다. `recover_inner`의 메시지 payload 조회는 `get(22..)`로 경계를 확인한다. `Validation::schema/channel`은 `matches_text`의 고정64바이트 스택 포맷과 encoding 직접 비교를 사용해 반복 `format!`/`to_string`/clone을 없앴다. 현재 최장 스키마 이름52바이트를 수용하며 기존 canonical 문자열·복구 계약/예약/공개 API를 유지한다. 타입 선언 추가/변경은 없다.

[EV-015](../../05_verification/verification_evidence.md#ev-015)에서 Rust1.85/1.96 workspace/all-features 각각201개(복구19·allocator9·doctest17 포함)·양쪽 clippy·fmt Pass. 이전 EV-014와 소스51개 중 recovery.rs만 달라졌고 최신 hash/로그는 EV-015다. Miri/16-feature matrix/rustdoc은 EV-014 이전 소스의 이력으로 보존한다. [REV-012](../../05_verification/reviews/REV-012_TASK-009_DEEP_REVIEW.md)는 Open→In Progress→Resolved, Human 최종 리뷰 전 Closed로 올리지 않는다. native GAP-004·장비 검증·TASK-009/MS-003 In Progress는 유지한다. 이번 요청으로 commit/push·TASK-010 착수는 수행하지 않았다.

2026-10-30 Human 「승인, 커밋 푸쉬하고 다음 작업 진행하자.」: TASK-009 최신 구현 승인·REV-011/012 Closed, 소스 `fe03468`은 EV-015 source51개와 일치한다. native/장비 미완료로 In Progress를 유지하며 TASK-010 진행을 승인받았다.

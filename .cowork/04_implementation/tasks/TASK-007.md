# TASK-007 — MCAP 봉투·writer·인덱스

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-003 |
| Story | US-002·005 |
| 요구사항 | FR-002 FR-003 FR-013 NFR-002 NFR-005 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~010 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-006 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: [`storage/`](../../../crates/dot-logger/src/storage/mod.rs)의 codec·writer·index·버전 봉투와 `tc_004_storage.rs`다.

첫 실패/리뷰 조건: **u64 receipt 왕복·벽시계 역행·unknown schema와 8 MiB 단일 record 경계**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-004`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] 독립 MCAP reader 대조·CRC/index on, index/encoder peak 예산 및 safe offset 검증.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-004](../../05_verification/test_case.md#tc-004). 제품 코드·테스트 실행 결과: [EV-010](../../05_verification/verification_evidence.md#ev-010). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 착수 — 2026-10-29

Human이 TASK-006 최신 수정분 승인 및 native 이월 상태의 TASK-007 진행을 승인했다. 선행 구현/로컬 검증/리뷰는 승인됐고 GAP-004 native는 계속 잔여다. TDD로 봉투/MCAP writer/인덱스를 구현하며 CI·과금 조회/재시도는 하지 않는다.

## 구현·검증 결과 — EV-010 / 2026-10-29

- `storage/{mod,codec,index,writer}.rs`, `envelope.proto/bin`을 추가했다. 버전1 Protobuf 봉투는 session/schema128·receipt sequence64·producer/channel/revision·mono/real/source/domain·level/importance·payload 길이를 보존한다. source time을 자동 UTC로 바꾸지 않으며 음수 real_ns·MCAP u32 metadata 범위 초과는 거절한다. 알려지지 않은 사용자 encoding/schema는 opaque bytes다. 독립 protoc golden으로 봉투를 대조했다.
- MCAP0.26 pinned, 무압축·chunk/data/summary CRC·message/chunk index on이다. file-local u16 IDs로 제품 ID를 매핑하고 MCAP의 sequence는0, 전체 식별자는 봉투에 있다. 직접 seek·명시 chunk 마감으로 chunk target을 내부 메모리 hard cap으로 오인하지 않는다.
- `Router::writer_budget`은 구체 출력 예약 후 남은 예산을 공유하며 writer/finished index의 합계를 단발 CAS lease로 차감한다. `TargetLease`가 대상별 활성 writer를 하나로 제한한다. 입력 spare capacity는 정규화하고 message/chunk/metadata/record 한도와 summary clone/Vec 성장을 보수적 메모리 예약에 포함한다.
- `append`는 같은 session/revision·등록 channel/schema·증가 sequence·길이/시각을 검증한다. record/chunk 상한은 NeedRotation으로 입력 전에 거절하며 자동 회전/삭제는 하지 않는다. 성공한 chunk flush 뒤에만 safe offset과 채널별 mono/real min/max·offset/count를 공개한다. IO 실패는 Poisoned이고 safe offset을 앞당기지 않는다. 파일 쓰기 후 완료 이력이 만료되면 CompletionError::Expired를 보존해 입력 거절과 구별하며 이후 파일 쓰기는 계속할 수 있다. Drop은 footer IO나 성공을 만들지 않는다.
- `write_delivery`는 실제 write/완전 chunk flush 후 Written을 보고한다. writer/finished 파일의 `sync_work`는 실제 sink sync 후 결과를 보고하고 failure는 Written을 보존한다. control의 bounded file-coverage 검사로 새 파일이 이전 세그먼트의 pending/만료 기록을 Synced로 확인하지 못한다. closed-file sync는 footer도 포함하며 여러 파일에 걸친 회전 조정은 TASK-008이다.
- 새 저장 통합13개·기존 회귀, 두 Rust 각16 features/실제 파일 독립 consumer/예제/clippy/rustdoc/fmt·allocator7 Pass. Miri 기존81×8+새 봉투2×8=664 Pass(나머지 파일 통합11개는 일반 로컬 실행이며 Miri로 확대하지 않음).
- allocator는 payload256B/8MiB 각각129 records/chunks의 구성→마감·index 해제 peak104824/8493176 bytes, 예약232064/17008768 bytes, 최종 잔여0을 확인했다. 기존 RT allocation/deallocation0/0, query/sync128회 peak11602/concrete14994/잔여0, routing512개 peak2672/output3184/잔여0이다. Router 공유 예산 metadata 추가분16bytes도 예약에 반영된다.

상세 실행 대상·명령·환경·hash/로그와 scope는 [EV-010](../../05_verification/verification_evidence.md#ev-010)에 있다. Native CI·장비 성능/전원 차단·구OS 파일 교환·자동 회전/복구/active reader·worker 기동·제품 압축 선택은 미검증/후속이다. 현재 범위는 호스트가 제공한 파일을 일반 경로에서 직접 구동하는 writer다. Human 구현 리뷰는 대기, TASK-007/MS-003 In Progress·TC-004 부분 실행이다.

## 파일·타입 변경

| 구분 | 파일 | 역할 |
|---|---|---|
| 생성 | storage/mod.rs·codec.rs·index.rs·writer.rs·envelope.proto/bin | WriterBudget/Lease/TargetLease, WriterConfig/StorageChannel/PayloadSchema/StorageError, StoredRecord/Envelope/Source, ChunkIndexEntry, SegmentWriter/FinishedSegment/SyncSink/Sink/Signals |
| 수정 | routing/mod.rs·control/mod.rs·lib.rs | SharedLedger/TargetLedger 예산·활성 writer 표시, Router::writer_budget·SyncWork::covers_file·storage 공개 |
| 생성 | tc_004_storage.rs·fixtures/envelope_v1.bin | 실제 파일·독립 컨테이너 scanner/CRC/index·protoc golden·오류/한계 회귀 |
| 수정 | 독립 consumer·allocator tests·README | 파일 write→finish→실제 sync→shutdown 종단 및 메모리 peak 검증·실제 범위 정렬 |

자가 리뷰 Conditional — 로컬 구현/계약 검증 Pass, 추가 P1 미확인. 수량 기반 예산은 pinned MCAP의 보수적 예약이며 실제 장비의 RSS/캐시·OS sync 지연/내구성 보장은 아니다. commit/push·계정/과금 조회·CI 재시도·fresh audit는 하지 않았다.


## 리뷰 RV-006~010 수정 — EV-011 / 2026-10-29

Human 「코드리뷰 받은 내역을 확인하고 수정하자는 말이였어」로 수정 범위·방향을 승인했다. [#003 작업 로그](../../members/lim8603/workspace/session_logs/session_2026-10-29_003.md)에 결정·TDD·F-11 소비자 목록을 기록했다. 기존 Written 의미는 유지하며 새 ADR은 필요하지 않다.

| 항목 | 수정 내용·결과 |
|---|---|
| RV-006 | `write_batch(&[&Delivery])`가 호출자 guard 묶음을 chunk target까지 누적하고 최종 flush 뒤 Written을 보고한다. `BatchWriteError`는 appended/written/reported prefix를 보존한다. 인덱스 예약·선할당은 `min(max_records, max_chunks × channels)`로 제한한다. 단일 즉시 flush는 `write_delivery`로 명시한다. |
| RV-007 | IO 전 target/session/coverage 거절·Busy·Poisoned에서 `SyncWorkError::work = Some(work)`로 원래 guard를 반환한다. 실제 flush/sync 실패는 Failed·work None이며 반환한 guard를 버릴 때만 Unknown이다. |
| RV-008 | active/finished 공통 `check_sync`로 오류를 매핑하고 metadata Busy를 두 경로 모두 보존한다. 기존 리뷰의 query permit 포화 예시는 정정했다. coverage의 직접 poll은 query permit과 무관하다. |
| RV-009 | 명시 `write_buffer_bytes`를 공유 출력 예약에 포함한다(0=direct, 제품 기본값 없음). seek/flush 전에 배출·논리 position 조회·짧은 write/Interrupted/부분 실패·Drop 무IO를 처리한다. 128×64B 레코드의 File write 호출은 962→9(4KiB 버퍼), 파일 바이트·독립 CRC/index 대조 일치다. 장비 처리량/지연 목표는 미검증이다. |
| RV-010 | Busy 뒤 `Delivery::report(Update::Confirmed(Boundary::Written))`만 재시도하도록 rustdoc/계약/README에 명시했다. 단일/배치의 보고 중 Busy와 보고 prefix 재시도를 결정적으로 검증했다. |

저장 통합21개·private Busy2개·두 Rust 각16 features/2-record 실제 파일 consumer→finish→sync→shutdown/예제·clippy/rustdoc/fmt·allocator7 Pass. Miri는 lib14+raw17+routing19+completion15+control18=83×8=664 Pass이며 파일 IO 통합19개는 일반 로컬 실행이다. 봉투2개는 변경되지 않아 EV-010의 Miri 근거를 재사용한다. 128채널·4096레코드 sparse/dense 요청 메모리 peak2454992/611808 bytes ≤ 예약5838400, 최종 잔여0이다. 버퍼 포함 단일 채널256B/8MiB peak108926/8497278 ≤ 예약236160/17012864다.

수정분 AI 검토는 **Conditional**(로컬 수정·회귀 Pass, Human 최종 리뷰/native·장비 잔여)이다. RV-006~010은 로컬 수정 범위에서 해결했으며 TASK-007/MS-003 In Progress·TC-004 부분 실행을 유지한다. 실제 장비 성능/기본 chunk·buffer·batch 지연 설정은 TASK-022 후속이다. 수정 작업 당시 commit/push 지시는 없었다. 이후 Human 「커밋 푸쉬하고 세션 마감해줘」로 소스 `c932ee4`를 게시했고 EV-011 source46개/commit blob 일치를 확인했다. 최신 구현 Human 리뷰/native는 계속 잔여다.

## 정리 요약 — 2026-10-30

- **구현 범위:** 버전1 Protobuf 봉투, MCAP 0.26 무압축 writer·CRC/index, 공유 출력 예산, 완전 chunk 이후 safe offset/Written 공개, 실제 파일 sync와 완료 추적을 연결했다. `finish`는 footer를 마감하며 저장 동기화는 명시 sync로 구분한다.
- **리뷰 반영:** [REV-008](../../05_verification/reviews/REV-008_TASK-007_RV-006-010.md)의 RV-006~010 모두 구현되어 있다. 배치 쓰기·인덱스 예약 상한, IO 전 거절 시 원래 `SyncWork` 반환, 공통 Busy 매핑, 예산 내 쓰기 버퍼, 쓰기 중복 없는 보고 재시도를 제공한다. 배치 오류의 실제 필드는 `appended`/`written`/`reported`, sync 오류는 `SyncWorkError::work`다.
- **검증:** EV-011의 저장 통합21개·private Busy2개, Rust1.85/1.96 각16 features·독립 consumer·예제·clippy/rustdoc/fmt·allocator7, Miri664회 Pass 근거를 유지한다. 128×64B의 File write 호출962→9는 4KiB 버퍼 시험 관측이며 실제 syscall 수·장비 성능 결과가 아니다.
- **이번 작업:** EV-011 source46/46 SHA-256 일치와 기록된 검사10개 exit0을 확인하고 REV 처리 설명을 실제 API로 정정했다. 제품 코드는 추가 변경하지 않았고 동일 시험도 반복하지 않았다.
- **상태·후속:** TASK-007/MS-003 In Progress, REV-008 Resolved, TC-004 부분 실행이다. Human 최종 구현 리뷰·native GAP-004는 대기이며, 회전/보존·다중 세그먼트 및 directory metadata sync는 TASK-008, 장애 복구는 TASK-009, active reader는 TASK-010, 장비 성능은 TASK-022 후속이다. 이번 반영 지시는 최종 구현 승인으로 확대하지 않는다.

## 최종 구현 승인 — 2026-10-30

Human 「승인, TASK-008 진행하자.」로 최신 수정분 구현 리뷰 완료·REV-008 Closed, native 이월 상태의 TASK-008 착수를 승인했다. TASK-007은 native GAP-004 잔여로 In Progress를 유지한다.

## 검증 환경·시점 변경 — 2026-10-31

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체 수락하며 UPS 관련 판정은 시뮬레이션을 사용한다. native GAP-004는 GitHub 공개 완료 후(T3) 진행하는 승인된 이월이다. 별도 장치/실물 UPS 확보를 대기 조건으로 요구하지 않는다. 현재 일반 커널 실행/RT 커널 설치를 구분하며 RT 실측은 해당 커널 부팅 확인 후 기록한다. 기존 로컬 Pass·Human 승인/미승인과 미실행 범위는 보존하고 이번 정책 변경만으로 Task를 Done 처리하지 않는다.

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- native·다른 OS: [TASK-024](TASK-024.md)(T3, GAP-004)
- 이 PC 정량 성능·RT: [TASK-023](TASK-023.md)에서 수행한 범위(EV-064~070)

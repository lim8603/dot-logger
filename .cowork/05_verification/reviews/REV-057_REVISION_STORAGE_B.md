# REV-057 — TASK-030 S2 B revision 회전·source별 조회/복구 개발 자가검토

| 항목 | 내용 |
|---|---|
| 일자·리뷰어 | 2026-11-17 / AI, code-review 스킬 적용 |
| 대상 | `509ac01` 이후 TASK-030 S2 B 제품·시험·fixture·SDK/CLI 변경 |
| 판정 | Pass — 두 Rust 전체·18 feature 구성·정적 검사·Miri Pass |
| 상태 | Closed — RV-145 수정·최종 회귀·2026-11-17 위임 수락 |
| 근거 | [EV-084](../verification_evidence.md#ev-084), 검증 결과(비공개 원시 근거) |

## 요약정리

승인된 ADR-013 D3에 따라 single active writer를 유지하며 교차 revision 파일 회전, 파일별 identity의 query/replay/recovery, manifest v2를 구현했다. 새 게이트의 소비자·기존 fixture를 함께 정렬했고 RT 수집·unsafe·원자 순서는 변경하지 않았다. 검토 중 Header만 있는 nonzero source가 file identity 검증 없이 빈 정상 조회로 수락되는 공백(RV-145)을 재현·수정했다. 손실·runtime WIP·GUI/성능/커버리지 재측정은 후속이다.

## 파일 변경표

| 구분 | 파일 | 변경 |
|---|---|---|
| 수정 | `routing/mod.rs`, `storage/{mod,catalog,retention}.rs` | 같은 router/target budget 검증, Delivery 파생 budget, revision 경계 회전, catalog identity, 실패 전 발행 prefix 보고 |
| 수정 | `query/mod.rs`, `storage/recovery.rs` | source별 revision/sequence·receipt 중복·필수 file identity 검증, recovery sequence 보존/새 output sequence 정렬 |
| 수정 | client `offline.rs`, CLI `model.rs`·`commands.rs` | revision을 source로 이동, v2 입력/상태, 파일별 복구·전체 파일/바이트/기록 한도 |
| 생성 | `tests/tc_016_revision.rs` | 실제 interleaved Delivery, event/pin·fault·sync·중복 receipt 통합 8개 |
| 수정 | CLI 통합 시험, query/replay/remote 시험, API consumer·RT allocator·UPS 예제, manifest 예제·README | v2 실파일·blob·복구·Header-only 4개 신규, 실제 sequence와 RecoveryOptions 정렬 |
| 수정 | API·데이터 모델·Task/MS·TC/EV·상태 | 구현 계약·근거·후속 범위 동기화 |

## 클래스(타입) 변경표

| 구분 | 소속 | 타입 | 변경 |
|---|---|---|---|
| 수정 | routing/storage | `Delivery`, `WriterBudget`, `SegmentLog` | identity 검증·내부 budget 파생·명시 append budget·교차 회전, 논리 수락/발행 watermark |
| 수정 | storage | `SegmentInfo`, `RotationReason`, `RetentionBatchError` | revision 필드/사유, 실제 발행된 Written prefix 보존 |
| 수정 | query | `QuerySource`, `Snapshot` | source revision, 전역 revision 제거·receipt uniqueness·필수 file identity |
| 수정 | recovery | `RecoveryOptions`, 내부 `Validation` | 원본 source_sequence·출력 sequence 분리, file_sequence 조회/대조 |
| 수정 | client/CLI | `OfflineConfig`, `OfflineSource`, `Manifest`, `Source` | 전역 revision 제거·source별 revision·schema v2 |
| 생성 | TC-016 시험 | `State`, `Store`, `Sink` | 유한 fake store와 create/header/pair/기존 파일 보존 fault 관측 |

## 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 문제·처리 | 판정/상태 |
|---|---|---|---|---|
| RV-145 | P2 | query/recovery source identity | Header만 있고 dot_logger.file이 없는 nonzero 입력이 정상 빈 source로 끝남. scan 종료 때 root identity 존재를 확인 | 수용·수정·EV-084 Pass / Closed |

### RV-145 — Header만으로 정상 빈 source 판정

**[리뷰어]** 실제 파일에서 MCAP Header 끝을 safe_offset으로 지정하면 CLI query가 `complete`, records 0으로 종료됐다. source별 revision/sequence 검증을 요구하는 계약과 다르게, 검증할 Metadata가 없다는 사실을 성공으로 해석했다. recovery도 Header 뒤 EOF만으로 정상 빈 복구가 될 수 있었다. safe_offset 0의 미공개 active 범위는 별도 계약이므로 유지하고, nonzero source의 identity 미확인을 거절해야 한다.

**[처리담당]** 수용. query scan 종료에서 `Validation::file_sequence`를 확인해 CorruptData로 거절한다. recovery는 DataEnd/EndOfInput이어도 identity 미확인이면 InvalidRecord로 보고하며 부분 출력 보존·file sync와 과거 completion의 구분을 유지한다. CLI `native_header_without_file_identity_cannot_be_accepted_as_an_empty_snapshot`이 query Red(exit0→기대4)를 재현했고, 수정 뒤 query exit4·recover InvalidRecord/partial exit3를 확인했다. Red 원본(비공개 원시 근거).

## 리뷰 포인트

- RT: 새 budget 파생·Arc clone·sort·파일 작업은 일반 IO/조회 경로다. capture/거절/RT Drop·unsafe·원자 순서에 새 동작을 넣지 않았다. 기존 allocator19 및 Miri의 영향 query/교차 revision20을 확인한다.
- 소유권: live Delivery의 shared/target ledger가 모두 맞아야 budget을 파생한다. 고급 경로도 같은 router/target/session/producers를 확인한다. target lease는 유지하고 publish가 이전 writer를 해제한 뒤 새 writer를 생성한다. plan guard는 Delivery에 남는다.
- 완료: capacity는 active 보존, create/header/pair 실패는 기존 published 파일 보존·poison. batch는 논리 prefix를 보존하고 실제 written/report 범위를 넘겨 보고하지 않는다. sync work가 모든 producer 범위를 확인하며 파일 이름·새 revision 게시를 Synced로 대체하지 않는다.
- 유한 메모리: catalog의 실제 SegmentInfo 폭을 예약하고, source revision은 기존 QuerySource 폭에 포함한다. 중복 receipt 검사는 예약된 order 버퍼를 재사용한다. CLI adapter는 manifest4MiB·source1024 상한과 query/recovery의 기존 유한 예약을 유지한다. 다중 recovery는 하나씩 실행하며 전체 읽기/기록 한도를 갱신한다.
- F-11: 실제 source sequence를 요구하므로 standalone 시험/consumer/remote/allocator를 0으로 정렬했다. 가짜 sequence로 같은 파일 prefix/전체를 중복 등록하던 query 시험은 sequence 위조 거절로 바꾸고, 정상 다중 source의 병합은 실제 revision 회전 파일·CLI blob/replay 시험으로 검증한다.
- 검증 범위: 실제 PC의 현재 RT 커널에서 로컬 기능/메모리 검증이며 RT 지연·처리량 시험이 아니다. 공개 후 다른 OS/native·24h·실앱·GUI·커버리지 재측정은 미실행으로 유지한다. native loss는 unavailable이고 TASK-035 S2·TASK-034 runtime은 미착수/미통합이다.

## 검증과 마감

두 Rust workspace 각각642 Pass·5 ignored, clippy·workspace/core rustdoc Pass, fmt·표면/레시피34/60 Pass, Miri query/교차 revision20 Pass. 두 Rust 각18 feature 구성 Pass·source175개와 최종 `63a9c87` blob 일치 확인(제품 `91ac496`, 상한 시험 보강 `63a9c87`) 후 위임 수락했다. Open(진단) → In Progress(수정) → Resolved(회귀) → Closed(2026-11-17 위임 수락)로 마감했다. TASK-030 전체·MS-009·Gate4/5 완료를 선언하지 않는다.

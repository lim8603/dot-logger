# REV-059 — ADR-013 native 저장(A)·revision 회전(B)·손실 누적 checkpoint(C) 및 REV-055 조치 통합 코드리뷰

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-059 / 2026-11-17 |
| 리뷰어 | Antigravity AI — 리뷰 전용 세션 |
| 대상 | REV-055 이후 누적 코드 커밋 `949a74e..0270f03` (5개 코드 커밋: `5d7fe6a`, `048cdf3`, `2dff5f0`, `70d2f6a`, `0270f03`) |
| 범위 | REV-055 결함 조치(`5d7fe6a`), TASK-030 S2 A native 표준 MCAP pair 저장/읽기(`048cdf3`), B revision 경계 회전·source별 조회/복구·manifest v2(`2dff5f0`, `70d2f6a`), TASK-035 S2 C 누적 손실 카운터·native `_dot_logger/loss` 체크포인트·오프라인 조회/복구(`0270f03`) |
| 종합 판정 | 초기 **Conditional**(P1 1·P2 2·P3 1) → 조치 후 **Pass**([EV-086](../verification_evidence.md#ev-086)) |
| 상태 | **Closed** — 2026-11-17, RV-149~152 조치·위임 수락 |

워크스페이스 전체 테스트(665 passed·5 ignored), RT 무할당 시험(20 passed), `clippy`, `fmt`, strict `rustdoc`, NFR-008 표면/레시피 가드(`34/60`)는 모두 통과했다. 그러나 `949a74e..0270f03` 변경분을 ADR-013 및 API §22~23 계약과 대조해 정밀 검토한 결과 다음 4건을 확인했다:
1. **P1 (`RV-149`):** `recover_segment` / `recover_to_store`가 chunk 내부 레코드를 복사하기 전에 `options.writer.max_records`(및 출력 chunk 상한)를 chunk 단위로 선검사하지 않아, 단일 chunk 내 2건 이상의 사용자 레코드 중 일부만 출력에 쓴 뒤 중간에 `NeedRotation`으로 실패(부분 chunk 반출)하고, `writer.append`의 `?` 조기 반환으로 인해 앞선 chunk에서 이미 검증·복구된 `Baseline` loss report나 사용자 레코드가 `writer.finish()`·`sync_recovered()` 없이 미완결 파일로 남는다. 또한 루트 `dot_logger.file` 선행 peek(`recovery.rs:308`)의 `?`가 손상된 메타데이터를 `RecoveryStop::InvalidRecord` 대신 `RecoveryStop::Failed`로 조기 실패시키며, CLI `recover` 성공 JSON 행에 `"target"` 필드가 누락되어 있다.
2. **P2 (`RV-150`):** `SegmentWriter`가 회전된 후속 세그먼트(`sequence >= 1`)에서도 `sample_start = 0`을 `Coverage::Baseline`의 `observed_from_ns`로 기록하여, 회전 파일의 `LossDelta.observed_from_ns`가 항상 `0`으로 왜곡되고 CLI `loss --from/--to` 필터가 파일 개설 이전 구간과 거짓 매칭된다. 또한 다중 출력이 모두 첫 주기 체크포인트 전에 고장났을 때 각 파일의 초기 `Baseline`끼리 합산(`complete_count >= 2`)되어 `delta = Some(0)`이 만들어지는 문제와 다중 part 체크포인트의 개별 part 시간 필터링 문제가 있다.
3. **P2 (`RV-151`):** 종료(`closing`) 경계에서 `GeneralProducer::record_until`의 `Deadline` 분기와 `CaptureCore::loss_before_submit`(`Writer::try_log_at`의 `Encode`/`InvalidInput` 경로)이 `ActiveCall` 가드 내부의 `closing`(`Ordering::SeqCst`) 확인을 거치지 않아, 로거 종료 후에도 `Encode`/`InvalidInput`/`Deadline` 손실 카운터가 증가하고 `LogError::Closing` 대신 `LogError::Encode`/`InvalidInput`이 반환될 수 있다.
4. **P3 (`RV-152`):** `NativeReader::new`의 `read::registrations` 초기 스캔에서 중복 Schema/Channel ID 덮어쓰기 방지 및 `recover_inner`의 `PayloadSchema::name` 초과 capacity 축소(`into_boxed_str`) 정렬을 권고한다.

리뷰 전용 세션 원칙에 따라 제품 코드·저장소 테스트는 수정하지 않았다.

## 2. 파일 변경표

| 구분 | 파일 | 변경 내용 |
|---|---|---|
| 생성 | [REV-059](REV-059_NATIVE_REVISION_LOSS_REVIEW.md) | `949a74e..0270f03` 독립 코드리뷰 결과(`RV-149~152`) 및 처리담당 영역 기록 (`Open`) |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | 미처리 리뷰 목록·코드리뷰 현황에 REV-059 (`Open`, P1 1·P2 2·P3 1) 등록 및 다음 시작점 동기화 |

검토 대상 코드 파일은 [loss.rs](../../../crates/dot-logger/src/loss.rs), [capture/mod.rs](../../../crates/dot-logger/src/capture/mod.rs), [completion/mod.rs](../../../crates/dot-logger/src/completion/mod.rs), [routing/mod.rs](../../../crates/dot-logger/src/routing/mod.rs), [storage/mod.rs](../../../crates/dot-logger/src/storage/mod.rs), [storage/codec.rs](../../../crates/dot-logger/src/storage/codec.rs), [storage/writer.rs](../../../crates/dot-logger/src/storage/writer.rs), [storage/read.rs](../../../crates/dot-logger/src/storage/read.rs), [storage/retention.rs](../../../crates/dot-logger/src/storage/retention.rs), [storage/recovery.rs](../../../crates/dot-logger/src/storage/recovery.rs), [query/mod.rs](../../../crates/dot-logger/src/query/mod.rs), [logger/writer.rs](../../../crates/dot-logger/src/logger/writer.rs), [logger/budget.rs](../../../crates/dot-logger/src/logger/budget.rs), [logger/host.rs](../../../crates/dot-logger/src/logger/host.rs), [offline.rs](../../../crates/dot-logger-client/src/offline.rs), [commands.rs](../../../crates/dot-logger-cli/src/commands.rs), [model.rs](../../../crates/dot-logger-cli/src/model.rs), [linear_reader.rs](../../../vendor/mcap-0.26.0/src/sans_io/linear_reader.rs) 및 관련 통합 시험(`tc_015_native.rs`, `tc_016_revision.rs`, `tc_017_loss.rs`, `cli.rs`, `rt-allocation`)이다.

## 3. 클래스(타입) 변경표

**이번 리뷰 세션의 제품 코드 변경은 없음.** 검토 대상 핵심 타입은 아래와 같다.

| 구분 | 소속 파일 | 타입 이름 | 역할 및 검토 요약 |
|---|---|---|---|
| 검토(신규) | [loss.rs](../../../crates/dot-logger/src/loss.rs) | `Scope`, `Cause`, `Coverage`, `LossCount`, `LossReport`, `LossDelta`, `Counters`, `Bank` | 세션 누적 손실 카운터·bounded protobuf 체크포인트·watermark 차분 (`RV-150`) |
| 검토(신규) | [storage/read.rs](../../../crates/dot-logger/src/storage/read.rs) | `FileInfo`, `NativeReader`, `LossReader` | `dot-logger/standard/1` 헤더/등록 검증 및 chunk 선검증 borrowed 판독기 (`RV-152`) |
| 검토(수정) | [storage/writer.rs](../../../crates/dot-logger/src/storage/writer.rs) | `WriterConfig`, `SegmentWriter`, `FinishedSegment` | 인접 user/RecordMeta pair 기록, baseline/주기/final loss 체크포인트 (`RV-150`) |
| 검토(수정) | [storage/retention.rs](../../../crates/dot-logger/src/storage/retention.rs) | `SegmentLog`, `RotationReason`, `RetentionBatchError` | delivery revision 경계 회전, 주기 체크포인트 및 인덱스 상한 회전 |
| 검토(수정) | [storage/recovery.rs](../../../crates/dot-logger/src/storage/recovery.rs) | `RecoveryOptions`, `RecoveryReport`, `Validation` | source별 sequence/target 검증, 원 loss report 보존 복구 (`RV-149`, `RV-152`) |
| 검토(수정) | [query/mod.rs](../../../crates/dot-logger/src/query/mod.rs) | `QueryConfig`, `QuerySource`, `Snapshot`, `LossSnapshot` | 다중 revision 조회, receipt 중복 거절, 별도 loss-only 스냅샷 |
| 검토(수정) | [capture/mod.rs](../../../crates/dot-logger/src/capture/mod.rs), [logger/writer.rs](../../../crates/dot-logger/src/logger/writer.rs) | `CaptureOwner`, `CaptureCore`, `Producer`, `GeneralProducer`, `Writer` | 손실 카운터 활성화·RT/일반 경로 거절 귀속·종료 경계 (`RV-151`) |
| 검토(수정) | [commands.rs](../../../crates/dot-logger-cli/src/commands.rs), [model.rs](../../../crates/dot-logger-cli/src/model.rs) | `Manifest`, `Source`, `Limits`, `Context` | manifest v2, `loss` 서브커맨드, 다중 source `recover` (`RV-149`, `RV-150`) |

## 4. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-149 | P1 | `storage/recovery.rs:304-318, 416-455`, `cli/src/commands.rs:651-673`, ADR-013 §2·§5, API §23.2·§23.3 | `recover_segment`가 chunk 복사 전 `options.writer.max_records`(및 출력 chunk 상한)를 선검사하지 않아 단일 chunk 내 부분 레코드가 출력에 쓰인 뒤 `NeedRotation`으로 중단되고, 앞선 chunk의 유효한 loss/user prefix가 `finish()`/`sync_all()` 없이 미완결로 남음; 루트 `op==12` peek의 `?`가 손상된 메타데이터를 `Failed`로 조기 실패시킴; CLI `recover` 성공 JSON에 `"target"` 누락 | chunk 복사 전 `report.records + count > options.writer.max_records` 및 출력 writer chunk 여유를 검사해 `RecoveryStop::Limit; break 'scan;`으로 정상 마감; 루트 peek을 `if let Ok(file)`로 변경; CLI 성공 행에 `"target"` 추가 | 수용 | Closed |
| RV-150 | P2 | `storage/writer.rs:545, 699-710`, `loss.rs:723-736, 761-819`, `cli/src/commands.rs:299-304`, ADR-013 §3, API §23.3 | 회전된 파일의 `Baseline`도 `observed_from_ns = 0`을 기록해 `summarize`의 `observed_from_ns`가 `0`으로 고정되고 CLI `--from/--to` 필터가 왜곡됨; 모든 출력이 첫 주기 체크포인트 전에 고장나도 다중 출력 `Baseline`끼리 합산돼 `delta = Some(0)`이 됨; `tail_confirmed` 및 다중 part 시간 필터 경계 미비 | `Coverage::Baseline`은 `started`를 `observed_from_ns`로 기록; `summarize`에서 실질적 관측 구간(비-Baseline 또는 동일 observer 다중 체크포인트) 및 `valid`·최신 체크포인트 기준으로 `delta`/`tail_confirmed` 판정; CLI 시간 필터를 체크포인트 그룹 단위로 적용 | 수용 | Closed |
| RV-151 | P2 | `capture/mod.rs:522-538, 1130-1152`, `logger/writer.rs:329-353`, ADR-013 §3, API §23.3 | `GeneralProducer::record_until`의 `Deadline` 분기와 `CaptureCore::loss_before_submit`(`Writer::try_log_at`의 `Encode`/`InvalidInput`)이 `ActiveCall` 내부 `closing`(`SeqCst`) 확인을 거치지 않아 종료 후에도 손실 카운터가 증가하고 `Closing` 대신 `Encode`/`InvalidInput` 반환 | `loss_before_submit`과 `record_until`이 `ActiveCall` 진입 후 `closing.load(Ordering::SeqCst)`를 확인해 종료 후 카운터 증가를 차단하고, `Writer::try_log_at`도 종료 상태를 우선 반영 | 수용 | Closed |
| RV-152 | P3 | `storage/read.rs:368-415`, `storage/recovery.rs:329-336`, API §23.1 | `read::registrations`에서 중복 `Schema`/`Channel` ID 덮어쓰기 미방지 및 `NativeReader::new` 직후 `channels()`의 스키마 이름/제어 스키마 별칭 미검증; `recover_inner`에서 `PayloadSchema::name` 초과 capacity 축소 누락 | `registrations`에서 중복 ID 및 스키마 이름 일치를 즉시 검사하고, `recovery.rs:335`에 `s.name` box slice 축소 추가 | 수용 | Closed |

---

### RV-149 — `recover_segment`의 `max_records` 미선검사로 인한 부분 chunk 반출·앞선 prefix 미완결, 루트 메타 peek 조기 실패 및 CLI `"target"` 누락 (P1)

#### [리뷰어]

- **위치:**
  - [recovery.rs:304-318, 388-458](../../../crates/dot-logger/src/storage/recovery.rs#L388-L458)
  - [commands.rs:651-673](../../../crates/dot-logger-cli/src/commands.rs#L651-L673)
  - [cli.rs:2728-2763](../../../crates/dot-logger-cli/tests/cli.rs#L2728-L2763)
  - 계약: ADR-013 §2·§5 ("한 chunk의 모든 pair·control record·참조·길이·CRC를 먼저 검증한 뒤 query/export/recovery에 반출한다. 손상 chunk의 앞부분 메시지만 정상으로 내보내지 않는다"), `recovery.rs:424` ("Validation of every nested message precedes the first append"), API §23.2·§23.3.
- **원인 및 영향:**
  1. **단일 chunk 내 부분 레코드 반출 및 앞선 검증 완료 chunk의 미완결(`finish`/`sync` 누락):**
     `recover_inner`의 `op == 6`(Chunk) 처리는 `report.loss_reports.checked_add(controls).is_none_or(|n| n > options.writer.max_loss_reports)`(`recovery.rs:416-423`)만 chunk 복사 전에 확인하여 `report.stop = RecoveryStop::Limit; break 'scan;`으로 빠져나가고, 사용자 레코드 상한(`report.records.checked_add(count).is_none_or(|n| n > options.writer.max_records)`)과 출력 `SegmentWriter`의 `max_chunks` 여유는 chunk 복사 전에 확인하지 않는다.
     - **부분 chunk 기록:** 한 source chunk 안에 사용자 레코드가 2건 이상 있을 때(예: `fault_storage.rs`의 기본 `fixture`는 chunk당 2건) `options.writer.max_records = 1`로 복구를 호출하면, chunk 내부 루프(`recovery.rs:426-450`)에서 첫 번째 사용자 레코드는 `writer.append`에 성공해 출력 sink에 기록된 뒤 같은 chunk의 두 번째 레코드에서 `Err(StorageError::NeedRotation)`이 발생한다. 즉, chunk 전체가 출력 한도에 들어가는지 확인하기 전에 부분 chunk가 먼저 출력에 쓰인다.
     - **앞선 정상 chunk 미완결:** `writer.append`가 `Err(StorageError::NeedRotation)`을 반환하면 `recovery.rs:448`의 `?`가 `'scan`을 정상 종료하는 대신 `recover_inner` 전체를 `Err(Some(StorageError::NeedRotation))`으로 즉시 중단시켜 `writer.finish()`와 `finished.sync_recovered()`(`recovery.rs:489-490`)를 건너뛴다. 그 결과 앞선 chunk에서 이미 검증·복사된 사용자 레코드나 `Baseline` loss report가 MCAP Summary/Footer 없이 닫히지 않고 파일 동기화도 수행되지 않는다. 실제로 `cli.rs:2728-2763`(`fanout = true`)에서 두 번째 파일(`20-0.mcap`)은 앞선 Chunk 1·2에서 `Baseline` loss report 2건을 정상 복사했음에도 Chunk 3의 사용자 레코드에서 `NeedRotation`으로 `Err` 반환되어 `loss_reports`가 완결된 파일로 보존되지 못했다.
  2. **루트 `dot_logger.file`(`op == 12`) 선행 peek의 `?` 조기 실패 (`recovery.rs:308`):**
     `recover_inner`는 `SegmentWriter`를 생성하기 전에 첫 레코드가 `op == 12`이면 `let file = super::FileInfo::parse(&buffer[..len])?;`(`recovery.rs:308`)를 호출한 뒤 다시 `root_start`로 되감는다. 이때 `FileInfo::parse`에 `?`가 붙어 있어, `op == 12`의 본문이 손상된 파일에서는 `'scan` 루프의 `12 => state.metadata(body)`(`recovery.rs:374, 617` — `RecoveryStop::InvalidRecord`로 분류하도록 설계된 분기)에 도달하지 못하고 `Err(Some(StorageError::CorruptData))` → `RecoveryStop::Failed`(CLI exit 4 `io_or_transport`)로 조기 실패한다.
  3. **CLI `recover` 성공 JSON 행의 `"target"` 필드 누락 (`commands.rs:666`):**
     `recover_one`에서 `Err` 분기(`commands.rs:657`)는 `"target": source.target.map(|id| id.to_string())`를 출력하지만, `Ok` 분기(`commands.rs:666`)는 `"target"`을 출력하지 않아 다중 출력 복구 시 성공 행에서만 `target` 식별자가 빠진다.
- **처리 제안:**
  1. `recovery.rs:416`에서 `report.loss_reports` 검사와 나란히 `report.records.checked_add(count).is_none_or(|n| n > options.writer.max_records)` 및 출력 `writer`의 남은 `max_chunks` 여유를 검사하여, 한도 초과 시 chunk의 어떤 메시지도 쓰기 전에 `report.stop = RecoveryStop::Limit; break 'scan;`으로 빠져나가 앞선 검증된 chunk들을 `writer.finish()`·`sync_recovered()`로 완결한다.
  2. `recovery.rs:308`을 `if let Ok(file) = super::FileInfo::parse(&buffer[..len])`로 변경하여 손상된 루트 메타데이터가 `'scan` 루프의 `state.metadata(body)`를 통해 일관되게 `RecoveryStop::InvalidRecord`로 처리되게 한다.
  3. `commands.rs:666`의 `out.emit("recovery", ...)`에 `"target": source.target.map(|id| id.to_string())`를 추가한다.

#### [처리담당]

- **판정:** 수용 — 승인된 ADR-013의 완전 chunk 반출·prefix 복구 계약 복원.
- **처리:** `recover_inner`는 max_records와 loss part 총량을 선검사하고 `SegmentWriter::recovery_chunk_fits`로 출력 chunk target에 따른 user split·control flush·초기 등록 chunk 비용까지 재인코딩해 검사한다. 한도는 `Limit`로 scan을 끝내고 `finish`/`sync_recovered`한다. 루트 identity peek의 parse 실패는 scan의 `InvalidRecord` 분류로 넘긴다. CLI 성공 행에 target을 추가했다.
- **검증:** [EV-086](../verification_evidence.md#ev-086). `fault_storage::exhausted_output_capacity_finishes_only_whole_source_chunks`(max_records1/3, max_chunks1, 작은 출력 target의 경계/정확 수락), `damaged_root_metadata_returns_a_finished_invalid_record_prefix` Red→Green. `cli::recovery_user_record_quota_does_not_consume_control_only_later_sources`는 fanout의 두 Baseline 보존·file_synced·Limit 및 양쪽 target을 대조한다. OS IO/취소의 기존 실패·비발행 규칙은 회귀로 확인했다.

---

### RV-150 — 회전 세그먼트 `Baseline`의 `observed_from_ns = 0` 고정, 다중 출력 단일 `Baseline` 간 `delta = Some(0)` 오판 및 다중 part 시간 필터 경계 (P2)

#### [리뷰어]

- **위치:**
  - [writer.rs:545, 558, 699-710](../../../crates/dot-logger/src/storage/writer.rs#L699-L710) (`SegmentWriter::with_target`, `SegmentWriter::collect_loss`)
  - [loss.rs:723-736, 761-819](../../../crates/dot-logger/src/loss.rs#L761-L819) (`summarize`)
  - [commands.rs:299-304](../../../crates/dot-logger-cli/src/commands.rs#L299-L304) (`local` `"loss"`)
  - 계약: ADR-013 §3 ("이전 표본 읽기 시작부터 현재 표본 읽기 종료까지를 delta의 보수적 관측 구간으로 사용한다... 처음 baseline 이전/끝 checkpoint 이후는 미확인이다... 파일이 없거나 report가 없다는 사실을 0손실 증거로 쓰지 않는다"), API §23.3.
- **원인 및 영향:**
  1. **회전 세그먼트 `Baseline`의 `observed_from_ns = 0` 기록 (`writer.rs:545, 704`):**
     `SegmentWriter::with_target`은 세그먼트 회전으로 생성되는 후속 파일(`sequence >= 1`)에서도 `sample_start: 0`으로 시작하여 첫 `collect_loss(Coverage::Baseline, false)` 호출 시 `observed_from_ns = 0`, `observed_to_ns = T_open`을 기록한다.
     - 그 결과 시각 $T_1$에 회전으로 열린 파일 `1.mcap`만 조회하거나 앞선 `0.mcap`이 ring 보존으로 삭제된 상태에서 `summarize`를 호출하면, `1.mcap`의 `Baseline`($T_1$)과 이후 체크포인트($T_2$) 사이의 관측 구간이 `[T_1, T_2]`가 아니라 `from.min(r.observed_from_ns)`에 의해 **`[0, T_2]`(`observed_from_ns = 0`)로 왜곡**된다(`0..T_1`은 `1.mcap`의 baseline 이전 미관측 구간임).
     - 또한 CLI `loss --from <A> --to <B>`(`commands.rs:301`)에서 `1.mcap`이 열리기 한참 전인 구간($B \le T_1$)을 조회해도 `1.mcap`의 `Baseline`(`0 < B && T_1 >= A`)이 겹침 조건을 만족해 잘못 선택된다.
  2. **다중 출력이 모두 첫 주기 체크포인트 전에 고장났을 때 초기 `Baseline`끼리 비교해 `delta = Some(0)`을 반환 (`loss.rs:782-790`):**
     `summarize`는 `complete_count`를 observer 구분 없이 모든 complete 체크포인트에 대해 누적한다. 따라서 2개 출력(`target 10`, `target 20`)이 모두 파일 생성 시점의 `Baseline`(`checkpoint = 1, count = 0`)만 기록한 뒤 손실이 발생하고 첫 주기 체크포인트에서 IO 고장이 나면(`tc_017_loss.rs:1076-1160`의 `all_failed = true`), 각 파일 단독으로는 `delta == None`이지만 두 파일의 prefix를 함께 `summarize`에 전달하면 두 초기 `Baseline`이 합산(`complete_count == 2`)되어 **모든 손실 항목의 `delta`가 `Some(0)`으로 둔갑**한다.
  3. **`tail_confirmed` 판정 및 다중 part 체크포인트의 개별 part 시간 필터링 (`loss.rs:807-809`, `commands.rs:299-303`):**
     - `loss.rs:807`의 `tail_confirmed`는 `valid == false`(동일 observer의 카운터 감소·중복 checkpoint 등 무효 상태)이거나 `Quiescent` 체크포인트보다 나중 시점/번호의 미완성(`complete == false`) 또는 비-`Quiescent` 체크포인트가 존재하는 경우에도 `c.count == max`이기만 하면 `true`가 된다.
     - `commands.rs:301`은 `parts > 1`인 체크포인트를 그룹화하기 전에 개별 `LossReport` part의 `observed_to_ns >= from`으로 필터링하므로, part 간 `self.now()` 차이($t_0 < \text{from} \le t_1$)로 인해 동일 체크포인트의 앞 part만 잘려 나가 완전한 체크포인트가 incomplete로 깨질 수 있다.
- **처리 제안:**
  1. `SegmentWriter::collect_loss`에서 `Coverage::Baseline`일 때는 `self.sample_start`(`0`) 대신 이번 스캔 시작 시각인 `started`를 `bank.sample`의 `from` 인자로 전달한다.
  2. `loss::summarize`에서 모든 complete 관측이 각 observer의 초기 `Baseline`뿐인 경우(동일 observer의 2개 이상 complete 체크포인트나 비-`Baseline` complete 체크포인트가 없는 경우) `delta`를 `None`으로 유지하고, `tail_confirmed`는 무결한(`valid` 및 비중복) 관측 중 가장 최신 체크포인트가 complete `Quiescent`이고 `c.count == max`일 때만 `true`로 설정한다.
  3. `commands.rs:299-303`에서 `(session, epoch, observer, checkpoint)` 그룹의 전체 관측 구간 단위로 `--from`/`--to` 필터를 적용한다.

#### [처리담당]

- **판정:** 수용 — 초기 Baseline 이전·마지막 성공 checkpoint 이후의 미관측 구간을 0손실로 표현하지 않음.
- **처리:** `collect_loss`의 Baseline 시작점을 이번 읽기 시작으로 설정했다. `summarize`는 초기 replica만 있는 경우 delta=None을 유지하고, 중복/감소/overflow 유효성과 session/epoch 최신 multipart 그룹의 complete Quiescent 여부를 분리해 꼬리를 판정한다. CLI 시간 필터는 checkpoint 그룹의 전체 구간에 적용한다.
- **검증:** [EV-086](../verification_evidence.md#ev-086). `loss::tests::baseline_replicas_and_stale_quiescent_reports_do_not_confirm_unobserved_work`, TC-017의 전 출력 IO 실패·saturated baseline 시험 Red→Green. 신규 `rotated_file_baselines_start_at_their_own_observation`은 실제 sequence1 파일의 시작점과 summarize 구간을 대조한다. `commands::loss_tests::time_windows_keep_all_parts_of_each_selected_checkpoint`는 part 사이 from 경계에서 개별 필터 변이 Red→그룹 필터 Green을 확인했다.

---

### RV-151 — 종료(`closing`) 경계에서 `loss_before_submit` 및 `GeneralProducer::record_until`의 `ActiveCall`·`closing` 확인 누락 (P2)

#### [리뷰어]

- **위치:**
  - [capture/mod.rs:522-538, 1130-1152](../../../crates/dot-logger/src/capture/mod.rs#L522-L538) (`CaptureCore::loss_before_submit`, `GeneralProducer::record_until`)
  - [logger/writer.rs:329-353](../../../crates/dot-logger/src/logger/writer.rs#L329-L353) (`Writer::try_log_at`)
  - [logger/host.rs:522-529](../../../crates/dot-logger/src/logger/host.rs#L522-L529)
  - 계약: ADR-013 §3 ("종료 후 호출이 무한히 증가시킬 수 있는 Closing 거절은 저장 종료 watermark 밖"), API §23.3 ("Closing 이후 시도는 저장 종료 watermark 밖이다"), REV-058 ("active call guard가 counter 증가까지 감싼다").
- **원인 및 영향:**
  1. **`GeneralProducer::record_until`의 `ActiveCall` 진입 후 `closing` 미확인 (`capture/mod.rs:1131-1151`):**
     `Producer`의 `try_record_small`, `try_acquire`, `try_record_raw`는 모두 `completion.enter`(`active.store(true, SeqCst)`) 진입 **후**에 `self.closing.load(Ordering::SeqCst)`를 검사하여 종료 코디네이터(`closing.store(true, SeqCst)` → `active_calls() == 0`)와의 Dekker 순서를 보장한다. 반면 `GeneralProducer::record_until`은 루프 시작(`1131행`)에서 `ActiveCall` 밖에서 `closing.load(Ordering::Acquire)`만 확인하고, `Instant::now() >= deadline`(`1134행`) 분기에서 `completion.enter`에 진입한 뒤에는 `closing.load(Ordering::SeqCst)`를 재확인하지 않고 `bank.ingress(..., Cause::Deadline)`을 호출한다. 따라서 `1131행`과 `1135행` 사이에 종료가 완료되면 종료 이후에 `Cause::Deadline` 카운터가 증가하고 `Closing` 대신 `DeadlineExceeded`가 반환될 수 있다.
  2. **`CaptureCore::loss_before_submit` 및 `Writer::try_log_at`의 `closing`·`ActiveCall` 누락 (`capture/mod.rs:522-533`, `logger/writer.rs:331-352`):**
     `Writer::try_log_at`은 로거가 이미 닫힌 상태(`core.closing == true`)에서도 `channel.session != self.session`과 `message.encode()`를 먼저 실행하고 실패 시 `loss_before_submit`을 호출한다. `loss_before_submit` 역시 `completion.enter`나 `self.closing.load(Ordering::SeqCst)`를 확인하지 않으므로, 로거 종료 후에도 `Encode` 또는 `InvalidInput` 시도가 `bank.ingress` 카운터를 증가시키며 반환값도 `LogError::Closing`이 아닌 `LogError::Encode`/`LogError::InvalidInput`이 된다(반면 인코딩이 성공한 메시지는 `try_record_small`에서 `LogError::Closing`으로 거절되고 카운터를 증가시키지 않음).
- **처리 제안:**
  1. `CaptureCore::loss_before_submit`(또는 통합된 헬퍼)에서 `_active = self.completion().map(|c| c.enter(producer as u32));`를 획득한 뒤 `if self.closing.load(Ordering::SeqCst) { return; }`를 확인하고, `Writer::try_log_at`에서도 `owned.core`가 이미 닫혔다면 `LogError::Closing`을 반환하도록 정렬한다.
  2. `GeneralProducer::record_until`의 `Instant::now() >= deadline` 분기에서 `_active` 획득 직후 `if self.producer.closing.load(Ordering::SeqCst) { return Err(CaptureError::Closing); }`를 확인한다.

#### [처리담당]

- **판정:** 수용 — 종료가 먼저 확정된 호출은 Closing 반환·저장 watermark 밖으로 유지.
- **처리:** `loss_before_submit`은 ActiveCall 진입 뒤 SeqCst closing을 읽고 종료 시 CaptureError::Closing을 반환한다. `try_log_at`은 초기 closing을 우선 검사하고 encode/foreign-channel 실패의 helper 결과도 전파한다. Deadline 분기도 ActiveCall 직후 closing을 다시 검사한다. RT 경로에 할당·해제·I/O·재시도/lock을 추가하지 않았다.
- **검증:** [EV-086](../verification_evidence.md#ev-086). `capture::tests::pre_submit_failures_after_closing_do_not_change_loss_watermarks`와 TC-017의 outstanding writer encode 결과 Red→Green. `logger::writer::closing_tests::closing_during_encoding_outranks_encode_and_foreign_channel_errors`는 encode 내부에서 close해 겹침 순서를 결정적으로 만들고 encode/foreign-channel보다 Closing이 우선함을 확인한다. 일반 Deadline 기존 회귀와 RT allocation20개를 통과했다. 모든 동시 실행 순서를 증명한 것은 아니다.

---

### RV-152 — `read::registrations`의 중복 등록 ID 검사/초기 스키마 검증 정합성 및 `recover_inner`의 `PayloadSchema::name` capacity 축소 누락 (P3)

#### [리뷰어]

- **위치:**
  - [read.rs:368-415](../../../crates/dot-logger/src/storage/read.rs#L368-L415) (`registrations`)
  - [recovery.rs:329-336](../../../crates/dot-logger/src/storage/recovery.rs#L329-L336) (`recover_inner`)
- **내용:**
  1. `read::registrations`는 `Record::Schema` 처리 시 `if schemas.len() > file.channels + 2`만 검사하고 `schemas.insert(header.id, ...)`가 기존 키를 덮어쓰는지 확인하지 않으며(`Record::Channel`의 `channels.insert(c.id, ...)`도 동일), `Record::Schema`의 `header.name`이 채널의 `payload_schema_name`(또는 `payload.<id>.v<ver>`)과 일치하는지·제어 스키마(`dot_logger.RecordMeta`, `dot_logger.LossReport`)를 사용자 채널이 참조하는지는 첫 `next()` 호출 시점의 `Validation`까지 지연된다. `registrations`에서도 중복 `header.id`/`c.id` 덮어쓰기(`insert(...).is_some()`)와 제어 스키마 참조를 즉시 거절하면 `NativeReader::new` 직후 `reader.channels()` 조회 시점에도 완전한 불변식이 보장된다.
  2. `recovery.rs:329-336`에서 호출자로부터 전달받은 `channels`의 초과 capacity를 정리할 때 `s.encoding`과 `s.descriptor`는 `into_boxed_str`/`into_boxed_slice`로 축소하지만, `s.name = s.name.take().map(|n| n.into_boxed_str().into_string());`는 빠져 있다(`writer.rs:424` 및 `retention.rs:374`와 동일하게 맞추는 것이 일관됨).

#### [처리담당]

- **판정:** 수용 — 초기 reader 등록 불변식 및 caller capacity 정리 정렬.
- **처리:** `registrations`에서 Schema/Channel 중복 ID를 거절하고 user schema 이름을 명시 metadata 또는 payload.<id>.v<ver>와 대조한다. user의 제어 schema 참조도 즉시 거절한다. `recover_inner`는 PayloadSchema.name도 boxed string으로 축소한다.
- **검증:** [EV-086](../verification_evidence.md#ev-086). `tc_015_native::registrations_reject_duplicate_ids_names_and_control_schema_aliases_at_construction`의 중복 Schema/Channel ID·이름 변조·제어 별칭 네 조건을 new() 시점에서 거절한다(중복 Schema ID 기존 수락 Red 확인). 기존 native writer/reader·복구 및 RT 회귀 통과.

---

## 5. 검증 및 인계

### 검증 실행 결과

| 검증 | 결과 |
|---|---|
| `cargo +1.96.0 fmt --all --check` | Pass |
| `cargo +1.96.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass |
| `cargo +1.96.0 test --workspace --all-features --locked` | Pass — 665 passed, 0 failed, 5 ignored(외부 DB 시험), RT 할당 20 passed 포함 |
| `RUSTDOCFLAGS='-D warnings' cargo +1.96.0 doc --workspace --all-features --no-deps --locked` | Pass |
| `RUSTDOCFLAGS='-D warnings' cargo +1.96.0 doc -p dot-logger --no-deps --locked` | Pass |
| `python3 scripts/check-integration-surface.py` | Pass — facade 이름 있는 항목 34/60, L1~L3 레시피·실행·README 일치 |
| 제품 코드·저장소 테스트 변경 | 없음 (Strict Read-Only 준수) |

### 체크리스트 대조 요약

1. **RT(실시간) 경로 제약:** `Producer::try_record_small`, `try_acquire`, `try_record_raw`, `Writer::try_log_at`, `Loan::drop`, `Writer::drop` 경로에 동적 할당/해제, 블로킹 lock, I/O, 문자열 포맷팅, CAS 무한 루프가 없음을 코드와 `dot-logger-rt-allocation` 20개 시험으로 확인했다. `Cell::single`은 producer 단독 `Relaxed` load/store를 사용하고 공유 RMW(`Cell::shared`)는 일반 경로(`Completion::report`/`reconcile`)에서만 실행된다.
2. **Rust 언어 및 동시성/메모리 안전성:** 신규 `unsafe` 블록 추가는 없으며, `RecordSlot::channel_index`는 `Reservation::commit`에서 `slot.phase.store(..., Release)` 전에 기록되고 `Completion::report`/`reconcile`에서 `phase` `Acquire`/`AcqRel` 성공 후에만 읽힌다. 다만 종료 경계에서의 `loss_before_submit` 및 `record_until`의 `ActiveCall`/`closing` 순서는 `RV-151`(P2)로 보완이 필요하다.
3. **아키텍처·API 경계 및 유한 예산:** `WireReport`/`WireCount`/`Meta` 등 protobuf wire 타입은 모듈 내부 비공개로 격리되었고, `bounded_wire`와 `FileInfo::parse`가 `prost`/MCAP 할당 전에 입력 폭과 필드 수를 제한한다. 복구 경로의 `max_records` chunk 경계 선검사는 `RV-149`(P1), 회전 파일 `Baseline` 관측 구간과 다중 출력 `Baseline` 차분 조건은 `RV-150`(P2)으로 조치한다.

### 다음 개발 세션 인계

- 다음 개발 세션은 **TASK-034 runtime 통합(D) 착수 전에** 본 문서(`REV-059`)의 `RV-149`(P1), `RV-150~151`(P2), `RV-152`(P3)에 대해 실패 재현 테스트(Red) 작성 → 최소 코드 수정(Green) → 회귀 검증을 수행하고 `[처리담당]` 영역을 갱신한다.


## 6. 개발 세션 최종 처리 — 2026-11-17

Human의 브리핑·리뷰 지적사항 수정 요청과 기존 위임에 따라 `Open → In Progress → Resolved → Closed`로 처리했다. 제품/시험 커밋은 `8b59aa6`, 검증은 [EV-086](../verification_evidence.md#ev-086)이다. 앞의 리뷰어 원문·최초 검증은 당시 이력으로 보존한다.

| 구분 | 변경 파일 | 내용 |
|---|---|---|
| 수정 | `storage/recovery.rs`·`storage/writer.rs` | 복구 chunk 선검사·prefix finish/sync·root 분류·Baseline 시작·schema.name 축소 |
| 수정 | `loss.rs`·`storage/read.rs` | 초기 replica/최신 꼬리 판정·등록 ID/이름 즉시 검증 |
| 수정 | `capture/mod.rs`·`logger/writer.rs` | ActiveCall/SeqCst 종료 경계·Closing 우선 반환 |
| 수정 | CLI `commands.rs`·`tests/cli.rs` | 그룹 시간 필터·recovery target·control prefix 시험 |
| 수정 | `capture/tests.rs`·`fault_storage.rs`·`tc_015_native.rs`·`tc_017_loss.rs` 및 위 모듈 단위 시험 | 7개 신규·기존 시험 확장, Red→Green 및 전체 회귀 |

| 타입 | 동작 변경 |
|---|---|
| `SegmentWriter` | 복구 출력 chunk 수 시뮬레이션·파일별 Baseline 시작 |
| `CaptureCore`·`GeneralProducer`·facade `Writer` | 종료가 먼저면 손실 counter를 늘리지 않고 Closing 반환 |
| `NativeReader` | new()에서 등록 불변식 검증 강화 |
| `LossDelta` | 타입 선언 변경 없이 summarize의 delta/tail 판정 강화 |

자가검토: 새 unsafe·RT 할당/해제·I/O·lock·무한 재시도 없음. ActiveCall의 기존 SeqCst/Dekker 종료 순서를 실패 counter까지 확장한다. 복구 선검사는 출력 metadata를 실제로 재인코딩해 비정규 source protobuf 길이나 더 작은 output chunk target에도 한도 초과 전에 멈춘다. 전체 각672·RT20·Miri7·정적 검사·가드 Pass이며 GUI/호출 비용/최종 커버리지·native/장기/실앱은 이번 Pass 범위 밖이다. 다음은 D runtime 통합이다.

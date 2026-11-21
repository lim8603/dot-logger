# REV-055 — 표준 MCAP export·S3/S4 시험·client diagnostics 이동·facade 노출 누적 코드리뷰

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-055 / 2026-11-15 |
| 리뷰어 | Antigravity AI — 리뷰 전용 세션 |
| 대상 | REV-054 이후 누적 코드 커밋 `a536486..fa72a1a` (9개 코드 커밋: `3cbd1b2`, `783f2e3`, `17ebda6`, `3a554ac`, `8703459`, `ce0efcd`, `a543432`, `67fa480`, `fa72a1a`, WIP `36c1724` 제외) |
| 범위 | TASK-030 S1 표준 MCAP export, TASK-033 S3/S4 시험 및 ring `2 × segment` 하한 수정, client `diagnostics` 이동, TASK-034 S2a~S2c(중요도·시작 시 최소 level·이벤트 보존) |
| 종합 판정 | **Conditional — P1 1건·P2 3건·P3 1건 (`RV-136~140`)** |
| 상태 | **Closed** — 2026-11-16 RV-136~140 조치·검증·위임 수락 |
| 조치 판정 | 로컬 **Pass**, 소스 `3e6dcb5`, [EV-082](../verification_evidence.md#ev-082) |

전체 회귀(1.96.0 및 MSRV 1.85.0 각 625 passed·5 ignored), `clippy`, `fmt`, strict `rustdoc`, `--no-default-features` 빌드, NFR-008 표면/레시피 검사(`34/60` 항목), RT 무할당 시험(19개)은 모두 통과했다. 그러나 심층 검토 및 `/tmp/dot-logger-rev055` 재현 진단 결과:
1. **P1 (`RV-136`):** 손상된 무압축 chunk를 `standard::export`로 읽을 때 `vendor/mcap` `LinearReader`에서 산술 언더플로 패닉(release에서는 `u64` wrap)과 CLI 실패 시 출력 파일 미삭제가 발생하고, `crc == 0` 또는 chunk 내부 레코드 파싱 실패 시 검증되지 않은 chunk의 부분 메시지가 출력에 포함되는 결함을 실제 재현했다.
2. **P2 (`RV-137~139`):** 다중 이벤트 출력의 롤백 누락·오류 우선순위(`worse`) 역전·IO 패닉(`drain_failed`) 시 `target.marks` 미정리로 인한 대기 지연(`RV-137`), 이벤트 만료(`expire`) 실행 시점 순서로 인한 기한 경과 후 거짓 `Quota` 및 `write_batch` 선행 실패 가능성(`RV-138`), `standard::export`의 `payload_schema_file_id` 자기참조(`dot_logger.Envelope` 노출) 및 재등장 채널 메타데이터 미검증(`RV-139`)을 확인했다.
3. **P3 (`RV-140`):** `SegmentLog::new`의 `PayloadSchema::name` 초과 capacity 축소 누락 및 `standard::export` 오류 시 출력 스트림 상태를 정리했다.

리뷰 전용 세션 원칙에 따라 제품 코드·저장소 테스트는 수정하지 않았다.

## 2. 파일 변경표

| 구분 | 파일 | 변경 내용 |
|---|---|---|
| 생성 | [REV-055](REV-055_FACADE_STANDARD_EXPORT_REVIEW.md) | `a536486..fa72a1a` 심층 리뷰 결과(`RV-136~140`), 재현 근거 및 처리담당 영역 기록 (`Open`) |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | 미처리 리뷰 목록에 REV-055 (`Open`, P1 1·P2 3·P3 1) 등록 및 다음 시작점 동기화 |

검토한 핵심 파일은 `crates/dot-logger/src/logger/{event.rs, budget.rs, host.rs, mod.rs, writer.rs}`, `crates/dot-logger/src/storage/{standard.rs, writer.rs, recovery.rs, retention.rs, catalog.rs}`, `vendor/mcap-0.26.0/src/sans_io/linear_reader.rs`, `crates/dot-logger-cli/src/{args.rs, commands.rs, main.rs, model.rs}`, `crates/dot-logger-client/{src/lib.rs, examples/support/dot_probe*}`, 관련 단위·통합 시험(`tc_013_facade.rs`, `tc_014_standard.rs`, `tc_003_completion.rs`, `tc_009_report.rs`, `tc_013_tracing.rs`, `cli.rs`, `rt-allocation`)이다.

## 3. 클래스(타입) 변경표

**이번 리뷰 세션의 코드 변경은 없음.** 검토 대상 신규·수정 타입은 아래와 같다.

| 구분 | 소속 파일 | 타입 이름 | 역할 및 검토 요약 |
|---|---|---|---|
| 검토(신규) | `crates/dot-logger/src/logger/event.rs` | `EventMark`, `EventError`, `EventSpec`, `Marks`, `EventWork`, `Held` | facade 이벤트 보존 핸들·오류·IO 스레드 상태 (`RV-137`, `RV-138`) |
| 검토(신규) | `crates/dot-logger/src/logger/budget.rs` | `EventWindow` | `Retention::events`용 pre/post/hold/limits 설정 |
| 검토(수정) | `crates/dot-logger/src/logger/writer.rs` | `Channel<T>` | `importance: Option<Importance>` 필드 추가 (`Copy`·RT 무할당 유지) |
| 검토(수정) | `crates/dot-logger/src/logger/mod.rs` | `ChannelOptions`, `PlanWarning`, `WriterQuota` | `important_from`, `minimum`, `NoImportantReserve`, `important(slots)` 추가 |
| 검토(신규) | `crates/dot-logger/src/storage/standard.rs` | `ExportLimits`, `ExportedChannel`, `ExportReport`, `ExportError` | `dot-logger/1` → 표준 MCAP 변환 (`RV-136`, `RV-139`, `RV-140`) |
| 검토(이동) | `crates/dot-logger-client/examples/support/dot_probe/check.rs` | `SnapshotCheck` | `dot_logger_client::diagnostics`에서 예제 지원 모듈(`#[path]`)로 이동 |

## 4. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-136 | P1 | `vendor/mcap-0.26.0/.../linear_reader.rs:672,745`, `storage/standard.rs:161-203`, `cli/src/commands.rs:919-936`, API §21.2 | 손상된 무압축 chunk 내부 레코드 길이 초과 시 언더플로 패닉(release wrap) 및 CLI 출력 파일 미삭제; `crc==0` 또는 chunk 내부 레코드 오류 시 미검증 chunk의 부분 메시지가 출력에 기록됨 | `LinearReader` 무압축 chunk 경계 검사 추가, chunk 단위 선검증(또는 `crc==0` 거절/선로드) 보장, CLI 출력 파일 정리 가드 보강 | 수용 | Closed |
| RV-137 | P2 | `logger/mod.rs:1106-1171`, `logger/host.rs:682-693,994-1021`, API §22.3·22.9 | 다중 이벤트 출력 롤백 시 `worker_failed`/첫 `Closing`에서 조기 반환해 후속 출력 해제 누락; `Disconnected` 시 `worse(result, Closing)` 미적용으로 `Failed` 덮어씀; `drain_failed`에서 `target.marks` 미정리로 `overdue` 전체 대기 | 롤백 전송을 전 출력 best-effort로 분리, `Disconnected`/`Timeout`에도 `worse()` 반영, `drain_failed` 진입 시 `target.marks = None` 처리 | 수용 | Closed |
| RV-138 | P2 | `logger/event.rs:135-159`, `logger/host.rs:715-740,777-784`, API §22.3·22.9 | `Marks::mark`가 `protect_event` 전에 만료를 정리하지 않아 `at + post + hold` 경과 후에도 다음 `sync_every` 전까지 `EventError::Quota` 발생; `io_loop`에서 `write_batch`가 `expire`보다 먼저 실행됨 | `Marks::mark` 진입 시 `self.expire(log, now)` 호출, `io_loop`에서 주기 만료를 `write_batch` 전에 수행 | 수용 | Closed |
| RV-139 | P2 | `storage/standard.rs:226-263`, API §21.1·21.2 | `payload_schema_file_id`가 `dot_logger.Envelope` 스키마를 가리켜도 허용되어 출력에 Envelope 스키마 노출; 동일 `c.id` 채널 재등장 시 `topic` 외 필드 불일치 미검사 | `payload_schema_file_id == c.schema_id`(및 빈 `payload_schema_name`) 거절, 재등장 채널의 `(schema_id, topic, message_encoding, metadata)` 일치 검증 | 수용 | Closed |
| RV-140 | P3 | `storage/retention.rs:313-321`, `storage/standard.rs:106-123,169-170` | `SegmentLog::new`에서 `schema.name` 초과 capacity 축소 누락(`writer.rs`와 비대칭); `standard::export` 실패 시 `output` 부분 기록 안내 및 메타 맵 상한 참고 | `retention.rs`에 `schema.name` shrink 추가, `export` 오류 시 `output` 폐기 조건 rustdoc 명시 | 수용 | Closed |

---

### RV-136 — 손상된 chunk에서 `standard::export` 언더플로 패닉·CLI 출력 잔존 및 미검증 chunk 부분 메시지 반출 (P1)

#### [리뷰어]

- **위치:**
  - [linear_reader.rs:672-676, 730-746](../../../vendor/mcap-0.26.0/src/sans_io/linear_reader.rs#L672-L746)
  - [standard.rs:13-17, 161-203, 284-330](https://github.com/lim8603/dot-logger/blob/fa72a1a/crates/dot-logger/src/storage/standard.rs#L161-L203)
  - [commands.rs:904-937](../../../crates/dot-logger-cli/src/commands.rs#L904-L937)
  - 계약: API §21.2 ("잘린 파일·CRC 불일치 chunk·해석 불가 레코드를 만나면 그 앞의 검증된 chunk까지 내보내고 `ExportReport::truncated`에 이유를 남긴다... 실패·취소 시 만든 출력을 지운다"), `standard.rs:16-17` ("a chunk is checked before any of its messages is written").
- **원인:**
  1. **무압축 chunk 내부 레코드 경계 미검사 및 언더플로 패닉:** `vendor/mcap-0.26.0`의 `LinearReader`는 압축된 chunk(`Some(decompressor)`)에서는 `state.uncompressed_remaining`을 검사하지만, 무압축 chunk(`None`, `dot-logger/1`의 기본 형식)에서는 내부 레코드의 `(OPCODE_LEN_SIZE + len) as u64 <= state.compressed_remaining` 여부를 확인하지 않고 `self.file_data`에서 `consume!(OPCODE_LEN_SIZE + len)`을 읽은 뒤 `state.compressed_remaining -= (OPCODE_LEN_SIZE + len) as u64;`(`linear_reader.rs:745`)를 수행한다. 손상된 chunk 내부 레코드의 `len`이 남은 `compressed_remaining`을 넘으면 debug 빌드에서 즉시 패닉이 나고, release 빌드에서는 `u64::MAX`로 wrap되어 chunk 뒤의 파일 바이트를 chunk 내부 레코드로 잘못 읽는다. 또한 CLI `export_standard`는 패닉 언와인드 시 `std::fs::remove_file(target)`에 도달하지 못해 생성된 출력 파일을 남긴다.
  2. **미검증/손상 chunk의 부분 메시지 반출:**
     - `LinearReader`는 `if self.options.prevalidate_chunk_crcs && state.crc != 0`(`linear_reader.rs:672`)일 때만 chunk 전체를 선로드·검증한다. 손상된 파일의 chunk 헤더에서 `uncompressed_crc == 0`이면 선검증과 `ChunkTooLarge` 선검사를 건너뛰고 즉시 레코드를 스트리밍하므로, chunk 중간이 잘려 있어도 앞부분 메시지가 먼저 출력 파일에 기록된다.
     - `crc != 0`으로 CRC가 일치하는 chunk라도, `standard::export`는 chunk 내부 첫 `Message`를 받는 즉시 `w.write_to_known_channel`로 출력에 쓴다. 같은 chunk의 뒤따르는 레코드에서 `mcap::parse_record`가 실패하면 `standard.rs:198`의 `Err(e) if writer.is_some()`이 `report.truncated = Some(...)`만 설정하고 루프를 종료해, 손상된 chunk의 일부 메시지가 포함된 출력 파일을 정상 완료(`Ok`)로 반환한다.
- **재현 (`/tmp/dot-logger-rev055`):**
  1. `underflow.mcap` (10바이트 무압축 chunk 내부에 `opcode=5, len=10` 헤더와 패딩 10바이트, 유효 CRC 설정):
     - 실행: `/data/build/target/debug/dot-logger export --standard --input /tmp/dot-logger-rev055/underflow.mcap --output /tmp/dot-logger-rev055/underflow_out.mcap`
     - 관측: `thread 'tokio-rt-worker' panicked at vendor/mcap-0.26.0/src/sans_io/linear_reader.rs:745:29: attempt to subtract with overflow` (exit 4), 그리고 **`/tmp/dot-logger-rev055/underflow_out.mcap` 파일이 삭제되지 않고 디스크에 남음**.
  2. `partial_chunk.mcap` (유효 CRC를 가진 단일 chunk 안에 유효 `Message` 1개 + 1바이트짜리 깨진 `Schema` 레코드 1개):
     - 실행 결과: exit 0, `{"messages":1,"truncated":"Record parse failed"}` — 첫 번째 완전한 chunk가 하나도 없는데도 손상된 chunk 내부의 첫 메시지가 출력 파일에 기록됨.
  3. `crc0_trunc.mcap` (`uncompressed_crc = 0`인 단일 chunk 안에 유효 `Message` 2개 중 2번째 메시지 끝 10바이트를 잘라냄):
     - 실행 결과: exit 0, `{"messages":1,"truncated":"MCAP file ended in the middle of a record"}` — 잘린 chunk의 첫 메시지가 출력 파일에 기록됨.
- **처리 제안:**
  1. `vendor/mcap-0.26.0/src/sans_io/linear_reader.rs`의 무압축 `ChunkRecord`(또는 `ValidatingChunkCrc`)에서 `(OPCODE_LEN_SIZE + len) as u64 > state.compressed_remaining`(및 남은 바이트 `< OPCODE_LEN_SIZE`)일 때 패닉/wrap 대신 `Err(McapError::BadChunkLength / UnexpectedEoc)`를 반환하도록 방어한다(ADR-010 vendor 패치 범위 또는 `standard.rs` 선검증).
  2. `standard::export`가 "검증된 완전한 chunk 단위로만 메시지를 내보낸다"는 계약을 지키도록, `crc == 0`인 chunk도 전체 길이 로드 및 내부 레코드 파싱/디코딩 검증을 통과한 뒤에만 출력 `mcap::Writer`에 기록하거나(chunk 단위 스테이징), chunk 내부 레코드 손상을 `InvalidRecord`와 `truncated` 중 어디로 다룰지 계약과 구현을 일치시킨다.
  3. CLI `export_standard`는 파일 생성 직후 Drop 가드(또는 완료 시 disarm되는 정리 가드)를 두어 어떤 실패/취소/언와인드에서도 불완전한 출력 파일이 남지 않게 한다.

#### [처리담당]

- **판정:** 수용.
- **처리:** `LinearReader`에 짧은 chunk header·무압축 크기 불일치·내부 record 경계 검사를 추가했다. `prevalidate_chunk_crcs`는 CRC 0도 전체 chunk를 한도 안에서 로드하고 내부 레코드를 파싱한 뒤 반출한다. 비영 CRC는 기존대로 검사한다. CLI `ExportCleanup`은 모든 오류와 unwind에서 새 출력만 정리하고 결과 반출까지 성공해야 해제된다.
- **검증:** `tc_014_standard` f13~f15의 수정 전 Red(패닉·부분 반출·상한 우회) → Green, f18 압축/CRC 0 호환·크기 상한, 실제 CLI 손상 chunk/summary 통합, 정리 가드 오류·unwind·commit 시험 및 생략 변이 Red. vendor 전체 패키지 lib 131 Pass. [EV-082](../verification_evidence.md#ev-082), 소스 `3e6dcb5`.

---

### RV-137 — `Logger::mark_event`·`ask`·`drain_failed`의 다중 출력 롤백 누락, 오류 우선순위 역전 및 IO 패닉 시 대기 지연 (P2)

#### [리뷰어]

- **위치:**
  - [mod.rs:1106-1171, 1224-1235](../../../crates/dot-logger/src/logger/mod.rs#L1106-L1171) (`Logger::mark_event`, `Logger::ask`, `worse`)
  - [host.rs:399-407, 682-693, 994-1021](../../../crates/dot-logger/src/logger/host.rs#L682-L693) (`io_main`, `drain_failed`)
  - 계약: API §22.3·§22.9 (`fa72a1a`).
- **원인 및 영향:**
  1. **다중 출력 롤백 조기 중단:** `mark_event()`는 `self.ask(Mark)` 실패 시 롤백으로 `self.ask(Release)`를 재사용한다. 그런데 `ask()`는 ① 진입 시 `if self.worker_failed() { return Err(EventError::Closing); }`로 즉시 반환하고, ② 전송 루프 `for output in &self.events`에서 첫 `output.send(...)` 실패 시 `?`로 즉시 반환한다. 따라서 이벤트 출력이 2개 이상(`io_threads > 1` 등)일 때 한 IO 스레드가 패닉(`worker_failed = true`)하거나 앞선 출력의 수신단이 닫히면, 이미 보호에 성공한 다른 살아 있는 출력으로 `Release`가 전송되지 않는다. 또한 `self.ask(Mark)` 전송 루프 도중 `events[1].send`가 실패하면 `events[0]`에 이미 보낸 `Mark` 요청의 처리를 기다리지 않고 반환한다.
  2. **오류 우선순위(`worse`) 역전:** API §22.9와 `worse()`는 여러 출력의 응답이 다를 때 `Failed(3) > Closing(2) > Quota(1)` 순으로 반환하도록 정의한다. 그러나 `ask()`의 수신 루프(`mod.rs:1164-1167`)는 `Ok(Err(error)) => result = worse(result, error)`로 누적해 놓고도, 다음 출력에서 `Err(RecvTimeoutError::Disconnected)`가 나오면 `result`를 무시하고 곧바로 `return Err(EventError::Closing)`을 실행한다(타임아웃은 `Failed`라 최고 순위지만 `Disconnected`는 `Closing`이라 이미 받은 `Failed`를 덮어쓴다).
  3. **IO 패닉(`drain_failed`) 중 `target.marks` 미정리로 인한 `overdue` 대기:** `host.rs:405`의 `target.marks = None`은 `io_main`이 반환한 뒤에 실행된다. 그런데 IO 스레드가 패닉하면 `io_main`은 `shutdown`이 호출될 때까지 `drain_failed` 루프(`host.rs:691, 994-1021`) 안에 머문다. `drain_failed`는 `target.marks`를 `None`으로 비우지 않으므로, 패닉 직전이나 동시에 전송된 `EventWork`가 `marks.work` 큐에 `reply: Sender`를 쥔 채 남는다. 그 결과 호출자는 즉시 `EventError::Closing`을 받지 못하고 `4 × sync_every + 1 s`(`self.overdue`) 전체를 블로킹 대기한 끝에 `EventError::Failed`(timeout)를 받게 된다.
- **처리 제안:**
  1. `drain_failed` 진입 즉시 `for target in targets.iter_mut() { target.marks = None; }`를 수행해 큐에 남은 요청들의 `reply`를 즉시 드롭시킨다.
  2. `ask()`의 `RecvTimeoutError::Disconnected` 분기에서 `return worse(result, EventError::Closing)`을 반환하도록 수정한다.
  3. `mark_event()`의 롤백 경로는 `ask()`와 분리하여 `worker_failed()` 여부나 앞선 출력의 `send` 실패와 무관하게 `self.events`의 모든 출력에 `EventWork::Release`를 전송하고, 전송에 성공한 출력들의 응답을 기한까지 수거한다.

#### [처리담당]

- **판정:** 수용.
- **처리:** `ask_all`은 전송 실패에서 중단하지 않고 성공한 전송 수만큼 응답을 수거한다. mark 실패 뒤 rollback은 worker 실패 flag와 무관하게 `ask_all(Release)`를 호출한다. timeout/disconnect도 `worse`로 누적해 `Failed > Closing > Quota`를 유지한다. `drain_failed` 진입 시 모든 `target.marks`를 버려 대기 reply를 즉시 끊는다.
- **검증:** 실제 Logger+모의 출력의 끊긴 첫 출력/worker 실패 rollback·Failed 뒤 Disconnected 시험 3건은 수정 전 모두 Red → Green. 실제 IO target+라우팅 guard의 queued reply 해제 시험은 수정 생략 변이로 Red 확인. [EV-082](../verification_evidence.md#ev-082), 소스 `3e6dcb5`.

---

### RV-138 — 이벤트 만료(`expire`) 실행 시점 순서로 인한 기한 경과 후 `mark_event` 거짓 `Quota` 및 `write_batch` 선행 실패 가능성 (P2)

#### [리뷰어]

- **위치:**
  - [event.rs:135-159](../../../crates/dot-logger/src/logger/event.rs#L135-L159) (`Marks::mark`)
  - [host.rs:715-740, 777-784](../../../crates/dot-logger/src/logger/host.rs#L715-L784) (`io_loop`)
  - [retention.rs:678-682](../../../crates/dot-logger/src/storage/retention.rs#L678-L682) (`SegmentLog::expire_events`)
- **원인:**
  1. Core `SegmentLog::protect_event`는 기존 이벤트를 암묵적으로 만료시키지 않는다(`retention.rs:678`: "Explicit expiry sweep. Other operations never silently expire protection"). 그런데 facade `Marks::mark` 역시 `log.protect_event(at, expires, now)`를 부르기 전에 `self.expire(log, now)`를 호출하지 않는다.
  2. 따라서 이전 이벤트들의 보호 기한(`at + post + hold`)이 이미 지났더라도(`now >= expires`), `io_loop`의 `expired.elapsed() >= tuning.sync_every` 주기가 아직 돌지 않았다면(예: `post + hold`가 `sync_every`보다 짧거나, 만료 직후 새 `mark_event()`를 호출한 경우) `Marks::mark`는 이미 만료된 창이 점유 중인 `max_events` 또는 `protected_bytes` 때문에 `EventError::Quota`로 거절한다(`EventError::Quota` 문서의 "Release a mark or wait for expiry" 기대와 어긋남).
  3. 또한 `io_loop`에서 `write_batch`(`host.rs:737`)가 주기적 만료 검사(`host.rs:778-784`)보다 **앞에** 배치되어 있다. 유휴 대기 후 깨어난 턴에 `sync_every`가 이미 경과해 만료 대상 이벤트가 있더라도, 같은 턴에서 `write_batch`가 먼저 실행되면서 아직 해제되지 않은 보호 파일 때문에 세그먼트 회전/삭제(`room()`)나 `protected_bytes` 한도에서 레코드 쓰기가 실패할 수 있다.
- **처리 제안:**
  1. `Marks::mark` 시작부에서 `self.expire(log, now)`를 먼저 호출해 `now` 시점에 이미 만료된 창을 정리한 뒤 `log.protect_event(at, expires, now)`를 수행한다.
  2. `io_loop`에서 `expired.elapsed() >= tuning.sync_every` 만료 정리를 배치 쓰기(`write_batch`) 전에 턴 시작 시각 `ns(origin, now)`로 먼저 실행한다(`SegmentLog::clock`은 `now == self.now`를 허용하므로 이후 같은 `ns(origin, now)`로 실행되는 `write_batch`와 시계 역전이 발생하지 않는다).

#### [처리담당]

- **판정:** 수용.
- **처리:** `Marks::mark` 시작에 만료 정리를 넣었다. `io_loop`는 주기 만료를 배치 쓰기 앞으로 옮기고 해당 turn 시작 시각을 공유해 시계 역전을 피한다. 요청 처리의 fresh clock은 유지한다.
- **검증:** `tc_013_facade::f37`은 주기 sync(60초) 전에 만료된 1초 hold의 quota 재사용을 확인한다. host 시험은 1-record segment·2-file 한도·실제 라우팅 guard에서 만료된 보호가 회전을 막지 않는지 확인한다. 각 수정 생략 변이는 Quota/실패 record 1건으로 Red, 원복 후 Green. [EV-082](../verification_evidence.md#ev-082), 소스 `3e6dcb5`.

---

### RV-139 — `standard::export`의 채널 메타데이터 검증 공백 (`payload_schema_file_id` 자기참조 및 재등장 채널 불일치) (P2)

#### [리뷰어]

- **위치:** [standard.rs:226-263](https://github.com/lim8603/dot-logger/blob/3cbd1b2/crates/dot-logger/src/storage/standard.rs#L226-L263) (`3cbd1b2`).
- **원인:**
  1. `standard::export`는 입력 MCAP의 `dot_logger.Envelope` 스키마와 사용자 페이로드 descriptor 스키마를 동일한 `schemas: HashMap<u16, (String, String, Vec<u8>)>`에 저장한다. `Record::Channel(c)`에서 `payload_schema_file_id`를 조회할 때 `id == c.schema_id`이거나 조회된 스키마 이름이 `dot_logger.Envelope`(`ENVELOPE_SCHEMA`)인 경우를 걸러내지 않으며, `payload_schema_name`이 빈 문자열(`""`)이거나 `payload_schema_file_id` 없이 단독으로 존재하는 경우도 검사하지 않는다(`recovery.rs:545-558`은 이를 엄격히 거절함).
  2. 동일 `c.id`의 `Record::Channel`이 Summary 섹션 등에서 다시 등장할 때 `known.topic != c.topic`만 검사하고 `c.schema_id`, `c.message_encoding`, `c.metadata`가 첫 선언과 같은지 확인하지 않는다(`Record::Schema`는 재등장 시 `(name, encoding, data)` 전체 일치를 검사하는 것과 비대칭).
- **재현 (`/tmp/dot-logger-rev055`):**
  1. `env_schema.mcap` (`schema 1 = dot_logger.Envelope`만 있는 파일에서 채널 1의 메타데이터에 `payload_schema_file_id = "1"` 부여):
     - 실행 결과: exit 0, `{"channels":[{"message_encoding":"protobuf","messages":0,"schema_name":"dot_logger.Envelope","topic":"robot/a"}]}` — 사용자 채널의 스키마로 `dot_logger.Envelope`가 그대로 내보내져 API §21.1 및 `tc_014_standard` `f1`의 불변식("User channels: the application's schema and encoding, never the envelope")이 깨진다.
  2. `dup_chan.mcap` (채널 1을 `schema_id=1, message_encoding="protobuf", payload_encoding="json"`으로 선언한 뒤, 이어서 같은 채널 1을 `schema_id=0, message_encoding="json", payload_encoding="cbor"`로 재정의):
     - 실행 결과: exit 0으로 오류 없이 통과함.
- **처리 제안:**
  1. `payload_schema_file_id`가 `c.schema_id`와 같거나 해당 스키마의 이름이 `ENVELOPE_SCHEMA`인 경우, `payload_schema_name`이 빈 문자열이거나 `payload_schema_file_id` 없이 단독으로 쓰인 경우 `ExportError::InvalidRecord`로 거절한다.
  2. `Mapped`에 첫 `Record::Channel`의 `(schema_id, message_encoding, metadata)`(또는 핵심 메타데이터)를 보관하여 동일 `c.id` 재등장 시 전체 일치를 검증한다.

#### [처리담당]

- **판정:** 수용.
- **처리:** payload descriptor가 채널 Envelope 스키마 자신 또는 다른 Envelope 스키마를 가리키는 경우를 거절한다. 유효 이름도 Envelope/빈 문자열은 거절하고 descriptor id 없는 이름도 거절한다. `Mapped.input`에 첫 Channel 전체를 보관해 재등장 시 id/schema/topic/encoding/metadata 전부 일치해야 한다.
- **검증:** f16(자기참조·다른 Envelope·빈 이름·단독 이름·Envelope alias 5조건)·f17(채널 필드별 불일치 5조건)을 추가해 시험 함수 단위로 수정 전 Red → Green을 확인했다. 실제 CLI는 summary 채널 불일치를 exit 4 `invalid_data`로 보고하고 새 출력을 삭제한다. 정상 descriptor/schemaless·summary 반복은 기존 f1·f9·CLI 왕복이 검증한다. [EV-082](../verification_evidence.md#ev-082), 소스 `3e6dcb5`.

---

### RV-140 — `SegmentLog::new`의 `PayloadSchema::name` 초과 capacity 축소 누락 및 `standard::export` 실패 시 출력 상태 문서화 (P3)

#### [리뷰어]

- **위치:**
  - [retention.rs:313-321](../../../crates/dot-logger/src/storage/retention.rs#L313-L321)
  - [writer.rs:362-370](../../../crates/dot-logger/src/storage/writer.rs#L362-L370)
  - [standard.rs:106-123, 153-160](https://github.com/lim8603/dot-logger/blob/fa72a1a/crates/dot-logger/src/storage/standard.rs#L106-L160)
- **내용:**
  1. `3cbd1b2`에서 `PayloadSchema::name: Option<String>`을 추가하면서 `SegmentWriter::new`(`writer.rs:369`)에는 `s.name = s.name.take().map(|n| n.into_boxed_str().into_string());`를 넣어 `String`의 초과 capacity가 output 메모리 예산을 넘지 않도록 축소했으나, 같은 역할을 하는 `SegmentLog::new`(`retention.rs:313-321`)에는 `schema.name` 축소가 빠졌다.
  2. `standard::export`는 첫 `Record::Header`를 읽은 직후 `output: W`에 MCAP Magic·Header·`_dot_logger/meta` 채널을 기록한다. 따라서 `ExportError::Cancelled`뿐 아니라 이후 레코드에서 `NotDotLogger`·`TopicConflict`·`RecordTooLarge`·`InvalidRecord`가 반환될 때도 `output`에는 이미 바이트가 쓰여 있다. 현재는 `ExportError::Cancelled`에만 "the output is incomplete and must be discarded"라고 적혀 있으므로, `export` 함수 또는 `ExportError` 문서에 모든 `Err` 반환 시 `output`을 폐기해야 함을 명시하는 것이 명확하다(참고로 다수의 작은 `Schema`/`Channel` 레코드 반복 시 `schemas`/`channels` 맵 엔트리 수 상한 여부도 향후 필요 시 검토).

#### [처리담당]

- **판정:** 수용.
- **처리:** `SegmentLog::new`의 `schema.name`을 `into_boxed_str().into_string()`으로 정리해 기존 `SegmentWriter::new`와 일치시켰다. `standard::export` rustdoc/API §21.2에 모든 `Err`에서 출력 폐기, 개별 record/chunk 상한과 누적 schema/channel map 한계의 차이를 명시했다.
- **검증:** 두 생성자의 정규화 코드 대조, 전체 저장/보존·facade 회귀, rustdoc 검사. 누적 metadata 총량의 새 API/예산 정책은 이번 결함 조치에 추가하지 않았다. [EV-082](../verification_evidence.md#ev-082), 소스 `3e6dcb5`.

---

## 5. 검증 및 인계

### 검증 실행 결과

| 검증 | 결과 |
|---|---|
| `cargo +1.96.0 fmt --all --check` | Pass |
| `cargo +1.96.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass |
| `cargo +1.96.0 test --workspace --all-features --locked` | Pass — 625 passed, 0 failed, 5 ignored(외부 DB 시험), RT 할당 19 passed 포함 |
| `cargo +1.85.0 test --workspace --all-features --locked` | Pass — 625 passed, 0 failed, 5 ignored |
| `RUSTDOCFLAGS='-D warnings' cargo +1.96.0 doc --workspace --all-features --no-deps --locked` | Pass |
| `RUSTDOCFLAGS='-D warnings' cargo +1.96.0 doc -p dot-logger --no-default-features --no-deps --locked` | Pass |
| `cargo +1.96.0 check -p dot-logger --no-default-features --all-targets --locked` | Pass |
| `cargo +1.96.0 check -p dot-logger-client --no-default-features --all-targets --locked` | Pass |
| `python3 scripts/check-integration-surface.py` | Pass — facade 이름 있는 항목 34/60, L1~L3 레시피·실행·README 일치 |
| `/tmp/dot-logger-rev055` 재현 진단 4건 (`underflow.mcap`, `partial_chunk.mcap`, `crc0_trunc.mcap`, `env_schema.mcap`/`dup_chan.mcap`) | RV-136 패닉·파일 잔존·부분 chunk 반출 및 RV-139 검증 공백 실제 재현 |
| 제품 코드·저장소 테스트 변경 | 없음 (Strict Read-Only 준수) |

### 중점 ①~⑥ 검토 요약

1. **① `fa72a1a` 이벤트 보존:** Logger→IO thread 전용 채널과 IO loop `try_recv`, `at.min(now)` 단조 시계, `4 × sync_every + 1 s` 응답 기한(`overdue`), `held_bytes` 메모리 사이징(`f36` 1바이트 오차 검증), `protected_bytes + 2 × segment <= ring` 검증은 정확하다. 다만 다중 출력 롤백·오류 우선순위·IO 패닉 시 `target.marks` 정리(`RV-137`, P2)와 `mark`/`write_batch` 전 만료 정리 순서(`RV-138`, P2)에 보완이 필요하다.
2. **② `a543432`·`67fa480` 중요도·최소 level:** `Channel<T>`에 `importance: Option<Importance>` 추가 후 `Copy`·무할당이 유지되고 RT 할당 시험(`f2`, `Channel::important()`)이 통과한다. `ByLevel` 매핑 경계, `important_slots >= small_slots` 거절, `with_raw` 호출 순서 무관성, `self.writers`만 검사해 `tracing` 내부 writer를 제외한 `PlanWarning::NoImportantReserve`, `ChannelOptions::minimum`의 `Logged::Skipped` RT 경로 모두 결함이 없다.
3. **③ `17ebda6` ring `2 × segment` 하한:** `segment = (r.bytes / 8).clamp(floor, SEGMENT_CEILING.max(floor))` 및 `ring_minimum = segment.saturating_mul(2)`로 15 MiB 거절 / 16 MiB 통과 경계값이 정확하며, `Continuous` 모드에는 영향을 주지 않고 `preset`/`calibrate`와도 충돌하지 않는다.
4. **④ `3a554ac` client `diagnostics` 이동:** `dot_logger_client::diagnostics` 공개 모듈 제거 및 `examples/support/dot_probe/` 이동이 깔끔하고, `#![allow(dead_code, unused_imports)]`는 `dot_probe.rs` 지원 모듈 내부에만 적용된다. `--no-default-features` 빌드도 통과해 `remote` feature 경계가 온전하다.
5. **⑤ `3cbd1b2` TASK-030 S1 표준 export:** 정상 파일 및 단순 파일 끝 잘림(`f1`~`f7`) 동작은 확인되었으나, 손상된 무압축 chunk에서의 `LinearReader` 언더플로 패닉·CLI 출력 파일 미삭제·미검증 chunk 부분 메시지 반출(`RV-136`, P1), 채널 메타데이터 자기참조·재등장 검증 공백(`RV-139`, P2), `SegmentLog::new`의 `schema.name` 축소 누락(`RV-140`, P3)을 발견했다.
6. **⑥ 시험 커밋(`783f2e3`·`8703459`·`ce0efcd`·`17ebda6` 시험부):** `cli.rs`의 `ReplayAction::Speed` mock 변경(`Speed::new(3, 2)` 불일치 시 `InvalidInput` 반환)은 CLI가 실제로 인자를 전달했는지 검증을 강화한 것이며 계약을 가리지 않는다. 추가된 시험들의 결정성과 `/tmp` 임시 디렉터리 정리도 정상이다.

### 다음 개발 세션 인계

- 재현 입력과 진단 스크립트 결과는 `/tmp/dot-logger-rev055`에 생성해 확인했으며, 핵심 재현 바이트 구성을 본 문서에 모두 기록했다.
- 다음 개발 세션은 **TASK-030 S2 ADR보다 먼저** `RV-136`(P1) 및 `RV-137~139`(P2), `RV-140`(P3)에 대해 실패 재현 테스트(Red)를 저장소 시험으로 옮기고 최소 수정(Green) 및 회귀 검증을 수행한 뒤 `[처리담당]` 영역을 갱신한다.

## 6. 개발 조치 마감 — 2026-11-16

- **처리 결과:** RV-136~140 전부 수용, `Open` → `In Progress` → `Resolved`(재현·수정·회귀 완료) → `Closed`(2026-10-31 구현·검증·리뷰 마감 위임에 따른 AI 수락). 최초 리뷰의 Conditional 판정과 리뷰어 원문은 당시 기록으로 보존한다.
- **대상 소스:** `3e6dcb5`. 전체 근거 [EV-082](../verification_evidence.md#ev-082).
- **검증:** 1.96.0·MSRV 1.85.0 각각 workspace 639 Pass·0 Failed·5 ignored, 18 feature 구성·독립 consumer/RT allocator, clippy·strict rustdoc(core-only 포함)·fmt Pass. 표면 guard 자기 시험 18개와 NFR-008 guard 34/60 Pass. vendor full-package lib 131 Pass.
- **파일 변경:** vendor reader·VENDORED/Cargo 주석, `storage/{standard,retention}.rs`, `logger/{mod,event,host,tests}.rs`, CLI `commands.rs`/통합 시험, `tc_013_facade`/`tc_014_standard`, API·ADR-010 결함 조치 보완·TC/EV·Task/목록·상태 문서. 세션 로그는 기존 ignore 규칙으로 로컬 보관한다.
- **타입 변경:** 생성 `ExportCleanup`(실패 출력 Drop 정리); 수정 `Mapped`(첫 Channel 전체), `LinearReader`(경계/선검증)·`LinearReaderOptions`(기존 옵션 의미 문서), `Logger`(전체 출력 응답/rollback), `Marks`(보호 전 만료), `SegmentLog`(schema.name capacity 정리), 시험 `Temp`(host fixture 재사용 가시성). 공개 facade 항목 추가 없음.
- **자가 검토:** 새 RT 할당·대기·IO 없음, 새 unsafe/원자 순서 변경 없음. 오류를 숨기지 않고 기존 오류 우선순위/출력 폐기 계약과 일치시켰다. F-11 소비자 종단 검증은 EV-082에 연결했다.
- **후속:** TASK-034 기존 구현 결과 Human 확인 대기는 유지. 다음 개발 시작점은 TASK-030 S2 저장 ADR(native 표준 쓰기·TASK-035 손실 기록·revision 회전/다중 revision 조회·복구). native·장기·실앱 및 커버리지 재측정은 기존 소유 Task와 시점 유지, Gate4/5 승격 없음.

# TASK-033 — 로깅 사용 사례 조사와 커버리지 기반 완료 판정

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-001·002·003·004·005 |
| 요구사항 | FR 전체(Must)·NFR-004·NFR-008 |
| 설계 | [인터페이스](../../03_design_artifacts/interface_contract.md) · [시험 전략](../../05_verification/test_strategy.md) · [추적표](../../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트) |
| ADR | 없음 — 완료 기준은 [quality_gate](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-15 |
| 다음 액션 | S4 기준 1~3 충족(2026-11-15, 라인 93.56%) → TASK-034·035 뒤 재측정으로 판정(기준 4) |
| 의존 | 없음(기존 시험·EV-078 기준선 사용) |
| 예상 소요 | 3~5 인일, 리뷰 대기 제외 |

## 목표

2026-11-13 Human 결정에 따라 프로젝트 일정은 실사용처 적용이 아니라 **시험 커버리지**와 **실제 로깅 사용 사례 조사**로 판단한다. 이 Task는 그 판단 근거를 만든다.

1. 로보틱스·RTOS·임베디드 로깅의 실제 사용 사례를 조사해 사용 사례(UC) 목록을 만든다.
2. 각 UC를 기존 TC·시험 함수와 대응시키고, 대응이 없는 UC는 시험을 추가하거나 범위 밖 사유를 기록한다.
3. 라인·함수·리전 커버리지 기준선에서 파일별 공백을 분류하고, 실제 동작 공백은 시험으로 메운다.
4. 목표치(커버리지·UC 대응률)를 제안해 Human 승인을 받고, MS-007 일정 판단에 쓴다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 커버리지 측정 도구와 기준선 | `cargo llvm-cov` workspace all-features 측정·파일별 미실행 줄 목록, 저위 파일 분류 — [EV-078](../../05_verification/verification_evidence.md#ev-078) | Done |
| S2 | 로깅 사용 사례 조사 | 공식 문서·공개 저장소 근거가 붙은 UC 목록(UC-ID·출처·요구 동작·관련 FR/NFR). 조사 대상 후보: ROS 2 rosbag2/MCAP, Foxglove·Lichtblick, PX4 ULog, ArduPilot DataFlash, Zephyr·FreeRTOS 로깅, defmt, spdlog·Quill 같은 비동기 로거, Rust `tracing`/`log` 생태계, 자동차 DLT. 사실·추정 구분 | **Done** 2026-11-15 — [UC 31개](../../06_evolution/imported_context/research_2026-11-15_logging_use_cases.md)(12개 시스템, 1차 출처, 표본 2건 원본 재확인). 예비 대응: 있음 10·일부 16·없음 2(UC-07 rate 제한, UC-18 backtrace)·S3 보류 3 |
| S3 | UC↔TC 대응과 공백 시험 | UC별 대응 시험 표, 대응 없는 UC의 시험 추가(Red→Green) 또는 범위 밖 사유, 아래 커버리지 공백 중 「실제 동작」 항목 처리 | **Done** 2026-11-15 — 대응표, 시험 17개 신규·1개 확장, 결함 1건 수정, 기능 공백 결정 4건 반영(TASK-034·035·B1·bridge 이동), 재측정 라인 92.40% |
| S4 | 목표치와 완료 판정 | 커버리지·UC 대응률 목표 제안 → Human 승인 → 재측정 결과로 MS-007 일정 판단 | In Progress — 목표치 승인, 기준 1~3 충족(2026-11-15). 판정은 TASK-034·035 뒤 재측정(기준 4) |

## 커버리지 기준선과 공백 분류 — S1 (2026-11-13)

측정: [EV-078](../../05_verification/verification_evidence.md#ev-078). workspace 전체 라인 **91.15%**(18,826줄 중 1,667줄 미실행), 함수 90.92%, 리전 88.27%. doctest와 `#[ignore]` 성능 시험은 측정에 들어가지 않는다.

라인 커버리지 85% 미만 파일을 미실행 줄 단위로 직접 확인해 분류했다.

| 파일 | 라인 | 분류 | 내용 |
|---|---|---|---|
| `dot-logger/src/routing/types.rs` | 0% (6줄) | 비동작 | `Display`·`From` 구현 |
| `dot-logger/src/capture/types.rs` | 63.6% | 비동작 | `Display` 구현 |
| `dot-logger/src/completion/types.rs`·`control/types.rs` | 76~78% | 비동작 | `Display`·`Debug` 구현 |
| `dot-logger/src/logger/writer.rs` | 79.5% | 일부 **실제 동작** | `Schema::new`·`descriptor`는 const 평가라 측정에 안 잡힘(비동작). **다른 Logger의 Channel 거절은 시험이 없었음 → 2026-11-13 추가.** **설정 변경(level 필터) 뒤 facade `try_log`가 `Logged::Skipped`를 돌려주는 경로는 시험 없음 → S3** |
| `dot-logger/src/logger/sink.rs` | 81.8% | 대부분 비동작 | tracing `Subscriber`의 span 관련 빈 메서드·`Debug`. `log` crate의 `enabled`·`flush` 직접 호출은 S3에서 확인 |
| `dot-logger-cli/src/commands.rs` | 74.9% (204줄) | **실제 동작** | remote `blob`·`completion`·`replay` 관리 명령의 CLI 경로, query/replay `--include-payload`의 blob(큰 payload) 읽기, 일부 출력 한도 초과 → S3 |
| `dot-logger-cli/src/output.rs`·`model.rs` | 82~83% | 혼합 | 출력 한도·입력 검증 분기 → S3에서 분류 |
| `dot-logger-client/src/diagnostics/bridge.rs` (2026-11-15 `examples/support/dot_probe/`로 이동) | 74.4% | **실제 동작** + 범위 의문 → 이동 완료 | 스냅샷 파일 검증 실패·단계 사이 취소/기한 만료 분기. 이 모듈은 dot-probe export 모양을 전제한 **사용처 전용 코드가 라이브러리 `src`에 있는 경우**다. 범용 라이브러리 기준에서 예제로 옮길지 S3에서 판단(코드 이동은 Human 확인) |
| `dot-logger-client/src/diagnostics/mod.rs` | 73.0% | 혼합 | 위 bridge의 입력 검증 → S3 |
| `dot-logger/src/storage/standard.rs` | 83.8% | 실제 동작 | TASK-030 진행 중 범위(손상 입력 분기) — TASK-030 S1 리뷰에서 처리 |
| `dot-logger/src/completion/query.rs`·`logger/health.rs`·`logger/mod.rs`·`telemetry/capacity.rs`·`adapters/remote/wire.rs`·`client/codec.rs` | 82~86% | 미분류 | S3에서 분류 |

## UC↔TC 대응 — S3 (2026-11-15)

UC 정의는 [S2 조사](../../06_evolution/imported_context/research_2026-11-15_logging_use_cases.md)에 있다. 판정은 읽기 전용 조사(시험 본문과 제품 코드 대조)로 했고, 핵심 주장 3건은 코드에서 직접 다시 확인했다. 확인한 3건은 다음과 같다.
- 손실 기록을 파일 안에 쓰지 않는다: [writer.rs](../../../crates/dot-logger/src/storage/writer.rs)에 Metadata 쓰기가 없다.
- facade가 이벤트 보존을 끈다: [budget.rs](../../../crates/dot-logger/src/logger/budget.rs)에서 `max_events: 0`·`pre_ns`·`post_ns` 0이다.
- 압축이 꺼져 있다: writer가 `.compression(None)`으로 고정한다.

판정 기준:
- **대응:** 기능이 있고 시험이 그 동작을 단언한다.
- **일부:** 하위 동작 중 일부만 있거나 일부만 시험된다.
- 공백은 두 종류로 나눈다.
  - **시험 공백:** 구현은 있으나 시험이 없다. AI가 이번에 시험을 추가한다.
  - **기능 공백:** 구현이 없다. 새 범위이므로 Human이 결정한다.
- 「core 전용」은 고급 경로(`CaptureOwner`·`SegmentLog`)에만 있고 `Logger` facade에는 노출되지 않은 기능이다.

| UC | 판정 | 대표 시험 | 시험 공백(S3 추가) | 기능 공백(Human 결정) |
|---|---|---|---|---|
| UC-01 RT 무대기 호출 | 대응 | `tc_002_capture::copies_input_preserves_metadata…`, `tc_013_facade::f1…`, `tc_012_performance::call_cost_separates…` | 없음 — facade `try_log`·`producer()` 무할당은 `tests/rt-allocation::f2_try_log_rejections_and_producer_view_never_allocate_or_free`(대응 조사에서 이 crate를 놓쳐 공백으로 잘못 분류, 2026-11-15 정정) | — |
| UC-02 버퍼 포화 정책 선택 | 일부 | `tc_002_capture::normal_saturation_preserves_important_reservation…`, `::general_producer_observes_deadline…` | 없음 — `writer.producer()`는 core와 같은 `Producer` 타입이고 `into_general().record_until`은 `tc_002_capture`가 시험한다(정정) | 오래된 기록 덮어쓰기 없음. 정책은 고정(새 기록 거절) |
| UC-03 중요 기록 우선 | 대응(core) | `tc_002_capture::normal_saturation…`, `tc_002_pool::normal_loans_cannot_borrow_important_bytes…`, `tc_009_config_update::sampling_is_explicit_and_important_bypasses_it…` | 표준 export 메타 `importance`·`source` → **F7 추가** | facade 단순 경로(`try_log`)에서 중요도를 지정할 수 없다(고급 `producer()`로만 가능) |
| UC-04 손실 계수·파일 내 표시 | 일부 | `tc_009_telemetry::capture_small_raw_skip_and_reject_totals…`, `tc_013_facade::f10…`, `tc_013_tracing::f12_oversized_and_recursive…` | — | 채널별 손실 수 없음(producer 단위). **파일 안 손실 기록 없음** — 수락된 기록만 sequence가 올라가 표준 도구가 구멍을 볼 수 없다 |
| UC-05 버퍼 건강 지표 | 일부 | `tc_002_pool::rejected_loan_is_the_same_loan…`, `tc_013_facade::f9_health_is_running…`, `tc_009_report::full_resource_or_disk_reserve…` | — | 소형 ingress 점유·최대 사용량(high-water) 지표 없음. `BufferTrend`는 host가 채워야 한다 |
| UC-06 다중 생산자 순서 | 일부 | `tc_002_capture::two_concurrent_producers_keep_independent_fifo…`, `tc_003_routing::deficit_round_robin…` | 같은 시각 다중 producer 병합 순서 → `tc_006_query::equal_timestamps_from_two_producers_tie_break…` | — (계약은 producer별 FIFO, 전역 순서 없음 — ADR-008) |
| UC-07 채널 rate 제한 | 일부(core 전용) | `tc_009_config_update::sampling_is_explicit…` | — | N개 중 1개 샘플링만 core에 있다. 시간 기준 rate·출력·상태별 rate 없음. facade 미노출 |
| UC-08 실행 중 level 변경 | 일부(core 전용) | `tc_009_config_update::producer_boundaries_apply_partially…`, `tc_013_tracing::f12…` | — | facade는 config update를 켜지 않고 원격 ApplyConfig는 Unsupported → `Logged::Skipped` 도달 불가. 환경 변수 초기값·컴파일 상한·출력별 level 없음 |
| UC-09 다중 출력·출력별 필터 | 대응 | `tc_013_routing::f16_channels_reach_only_their_selected_outputs`, `tc_003_routing::stopped_required_a_does_not_stop_b…` | 원격 hub 채널·최소 level 제외 → `remote::tests::subscription_channel_and_minimum_level_exclude…` | 출력 켜기·끄기는 core route patch로만 가능 |
| UC-10 채널 선택 | 일부 | `tc_013_routing::f16…`, `tc_006_query::pages_sort_out_of_order…` | — | 정규식·타입·제외·프로필 선택 없음(명시 등록만) |
| UC-11 시각 2종·시계 주입 | 일부 | `tc_002_capture::copies_input_preserves_metadata…`, `tc_004_storage::file_writer_preserves_large_records_wall_reversal…` | `Writer::try_log_at` 호출자 시각 → `tc_013_facade::f22…`, 메타 `source` → `tc_014_standard::f7` | 시계 객체 주입 없음. wall 시각이 epoch 이전이면 표시 없이 `real_ns=0`. facade 단순 경로에서 `source_time` 지정 불가 |
| UC-12 스스로 설명하는 파일 | 일부 | `tc_014_standard::f1…`, `tc_004_storage::schema_versions_and_wide_channel_ids…` | — | 사용자 파일 metadata·첨부·단위·파라미터 없음. library 문자열 고정 |
| UC-13 binary 인코딩 | 대응 | `tc_013_facade::f1…`, `tc_014_standard::f1…` | — | 포맷 사전은 해당 없음 |
| UC-14 구조화 필드·facade 연결 | 일부 | `tc_013_tracing::f12_tracing_and_log_events…`, `::f12_oversized_and_recursive_events…`(재귀 방지), `tc_007_tracing::tracing_preserves_target_fields…` | — | `log` kv·tracing span 미지원 |
| UC-15 원본 payload 채널 | 대응 | `tc_002_pool::accepted_payload_is_not_copied…`, `tc_013_facade::f19_raw_loans…` | — | — |
| UC-16 분할·보존 상한 | 일부 | `tc_005_retention::bytes_time_and_index_limits_rotate…`, `::continuous_stops_without_deleting…`, `::disk_byte_quota_stops_continuous…` | facade ring 순환 삭제 → `tc_013_facade::f20`, `reserve` → `f21`(실행 중 여유 공간 감소는 probe 주입 불가로 미시험). **결함 발견·수정 → `f24`** | 요청 시·상태 변화 시 분할은 core `rotate()`만 있고 facade·원격에는 없다 |
| UC-17 snapshot·이벤트 전후 보존 | 일부(core 전용) | `tc_005_retention::overlapping_events_charge_unique_files…`, `::event_quota_rejects_history…` | — | **facade·원격에서 이벤트 trigger 불가**(facade가 `max_events: 0`). one-shot·latched 재기록 없음 |
| UC-18 backtrace | 기능 없음 | — | — | 미구현 |
| UC-19 동기화 정책 | 일부 | `tc_013_facade::f9_health…periodic_sync`, `::f17_continuous_load_keeps_periodic_syncs…`, `tc_003_control::polling_never_syncs…` | — | facade는 `sync_every`뿐. 바이트·심각도 기준 sync와 요청 시 sync 없음(core `request_sync`만) |
| UC-20 크래시·전원 차단 복구 | 대응 | `fault_storage` 복구 시험들, `tc_005_retention` | — | — |
| UC-21 치명 오류 flush | 일부 | `tc_013_facade::f7_drop_without_shutdown…`, `::f4_expired_deadline…`, `logger::tests::io_worker_panic…`, `tc_011_emergency` | — | 앱 panic hook·signal 처리 없음(CLI만 Ctrl-C) |
| UC-22 디스크 가득 참·쓰기 오류 | 일부 | `fault_storage::disk_full_on_one_output_preserves_the_other…`, `tc_013_facade::f5…`, `::f9_health_reports_a_poisoned_output…` | — | 쓰기 오류 뒤 재시도·재열기 없음(출력 영구 실패, 나머지는 계속) |
| UC-23 압축 선택 | 기능 없음 | `fault_storage::missing_crc_compression…`(복구가 압축을 거절) | — | writer가 `.compression(None)`으로 고정 |
| UC-24 재생 제어 | 일부 | `tc_006_replay::mono_order_speed_pause…`, `::seek_discards_prefetch…`, `::finite_rate_changes…` | `to`에서 끝남·채널 일치 → `tc_006_replay::replay_with_upper_bound…`·`::replay_with_positive_channel_filter…`, CLI `replay --action speed` → `cli::actual_cli_remote_query_blob_and_replay_control…` 확장 | 반복·step·remap·real 기준 순서·clock publish 없음. facade 원격 query·replay는 Unsupported |
| UC-25 변환·병합·필터 | 일부 | `tc_014_standard::f1…`, `cli::export_standard_writes_a_new_file…` | — | 병합·분할·시간·topic 필터 export 없음 |
| UC-26 기록 중 원격 전송 | 대응 | TC-007 `tail` overflow Gap·disconnect, `tail_screen` | — | — |
| UC-27 표준 도구 판독 | 대응(파일) | `tc_014_standard`, EV-077 Foxglove 판독 | — | live·Log panel 형식은 TASK-031 |
| UC-28 원격 실행 중 제어 | 일부 | `tc_009_operations::apply_maps_conflict_busy_restart…`, `operations_host::cli_status_apply_and_compare…`, `cli::actual_cli_remote_query_blob_and_replay_control…` | CLI `blob`(같은 시험 확장)·`completion` → `cli::actual_cli_remote_completion_reports…`·`cli::invalid_blob_and_completion_arguments…`, completion 결과 14종 wire → `remote::tests::completion_reply_pins_every_outcome…`·`codec::tests::completion_reply_decodes_every_outcome…`·`transport::every_completion_outcome_and_reason_survives…` | 기록 일시정지·재개·분할·snapshot 명령 없음. facade 원격 ApplyConfig Unsupported |
| UC-29 상태 기반 기록 시작·중지 | 기능 없음 | (간접) `tc_009_config_update::configuration_state_and_sampling_survive…` 빈 route | — | 미구현 |
| UC-30 백그라운드 affinity | 대응 | `tc_013_affinity`, `tc_013_facade::f11_unknown_worker_cpu…` | — | — |
| UC-31 조회 필터 | 일부 | `tc_006_query::interleaved_queries_and_reused_cursors…`, `cli::real_offline_query_replay…` | 채널 일치 조회 → `tc_006_query::positive_channel_filter_returns_only_the_selected_channel…`. CLI `--channel/--level/--basis` → `cli::offline_query_channel_level_and_basis_flags_reach_the_query`(`4559fc7`) | 텍스트 검색 없음 |

**요약:**
- 31개 판정: 대응 9, 일부 19, 기능 없음 3(UC-18·23·29).
- 「일부」 중 5개(UC-03 단순 경로, UC-07, UC-08, UC-16 요청 분할, UC-17)는 **core에는 있지만 facade에 노출되지 않은** 경우다.
- 기능 공백 결정 요청은 아래 「Human 결정 요청」을 따른다.

**S1 분류 정정:**
- 「facade `Logged::Skipped` 경로 시험 없음」은 틀린 분류였다. facade는 config update를 켜지 않아 이 경로에 **도달할 수 없다**(UC-08 기능 공백).
- CLI 원격 `replay`는 커버리지에서 실행된 것으로 나온다. 실제로 실행되지 않은 것은 `blob`·`completion`이다.

### S3 공백 시험 추가 — 2026-11-15

시험 16개를 새로 추가했고 1개를 확장했다. 모두 구현은 이미 있었고 시험만 없었다. 그래서 Red는 시험마다 해당 동작만 깨는 제품 코드 변이로 실패를 확인했고, 확인 뒤 변이는 원복했다. 시험 이름은 위 표에 있다. 제품 코드 변경은 아래 결함 수정 1건뿐이다.

| 묶음 | 시험 | 변이 예와 실패 |
|---|---|---|
| 표준 export 메타 | `tc_014_standard::f7_important_records_and_source_time_survive_in_meta` | Important→0: `left: 0 right: 1`. source 버림: 실패 |
| facade | `tc_013_facade::f20`~`f24` | f20 ring: 삭제 생략 → 「the oldest segment file must be deleted」. f21: reserve 0 → 시작됨. f22: real을 `now()`로 → 값 불일치. f23: 디렉터리 검사 제거 → `NotADirectory`/`NotFound`. f24: 아래 결함 |
| 조회·재생 | `tc_006_query` 2개, `tc_006_replay` 2개 | 채널 필터 무시 → 6건 모두 반환. 병합 키 producer↔seq 교환 → 순서 불일치. `to` 무시 → 4건 전달 |
| 원격·CLI | `remote::tests` 2개, `codec::tests` 1개, `transport` 1개, `cli` 2개 신규·1개 확장 | `Busy => 6` → `(NotRouted)`. blob offset 0 → `"0102"`. synced→Written → exit 2. hub level `<`→`<=` → 「seq 3 must reach 1」 |

남은 시험 공백 처리(2026-11-15):
- UC-01 facade 무할당: 이미 `tests/rt-allocation`에 있었다(정정).
- UC-02 facade 제한 시간 대기: core와 같은 타입 경로라 이미 시험된다(정정).
- UC-31 CLI 플래그: 시험을 추가했다. 채널 무시 변이와 real→mono 변이로 Red를 확인했다.
- client bridge: 결정 4로 예제 support로 옮겼다.

### 결함 수정 — 작은 ring이 시작 뒤 기록 대부분을 잃음 (2026-11-15)

- **발견:** f20을 작성하다 찾았다.
- **증상:** `Retention::ring(bytes)`가 8 MiB 이상 16 MiB 미만이면 시작은 성공한다. 그러나 첫 segment가 차면 회전할 수 없어 이후 기록이 파일 단계에서 실패한다. 재현(Pose 30,000건):
  - 9·15 MiB: `failed=26743`, `success=false`, `0.mcap`만 남는다.
  - 16·17·24 MiB: `failed=0`.
- **원인:** 64 MiB 미만 ring의 segment는 하한 8 MiB로 고정된다. 회전하려면 닫히는 segment와 다음 segment가 둘 다 들어가야 한다(2 × segment). 시작 검증은 segment 하한만 봤다.
- **수정:** ring이 2 × segment 미만이면 시작 전에 `StartError::Retention{required: 2 × segment, available}`로 거절한다. continuous는 그대로 둔다(가득 차면 멈추는 것이 계약). 위치: [budget.rs](../../../crates/dot-logger/src/logger/budget.rs), API 「파일 규칙」에 한 줄 추가, `Retention::ring` rustdoc.
- **시험:** `tc_013_facade::f24_a_ring_below_two_segments_is_rejected_before_start`. 수정 전에는 15 MiB ring이 시작돼 Red였고, 수정 후 Green이다.
- **영향:** 저장소 안의 ring은 모두 64 MiB 이상이라 바뀌는 시험·예제가 없다.

### Human 결정 요청 — S3 기능 공백 (2026-11-15)

**결정(2026-11-15 Human 「모두 권고대로 진행하자」):**
- 1번 → [TASK-034](TASK-034.md)(①~③, MS-008 재개). ④는 B1.
- 2번 → [TASK-035](TASK-035.md)(MS-009, TASK-030 S2 ADR과 함께).
- 3번 → 범위 밖·백로그 B1, 일정 판단 제외.
- 4번 → client `diagnostics`(`SnapshotCheck` + bridge)를 `examples/support/dot_probe`로 이동(`4bd6d04`). bridge만 옮기면 라이브러리의 `SnapshotCheck`가 예제 코드에 의존하게 되므로 모듈 전체를 옮겼다.

원래 요청:

1. **core에는 있지만 facade에 없는 기능을 facade에 노출할지**(새 범위). 후보:
   - ① 실행 중 level·샘플링 변경: FR-017, UC-07·08. 지금은 facade 원격 ApplyConfig가 Unsupported다.
   - ② 이벤트 전후 보존 trigger: **FR-011 Must**, UC-17.
   - ③ 단순 경로(`try_log`)의 중요도 지정: **FR-009 Must**, UC-03.
   - ④ 요청 시 분할·sync: UC-16·19.

   **권고:** ①~③을 새 Task(MS-008 후속)로 등록하고 ④는 백로그로 둔다.
   - Must 동작이 지금은 facade 사용자에게 고급 경로로만 열려 있다.
   - core 구현과 시험이 이미 있어 노출 비용이 작다.
   - NFR-008의 공개 항목 수(현재 30/60)와 L1 레시피 계수가 늘어나는지 확인이 필요하다.
2. **파일 안 손실 기록(UC-04)을 추가할지.** 손실 개수·구간을 MCAP 안에 기록하고, 채널별로 센다.
   - **권고:** 추가한다(1번 Task에 포함하거나 별도 Task).
   - 근거: PX4·ArduPilot·Zephyr 모두 손실을 기록 안에 남긴다. dot-logger는 수락된 기록만 sequence가 올라가서, 표준 도구에서 구멍이 보이지 않는다.
3. **나머지 기능 공백을 어떻게 처리할지.** 대상: UC-02 덮어쓰기, 05 high-water, 07 시간 rate, 10, 11 시계 객체, 12, 14, 18, 21, 22, 23, 24 반복 등, 25, 28, 29, 31 텍스트.
   - **권고:** 범위 밖으로 기록하고 백로그 B1에 일괄 등록한다. 일정 판단에서는 제외한다.
4. **client `diagnostics/bridge.rs`를 어떻게 할지.**
   - **권고:** `examples/support`로 옮기고 시험은 유지한다.
   - 근거: dot-probe export 모양을 전제한 사용처 전용 코드다. 아직 1.0 전이고 외부 사용처가 없어서 공개 항목을 제거해도 영향이 없다.

### 커버리지 재측정 — S3 종료 (2026-11-15)

`cargo llvm-cov --workspace --all-features`(시험 565 passed·0 failed, doctest·`#[ignore]` 제외):

| 지표 | S1 기준선(EV-078) | 같은 범위로 비교(옮긴 client `diagnostics` 597줄 제외) | S3 종료 |
|---|---|---|---|
| 라인 | 91.15% | 91.73% | **92.40%**(18,284줄 중 1,389 미실행) |
| 함수 | 90.92% | — | 92.08% |
| 리전 | 88.27% | — | 89.50% |

라인 85% 미만 파일의 남은 미실행 줄 분류:
- **비동작:** `Display`·`Debug`·`From`, const fn(`Schema::new`·`descriptor`), tracing span 빈 메서드. 해당 파일은 `*/types.rs`, `logger/writer.rs`, `logger/sink.rs`, `storage/standard.rs` 일부다.
- **도달 불가:** `writer.rs` facade `Logged::Skipped` 분기 → [TASK-034](TASK-034.md)에서 도달 가능해진다.
- **경미한 실제 동작**(오류 코드 매핑·입력 검증 분기):
  - CLI: `output.rs`의 `QueryError` → exit code 매핑, `model.rs`의 hex·manifest 입력 오류
  - `completion/query.rs`: 로컬 completion 조회의 BufferTooSmall·취소 중 page, 비완료 결과 집계
  - `standard.rs`: 손상 입력(schema 재정의·channel topic 충돌·Envelope 아닌 채널)
  - `sink.rs`: `log` level 일부 매핑
  - `health.rs`: 원격 affinity 결과
  - `telemetry/capacity.rs`: forecast 결합 분기

  이 항목들은 S4 규칙 승인 뒤 처리한다.

### S4 목표치 — 2026-11-15 Human 「승인」

| # | 기준 | 제안값 | 현재 |
|---|---|---|---|
| 1 | workspace 커버리지 하한(회귀 방지) | 라인 ≥ 92%, 함수 ≥ 92%, 리전 ≥ 89%. 측정은 `cargo llvm-cov --workspace --all-features`(doctest·`#[ignore]` 제외) | 92.40 / 92.08 / 89.50 |
| 2 | 파일별 공백 규칙 | 라인 85% 미만 파일은 미실행 줄을 분류한다. 「실제 동작」 공백은 시험으로 메우거나 사유를 기록해 미처리 0으로 만든다 | 경미 분기 6묶음 남음(위) |
| 3 | UC 대응률 | 범위 안 UC에서 구현된 동작의 시험 대응 100%. 기능 공백은 B1(범위 밖) 또는 TASK-034·035가 소유한다 | 시험 공백 0(S3) |
| 4 | 일정 판단 시점 | 1~3을 충족하고, **TASK-034·035 완료 뒤 재측정 Pass**로 MS-007 일정 판단을 마무리한다 | — |

- 권고 근거(퍼센트 하한): 무작정 높이면 `Display` 같은 비동작 줄을 채우는 시험이 늘어난다. 그래서 퍼센트는 회귀 방지 하한으로만 두고, 판단은 2·3의 분류 규칙으로 한다.
- 권고 근거(판정 시점): TASK-034·035는 Must 요구(FR-009·011)와 관련된 공백이다. 이것들이 끝난 뒤 판정해야 일정 판단이 실제 사용 사례를 반영한다.

### S4 경미 분기 처리 — 2026-11-15

시험 22개를 새로 추가했고 3개를 확장했다(core 8개, CLI 14개 신규·3개 확장). 모든 신규·확장 시험은 해당 동작만 깨는 제품 변이로 Red를 확인했고 원복했다. 제품 코드 변경은 없다. `src/completion/tests.rs`·`src/logger/tests.rs`는 crate 내부 상태가 필요해 그 안의 시험 모듈에 추가했다.

| 묶음 | 시험 | 남긴 줄과 사유 |
|---|---|---|
| `storage/standard.rs` 손상 입력 | `tc_014_standard::f8`~`f12`: schema 재정의 → `InvalidRecord`, channel topic 재정의 → `InvalidRecord`, Envelope 아닌 채널 4종 → `NotDotLogger`, 중간 파싱 오류 → truncated, Metadata·Attachment → `skipped_records` | L189: sans-io `LinearReader`가 `McapError::Io`를 만들지 않는다 |
| `completion/query.rs` | `tc_003_completion::query_summary_counts_each_outcome_and_wait_ends…`(결과별 집계·`BufferTooSmall`·`InvalidScope`·wait의 TimedOut/Settled/Complete), `completion::tests::a_query_page_reports_a_row_under_publication_as_busy` | L118: 2^64개 넘는 기록이 있어야 닿는 overflow 보호 |
| `logger/sink.rs` | `tc_013_tracing::f12_log_levels_are_filtered_against_the_declared_minimum`(`log` 5개 level) | span 콜백: `enabled`가 span을 거절해 tracing이 호출하지 않는다 |
| `logger/health.rs` | `logger::tests::health_reports_overdue_sync_telemetry_age_and_remote_placement` | — |
| `telemetry/capacity.rs` | `tc_009_report::the_buffer_limit_takes_the_finite_horizon…` | L158·L181: 호출 측(`report.rs`)이 해당 입력을 넘기지 않는다 |
| CLI `commands.rs`·`output.rs`·`model.rs` | `cli.rs` 14개 신규·3개 확장: manifest 검증, offline 원본 오류의 exit code, payload·`--max-output-bytes`, paging·정확한 limit, 잘못된 모드의 명령, 연결 전 입력 검증, config diff, text 출력, recover, calibrate, export, 원격 응답 오류의 exit code, tail overflow, completion 경계 5종, summary 상한, 연결 불가 | ACK 사이 연결 끊김·명령 도중 Ctrl-C·파일 교체 같은 타이밍·결함 주입 경로, 클라이언트 decoder가 먼저 거절하는 응답 형식, `validate()`가 먼저 막는 분기 |

결과(1.96.0·1.85.0): 각 **611 passed**·0 failed·5 ignored. fmt·clippy·rustdoc·표면 가드 Pass. CLI 파일 커버리지: `commands.rs` 84→96.4%, `output.rs` 83→94.4%, `model.rs` 86→99.5%.

**재측정:** 라인 **93.56%**(미실행 1,178), 함수 92.93%, 리전 90.88%. 승인 기준 1(92/92/89)을 충족한다.

라인 85% 미만으로 남은 파일 6개와 그 사유:
- `routing/types.rs`·`capture/types.rs`·`control/types.rs`: `Display`
- `logger/writer.rs`: const fn·`Debug`, 도달 불가 `Skipped` → TASK-034
- `logger/affinity.rs`: `sched_getaffinity` 실패, OS 결함 주입이 필요하다
- `logger/sink.rs`: span 콜백·`Debug`

→ 기준 2(실제 동작 공백 미처리 0)를 충족한다.

**관찰(후속 판단, 일정 판단 밖):**
- CLI `recover`는 원본이 손상돼도 outcome을 `io_or_transport`로 낸다(`commands.rs` 오류 매핑). exit 4는 일반 규칙(「원본 손상은 `invalid_data`/exit4」)과 같지만 outcome 이름이 다르다.
- 읽기 전용 원격 명령(compare·check plan)도 Corrupt를 받으면 보수적으로 `posted_host_io: completion_unconfirmed`를 붙인다.
- offline query의 payload 사전 검사 몇 곳은 blob을 읽기 전에 거르는 최적화일 뿐이다. 결과는 `Output::write` 상한이 결정한다.

## 2026-11-13 공백 시험 추가

| 시험 | 대상 | 확인 |
|---|---|---|
| `tc_013_facade::f8_channel_from_another_logger_is_rejected_and_never_stored` | facade `try_log`의 세션 검사 | Pass. 세션 검사를 끈 변이에서 실패 확인 후 원복 |
| `tc_013_calibrate::f19_cancellation_during_measurement_stops_and_removes_the_probe` | 보정 측정 루프의 취소(TASK-028 잔여) | Pass. 루프의 취소 검사를 끈 변이에서 `Ok(Profile)`로 끝나 실패 확인 후 원복(tmpfs, 변이 1회 512 MiB) |

두 시험 모두 구현은 이미 올바랐고 시험만 없었다. 그래서 Red는 구현 전 실패가 아니라 변이로 확인했다.

## 완료 기준

- [x] S1 커버리지 기준선·파일별 공백 분류(EV-078).
- [x] S2 사용 사례 목록(출처 포함) — 2026-11-15, [조사 문서](../../06_evolution/imported_context/research_2026-11-15_logging_use_cases.md).
- [x] S3 UC↔TC 대응표와 공백 시험 — 2026-11-15(EV-079). 남은 경미 분기는 S4 규칙으로 처리.
- [ ] S4 목표치 Human 승인과 재측정 결과.
- [ ] Human 리뷰 완료.

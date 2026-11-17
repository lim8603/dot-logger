# dot-logger CLI

```sh
cargo run -p dot-logger-cli -- --help
cargo run -p dot-logger-cli -- status --endpoint http://127.0.0.1:50051 --json
cargo run -p dot-logger-cli -- query --endpoint http://127.0.0.1:50051 --from 0 --to 1000000000 --json
```

서버는 호스트가 명시적으로 활성화해야 한다. CLI는 daemon/운영 worker를 시작하지 않는다. token은 기본 `DOT_LOGGER_TOKEN` 환경 변수에서 읽으며 `--token-env NAME`으로 바꿀 수 있다. token을 argv로 받거나 오류에 출력하지 않는다. 외부 endpoint는 HTTPS+token, HTTPS는 `--ca <PEM>`을 요구한다. 인증서 이름을 명시할 때 `--tls-domain NAME`을 사용한다. 평문은 literal loopback IP만 허용한다.

## 명령과 결과

| 명령 | 용도 |
|---|---|
| `status` | session/revision과 host의 `dot_logger.status.v1` 지표. 지표 backend가 없는 host는 Unavailable |
| `query` | `[from,to)`·mono/real/source·channel/level 필터와 유한 페이지 |
| `blob` | `--snapshot --blob --offset --length`로 원격 capability 범위 읽기 |
| `replay` | remote `--handle --action start/pause/resume/seek/stop/speed --generation --speed N/D`; offline timed playback |
| `tail` | `--records --record-bytes --age-ms` 구독, 원본은 `--originals` opt-in, overflow는 Gap 또는 `--disconnect-on-overflow` |
| `completion` | `--session <32자리hex> --producer --sequence --target --boundary`의 완료 관측 |
| `config validate` | offline manifest의 등록/제한·메모리 예약 검증. 원본 scan/runtime 검증이 아님 |
| `config diff` | `--before FILE --after FILE`의 차이 여부만 표시하고 값은 숨김 |
| `config apply` | Manage 권한으로 `--base-revision --patch FILE`(`dot_logger.config_patch.v1`)을 host에 게시 |
| `check plan`, `check run` | `--plan ID`로 등록된 host 계획을 무부하 조회/명시 실행 |
| `compare` | `--before ID --after ID`로 host가 등록한 snapshot을 비교한 `dot_logger.compare.v1` 결과 |
| `recover` | manifest의 파일별 revision으로 독립 복구. 단일 source는 `--output NEW_FILE`, 여러 source는 `--output NEW_DIRECTORY` 아래 `<sequence>.mcap`. 기존 경로 덮어쓰기 없음 |

간편 점검은 호스트가 `diagnostics::CheckHost`에 전용 경로/입력/예산을 등록하고 일반 worker에서 Command를 명시 처리해야 한다. CLI는 등록 ID만 받는다. `check plan`은 경로 생성·실제 capability probe 없이 예산/입력/출처를 보여주며, `check run`은 Manage token이 필요하고 ID를 한 번만 소비한다. 호스트에 backend가 없거나 구 서버의 PlanCheck 미지원이면 Unsupported이며 실행으로 fallback하지 않는다.

```sh
# 7은 예시다. 호스트가 등록해 전달한 ID를 사용한다.
cargo run -p dot-logger-cli -- check plan --endpoint http://127.0.0.1:50051 --plan 7 --json
cargo run -p dot-logger-cli -- check run --endpoint http://127.0.0.1:50051 --plan 7 --json
```

점검 응답은 `dot_logger.check.v1` JSON이다. 실제 파일 결과 Pass는 exit0, Fail은 exit4, Aborted/Inconclusive는 exit3이다. 시간/취소는 새 부하를 중지하며 actual footer/sync/close/정리가 끝난 뒤 결과를 만든다. timeout 뒤에는 `completion_unconfirmed`를 유지하며 자동 재실행하지 않는다. 호스트 message 한도는 최소5120B가 필요하고 부족하면 IO/ID 소비 전 Limit이다. Linux의 실제 free probe는 statvfs를 사용한다. 합성 파일 결과·원래 운영 Report/profile을 분리하며, 서로 다른 source/check session 시각을 빼거나 profile 만료를 갱신하지 않는다. `capacity`/`network_measurement`는 Unavailable이며 실제 생산 capture/수신 ACK·dot-probe/성능 인증은 후속이다.

## 운영 상태·설정 적용·비교

호스트가 `telemetry::operations::OperationsHost`를 만들고 일반 worker에서 `GetStatus`/`ApplyConfig`/`Compare` Command를 `handle_command(command, telemetry, config_updates, now_ns)`로 명시 처리해야 한다. 1초 `Telemetry::sample`·비교용 snapshot 등록(`register_snapshot`)·producer의 `apply_pending_config()` 호출은 호스트 책임이며 자동 worker는 없다.

- `status`의 `metrics`는 최근 표본과 10/60/300초 window를 각각 Observed/Stale/Unavailable로 표시하고, 비교에 쓸 수 있는 snapshot ID 목록을 함께 준다. 미관측 값은 0으로 채우지 않는다.
- `config apply` patch 예시. 알 수 없는 필드는 거절한다. level은 trace/debug/info/warn/error이고, 생략한 목록은 바꾸지 않는다:

```json
{"schema":"dot_logger.config_patch.v1",
 "channels":[{"channel":7,"minimum_level":"warn","sample_every":2}],
 "routes":[{"channel":7,"targets":[10]}]}
```

| 결과 | outcome / exit | 의미 |
|---|---|---|
| 적용 | `applied` / 0 | posted revision. 각 producer의 실제 전환은 별도 |
| base 불일치 | `revision_conflict` / 3 | 게시 없음. status로 revision을 다시 읽고 재시도 |
| 이전 변경 배출 중 | `revision_busy` / 3 | 게시 없음. idle producer poll·old record 배출 후 재시도 |
| memory 변경 | `restart_required` / 2 | 게시 없음. owner 재시작 필요 |
| 잘못된 patch | `invalid_input` / 2 | 게시 없음 |

세 거절 모두 게시가 없으므로 `posted_host_io`를 붙이지 않는다. `compare`는 두 snapshot의 조건 차이를 모두 보존하며 비교 불가 조건이면 `comparable:false`와 수치 Unavailable을 준다. CPU/메모리/지연은 호스트가 측정해 넣지 않으면 Unavailable이다.

Query 결과의 snapshot을 Replay handle로 전달한다. 제어 요청에는 확인된 generation을 유지하고 seek 응답의 새 generation을 반영한다. Snapshot/blob/cursor는 호스트의 유한 수명 capability이며 파일 경로가 아니다. CLI는 실패 후 자동 재연결·재전송하지 않는다.

## 오프라인 파일

[offline_session.json](examples/offline_session.json)을 원본 session·생산자 수·원래 채널 등록 순서·각 source의 revision/sequence/경로와 예산으로 수정한다. schema/descriptor가 있으면 channel의 `descriptor: {"encoding": "...", "data_hex": "..."}`를 원래 값으로 지정한다. 예제 값은 사용 예이며 제품/장비 기본값이 아니다.

```sh
cargo run -p dot-logger-cli -- config validate --manifest session.json --json
cargo run -p dot-logger-cli -- query --manifest session.json --from 0 --to 1000000000 --json
cargo run -p dot-logger-cli -- replay --manifest session.json --speed 2/1 --json
cargo run -p dot-logger-cli -- recover --manifest session.json --output recovered.mcap --json
```

manifest는 `schema_version: 2`이고 전역 revision 대신 각 `sources[]`에 `revision`을 필수로 둔다. v1과 전역 `revision`·형식 선택 필드는 지원하지 않는다. 파일 Metadata의 session/revision/sequence와 등록을 대조하며, 중복 sequence/경로·동일 receipt의 재등장을 거절한다. standalone `SegmentWriter`의 sequence는 0이고, 회전 파일은 catalog의 실제 sequence를 사용한다. offline status는 `sources` 배열에 파일별 sequence/revision을 표시한다.

manifest의 상대 source 경로는 manifest가 위치한 디렉터리 기준이다. 원본 파일은 읽기 전용이며 사용 중 불변 prefix를 유지해야 한다. active 파일은 명시적인 완전 chunk `safe_offset`이 필요하다. 여러 파일 복구는 manifest 순서로 진행하고 source마다 recovery 행에 sequence/revision·복구/제외 범위를 보고한다. `max_segments`는 파일 수, `max_source_bytes`는 개별 파일, `read_bytes`는 전체 원본 길이 합계, `max_records`는 전체 복구 사용자 기록 수를 제한한다. 뒤 파일 실패 시 이미 완료한 앞 파일과 실패 파일의 부분 출력은 유지하며 재시도·합치기·덮어쓰기는 하지 않는다. file sync와 namespace sync는 구분하며 CLI 복구는 `namespace_synced:false`다. 정상 완료에도 과거 프로세스의 Written/Synced를 복원했다고 주장하지 않는다.

단독 offline CLI의 cursor/blob는 프로세스의 Snapshot 수명에 한정된다. 기본 결과는 metadata/descriptor이며 실제 bytes는 `--include-payload`로 명시한다. 대형 원본 읽기는 개별/누적 blob 예산과 출력 예산을 함께 소비한다.

offline replay는 start와 실제 caller clock을 지원한다. 실행 중 제어/화면은 Rust Replay API, 프로세스 간 제어는 remote replay를 사용한다. Ctrl-C는 협력 취소 후 실제 reader/진행 IO 정리를 기다려 부분 결과를 반환한다. 진행 중 OS IO의 강제 취소나 정확한 종료 시간은 보장하지 않는다.

복구는 단일 immutable source의 전체 파일(또는 길이와 같은 safe_offset)을 대상으로 한다. 부분 active prefix의 복구는 Unsupported다. 제외 suffix에는 교체되는 옛 index/summary도 포함되므로 그 전체를 손실된 메시지로 세지 않는다. 생성한 출력의 file sync와 rebuilt index를 확인하나 parent directory sync는 수행하지 않으며 결과에도 `namespace_synced:false`로 표시한다. 오류 시 부분 출력은 조사용으로 남기고 자동 삭제/재시도하지 않는다. writer는 128MiB output partition·1024 chunk 한도를 사용한다. source/writer 예산 초과는 성공으로 보고하지 않는다.

## 출력·취소·한도

`--json`은 각 줄이 독립 객체인 NDJSON이다. 공통 필드는 `schema_version:1`, `command`, `kind`, `data`다. record/gap/status 등 뒤에 summary 또는 error가 온다. 128-bit ID는 32자리 hex, 손실 없는 u64/i128 시각·sequence/revision/handle은 문자열이다. `data_hex`는 명시 payload 출력에서만 제공한다. summary·error의 `records`는 이 명령이 stdout에 낸 `record` 줄 수, `acknowledged`는 그중 ACK한 수다 — `record` 줄을 내지 않는 명령(`calibrate`·`config` 등)은 항상 0이다. 임의 payload나 credential을 오류 메시지에 포함하지 않는다.

| 종료 코드 | 의미 |
|---|---|
| 0 | 요청한 유한 작업/관측 완료 |
| 2 | 입력·설정·권한·미지원 |
| 3 | 부분 완료·취소·기한/출력·읽기 한도·Gone/만료·ACK 불명 |
| 4 | IO·통신·불량 응답 |

기본 전체 작업 기한 5초(`--timeout-ms`, 최대1시간), output 16MiB(`--max-output-bytes`, 1024B~256MiB), 기록 상한1000(`--limit`, 최대100만), message cap263168B다. 데이터 출력은 terminal summary 공간을 남긴다. `tail --limit`은 명시한 기록 수를 stdout에 인계하고 ACK한 뒤 정상 종료한다. query/replay가 전체 선택을 끝내기 전에 한도에 닿으면 부분 완료다. stdout write/flush가 실패한 기록에는 ACK하지 않는다. 소비 인계 뒤 ACK 확인 실패는 Unknown이며 재전송으로 성공을 추정하지 않는다.

`gone_or_expired`, `permission_denied`, `incompatible_protocol`, `revision_conflict`, `revision_busy`, `restart_required`를 일반 통신 오류와 구분한다.

Ctrl-C/기한 초과가 이미 게시한 host 명령/IO를 취소하거나 미완료를 성공으로 바꾸지 않는다. 원격 취소/기한 결과의 `posted_host_io` 안내와 완료/상태 조회를 함께 사용한다. Metrics 없는 status에 0·성능 합격 같은 값을 만들어 넣지 않는다. 알 수 없는 status/compare report schema는 `invalid_data`/exit4다.

REV-022 보강: `tail`의 전체 입력/예약 검증은 연결 전에 수행한다. local replay는 정확히 limit개로 선택 범위가 끝나면 정상 완료하고 추가 기록이 있을 때만 부분 완료(exit3)다. `--limit` 이후 추가 기록은 출력하지 않는다. tail은 계속 전체 `--timeout-ms`에 제한되며, SDK의 로컬 next 대기 재시도와 무관하게 전체 기한을 자동 연장하지 않는다. `--age-ms`는 기록 수명 및 서버 idle lease 요청을 겸한다. 서버 lease 종료/새 구독 사이 누락은 Gap으로 보장되지 않으므로 별도 구독을 연속 수신이라고 해석하지 않는다.

만료/세대 폐기로 handoff가 사전 거절되면 `gone_or_expired`/exit3이고 ACK는 없다. 원본 손상은 `invalid_data`/exit4, 메시지 크기 상한은 `limit`/exit3이다. 원격 config apply/check run/replay의 Gone도 이미 게시된 호스트 작업의 완료를 증명하지 않으므로 `posted_host_io: completion_unconfirmed`를 표시한다. 읽기 전용 capability Gone과 Unsupported 응답의 기존 분류는 유지한다.

저장 파일은 `dot-logger/standard/1` native MCAP이며 사용자 payload와 schema를 직접 담는다. `_dot_logger/meta`가 원래 receipt·시계·설정 정보를 보존한다. 별도 `export --standard` 변환은 제공하지 않는다. 구형 파일은 지원하지 않는다.

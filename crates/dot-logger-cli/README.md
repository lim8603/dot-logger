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
| `status` | session/revision. 아직 없는 runtime 지표는 Unavailable |
| `query` | `[from,to)`·mono/real/source·channel/level 필터와 유한 페이지 |
| `blob` | `--snapshot --blob --offset --length`로 원격 capability 범위 읽기 |
| `replay` | remote `--handle --action start/pause/resume/seek/stop/speed --generation --speed N/D`; offline timed playback |
| `tail` | `--records --record-bytes --age-ms` 구독, 원본은 `--originals` opt-in, overflow는 Gap 또는 `--disconnect-on-overflow` |
| `completion` | `--session <32자리hex> --producer --sequence --target --boundary`의 완료 관측 |
| `config validate` | offline manifest의 등록/제한·메모리 예약 검증. 원본 scan/runtime 검증이 아님 |
| `config diff` | `--before FILE --after FILE`의 차이 여부만 표시하고 값은 숨김 |
| `config apply` | Manage 권한으로 `--base-revision --patch FILE`을 host에 게시 |
| `check run`, `compare` | 등록된 plan/report capability를 host에 전달 |
| `recover` | 원래 manifest와 `--output NEW_FILE`로 독립 새 파일 복구. 기존 경로 덮어쓰기 없음 |

운영 설정/간편 점검/비교 backend는 후속 TASK-016/017 등에서 구현한다. 현재 호스트가 미지원하면 Unsupported를 전달하며 `check plan`도 그 backend 전에는 Unsupported다. ApplyConfig의 응답 revision과 host-owned Report를 전달하는 것은 해당 backend를 CLI가 구현했다는 뜻이 아니다.

Query 결과의 snapshot을 Replay handle로 전달한다. 제어 요청에는 확인된 generation을 유지하고 seek 응답의 새 generation을 반영한다. Snapshot/blob/cursor는 호스트의 유한 수명 capability이며 파일 경로가 아니다. CLI는 실패 후 자동 재연결·재전송하지 않는다.

## 오프라인 파일

[offline_session.json](examples/offline_session.json)을 원본 session/revision·생산자 수·원래 채널 등록 순서·파일 sequence/경로와 예산으로 수정한다. schema/descriptor가 있으면 channel의 `descriptor: {"encoding": "...", "data_hex": "..."}`를 원래 값으로 지정한다. 예제 값은 사용 예이며 제품/장비 기본값이 아니다.

```sh
cargo run -p dot-logger-cli -- config validate --manifest session.json --json
cargo run -p dot-logger-cli -- query --manifest session.json --from 0 --to 1000000000 --json
cargo run -p dot-logger-cli -- replay --manifest session.json --speed 2/1 --json
cargo run -p dot-logger-cli -- recover --manifest session.json --output recovered.mcap --json
```

manifest의 상대 source 경로는 manifest가 위치한 디렉터리 기준이다. 원본 파일은 읽기 전용이며 사용 중 불변 prefix를 유지해야 한다. active 파일은 명시적인 완전 chunk `safe_offset`이 필요하다. 단독 offline CLI의 cursor/blob는 프로세스의 Snapshot 수명에 한정된다. 기본 결과는 metadata/descriptor이며 실제 bytes는 `--include-payload`로 명시한다. 대형 원본 읽기는 개별/누적 blob 예산과 출력 예산을 함께 소비한다.

offline replay는 start와 실제 caller clock을 지원한다. 실행 중 제어/화면은 Rust Replay API, 프로세스 간 제어는 remote replay를 사용한다. Ctrl-C는 협력 취소 후 실제 reader/진행 IO 정리를 기다려 부분 결과를 반환한다. 진행 중 OS IO의 강제 취소나 정확한 종료 시간은 보장하지 않는다.

복구는 단일 immutable source의 전체 파일(또는 길이와 같은 safe_offset)을 대상으로 한다. 부분 active prefix의 복구는 Unsupported다. 제외 suffix에는 교체되는 옛 index/summary도 포함되므로 그 전체를 손실된 메시지로 세지 않는다. 생성한 출력의 file sync와 rebuilt index를 확인하나 parent directory sync는 수행하지 않으며 결과에도 `namespace_synced:false`로 표시한다. 오류 시 부분 출력은 조사용으로 남기고 자동 삭제/재시도하지 않는다. writer는 128MiB output partition·1024 chunk 한도를 사용한다. source/writer 예산 초과는 성공으로 보고하지 않는다.

## 출력·취소·한도

`--json`은 각 줄이 독립 객체인 NDJSON이다. 공통 필드는 `schema_version:1`, `command`, `kind`, `data`다. record/gap/status 등 뒤에 summary 또는 error가 온다. 128-bit ID는 32자리 hex, 손실 없는 u64/i128 시각·sequence/revision/handle은 문자열이다. `data_hex`는 명시 payload 출력에서만 제공한다. 임의 payload나 credential을 오류 메시지에 포함하지 않는다.

| 종료 코드 | 의미 |
|---|---|
| 0 | 요청한 유한 작업/관측 완료 |
| 2 | 입력·설정·권한·미지원 |
| 3 | 부분 완료·취소·기한/출력·읽기 한도·Gone/만료·ACK 불명 |
| 4 | IO·통신·불량 응답 |

기본 전체 작업 기한 5초(`--timeout-ms`, 최대1시간), output 16MiB(`--max-output-bytes`, 1024B~256MiB), 기록 상한1000(`--limit`, 최대100만), message cap263168B다. 데이터 출력은 terminal summary 공간을 남긴다. `tail --limit`은 명시한 기록 수를 stdout에 인계하고 ACK한 뒤 정상 종료한다. query/replay가 전체 선택을 끝내기 전에 한도에 닿으면 부분 완료다. stdout write/flush가 실패한 기록에는 ACK하지 않는다. 소비 인계 뒤 ACK 확인 실패는 Unknown이며 재전송으로 성공을 추정하지 않는다.

`gone_or_expired`, `permission_denied`, `incompatible_protocol`을 일반 통신 오류와 구분한다.

Ctrl-C/기한 초과가 이미 게시한 host 명령/IO를 취소하거나 미완료를 성공으로 바꾸지 않는다. 원격 취소/기한 결과의 `posted_host_io` 안내와 완료/상태 조회를 함께 사용한다. Metrics 없는 status에 0·성능 합격 같은 값을 만들어 넣지 않는다.

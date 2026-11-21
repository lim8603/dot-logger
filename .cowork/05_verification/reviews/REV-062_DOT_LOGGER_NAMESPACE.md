# REV-062 — `.logger` 예약 토픽·통합 예제 자가검토

| 항목 | 내용 |
|---|---|
| 일자 | 2026-11-18 |
| 대상 | `3b63219..aaf8bdc`, ADR-013 개정 |
| 판정 | **Pass** — [EV-090](../verification_evidence.md#ev-090) |
| 상태 | **Closed** — 기존 위임에 따른 검증·수락 |

| 파일 | 검토한 동작 |
|---|---|
| `storage/codec.rs`, `storage/read.rs` | 새 profile과 Metadata 형식 버전, writer·reader·복구가 공유하는 예약 토픽, 사용자 채널 충돌 거절 |
| `storage/record_meta.proto`, 저장·조회 주석 | 변경된 토픽·profile 표기, RecordMeta descriptor 재생성 결과 byte 일치 |
| TC-013/014/015 | 충돌·기록·독립 MCAP 판독·이전 profile 거절 회귀 |
| README·rustdoc·CLI README | 호스트가 `.logger/`를 넘기는 예제와 `.logger/session.json` 상대 경로 |

`FileInfo`의 파일 Metadata 기록·판독부에 형식 버전 2를 반영했다. RT 수집, 메모리 수명, 공개 Rust 타입 검토에서 신규 지적은 없었다. 이전 `standard/1`은 query/loss에서 Unsupported이고, recover는 기록을 복구하지 않고 실패했다. CLI recover가 실패 뒤 빈 출력 파일을 남기는 기존 동작은 [EV-090](../verification_evidence.md#ev-090)에 기록했다.

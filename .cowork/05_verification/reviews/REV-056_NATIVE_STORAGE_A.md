# REV-056 — TASK-030 S2 A native 저장·읽기 전환 개발 자가검토

| 항목 | 값 |
|---|---|
| 상태 | Closed — RV-141~144 수정·회귀·위임 수락 / 로컬 Pass |
| 날짜·검토자 | 2026-11-16 / AI(개발 세션 자가검토) |
| 기준 | `3a78494`, ADR-013 Accepted D1~D3 중 A |
| 범위 | native payload/RecordMeta pair writer/reader, 단일 revision query/recovery 정렬, 구형 경로·fixture/export 제거, 소비 경로 이관 |
| 범위 밖 | B 다중 revision·manifest v2, TASK-035 loss 수집, TASK-034 runtime WIP, 커버리지/성능/GUI 재측정 |
| 검증 | [EV-083](../verification_evidence.md#ev-083), 제품 커밋 048cdf3 |

## 요약표

| ID | 중요도 | 위치·계약 | 문제·처리 제안 | 판정/상태 |
|---|---|---|---|---|
| RV-141 | P1 | NativeReader producer 상태 할당 | 파일의 producer 수가 usize::MAX일 때 caller bound가 크면 capacity overflow 패닉. byte 폭을 할당 전에 검사 | 수용·수정·EV-083 Pass/Closed |
| RV-142 | P2 | native 등록 identity | 독립 reader가 서로 다른 MCAP channel의 product ID 중복을 허용. writer 등록 제약 공유 | 수용·수정·EV-083 Pass/Closed |
| RV-143 | P2 | RetentionConfig 최소 index 한도 | 두 물리 Message 예약 증가 후 생성 최소값1088B가 남아 불충분 설정 수락. 최소1152B 검사 | 수용·수정·EV-083 Pass/Closed |
| RV-144 | P2 | FileInfo::parse 제어 메타 예산 | generic map 할당 전 자체 body 상한 없음. 고정8필드의1KiB 한도를 먼저 검사 | 수용·수정·EV-083 Pass/Closed |

## 상세 내역

### RV-141

- **[리뷰어]** 자체 메타의 producer 수를 caller bound로만 확인하면 유효 usize이지만 Vec 원소 byte 폭을 넘는 파일이 패닉한다. 공개 reader는 정상 오류로 끝나야 한다.
- **[처리담당]** 수용. `read::registrations`에서 producers × size_of(u64) ≤ isize::MAX를 확인하고 오류를 반환한다. 수정 전 시험에서 capacity overflow Red, 수정 후 TC-015 `untrusted_file_metadata...` Green. 일반 writer/query/recovery의 기존 예약 한도는 유지한다.

### RV-142

- **[리뷰어]** file-local channel 두 개가 동일 product channel을 나타내면 원래 product identity가 모호하다. 파일을 생성할 때 거절하는 등록을 자체 판독에서도 허용하면 안 된다.
- **[처리담당]** 수용. `writer::registration_bytes`를 reader/writer가 공유하여 product ID 유일성, schema ID/version과 descriptor 일치, 예약 topic/schema, 문자열 길이를 동일 검증한다. CRC를 다시 계산한 중복 파일에서 수정 전 assertion Red, 수정 후 `reader_rejects_two_file_channels...` Green.

### RV-143

- **[리뷰어]** `projected_index_bytes`는 한 논리 기록에128B를 잡지만 `RetentionConfig::validate` 최소값은 구형64+512+512였다. 작은 설정을 수락하면 첫 append에서 빈 파일 회전을 하고도 새 index 예산을 넘는다.
- **[처리담당]** 수용. `writer::MIN_INDEX_BYTES=128+512+512`를 Catalog 검사가 사용한다. TC-005에서1151B 설정이 파일 생성 전에 InvalidConfig가 되어야 한다는 Red를 확인하고 수정했다. 최소1152B에서는 기존 index rotation 시험이 사용자1건 뒤 정상 회전한다. 최종 소스에서 두 Rust 전체 회귀630개/18 feature 구성 모두 Pass.

### RV-144

- **[리뷰어]** native file Metadata는 고정8개 제어 필드인데 일반 map parser로 먼저 넘겨 임의 입력의 임시 할당이 커질 수 있었다. schema/channel 안의 조작된 길이도 owned 파싱 전에 실제 body와 대조해야 한다. source record 한도와 별도로 제어 메타의 고정 예약 범위를 적용해야 한다.
- **[처리담당]** 수용. `FileInfo::parse`는 body>1024B를 먼저 거절하고 문자열을 빌려 최대8개 키만 파싱한다. 등록 schema/channel은 모든 길이를 borrowed Fields로 검사한 후에만 MCAP이 owned 데이터를 만든다. 최대폭의8개 필드를 충분히 수용하며 writer 출력은 바뀌지 않는다. 긴 숫자 표현과 u32::MAX schema/channel 문자열 길이를 넣어 기존 TC-015를 보강했다. 긴 숫자 표현에서 constructor가 수락하는 Red를 확인하고 제한 후 Green. 최종 두 Rust workspace·clippy·문서와 feature 조합 검증은 EV-083의 실행 범위를 따른다.

## 파일 변경표

| 구분 | 파일/그룹 | 내용 |
|---|---|---|
| 생성 | storage/read.rs, tests/tc_015_native.rs | 명시 제한 borrowed 판독, chunk 전체 선검증·native 회귀11개 |
| 수정 | storage/codec.rs·writer.rs·retention.rs·catalog.rs·mod.rs | RecordMeta, 파일 Metadata, pair 단위 저장/예산·실제 file sequence |
| 수정 | storage/recovery.rs·query/mod.rs | 공통 native pair 검증·직접 payload offset·복구 재작성 |
| 삭제 | storage/standard.rs·envelope.proto/bin·golden | 구형 codec/변환 전용 제품 경로 |
| 수정 | CLI args/commands/main/output·README, core/client 예제·시험 | 구형 명령 제거, 단말 출력 cap 회귀 이관, 소비자 새 판독 |
| 수정 | API·data_model·Task/MS/상태·EV | 구현 범위·새 형식·미실행/후속 단계 동기화 |

## 타입 변경표

| 구분 | 타입 | 변경 |
|---|---|---|
| 생성 | NativeReader, FileInfo | file/등록 정보, bounded producer 상태, borrowed 논리 record iterator |
| 수정 | StoredRecord<'a> | payload Vec → source bytes borrow |
| 수정 | SegmentWriter, SegmentLog, Validation, RetentionConfig | meta channel·파일 sequence·pair 검증/논리 수·최소 index 예산, legacy payload scratch 제거 |
| 생성/수정 | Meta, Source(비공개) | 기존 RecordMeta 태그1~9; 외부 wire 타입 공개 없음 |
| 수정/삭제 | StorageError, Envelope, Export* | UnsupportedFormat, 구형 UnsupportedVersion·Envelope·변환 전용 타입 제거 |

## 검토 포인트와 마감

- RT 수집/거절/Drop·unsafe·원자 순서 변경 없음. 새 prost encode·파일 IO·등록 할당은 일반 저장/판독 경로다.
- pair 앞 flush/회전·logical max_records·physical Statistics, 실패 후 poison·safe offset·Written·sync 채무·event/pin 의미를 확인했다. 기존 손상/복구·완료·allocator 회귀를 유지했다.
- native 데이터 전체를 신뢰하지 않으며 CRC 재계산된 의미 손상도 반출 전에 거절한다. native reader는 압축·임의 MCAP·구형 profile을 지원하지 않는다.
- fmt/doc/clippy·두 Rust 회귀630개·각18 feature·표면 가드 모두 Pass. Open(개발 진단) → In Progress(수정) → Resolved(검증) → Closed(2026-11-16 위임 수락)로 마감했다. 새 H/J 범위 변경·재승인은 없다.

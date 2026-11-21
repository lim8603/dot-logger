# [REV-013] TASK-010 조회·active snapshot 개발 자가 리뷰

## 개요

- 일자/리뷰어: 2026-10-30 / AI, [code-review 스킬](../../../.agents/skills/code-review/SKILL.md).
- 대상: TASK-009 승인/게시 소스5c60399·문서74fc9ba 위 TASK-010 query·pin 수명·통합/API consumer/allocator·CI recipe 변경.
- 판정: Conditional — 최초 행동 Red6개 후 조회/정렬/페이지/blob/공유 예약·읽기 수명 구현. 최종 EV-016 로컬 Pass, Human 최종 리뷰/native·실장비 잔여.
- 상태: Closed — 2026-10-31 Human 최종 구현 승인 (EV-016/017)

## 리뷰·처리 요약

| ID | 중요도 | 위치/문제 | 처리 | 상태 |
|---|---|---|---|---|
| RV-018 | P2 | Snapshot의 만료 판정이 각 호출의 now만 검사하여 과거 now를 넘기면 이미 만료된 cursor를 다시 읽을 수 있음 | 최대 관측 시각 Cell로 만료를 단조롭게 유지, 결정적 Red→Green 회귀 | Closed |
| RV-019 | P2 | `read_blob` 만료가 §14의 Gone 계약 대신 공통 Expired를 반환 | blob 경로의 만료를 Gone으로 매핑, 만료 후 과거 now 재시도까지 Red→Green 회귀 | Closed |
| RV-020 | P2 | 설정한 frame body 상한 초과를 CorruptData로 뭉뚱그려 입력 파일 손상과 자원 한도를 구분하지 못함 | scan_stop에서 Limit/Unsupported/CorruptData를 분리, body 한도 반례 Red→Green | Closed |

## RV-018 상세

### [리뷰어]

`page(cursor,100)`이 Expired를 반환한 뒤 `page(cursor,1)`이 다시 성공하는 반례다. cursor/blob 수명은 snapshot과 pin deadline을 넘겨 되살릴 수 없어야 한다. 소비자는 start·page·read_blob 및 borrowed page/원본 descriptor다.

### [처리담당]

- 판정: 수용(A — 승인된 만료/읽기 보호 계약의 구현 세부 수정).
- `observed_expiry_cannot_be_reversed_with_an_older_time_argument`에서 컴파일되는 반례 Red를 확인했다(Red(비공개 원시 근거: `imported_context/task_010_2026-10-30/red-expiry.log`)). scan 인수 tuple 정리의 컴파일 오류는 별도 harness 로그이며 행동 Red로 세지 않는다.
- `Snapshot::alive`가 최대 observed 시각을 Cell에 보관한다. snapshot deadline은 source pin deadline과 요청 deadline의 최소값이며, 모든 start/page/blob 경로에서 검사한다. 실제 진행 중 IO는 반환을 기다리고 source Drop 전에 pin을 풀지 않는다.
- 최종 Green/소스 근거는 [EV-016](../verification_evidence.md#ev-016)이다.

## RV-019 상세

### [리뷰어]

인터페이스 §14의 원본 읽기는 만료/Gone을 약속하지만 최초 query 구현의 공통 alive 검사는 Expired를 그대로 반환했다. cursor 만료와 blob 유실/만료의 공개 오류 계약을 구분해야 한다.

### [처리담당]

- 판정: 수용(A — 이미 승인된 오류 계약과의 정렬).
- 기존 blob 범위 통합에 deadline 시각 및 과거 시각으로 재호출하는 Gone 기대를 추가해 행동 Red(비공개 원시 근거: `imported_context/task_010_2026-10-30/red-blob-expiry.log`)를 확인했다.
- `read_blob`의 IO 전/후 alive 실패 중 Expired만 Gone으로 매핑한다. 취소·원 OS 오류·범위/한도 분류는 보존한다. cursor는 Expired를 유지한다.
- 직전 matrix9 명령·Miri1024 Pass 뒤 변경이므로 원본을 interim에 보존하고 최종53 source로 전체 matrix/Miri를 재실행한다. interim matrix의 최종 manifest guard는 README 설명의 일시 수정/원복 시점 때문에 exit1이었다. 실행 명령9개는 모두0이며 실행 코드 변동은 없었다. 이를 최종 소스 증거로 대체하지 않고 아래 EV-016 최종 실행만 현재 근거로 사용한다.

## RV-020 상세

### [리뷰어]

`max_record_bytes=40`으로 정상 MCAP의 더 큰 body를 읽으면 공유 recovery scanner는 Limit을 반환하지만 query의 공통 invalid 매핑이 CorruptData로 바꿨다. 설정한 자원 상한 초과를 파일 손상으로 보고하지 않아야 한다.

### [처리담당]

- 판정: 수용(A — 승인된 유한 조회/오류 분류의 구현 세부 정렬).
- `malformed_or_over_limit_sources_do_not_produce_partial_snapshots`에 body 크기 한도 Limit 기대를 추가해 행동 Red(비공개 원시 근거: `imported_context/task_010_2026-10-30/red-frame-limit.log`)를 확인했다.
- `scan_stop`에서 scanner의 Limit·지원하지 않는 압축·손상을 각 QueryError로 유지한다. 실제 IO 원인은 그대로 보존한다.
- 해당 테스트 확장으로 fake reader의 공유 seek 위치가 writer에 영향을 주는 fixture 결함도 드러났다. host 계약처럼 clone별 독립 offset과 공유 bytes를 사용하도록 수정했다. 이는 제품 코드 결함으로 세지 않는다. 중간 실패/중단 로그는 interim-frame-limit에 보존하고 query9 Green 뒤 최종 source53을 동결했다.

## 자가 점검

- 공개 API에 MCAP/wire/runtime 내부 타입을 노출하지 않는다. query는 일반 동기식 Read+Seek이고 RT 수집/거절/Drop·private unsafe·dependency/lock을 변경하지 않는다.
- recovery의 기존 Fields/Validation/read_frame/CRC를 crate-private로 공유해 complete chunk 검증과 동일한 경계를 사용한다. query는 opaque payload 위치를 체크된 산술로 계산하고 protobuf payload 중복/group wire를 거절한다. 원본 prefix 불변성·정확한 독립 descriptor 대응은 host 계약이다.
- 외부 source 크기/고정 frame buffer·등록/producer/record/segment 한도·shared external 예약을 IO 전에 검사한다. 페이지는 실제 메타데이터 크기와 inline bytes를 함께 제한하고 큰 원본은 파일 참조다. 페이지와 blob 반환은 snapshot borrow에 묶인다.
- 낮은 ReadPin의 Drop/만료는 여전히 보호를 유지한다. reader-owned permit만 실제 reader 종료 뒤 Release를 게시한다. catalog는 Acquire로 보호/슬롯을 확인하며 저수준 분실/forget/외부 background IO의 강제 회수는 없다.
- source pin은 active→closed 회전을 따라가며 시한 경과만으로 삭제하지 않는다. pin identity가 catalog 예약을 보유해 catalog보다 오래 사는 reader의 메타데이터 예산도 남긴다.
- 조회 순서는 벽시계 역행/동시각을 포함해 결정적이다. source ticks는 명시 domain 일치만 허용하고 clock 변환/UTC 보정을 꾸미지 않는다. live late 표시·replay/remote/CLI는 후속이다.
- closed도 scan 재구성 경로를 사용한다. standard summary로 scan을 생략하는 최적화는 미구현이며 제품 latency/IO 효율 달성으로 주장하지 않는다.

## 최종 검증

[EV-016](../verification_evidence.md#ev-016)·source53개/명령/로그 hash(비공개 원시 근거: `imported_context/task_010_2026-10-30/results.json`)로 고정했다. 최종 matrix9+Miri1=검사10개 exit0, source53개가 두 실행/현재 작업 트리와 일치한다. 두 Rust16 features/독립 실제 파일 consumer·예제·clippy/rustdoc/fmt·allocator10 및 Miri128×8=1024 Pass다. query256B/8MiB peak11143/25176199 ≤ external 예약110776/67217592 bytes, 잔여0·기존 RT0/0을 유지한다. 자가 검토에서 추가 미해결 P1/P2는 확인하지 못했다. Human 최종 구현 승인 전 Closed로 바꾸지 않는다. native/장비·closed summary scan 생략 최적화/재생/remote/CLI 미구현 범위를 유지한다.

후속 커밋 — 2026-10-30: Human 「커밋해줘」에 따라 TASK-010 소스 `4a54bcf`를 로컬 커밋했다. EV-016 source53개와 commit blob hash가 일치하여 기존 성공 시험을 반복하지 않았다. 커밋 대조(비공개 원시 근거: `imported_context/task_010_2026-10-30/commit.json`). 문서는 소스와 분리 커밋하며 푸시는 하지 않는다. 커밋 요청을 최종 구현 승인으로 해석하지 않고 REV-013 Resolved·Human 리뷰/native·장비 대기와 세션 종료 상태를 유지한다. 위 미커밋/commit 미실행 문장은 검증·마감 당시 이력이다.

## Human 최종 구현 승인 — 2026-10-31

Human 「승인, 다음작업 진행하자」로 직전 보고의 TASK-008 REV-009/010, TASK-010 REV-013/014, TASK-011 REV-017 구현·리뷰 조치를 수락했다. 본 리뷰와 해당 RV 항목은 Closed다. EV-016/017의 검증 근거 및 현재 구현 범위를 승인하며, 앞선 Resolved/승인 대기 본문은 승인 전 이력으로 보존한다. 낮은 ReadPin 유실의 강제 회수 없음·실제 reader close 뒤 반환 계약과 native 공개 후(T3)·현재 PC 실측/UPS 시뮬레이션 잔여는 유지한다. Task/MS 및 Gate4/5 완료를 뜻하지 않는다. 다음 작업은 TASK-022 프로필 구체화이며 커밋/푸시는 수행하지 않았다.

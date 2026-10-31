# 테스트 케이스와 요구사항 추적

- INT-001 / 2026-10-31 #014 / 상세 시험계획 Approved. TASK-002/003 당시 로컬/native Pass, TASK-004~015 로컬 구현·리뷰 조치는 EV-029까지 수락(REV-001~025 Closed). 최신 native/운영·실앱/실측 TC와 TASK-016 이후는 미완료다.
- TDD 방식은 승인됨. 아래 세부 기대값은 ADR-008·009/추가 AC와 함께 기준선으로 승인됐다. FR-015 관련 시험은 별도 범위 승인 후 활성화한다.
- TC는 12개 검증 묶음이며 각 파라미터·실패 시나리오는 실행 때 독립 결과를 남긴다. 한 하위 시험 Pass를 TC 전체나 요구 전체 Pass로 표시하지 않는다.

## 1. 운영 규칙

설계 검토 [x]는 Human 합의, 구현 [x]는 해당 MS의 코드/리뷰 완료, 테스트 [x]는 요구된 시나리오와 환경의 유효 EV가 있을 때만 표시한다. 일부 구현·모의 UPS 성공·교차 빌드만으로 나머지를 체크하지 않는다. 코드/계약 변경으로 근거가 무효가 되면 해당 검증을 재검증 상태로 되돌리고 과거 EV는 보존한다.

테스트 파일 후보는 `tests/tc_NNN_*.rs`이며 실제 이름·명령은 구현 Task에서 고정한다. TC-001의 TASK-002 실제 파일은 `crates/dot-logger/tests/tc_001_*.rs`와 독립 `tests/api-consumer`다. 나머지 계획 경로는 존재/실행된 것으로 오인하지 않는다. MS 인수 때 구현에는 commit/파일, 테스트에는 TC 하위 시나리오·EV/환경을 기록한다.

## 2. 케이스

<a id="tc-001"></a>
### TC-001 — 설정·공개 API·문서 계약

- 관련: MS-001·002·007 / FR-001 / NFR-004 / CON-001~007
- 상태: 부분 실행 / 최근 실행: 2026-10-27 TASK-002 native CI 6조합+audit Pass / [EV-001](verification_evidence.md). CLI·runtime 수명·실장비 환경 등 나머지는 Not Run.

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 설정 fixture: 총 64 MiB, quota 합 65 MiB; required 미지원 capability; default/각 feature 조합 | 재현 가능한 설정/입력·예산 기록 |
| 2 | 설정 검증·독립 consumer crate build·CLI API 문서 대조 | 잘못된 구성 시작 전 거절, 선택 의존성 없이 파일 API 사용 가능, 추적 누락 0 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

수집 범위·artifact/기술/승인 일치 리뷰와 compile-fail 포함. 구현 전 문서 검사와 이후 실행을 분리.

TASK-002 하위 실행: 구성 경계 13개·전체 feature 의존 probe 6개·불변 구성 compile-fail 1개. Rust 1.85.0/1.96.0 각각 16 feature 조합·독립 consumer·clippy, 개발 fmt/rustdoc Pass. MCAP 왕복은 의존성 시험이며 제품 파일 API 완료가 아니다. 상세 명령·환경·source hash·로그는 EV-001, native CI는 GAP-004, `paste` 경고는 GAP-005다. Human 구현 리뷰는 2026-10-27 승인됐다. native CI는 2026-10-27 6조합+audit Pass(GAP-004 Closed). MS 전체 범위가 남아 있으므로 관련 MS 체크는 유지한다.

<a id="tc-002"></a>
### TC-002 — RT 수집·포화·원본 수명

- 관련: MS-002 / FR-002·007·008·009·012 / NFR-001·005
- 상태: 부분 실행 — TASK-003 소형 ingress 로컬/native CI Pass, TASK-004 원본/반환 로컬 Pass(2026-10-27), Human 리뷰 승인·TASK-004 native 계정 사유 실행 전 차단·후속/실장비 잔여 / EV: [EV-002](verification_evidence.md#ev-002--task-003-소형-수집예약일반-대기)

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | producer 2개 각각 slots=8, Important reserve=2; 원본 슬롯 4개; 할당/해제 감시 | 재현 가능한 설정/입력·예산 기록 |
| 2 | Normal 6개→추가 Normal→Important 2개→추가 Important; 거절/생략 loan Drop; 회수 중지→재개 | 예약 보호·즉시 거절/소유권 반환, RT allocation/deallocation 0, 재개 후 모든 슬롯 정확 1회 회수 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

FIFO·중요도 매핑·동일 취급 모드(예약 0)·일반 deadline·owner보다 오래 사는 핸들 compile-fail·메모리 순서 모델/해석 검증. p99는 TC-012에서 별도.

TASK-003 하위 결과: 소형 통합 16개·경계 2개·수명 compile-fail 4개·allocator guard 1개 Pass. 두 생산자 동시 FIFO/고정 복사·예약 0/전체·513 B·예산 경계·sequence 소진·카운터 wrap·일반 대기 재개/close를 검증했다. guard의 소형 수락/거절/생략/마지막 핸들 Drop 구간 allocator 호출 0/0. 원본 loan/회수·원본 메모리 순서 모델은 TASK-004, p99·실제 RT는 TC-012다. 이 결과로 TC-002 전체 Pass를 표시하지 않는다.

TASK-004 하위 결과: [EV-003](verification_evidence.md#ev-003--task-004-원본-풀수명반환) 로컬 Pass. 원본 통합 17개·신규 private 경계 4개·신규 수명 compile-fail 7개·원본 allocator guard 1개, Miri(strict provenance, 8 seeds, lib 6개+원본 17개) Pass. 동일 loan 반환/생략·pool/producer/class 불일치·원본 예약 bytes·small/raw FIFO·no-copy 원본 reader·pin 중 재사용 금지·포화/coalescing 알림 이후 한정 스캔·stale/중복/최종 generation·동시 Drop/읽기·관리 host 예제를 검증했다. Human 리뷰 승인·변경분 native CI(계정 사유 차단)·실제 dot-probe 연결·p99/장비 시험은 잔여이며 TC-002 전체 Pass가 아니다.

<a id="tc-003"></a>
### TC-003 — fanout·완료·설정과 종료 경합

- 관련: MS-002 / FR-003·007·012·013 / NFR-003·005
- 상태: 부분 실행 — 기존 로컬 Pass·Human 승인 이력 유지. EV-006 RV-001~004는 EV-007, EV-008 RV-005는 EV-009 수정·로컬 회귀 Pass·Human 승인. native·runtime 설정/후속 writer·장비 경합 잔여 / EV: [EV-004](verification_evidence.md#ev-004--task-005-분배출력-격리예산), [EV-007](verification_evidence.md#ev-007), [EV-009](verification_evidence.md#ev-009)

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 필수 파일 A, 선택 B, 완료 window=8, seq1 pending·seq2 완료 | 재현 가능한 설정/입력·예산 기록 |
| 2 | B 정지/포화·A sync 실패·조회 timeout·이력 축출·shutdown과 record/sync 게시 경쟁 | A/B 별도 상태, highest seq로 전체 완료 오판 없음, expired/unknown 구분, 안전한 owner 유지·진입한 sync 게시/IO 종료 전 Closed 금지 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

입력 수락/대상 분배와 최종 상태 대조, 완료 추적 포화 시 신규 수락 거절, 예산 상한.

TASK-005 하위 결과: 분배 통합19·신규 compile-fail3, 두 Rust 각16 feature·독립 consumer·clippy/rustdoc/fmt·allocator 포화256회/512건·Miri8 seeds Pass. A 정지/B 지속, record/byte 공정성·혼합 FIFO·0-byte·startup 예산 경계, pending/queued/진행 IO 보유 한도·raw 최종 guard 회수·retry/expiry/cancel/close·event buffer 포화를 검사했다. 물리 원본은 한 번, 대상별 보유 원인/복사는 별도다. 완료 window·seq 공백/완료·sync 실패·이력/실제 shutdown 경합은 TASK-006 이후이며 TC-003 전체 Pass가 아니다. CI는 Human 확인 사용량 소진으로 미실행이다.

TASK-006 하위 결과: [EV-005](verification_evidence.md#ev-005--task-006-완료-추적명시-sync종료) 로컬 Pass. 완료 통합12·제어14·private publication 경계2·신규 lifetime compile-fail2, tracked RT 0/0·query/sync128회·두 Rust16 features/consumer/clippy/rustdoc·Miri70×8 seeds Pass. seq 공백/추적 포화/동일 loan 반환·단계별 확인/실패/Unknown·bounded page/scope/wait·retention age/Expired/retired 대조·유한 feedback Busy·stale generation/세션·동일 target sync 직렬화·기한 중 IO/loan 유지·전용 shutdown slot·수락/close 경합을 검사했다. 실제 파일 writer/OS sync/앱 thread join·runtime 설정 변경 경합·native/실장비는 남아 있어 TC-003 전체 Pass가 아니다.

TASK-001~006 후속 자가 리뷰: [EV-006](verification_evidence.md#ev-006). source36개가 EV-005와 같음을 확인했고, 두 Rust에서 신규 반례 3개가 각각 실패했다. writer 실패→뒤늦은 Enqueued feedback의 영구 fault, Normal의 완료창 점유에 따른 Important 예약 미보호, 필수 shutdown sync의 시작 전 만료 후 cleanup 정체다. 기존 성공 시나리오의 Pass는 보존하되 현 구현의 계약 충족으로 확대하지 않는다. 제품 테스트 파일은 변경하지 않았고 재현은 격리 복사본에서 수행했다. 수정 후 해당 반례를 회귀 시험에 포함해야 한다.

후속 수정 [EV-007](verification_evidence.md#ev-007): 정식 재현3개 Red 후 수정·통과, 추가 경계/allocator 포함 신규9개다. 완료 통합15·제어18·lib9, 두 Rust16 features/consumer/예제/clippy/rustdoc/fmt·RT allocation/deallocation0/0·Miri78×8 seeds Pass. Normal의 예약 소진/빈 cyclic hole/반복 wrap·소형/원본 예약 및 W<R/W=R, 지연 실패 feedback 이후 다음 입력 진행, 만료 shutdown의 유한 cleanup·늦은 raw write와 일반 sync 성공 보존·일반 request 만료 후 재요청을 검증했다. 수정분 리뷰/native·실장비는 잔여이며 TC-003 전체 Pass로 승격하지 않는다.

후속 수정 [EV-009](verification_evidence.md#ev-009): RV-005의 sync 열림 확인~게시 중 shutdown 경합을 두 Rust의 정식 반례2개로 먼저 실패시켰다. 진입 표시·열림 확인·close·정지 확인의 SeqCst 순서화/RAII 수정 후 게시 중 Closing/Incomplete 유지, 실제 IO 종료 뒤 성공 Closed, 만료 게시 뒤 실패 Closed 및 검증 오류 뒤 표시 해제를 private3개로 확인했다. 두 Rust 각16 features/consumer/예제/clippy/rustdoc/fmt·RT0/0·Miri lib12+raw17+routing19+completion15+control18=81×8(648) Pass다. source38개와 EV-007의 이전 수정 회귀·예산 exact-fit/반복 계측을 대조했으며 GAP-007 로컬 Closed다. 최종 Human 리뷰/native·실장비는 잔여다.

<a id="tc-004"></a>
### TC-004 — MCAP·시각·스키마·인덱스

- 관련: MS-003 / FR-002·003·013 / NFR-002·004·005
- 상태: TASK-007 하위 로컬 Pass, Human 리뷰/native·후속 active reader/OS 교환·장비 검증 잔여 / EV: [EV-010](verification_evidence.md#ev-010) · [EV-011](verification_evidence.md#ev-011)

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 소형 256 B·원본 8 MiB, u32 초과 receipt, wall clock 역행, 두 schema version | 재현 가능한 설정/입력·예산 기록 |
| 2 | 다중 파일 쓰기→chunk 마감→sync→독립 reader 읽기, unknown encoding·잘못된 길이 주입 | receipt/원본 정확 일치, mono/real/source 보존, CRC/인덱스 검증, unknown payload opaque 반환 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

chunk target 초과와 index 최대점유 계측. encoding 호환/구버전 golden 파일. 구현 선택 crate만의 roundtrip 외 독립 판독 포함.

TASK-007 초기 봉투·파일/CRC/index/golden·8MiB/시각·소유권/예산 검증은 [EV-010](verification_evidence.md#ev-010)에 보존한다. 리뷰 수정 후 [EV-011](verification_evidence.md#ev-011)의 저장 통합21개와 private Busy2개로 배치 단일 chunk·부분 수락/flush 실패·보고 재시도·active/finished SyncWork 반환·query permit 독립성·buffered short/Interrupted/부분 write 실패·Drop 무IO를 확인했다. 두 Rust 각16 features/2-record 실제 파일 consumer/예제·clippy/rustdoc/fmt·allocator7 Pass. Miri는 현재 core83×8=664이며 봉투2개는 변경되지 않은 EV-010 근거를 재사용한다. 파일 통합19개는 일반 로컬 실행이다. 버퍼 포함 peak/예약은256B 108926/236160,8MiB 8497278/17012864 bytes(각129 records/chunks),128채널4096레코드 sparse/dense peak2454992/611808 ≤ 예약5838400, 최종 잔여0이다. EV-011 source46개를 현재 기준으로 삼는다. TC 전체 완료로 승격하지 않는다.

<a id="tc-005"></a>
### TC-005 — 보존·회전·복구·디스크 장애

- 관련: MS-003 / FR-003·011 / NFR-001·003·005·007
- 상태: 부분 실행 — TASK-008 로컬 Pass, TASK-009 복구/장애 로컬 구현·native/장비 잔여 / 최근 실행: 2026-10-30 / EV: EV-012·EV-013·EV-014·EV-015

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 작은 disk quota·짧은 pre/post 창·겹친 event·reader pin; partial chunk/CRC 손상 파일 | 재현 가능한 설정/입력·예산 기록 |
| 2 | bytes/time/count/index 회전 각각 유발; ENOSPC/write/sync/rename 실패; 프로세스 중단 후 복구 | 보호 파일 무단 삭제 없음, 원본 보존·완전한 구간만 복구, 손상/미완료 범위 보고, RT 대기 없음 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

Linux/Windows/macOS 파일 공유/rename/durability 차이는 네이티브 실행. process kill은 실제 전원 차단과 다름.

TASK-008 하위 [EV-012](verification_evidence.md#ev-012): 보존 통합15개(실제 Unix 파일1개), 세 모드·bytes/time/count/index bytes/chunk 회전·overlap event/pre/post quota·reserve·pin bytes/deadline/실제 release 보호·file sync/rename/directory/delete 실패·multi-segment control/shutdown Pass. 두 Rust 각16 features/consumer/예제/clippy/rustdoc/fmt·allocator8, Miri97×8=776 Pass다. EV-012 source49개는 수정 전 이력이며 최신 REV-010 조치 근거는 EV-013이다. 새 FileStore 표준 directory sync는 Unix만 구현하며 unsupported는 거절한다. 실제 여유 probe/비Unix 어댑터·native/장비·중단/CRC 복구는 미검증이며 TC-005 전체 Pass가 아니다.

REV-010 조치 [EV-013](verification_evidence.md#ev-013): 보존 통합17개(신규2개), 회전 host probe 중복 제거와 다음 일반 append의 reserve 거절/재시도·독립 판독, 유실 pin의 만료 후 슬롯/삭제 보호 유지 계약을 검증한다. 두 Rust core/all-features workspace·consumer/예제·clippy/rustdoc/fmt·allocator8 및 보존 Miri16×8 범위다. pin 유실 위험 자체의 해소·리더 중단 후 안전 반환은 TASK-010 후속이다. TC-005 전체·native/장비 완료로 승격하지 않는다.

TASK-009 하위 [EV-014](verification_evidence.md#ev-014): 복구/장애19개(실제 Unix 파일1개), chunk 중간 절단·CRC/참조/봉투/길이 손상·손상 뒤 재동기화 금지·부분 chunk 미공개·opaque/full-width receipt/사용자 schema·index 재생성·원본 불변·IO/ENOSPC 원인 보존·file sync/rename/directory 순서/실패·취소/용량·다른 출력/수집 격리 Pass다. 두 Rust 각16 features/consumer/예제/lint/doc/fmt·allocator9, 확장 Miri117×8=936 Pass이며 source51개·명령/원본 로그는 EV-014로 고정했다. 과거 completion 응답 복원·자동 startup 발견/재개·worker/StorageDegraded·실제 전원/OS 장애 수락은 포함하지 않는다. 최신 REV-012 P3 조치는 [EV-015](verification_evidence.md#ev-015)로 재검증했다. 두 Rust workspace/all-features 각201개(복구19·allocator9 포함)·clippy/fmt Pass이며 현재 recovery.rs hash는 EV-015를 따른다. 이전 Miri/16-feature matrix/rustdoc은 EV-014 소스의 이력이다.

<a id="tc-006"></a>
### TC-006 — 조회·동시 쓰기·재생

- 관련: MS-004 / FR-005·010 / NFR-002·005
- 상태: 부분 실행 — TASK-010 조회/읽기 수명·TASK-011 local replay/진단 추가 조치 로컬 Pass, 실제 성능/native 후속 / 최근 실행: 2026-10-31 / EV: EV-016·017·018·019·020

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | active/closed 세그먼트, mono/real 역행·동일 시각 tie, 대용량 blob, controllable clock | 재현 가능한 설정/입력·예산 기록 |
| 2 | 기록 중 1,000건/256 KiB 페이지→cursor→만료; replay 0.5/1/2배속·pause·seek·느린 소비자 | safe offset 밖 비노출, 페이지 상한·결정적 tie-break, seek 뒤 old generation 제거, 제한 prefetch/핀 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

SourceTime domain 불명은 오류. 손상 index 재생성·범위 bounds. P5 응답 p95는 TC-012.

TASK-010 하위 [EV-016](verification_evidence.md#ev-016): query9·retention의 reader-owned close/error/cancel/expiry 및 active→closed pin 수명2개·독립 실제 파일 API consumer·두 Rust16조합·allocator10·Miri1024 로컬 Pass. 고정 complete offset·시간/채널/level 필터·결정적 정렬·page metadata+inline bytes/count·큰 원본 descriptor/범위·cursor/handle identity/만료·공유 외부 예약/누적 IO·잘못된 파일/CRC/한도를 확인했다. closed도 bounded scan으로 index를 재구성하며 summary 기반 scan 생략은 미구현이다. 재생/prefetch/generation·live/remote/CLI·P5 실제 지연/OS/장비는 이 실행으로 완료하지 않는다.

REV-014 후속 [EV-017](verification_evidence.md#ev-017): query10·retention19·두 Rust core/all-features workspace·독립 실제 파일 consumer·allocator10·lint/doc/fmt·영향 Miri224 Pass. 모든 필터 교차/이전 cursor 재개·새 동일 Query cursor·취소 복귀/만료 역행을 확인했다. 동일 Query order 재사용·inline/누적 읽기 안내를 반영했으며 실제 p95/IO 간섭·재생/native·장비는 미검증이다.

TASK-011 하위 [EV-018](verification_evidence.md#ev-018): 가상시계 replay10·query10/retention19, 두 Rust16features·실제 File replay consumer·allocator10·lint/doc/fmt·영향 Miri304 로컬 Pass. mono 순서/leading gap·빈 범위 End·0.5/1/2배속·pause/resume/seek/stop·generation/미전달 폐기·늦은 소비/변속·1건 prefetch/누적 read·blob IO/취소/만료·실제 reader close/pin 반환을 확인했다. 실제 timer/p95/IO 간섭/native/장비·remote/live/CLI는 미검증이다. 후속 [EV-020](verification_evidence.md#ev-020)은 배속 동등 비율·오류 진단 2개 추가(replay14), 두 Rust 회귀/consumer/allocator/lint/doc/fmt·Miri14 단일 seed Pass이며 REV-017 추가 조치는 2026-10-31 Human 승인·Closed다.


<a id="tc-007"></a>
### TC-007 — 원격·Rust API·CLI 사용자 경로

- 관련: MS-004·006 / FR-005·006·010·013 / NFR-002·004·007
- 상태: 부분 실행 — TASK-012/013 gRPC/native client/CLI·파일/취소/ACK 로컬 Pass, 실앱/native·성능 후속 / 최근 실행: 2026-10-31 / EV: EV-022~025

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | loopback server/client, 느린 구독자, 8 MiB 원본, 불량 조각/미인증 요청 | 재현 가능한 설정/입력·예산 기록 |
| 2 | query/subscribe/replay→ACK 전 disconnect→재연결; CLI JSON/exit code/Ctrl-C | 완전 record 확인 뒤 ACK, 불명·Gap·cursor 만료 명시, reassembly/출력 bytes 상한, 비밀값 비노출 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

네이티브 실제 소켓·TLS/권한, dot-probe 연동 예제 포함. 표시 렌더링 시간은 완료 지표와 구분.

TASK-012 하위 [EV-022](verification_evidence.md#ev-022): 원격 unit10/public3, 실제 loopback8MiB/32조각·CRC/소비 인계 ACK·Gap/Unknown·TLS/read-manage·version/auth/length/capacity/TTL/세대, File→Snapshot Query/ReadBlob3bytes·Replay/Completion 및 ACK/Unknown 모두에서 독립 파일Written을 확인했다. 두 Rust각16feature·독립core/remote consumer·RT allocator11/lint/doc/fmt·영향 Miri48 Pass. CLI JSON/exit/Ctrl-C·dot-probe/실제 관리 backend·LAN 정량 성능/전체RSS·다른 OS/native 공개 후는 미검증이며 TC 전체 완료로 표시하지 않는다.

TASK-012 후속 [EV-023](verification_evidence.md#ev-023): 연결 포화 자기 wake Red→10ms timer 대기 Green·permit 회수 뒤 재수락, 고정256B credential/길이 비교와 경계 입력, receiver gate 한도/이전 세대 handoff/연결별 예산 수명 회귀를 추가했다. 두 Rust각16feature·독립 consumer/allocator·lint/doc/fmt/proto·영향 Miri25 Pass. REV-018/019 Closed·위임에 따른 조치 검증 수락, CLI/실제 transport 재연결·실앱/LAN/RSS/native/성능은 잔여다.

TASK-013 착수 하위 [EV-024](verification_evidence.md#ev-024): ClientReceiver의 opaque 연결 identity·재연결 후 old fragment/ready handoff/ACK 차단·서버 ID 재사용·미인계 owner 실제 종료까지 예약 유지·generation/consumer 거절·invalid/foreign/중복 연결 거절 통합3개 Pass. 두 Rust workspace/default client/core consumer/lint/doc/fmt·Miri24 Pass, REV-020 첫 단위 자가 점검 Closed. 실제 transport 재연결 및 CLI JSON/exit/Ctrl-C·query→replay·dot-probe/native/성능은 미완료다.

TASK-013 현재 하위 [EV-025](verification_evidence.md#ev-025): native transport7·CLI process10·codec1·수신 수명3, 실제 8MiB32조각/CRC/인계 ACK·Gap/Unknown·TLS/권한·host 취소 후 실제 Command permit 수명·독립 파일 Query/ReadBlob→Replay start/pause/resume/seek/stop·offline query/timed replay·실제 Ctrl-C·stdout 실패 시 미ACK·출력 cap·복구/source bytes 불변을 확인했다. 두 Rust각16feature/workspace/core/default client/allocator·lint/doc/fmt/proto 두 사본·cached audit·Miri32 Pass, REV-021 RV-039~045 조치/로컬 수락 Closed다. SDK/CLI 예제·화면 API 계약은 제공했으며 실제 dot-probe 적용·다른 OS/native 공개 후T3·운영 backend/보안·LAN/RSS/PC/RT/UPS는 잔여다. 앞선 EV-022~024의 CLI/transport 미완료는 당시 이력이다. TC-007 전체 완료로 올리지 않는다.

<a id="tc-008"></a>
### TC-008 — DB 매핑·충돌·재시도

- 관련: MS-006 / FR-004 / NFR-003·005
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 선택 InfluxDB 실제 버전과 전용 namespace, 동일 series/timestamp 2개, timeout/부분 실패 서버 | 재현 가능한 설정/입력·예산 기록 |
| 2 | 수치·이벤트 write/query 대조→충돌/out-of-order→응답 전 단절/제한 retry | 무단 병합을 전량 성공으로 표시하지 않음, 파일 원본 유지, API ack/조회 확인/unknown 분리 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

원본 bytes를 DB로 몰래 전송하지 않음. retry 배치 불변·queue/TTL 상한. v3 호환은 별도 프로필.

<a id="tc-009"></a>
### TC-009 — 운영 리포트·변경·전후 비교

- 관련: MS-005 / FR-014·017 / NFR-006
- 상태: Partial — TASK-014/015 로컬 하위 Pass / 최근 실행: 2026-10-31 / EV: [EV-027](verification_evidence.md#ev-027)·[EV-028](verification_evidence.md#ev-028)·[EV-029](verification_evidence.md#ev-029). TASK-016/017·실측/실앱을 포함한 TC 전체는 미완료

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 검증 용량 없음/만료·계단 입력·counter reset·출력 정지/회복·샘플링 필터 | 재현 가능한 설정/입력·예산 기록 |
| 2 | 관측→필터/route patch→producer별 ack→before/after 비교 | 배출 뒤 카운터 정확 일치, observed/validated/estimated 분리, 손실 안정 상태는 Overloaded, 조건 차이 표시 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

5초 이상 안정 예측 구간 오차 ≤20%, 기본 계측 처리량 overhead ≤5%는 2026-10-26 승인된 시험 목표이며 달성은 미검증임.

TASK-014 하위 실행: 수량/대상·retired 합계·counter reset/누락·301ring/시각·포화·control 부족·동시 관측 TC-009 로컬6개와 추가 allocator2개·Miri56 Pass. 조건/측정경계·미실행은 EV-027을 따른다.

TASK-015 하위 실행 [EV-028](verification_evidence.md#ev-028): 리포트14개·실제 telemetry window 연결1개·관리 allocator1024회 할당/해제0·Miri8seeds×(리포트14+계측7)=168 Pass. profile 조건/세션/유효성·Stale/누락/포화·정수 범위·폐기 안정 큐·미측정 분모·음수/0 증가율·buffer bytes/slots/출력정지·filesystem/quota/순환 보존 분리·합성12조합 step simulation 오차≤20%(5초 이상)를 검증했다. 실제 PC/RT IO 오차·비용≤5%·물리 gauge 수집/운영 backend·실앱/native는 미완료다.

REV-025 하위 조치 [EV-029](verification_evidence.md#ev-029): TooLarge/Closing/Unsupported 오진·poison reset 실패 Red2개→Green. 제출/대여 범주·raw 재시도·용량 압력 유지·합산/차분/포화·reset, actual window0~11초와 target10 complete2/failed1/dropped1/unknown1/pending2 정렬, 추가 RT allocator 할당/해제0. 계측10/리포트15/telemetry unit3개·Miri8seeds×28=224, 두 Rust16feature/consumer/workspace/lint/doc/fmt 검사14개 Pass. host 정렬·가까운 시점 혼합·시도율/고유 입력 구분과 실제 성능 미검증은 유지한다. REV-025 Closed이며 TC-009 전체는 Partial이다.

<a id="tc-010"></a>
### TC-010 — 간편 점검·선택 추천

- 관련: MS-005 / FR-016 / NFR-005·006; FR-015는 제안
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 임시 전용 경로, 시간 10초·쓰기 32 MiB·메모리 16 MiB 예산, 수신자 부재 | 재현 가능한 설정/입력·예산 기록 |
| 2 | plan만 실행→run→예산 중 먼저 도달/취소/공간 reserve 위협 | plan에는 부하 없음, 중단 뒤 IO 정리·운영 파일 불변, 결과에 실제 범위·중단/불충분 이유 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

FR-015 승인 시만 unknown hardware/사용자 설정 보존/자동 적용 금지 검증. 수치는 단위 fixture이지 제품 기본값 아님.

<a id="tc-011"></a>
### TC-011 — UPS 옵션·기한·차단 시뮬레이션

- 관련: MS-006·007 / FR-018 / NFR-001·003
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | off 및 on 계획, rank 1/2·snapshot quota·최대 미동기화량, 전달 지연/안전여유 | 재현 가능한 설정/입력·예산 기록 |
| 2 | off 통지→on 모의 신호→중복/더 이른 기한→큰 IO 중 신호→모의 차단 기한/중단·재개/복구 | off는 전이/전용 예약 없음, deadline 연장 없음, 중요 순위/부분 결과, 모의 기한 전 실제 파일 sync/부분 결과·중단 후 외부 receipt oracle 대조(ADR-011 시뮬레이션 수락) |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

2026-10-31 Human 승인 ADR-011: 실물 UPS 없이 현재 PC의 시뮬레이션 결과로 수락한다. 모의 신호 원점→모의 차단 기한 전체 약3초·전달 지연·실제 파일 sync와 복구를 기록하며 물리 전원 차단 내구성의 관측 결과로 표시하지 않는다. 아직 시뮬레이션 실행 전 Not Run이다.

<a id="tc-012"></a>
### TC-012 — 플랫폼·실사용 성능·장시간 종합

- 관련: MS-007 / NFR-001~007 / 주요 FR 종단
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 승인된 OS/CPU/SSD/FS/NIC/제어 주기·P0 baseline, R1/P0~P6와 예산 512 MiB/2 GiB/4 GiB | 재현 가능한 설정/입력·예산 기록 |
| 2 | 현재 PC P1~P5 완전 경로 반복→P6 24시간/장애/UPS 시뮬레이션→공개 후 native/3 OS 파일 교환→dot-probe 조절 시나리오 | 요구 §12 수치·무손실 정상 부하·deadline miss 추가 0 목표·유한 자원·OS 기능 일치, 불충족별 Fail |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

P3 최소60초 및 RAM2배 이상, 마지막 sync까지 시간 포함. P4 receiver ACK, P5 P3동시. ADR-011로 실제 장치 시험은 현재 PC 실측으로 대체하고 native는 공개 완료 후 진행한다. 일반/RT는 부팅한 커널별로 기록하며 실행 안 한 조합은 미검증이다.

## 3. 요구사항–설계–구현–테스트 추적 체크리스트

각 행의 MS/TASK/TC 링크는 계획과 현재 근거를 구분한다. TASK-002는 Human 리뷰·로컬/native CI 완료(EV-001, 2026-10-27)로 Done이지만 해당 MS 전체 시나리오·환경이 남아 있으므로 MS 전체 체크는 유지한다. 요구별 여러 MS의 칸은 각각 체크해 부분 완료를 유지한다. FR-015는 제안 상태이며 Must에서 제외하되 행은 유지한다. ADR-008·009 및 상세 기준선 승인에 따라 설계 검토 칸만 체크했다. 제품 구현·테스트의 MS 전체 칸은 계속 미완료다.

| 요구사항 | 의도·AC | 상세설계 | 설계 검토 | 구현 체크 — MS/TASK·코드 근거 | 테스트 체크 — MS/TC·EV | 상태/공백 |
|---|---|---|---|---|---|---|
| INT-001 | 기록 활용과 운영 조절의 전체 가치 | [설계](../03_design_artifacts/domain_model.md#1-규모에-맞춘-아키텍처) | [x] 기준선 승인 | MS-001: 설계 완료([TASK-001](../04_implementation/tasks/TASK-001.md)); 제품 구현 해당 없음<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md)<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-005: [TC-009](#tc-009)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| FR-001 | 재사용 라이브러리/수명주기·US-001 AC1 | [설계](../03_design_artifacts/interface_contract.md#13-상세-api-설계안) | [x] 기준선 승인 | [ ] MS-002: [TASK-002](../04_implementation/tasks/TASK-002.md)(구성 코드 구현·Human 리뷰 승인), [TASK-006](../04_implementation/tasks/TASK-006.md)<br>[ ] MS-004: [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md)<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)(TASK-002 하위 [EV-001](verification_evidence.md) 로컬/native CI Pass; 전체 미완료)<br>[ ] MS-007: [TC-001](#tc-001)<br>[ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | TASK-002 Done·EV-001 / Human 리뷰·native CI Pass(2026-10-27); 후속 범위 잔여 ; TASK-006 구현·RV-001~005 수정 [EV-009](verification_evidence.md#ev-009) 로컬 Pass(Human 승인/native 대기); TASK-013 [client](../../crates/dot-logger-client/src/lib.rs)/[CLI](../../crates/dot-logger-cli/src/main.rs)·[EV-025](verification_evidence.md#ev-025) 로컬 구현/위임 수락 완료(native/실앱·성능 잔여, MS 전체 체크 유지) |
| FR-002 | 로그·수치·원본·US-001 AC2 | [설계](../03_design_artifacts/data_model.md#1-논리-스키마) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md), [TASK-004](../04_implementation/tasks/TASK-004.md)<br>[ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | TASK-003 EV-002 로컬/native Pass; TASK-004 원본 구현·[EV-003](verification_evidence.md#ev-003--task-004-원본-풀수명반환) 로컬 Pass(Human 리뷰 승인/native 대기); 후속 범위 잔여  ; TASK-007 [storage](../../crates/dot-logger/src/storage/mod.rs)·[EV-011](verification_evidence.md#ev-011) 로컬 Pass·Human 승인(native 대기); TASK-008 [EV-012](verification_evidence.md#ev-012)·[EV-013](verification_evidence.md#ev-013) 회전/보존·리뷰 조치 로컬 Pass(Human 구현 승인/native·실측 잔여) |
| FR-003 | 파일·완료·복구·US-002 AC2~4 | [설계](../03_design_artifacts/data_model.md#3-인덱스동시-읽기회전) | [x] 기준선 승인 | [ ] MS-002: [TASK-006](../04_implementation/tasks/TASK-006.md)<br>[ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md), [TASK-008](../04_implementation/tasks/TASK-008.md), [TASK-009](../04_implementation/tasks/TASK-009.md) | [ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-003: [TC-005](#tc-005) | 설계 승인 ; TASK-006 구현·RV-001~005 수정 [EV-009](verification_evidence.md#ev-009) 로컬 Pass(Human 승인/native 대기)  ; TASK-007 [storage](../../crates/dot-logger/src/storage/mod.rs)·[EV-011](verification_evidence.md#ev-011) 로컬 Pass·Human 승인(native 대기); TASK-008 [EV-012](verification_evidence.md#ev-012)·[EV-013](verification_evidence.md#ev-013) 회전/보존·리뷰 조치 로컬 Pass(Human 구현 승인/native·실측 잔여)  ; TASK-009 [recovery](../../crates/dot-logger/src/storage/recovery.rs)·[EV-014](verification_evidence.md#ev-014)·[EV-015](verification_evidence.md#ev-015) 복구/IO 장애·리뷰 조치 로컬 Pass·Human 승인(native·장비 잔여) |
| FR-004 | 시계열 DB·US-003 AC2~4 | [설계](../03_design_artifacts/data_model.md#5-db-projection과-장애-의미) | [x] 기준선 승인 | [ ] MS-006: [TASK-019](../04_implementation/tasks/TASK-019.md), [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-006: [TC-008](#tc-008) | 설계 승인·코드/EV 없음 |
| FR-005 | 기록 조회·US-003 AC2/5 | [설계](../03_design_artifacts/interface_contract.md#14-조회재생원격-계약안) | [x] 기준선 승인 | [ ] MS-004: [TASK-010](../04_implementation/tasks/TASK-010.md), [TASK-012](../04_implementation/tasks/TASK-012.md), [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-004: [TC-006](#tc-006)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음  ; TASK-010 [query](../../crates/dot-logger/src/query/mod.rs)·[EV-016](verification_evidence.md#ev-016) 조회/읽기 수명 로컬 Pass(Human 구현 승인/native·실측 잔여)  ; TASK-012 [remote](../../crates/dot-logger/src/adapters/remote/mod.rs)·[EV-022](verification_evidence.md#ev-022) gRPC/수신ACK/host bridge·예산·TLS 로컬 Pass(Human 리뷰/native·실앱·성능 후속); TASK-013 [client](../../crates/dot-logger-client/src/lib.rs)/[CLI](../../crates/dot-logger-cli/src/main.rs)·[EV-025](verification_evidence.md#ev-025) 로컬 구현/위임 수락 완료(native/실앱·성능 잔여, MS 전체 체크 유지) |
| FR-006 | Rust API+CLI·기존 화면 연동 | [설계](../03_design_artifacts/ui_spec.md#2-명령-후보와-결과) | [x] 기준선 승인 | [ ] MS-004: [TASK-012](../04_implementation/tasks/TASK-012.md), [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | 설계 승인·코드/EV 없음  ; TASK-012 [remote](../../crates/dot-logger/src/adapters/remote/mod.rs)·[EV-022](verification_evidence.md#ev-022) gRPC/수신ACK/host bridge·예산·TLS 로컬 Pass(Human 리뷰/native·실앱·성능 후속) ; TASK-013 [client 수신 수명](../../crates/dot-logger-client/src/receiver.rs)·[EV-024](verification_evidence.md#ev-024) 첫 단위 Pass(transport/CLI 후속); TASK-013 [client](../../crates/dot-logger-client/src/lib.rs)/[CLI](../../crates/dot-logger-cli/src/main.rs)·[EV-025](verification_evidence.md#ev-025) 로컬 구현/위임 수락 완료(native/실앱·성능 잔여, MS 전체 체크 유지) |
| FR-007 | 중요/동등 정책·US-001 AC3/4 | [설계](../03_design_artifacts/domain_model.md#4-수집소유권회수의-상세안) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md), [TASK-005](../04_implementation/tasks/TASK-005.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-002: [TC-003](#tc-003) | TASK-003 Done·[EV-002](verification_evidence.md#ev-002--task-003-소형-수집예약일반-대기) 로컬/native Pass / Human 리뷰 승인·후속 범위 잔여 ; TASK-005 구현·[EV-004](verification_evidence.md#ev-004--task-005-분배출력-격리예산) 로컬 Pass(Human 리뷰 승인/native 대기) |
| FR-008 | RT 무대기·일반 기한 대기 | [설계](../03_design_artifacts/interface_contract.md#13-상세-api-설계안) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md) | [ ] MS-002: [TC-002](#tc-002) | TASK-003 Done·[EV-002](verification_evidence.md#ev-002--task-003-소형-수집예약일반-대기) 로컬/native Pass / Human 리뷰 승인·후속 범위 잔여 |
| FR-009 | 별도 중요도/레벨 mapping | [설계](../03_design_artifacts/interface_contract.md#17-첫-검증-구성과-상세-리뷰-경계) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md)<br>[ ] MS-005: [TASK-016](../04_implementation/tasks/TASK-016.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-005: [TC-009](#tc-009) | TASK-003 Done·[EV-002](verification_evidence.md#ev-002--task-003-소형-수집예약일반-대기) 로컬/native Pass / Human 리뷰 승인·후속 범위 잔여 |
| FR-010 | 자체 시간 재생·US-003 AC1/5 | [설계](../03_design_artifacts/interface_contract.md#14-조회재생원격-계약안) | [x] 기준선 승인 | [ ] MS-004: [TASK-011](../04_implementation/tasks/TASK-011.md), [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-004: [TC-006](#tc-006)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | 설계 승인 ; TASK-011 [replay](../../crates/dot-logger/src/replay/mod.rs)·[EV-018](verification_evidence.md#ev-018)·[EV-019](verification_evidence.md#ev-019) 로컬 재생/리뷰 조치 Pass·EV-018/019 Human 승인, [EV-020](verification_evidence.md#ev-020) 추가 진단 조치 Human 승인/native·실측 잔여; TASK-013 [client](../../crates/dot-logger-client/src/lib.rs)/[CLI](../../crates/dot-logger-cli/src/main.rs)·[EV-025](verification_evidence.md#ev-025) 로컬 구현/위임 수락 완료(native/실앱·성능 잔여, MS 전체 체크 유지) |
| FR-011 | 연속/순환/이벤트·US-002 AC1 | [설계](../03_design_artifacts/data_model.md#4-보존과-복구) | [x] 기준선 승인 | [ ] MS-003: [TASK-008](../04_implementation/tasks/TASK-008.md) | [ ] MS-003: [TC-005](#tc-005) | TASK-008 [retention/catalog](../../crates/dot-logger/src/storage/retention.rs)·[EV-013](verification_evidence.md#ev-013) 리뷰 조치 로컬 Pass·Human 리뷰/native 대기 |
| FR-012 | 다중 생산자·US-001 AC1~3 | [설계](../03_design_artifacts/domain_model.md#3-실행-구조와-병목-경계) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md), [TASK-004](../04_implementation/tasks/TASK-004.md), [TASK-005](../04_implementation/tasks/TASK-005.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-007: [TC-012](#tc-012) | TASK-003 EV-002 로컬/native Pass; TASK-004 원본 구현·[EV-003](verification_evidence.md#ev-003--task-004-원본-풀수명반환) 로컬 Pass(Human 리뷰 승인/native 대기); 후속 범위 잔여 ; TASK-005 구현·[EV-004](verification_evidence.md#ev-004--task-005-분배출력-격리예산) 로컬 Pass(Human 리뷰 승인/native 대기) |
| FR-013 | 다중 파일/route·US-001 AC1 | [설계](../03_design_artifacts/domain_model.md#3-실행-구조와-병목-경계) | [x] 기준선 승인 | [ ] MS-002: [TASK-005](../04_implementation/tasks/TASK-005.md), [TASK-006](../04_implementation/tasks/TASK-006.md)<br>[ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md)<br>[ ] MS-004: [TASK-012](../04_implementation/tasks/TASK-012.md)<br>[ ] MS-005: [TASK-016](../04_implementation/tasks/TASK-016.md) | [ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-005: [TC-009](#tc-009) | 설계 승인 ; TASK-005 구현·[EV-004](verification_evidence.md#ev-004--task-005-분배출력-격리예산) 로컬 Pass(Human 리뷰 승인/native 대기) ; TASK-006 구현·RV-001~005 수정 [EV-009](verification_evidence.md#ev-009) 로컬 Pass(Human 승인/native 대기)  ; TASK-007 [storage](../../crates/dot-logger/src/storage/mod.rs)·[EV-011](verification_evidence.md#ev-011) 로컬 Pass·Human 승인(native 대기); TASK-008 [EV-012](verification_evidence.md#ev-012)·[EV-013](verification_evidence.md#ev-013) 회전/보존·리뷰 조치 로컬 Pass(Human 구현 승인/native·실측 잔여)  ; TASK-012 [remote](../../crates/dot-logger/src/adapters/remote/mod.rs)·[EV-022](verification_evidence.md#ev-022) gRPC/수신ACK/host bridge·예산·TLS 로컬 Pass(Human 리뷰/native·실앱·성능 후속) |
| FR-014 | 병목·여유·잔여시간·US-004 | [설계](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) | [x] 기준선 승인 | [ ] MS-005: [TASK-014](../04_implementation/tasks/TASK-014.md), [TASK-015](../04_implementation/tasks/TASK-015.md) | [ ] MS-005: [TC-009](#tc-009)<br>[ ] MS-007: [TC-012](#tc-012) | TASK-014 [telemetry](../../crates/dot-logger/src/telemetry/mod.rs)·[EV-027](verification_evidence.md#ev-027), TASK-015 [report](../../crates/dot-logger/src/telemetry/report.rs)·[EV-028](verification_evidence.md#ev-028) 로컬 Pass/위임 수락; MS-005 후속 Task·TC 전체·실측/실앱 미완료 ; REV-025 [EV-029](verification_evidence.md#ev-029) 거절 범주/시도 부하·host 끝점·이력 복구 조치 로컬 수락(sourceeaf736c), 전체 MS/TC는 미완료 |
| FR-015 | 제안: 질문 프리셋·설정 추천 | [설계](../03_design_artifacts/ui_spec.md#2-명령-후보와-결과) | [ ] 별도 제안 | [ ] MS-005: [TASK-018](../04_implementation/tasks/TASK-018.md) | [ ] MS-005: [TC-010](#tc-010) | 제안·범위 미승인 |
| FR-016 | 내장 간편 점검·US-004 | [설계](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) | [x] 기준선 승인 | [ ] MS-005: [TASK-017](../04_implementation/tasks/TASK-017.md) | [ ] MS-005: [TC-010](#tc-010)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| FR-017 | 개발자 조절·전후 비교·US-004 | [설계](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) | [x] 기준선 승인 | [ ] MS-005: [TASK-016](../04_implementation/tasks/TASK-016.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-005: [TC-009](#tc-009)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | 설계 승인·코드/EV 없음 ; REV-025 [EV-029](verification_evidence.md#ev-029) 거절 범주/시도 부하·host 끝점·이력 복구 조치 로컬 수락(sourceeaf736c), 전체 MS/TC는 미완료 |
| FR-018 | 기본 off·UPS 약3초·US-002 AC6~10 초안 | [설계](../03_design_artifacts/interface_contract.md#16-동기화긴급-동작의-구체안) | [x] 기준선 승인 | [ ] MS-006: [TASK-021](../04_implementation/tasks/TASK-021.md)<br>[ ] MS-007: [TASK-024](../04_implementation/tasks/TASK-024.md) | [ ] MS-006: [TC-011](#tc-011)<br>[ ] MS-007: [TC-011](#tc-011)<br>[ ] MS-007: [TC-012](#tc-012) | 추가 AC 승인·실물 미검증 |
| NFR-001 | RT 무대기·affinity·p99 진단 | [설계](../03_design_artifacts/domain_model.md#7-플랫폼과-보장의-범위) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md), [TASK-004](../04_implementation/tasks/TASK-004.md)<br>[ ] MS-003: [TASK-009](../04_implementation/tasks/TASK-009.md)<br>[ ] MS-006: [TASK-021](../04_implementation/tasks/TASK-021.md)<br>[ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-003: [TC-005](#tc-005)<br>[ ] MS-006: [TC-011](#tc-011)<br>[ ] MS-007: [TC-011](#tc-011)<br>[ ] MS-007: [TC-012](#tc-012) | TASK-003 EV-002 로컬/native Pass; TASK-004 원본 구현·[EV-003](verification_evidence.md#ev-003--task-004-원본-풀수명반환) 로컬 Pass(Human 리뷰 승인/native 대기); 후속 범위 잔여  ; TASK-009 [recovery](../../crates/dot-logger/src/storage/recovery.rs)·[EV-014](verification_evidence.md#ev-014)·[EV-015](verification_evidence.md#ev-015) 복구/IO 장애·리뷰 조치 로컬 Pass·Human 승인(native·장비 잔여) |
| NFR-002 | RAM 이후 활용 완료·P0~P5 | [설계](../03_design_artifacts/interface_contract.md#14-조회재생원격-계약안) | [x] 기준선 승인 | [ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md)<br>[ ] MS-004: [TASK-010](../04_implementation/tasks/TASK-010.md), [TASK-012](../04_implementation/tasks/TASK-012.md)<br>[ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-004: [TC-006](#tc-006)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음  ; TASK-007 [storage](../../crates/dot-logger/src/storage/mod.rs)·[EV-011](verification_evidence.md#ev-011) 로컬 Pass·Human 승인(native 대기); TASK-008 [EV-012](verification_evidence.md#ev-012)·[EV-013](verification_evidence.md#ev-013) 회전/보존·리뷰 조치 로컬 Pass(Human 구현 승인/native·실측 잔여)  ; TASK-010 [query](../../crates/dot-logger/src/query/mod.rs)·[EV-016](verification_evidence.md#ev-016) 조회/읽기 수명 로컬 Pass(Human 구현 승인/native·실측 잔여)  ; TASK-012 [remote](../../crates/dot-logger/src/adapters/remote/mod.rs)·[EV-022](verification_evidence.md#ev-022) gRPC/수신ACK/host bridge·예산·TLS 로컬 Pass(Human 리뷰/native·실앱·성능 후속) |
| NFR-003 | 24h·완료/복구·장애 | [설계](../03_design_artifacts/data_model.md#4-보존과-복구) | [x] 기준선 승인 | [ ] MS-002: [TASK-006](../04_implementation/tasks/TASK-006.md)<br>[ ] MS-003: [TASK-008](../04_implementation/tasks/TASK-008.md), [TASK-009](../04_implementation/tasks/TASK-009.md)<br>[ ] MS-006: [TASK-019](../04_implementation/tasks/TASK-019.md), [TASK-021](../04_implementation/tasks/TASK-021.md)<br>[ ] MS-007: [TASK-024](../04_implementation/tasks/TASK-024.md) | [ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-003: [TC-005](#tc-005)<br>[ ] MS-006: [TC-008](#tc-008)<br>[ ] MS-006: [TC-011](#tc-011)<br>[ ] MS-007: [TC-011](#tc-011)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인 ; TASK-006 구현·RV-001~005 수정 [EV-009](verification_evidence.md#ev-009) 로컬 Pass(Human 승인/native 대기); TASK-008 [EV-012](verification_evidence.md#ev-012)·[EV-013](verification_evidence.md#ev-013) 회전/보존·리뷰 조치 로컬 Pass(Human 구현 승인/native·실측 잔여)  ; TASK-009 [recovery](../../crates/dot-logger/src/storage/recovery.rs)·[EV-014](verification_evidence.md#ev-014)·[EV-015](verification_evidence.md#ev-015) 복구/IO 장애·리뷰 조치 로컬 Pass·Human 승인(native·장비 잔여) |
| NFR-004 | Rust 재사용·버전·호환 | [설계](../03_design_artifacts/domain_model.md#2-패키지와-의존-방향) | [x] 기준선 승인 | [ ] MS-002: [TASK-002](../04_implementation/tasks/TASK-002.md)(구성 코드 구현·Human 리뷰 승인)<br>[ ] MS-004: [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md)<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)(TASK-002 하위 [EV-001](verification_evidence.md) 로컬/native CI Pass; 전체 미완료)<br>[ ] MS-007: [TC-001](#tc-001)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | TASK-002 Done·EV-001 / Human 리뷰·native CI Pass(2026-10-27); 후속 범위 잔여  ; TASK-012 [remote](../../crates/dot-logger/src/adapters/remote/mod.rs)·[EV-022](verification_evidence.md#ev-022) gRPC/수신ACK/host bridge·예산·TLS 로컬 Pass(Human 리뷰/native·실앱·성능 후속); TASK-013 [client](../../crates/dot-logger-client/src/lib.rs)/[CLI](../../crates/dot-logger-cli/src/main.rs)·[EV-025](verification_evidence.md#ev-025) 로컬 구현/위임 수락 완료(native/실앱·성능 잔여, MS 전체 체크 유지) |
| NFR-005 | bytes/slots/인덱스/작업 예산 | [설계](../03_design_artifacts/domain_model.md#5-자원-예산과-설정) | [x] 기준선 승인 | [ ] MS-002: [TASK-004](../04_implementation/tasks/TASK-004.md), [TASK-005](../04_implementation/tasks/TASK-005.md)<br>[ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md), [TASK-008](../04_implementation/tasks/TASK-008.md)<br>[ ] MS-004: [TASK-010](../04_implementation/tasks/TASK-010.md), [TASK-011](../04_implementation/tasks/TASK-011.md)<br>[ ] MS-005: [TASK-015](../04_implementation/tasks/TASK-015.md), [TASK-017](../04_implementation/tasks/TASK-017.md)<br>[ ] MS-007: [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-003: [TC-005](#tc-005)<br>[ ] MS-004: [TC-006](#tc-006)<br>[ ] MS-005: [TC-010](#tc-010)<br>[ ] MS-007: [TC-012](#tc-012) | TASK-004 원본 구현·[EV-003](verification_evidence.md#ev-003--task-004-원본-풀수명반환) 로컬 Pass(Human 리뷰 승인/native 대기); 후속 범위 잔여 ; TASK-005 구현·[EV-004](verification_evidence.md#ev-004--task-005-분배출력-격리예산) 로컬 Pass(Human 리뷰 승인/native 대기)  ; TASK-007 [storage](../../crates/dot-logger/src/storage/mod.rs)·[EV-011](verification_evidence.md#ev-011) 로컬 Pass·Human 승인(native 대기); TASK-008 [EV-012](verification_evidence.md#ev-012)·[EV-013](verification_evidence.md#ev-013) 회전/보존·리뷰 조치 로컬 Pass(Human 구현 승인/native·실측 잔여)  ; TASK-010 [query](../../crates/dot-logger/src/query/mod.rs)·[EV-016](verification_evidence.md#ev-016) 조회/읽기 수명 로컬 Pass(Human 구현 승인/native·실측 잔여) ; TASK-011 [replay](../../crates/dot-logger/src/replay/mod.rs)·[EV-018](verification_evidence.md#ev-018)·[EV-019](verification_evidence.md#ev-019) 1건 prefetch/기존 external 예약 공유·잔여0 로컬 Pass·Human 승인/native·장비 잔여  ; TASK-012 [remote](../../crates/dot-logger/src/adapters/remote/mod.rs)·[EV-022](verification_evidence.md#ev-022) gRPC/수신ACK/host bridge·예산·TLS 로컬 Pass(Human 리뷰/native·실앱·성능 후속)  TASK-015 [EV-028](verification_evidence.md#ev-028) bytes/slots·출력정지/디스크 quota 한계 산술 로컬 Pass; 실제 자원/PC 수락 잔여 |
| NFR-006 | 관측/검증/추정·계측 비용 | [설계](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) | [x] 기준선 승인 | [ ] MS-005: [TASK-014](../04_implementation/tasks/TASK-014.md), [TASK-015](../04_implementation/tasks/TASK-015.md), [TASK-016](../04_implementation/tasks/TASK-016.md), [TASK-017](../04_implementation/tasks/TASK-017.md)<br>[ ] MS-007: [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-005: [TC-009](#tc-009)<br>[ ] MS-005: [TC-010](#tc-010)<br>[ ] MS-007: [TC-012](#tc-012) | TASK-014 [telemetry](../../crates/dot-logger/src/telemetry/mod.rs)·[EV-027](verification_evidence.md#ev-027), TASK-015 [report](../../crates/dot-logger/src/telemetry/report.rs)·[EV-028](verification_evidence.md#ev-028) 로컬 Pass/위임 수락; MS-005 후속 Task·TC 전체·실측/실앱 미완료 ; REV-025 [EV-029](verification_evidence.md#ev-029) 거절 범주/시도 부하·host 끝점·이력 복구 조치 로컬 수락(sourceeaf736c), 전체 MS/TC는 미완료 |
| NFR-007 | 3 OS·Tier별 검증·US-005 | [설계](../03_design_artifacts/domain_model.md#7-플랫폼과-보장의-범위) | [x] 기준선 승인 | [ ] MS-002: [TASK-002](../04_implementation/tasks/TASK-002.md)(구성 코드 구현·Human 리뷰 승인)<br>[ ] MS-003: [TASK-009](../04_implementation/tasks/TASK-009.md)<br>[ ] MS-004: [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-024](../04_implementation/tasks/TASK-024.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)(TASK-002 하위 [EV-001](verification_evidence.md) 로컬/native CI Pass; 전체 미완료)<br>[ ] MS-007: [TC-001](#tc-001)<br>[ ] MS-003: [TC-005](#tc-005)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-007: [TC-012](#tc-012) | TASK-002 Done·EV-001 / Human 리뷰·native CI Pass(2026-10-27); 후속 범위 잔여  ; TASK-009 [recovery](../../crates/dot-logger/src/storage/recovery.rs)·[EV-014](verification_evidence.md#ev-014)·[EV-015](verification_evidence.md#ev-015) 복구/IO 장애·리뷰 조치 로컬 Pass·Human 승인(native·장비 잔여); TASK-013 [client](../../crates/dot-logger-client/src/lib.rs)/[CLI](../../crates/dot-logger-cli/src/main.rs)·[EV-025](verification_evidence.md#ev-025) 로컬 구현/위임 수락 완료(native/실앱·성능 잔여, MS 전체 체크 유지) |
| CON-001 | Rust | [설계](../03_design_artifacts/tech_stack.md) | [x] 기준선 승인 | [ ] MS-002: [TASK-002](../04_implementation/tasks/TASK-002.md)(구성 코드 구현·Human 리뷰 승인) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)(TASK-002 하위 [EV-001](verification_evidence.md) 로컬/native CI Pass; 전체 미완료)<br>[ ] MS-007: [TC-001](#tc-001) | TASK-002 Done·EV-001 / Human 리뷰·native CI Pass(2026-10-27); 후속 범위 잔여 |
| CON-002 | Linux/macOS/Windows·Linux RT | [설계](../03_design_artifacts/domain_model.md#7-플랫폼과-보장의-범위) | [x] 기준선 승인 | [ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-024](../04_implementation/tasks/TASK-024.md) | [ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| CON-003 | dot-probe 첫 적용 | [설계](../03_design_artifacts/ui_spec.md#1-사용자-흐름) | [x] 기준선 승인 | [ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md)<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | [ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| CON-004 | 공식 조사·기록 | [설계](../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md) | [x] 기준선 승인 | MS-001: 설계 완료([TASK-001](../04_implementation/tasks/TASK-001.md)); 제품 구현 해당 없음 | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001) | 설계 승인·코드/EV 없음 |
| CON-005 | 조사 후 한 번에 한 질문 | [설계](../03_design_artifacts/domain_model.md#9-설계에서-남은-결정) | [x] 기준선 승인 | MS-001: 설계 완료([TASK-001](../04_implementation/tasks/TASK-001.md)); 제품 구현 해당 없음 | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001) | 설계 승인·코드/EV 없음 |
| CON-006 | ROS2는 첫 적용 후 T1 | [설계](../03_design_artifacts/domain_model.md#9-설계에서-남은-결정) | [x] 기준선 승인 | MS-001: 설계 완료([TASK-001](../04_implementation/tasks/TASK-001.md)); 제품 구현 해당 없음<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001) | 설계 승인·코드/EV 없음 |
| CON-007 | SSD·장비별 저장 예산 | [설계](../03_design_artifacts/domain_model.md#5-자원-예산과-설정) | [x] 기준선 승인 | [ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |

## 4. MS 인수 시 갱신 순서

1. Task 상세와 registry 상태를 코드·리뷰 결과에 맞춘다.
2. TC 하위 결과에 revision·환경·명령·기대/실제·결과 파일을 기록하고 verification_evidence에 EV를 등록한다.
3. 위 요구 행에서 해당 MS의 구현/테스트 칸만 체크한다. 미실행 환경·장비는 GAP로 연결한다.
4. MS Done은 관련 필수 요구의 해당 범위와 Human 리뷰가 완료된 때만 표시한다. Gate 4/5는 별도 판정한다.

최근 제품 실행 결과: TASK-002 구성/API·의존성 로컬/native CI Pass(EV-001). 나머지 TC·runtime·실장비는 미실행. 문서의 ID/링크 검사는 제품 테스트 통과 증거가 아니다.

## 2026-10-31 프로필 준비·구현 승인 반영

TASK-008/010/011 최종 구현 리뷰 승인으로 REV-009/010/013/014/017 Closed. TC 전체 및 MS 체크는 로컬 하위 검증/미실행 경계를 유지한다. [EV-021](verification_evidence.md#ev-021)은 TASK-022 local-pc-v1 Draft의 선언 검토/준비 점검만 수행했다. TC-012/011 제품 성능·RT·UPS 시뮬레이션 실행은 Not Run이다. 수치 프로필은 [TASK-022](../04_implementation/tasks/TASK-022.md#현재-pc-수치-프로필-초안--2026-10-31)에서10ms 조정 뒤 Human 승인으로 확정했다.

2026-10-31 local-pc-v1 수치 프로필 Approved. TC-011/012 제품 실행은 NotRun, [EV-021 승인 점검](verification_evidence.md#ev-021)은 선언 유효성/준비 잔여만 기록한다.

# TASK-011 — 재생 시간과 취소 제어

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-004 |
| Story | US-003·005 |
| 요구사항 | FR-010 NFR-005 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | Done |
| 마지막 갱신일 | 2026-11-13 |
| 다음 액션 | 없음 — Done(2026-11-13 완료 기준 변경). 잔여는 아래 「완료 판정」 절의 소유처로 이관 |
| 의존 | TASK-010 |
| 예상 소요 | 1~2 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: [`replay/mod.rs`](../../../crates/dot-logger/src/replay/mod.rs)·[`tc_006_replay.rs`](../../../crates/dot-logger/tests/tc_006_replay.rs), query 내부의 1건 page/기존 예약 버퍼 연동.

첫 실패/리뷰 조건: **seek 뒤 이전 generation 미전송·pause/resume·배속·늦은 소비자**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-006`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] 가상시계로 deterministic 순서·실제 파일 replay·결측·bounded prefetch 검증.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 리뷰 완료 — EV-018/019의 2026-10-31 최종 구현 승인 유지.
- [x] REV-017/EV-020 추가 조치 Human 리뷰 — 2026-10-31 승인. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-006](../../05_verification/test_case.md#tc-006). 제품 코드·테스트 실행 결과: [EV-018](../../05_verification/verification_evidence.md#ev-018). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 착수 — 2026-10-30

Human 「커밋 푸시하고 다음 작업 시작하자」에 따라 TASK-010 소스439d9c5/문서35e3ac3 게시 후 착수한다. TASK-010/008 Human 최종 리뷰·native/장비 GAP은 별도 유지한다. 승인된 §14의 동기식 local replay를 가상 mono ns 시계·양수 유리수 배속으로 구현한다. Snapshot을 소유하고 기존 예약 버퍼에 1건만 prefetch하며 큰 payload는 같은 reader의 bounded blob read를 사용한다. start/pause/resume/seek/stop·generation 변경·취소/만료의 실제 reader 종료를 검증한다. remote 전송/worker/live 정책은 후속 TASK-012다.

## 구현 결과 — 2026-10-30

- `Replay`가 원 Snapshot을 소유하고 일반 동기식 `poll(now_ns, consumer_ready, cancel)`로 최대1건만 prefetch한다. 기존 query의 page/order/blob 사전 예약 버퍼를 공유하며 RT·dependency/lock·unsafe는 변경하지 않는다.
- `Speed`는 양수 u32 유리수, mono native ns·checked u128 산술/ceil deadline을 사용한다. pause/resume·배속 변경·host 시각 역행 clamp·late 소비자의 후속 간격 보존, generation checked 증가와 잘못된 seek의 상태 보존을 제공한다. 제어 전환은 정수 media ns로 내림한다.
- frame borrow가 동기식 전달 중 seek/stop을 막고 Transition은 미전달 prefetch 폐기만 보고한다. 이미 반환/복사된 외부 전송 취소는 adapter 책임이며 원격은 TASK-012다. snapshot identity/원 blob 범위·개별/누적 read 예산을 유지하고 이전 seek generation을 거절한다.
- End는 빈 범위/선택 종료를 나타내며 누락 건수를 추정하지 않는다. 마지막 blob read를 위해 snapshot을 보유하고 stop/Drop·오류/취소/만료에서 실제 IO 반환·reader close 뒤에만 pin과 예약을 반환한다.
- 초기 API Red 뒤 행동4개 Green, RV-024/025 반례 Red2개 뒤 수정했다. 가상시계10개와 query/pin·실제 파일 public consumer·allocator256B/8MiB 경로를 검증한다. 최종 EV-018/Miri 및 REV-015 상태는 검증 근거를 따른다.

동일 승인 범위의 동기식 재생·시간 제어 세부 구현으로 새 ADR은 만들지 않았다. source54개·두 Rust16features/allocator10/lint/doc/fmt, 영향 Miri를 확인한다. 실제 timer/OS/성능·native GAP-004·장비·Human 최종 리뷰는 별도다. 신규 TASK-011 변경은 미커밋·미게시이며 TASK-012를 자동 착수하지 않는다.

## 최종 로컬 검증

EV-018 source54개/검사10개 exit0, 두 Rust 각16features·독립 실제 File replay API·예제/allocator10·clippy/rustdoc/fmt·영향 Miri38×8=304 Pass다. query/replay256B/8MiB peak11143/25176199 ≤ 예약110776/67217592B·잔여0, 기존 RT 할당/해제0/0 유지. REV-015 RV-024/025 Resolved·Human 리뷰/native GAP-004·장비·실제 timer/성능 대기이며 신규 소스/문서는 미커밋·미게시다.

## 세션 마감·게시 — 2026-10-30

Human 「커밋, 푸쉬해주고 세션마무리」에 따라 EV-018 source54개·검사10개 exit0·로그19개의 원본/뷰 hash 일치를 확인했고, 동일 성공 시험은 반복하지 않았다. 소스 `026999f`를 커밋·origin/main 게시했으며 모든 source commit blob hash도 일치한다. [커밋 대조](../../06_evolution/imported_context/task_011_2026-10-30/commit.json)·[소스 게시 근거](../../06_evolution/imported_context/task_011_2026-10-30/push.json). 검증/마감 문서는 별도 커밋·[skip ci]로 게시한다. 위 미커밋/미게시 문장은 검증 당시 이력이다.

공유/개인 상태와 세션 로그를 종료로 동기화했다. 다음 시작점은 TASK-011·REV-015·EV-018의 Human 최종 구현 리뷰다. 커밋/게시 요청을 별도 최종 구현 승인으로 해석하지 않아 REV-015 Resolved·TASK-011/MS-004 In Progress·native GAP-004/장비·Gate4/5 Not Ready를 유지하며 TASK-012를 자동 착수하지 않는다.

## 리뷰 반입·처리 및 요약 — 2026-10-31

- 현재 기능: frozen Snapshot 기반 동기식 mono 재생, 양수 유리수 배속, pause/resume/seek/stop, generation, 최대1건 prefetch, bounded blob read와 실제 reader close 뒤 예약 반환이다. 늦은 소비자의 후속 간격과 기존 RT 수집 경계를 유지한다.
- REV-016 Open→In Progress 반입 후 RV-026은 현 불변식상 overflow 불가로 판정하되 방어적 terminal 정리를 수용했고, RV-027 Speed 접근자/Display/REALTIME·RV-028 세 반환 타입 Debug를 추가했다. 공개 진단 API compile Red→Green, host 상한 경계 시험은 수정 전 Pass로 구분했다.
- [EV-019](../../05_verification/verification_evidence.md#ev-019): 두 Rust workspace/all-features·독립 실제 파일 consumer·allocator10·clippy/rustdoc/fmt, replay12×8=96 Miri Pass. 소스2파일 변경·source54개 hash 고정·검사10개 exit0다. 기존 16feature 전 조합/Miri304는 EV-018 이력이며 이번 영향 검증과 구분한다.
- REV-015/016 Resolved·Human 최종 리뷰/native GAP-004·장비/실제 timer 성능·remote/live/CLI는 잔여다. 신규 변경은 미커밋·미게시이며 TASK-011/MS-004 In Progress, TASK-012 자동 착수 없음.

## Human 최종 구현 승인 — 2026-10-31

구현 계약 확인(배속·pause/resume·seek/stop·세대·late 간격·prefetch/읽기 예산·reader 수명) 보고 후 Human 「승인」을 TASK-011 최종 구현 승인으로 반영했다. 승인 대상은 EV-018 구현 및 EV-019/source54개 hash로 고정된 현재 리뷰 조치 작업 트리다. REV-015/016 및 RV-024~028을 Closed로 마감한다. 이전 본문의 Resolved/Human 대기 표기는 승인 전 이력이다.

native GAP-004·다른 OS/장비·실제 timer/성능 검증은 잔여이며 TASK-011/MS-004 In Progress·Gate4/5 Not Ready를 유지한다. 소스 hash가 EV-019와 같아 성공 시험은 반복하지 않았다. 이번 승인은 구현 수락이며 커밋/푸시·TASK-012 착수는 수행하지 않았다.

## 검증 환경·시점 변경 — 2026-10-31

Human 승인 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 실제 장치 시험은 TASK-022의 현재 PC 실측으로 대체 수락하며 UPS 관련 판정은 시뮬레이션을 사용한다. native GAP-004는 GitHub 공개 완료 후(T3) 진행하는 승인된 이월이다. 별도 장치/실물 UPS 확보를 대기 조건으로 요구하지 않는다. 현재 일반 커널 실행/RT 커널 설치를 구분하며 RT 실측은 해당 커널 부팅 확인 후 기록한다. 기존 로컬 Pass·Human 승인/미승인과 미실행 범위는 보존하고 이번 정책 변경만으로 Task를 Done 처리하지 않는다.

## 세션 마감·게시 — 2026-10-31

Human 「커밋 푸쉬 후 세션 마무리해줘」에 따라 리뷰 조치 소스 `9660d7d`를 origin/main에 게시하고 원격 hash 일치를 확인했다. EV-019 source54개·검사10개 exit0·Miri96·원본/뷰 로그12쌍 hash와 source commit blob54개가 일치해 동일 성공 시험을 반복하지 않았다. [커밋 대조](../../06_evolution/imported_context/task_011_review_fixes_2026-10-31/commit.json)·[소스 게시 근거](../../06_evolution/imported_context/task_011_review_fixes_2026-10-31/push.json). 이전 미커밋/미게시 문장은 검증 당시 이력이다.

TASK-011 Human 승인·REV-015/016 Closed, ADR-011 Accepted를 유지한다. 현재 PC 실측/UPS 시뮬레이션을 수락하며 native는 공개 완료 후(T3) 진행한다. 다음 시작점은 TASK-022의 실행 커널 확인·P0/제어 부하/기한/경로/예산 프로필 준비다. 현재 generic 실행/RT 설치를 구분하며 재부팅·부트 변경은 수행하지 않았다. TASK-008/010 Human 리뷰·PC 실측/UPS/24h 미실행·Gate4/5 Not Ready는 잔여다. 검증/수락/마감 문서는 별도 docs(cowork)·[skip ci] 커밋으로 게시하며 세션 종료 후 새 Task를 자동 착수하지 않는다.

## REV-017 반입·추가 조치 — 2026-10-31

RV-029는 MSRV1.85 보존을 위해 query 예약 함수에만 deprecated 경고 억제, RV-030은 성분 Eq 유지·`Speed::is_equivalent` 비율 비교, RV-031은 variant별 Display 및 원 Query/IO 진단 보존으로 조치했다. 기존 배속 제어/anchor·소유권·오류 분류/terminal 정리·RT/unsafe·의존성은 유지한다. [REV-017](../../05_verification/reviews/REV-017_TASK-011_FINAL_REVIEW.md)의 처리담당 판정과 [EV-020](../../05_verification/verification_evidence.md#ev-020)에 두 Rust 검증10개·Miri14 단일 seed·source54개 hash를 기록했다.

REV-017 Resolved·이번 추가 조치 Human 승인 대기, 기존 EV-018/019 승인·REV-015/016 Closed는 유지한다. 미커밋·미게시이며 TASK-008/010 리뷰·PC 성능/실제 timer·UPS 시뮬레이션·native 공개 후(T3) 잔여, TASK-011/MS-004 In Progress·Gate4/5 Not Ready다. TASK-012 자동 착수 없음.

## 후속 Human 승인 — 2026-10-31

Human 「승인, 다음작업 진행하자」로 EV-020의 구현/조치 최종 리뷰를 승인했다. 관련 리뷰 Closed·Human 리뷰 체크 완료. 앞선 승인 대기 본문은 이력이며 현재 PC 실측/UPS 시뮬레이션·native 공개 후(T3) 잔여로 In Progress를 유지한다. 다음 작업은 TASK-022 프로필 구체화다.

## 완료 판정 — 2026-11-13

[완료 기준 변경](../../05_verification/quality_gate.md#2026-11-13-taskms-완료-기준-변경--human-결정)에 따라 **Done**으로 닫는다. DoD 범위의 시험과 로컬 회귀, Human 리뷰(또는 위임 수락)는 이 문서의 EV·REV 기록으로 끝났다. 1.96.0·1.85.0 전체 회귀와 커버리지 기준선은 [EV-078](../../05_verification/verification_evidence.md#ev-078)이다. 아래 잔여는 소유처로 옮긴다. 미실행 상태는 그대로이며 Pass로 바꾸지 않는다.

- native·다른 OS: [TASK-024](TASK-024.md)(T3, GAP-004)
- 이 PC 정량 성능·RT: [TASK-023](TASK-023.md)에서 수행한 범위(EV-064~070)

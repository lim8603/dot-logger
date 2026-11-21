# [REV-015] TASK-011 재생 제어 개발 자가 리뷰

## 개요

- 일자/리뷰어: 2026-10-30 / AI, [code-review 스킬](../../../.agents/skills/code-review/SKILL.md).
- 대상: TASK-010 소스1972649·문서f92dd17 게시 후 작업 트리의 replay/query/lib·통합/pin/consumer/allocator·CI recipe.
- 판정: Conditional — 초기 미구현 API Red 후 구현, 재생 경계의 RV-024/025 행동 Red→Green, 최종 EV-018 로컬 Pass·Human 리뷰/native·장비 대기.
- 상태: Closed (EV-018/019 로컬 Pass·2026-10-31 Human 최종 구현 승인). 리뷰 전용 세션이 아닌 승인된 개발 세션의 자가 리뷰다.

| ID | 중요도 | 위치·문제 | 처리 | 상태 |
|---|---|---|---|---|
| RV-024 | P2 | blocked 상태에서 set_speed/pause가 media anchor를 미전달 record보다 앞으로 옮기면 뒤 레코드 due가 현재 시각으로 뭉침 | 전달 시 media anchor가 해당 record를 지났으면 실제 전달 시각/원 mono로 재기준화 | Closed |
| RV-025 | P2 | read_blob IO 실패 뒤 Playing/예약을 유지해 terminal 오류 계약과 다름 | bounded IO 반환을 확인한 뒤 pending/cursor·snapshot을 Drop하여 reader close와 pin/예약 반환 | Closed |

## RV-024

### [리뷰어]

10/20/30ns 원본의 첫 record가 blocked인 상태에서 host40ns에 2배속으로 바꾸면 media가40으로 옮겨져 두 번째 record도 즉시 나왔다. offline 소비자가 늦을 때 재생 시계를 늦춰야 하는 §14와 다르다. 중요도 P2.

### [처리담당]

- 수용(A — 승인된 late consumer 스케줄 계약 정렬). `speed_change_while_overdue_does_not_collapse_future_record_spacing`에서 첫 전달 후 두 번째 due45 기대가 실패하는 Red를 확인했다.
- `Replay::poll`이 media anchor가 미전달 mono를 지난 경우 실제 전달 시각/원 mono로 재기준화한다. 일반 경로는 기존 anchor를 late만큼만 밀어 절대 deadline의 ceil 반올림 누적을 피한다.
- Red: `red-spacing-blob-cleanup.log`, Green 및 최종 근거 EV-018.

## RV-025

### [리뷰어]

주입한 blob reader IO 오류 후 상태가 Playing이며 snapshot reservation이 Replay Drop까지 남는다. query 오류의 terminal stop·reader 종료 확인 뒤 반환 계약과 다르다. 조기 해제/메모리 비안전성은 아니며 P2.

### [처리담당]

- 수용(A — 동기식 IO 오류/자원 수명 계약의 구현 세부). `failed_blob_io_is_terminal_and_releases_the_owned_snapshot`에서 IO 후 Stopped/used0 기대가 실패하는 Red를 확인했다.
- `Replay::read_blob`이 IO 결과를 먼저 길이 없는 성공/오류로 회수하여 borrow를 끝내고, 오류는 `fail`→`stop`으로 실제 reader를 닫는다. 성공 시 검증된 blob scratch만 다시 borrow한다. pending/cursor를 snapshot보다 먼저 버리는 Drop도 명시했다.
- Red: `red-spacing-blob-cleanup.log`, Green 및 최종 근거 EV-018. retention의 custom Reader Drop 중에도 PinLimit임을 확인하고 close 후만 새 pin을 얻는 기존 통합 시험에 replay 성공/IO/취소/expiry 경로를 추가했다.

## 자가 점검

신규 unsafe/RT 경로 변경0, 외부 runtime/wire 타입 노출0. 체크된 generation/스케줄 산술·동일 query 정렬/기존 page 계약·prefetch1건·동일 shared external 예약·실제 reader close 순서와 취소/만료 단조성을 검토했다. 두 Rust 각16features·실제 파일 consumer·allocator256B/8MiB·lint/doc/fmt·영향 Miri304는 EV-018 로컬 Pass다. 실제 timer/OS 성능·remote in-flight/CLI는 범위 밖이며 Gate4/5 완료를 주장하지 않는다.

## 최종 검증

[EV-018](../verification_evidence.md#ev-018)·source54개/검사10개·로그 hash(비공개 원시 근거: `imported_context/task_011_2026-10-30/results.json`)가 일치한다. Miri query10+replay10+retention18×8=304 Pass, allocator10/RT0/0 유지, query/replay256B/8MiB peak11143/25176199 ≤ 예약110776/67217592B·잔여0다. TASK-011 변경은 소스1972649/문서f92dd17 게시 후 작업 트리이며 미커밋·미게시다. Human 최종 구현 리뷰 전 Closed로 올리지 않고 TASK-011/MS-004 In Progress·native GAP-004/장비 잔여를 유지한다.

## 마감·형상 기준 — 2026-10-30

Human 커밋·푸시·세션 종료 지시로 소스 `ccecf4a`를 origin/main 게시했다. EV-018 source54개와 commit blob hash가 일치하며 시험을 반복하지 않았다. 커밋 대조(비공개 원시 근거: `imported_context/task_011_2026-10-30/commit.json`)·게시 근거(비공개 원시 근거: `imported_context/task_011_2026-10-30/push.json`). 검증/마감 문서는 소스와 별도 커밋한다. 위 미커밋/미게시 설명은 검증 당시 이력이다. Human 최종 구현 리뷰는 별도이므로 Resolved를 유지한다.

## Human 최종 구현 승인 — 2026-10-31

구현 계약 확인(배속·pause/resume·seek/stop·세대·late 간격·prefetch/읽기 예산·reader 수명) 보고 후 Human 「승인」을 TASK-011 최종 구현 승인으로 반영했다. 승인 대상은 EV-018 구현 및 EV-019/source54개 hash로 고정된 현재 리뷰 조치 작업 트리다. REV-015/016 및 RV-024~028을 Closed로 마감한다. 이전 본문의 Resolved/Human 대기 표기는 승인 전 이력이다.

native GAP-004·다른 OS/장비·실제 timer/성능 검증은 잔여이며 TASK-011/MS-004 In Progress·Gate4/5 Not Ready를 유지한다. 소스 hash가 EV-019와 같아 성공 시험은 반복하지 않았다. 이번 승인은 구현 수락이며 커밋/푸시·TASK-012 착수는 수행하지 않았다.

## 세션 마감·게시 — 2026-10-31

Human 「커밋 푸쉬 후 세션 마무리해줘」에 따라 리뷰 조치 소스 `c6cc5a5`를 origin/main에 게시하고 원격 hash 일치를 확인했다. EV-019 source54개·검사10개 exit0·Miri96·원본/뷰 로그12쌍 hash와 source commit blob54개가 일치해 동일 성공 시험을 반복하지 않았다. 커밋 대조(비공개 원시 근거: `imported_context/task_011_review_fixes_2026-10-31/commit.json`)·소스 게시 근거(비공개 원시 근거: `imported_context/task_011_review_fixes_2026-10-31/push.json`). 이전 미커밋/미게시 문장은 검증 당시 이력이다.

TASK-011 Human 승인·REV-015/016 Closed, ADR-011 Accepted를 유지한다. 현재 PC 실측/UPS 시뮬레이션을 수락하며 native는 공개 완료 후(T3) 진행한다. 다음 시작점은 TASK-022의 실행 커널 확인·P0/제어 부하/기한/경로/예산 프로필 준비다. 현재 generic 실행/RT 설치를 구분하며 재부팅·부트 변경은 수행하지 않았다. TASK-008/010 Human 리뷰·PC 실측/UPS/24h 미실행·Gate4/5 Not Ready는 잔여다. 검증/수락/마감 문서는 별도 docs(cowork)·[skip ci] 커밋으로 게시하며 세션 종료 후 새 Task를 자동 착수하지 않는다.

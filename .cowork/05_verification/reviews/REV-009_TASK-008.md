# [REV-009] TASK-008 회전·보존·이벤트 보호 자가 리뷰

## 개요

- 일자/리뷰어: 2026-10-30 / AI, [code-review 스킬](../../../.agents/skills/code-review/SKILL.md).
- 대상: `71b3e2c` 이후 TASK-008 구현, 로컬 소스 commit `7388145`(source49개·최종 hash/검증은 EV-012).
- 판정: Conditional — 로컬 통합15개·두 Rust 각16 features/consumer/예제/lint/doc/fmt/allocator8·Miri776 Pass, Human 최종 구현 리뷰/native·장비 잔여.
- 상태: Resolved — EV-012 로컬 수정/회귀 Pass, Human 최종 구현 리뷰 대기.

## 리뷰·처리 요약

| ID | 중요도 | 위치·문제 | 처리 | 상태 |
|---|---|---|---|---|
| RV-011 | P2 | retention `room`: unlink 성공 뒤 directory sync 실패하면 삭제된 파일이 live catalog에 잔존할 수 있음 | unlink 성공 즉시 catalog 제외, directory sync 실패는 Poisoned·오류 보고 | Resolved(로컬) |

## RV-011 상세

### [리뷰어]

unlink와 directory sync는 서로 다른 관측이다. sync 실패 시 durable 삭제는 미확인이나 현재 namespace의 파일은 이미 없어졌다. 기존 catalog에 남겨 pin을 발급하면 존재하지 않는 파일을 조회 대상으로 제시한다. 제거 실패와 제거 후 내구성 실패를 나눠 처리해야 한다.

### [처리담당]

- 판정: 수용(A — 승인된 오류/공개 순서 안의 구현 수정).
- `deletion_directory_failure_removes_unlinked_files_from_the_live_catalog` 행동 반례 Red 후 수정·Green.
- 성공한 unlink 즉시 closed entry를 제거하고 directory sync 실패는 Poisoned로 멈춘다. read pin은 UnknownToken으로 거절한다. 재기동 durable namespace 판정은 TASK-009에서 조사한다.
- 근거: TASK-008 `red-delete.log`·`green-e2e.log`, 최종 EV-012.

## 검토 포인트

- RT 입력·원본 unsafe·dependency/lock·worker 기동은 바꾸지 않았다. 새 IO/할당·host callback은 일반 경로다.
- target claim을 private Arc로 공유해 회전 사이 외부 writer 진입을 막고 마지막 소유자가 반환한다. 기존 단일 writer/finished reservation 회귀 대상이다.
- catalog/event/pin과 채널 복제·scratch는 유한 output 예약 안에 둔다. active maximum file reserve와 overlap unique accounting, pin 만료 후 실제 release 전 보호를 확인한다.
- event actual은 세그먼트 mono min/max 기반 coarse extents이며 gap-free 존재 보장이 아니다. file/index byte 판정도 보수적 상한이다.
- filesystem 여유는 host 관측이고 외부 프로세스와의 경쟁은 별도다. FileStore Unix directory sync만 구현했고 미지원은 Unsupported다. native·다른 OS 어댑터/실장비·재기동 복구·active reader는 잔여다.

## 최종 검증

[EV-012](../verification_evidence.md#ev-012)·[source49개/명령/로그 hash](../../06_evolution/imported_context/task_008_2026-10-30/results.json)를 확인했다. `97 tests × 8 seeds = 776` Miri 실행 모두 Pass이며 실제 파일 통합1개는 Miri 범위 밖의 일반 Linux 실행이다. 소스/manifest/lock 49개와 최종 검사10개의 source hash가 일치한다. 현재 검토에서 추가 미해결 P1/P2는 확인하지 못했다. 최종 구현 Human 승인 전이므로 Closed로 승격하지 않는다.

2026-10-30 로컬 커밋 `7388145`는 EV-012 source49개와 일치한다. 커밋·세션 종료 요청을 최종 구현 승인으로 확대하지 않으며 Resolved를 유지한다. 푸시는 수행하지 않았다.

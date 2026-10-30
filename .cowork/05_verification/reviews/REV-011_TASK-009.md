# [REV-011] TASK-009 중단 복구·IO 장애 개발 자가 리뷰

## 개요

- 일자/리뷰어: 2026-10-30 / AI, [code-review 스킬](../../../.agents/skills/code-review/SKILL.md).
- 대상: 소스 `e5f3147`(게시 1f2f44d 이후 TASK-009 구현). EV-014의 최종 source51개/commit blob과 일치한다.
- 판정: Conditional — 복구 통합19개·두 Rust 각16 features/consumer/예제/lint/doc/fmt·allocator9·Miri936 Pass, Human 최종 리뷰/native·장비 잔여.
- 상태: Resolved — Open 등록 → In Progress 조치 → EV-014 로컬 회귀 Pass. Human 최종 구현 리뷰 대기다. 사용자 요청의 리뷰 전용 세션은 아니다.

## 리뷰·처리 요약

| ID | 중요도 | 위치/문제 | 처리 | 상태 |
|---|---|---|---|---|
| RV-015 | P2 | 복구 최종 file sync 도중 취소가 도착하면 성공을 반환할 수 있음 | 최종 sync 이후 취소 재검사, 동기화 sink가 직접 취소를 게시하는 결정적 반례 Red→Green | Resolved(로컬) |

## RV-015 상세

### [리뷰어]

일반 경로는 blocking sync를 강제 취소할 수 없지만, sync가 반환한 뒤 취소가 이미 도착했다면 성공한 복구 결과로 반환하지 않아야 한다. store 공개 전 검사만으로는 직접 `recover_segment`를 쓰는 호출자를 보호하지 못한다. 새 완료 판정의 소비자는 직접 recover 호출자·recover_to_store·실제 FileStore 복구·취소/장애 fixture·독립 판독이다.

### [처리담당]

- 판정: 수용(A — 승인된 취소/실패 계약의 구현 세부 수정).
- `cancellation_arriving_during_final_sync_does_not_return_a_successful_recovery`에서 sync callback이 AtomicBool을 게시하는 반례 Red를 확인했다(`red-cancel-sync.log`).
- `finished.sync_recovered` 뒤 `cancelled`를 검사해 취소 시 새 segment를 성공으로 반환하지 않는다. 진행 중 IO를 강제로 중단한 것으로 표시하지 않는다.
- Green: `green-cancel-sync.log`와 최종 EV-014, 반환 시 prefix/원인·Drop 무추가 IO·store rename/directory 단계·실제 파일 독립 판독을 회귀한다.

## 자가 점검 포인트

- RT 수집/거절/Drop·unsafe·dependency/lock은 변경하지 않는다. 모든 decoder·IO·할당은 일반 복구 경로다. 외부 MCAP/runtime 타입을 제품 API에 노출하지 않는다.
- file-controlled lengths를 frozen source length/overflow/fixed buffer와 대조한 후 읽는다. nested framing/schema/channel/metadata/봉투/producer sequence·time bounds를 확인한다. chunk 전체 검증 뒤 출력하므로 손상 chunk의 선행 메시지도 복구하지 않는다.
- 원본은 Write bound가 없는 source로 받으며 호스트가 read-only open·불변성을 보장한다. 표준 filesystem fixture에서 원본 바이트 불변·독립 출력·기존 경로 거절을 확인한다. custom source/sink/store의 계약 위반까지 방어했다고 주장하지 않는다.
- source message index/summary는 재생성하며 옛 index 바이트를 데이터 손실 메시지 수로 세지 않는다. 새 파일의 sync는 과거 프로세스의 completion acknowledgment가 아니다.
- 공유 output 추가 예약·writer 예약·한계/오류의 lease 반환 및 실제 allocator peak를 대조한다. 8MiB test는 일반 allocator 시험이며 Miri는 작은 fixture만 다룬다.
- recover_to_store는 capability→create→복구/file sync→close→rename→directory sync이며 실패는 성공 catalog를 만들지 않는다. 실패 파일은 보존하고 retry/delete하지 않는다. 다른 OS native·전원 차단·자율 startup 발견/재개·worker/StorageDegraded는 이번 primitive 검증으로 완료하지 않는다.

## 최종 검증

[EV-014](../verification_evidence.md#ev-014)·[source51개/명령/로그 hash](../../06_evolution/imported_context/task_009_2026-10-30/results.json)를 확인했다. 검사10개 exit0·Miri7 programs×8 seeds/56개 성공 summary·936회 Pass이며 두 Rust16조합/독립 consumer/예제/allocator9/lint/doc/fmt가 같은 source51개를 가리킨다. 현재 점검에서 추가 미해결 P1/P2는 확인하지 못했지만 Human/native·실물 검증을 대신하지 않으며 Conditional/Resolved를 유지한다. TASK-009 소스 e5f3147은 게시 요청을 받아 커밋했으며 Human 최종 구현 리뷰 대상이다.

세션 마감 — 2026-10-30: Human 「세션 마무리해줘」 및 「커밋 푸쉬도 해줘」에 따라 TASK-009 소스 `e5f3147`를 커밋했다. EV-014 source51개/commit blob 및 검사10개 근거가 일치하여 시험을 반복하지 않았다. `.cowork` 문서와 분리해 게시하며, 구현 Human 승인·Resolved 리뷰·native/장비 이월 상태는 유지한다. [커밋 근거](../../06_evolution/imported_context/task_009_2026-10-30/commit.json).

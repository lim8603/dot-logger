# REV-083 — MS-013 r10 설계 재리뷰

| 항목 | 내용 |
|---|---|
| 일자 / 리뷰어 | 2026-11-28 / Codex, 독립 리뷰 세션 |
| 대상 | [MS-013 r10](../../04_implementation/milestones/MS-013.md), [REV-083 요청서](REV-083_REQUEST.md), [REV-082](REV-082_MS013_R9_DESIGN_REVIEW.md) |
| 기준 | r10 및 검토 시 HEAD `346a993219a3122c2fc2b5057ece223541edd6d9`, Human 승인 기록 `3be4ffc` |
| 판정 / 상태 | **Pass / Open** |

## 1. 요약정리

**이전 지적 2건 해소, 새 지적 0건(P1 0·P2 0·P3 0).** r10에서 파일 작업자의 성공 확정은 실패를 만들 수 있는 정리 단계 뒤로 이동했고, `next()` 호출 기한과 `AckGone` 절대 기한의 결과가 구분됐다. 시험과 성능 측정은 요청 범위 밖이다. 이 판정은 r9→r10 변경과 현재 Rust 소스에 대한 정적 설계 대조다.

## 2. 이전 지적 대조

| 이전 지적 | r10 대조 결과 |
|---|---|
| RV-268 · 파일 성공 확정 | **해소.** 마지막 `finish` 직후의 시각은 스레드가 보관한다. 남은 sync 응답과 진입 closure의 정리를 마친 뒤에만 셀을 종결한다. 파일 IO 실패 지점은 IO 스레드 안에 있어 담당 스레드의 `Failed`로 반영할 수 있다. 종료한 스레드의 셀이 `Pending`이면 host가 `Failed`로 종결한다. `Published` 상태로 회수만 지연된 경우에는 파일 성공이 유지되고 `Closed`만 늦어진다. [r10 셀 종결](../../04_implementation/milestones/MS-013.md#L424), [현행 IO 진입 및 해제](../../../crates/dot-logger/src/logger/host.rs#L474), [IO 실패 경로](../../../crates/dot-logger/src/logger/host.rs#L818), [finish 뒤 응답 처리](../../../crates/dot-logger/src/logger/host.rs#L948) |
| RV-269 · 두 대기 기한 | **해소.** 처음 `Gone`을 본 같은 `next()`에서 호출 기한만 끝나면 `TimedOut`을 반환하고 기존 `AckGone` 상태 및 절대 기한을 보존한다. 후속 호출은 그 절대 기한 전에 End를 읽을 수 있다. 절대 기한 만료와 동시 만료는 sticky `AcknowledgementUnknown`으로 끝나며 중간 이벤트는 기한을 연장하지 않는다. [r10 기한 규칙과 시험](../../04_implementation/milestones/MS-013.md#L390), [현행 재시도 가능한 호출 timeout](../../../crates/dot-logger-client/src/transport.rs#L710) |

RV-262의 미확인 발행 시각, RV-265의 실패 종결 및 회수 경계, RV-266의 모드별 종료, RV-267의 재설정 없는 절대 기한이 유지된다. IO 셀 종결과 IO·네트워크·legacy tap 회수 조건은 승인된 모드별 계약과 일치한다. 원본 guard 미전달, 복사 예산, completion·관리 wire 분리, 기존 route 및 legacy tap 호환, DB 단계 6과 H6 조기 판정에서 r10 변경으로 인한 퇴행을 확인하지 못했다. [모드별 종료 조건](../../04_implementation/milestones/MS-013.md#L433), [반영표](../../04_implementation/milestones/MS-013.md#L707)

## 3. 리뷰 포인트

새 지적 0건. 승인된 설계 결정의 재검토 의견 0건. 원자 순서와 실행 시간, 종료 실패 주입 및 wire 시험은 설계에 명시된 골든·spike·구현 단계에서 확인한다.

## 4. 인계

지적 0건 수렴 조건을 충족했다. 리뷰 문서는 최초 상태 `Open`으로 등록한다. 개발 세션에서 결과를 반입한 뒤, 2026-11-28에 승인한 범위의 ADR-017~019·FR/NFR·TASK를 등록한다. MS-013 구현 순서는 TASK-041 v0.1.1 뒤다.

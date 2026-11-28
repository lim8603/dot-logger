# TASK-050 — MS-013 단계 2: 네트워크 출력·파일 없는 시작·손실 회계·종료

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / [MS-013](../milestones/MS-013.md) 단계 2 |
| Story | US-001·003·004 |
| 요구사항 | FR-019a·021·022, NFR-010 — [TC-015](../../05_verification/test_case.md#tc-015--출력-구성네트워크-전용전송-스위치) 2단계 |
| 설계 | MS-013 §5.2·§5.4·§5.8·§5.9·§5.10·§6·§8.1 |
| ADR | [ADR-017](../../03_design_artifacts/adrs/ADR-017_output_model_copy_observer.md) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-28 |
| 다음 액션 | S0 — TASK-049 Done 뒤 |
| 의존 | TASK-049 |
| 예상 소요 | 5~6 인일 |

## 목표

facade에서 네트워크 출력을 선언하고, 파일 없이 시작한다. 송신 쪽에서 네트워크 손실을 세고, 종료 시 구성된 자원만 기다리며 그 결과를 보고한다. 0.2의 facade breaking 대부분이 이 단계에 있다.

## 작업 단계

| 단계 | 목표 | 종료 상태 | 상태 |
|---|---|---|---|
| S0 | 계약·이관 비용 | API §25.2(facade 네트워크 출력·보고·종료). 기존 보고 타입 `#[non_exhaustive]` 이관 비용 확인과 결정 기록(Q7). 공개 표면 예상 항목 수 | Planned |
| S1 | 선언·구성 검증 | `network_output`·`NetworkOutput`, `Missing::Output`, `StartError::Conflict`(2개·`remote()` 동시), H5 기본 경로, `outputs_named`·`file_output_named`·혼용 거절, C5 tap 불변. 조합 3종·출력 0개 시험 Green | Planned |
| S2 | 네트워크 스레드·hub 회계 | 네트워크 출력 스레드, hub 기록 상태(`queued`→`partial`→`sent`→`acked`)·잠금 안 계수, L1~L3·`offer_pending`, 항목 시작·offer·항목 끝 정산, Health seqlock(3회·`stale`). §5.4 시험 목록 Green | Planned |
| S3 | 네트워크 전용 모드 | 파일 IO 0, §5.10 예약 합계 검증, `Health.durability = None`, `Plan.sync_every = ZERO`·`PlanWarning::Unused`, `EventError::NotConfigured`, 예산 경계 ±1, 재연결 1,000회·최대 구독·포화 heap | Planned |
| S4 | 종료·실패 격리 | 감독 정산(생산 닫기·봉인·위치 정산, 멱등), IO 스레드별 발행 셀(마지막 동작 CAS), `files_late`·`files_complete()`·`files_settled()`, `finalized`/`reclaimed`, 모드별 `Closed`, `finish` 대기 중 host 기한 감시. §5.9 두 판정표·모드 표 행별 시험 Green | Planned |
| S5 | `loss_file` 옵션 | 선택 로컬 손실 요약 파일(1초 버퍼·1,024 구간·4초 큐·`max_bytes` 회전, `Degraded(OutputLosing)`) | Planned |
| S6 | 문서·마감 | README·rustdoc 모드별 보장표(NFR-010), L3 「네트워크 전용」 레시피, 공개 표면 gate(≤ 60, `--bless`는 diff 확인 뒤), 외부 소비자 검증, CHANGELOG, 두 Rust 회귀·Clippy·feature·Miri·CI, EV, 독립 리뷰 | Planned |

## 완료 기준

- [ ] FR-019a·021·022·NFR-010 수락 시험 Pass.
- [ ] 표면 gate·외부 소비자·CI Pass, 독립 리뷰 Pass, EV.

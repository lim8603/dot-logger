# REV-074 리뷰 요청 — MS-013 요구사항과 리팩토링 계획(설계 리뷰 1회차)

| 항목 | 내용 |
|---|---|
| 요청 일자 / 요청자 | 2026-11-27 / lim8603(프로젝트 오너), 작성 AI 개발 세션 |
| 받는 쪽 | 독립 리뷰 전용 세션([세션 프로토콜 §1E](../../01_cowork_protocol/session_protocol.md)) — 제품 코드·시험·스크립트·CI 수정 금지. **대상 문서(MS-013)도 고치지 않는다**(지적만) |
| 대상 | [MS-013](../../04_implementation/milestones/MS-013.md) 「출력 구성 선택과 네트워크 전용 모드: 요구사항과 리팩토링 계획」(Draft). 코드 변경 없음 — **설계 문서 리뷰** |
| 기준 커밋 | 이 요청서와 MS-013을 넣은 `main` 커밋(리뷰 시작 때 `git log -1 -- .cowork/04_implementation/milestones/MS-013.md`로 확인). 코드 대조 기준은 같은 커밋 |
| 결과물 | 이 폴더에 `REV-074_<목적>.md`(초기 상태 `Open`), 지적 번호는 `RV-213`부터. 형식은 [REV-073](REV-073_HARNESS_AND_LATENCY_REVIEW.md)과 같게(요약정리 → 대상 표 → 리뷰 포인트, P1/P2/P3, 종합 판정 Pass/Conditional/Reject). 설계 리뷰라 「파일 변경표·클래스 변경표」 대신 **문서 절 → 관련 코드 대조표**를 둔다 |
| 상태 동기화 | 리뷰 세션이 [project_state.md](../../06_evolution/project_state.md)의 미처리 리뷰·코드리뷰 현황에 REV-074를 등록한다 |
| 이후 | 개발 세션이 지적을 반영해 MS-013을 고친 뒤 **재리뷰(REV-075)를 1회** 받는다(2026-11-27 Human 결정). 재리뷰 요청서는 개발 세션이 쓴다 |

## 1. 왜 리뷰하나

Human이 요구 R1~R3(출력 선택·네트워크 전용 고부하 모드·전송 스위치)을 제안했고, 우려 C1~C11을 검토해 방향을 정했다(MS-013 §2.1). 구현은 facade 출력 구조의 핵심(원격을 파일 IO의 tap에서 Router 출력으로)을 바꾸고 0.2 공개 API를 깨므로, 코드를 쓰기 전에 독립 리뷰로 설계 결함·누락·과잉을 잡는다.

## 2. 읽을 것

- 대상: [MS-013](../../04_implementation/milestones/MS-013.md) 전체.
- 사실 대조용 코드: `crates/dot-logger/src/logger/{mod.rs, budget.rs, host.rs, remote.rs, health.rs}`, `crates/dot-logger/src/routing/{mod.rs, types.rs}`, `crates/dot-logger/src/adapters/remote/{hub.rs, types.rs, server.rs}`, `crates/dot-logger/src/completion/`, `crates/dot-logger/src/storage/{codec.rs, writer.rs}`.
- 기존 계약·결정: [API §18.11·§18.12·§22.2](../../03_design_artifacts/interface_contract.md#1811-s3c-remote-선택-연결--2026-11-07), [ADR-002](../../03_design_artifacts/adrs/ADR-002_rt_logging_never_waits_for_storage.md)·[ADR-008](../../03_design_artifacts/adrs/ADR-008_bounded_ownership_completion.md)·[ADR-012](../../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md)·[ADR-015](../../03_design_artifacts/adrs/ADR-015_batched_pipelined_remote_ack.md), [EV-105](../verification_evidence.md#ev-105)(처리량), [EV-123](../verification_evidence.md#ev-123)(F1 역류·F4 원격).
- Human 결정의 원문 맥락: [project_state 기능 논의 안건 상세](../../06_evolution/project_state.md#기능-논의-안건-상세--출력-구성-선택과-네트워크-전용-모드-2026-11-26).

## 3. 중점 관점

### 중점 1 — 현재 구조 기술(§3)이 코드와 맞는가

- §3 표의 각 행을 코드로 확인한다. 특히: 원격이 Router target이 아니고 첫 파일 출력 IO 스레드의 tap이라는 것, tap 결과를 버린다는 것, 모든 파일 target이 `required`·`pending_slots 0`이고 공유 한도가 먼저 찬다는 것, 원격 손실이 로거 쪽 어디에도 집계되지 않는다는 것.
- 계획이 기대는 core 동작이 실제로 있는지: Router의 출력별 FIFO·생산자별 순서, `required: false` + `pending_slots: 0` target이 가득 찼을 때 대기 없이 버리고 무엇을 보고하는지, 공유 기록 한도(`shared_records`)와 target별 `queue_slots`·`retained_bytes`의 관계.

### 중점 2 — 목표 설계가 RT·손실 계약을 지키는가

- **ADR-002(RT는 저장을 기다리지 않음):** BestEffort 네트워크 출력이 멈췄을 때 RT `try_log`가 기다리거나, Required 파일 출력의 완료·`is_success`가 나빠지는 경로가 없는지(§5.1 K1 공유 한도 잠식, 「기본 전체의 1/2」 상한이 충분한 근거인지).
- **완료 경계(§5.3):** `Handed`를 경계로 두는 것이 ADR-008 완료 모델과 맞는지, 원본을 복사 직후 돌려주는 것이 raw(zero-copy) 기록의 수명 계약과 충돌하지 않는지. `ReceiverAcknowledged` 선택 시 원본 보유가 K1을 다시 만들지 않는지.
- **손실 집계(§5.4):** 계수 정의와 불변식이 셀 수 있는 것인지(특히 tokio 스트림 안쪽 「보냄」을 관측하지 않기로 한 것), 빠진 손실 경로(재연결, 구독 필터 불일치, 메시지 크기 초과, 종료 시 버퍼 잔여)가 없는지.
- **tap 유지 규칙(§5.2, C5):** 「네트워크 출력이 있으면 tap 끔」이 기존 `remote()` 사용자 의미를 실제로 보존하는지, 두 원천이 섞이는 경계 사례가 없는지.

### 중점 3 — 요구를 빠짐없이, 과하지 않게 덮는가

- R1~R3·C1~C11(§2.1)이 §1.3 요구·§7 단계·§8 시험으로 각각 추적되는지. 빠진 요구나 근거 없이 늘어난 범위가 있는지.
- §1.2의 해석(100 MB/s는 보장 목표가 아님, 측정·공개)과 §8.2 측정 계획·P7 목표가 서로 맞는지.
- 범위 밖(§1.4)으로 뺀 것 중 R2·R3 사용에 사실상 필수인 것이 있는지(예: keepalive 없이 조용한 단절을 손실 집계가 놓치는지 — EV-123 F4).

### 중점 4 — 공개 API와 설정 파일 설계

- §6 breaking 목록이 완전한지(공개 struct 필드·enum 변형 추가, 의미 변경), `#[non_exhaustive]` 제안(Q7)의 득실.
- §5.5 코드/파일 분담(타입은 코드, 정책은 파일)과 우선순위(호출 순서, Q5)가 혼란을 만들지 않는지. 비밀 처리(값 직접 기재 거절, 오류에 값 미출력)가 충분한지. TOML·feature 분리가 범용 라이브러리 방향(C7)에 맞는지, 의존성 추가(serde·toml)의 audit·MSRV 1.85 영향.
- 출력 index 규칙(Q8)이 기존 `ChannelOptions::outputs` 사용자 의미를 보존하는지.
- 스위치(§5.6): Router target 켜짐 플래그와 경계 기록(`.logger/remote`)이 기존 예약 토픽 규칙(`.logger/meta`·`.logger/loss`, ADR-013 개정)과 맞는지, 켜기·끄기와 동시 기록의 경합, 「끈 동안 분배 0」을 시험으로 증명할 수 있는지.

### 중점 5 — 단계 계획의 실행 가능성

- §7 단계 순서가 각 단계 끝에 「전체 회귀 통과」를 유지할 수 있게 나뉘었는지(특히 단계 1 「동작 불변 리팩토링」이 정말 공개 동작을 안 바꾸는지).
- 인일 추정과 위험(§9.1)이 현실적인지, 빠진 위험이 있는지(예: Windows·macOS의 짧은 대기 해상도 EV-128이 네트워크 출력 스레드에 주는 영향).
- §8 시험이 각 요구의 실패를 실제로 잡을 수 있는지(Red가 가능한지).

## 4. 범위 밖

- 제품 코드 수정·시험 실행으로 설계를 증명하는 일(필요하면 「단계 0에서 확인할 것」으로 지적).
- 성능 재측정. EV-105 등 기존 근거로만 판단한다.
- MS-012(InfluxDB) 진행 상황 자체. §5.7 DB 출력 게이트의 타당성만 본다.
- §9.2 열린 질문의 최종 결정(Human 몫). 리뷰어 의견은 환영한다.

## 5. 참고 명령

```sh
git log -1 -- .cowork/04_implementation/milestones/MS-013.md
grep -n "tap\|required: true\|pending_slots" crates/dot-logger/src/logger/{host.rs,budget.rs,remote.rs}
grep -n "pub struct TargetConfig" -A20 crates/dot-logger/src/routing/types.rs
grep -n "fn offer\|Mutex<Registry>\|Gap\|Disconnect" crates/dot-logger/src/adapters/remote/hub.rs
cargo +1.96.0 doc -p dot-logger --all-features --no-deps --locked   # 공개 API 확인용
```

코드 빌드·시험은 필요할 때만, `CARGO_TARGET_DIR=/data/build/target`으로. 장시간 실행은 하지 않는다.

# ADR-012 — 기본 실행 계층(facade)의 소유 모델: 공유 core + 소유 endpoint 핸들 + 빌림 뷰

- 상태: Accepted / 날짜: 2026-11-05 / 작성: AI(TASK-027 S1)
- 승인: 2026-11-05 Human — 리뷰 포인트 5개를 하나씩 검토해 모두 권고안을 선택했다: ① 옵션 F, ② 기본값 혼합(성능 값은 facade 기본값, 손실 값은 필수), ③ `Logger::drop`은 기한 있는 종료, ④ NFR-008 수치 제안대로, ⑤ `writer.try_log(channel, &msg)` 분리 형태. 상세는 아래 「결정」과 [API §18.7](../interface_contract.md#187-human-결정--2026-11-05).
- 관련: INT-001 / MS-008 / TASK-027(S2·S3)·TASK-023 S2/S3·TASK-026 / FR-001·003·008·012·013 / NFR-001·004·005·008 / US-001~003
- 관계: [ADR-008](ADR-008_bounded_ownership_completion.md)의 "RT 원본 수명" 행(session owner + borrowed producer/loan)을 **유지**하고, 그 위에 소유형 facade 계층을 추가한다. ADR-007의 내장형 파이프라인·별도 데몬 없음은 그대로다. 대체하는 ADR은 없다.

## ADR 필요성 판단

- 제약: 공개 핸들의 수명 모델(`'static` 여부)은 모든 소비처 코드 구조를 결정하고, 공개 후 되돌리기 어렵다.
- 확장성: typed channel·tracing·remote·preset(TASK-028)·viewer(MS-009)가 모두 이 핸들 위에 얹힌다.
- 운영 영향: 기본 worker 스레드·종료/Drop 동작·worker panic 처리가 운영 절차를 바꾼다.
- 세 축이 함께 얽히고 ADR-008의 Arc 위험 판단과 직접 맞물리므로 ADR로 남긴다.

## 맥락

2026-11-04 사용성 평가에서 Human은 소비처 접점이 너무 무겁다고 판단했다. 핵심 crate의 공개 항목은 534개다. thread를 만드는 기본 worker가 없어서 호스트가 capture→router→writer→retention→sync 배선을 직접 작성한다. 메모리 7분할과 풀 slot도 명시해야 한다. 그 결과 `dot_probe_pipeline` 본문 39줄에 support 약 1,000줄이 필요했다.

현재 core는 모든 핸들이 `CaptureOwner`의 빌림 수명에 묶여 있다.

- `CaptureOwner::scope(&mut self, |scope| ..)`가 `Producer<'owner>`(`&'owner mut WriteState` 등 13개 참조)와 `CaptureConsumer<'owner>`를 빌려준다.
- `Router<'owner>`·`Delivery<'owner>`·`ControlOwner<'owner>`·`SyncWork<'control,'owner>`·`Loan<'owner>`도 같은 수명을 공유한다.
- 따라서 앱의 장기 thread(`std::thread::spawn`, tokio task)로 `'static` 핸들을 넘길 수 없다. 모든 RT 코드를 owner scope 안에 두어야 한다.

ADR-008은 borrowed 모델을 고른 이유로 "Arc 마지막 Drop은 RT free 가능"을 기록했다. interface §13은 "미검증 Arc로 수명을 우회하지 않는다"고 정했다. 이 ADR은 그 위험을 해소하는 구체 장치와 검증 조건을 함께 정한다.

추가 관측(코드): `WriteState`는 `rtrb::Producer` 두 개와 `sampling: Box<[u32]>`를 소유한다. 핸들이 이 상태를 값으로 소유한 채 Drop되면 RT thread에서 heap 해제가 일어난다. `rtrb` endpoint의 Drop은 반대편이 살아 있는 한 Arc 감소만 한다.

## 결정 요인

1. RT 경로(`try_log`/`try_record_*`) 무대기·무할당·Arc clone 0을 유지한다(NFR-001, ADR-002).
2. 이미 검증된 RT 코드와 private unsafe 경계(`pool/storage`)를 바꾸지 않는다.
3. 앱 thread 구조를 강제하지 않는다. `'static + Send` 핸들, scope·수명 매개변수 없이 쓸 수 있어야 한다(NFR-008).
4. 어떤 Drop 순서에서도 RT thread가 heap을 해제하지 않는다. 사용 중인 메모리·IO를 강제로 해제하지 않는다(ADR-008 종료 계약).
5. 기존 세부 API는 고급 경로로 유지하고 외부 시그니처를 바꾸지 않는다.

## 검토한 옵션

| 옵션 | 개요 | 판단 |
|---|---|---|
| A. core 전면 Arc 소유 | `Producer`·`Loan`·`Delivery` 등 모든 핸들이 `Arc` 보유 `'static` 타입 | 기각. 검증된 RT/unsafe 코드를 전면 수정한다. Loan·Delivery마다 Arc clone(원자 연산)이 생긴다. 마지막 Drop 위험이 모든 타입으로 퍼진다. |
| B. scope 유지 + 제어 역전 | `Logger::run(cfg, \|handles\| app_body)`, 내부 thread는 scope 안에서 실행 | 기각. 안전하지만 앱 전체가 closure 안에 들어가야 한다. dot-probe의 장기 thread·tokio 구조와 맞지 않는다(ADR-008 결과 절의 우려 그대로). |
| C. 프록시 링 | 앱용 `'static` 링 → 내부 host thread → borrowed Producer로 재제시 | 기각. 복사 2회·메모리 2배·지연 증가. 원본 zero-copy loan을 전달할 수 없다. |
| D. `Box::leak` 세션 | owner를 leak해 `&'static`을 얻어 기존 API를 그대로 사용 | 기각. 안전하지만 원본 풀·링 메모리가 프로세스 종료까지 회수되지 않는다. 재시작·설정 재기동마다 누수가 누적된다. |
| E. 자기참조 셀 | `self_cell`/`ouroboros` 또는 unsafe 수명 확장 | 기각. 새 unsafe 경계나 의존성이 생긴다. `&mut WriteState` 독점 빌림은 표현하기 어렵다. |
| **F. 공유 core + 소유 endpoint + 빌림 뷰** | 아래 결정 | **권고** |

## 결정

### 1. core를 "공유 불변부"와 "독점 endpoint"로 내부 분해한다

- `CaptureOwner` 내부를 `CaptureCore`(채널 등록·closing flag·producer별 `consumed`·slot 크기·`RawPool`·completion registry·telemetry bank·config update 상태·memory budget)와 producer별 endpoint(`WriteState`, `ReadState`)로 나눈다.
- 기존 `CaptureOwner::new/new_tracked/scope`와 `Producer<'owner>`의 공개 시그니처·RT 구현은 바꾸지 않는다. `scope`는 `(&core, &mut endpoints)`에서 같은 뷰를 만든다.
- facade 전용 `pub(crate)` 경로가 `Arc<CaptureCore>`와 endpoint 소유권을 분리해서 넘긴다.

### 2. 앱 핸들은 `'static + Send + !Sync` 소유 핸들이다

- `Writer`는 `Arc<CaptureCore>` 하나, 자기 producer의 `WriteState`, 사전 할당 encode scratch, 반환 mailbox endpoint를 소유한다.
- 매 호출(또는 thread 본문 전체)마다 `Writer::producer(&mut self) -> Producer<'_>` 빌림 뷰를 만든다. 뷰는 `&mut self.state`와 `&self.core`의 필드 참조를 복사한 stack 값이다. 할당 0, Arc clone 0, 원자 연산 0이다.
- 따라서 기존 고급 RT API(`try_acquire`/`Loan`/`try_record_raw`)를 `'static` thread에서 그대로 쓸 수 있다. 예: `thread::spawn(move || { let mut p = writer.producer(); loop { .. } })`. `Loan<'_>`은 뷰의 수명을 따른다.
- facade의 `Writer::try_log(channel, &msg)`는 이 뷰 위에서 기존 `try_record_small`(S3에서 slot 직접 encode)을 호출하는 얇은 계층이다.
- `Channel<T>`는 `Copy` 식별 토큰(session·channel·schema)이다. 메모리를 소유하지 않는다.

### 3. Drop은 어떤 순서에서도 RT thread에서 heap을 해제하지 않는다

- `Writer::drop`은 소유 상태(`WriteState`·scratch)를 시작 때 미리 만든 1-slot 반환 mailbox(`rtrb`)로 **이동**만 한다. 그다음 Arc 감소와 mailbox endpoint 감소만 한다. 이동·감소에는 할당/해제가 없다.
- 불변식: `Logger` 또는 worker host가 살아 있는 동안, 그리고 종료 뒤에는 **reaper**가 `Arc<CaptureCore>`·모든 `ReadState`·반환 mailbox 수신단을 보유한다. 그래서 `Writer`는 어떤 공유 자원의 마지막 소유자도 되지 않는다.
- 종료 시 반환되지 않은 `Writer`가 있으면 보고서에 `writers_outstanding`으로 남긴다. 남은 자원은 management reaper thread로 옮긴다. reaper는 모든 반환을 확인한 뒤 management thread에서 해제한다. 강제 해제·무효화는 하지 않는다.
- 구현 보강(2026-11-07, REV-051 RV-120, 결정 변경 없음): "반환 확인"은 상태가 mailbox에 들어온 시점이 아니라 `Writer`의 mailbox `Producer`가 drop된 시점(rtrb `Consumer::is_abandoned`)이다. 상태만 들어온 시점에 해제하면 RT 스레드가 링의 두 번째 dropper가 되어 링·상태를 해제할 수 있다. 또한 `Arc<CaptureCore>`도 반환 상태 안으로 옮겨, push 뒤 `Writer`에 남는 필드가 어떤 공유 자원의 감소도 하지 않게 했다.
- 검증 조건(S2 DoD): `tests/rt-allocation` allocator guard로 다음 Drop에서 할당·해제가 0임을 확인한다. ① Logger 동작 중 Writer Drop, ② shutdown 반환 뒤 Writer Drop, ③ Logger Drop 뒤 Writer Drop.

### 4. worker 측은 facade 소유 thread 안에서 기존 scope API를 그대로 쓴다

- `start()`가 management host thread 하나를 만든다. host는 `Arc<CaptureCore>`와 모든 `ReadState`를 소유하고 `Router`·`ControlOwner`·`SegmentLog`·telemetry를 자기 stack에서 빌려 구성한다. 빌림 수명은 host thread 안에서 끝나므로 core의 borrowed 계약을 바꾸지 않는다.
- host 안에서 `std::thread::scope`로 다음을 실행한다.
  - dispatcher·control loop: dispatch turn, reclaim, sync 요청·적용, completion feedback, telemetry 표본
  - IO thread N개(기본 1): file target들을 round-robin으로 맡는다. `TargetConsumer`/`Delivery`/`SyncWork` 빌림을 그대로 사용한다.
- 원격·DB·tracing 어댑터는 S3의 선택 연결이다. remote의 tokio runtime은 facade가 소유한다.

### 5. 종료·실패 계약

- `Logger::shutdown(deadline)`는 기존 `begin_shutdown`/`poll_shutdown`을 구동한다.
  - 기한 안에 끝나면 `Closed(report)`를 반환한다.
  - 기한을 넘기면 `Incomplete { report, pending }`을 반환한다. `pending`은 `wait(deadline)`로 이어서 기다릴 수 있다. `pending`을 버리면 host가 정리를 계속하고 끝난 뒤 자원을 해제한다(detach). 진행 중 IO·guard를 무효화하지 않는다.
  - 보고서는 core `ShutdownReport`(필수/전체 totals·잔여 IO·sync 실패)에 다음을 더한다: 미확인 producer 범위, `writers_outstanding`, `worker_failed`.
- worker panic: host는 scoped thread의 panic을 join에서 수집한다. 그 즉시 ingress를 close하므로 이후 `try_log`는 `Closing`이다. 처리 중이던 guard는 기존 Drop 계약대로 Unknown이 된다. 보고서와 `health()`에는 `WorkerFailed`를 표시한다. host 자체 panic에도 drop guard가 closing을 설정한다. owner 해제는 항상 host(management) thread에서 일어난다.
- `Logger::drop`(2026-11-05 Human 결정): `Budget.drop_deadline` 동안 `shutdown`과 같은 기한 있는 종료(배출 → 최종 sync → join)를 수행하고 보고서는 버린다. `Logger`는 management thread에서만 다루므로 RT thread를 멈추지 않는다. 기한을 넘기면 `PendingShutdown` Drop과 같이 detach한다.

### 6. 기본값 위치 (2026-11-05 Human 결정)

- 성능 값(IO thread 수=1, CPU 고정 없음, idle 대기 상한)에는 facade 기본값을 둔다. 잘못 정해도 성능만 바뀌고 손실 범위는 바뀌지 않는다.
- 손실 값(sync 주기, `shutdown`·`drop_deadline` 기한, 메모리·slot 크기, 보존 용량)은 `Budget` 또는 preset(TASK-028)에서 필수로 받는다. 빠지면 `start()`가 빠진 항목 목록과 함께 거절한다. ADR-008의 '장비 미확정 손실 기본값 금지'를 유지한다.

## 결과

### 긍정적

- 앱은 수명 매개변수·scope·support 모듈 없이 `'static` 핸들로 연동한다. 고급 RT API도 같은 핸들의 뷰로 계속 쓸 수 있다.
- 검증된 RT 경로·unsafe 경계·세부 공개 API는 바뀌지 않는다(내부 필드 분해만). TASK-023 S2/S3는 실제 제품 경로를 측정한다.
- ADR-008의 Arc 마지막 Drop 위험을 "반환 mailbox + 관리 측 최종 소유" 불변식과 allocator guard 시험으로 닫는다.

### 부정적 / Trade-off

- `Writer`당 반환 mailbox(1-slot 링)·scratch 버퍼가 추가된다. facade 고정 overhead로 control/worker 예산에 포함하고 계산 근거를 공개한다.
- `Writer`는 thread당 하나(`!Sync`)다. 여러 thread가 같은 채널에 쓰려면 Writer를 여럿 선언한다. 초기 스케치의 `channel.try_log(&msg)` 대신 `writer.try_log(channel, &msg)` 형태가 된다.
- 반환되지 않은 Writer가 있으면 메모리가 reaper에 남는다. 보고는 되지만 앱이 Writer를 잃어버리면 회수되지 않는다(ReadPin 분실 계약과 같은 성격).
- `try_log`의 기본 시각 획득(단조·실시간 시계 읽기 2회)은 RT 비용 항목이다. TASK-023에서 측정하고, 시각을 이미 가진 호출자를 위해 `try_log_at`을 제공한다.

## 승인 범위와 잔여

- 승인 범위: 옵션 F의 소유 모델, 3절의 Drop 불변식과 검증 조건, 4절의 worker 구성 원칙, 5절의 종료·panic·Drop 계약, 6절의 기본값 위치.
- 상세 API는 [interface §18](../interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05), NFR-008 수치는 §18.6에 둔다.
- 실제 RT 지연·처리량은 TASK-023, native는 T3로 이월한다. 이 ADR의 승인은 구현·성능 합격이 아니다.

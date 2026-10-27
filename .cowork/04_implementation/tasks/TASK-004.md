# TASK-004 — 원본 풀·수명·무해제 반환

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-002 |
| Story | US-001·005 |
| 요구사항 | FR-002 FR-012 NFR-001 NFR-005 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-27 |
| 다음 액션 | 구현·EV-003 로컬 검증 완료; Human 리뷰 승인·변경분 native CI 대기 |
| 의존 | TASK-003 |
| 예상 소요 | 2~4 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현: [`pool/`](../../../crates/dot-logger/src/pool/), 기존 `capture/` 연동, [`examples/scoped_capture.rs`](../../../examples/scoped_capture.rs), `tc_002_pool.rs`, allocator guard·CI recipe.

첫 실패/리뷰 조건: **거절 loan Drop과 마지막 RT 핸들 Drop에 allocator 호출 0, return 알림 포화에도 전량 회수**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-002`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] owner outlive compile-fail·대여 유일성·generation·Miri/모델 검사, dot-probe thread 수명 적합성 검토.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 리뷰 완료(2026-10-27 Human 「승인하고, 커밋 푸쉬」). 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-002](../../05_verification/test_case.md#tc-002). 실행: 2026-10-27 / [EV-003](../../05_verification/verification_evidence.md#ev-003--task-004-원본-풀수명반환). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.


## 구현 결과·리뷰 범위 — 2026-10-27

- `CaptureOwner`가 생산자별 원본 allocation을 보유한다. `Producer::try_acquire(length, importance)`의 `Loan`은 복제/공유 불가·owner보다 오래 살 수 없는 borrowed guard다. 고정 슬롯의 요청 길이만 쓰고 최종 allocation 해제는 owner의 일반 Drop에서 수행한다.
- Normal은 Normal 물리 슬롯만, Important는 예약 슬롯 우선 후 Normal 슬롯도 대여한다. 대여 시부터 slots/최대 bytes를 보호하며 다른 생산자 quota는 빌리지 않는다. 원본 payload는 raw, 슬롯 metadata는 control, 큐/endpoint는 input 예산에 각각 한 번 센다.
- `try_record_raw`는 채널/schema·pool/producer·대여 중요도를 검증한다. 거절은 이유와 동일 loan, 생략은 동일 loan을 반환한다. 수락만 sequence 증가·불변 payload 게시이며 small/raw가 하나의 FIFO를 사용한다. `RawRecord`가 살아 있거나 bytes 참조가 남아 있으면 회수·쓰기 재사용이 불가능하다.
- 슬롯당 `generation:61 bits + state:3 bits` atomic word를 사용한다. stale/중복/범위 밖 generation을 거절하고 마지막 generation 후 슬롯을 Retired로 보존한다. `Loaned→Queued→InUse→ReturnPending→Free`의 Release/Acquire 관계와 UnsafeCell 접근 증명은 `pool/storage.rs`에 기록했다.
- 반환 알림은 capacity 1의 coalescing atomic mailbox다. 포화 시에도 각 슬롯의 ReturnPending이 유지된다. 일반 소비자의 `reclaim_raw(limit)`는 알림과 관계없이 고정 개수만 순환 스캔해 정확 1회 회수한다. RT 대여는 최대 2×슬롯 수의 유한 탐색·후보별 CAS 1회이며 재시도 loop·OS 깨우기·할당/해제·사용자 callback을 하지 않는다. 실제 p99/장비 지연 목표는 미검증이다.
- **unsafe 경계:** workspace forbid 유지. `dot-logger` package만 deny로 설정하고 private `pool/storage`에만 allow를 둔다. UnsafeCell dereference 3곳과 Slot의 Sync 구현을 이 모듈에 한정한다. 기존 공개 수명·실패 의미를 구현하는 내부 세부이며, 이 예외도 Human 구현 리뷰 대상이다.

검증: 원본 통합 17개, 신규 private 경계 4개, 신규 compile-fail 7개, 원본 allocator guard 1개 Pass. 기존 소형/구성/의존성 회귀도 Pass. 두 Rust 각 16 feature·독립 consumer·scope 예제·allocator guard·clippy·rustdoc, 개발 fmt Pass. Miri nightly-2026-10-26 strict provenance, seeds 0..8(8개)에서 lib 6개·원본 17개 Pass다. 이는 실행한 경로의 해석 검사이며 모든 가능한 동시 실행의 증명은 아니다.

## dot-probe 수명 적합성 검토

현재 `RobotProcess::spawn`·control/safety의 `thread::Builder::spawn`은 'static 소유 값을 넘기며 shutdown은 일반/비동기 bridge에서 join한다. borrowed Producer를 현재 spawn에 직접 넣을 수는 없다. owner를 가진 'static 관리 host 내부에서 scoped RT worker를 띄우고, worker join→drain/reclaim→owner Drop 순서를 보장하는 예제를 두 Rust에서 실행했다. 기존 borrowed 계약을 유지한 수명 모델은 성립하며, 실제 run entry point/생성부 조정과 애플리케이션 검증은 TASK-026이다. dot-probe 파일은 읽기만 했고 첫 적용 완료로 표시하지 않는다(T1 미도래).

상태: **In Progress — Human 리뷰 승인·native CI 대기**. 원본 fanout·완료 추적·파일/durability·실제 shutdown·실장비 수락은 후속 Task다. 변경분 native CI는 6개 OS/Rust 조합 + audit + 신규 raw-memory job의 8개 작업을 게시 후 확인한다. GAP-004는 이 변경분 재검증을 위해 Open이다.

Human 구현 리뷰 승인: 2026-10-27 / 게시·CI 확인 후 오늘 세션 종료 요청. private unsafe 경계와 dot-probe 생성부 조건을 포함한 보고 범위를 승인받았다.

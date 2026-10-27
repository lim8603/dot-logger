# TASK-003 — 소형 RT 수집과 중요 예약

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-002 |
| Story | US-001·005 |
| 요구사항 | FR-002 FR-007 FR-008 FR-009 FR-012 NFR-001 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-27 |
| 다음 액션 | 구현·로컬 검증 완료(EV-002), Human 리뷰 승인·변경분 native CI 대기 |
| 의존 | TASK-002 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: [`crates/dot-logger/src/capture/`](../../../crates/dot-logger/src/capture/) 및 `crates/dot-logger/tests/tc_002_capture.rs`.

첫 실패/리뷰 조건: **Normal 포화 뒤 Important 예약 수락·동일 등급 초과 거절·일반 deadline 만료**.

- [x] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [x] 최소 구현 후 관련 `TC-002`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [x] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] 생산자별 FIFO, 소형 입력 사후 mutation 불영향, 두 대기 정책·레벨/중요도 매핑.
- [x] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [x] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [x] Human 리뷰 완료(2026-10-27 Human 「승인」). 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-002](../../05_verification/test_case.md#tc-002). 실행: 2026-10-27 / [EV-002](../../05_verification/verification_evidence.md#ev-002--task-003-소형-수집예약일반-대기). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 구현 결과와 리뷰 범위 — 2026-10-27

- `CaptureOwner::new`는 검증된 구성에서 **소형 ingress만** 생성한다. input에 payload·봉투·endpoint·rtrb descriptor, control에 owner·채널 metadata를 검사하고 저장소를 예열한다. allocator bookkeeping/반올림은 구성 headroom 대상이며 전체 RSS/RT 지연 보장이 아니다.
- `owner.scope`가 빌린 생산자/소비자 쌍을 제공한다. owner가 양쪽 owning endpoint를 계속 소유하므로 RT 핸들 Drop은 해제하지 않는다. 핸들 탈출·owner 조기 Drop·복제·Sync는 compile-fail로 검증했다.
- 생산자별 metadata/byte SPSC 두 개를 사전 할당한다. bytes 게시 후 envelope 게시, 소비 시 bytes 반환 후 envelope 반환. 안전한 rtrb chunk API만 사용하며 제품 코드는 기존 `unsafe_code = forbid`를 유지한다.
- Normal 미소비 수가 `slots - important_slots`를 넘지 못하게 해 최대 입력 크기 기준 Important slots/bytes를 보호한다. 다른 생산자의 quota는 빌리지 않는다. 같은 등급 포화 시 기존 기록을 유지하고 신규 거절한다.
- 명시 importance → 완전한 채널 레벨 매핑/기본값 → Normal. channel/schema/크기 검증을 필터보다 먼저 수행하고, 수락 때만 sequence를 증가시킨다. `GeneralProducer::record_until`만 시각 확인·재시도하며 close 또는 deadline으로 끝난다.
- `CaptureReceipt`는 ingress 인수만 나타낸다. 전체 `SessionBuilder::build`·원본 풀(TASK-004)·분배(TASK-005)·완료 window/sync/실제 shutdown(TASK-006)·파일/장비 판정은 미구현이다. 일반 대기의 취소 검증은 ingress close 경로이며 미수락 입력은 큐에 남지 않는다.

검증: 수집 통합 16개 + private 경계 2개 + 신규 수명 compile-fail 4개 + allocator guard 1개 Pass. 두 Rust 각 16 feature 조합·독립 공개 consumer·clippy·rustdoc, 개발 fmt Pass. guard 보정에서 실제 할당/해제를 먼저 감지한 뒤 RT 관측 구간 0/0을 확인했다. allocator 관측용 unsafe는 독립 test-only package `tests/rt-allocation`의 좁은 모듈에만 둔다.

상태는 **In Progress — Human 리뷰 승인·native CI 대기**다. TASK-002의 native CI Pass는 이 변경분의 결과가 아니며 GAP-004를 재개했다. Human 리뷰는 2026-10-27 승인됐다. 변경분 native CI 확인 뒤 Done을 판단하고 TASK-004로 이어간다.

Human 리뷰 승인: 2026-10-27 / 코드 `39f9c72` / 다음 native CI 실행·결과 확인.

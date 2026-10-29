# 테스트 전략

> 요구사항·계약을 테스트로 표현하고 구현 중부터 검증한다.

## 문서 정보

| 항목 | 내용 |
|------|------|
| 관련 Intent | INT-001 |
| 관련 Milestone / Task | MS-001~007 / TASK-001~026(계획 승인) |
| 상태 | TDD 개발 방식 Approved — 2026-10-25 Human 「응 그렇게 하는 것으로, 최종 승인」 |
| 승인 범위 | 실패 테스트 → 최소 구현 → 리팩토링. 실제 장비의 통합·성능·전원 차단 검증 병행 |
| 현재 단계 | Build — TASK-006 최신 수정분 Human 승인, TASK-007 RV-006~010 수정·EV-011 로컬 Pass. TASK-007 리뷰/worker/native·장비 검증 잔여 |
| 갱신 | 2026-10-29 / AI |

## 1. TDD 개발 순서

구현할 동작을 승인된 요구사항/수락 기준과 합의된 계약에 연결하고 작은 동작 단위로 다음 순서를 반복한다.

1. **실패 테스트(Red):** 기대 동작과 정상/경계/실패 조건을 먼저 테스트로 표현하고 실행해 의도한 미구현 동작 때문에 실패하는지 확인한다. 환경·문법·의존성 오류를 기대 동작의 실패 증거로 대체하지 않는다.
2. **최소 구현(Green):** 테스트를 통과하는 데 필요한 동작을 구현하고 관련 기존 테스트도 통과하는지 확인한다.
3. **리팩토링(Refactor):** 테스트가 통과하는 상태에서 중복과 구조를 개선하고 영향 범위의 테스트를 다시 실행한다.

버그 수정은 재현 테스트를 먼저 실패시킨 뒤 수정한다. 테스트는 외부에서 관측할 수 있는 결과·상태·자원/소유권 계약을 검증하며 내부 구현을 그대로 따라 적거나 커버리지 숫자만 높이는 테스트를 만들지 않는다. 문서/조사처럼 실행 동작을 바꾸지 않는 작업은 해당 문서/근거 검증을 수행하며 형식적인 단위 테스트를 추가하지 않는다.

이 개발 방식의 승인은 미결 API·오류 전략·추가 AC의 승인이 아니다. Draft 계약은 합의 후 테스트의 기대값으로 사용한다. TDD로 개발하더라도 구현 진입 전 Gate 3의 설계/기술/작업 구조 조건을 충족해야 한다.

## 2. 검증 계층과 역할

| 계층 | 목적·대상 | 실행 시점/조건 |
|------|-----------|----------------|
| 단위/계약 | 수락·거절·생략, 소유권·반환, 예산/포화 판정, 부분 완료·옵션 기본값 등 합의된 관측 동작 | 구현 중 Red → Green → Refactor의 빠른 반복 |
| 통합 | 수집→분배→파일/외부 어댑터·완료/회수, 종료/설정 경합·장애 | 해당 경로 구현 중부터 실제 연동 검증. 대역의 성공만으로 대체하지 않음 |
| 종단 | 기록→완료 확인→조회/재생, 리포트·조절 전후 비교·복구 | 실행 가능한 사용자 경로가 형성되면 검증 |
| 성능/RT·자원 | 전체 호출 지연·처리/저장/원격 완료·미처리량·메모리·제어 주기 영향·계측 비용 | 선언한 장비/부하/완료 경계에서 별도 시험 |
| 전원 차단/복구 | FR-018 활성 구성의 중요 파일 기록·동기화와 실제 전원 차단 후 복구 | 신호 발생부터 차단까지 전체 약 3초 조건·UPS/SSD/OS별 검증 |

테스트 작성은 AI가 수행하고 Human이 검토한다. 전략·수락 기준·장비 프로필의 새 결정은 권한 매트릭스를 따른다. 검증 도구는 ADR-009 채택을 따르며 실제 patch/CI 환경은 TASK-002·022에서 고정한다.

## 3. 우선 테스트로 표현할 계약

| 범위 | 연결 요구/Story | 현재 준비 상태 |
|------|---------------|----------------|
| 수락/생략/거절, RT 무대기, 포화 시 실패 관측 | FR-007~009 / NFR-001 / US-001 | ADR-008의 예약/등급·반납 계약 승인. 알고리즘 구현/증명은 TASK-003·004 |
| 원본 수명/거절 반환·최종 회수 | FR-002·012 / US-001, 인터페이스 §3 | ADR-008/인터페이스 §13 승인. 정상/거절/종료 기대값을 TC-002로 검증 |
| 대상별 완료/실패·부분 완료·조회 시간 초과 | FR-003·013 / US-002, 인터페이스 §4 | 토큰/추적/시간 초과·부분 완료 계약 승인, TC-003 실행 전 |
| FR-018 선택 옵션·기본 비활성 | FR-018 / US-002 | 사용 정책·설정/통지 API·AC-6~10 승인, 모의/실물 시험 모두 실행 전 |
| 긴급 기록 순위·기한·장애·복구 | FR-018 / US-002, 인터페이스 §12 | 제어 로직의결정적 시험과 실제 전원 차단 검증을 구분해 준비 |

TC-001~012 검증 묶음과 MS별 요구 추적표를 [테스트 케이스](test_case.md)에 등록했다. TASK-002 구성/의존성(EV-001)·TASK-003 소형 수집(EV-002)은 로컬/native Pass, TASK-004 원본 수집/반환(EV-003)은 로컬 Pass이고 후속 제품/장비 시험은 잔여이며 상세 기대값은 2026-10-26 기준선 승인됐다.

## 4. 실행 결과와 요구사항 추적

구현 Task 또는 작업 로그에는 요구/계약·실행 명령·의도한 실패 확인·구현 후 통과·미실행 범위를 간결하게 남긴다. 모든 반복의 전체 출력이나 실패 로그를 영구 복제할 필요는 없다. 리팩토링 뒤에는 영향받는 기존 경로까지 검증하고 회귀를 확인한다.

Gate 4·5 판단에 사용할 결과는 [검증 근거](verification_evidence.md)의 대상 revision·환경·유효 범위 규칙에 따라 [테스트 케이스](test_case.md)와 연결한다. 단위 테스트 통과를 실제 RT 지연·동기화/장치 내구성·OS 호환·UPS 복구의 통과로 확대하지 않는다. 장비/경로가 없어서 실행하지 못한 항목은 미검증으로 남긴다.

## 5. 환경·시간 제어·판정 기준

- 빠른 계약 시험은 필요한 경우 제어 가능한 시각·명시적 장애 주입·제한된 자원 대역을 사용해 포화/시간 초과를 재현한다. 시험 대역의 결과와 실제 OS/장치 결과를 구분한다.
- 실제 검증은 Linux 일반/PREEMPT_RT·macOS·Windows의 해당 장비/프로필에서 수행한다. 교차 빌드나 한 OS의 통과를 다른 OS의 실행 근거로 사용하지 않는다.
- FR-018은 기본 비활성 경로를 확인하고 활성 구성에서는 중요 파일/보호량·전달 지연·진행 중 IO·쓰기/영속 동기화·전원 차단/재기동을 함께 확인한다. 제어 가능한 시각의 모의 시험만으로 전체 약 3초 조건을 달성했다고 판정하지 않는다.
- 커버리지 수치·환경/CPU/MSRV·계측 비용/오차·최종 수락 수치와 도구는 OQ-002~005의 보완 계획에 따라 구현/검증 전에 확정한다. 임의의 커버리지 목표나 전 OS 성능 보장을 만들지 않는다.

## 6. 관련 기준과 승인 근거

- Human의 2026-10-25 TDD 제안 최종 승인. [세션 로그 #002](../members/lim8603/workspace/session_logs/session_2026-10-25_002.md)에 승인 문맥을 기록한다.
- [요구사항 §15~17](../02_project_definition/requirement_spec.md#154-미결을-설계에서-확정할-보완-계획), [Story 목록](../02_project_definition/user_story_registry.md), [인터페이스 계약](../03_design_artifacts/interface_contract.md).
- [코딩 규칙](../04_implementation/coding_convention.md), [리뷰 체크리스트](../04_implementation/review_checklist.md), [품질 게이트](quality_gate.md).

## 7. 상세 검증 실행안 — 2026-10-26 Approved

TC-001~012와 TASK-001~026을 연결했다. 수치/케이스의 추가 기대값은 ADR-008·009와 함께 승인됐다. 기본 계측 비용 ≤5%, 안정 예측(5초 이상 구간) 오차 ≤20%는 승인된 시험 목표이며 실제 달성은 미검증이다.

성능 시험은 프로필별 10초 이상 예열 후 독립 3회 측정을 제안한다. P3는 매회 60초 이상·RAM 2배 이상을 모두 만족하고 마지막 동기화 완료까지 시간에 포함한다. 호출 latency histogram/최대값·raw counts를 기록하고 반복별 percentile을 평균내 하나의 p99로 표시하지 않는다. 정상 성공·거절·재활용·포화 경로와 계측 on/off를 구분한다. 입력 생성 누락과 의도적 생략은 파일 완료 bytes에 섞지 않는다.

P6 24시간은 정량적으로 선언한 배포 부하 1회 이상, 장애 주입별 정상 구간/복구 구간을 구분한다. steady-state RSS·관리 할당·큐/미완료·index peak를 기록하고 allocator high-water와 실제 누수를 구분한다. 허용 상한은 구성 예산이며 부족한 장비에 1 GB/s를 강제하지 않는다. RT deadline miss는 같은 제어 작업·CPU/IRQ/커널 설정의 logger off/on 기준 실행을 대조한다.

OS 기능 행은 Linux 일반, Linux PREEMPT_RT, Windows, macOS로 구분하고 생성 OS × 읽기 OS의 파일 교환을 포함한다. 지원 CPU/버전과 도구chain·DB version은 ADR-009 후보와 TASK-022 실제 환경으로 고정한다. 실제 UPS 시험은 신호 원점·전달 지연·보호 bytes·진행 IO·차단/재기동·외부 oracle을 함께 기록한다.

TASK-002에서 프로젝트 toolchain 1.96.0과 MSRV 1.85.0의 로컬 실행을 확인했다(EV-001). 전역 rustup default는 필요하지 않다. TASK-003까지 hosted native CI Pass 이력은 있으나 TASK-004 변경분 native CI·실장비 미확정은 계속 검증 공백이며, Ubuntu 26.04.1 로컬 Pass를 다른 OS·PREEMPT_RT·실물 장비 Pass로 확장하지 않는다. 나머지 구현도 TDD로 진행한다.


TASK-004는 ADR-009의 Miri 선택 검증을 nightly-2026-10-26으로 구체화했다. `MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-many-seeds=0..8" cargo +nightly-2026-10-26 miri test -p dot-logger --lib --test tc_002_pool --locked`로 private 경계/원본 통합을 검사한다. 해석한 seed/입력 밖의 모든 동시 실행이나 soundness를 증명한 것으로 표시하지 않는다. 생산자별 원본/소형 FIFO·거절/생략 동일 loan·포화된 반환 알림·한정 스캔·읽는 중 재사용 금지·관리 host 수명 예제는 EV-003으로 추적한다. 새 native CI에는 기존 6개 OS/Rust·audit 외에 raw-memory job을 포함한다.

2026-10-27 TASK-004 게시 후 [native CI](https://github.com/lim8603/dot-logger/actions/runs/40520558735)는 계정 결제 실패 또는 spending limit 문제로 8개 job 모두 실행 전에 차단됐다. Human 리뷰 승인·로컬 Pass는 유지하고 native 제품 시험은 Not Run으로 기록한다. 계정 조건 해결 후 동일 source에서 8개 job을 재실행한다.


TASK-005의 TC-003 하위 결과는 [EV-004](verification_evidence.md#ev-004--task-005-분배출력-격리예산)다. 정지한 필수 A/정상 B의 격리, record/byte 공정성·FIFO·0-byte, 대상별 queued/pending/진행 IO slots/bytes, shared 한도·원본 최종 guard와 회수, event buffer 포화·retry/expiry/cancel/close, startup exact-fit/1-byte 부족/overflow를 검증했다. 독립 consumer로 capture→routing→target guard를 실행했다. allocator 시험은 구성/Router 생성부터 256회 포화/512 records·최종 해제의 요청 bytes를 계측하며 allocator 내부/RSS·실물 IO 비용의 증거로 확대하지 않는다. 기존 RT guard는 계속 0/0이다. `--test tc_003_routing`의 Miri8 seeds를 추가했고 CI recipe도 같은 대상을 포함한다. Human 확인 원인은 사용량 전량 소진이며 native는 Not Run으로 유지한다. 계정 재확인/차단 재시도는 하지 않고, CI 실행 여건 확보 뒤 해당 revision을 고정해 검증한다. TASK-005 착수 지시는 선행 CI 대기 중 개발 진행 예외이며 검증 면제가 아니다.


TASK-006 [EV-005](verification_evidence.md#ev-005--task-006-완료-추적명시-sync종료): tracked 수락→분배→대상별 Written/오류→명시 sync 대역 확인→deadline shutdown을 독립 consumer로 실행했다. tracking slot은 입력 수락 전 예약하고, query는 불변 producer range/page/permit·이력 age/Expired·timeout/cancel을 관측한다. 실제 sync는 query가 아니라 writer guard의 acknowledgment로 분리한다. close/record 경합·loan/진행 IO·피드백/반납이 남는 Incomplete 후 같은 owner로 cleanup을 시험했다. 기존 RT guard에 tracked 수락/추적 포화/raw 거절/skip/마지막 핸들 guard를 추가(0/0)했고 query/sync128회 초기 생성~종료의 요청 bytes를 계측했다. Miri 범위를 lib8+raw17+routing19+completion12+control14, 총70×8 seeds로 확장했다. 실제 파일/OS primitive·backend worker/thread join·장비 latency/durability·runtime 설정 변경·native/fresh audit는 이 근거에 포함하지 않는다. CI 사용량 소진은 유지하며 계정 재확인/차단 재시도를 하지 않는다.

TASK-006 최신 수정 근거는 [EV-009](verification_evidence.md#ev-009)다. RV-005 일반 sync 게시/종료 경합의 결정적 private3개(실제 IO 종료 후 Closed·게시 중 deadline·검증 오류 반환/표시 해제)를 추가하고, 기존 EV-007 RV-001~004 회귀를 유지했다. 스레드별 cfg(test) scheduler로 검사 직후 중단만 제어하고 실제 공개 API를 두 scoped thread에서 호출한다. 진입 표시는 단발 CAS와 SeqCst 순서화이며 RT 경로는 유지한다. request/next_sync/poll/snapshot/release·일반/전용 shutdown·deadline/IO/loan/feedback·독립 consumer/예제·exact-fit 예산/allocator를 종단 회귀했다. 두 Rust 각16 features·clippy/rustdoc/fmt·RT0/0·Miri81×8(648) Pass, source38개를 고정했다. 최종 Human 리뷰/native·장비 검증은 별개이며 TC-003 전체 완료로 승격하지 않는다.

TASK-007 [EV-010](verification_evidence.md#ev-010): 프로토콜/소유권 반례 Red 후 수정, 실제 파일과 독립 scanner/protoc golden·자원 계측을 결합했다. File::sync_all 이후에만 guard를 확인하고 footer·완전 chunk safe offset·다른 세그먼트 coverage·오류 관측을 분리한다. TC-004 하위13개·두 Rust 기능 조합/독립 파일 consumer·기존 RT/메모리 회귀를 검증했다. Miri의 새 범위는 봉투2개뿐이며 파일 IO/8MiB/독립 CRC는 일반 실행이다. 제품 압축/회전·복구·OS 교환·실물 장비 보장을 이 근거에 포함하지 않는다.


TASK-007 리뷰 수정 [EV-011](verification_evidence.md#ev-011)은 배치 Written/진행 prefix·pre-IO SyncWork 반환·공통 Busy 매핑·유한 쓰기 버퍼·report-only 재시도를 검증한다. 실제 File write 호출과 독립 파일 바이트를 대조하고 allocator는128채널4096레코드의 sparse/dense 인덱스를 포함한다. private metadata contention 시험2개는 메모리 sink로 active/finished sync와 단일/배치 report Busy를 결정적으로 재현하며 현재 core83×8 Miri에 포함한다. query permit 포화를 coverage Busy 원인으로 해석하지 않는다. 단일 즉시 Written과 배치 지연 설정은 별개이며 장비 기본값은 TASK-022에서 측정한다.

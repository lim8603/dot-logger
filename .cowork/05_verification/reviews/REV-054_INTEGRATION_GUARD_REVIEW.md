# REV-054 — 연결 레시피·공개 API 회귀 가드 코드리뷰

## 1. 요약정리

| 항목 | 내용 |
|---|---|
| 회차 / 일자 | REV-054 / 2026-11-12 |
| 리뷰어 | Codex AI — 리뷰 전용 세션 |
| 대상 | REV-053 이후 누적 변경 `da908a0..00ecb78`, 시작 작업 트리 clean |
| 범위 | 40파일: REV-053 완료 추적·비 Linux calibration 조치, TASK-029 S1~S4의 공개 API 검사·레시피·README/rustdoc·facade 예제 |
| 종합 판정 | **Conditional — P1 0건·P2 2건·P3 0건** |
| 상태 | **Closed** — RV-134~135 수용·조치 [EV-076](../verification_evidence.md#ev-076) `a536486`, 2026-11-12 Human 「승인」 |

현재 제품 회귀는 통과했지만, 새 NFR-008 가드가 API 계약 변경과 레시피 개념 상한 초과를 놓치는 두 경로를 재현했다. 제품 코드·저장소 테스트는 수정하지 않았다. 기존 TASK-029 수락·EV-072~075의 실행 사실과 native T3 이월은 유지한다.

## 2. 파일 변경표

| 구분 | 파일 | 변경 내용 |
|---|---|---|
| 생성 | [REV-054](REV-054_INTEGRATION_GUARD_REVIEW.md) | 발견·재현·검증 범위와 처리담당 영역 |
| 수정 | [project_state.md](../../06_evolution/project_state.md) | Open 리뷰 등록, 후속 개발 시작점 동기화 |

검토한 핵심 파일은 `scripts/check-integration-surface.py`, `tests/api-surface/logger.txt`, `.github/workflows/ci.yml`, `completion/{mod,tests}.rs`, `calibrate.rs`, `logger/{mod,preset,writer}.rs`, 관련 facade/calibration/CLI 시험, `examples/recipes/*.rs`, `examples/dot_probe_facade.rs`, README와 변경된 모듈 rustdoc이다.

## 3. 클래스(타입) 변경표

**이번 세션의 struct·enum·trait 변경은 없음.** 검토 대상은 `Registry`, `Completion`, `SweepOwner`, `Shutdown`, `FacadeReport`, `Fact`, `Record`, `Channel` 및 Python 검사기 `Surface` 등이다.

## 4. 리뷰 항목 요약표

| ID | 중요도 | 위치·계약 | 문제 요약 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-134 | P2 | `check-integration-surface.py:204~253`, API §20.1 | trait 제약·연관 상수 타입 등 공개 계약 변경이 snapshot에 나타나지 않음 | 계약 필드를 정규화해 직렬화하고 실제 rustdoc JSON 변이로 검증 | 수용 | Closed |
| RV-135 | P2 | `check-integration-surface.py:348~368`, API §18.6·20.2 | 모듈을 거친 import에서 개념이 0개로 계산되어 L1의 7개념 초과가 통과 | import 경로를 해석하거나 지원하지 않는 표기를 명시적으로 거절 | 수용 | Closed |

### RV-134 — 공개 계약 일부가 snapshot에서 빠짐 (P2)

#### [리뷰어]

- **위치:** [Surface.lines](../../../scripts/check-integration-surface.py:204), [trait 연관 항목](../../../scripts/check-integration-surface.py:232), [impl 출력](../../../scripts/check-integration-surface.py:240). 도입 `681acd7`.
- **원인:** trait 선언은 `inner.generics`만 출력하며 `inner.bounds`를 쓰지 않는다. 연관 상수·타입은 종류와 이름만 출력한다. impl은 대상·제네릭 제약을 생략하고 trait 이름 또는 메서드 자체 시그니처만 남긴다. 따라서 같은 이름을 유지한 채 소비자 구현·호출 조건이 바뀌어도 snapshot이 같을 수 있다.
- **실제 rustdoc 재현:** 저장소 밖 최소 crate `pub mod logger { pub trait Record { const SCHEMA: u64; fn encode(&self); } }`를 고정 `nightly-2026-10-26`의 `rustdoc -Z unstable-options --output-format json`으로 추출하고 원본 `Surface`로 직렬화했다. 각각 독립적으로 `Record: 'static`을 추가하거나 `SCHEMA: u64`를 `u32`로 바꿔 다시 추출해도 세 줄이 완전히 같다: `trait logger::Record`, `assoc_const SCHEMA`, `fn encode(&Self)`. 두 경우 모두 차이 감지 **False**다.
- **현재 제품 JSON 대조:** 원본 `Record`의 bounds에 `'static`을 추가한 복사본, `Channel`의 `Copy` impl에 `T: 'static` where 조건을 넣은 복사본도 기존 301줄과 동일하다. 이 두 검사는 JSON 변이이며 실제 제품 소스를 바꾸거나 재컴파일한 결과로 주장하지 않는다.
- **영향:** 예를 들어 공개 `Record`에 `'static`을 추가하면 빌린 데이터를 가진 사용자 메시지의 구현이 깨질 수 있지만 표면 가드가 알리지 않는다. 현재 공개 API가 이미 깨졌다는 지적이 아니라, 이번에 도입한 변경 감지기의 검증 공백이다. 이름 있는 항목 30개 계수 자체는 맞다.
- **처리 제안:** trait 상위 제약·연관 상수/타입의 타입과 제약·impl 대상과 제네릭 조건을 안정적인 경로로 직렬화한다. 지원하지 않는 항목은 조용히 축약하지 않고 실패시킨다. 위 최소 crate를 이용해 계약 변경은 차이가 나고 구현 본문만 바뀔 때는 같은지 검증한다.

#### [처리담당]

- 판정 결과: **수용** (2026-11-12, 처리담당 AI).
- 처리 내용: `Surface.lines`를 계약 요소별 렌더러로 나눴다. trait는 `unsafe`/`auto`·제네릭·상위 제약(`bounds`)·where, 연관 항목은 `const NAME: 타입`(기본값 있으면 ` = ..`)·`type NAME<..>: 제약 where .. = 기본`·메서드는 기본 구현 여부(` { .. }`). trait impl은 `impl<제약> Trait for 대상 where ..`(`unsafe`·부정 impl 포함), 고유 impl은 제네릭·where가 있으면 머리줄 `impl<T: Record + ?Sized> Channel<T>`을 출력한다. where 술어(bound·lifetime·eq, HRTB)와 연관 타입 제약은 JSON 덤프 대신 Rust 표기로 정규화했다. 리뷰 범위 밖이지만 같은 공백이라 struct/union 비공개 필드·enum 숨은 variant 표시(`field .. (private)`, `variant ..`), `#[non_exhaustive]`, type alias·const·static의 대상 타입도 넣었다. 렌더러가 모르는 항목 종류(macro 등)·연관 항목 종류는 축약하지 않고 실패한다. 리뷰어 최소 crate를 `scripts/test_check_integration_surface.py`의 시험 11개(계약 변경 10종은 차이, 본문만 변경은 동일)로 옮겼고 CI `integration-surface` job에서 실행한다. 제품 snapshot 301 → 314줄 재승인(bless).
- 검증 근거(EV·시험·커밋): [EV-076](../verification_evidence.md#ev-076). Red 10/11 → Green 11/11, 제품 JSON 변이 2건(`Record: 'static`, `Copy for Channel` where `T: 'static`) 차이 감지 False → True. 커밋 `a536486`.

### RV-135 — 모듈 import로 레시피 개념 한도를 우회함 (P2)

#### [리뷰어]

- **위치:** [concepts](../../../scripts/check-integration-surface.py:348), [상한 판정](../../../scripts/check-integration-surface.py:430). 도입 `681acd7`.
- **원인:** `use dot_logger::...`와 본문의 `dot_logger::...`만 수집한다. `use dot_logger::logger; use logger::{...};`의 두 번째 import와 `logger::Shutdown` 같은 모듈 상대 경로는 추적하지 않는다. 첫 import는 모듈이라 개념에서 제외되어 결과가 빈 집합이 된다. 지원하지 않는 import 형태를 막는 검사도 없다.
- **재현:** 원본 L1을 임시 디렉터리에 복사한 뒤 import를 아래와 같이 바꾸고 종료 조건만 `shutdown if logger::Shutdown::is_success(&shutdown) => Ok(())`로 바꿨다. 이름으로 도입하는 타입·trait은 기존 6개 + `Shutdown` = **7개**, 연동 줄은 **20줄**이다.

  ```rust
  use dot_logger::logger;
  use logger::{EncodeError, Logger, Preset, Record, Retention, Schema};
  ```

- **관측:** 원본 함수 `concepts()`는 `{}`, `check_recipes(surface, False)`는 `l1_file.rs: 20 lines, 0 concepts []`와 **True**를 반환한다. rustfmt 검사도 통과했다. 별도로 이 변경된 임시 L1을 실제 제품 rlib에 연결해 `rustc +1.96.0 --edition=2024 -Dwarnings`로 컴파일·실행했고 exit **0**이었다. `check_recipes(..., False)`는 실행을 생략한 검사이며, 변이본 실행은 이 별도 명령으로 확인했다.
- **영향:** 유효한 Rust import 정리만으로 개념 수를 숨길 수 있어 NFR-008의 L1 ≤6·L2 ≤10 회귀 감시가 보장되지 않는다. 현재 원본 레시피의 6/7개 계수가 틀렸다는 뜻은 아니다.
- **처리 제안:** 모듈·별칭·중첩 import와 본문 경로를 정규화해 실제 항목을 세거나, 이 가드가 지원하는 표기만 허용하고 나머지는 실패시킨다. 미해석 경로의 `FAIL` 출력도 최종 실패값으로 전파한다. 위 7개념 변이본은 실패하고 원본은 통과하는 경계 검증을 남긴다.

#### [처리담당]

- 판정 결과: **수용** (2026-11-12, 처리담당 AI).
- 처리 내용: `concepts()`가 주석·문자열을 지운 뒤 모든 `use` 트리(중첩·`self`·`as` 별칭·`extern crate dot_logger as ..`)를 고정점까지 풀어 이름 → crate 공개 경로 binding을 만들고, 본문의 `a::b::..` 경로를 binding으로 해석한다. 경로는 공개 트리에서 가장 긴 접두사로 찾고 모듈이 아닌 첫 항목(타입·trait·자유 함수)을 개념으로 센다(`Shutdown::is_success` → `Shutdown`). 같은 항목은 rustdoc id로 한 번만 센다. glob import(`dot_logger::..::*`, binding 모듈의 `*`)는 계수 불가로 실패, 풀리지 않는 `dot_logger` 경로도 실패하며 `check_recipes`가 이 오류를 최종 결과에 전파한다(기존에는 `FAIL:` 출력만 하고 개념 1개로 셌다). 한도 판정은 `limit_verdict()`로 분리했다. 리뷰어 변이본(모듈 import + `logger::Shutdown::is_success`)은 7개념으로 L1 `concepts 7 > 6` 실패, 원본은 6개념 통과를 시험으로 남겼다. 별칭·중첩·crate 별칭·glob·미해석·주석/문자열 무시 시험 포함 7개.
- 검증 근거(EV·시험·커밋): [EV-076](../verification_evidence.md#ev-076). 네 레시피 계수 불변(L1 6·L2 7·L3 7/7). 커밋 `a536486`.

## 5. 검증 및 인계

| 검증 | 결과 |
|---|---|
| `cargo +1.96.0 fmt --all --check` | Pass |
| `cargo +1.96.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass |
| `cargo +1.96.0 test --workspace --all-features --locked` | Pass — 561 passed, 5 ignored(기존 외부 DB 시험) |
| `cargo +1.85.0 test --workspace --all-features --locked` | Pass — 561 passed, 5 ignored |
| `RUSTDOCFLAGS='-D warnings' cargo +1.96.0 doc --workspace --all-features --no-deps --locked` | Pass |
| 같은 strict rustdoc, `-p dot-logger --no-default-features --no-deps --locked` | Pass |
| `python3 scripts/check-integration-surface.py` | 원본 Pass — 30항목·301줄 snapshot, L1 20줄/6개념·L2 40줄/7개념·L3 +5/+3줄, 4개 레시피 실행·README 일치 |
| `dot_probe_facade` 실제 실행 | exit 0, synchronized 2062 records, control/camera 거절 각각 0 |
| Miri `completion::tests` (strict provenance, seeds 0..8) | Pass — 10시험 × 8 seeds. REV-053 정지 순서 회귀·UPDATING 재훑기 포함 |
| RV-134 실제 rustdoc JSON·제품 JSON 변이 진단 | 변경을 놓치는 결과 재현 |
| RV-135 임시 레시피 진단·컴파일·실행 | 7개념을 0개로 계산하는 결과 재현, 변이본 `-Dwarnings` 빌드·실행 성공 |
| 제품 코드·저장소 테스트 변경 | 없음 |
| native·전체 feature 조합 개별 실행·정식 성능/RT·장기/실앱 | 이번 회차 미실행. 기존 T3·TASK-022~024/026 후속 유지 |

완료 추적의 단일 CAS claim은 동시 `reconcile`의 상태 저장을 한 소유자로 제한하고 Drop guard가 claim을 해제한다. 이번 범위에서 추가 RT 할당·해제·blocking 또는 새 메모리 안전성 결함은 확인하지 못했다. 비 Linux calibration 조치의 분기와 기대값은 코드로 검토했으며, 이번 세션에서 cross/native 검증은 다시 실행하지 않았다.

실행 로그·진단 스크립트는 `/tmp/dot-logger-rev054-*`에 있다. `/tmp`의 영구 보존은 전제하지 않으며 핵심 입력·관측값은 위에 기록했다. 새로 도래한 T 항목은 없다.

다음 개발 세션은 RV-134~135에 대해 실패 재현을 저장소 검사로 옮기고 조치한다. 각 처리담당 영역에 판정·변경·EV를 기록해 `Open → In Progress → Resolved → Closed`로 추적한다. TASK-030의 새 설계 결정 4건과 native T3 이월은 별도로 유지한다.

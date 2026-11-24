# Coding Convention

> 코드 작성 규칙 — AI가 생성하는 코드와 Human이 작성하는 코드가 동일한 스타일을 유지하도록
> 명시적인 규칙을 정의한다.

---

## 사용 방법

1. **기술스택 확정 시**: `03_design_artifacts/tech_stack.md`에 기술이 확정되면, AI가 이 문서의 언어/프레임워크별 섹션을 자동으로 구성한다.
   - 확정된 기술에 해당하는 컨벤션 섹션만 존재한다.
   - 프로젝트에서 사용하지 않는 기술의 컨벤션은 포함하지 않는다.
   - 기존에 없는 기술이 선정되면 AI가 모범 사례 기반으로 초안을 제안한다.
2. **AI 자동 적용**: `CLAUDE.md` 또는 `copilot-instructions.md`에서 이 파일을 참조하므로,
   세션 시작 후 AI가 자동으로 해당 컨벤션을 따른다.
3. **추가/수정**: 팀 규칙에 맞게 각 섹션을 자유롭게 수정한다.

---

## 공통 규칙 (모든 언어)

| 항목 | 규칙 |
|------|------|
| 파일 인코딩 | UTF-8 |
| 줄 끝 | LF (Windows 프로젝트는 CRLF) |
| 후행 공백 | 제거 |
| 최종 개행 | 파일 끝 개행 1개 |
| 대화 언어 | 한국어 |
| 코드 / 커밋 언어 | 영문 |

### TDD 개발 규칙

2026-10-25 Human 최종 승인으로 구현은 **실패 테스트 → 최소 구현 → 리팩토링** 순서로 진행한다. 합의된 요구/계약의 관측 동작을 먼저 테스트로 표현하고 의도한 실패를 확인한 뒤 구현한다. 버그 수정은 재현 테스트를 먼저 작성한다. 리팩토링 뒤에는 영향받는 기존 테스트도 통과해야 한다.

구현 세부를 복제하는 테스트나 문서/조사 작업의 형식적인 단위 테스트는 만들지 않는다. 실제 RT 지연·IO 성능·OS 호환·전원 차단 후 복구는 해당 환경의 통합/성능/장비 시험으로 검증한다. 범위·추적·미검증 관리의 기준은 [테스트 전략](../05_verification/test_strategy.md)을 따른다.

### 주석 규칙 (공통)

| 항목 | 규칙 |
|------|------|
| 공개 API | 반드시 문서 주석 작성 |
| 복잡한 로직 | Why 중심 주석 (What은 코드가 설명) |
| TODO | `// TODO(담당자): 설명` |
| FIXME | `// FIXME: 설명 + 재현 조건` |

### Git 컨벤션 (공통)

**Commit Message (Conventional Commits)**
```
<type>(<scope>): <subject>

<body>

<footer>
```
**Types:** `feat` `fix` `refactor` `docs` `test` `chore` `perf` `ci`

**Branch Naming**
```
<type>/<short-description>
예: feat/motion-planner-refactor, fix/velocity-overflow
```

### Git Workflow (팀 프로젝트)

> 아래는 기본 템플릿이다. 프로젝트 초기에 AI가 팀 규모와 프로젝트 성격을 분석하여 적절한 전략을 제안하고, Human 승인 후 확정한다.

**브랜치 전략**

| 브랜치 | 역할 | 머지 대상 |
|---------|------|----------|
| `main` | 안정 릴리즈 | ← `develop` (Quality Gate 통과 후) |
| `develop` | 통합 개발 | ← `feature/*`, `fix/*`, `docs/*` |
| `feature/<설명>` | 기능 개발 | → `develop` |
| `fix/<설명>` | 버그 수정 | → `develop` |
| `docs/<설명>` | 문서 변경 (`.cowork/` 포함) | → `develop` |

> 1인 프로젝트나 소규모 팀에서는 `main` + `feature/*` 단순 구조로 충분하다.

**머지 규칙**

| 대상 | 규칙 |
|------|------|
| feature → develop | PR 필수, 최소 1인 리뷰 (팀 2인 이상 시) |
| develop → main | Quality Gate 통과 + Master 승인 |
| 긴급 수정 (hotfix) | main에서 분기 → main + develop 동시 머지 |

**커밋 분리 원칙**

| 커밋 유형 | 접두사 | 예시 |
|----------|---------|------|
| `.cowork/` 문서 변경 | `docs(cowork):` | `docs(cowork): update project_state for Phase 2` |
| 소스코드 변경 | `feat/fix/refactor:` | `feat(planner): add velocity limit check` |
| 테스트 변경 | `test:` | `test(planner): add boundary condition tests` |

> `.cowork/` 문서 변경과 소스코드 변경은 별도 커밋으로 분리한다.

### .cowork 파일 충돌 방지 및 해결

팀 프로젝트에서 `.cowork/` 파일의 머지 충돌을 최소화하기 위한 규칙이다.

**충돌 방지 설계**

| 파일 | 수정 범위 | 충돌 위험 |
|------|----------|----------|
| `project_state.md` | 세션 종료 시 동기화 | 중간 — push 전 pull 필수 |
| `team_board.md` | 각자 담당 행만 수정 | 낮음 — 행 단위 분리로 충돌 최소 |
| `my_state.md` | 본인만 수정 | 없음 |
| 개인 `session_logs/` | 본인만 생성, git 제외 | 없음 |
| ADR 파일 | 신규 생성 (기존 수정 듬무) | 낮음 |
| `deliverable_plan.md` | 확정 후 변경 드뭄 | 낮음 |
| `requirement_spec.md` 등 공유 문서 | 복수 팀원 수정 가능 | 중간 — docs/ 브랜치 권장 |

**충돌 발생 시 해결 우선순위**

1. **`project_state.md`**: 최신 세션 데이터 우선 — 더 최근에 갱신된 데이터 채택
2. **`team_board.md`**: 각자 담당 행은 해당 팀원의 버전 우선
3. **공유 소스 문서**: 승인된 ADR/결정 기록 우선 → 세션 로그 → 대화 내용
4. **판단 불가 시**: 두 버전 모두 보존하고 Human에게 확인 요청

---

## 언어/프레임워크별 컨벤션

> 이 섹션은 `03_design_artifacts/tech_stack.md`에 기술스택이 확정되면,
> AI가 해당 기술에 맞는 컨벤션을 자동으로 구성한다.
>
> - 확정된 기술에 해당하는 컨벤션 섹션만 생성한다
> - 프로젝트에서 사용하지 않는 기술의 컨벤션은 포함하지 않는다
> - 기존에 없는 기술이 선정되면 AI가 모범 사례 기반으로 초안을 제안한다

### Rust·Cargo — ADR-007~009 승인 기준

- Cargo workspace·edition 2024. MSRV 1.85 후보와 실제 dependency patch/feature 호환은 TASK-002에서 검증하고 고정한다.
- 공개 API에 외부 crate의 내부 wire/runtime 타입을 강제 노출하지 않는다. core와 DB/remote feature 의존을 분리한다.
- RT 호출/거절/Drop 경로의 할당·해제·I/O·blocking·무한 재시도·사용자 콜백을 금지한다. 등록/예열과 최종 owner 해제는 일반 경로다.
- core의 std `Mutex`는 `crate::sync::warm_mutex`로 만든다. Apple 대상의 std `Mutex`는 첫 잠금 때 `pthread_mutex_t`를 할당하므로 생성 시 예열해 운영 중 할당을 막는다. 그 구조가 예약 회계에 들어가면 `crate::sync::MUTEX_BYTES`를 더한다(2026-11-21 Human 「A로 진행」, TASK-036 S5). 예약량을 바꿀 때는 그 양을 밖에서 계산해 예산을 맞추는 사용처(다른 crate·시험)를 함께 찾는다. 사용처가 공식을 베끼지 않도록 코어가 예약량 함수(예: `Reassembler::reservation_bytes`)를 공개하고 사용처는 그것을 쓴다(RV-199 뒤 macOS viewer 회귀, EV-124).
- `unsafe`가 필요한 수명/원자 연산 경계는 최소 모듈로 한정하고 Safety 불변식·메모리 순서·소유권 증명과 실패 테스트를 함께 둔다. 후보 queue의 특성으로 전체 API 안전성을 대체하지 않는다.
- 예상 가능한 오류는 고정 코드/`Result`로 반환하고 RT 경로에서 문자열을 포맷하지 않는다. 일반 I/O 오류에 `unwrap`/`expect`를 사용하지 않는다.
- 형식/계약에 영향 없는 내부 변경은 자율 수행한다. 공개 소유권·오류·완료 의미나 MSRV/기술 계열 변경은 ADR 영향도를 검토한다.
- 구현 Task의 기본 검증은 영향 feature에 맞춘 `cargo test`, `cargo fmt --check`, `cargo clippy`이며 native/장비 검증은 별도 TC/EV로 남긴다. 외부 의존성의 실행 성공과 성능 달성을 혼동하지 않는다.

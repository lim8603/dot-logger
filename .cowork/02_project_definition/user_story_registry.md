# User Story Registry

> User Story 인덱스 — 요구사항 흐름과 구현 추적의 연결 지점을 관리한다

---

## 목적

- User Story의 상태와 우선순위를 짧게 관리한다
- Story 상세는 `user_stories/US-*.md`에서 관리한다
- Intent, Milestone, Task를 연결하는 추적 인덱스로 사용한다

---

## 기록 규칙

- registry에는 `US-000` 같은 더미 ID를 남기지 않는다.
- 항목이 없을 때는 표에 예시 행을 넣지 않고 `현재 등록 User Story 없음`만 남긴다.
- `관련 Intent`, `관련 Milestone`은 실제 연결된 ID만 기록하고, 미연결 항목은 `없음`으로 적거나 비운다.
- `문서 경로`는 상세 Story 문서를 만들었을 때만 채운다.

---

## User Story 목록

| User Story ID | 제목 | 관련 Intent | 관련 Milestone | 우선순위 | 상태 | 문서 경로 | 비고 |
|---------------|------|-------------|----------------|----------|------|----------|------|
| US-001 | RT 제어를 대기시키지 않는 동시 기록 | INT-001 | MS-002·006·007 | Must | Approved | [상세](user_stories/US-001_rt_concurrent_capture.md) | 2026-10-25 Human 「확정」으로 설계 기준선/AC 승인. 구현·성능 검증 전 |
| US-002 | 예산에 맞춘 파일 운영과 완료 상태 확인 | INT-001 | MS-003·006·007 | Must | Approved | [상세](user_stories/US-002_retention_and_recovery.md) | 기존 AC-1~5 승인. FR-018은 선택 옵션·기본 비활성 확정. 활성 시 전체 약 3초 조건, AC-6~10도 2026-10-26 상세 기준선 승인. 구현·검증 전 |
| US-003 | 기록 데이터의 조회·재생과 외부 활용 | INT-001 | MS-004·006·007 | Must | Approved | [상세](user_stories/US-003_query_replay_integrations.md) | 2026-10-25 Human 「확정」으로 설계 기준선/AC 승인. 구현·성능 검증 전 |
| US-004 | 운영 중 한계 확인과 개발자 기록량 조절 | INT-001 | MS-005·007 | Must | Approved | [상세](user_stories/US-004_observability_and_tuning.md) | 2026-10-25 Human 「확정」으로 설계 기준선/AC 승인. 구현·성능 검증 전 |
| US-005 | Linux·macOS·Windows의 공통 기능과 기록 교환 | INT-001 | MS-002·003·004·007 | Must | Approved | [상세](user_stories/US-005_cross_platform_support.md) | ADR-006, 지원 방향 승인. 세부 매트릭스/실측 전 |

- `우선순위`: `Must` / `Should` / `Could` / `Won't`
- `상태`: `Draft` / `Approved` / `Implemented` / `Deferred`

---

## 운영 규칙

- Story가 생기면 `user_stories/US-*.md`를 생성하고 여기에 등록한다.
- Story 상태는 구현 진행과 함께 갱신한다.
- 상세 Acceptance Criteria는 개별 Story 문서에 기록한다.

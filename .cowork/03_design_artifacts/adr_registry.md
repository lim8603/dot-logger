# ADR Registry

> 아키텍처 의사결정 인덱스 — 승인된 결정과 검토 중 결정을 짧게 추적한다

---

## 목적

- ADR의 상태와 제목, 날짜를 빠르게 확인한다
- 상세 결정 내용은 `adrs/ADR-*.md`에서 관리한다
- 기술스택, 설계 변경, 주요 예외 판단의 추적 기준으로 사용한다

---

## 기록 규칙

- registry에는 `ADR-000` 같은 더미 ID를 남기지 않는다.
- 항목이 없을 때는 표에 예시 행을 넣지 않고 `현재 등록 ADR 없음`만 남긴다.
- `관련 Intent`, `관련 Milestone`에는 실제 연결된 ID만 적고, 관계가 없으면 `없음`으로 적거나 비운다.
- 대체되거나 폐기된 ADR도 삭제하지 않되, 상태를 실제 값으로 갱신하고 대체 ADR이 있으면 `비고`에 남긴다.

---

## ADR 목록

| ADR ID | 제목 | 상태 | 날짜 | 관련 Intent | 관련 Milestone | 문서 경로 | 비고 |
|--------|------|------|------|-------------|----------------|----------|------|
| ADR-001 | Rust 재사용 라이브러리와 Linux + PREEMPT_RT 목표 환경 | Superseded | 2026-10-25 | INT-001 | 없음 | [상세](adrs/ADR-001_rust_linux_preempt_rt.md) | ADR-006으로 대체. Rust/실시간 목표 계승, 지원 OS 확장 |
| ADR-002 | 로깅보다 실시간 제어 주기를 우선한다 | Superseded | 2026-10-25 | INT-001 | 없음 | [상세](adrs/ADR-002_rt_logging_never_waits_for_storage.md) | ADR-003으로 대체. Q1·Q2 선택을 기본값으로 명확히 함 |
| ADR-003 | 정책을 옵션으로 제공하고 기본값을 구분한다 | Accepted | 2026-10-25 | INT-001 | 없음 | [상세](adrs/ADR-003_configurable_policies_and_defaults.md) | 무대기·중요 기록 우선·별도 보존 중요도가 기본값. RT 호출은 무대기 유지 |
| ADR-004 | 입력·처리 성능과 저장 예산·정책을 분리한다 | Accepted | 2026-10-25 | INT-001 | 없음 | [상세](adrs/ADR-004_processing_and_storage_budgets.md) | 순간 처리와 영속 저장 별도 평가, 보존 모드는 옵션, SSD 기본 가정 |
| ADR-005 | 초기 설계 기준선과 미결 사항의 설계 이월을 확정한다 | Accepted | 2026-10-25 | INT-001 | 없음 | [상세](adrs/ADR-005_design_baseline_and_deferred_decisions.md) | Human 「확정」. 조건부 목표·AC·문서 계획·이월 승인, Design 진입. 기술 후보 채택/실측과 구분 |
| ADR-006 | Linux·macOS·Windows의 크로스 플랫폼 지원 | Accepted | 2026-10-25 | INT-001 | 없음 | [상세](adrs/ADR-006_cross_platform_linux_macos_windows.md) | ADR-001 대체. 라이브러리/클라이언트 공통 지원, Linux RT 프로필 유지 |
| ADR-007 | 내장형 Ports and Adapters와 제한된 파이프라인 | Accepted | 2026-10-26 | INT-001 | MS-001 | [상세](adrs/ADR-007_embedded_ports_adapters_pipeline.md) | Human 권고안 채택, 기술·세부 계약 별도 |
| ADR-008 | 제한된 소유권·예약·완료와 실패 계약 | Accepted | 2026-10-26 | INT-001 | MS-001 | [상세](adrs/ADR-008_bounded_ownership_completion.md) | Human 기준선 채택·추가 AC 승인, 구현/장비 검증 전 |
| ADR-009 | 파일·외부 연동·도구체인 권고 | Accepted | 2026-10-26 | INT-001 | MS-001 | [상세](adrs/ADR-009_storage_transport_toolchain.md) | Human 기술 기준선 채택, patch/MSRV·장비 호환 검증은 Task에서 수행 |

- `상태`: `Proposed` / `Accepted` / `Deprecated` / `Superseded`
- `날짜`: `YYYY-MM-DD`

---

## 운영 규칙

- 모든 설계 메모를 ADR로 만들지 않는다. `제약`, `비용`, `확장성`, `보안`, `운영 영향` 중 하나라도 장기적이고 되돌리기 어렵거나 두 축 이상이 함께 얽힌 결정만 ADR로 승격한다.
- 위 기준에 못 미치는 소규모 결정은 관련 canonical 문서 또는 세션 로그에 남기고, ADR registry에는 등록하지 않는다.
- ADR 승격이 필요하다고 판단되면 `adrs/ADR-*.md`를 생성하고 이 문서에 등록한다.
- `tech_stack.md`, `domain_model.md`, `interface_contract.md`의 주요 결정은 관련 ADR과 연결한다.
- 폐기된 ADR도 삭제하지 않고 상태를 갱신한다.

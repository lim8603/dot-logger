# Intent Registry

> 프로젝트 Intent 인덱스 — 어떤 Intent가 활성인지, 우선순위와 상태가 무엇인지 관리한다

---

## 목적

- 세션 시작 시 우선 로드한다
- 활성 Intent와 우선순위를 빠르게 확인한다
- 상세 내용은 `intents/INT-*.md`에서 관리한다

---

## 기록 규칙

- registry는 **실제 활성/이력 데이터 인덱스**이며, `INT-000` 같은 더미 ID를 기록하지 않는다.
- 항목이 없을 때는 표에 예시 행을 넣지 않고 `현재 등록 Intent 없음`만 남긴다.
- `문서 경로`는 상세 문서가 있을 때만 채우며, 상세 문서를 만들지 않은 경량 운영에서는 비워둘 수 있다.
- `관련 Milestone`에는 실제 연결된 `MS-*`만 적고, 연결이 없으면 `없음`으로 적는다.

---

## Intent 목록

| Intent ID | 제목 | 상태 | 우선순위 | 현재 초점 Phase | 관련 Milestone | 문서 경로 | 비고 |
|-----------|------|------|----------|-----------------|----------------|----------|------|
| INT-001 | 재사용 가능한 실시간 로보틱스 로깅·텔레메트리 | Approved | Must | Design | MS-001~007 | [상세](intents/INT-001_realtime_robotics_logging.md) | Human이 프로젝트 목표·핵심 가치를 지정. ADR-005 초기 기준선과 ADR-007~009 상세설계·계획 승인. Gate 3 Pass, Design 완료·Build 착수 준비 |

- `상태`: `Draft` / `Approved` / `Superseded` / `Split` / `Closed`
- `우선순위`: `Must` / `Should` / `Could`
- `현재 초점 Phase`: `Define` / `Design` / `Build` / `Verify` / `Evolve` / `Deliver`

---

## 활성 Intent 요약

- 현재 활성 Intent: INT-001
- 다음 검토 대상: 도메인/모듈·자원 경계와 핵심 API 계약 초안. ADR-005의 설계 기준선/이월은 재승인받지 않는다.
- Human 확인 필요 사항: OQ-002~005의 설계 이월/수락 기준. OQ-001은 ADR-003으로 해결됐으므로 다시 묻지 않는다.

---

## 운영 규칙

- 새 Intent가 필요하면 `intents/INT-*.md`를 생성하고 이 문서에 등록한다.
- 활성 상태 변경 시 `project_state.md`와 함께 갱신한다.
- 종료된 Intent도 삭제하지 않고 상태만 갱신한다.

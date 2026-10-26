# Verification Evidence

> 검증 근거 인덱스 — 테스트, 리뷰, NFR, 릴리즈 readiness 근거를 한곳에서 요약하고 연결한다

---

## 목적

이 문서는 Verify 단계에서 생성되는 근거를 **게이트 판정용 canonical 인덱스**로 정리한다.

- 세션 로그나 개별 Task 문서에 흩어진 검증 근거를 빠르게 복원한다
- Gate 4, Gate 5 판정 시 "무엇이 검증되었고 어디에 근거가 있는가"를 한 번에 보여 준다
- 원본 로그, 스크린샷, 외부 리포트를 이 문서에 복제하지 않고 **요약 + 위치 링크**만 유지한다
- TDD처럼 테스트량이 누적되는 프로젝트에서도, 이 문서는 로그 저장소가 아니라 **신뢰 가능한 증거 인덱스**로 유지한다

---

## 문서 정보

| 항목 | 내용 |
|------|------|
| 관련 Intent | 활성 Intent 또는 관련 `INT-*` 참조 |
| 관련 Milestone | 검증 근거가 연결되는 `MS-*` (해당 시) |
| 관련 Test Strategy | `test_strategy.md` |
| 관련 Test Case | `test_case.md` |
| 버전 | 프로젝트 기준 |

---

## 운영 원칙

- 증거의 유효 범위는 **대상 commit/빌드·실행일·환경/구성·시나리오**로 명시한다. 명령/절차와 결과 위치를 연결하고, 실행 주체가 AI인지 Human인지 구분한다. dirty 작업 트리면 변경 범위도 남긴다.
- 전체 테스트 개수나 로컬 Pass를 제품 전체/다른 환경의 Pass로 확장하지 않는다. 설정·권한·배포물 변경은 선언, 소비 코드, 실제 사용 환경까지 이어지는 경로를 확인한다. 예: 설정 scope 변경이면 단일/다중 workspace와 설치 후 reload 경로 중 영향을 받는 것을 검사한다.
- 관련 코드·계약·환경이 달라지면 이전 Pass는 이력으로 보존하고 현재 판정은 재검증 또는 미검증으로 표시한다. 관련 없는 변경 때문에 전체 테스트를 반복하지 않는다.
- CI/수동 검증을 연기하면 **마지막 검증 revision, 미검증 범위, 담당, 재실행 시점/조건**을 `Open Evidence Gaps`에 남긴다. 비용 때문에 연기한 검증도 Pass로 바꾸지 않는다. 공유 상태에는 공백 ID와 링크만 둔다.
- Task 완료·프로젝트 건강 상태·릴리즈 준비 상태를 구분한다. Green은 Gate 통과 증거가 아니다. 게이트는 해당 대상과 필수 범위의 유효한 근거로 판단하고, 수용한 예외는 승인자·이유·후속 조건을 기록한다.
- 검증 결과나 이월 조건이 바뀐 실행 사이클 종료 시 관련 EV/GAP와 현재 Gate 요약을 갱신한다. 과거 Gate 판정은 버전/날짜를 표시해 현재 판정과 구분한다.

- 이 문서는 Verify/Release 판정의 **증거 요약 인덱스**다.
- 상세 절차와 기대 결과는 `test_strategy.md`, `test_case.md`, `review_checklist.md`, 관련 `TASK-*` 문서에서 관리한다.
- 원본 실행 로그, 외부 리포트, 측정 결과 원문은 원래 위치에 두고, 이 문서에는 무엇을 입증하는지와 위치만 적는다.
- 가능하면 Gate 4, Gate 5 항목은 `EV-*` ID로 추적한다.
- 근거가 아직 부족하면 삭제하지 말고 `Open Evidence Gaps`에 남긴다.

---

## 근거 영역 요약

INT-001 / 2026-10-26 / Design 기준선 완료·Gate 3 Pass, 제품 구현 전. 제품 코드와 실행 근거는 없다. 문서 링크/추적 검사는 제품 성능·기능 Pass 증거가 아니다.

## Evidence Index

현재 제품 실행 EV 없음. TC-001~012는 절차 승인/Not Run이다. 실제 실행 뒤 revision·환경·실행자·명령·결과 경로와 함께 EV를 채번한다.

## Gate 판정 메모

Gate 4/5: Not Ready — 구현/실제 환경 검증 전. Gate 3는 quality_gate.md의 설계 판정을 따른다.

## Open Evidence Gaps

| ID | 미검증 범위/마지막 revision | 담당·다음 조건 | 상태 |
|---|---|---|---|
| GAP-001 | 실제 OS/CPU/SSD/제어 주기/NIC/UPS 미정. 실행 근거 없음 | Human/AI, TASK-022 환경 확정 후 TC-011·012 | Open |
| GAP-002 | 전체 제품 코드·TC 실행 없음 | AI, Gate 3 Pass 이후 각 TDD Task와 MS 인수 | Open |
| GAP-003 | rustup default toolchain 미설정, MSRV/patch/native CI 미검증 | AI, TASK-002 toolchain 구성 후 feature/MSRV 검증 | Open |

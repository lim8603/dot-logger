# Milestone 목록과 승인된 실행 계획

- 상위 Intent: INT-001(Approved). 하위 MS 계획 Approved(2026-10-26), MS-001 Done, MS-002 In Progress — TASK-002 Done(구성 구현·Human 리뷰·로컬/native CI Pass, EV-001). TASK-003 Done(구현·Human 리뷰·로컬/native EV-002 Pass), TASK-004 In Progress(원본 풀·EV-003 로컬 Pass·Human 승인·게시 완료, native CI 계정 사유 대기), TASK-005는 구현·EV-004 로컬 Pass·Human 리뷰 승인/native 대기, TASK-006은 완료/제어 구현·EV-005 로컬 Pass·Human 리뷰 승인/native 대기.
- 1인 순차 진행 기준의 상대 일정이다. 시작일·가용 시간·검증 장비가 미정이므로 달력 날짜/납기를 확정하지 않는다.
- 인일은 설계 추정치이며 TASK-004 수명·MCAP 복구·실제 장비 검증 후 재산정한다. 소요 범위에 리뷰/장비 대기·실제 24시간 경과 대기는 별도다.

| MS | 제목 | Phase | 상태 | Task | 종료 결과 | 인일 초안 | 상세 |
|---|---|---|---|---|---|---|---|
| MS-001 | 상세설계 합의 | Design | Done | TASK-001 | 검토 가능한 상세설계·추적표·구현 계획 확정 | 1~2 | [상세](milestones/MS-001.md) |
| MS-002 | 제한된 수집 코어 | Build | In Progress | TASK-002·TASK-003·TASK-004·TASK-005·TASK-006 | RT 수집·원본 수명·fanout·완료를 대역으로 증명 | 8~14 | [상세](milestones/MS-002.md) |
| MS-003 | 파일 영속성과 보존 | Build | Planned | TASK-007·TASK-008·TASK-009 | 다중 파일 기록·회전·이벤트·복구 완성 | 6~9 | [상세](milestones/MS-003.md) |
| MS-004 | 조회·재생·클라이언트 | Build | Planned | TASK-010·TASK-011·TASK-012·TASK-013 | Rust API/CLI로 실제 기록 활용 | 7~12 | [상세](milestones/MS-004.md) |
| MS-005 | 운영 관측·조절·간편 점검 | Build | Planned | TASK-014·TASK-015·TASK-016·TASK-017·TASK-018 | 한계 확인→개발자 조절→전후 비교 완성 | 7~11 (+선택 1~2) | [상세](milestones/MS-005.md) |
| MS-006 | 외부 연동과 긴급 기록 | Build | Planned | TASK-019·TASK-020·TASK-021·TASK-026 | DB·dot-probe 통합 예제·UPS 소프트웨어 경로 완성 | 8~14 | [상세](milestones/MS-006.md) |
| MS-007 | 장비 검증과 인수 | Verify | Planned | TASK-022·TASK-023·TASK-024·TASK-025 | 실측·24시간·실제 차단·OS 교환 근거 확보 | 8~14 | [상세](milestones/MS-007.md) |

필수 합계 **45~76 인일**(주 5일 전담 기준 약 9.0~15.2주), FR-015 선택 작업 1~2 인일 별도. 가용 시간이 적거나 장비/리뷰 대기가 있으면 달력 기간은 늘어난다. 납기 약속이 아니다.

첫 구현 진입점은 **TASK-002 → TASK-003 → TASK-004**다. 원본 수명/메모리 안전성과 작은 종단 기록을 먼저 검증한다. 계측 TASK-014는 코어 이후 조기 구현할 수 있고, 장비 프로필 TASK-022는 설계 합의 직후 수집 가능하다. 각 의존의 단일 기준은 task_registry다.

Must 종료 전에 FR-018 소프트웨어 옵션과 실제 장비 검증을 모두 추적한다. 장비 미확정 상태를 완료로 체크하지 않는다. dot-probe 첫 적용 완료가 확인되면 project_state T1(ROS 2 검토)을 알린다.

2026-10-28 TASK-006: 고정 완료 window·유한 query/history·명시 sync barrier·deadline shutdown coordinator를 구현하고 로컬 검증했다. 실제 파일/worker/장비 보장은 후속이며 MS-002는 native 검증 잔여로 In Progress다.

후속 자가 리뷰 EV-006의 P1 3건과 README 불일치는 EV-007에서 수정·로컬 회귀 Pass했다. 수정분 Human 리뷰와 native/실장비 검증은 남아 MS-002 In Progress를 유지한다.

# Task 목록과 실행 계획

- INT-001 / 2026-10-28 / 계획 Approved(2026-10-26). TASK-001 Done, TASK-002 Done — 구현·Human 리뷰·로컬/native CI 완료(EV-001 이력). TASK-003 Done — 구현·Human 리뷰·로컬/native CI 완료(EV-002). TASK-004 In Progress — 구현·Human 리뷰 승인·게시 완료(EV-003 로컬 Pass); GitHub 계정 결제/한도로 native CI 실행 전 차단.
- 담당 AI는 승인된 실행 담당이며 Human이 H/J 결정·리뷰를 맡는다. solo 모드를 유지한다.
- 2026-10-28 Human: CI 차단 원인은 사용량 전량 소진, 재확인 불필요. TASK-005 착수 지시를 TASK-004 native CI 후속 검증 상태의 개발 진행 예외로 반영한다. TASK-004/GAP-004 검증 상태는 유지한다.
- TASK-006도 2026-10-28 Human 「다음 진행하자」로 착수. 선행 TASK-004/005 구현·로컬 Pass·Human 승인을 근거로 native 후속 검증 상태에서 개발을 진행했다. 완료/제어 EV-005 로컬 Pass·Human 리뷰 승인 이력은 보존한다. 최신 RV-001~005 수정은 EV-009 로컬 Pass·2026-10-29 Human 승인/native 대기다.
- 2026-10-29 Human 「승인」으로 TASK-006 최신 수정분과 native 이월 상태의 TASK-007 진행을 승인했다. TASK-007은 RV-006~010 수정·EV-011 로컬 Pass, 당시 최신 구현 Human 리뷰/native 대기였다. 2026-10-30 최신 구현 Human 승인 완료, native는 잔여다.
- 2026-10-30 Human 「커밋 푸쉬 후, 다음 테스크 진행」으로 TASK-008 EV-013 소스14fd1a4/문서1f2f44d를 게시하고 TASK-009 후속 개발을 진행했다. TASK-008 최종 리뷰·native는 잔여이며 TASK-009는 별도 로컬 구현/EV-014·REV-011로 추적한다.
- 모든 Build Task는 TASK-001의 Gate 3와 관련 ADR/계약 합의가 선행한다. 의존 Task의 완료가 추가 진입 조건이다.
- TDD: 해당 상세 문서의 실패 동작부터 Red → Green → Refactor. 구현/테스트/근거는 서로 별도 체크한다.
- TASK-018은 FR-015 범위 승인이 추가로 필요하며 Must 완료의 필수 의존에 포함하지 않는다.

| Task ID | 제목 | MS | Intent | Story | 담당 | 상태 | 갱신 | 다음 액션 | 의존 | 상세 | 인일 초안 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| TASK-001 | 상세설계 합의와 Gate 3 정렬 | MS-001 | INT-001 | US-001·002·003·004·005 | AI | Done | 2026-10-26 | 없음 — 설계 기준선 승인·Gate 3 Pass | 없음 | [상세](tasks/TASK-001.md) | 1~2 |
| TASK-002 | workspace와 플랫폼·설정 계약 | MS-002 | INT-001 | US-001·005 | AI | Done | 2026-10-27 | 완료 — EV-001·Human 리뷰·native CI Pass | TASK-001 | [상세](tasks/TASK-002.md) | 1~2 |
| TASK-003 | 소형 RT 수집과 중요 예약 | MS-002 | INT-001 | US-001·005 | AI | Done | 2026-10-27 | EV-002 로컬/native CI Pass·Human 리뷰 승인; 다음 TASK-004 | TASK-002 | [상세](tasks/TASK-003.md) | 2~3 |
| TASK-004 | 원본 풀·수명·무해제 반환 | MS-002 | INT-001 | US-001·005 | AI | In Progress | 2026-10-28 | EV-003 로컬 Pass·Human 승인·게시 완료; 사용량 소진에 따른 native 후속 검증 | TASK-003 | [상세](tasks/TASK-004.md) | 2~4 |
| TASK-005 | 분배·출력 격리·예산 | MS-002 | INT-001 | US-001·005 | AI | In Progress | 2026-10-28 | Human 지시로 선행 CI 대기 중 착수; EV-004 구현·로컬 Pass; Human 리뷰 승인/native 대기 | TASK-004 | [상세](tasks/TASK-005.md) | 1~2 |
| TASK-006 | 완료 추적·명시 sync·종료 | MS-002 | INT-001 | US-001·005 | AI | In Progress | 2026-10-29 | EV-009 RV-001~005 Human 승인·로컬 Pass; native 후속 검증 | TASK-005 | [상세](tasks/TASK-006.md) | 2~3 |
| TASK-007 | MCAP 봉투·writer·인덱스 | MS-003 | INT-001 | US-002·005 | AI | In Progress | 2026-10-30 | EV-011 로컬 Pass·Human 승인 완료; native 대기 | TASK-006 | [상세](tasks/TASK-007.md) | 2~3 |
| TASK-008 | 회전·순환·이벤트 보호 | MS-003 | INT-001 | US-002·005 | AI | In Progress | 2026-10-30 | EV-013 로컬 Pass·REV-009/010 Resolved, Human 구현 리뷰 대기; native 후속 | TASK-007 | [상세](tasks/TASK-008.md) | 2~3 |
| TASK-009 | 중단 복구와 I/O 장애 | MS-003 | INT-001 | US-002·005 | AI | In Progress | 2026-10-30 | EV-014/015 로컬 Pass·Human 승인·REV-011/012 Closed → native·장비 후속 | TASK-008 | [상세](tasks/TASK-009.md) | 2~3 |
| TASK-010 | 시간·채널 조회와 active snapshot | MS-004 | INT-001 | US-003·005 | AI | Planned | 2026-10-26 | 쓰는 중 incomplete chunk 비노출·cursor 만료·페이지 bytes/count 이중 제한 | TASK-009 | [상세](tasks/TASK-010.md) | 2~3 |
| TASK-011 | 재생 시간과 취소 제어 | MS-004 | INT-001 | US-003·005 | AI | Planned | 2026-10-26 | seek 뒤 이전 generation 미전송·pause/resume·배속·늦은 소비자 | TASK-010 | [상세](tasks/TASK-011.md) | 1~2 |
| TASK-012 | gRPC 구독·ACK·관리 서버 | MS-004 | INT-001 | US-003·005 | AI | Planned | 2026-10-26 | 느린 구독자·잘못된 조각·ACK 전 disconnect·인증 없는 비-loopback | TASK-010·TASK-006 | [상세](tasks/TASK-012.md) | 2~4 |
| TASK-013 | Rust 클라이언트와 CLI | MS-004 | INT-001 | US-003·005 | AI | Planned | 2026-10-26 | CLI JSON schema·exit code·query→replay 사용자 흐름과 Ctrl-C | TASK-011·TASK-012 | [상세](tasks/TASK-013.md) | 2~3 |
| TASK-014 | 운영 계측과 제한된 집계 | MS-005 | INT-001 | US-004 | AI | Planned | 2026-10-26 | 수락/거절/생략/대상별 결과 합계·counter reset·누락 지표 | TASK-005 | [상세](tasks/TASK-014.md) | 1~2 |
| TASK-015 | 여유·병목·보존 시간 리포트 | MS-005 | INT-001 | US-004 | AI | Planned | 2026-10-26 | 폐기로 안정된 큐 Overloaded·미측정 분모·음수/0 증가율 | TASK-014·TASK-008 | [상세](tasks/TASK-015.md) | 2~3 |
| TASK-016 | revision 설정 변경과 전후 비교 | MS-005 | INT-001 | US-004 | AI | Planned | 2026-10-26 | old plan 처리 중 재변경 Busy·부분 producer 적용·base revision 충돌 | TASK-015·TASK-013 | [상세](tasks/TASK-016.md) | 2~3 |
| TASK-017 | 예산 제한 간편 점검 | MS-005 | INT-001 | US-004 | AI | Planned | 2026-10-26 | dry-run 무부하·쓰기/시간 예산 선도달·수신자 없음·취소 | TASK-015·TASK-013 | [상세](tasks/TASK-017.md) | 2~3 |
| TASK-018 | 질문 프리셋·설정 추천 선택 기능 | MS-005 | INT-001 | US-004 | AI | Planned | 2026-10-26 | unknown hardware와 사용자 고정 설정에서 과신 추천/자동 적용 금지 | TASK-017 | [상세](tasks/TASK-018.md) | 1~2 |
| TASK-019 | InfluxDB 어댑터와 충돌·장애 | MS-006 | INT-001 | US-001·002·003·004 | AI | Planned | 2026-10-26 | 동일 timestamp 충돌·timeout 이후 unknown·부분 batch 오류 | TASK-006·TASK-007 | [상세](tasks/TASK-019.md) | 2~3 |
| TASK-020 | tracing 어댑터와 dot-probe 통합 예제 | MS-006 | INT-001 | US-001·002·003·004 | AI | Planned | 2026-10-26 | 일반 tracing을 RT 경로로 사용하지 않음·화면 client 데이터 흐름 | TASK-013·TASK-016·TASK-019 | [상세](tasks/TASK-020.md) | 2~3 |
| TASK-021 | UPS 긴급 계획·상태·모의 시험 | MS-006 | INT-001 | US-001·002·003·004 | AI | Planned | 2026-10-26 | off에서는 전이/예약 없음·중복 신호로 deadline 연장 없음·종료 경합 | TASK-009·TASK-006 | [상세](tasks/TASK-021.md) | 2~4 |
| TASK-022 | 실제 장비·플랫폼 검증 프로필 | MS-007 | INT-001 | US-001·002·003·004·005 | AI | Planned | 2026-10-26 | 장비/OS/UPS 프로필을 Human과 확정 | TASK-001 | [상세](tasks/TASK-022.md) | 1~2 |
| TASK-023 | P0~P5 성능·RT·자원 검증 | MS-007 | INT-001 | US-001·002·003·004·005 | AI | Planned | 2026-10-26 | P0 기준선 없는 비율·RAM 수락만 잰 처리량을 결과에서 거절 | TASK-026·TASK-017·TASK-021·TASK-022 | [상세](tasks/TASK-023.md) | 3~5 |
| TASK-024 | 24시간·전원 차단·OS 교환 검증 | MS-007 | INT-001 | US-001·002·003·004·005 | AI | Planned | 2026-10-26 | P6 각 장애의 기대 손실/복구 기준과 실제 UPS oracle 준비 | TASK-023 | [상세](tasks/TASK-024.md) | 3~5 |
| TASK-025 | 사용 문서·MS 추적표·인수 정리 | MS-007 | INT-001 | US-001·002·003·004·005 | AI | Planned | 2026-10-26 | 문서 검토: 설치/통합/관측/조절/장애/보존 예제와 실제 기능의 불일치 | TASK-024 | [상세](tasks/TASK-025.md) | 1~2 |
| TASK-026 | dot-probe 실제 첫 적용과 화면 연동 | MS-006 | INT-001 | US-001·002·003·004 | AI | Planned | 2026-10-26 | 실제 앱에서 logger off/on 동일 제어 시나리오·record→query→replay→관측 조절 | TASK-020·TASK-021 | [상세](tasks/TASK-026.md) | 2~4 |

2026-10-30 Human 「승인, TASK-008 진행하자.」: TASK-007 최신 구현 승인·REV-008 Closed, native 이월 상태로 TASK-008 착수.

TASK-008 EV-012 로컬 Pass: 보존 통합15개·두 Rust 각16 features/consumer/예제/lint/doc/fmt·allocator8·Miri776. REV-009 Resolved·최신 구현 Human 리뷰/native·장비는 대기다.

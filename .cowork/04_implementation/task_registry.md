# Task 목록과 실행 계획

- INT-001 / 2026-10-26 / 계획 Approved. TASK-001 Done, TASK-002 In Progress — 구현·로컬 검증 완료(EV-001), Human 리뷰 승인(2026-10-27)·native CI 대기.
- 담당 AI는 승인된 실행 담당이며 Human이 H/J 결정·리뷰를 맡는다. solo 모드를 유지한다.
- 모든 Build Task는 TASK-001의 Gate 3와 관련 ADR/계약 합의가 선행한다. 의존 Task의 완료가 추가 진입 조건이다.
- TDD: 해당 상세 문서의 실패 동작부터 Red → Green → Refactor. 구현/테스트/근거는 서로 별도 체크한다.
- TASK-018은 FR-015 범위 승인이 추가로 필요하며 Must 완료의 필수 의존에 포함하지 않는다.

| Task ID | 제목 | MS | Intent | Story | 담당 | 상태 | 갱신 | 다음 액션 | 의존 | 상세 | 인일 초안 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| TASK-001 | 상세설계 합의와 Gate 3 정렬 | MS-001 | INT-001 | US-001·002·003·004·005 | AI | Done | 2026-10-26 | 없음 — 설계 기준선 승인·Gate 3 Pass | 없음 | [상세](tasks/TASK-001.md) | 1~2 |
| TASK-002 | workspace와 플랫폼·설정 계약 | MS-002 | INT-001 | US-001·005 | AI | In Progress | 2026-10-27 | Human 리뷰 승인·GitHub 연결/게시 후 native CI 확인(EV-001/GAP-004) | TASK-001 | [상세](tasks/TASK-002.md) | 1~2 |
| TASK-003 | 소형 RT 수집과 중요 예약 | MS-002 | INT-001 | US-001·005 | AI | Planned | 2026-10-26 | Normal 포화 뒤 Important 예약 수락·동일 등급 초과 거절·일반 deadline 만료 | TASK-002 | [상세](tasks/TASK-003.md) | 2~3 |
| TASK-004 | 원본 풀·수명·무해제 반환 | MS-002 | INT-001 | US-001·005 | AI | Planned | 2026-10-26 | 거절 loan Drop과 마지막 RT 핸들 Drop에 allocator 호출 0, return 알림 포화에도 전량 회수 | TASK-003 | [상세](tasks/TASK-004.md) | 2~4 |
| TASK-005 | 분배·출력 격리·예산 | MS-002 | INT-001 | US-001·005 | AI | Planned | 2026-10-26 | 파일 A 큐 정지에도 B enqueue 지속, 물리 bytes 중복 계상/누락 없음 | TASK-004 | [상세](tasks/TASK-005.md) | 1~2 |
| TASK-006 | 완료 추적·명시 sync·종료 | MS-002 | INT-001 | US-001·005 | AI | Planned | 2026-10-26 | seq1 pending·seq2 done이면 전체 완료 아님, window 포화 수락 거절 | TASK-005 | [상세](tasks/TASK-006.md) | 2~3 |
| TASK-007 | MCAP 봉투·writer·인덱스 | MS-003 | INT-001 | US-002·005 | AI | Planned | 2026-10-26 | u64 receipt 왕복·벽시계 역행·unknown schema와 8 MiB 단일 record 경계 | TASK-006 | [상세](tasks/TASK-007.md) | 2~3 |
| TASK-008 | 회전·순환·이벤트 보호 | MS-003 | INT-001 | US-002·005 | AI | Planned | 2026-10-26 | bytes/time/count/index 한계별 회전·겹친 이벤트와 읽기 핀으로 삭제 불가 | TASK-007 | [상세](tasks/TASK-008.md) | 2~3 |
| TASK-009 | 중단 복구와 I/O 장애 | MS-003 | INT-001 | US-002·005 | AI | Planned | 2026-10-26 | chunk 중간 절단·CRC 손상·ENOSPC·sync/rename 실패 | TASK-008 | [상세](tasks/TASK-009.md) | 2~3 |
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

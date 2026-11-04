# Task 목록과 실행 계획

- 현재(2026-11-01 #015): TASK-020 S2 통합·설정 비교 예제를 EV-041/REV-040으로 로컬 수락했다. S1/S2 수락, 다음 S3 화면 구독 불연속. TASK-019/020 통합·실앱/환경/native 잔여, TASK-018은 FR-015 별도 승인 대기다.
- INT-001 / 2026-10-28 / 계획 Approved(2026-10-26). TASK-001 Done, TASK-002 Done — 구현·Human 리뷰·로컬/native CI 완료(EV-001 이력). TASK-003 Done — 구현·Human 리뷰·로컬/native CI 완료(EV-002). TASK-004 In Progress — 구현·Human 리뷰 승인·게시 완료(EV-003 로컬 Pass); GitHub 계정 결제/한도로 native CI 실행 전 차단.
- 담당 AI는 승인된 실행 담당이며 Human은 프로젝트 오너다. 2026-10-31 모든 권한/게시 위임에 따라 승인된 목표의 구현·검증·로컬 수락/마감·단위별 커밋/푸시는 AI가 수행·보고한다. 같은 범위의 재승인은 받지 않으며 solo를 유지한다.
- 2026-10-28 Human: CI 차단 원인은 사용량 전량 소진, 재확인 불필요. TASK-005 착수 지시를 TASK-004 native CI 후속 검증 상태의 개발 진행 예외로 반영한다. TASK-004/GAP-004 검증 상태는 유지한다.
- TASK-006도 2026-10-28 Human 「다음 진행하자」로 착수. 선행 TASK-004/005 구현·로컬 Pass·Human 승인을 근거로 native 후속 검증 상태에서 개발을 진행했다. 완료/제어 EV-005 로컬 Pass·Human 리뷰 승인 이력은 보존한다. 최신 RV-001~005 수정은 EV-009 로컬 Pass·2026-10-29 Human 승인/native 대기다.
- 2026-10-29 Human 「승인」으로 TASK-006 최신 수정분과 native 이월 상태의 TASK-007 진행을 승인했다. TASK-007은 RV-006~010 수정·EV-011 로컬 Pass, 당시 최신 구현 Human 리뷰/native 대기였다. 2026-10-30 최신 구현 Human 승인 완료, native는 잔여다.
- 2026-10-30 Human 「커밋 푸쉬 후, 다음 테스크 진행」으로 TASK-008 EV-013 소스14fd1a4/문서1f2f44d를 게시하고 TASK-009 후속 개발을 진행했다. TASK-008 최종 리뷰·native는 잔여이며 TASK-009는 별도 로컬 구현/EV-014·REV-011로 추적한다.
- 모든 Build Task는 TASK-001의 Gate 3와 관련 ADR/계약 합의가 선행한다. 의존 Task의 완료가 추가 진입 조건이다.
- TDD: 해당 상세 문서의 실패 동작부터 Red → Green → Refactor. 구현/테스트/근거는 서로 별도 체크한다.
- TASK-018은 FR-015 범위 승인이 추가로 필요하며 Must 완료의 필수 의존에 포함하지 않는다.

| Task ID | 제목 | MS | Intent | Story | 담당 | 상태 | 갱신 | 다음 액션 | 의존 | 상세 | 인일 초안 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| TASK-001 | 상세설계 합의와 Gate 3 정렬 | MS-001 | INT-001 | US-001·002·003·004·005 | AI | Done | 2026-10-26 | 없음 — 설계 기준선 승인·Gate 3 Pass | 없음 | [상세](tasks/TASK-001.md) | 1~2 |
| TASK-002 | workspace와 플랫폼·설정 계약 | MS-002 | INT-001 | US-001·005 | AI | Done | 2026-10-27 | 완료 — EV-001·Human 리뷰·native CI Pass | TASK-001 | [상세](tasks/TASK-002.md) | 1~2 |
| TASK-003 | 소형 RT 수집과 중요 예약 | MS-002 | INT-001 | US-001·005 | AI | Done | 2026-10-27 | EV-002 로컬/native CI Pass·Human 리뷰 승인; 다음 TASK-004 | TASK-002 | [상세](tasks/TASK-003.md) | 2~3 |
| TASK-004 | 원본 풀·수명·무해제 반환 | MS-002 | INT-001 | US-001·005 | AI | In Progress | 2026-10-31 | 로컬 Pass·Human 승인 유지; native GAP-004는 공개 완료 후(T3) 실행 | TASK-003 | [상세](tasks/TASK-004.md) | 2~4 |
| TASK-005 | 분배·출력 격리·예산 | MS-002 | INT-001 | US-001·005 | AI | In Progress | 2026-10-31 | EV-004 로컬 Pass·Human 승인 유지; native는 공개 완료 후(T3) 실행 | TASK-004 | [상세](tasks/TASK-005.md) | 1~2 |
| TASK-006 | 완료 추적·명시 sync·종료 | MS-002 | INT-001 | US-001·005 | AI | In Progress | 2026-10-31 | EV-009 최신 수정분 로컬 Pass·Human 승인; native는 공개 완료 후(T3) 실행 | TASK-005 | [상세](tasks/TASK-006.md) | 2~3 |
| TASK-007 | MCAP 봉투·writer·인덱스 | MS-003 | INT-001 | US-002·005 | AI | In Progress | 2026-10-31 | EV-011 로컬 Pass·Human 승인·REV-008 Closed; native는 공개 완료 후(T3) 실행 | TASK-006 | [상세](tasks/TASK-007.md) | 2~3 |
| TASK-008 | 회전·순환·이벤트 보호 | MS-003 | INT-001 | US-002·005 | AI | In Progress | 2026-10-31 | Human 구현 승인·현재 PC 실측 준비, native 공개 후(T3) | TASK-007 | [상세](tasks/TASK-008.md) | 2~3 |
| TASK-009 | 중단 복구와 I/O 장애 | MS-003 | INT-001 | US-002·005 | AI | In Progress | 2026-10-31 | EV-014/015 로컬 Pass·Human 승인; 현재 PC 대체 실측·모의 장애, native 공개 후 | TASK-008 | [상세](tasks/TASK-009.md) | 2~3 |
| TASK-010 | 시간·채널 조회와 active snapshot | MS-004 | INT-001 | US-003·005 | AI | In Progress | 2026-10-31 | Human 구현 승인·현재 PC 실측 준비, native 공개 후(T3) | TASK-009 | [상세](tasks/TASK-010.md) | 2~3 |
| TASK-011 | 재생 시간과 취소 제어 | MS-004 | INT-001 | US-003·005 | AI | In Progress | 2026-10-31 | Human 구현 승인·현재 PC 실측 준비, native 공개 후(T3) | TASK-010 | [상세](tasks/TASK-011.md) | 1~2 |
| TASK-012 | gRPC 구독·ACK·관리 서버 | MS-004 | INT-001 | US-003·005 | AI | In Progress | 2026-10-31 | EV-023 수락 유지·REV-022 Closed/EV-026; native/실측 후속 | TASK-010·TASK-006 | [상세](tasks/TASK-012.md) | 2~4 |
| TASK-013 | Rust 클라이언트와 CLI | MS-004 | INT-001 | US-003·005 | AI | In Progress | 2026-10-31 | EV-025 수락 유지·REV-022 Closed/EV-026; native T3·실앱/성능 후속 | TASK-011·TASK-012 | [상세](tasks/TASK-013.md) | 2~3 |
| TASK-014 | 운영 계측과 제한된 집계 | MS-005 | INT-001 | US-004 | AI | In Progress | 2026-10-31 | EV-029 범주·이력 복구 수락/REV-025 Closed; 비용/실측/native 후속 | TASK-005 | [상세](tasks/TASK-014.md) | 1~2 |
| TASK-015 | 여유·병목·보존 시간 리포트 | MS-005 | INT-001 | US-004 | AI | In Progress | 2026-10-31 | EV-029 압력/시도 부하·host 정렬 수락/REV-025 Closed; PC/RT 오차/native·실앱 후속 | TASK-014·TASK-008 | [상세](tasks/TASK-015.md) | 2~3 |
| TASK-016 | revision 설정 변경과 전후 비교 | MS-005 | INT-001 | US-004 | AI | In Progress | 2026-11-03 | EV-030/031 로컬 수락·운영 backend EV-044/REV-044 Closed; 실앱/실측/native 후속 | TASK-015·TASK-013 | [상세](tasks/TASK-016.md) | 2~3 |
| TASK-017 | 예산 제한 간편 점검 | MS-005 | INT-001 | US-004 | AI | In Progress | 2026-11-01 | 고정 routed RunCheck 로컬 구현/기능 검증 EV-035; 지속 앱/화면 TASK-020/026·통합 실측·native(T3) 수락 대기 | TASK-015·TASK-013 | [상세](tasks/TASK-017.md) | 2~3 |
| TASK-018 | 질문 프리셋·설정 추천 선택 기능 | MS-005 | INT-001 | US-004 | AI | Planned | 2026-11-04 | 질문 프리셋 부분만 FR-015 승인 대기(장비 측정 preset은 TASK-028 S2) | TASK-017 | [상세](tasks/TASK-018.md) | 1~2 |
| TASK-019 | InfluxDB 어댑터와 충돌·장애 | MS-006 | INT-001 | US-001·002·003·004 | AI | In Progress | 2026-11-01 | S1/S2/S3 EV-037~039 수락; 통합/환경·native 후속, 다음 후보 TASK-020 | TASK-006·TASK-007 | [상세](tasks/TASK-019.md) | 2~3 |
| TASK-020 | tracing 어댑터와 dot-probe 통합 예제 | MS-006 | INT-001 | US-001·002·003·004 | AI | In Progress | 2026-11-02 | S1~S3 EV-040~042 로컬 수락·REV-041 Closed; 통합 TASK-026/환경·native 대기 | TASK-013·TASK-016·TASK-019 | [상세](tasks/TASK-020.md) | 2~3 |
| TASK-021 | UPS 긴급 계획·상태·모의 시험 | MS-006 | INT-001 | US-001·002·003·004 | AI | In Progress | 2026-11-02 | S1~S3 EV-043 로컬 구현/UPS 모의 수락·REV-042 Closed, native T3 이월; 다음 TASK-026 | TASK-009·TASK-006 | [상세](tasks/TASK-021.md) | 2~4 |
| TASK-022 | 실제 장비·플랫폼 검증 프로필 | MS-007 | INT-001 | US-001·002·003·004·005 | AI | In Progress | 2026-10-31 | local-pc-v1 Approved·선행 harness 준비; RT 부팅/capability 확인 뒤 실행 | TASK-001 | [상세](tasks/TASK-022.md) | 1~2 |
| TASK-023 | P0~P5 성능·RT·자원 검증 | MS-007 | INT-001 | US-001·002·003·004·005 | AI | In Progress | 2026-11-04 | S2 — TASK-027 facade 기본 worker 경로로 logger on 측정(S1 EV-046 완료) | TASK-027·TASK-026·TASK-017·TASK-021·TASK-022 | [상세](tasks/TASK-023.md) | 3~5 |
| TASK-024 | 24시간·전원 차단·OS 교환 검증 | MS-007 | INT-001 | US-001·002·003·004·005 | AI | Planned | 2026-10-31 | 현재 PC24h/장애·UPS 시뮬레이션·native/OS 교환 공개 후 | TASK-023 | [상세](tasks/TASK-024.md) | 3~5 |
| TASK-025 | 사용 문서·MS 추적표·인수 정리 | MS-007 | INT-001 | US-001·002·003·004·005 | AI | Planned | 2026-10-26 | 문서 검토: 설치/통합/관측/조절/장애/보존 예제와 실제 기능의 불일치 | TASK-024 | [상세](tasks/TASK-025.md) | 1~2 |
| TASK-026 | dot-probe 실제 첫 적용과 화면 연동 | MS-006 | INT-001 | US-001·002·003·004 | AI | Planned | 2026-10-26 | 실제 앱에서 logger off/on 동일 제어 시나리오·record→query→replay→관측 조절 | TASK-020·TASK-021 | [상세](tasks/TASK-026.md) | 2~4 |
| TASK-027 | 기본 실행 계층(facade)과 기본 worker | MS-008 | INT-001 | US-001·002·003 | AI | Planned | 2026-11-04 | S1 — 소유 모델 ADR 초안·facade 공개 API(API §18)·NFR-008 수치 제안 | TASK-006·TASK-008·TASK-014·TASK-021 | [상세](tasks/TASK-027.md) | 4~6 |
| TASK-028 | 예산 preset·사이징 도우미·장비 측정 auto preset | MS-008 | INT-001 | US-002·004·005 | AI | Planned | 2026-11-04 | S1 — TASK-027 뒤 상위 부하 입력→예산 결정적 계산·preset | TASK-027·TASK-017·TASK-023 | [상세](tasks/TASK-028.md) | 3~5 |
| TASK-029 | 연결 레시피·문서·접점 회귀 가드 | MS-008 | INT-001 | US-001·003 | AI | Planned | 2026-11-04 | TASK-027·028 S1 뒤 레시피 3단계·quickstart·공개 API 회귀 가드 | TASK-027·TASK-028 | [상세](tasks/TASK-029.md) | 1~2 |
| TASK-030 | MCAP 표준 호환(표준 도구 판독) | MS-009 | INT-001 | US-003·005 | AI | Planned | 2026-11-04 | S1 — Envelope 해제 표준 MCAP export와 독립 reader·Foxglove 계열 판독 | TASK-007·TASK-009·TASK-010·TASK-011·TASK-027 | [상세](tasks/TASK-030.md) | 4~6 |
| TASK-031 | 원격 소비처 dot-logger-viewer와 시각화 bridge | MS-009 | INT-001 | US-003·004 | AI | Planned | 2026-11-04 | S1 — Foxglove WebSocket 프로토콜·SDK·Lichtblick 호환/라이선스 조사와 결정 | TASK-012·TASK-013·TASK-030 | [상세](tasks/TASK-031.md) | 2~3 |
| TASK-032 | viewer 운영 대시보드 | MS-009 | INT-001 | US-004 | AI | Planned | 2026-11-04 | S1 — UI 방식(viewer 내장 웹·native·Foxglove 확장 패널) 결정 | TASK-031·TASK-016·TASK-017 | [상세](tasks/TASK-032.md) | 2~4 |

2026-10-30 Human 「승인, TASK-008 진행하자.」: TASK-007 최신 구현 승인·REV-008 Closed, native 이월 상태로 TASK-008 착수.

TASK-008 EV-012 로컬 Pass: 보존 통합15개·두 Rust 각16 features/consumer/예제/lint/doc/fmt·allocator8·Miri776. REV-009/010은 2026-10-31 Human 승인·Closed, 현재 PC 실측/native 후속은 잔여다.

2026-10-31 Human 승인 ADR-011: 현재 PC 실측을 실제 장치 시험 대체 근거로 수락, UPS 시뮬레이션 판정, GitHub native는 공개 완료 후 진행한다. TASK-022는 환경/수락 프로필 준비로 In Progress이며 현재 generic 실행/RT 설치를 구분했다. TASK-004~011의 실행 근거·Human 리뷰 상태는 개별 유지하며 이번 정책 변경만으로 미실행 시험/Task를 일괄 Pass/Done으로 바꾸지 않는다.

2026-10-31 TASK-013: [EV-025](../05_verification/verification_evidence.md#ev-025) source92개/검사15개·두 Rust각16feature·CLI10/transport7·Miri32 Pass, [REV-021](../05_verification/reviews/REV-021_TASK-013_CLIENT_CLI.md) RV-039~045 조치 및 위임된 AI 로컬 수락 완료다. native/실앱/성능 잔여 때문에 Task/MS 상태는 In Progress를 유지한다. 다음 TASK-014는 이번 요청으로 자동 착수하지 않았다.

2026-10-31 TASK-014: [EV-027](../05_verification/verification_evidence.md#ev-027) source97개/검사14개·Miri56·소스d99ba3b와 [REV-023](../05_verification/reviews/REV-023_TASK-014_TELEMETRY.md) 위임 로컬 수락 완료. MS-005 In Progress, 비용≤5%/RT/PC/native/실앱·TASK-015~017은 후속이며 다음 후보 TASK-015는 미착수다.

2026-10-31 #011 종료: Human 「다음 세션에서 TASK-015 시작하자」로 다음 세션 착수 승인. TASK-015는 Planned/미착수 유지, 선행 TASK-008/014 로컬 수락 근거로 재개하고 native/실측·Gate4/5 미완료는 유지한다.

2026-10-31 #012: TASK-015 리포트·용량 근거·분리된 버퍼/디스크/보존 계산을 EV-028 source100개/검사13개·Miri168·소스a247d01로 위임 로컬 수락했다. [REV-024](../05_verification/reviews/REV-024_TASK-015_REPORT.md) RV-056~059 Closed. 실제 PC/RT/native·실앱 검증과 TASK-016/017은 후속이다.

- 현재(2026-11-01 #008): TASK-017 고정 routed RunCheck·원 source·기존 CLI 권한/취소·실제 exporter host 실행은 EV-035/REV-033. 로컬 기능 구현 잔여 완료, 별도 통합/실측/native 수락 조건은 유지하여 In Progress다.

- 2026-11-04 #001: Human 승인으로 MS-008 TASK-027~029·MS-009 TASK-030~032를 Planned 등록했다. TASK-023 S2는 TASK-027 facade 기본 worker 경로로 측정하도록 의존을 추가했다. FR-015 장비 측정 부분은 TASK-028 S2, 질문 프리셋은 TASK-018에 남는다.

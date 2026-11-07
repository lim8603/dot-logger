# 프로젝트 상태 (Project State)

## 현재 상태 요약

| 항목 | 내용 |
|---|---|
| 프로젝트 | dot-logger — TASK-021 로컬 구현·UPS 시뮬레이션 수락, 지속 실앱 후속 |
| 팀/실행 모드 | lim8603 프로젝트 오너, AI 지원 / solo |
| 현재 Phase | Build — MS-008 In Progress(TASK-027 S1~S3·TASK-028 S1 로컬 완료 EV-047~052), MS-006/007 진행 |
| 개발 방식 | TDD Approved — 실패 테스트 → 최소 구현 → 리팩토링 |
| 활성 Intent | INT-001 Approved |
| 활성 Milestone | MS-008 In Progress — 소비처 통합 접점 경량화; MS-009 Planned; MS-006 In Progress; MS-002~005/007 검증 후속 |
| 활성 Task | TASK-023 S2 — facade·preset 경로 logger on 측정 착수(이월 T4 판정 입력). REV-051 조치 완료(EV-053). TASK-027 S1~S3·TASK-028 S1 로컬 완료 |
| 미처리 리뷰 | 없음 — [REV-051](../05_verification/reviews/REV-051_FACADE_PRESET_PERF_INDEPENDENT_REVIEW.md) Closed(2026-11-07 Human 「승인」, [EV-053](../05_verification/verification_evidence.md#ev-053) `60ea9bb`) |
| 상태 | Yellow — UPS 모의 수락, REV-051 Closed, 성능/RT/장기·운영/실앱·native 후속 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-11-07 / #003 / AI |
| 세션 상태 | 진행 — 2026-11-07 #003 REV-051 Closed(Human 「승인」). 다음 TASK-023 S2 |
| 최근 세션 | [2026-11-07 #003](../members/lim8603/workspace/session_logs/session_2026-11-07_003.md) — REV-051 RV-120~127 조치(EV-053) |

## 현재 결정과 제약

- **2026-11-07 Human 결정 — TASK-028 S1 preset/사이징:** preset 3종(`control`·`sensor`·`black_box`) + `Sizing`, 흡수 1초 × slot 1.25·메모리 +10%(`black_box` 흡수 2초), start 시 계산 + `plan()`. preset은 빈 손실 값만 채우고 사용자 값을 덮어쓰지 않는다. [API §19](../03_design_artifacts/interface_contract.md#19-예산-preset사이징-계약--2026-11-07).

- **2026-11-07 Human 결정 — TASK-027 S3c remote 연결:** ① 구독 기록은 관측용 사본(완료 대상 아님, 포화는 그 구독의 Gap/Disconnect) ② 원격 관리는 상태 조회만(나머지 Unsupported, 이력 조회·재생은 MS-009 TASK-031) ③ 한도는 writer quota 기반 기본값 + `remote_with` 명시. 상세 [API §18.11](../03_design_artifacts/interface_contract.md#1811-s3c-remote-선택-연결--2026-11-07).

- **2026-11-07 Human 결정 — TASK-027 S3d 제외:** 「승인」으로 slot 직접 encode를 S3에서 빼고 scratch encode + byte ring 복사 경로를 유지한다(byte ring 감김·rtrb 0 채우기·unsafe/형식 변경 필요). TASK-023 S2 P1(256B)에서 encode·복사 비용이 의미 있게 나오면 별도 설계한다 — Human 요청으로 이월 백로그 T4에 기록. API §18.3 문구 정정.

- **2026-11-05 Human 결정 — TASK-027 S1 facade 계약 승인:** 리뷰 포인트 5개를 하나씩 검토해 모두 권고안을 선택했다. ① [ADR-012](../03_design_artifacts/adrs/ADR-012_facade_owned_writer_handles.md) Accepted: 공유 `CaptureCore` + 소유 `Writer` + 빌림 `Producer<'_>` 뷰, Writer Drop은 반납함 이동만 하고 host/reaper가 최종 소유(ADR-008 borrowed core 유지). ② 성능 값(IO 1·CPU 고정 없음)은 facade 기본값, 손실 값(sync 주기·종료/Drop 기한·메모리·보존)은 `Budget`/preset 필수. ③ `Logger::drop`은 `drop_deadline` 기한 있는 종료. ④ NFR-008: L1 ≤20줄·≤6개념, L2 ≤40·≤10, L3 +6, facade 공개 ≤60. ⑤ `writer.try_log(channel, &msg)`. 계약은 [API §18](../03_design_artifacts/interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05), 시험은 TC-013 F1~F8.

- **2026-11-04 Human 결정 — 소비처 접점 경량화·시각화 범위 추가:** 사용성 평가(공개 항목 534개·기본 worker 없음·예산 명시 부담·Envelope 저장으로 표준 도구 판독 불가)에 동의하고 「권고대로」·「TASK-032 포함」·「MS 분리 가능」. [MS-008](../04_implementation/milestones/MS-008.md)(TASK-027 facade·기본 worker, TASK-028 preset·auto preset, TASK-029 레시피)와 [MS-009](../04_implementation/milestones/MS-009.md)(TASK-030 MCAP 표준 호환, TASK-031 viewer·Foxglove 계열 bridge, TASK-032 운영 대시보드) 등록. NFR-008 통합 용이성 추가, FR-006 viewer 확장, FR-015 장비 측정 auto preset만 TASK-028 S2로 부분 채택(질문 프리셋은 TASK-018 제안 유지). **순서:** TASK-027 → TASK-028 S1 → TASK-023 S2/S3(facade 경로)·RT 요청 → TASK-028 S2 → TASK-029 → MS-009 → probe 인계 → TASK-026. 11-03 결정의 "단독 작업 먼저, probe 연동 나중" 원칙 안에서 단독 작업 범위를 넓혔다. ADR 후보: facade 소유 모델(TASK-027 S1)·MCAP 표준 저장 모드(TASK-030 S2 전)·시각화 방식(TASK-031 S1).

- **2026-11-03 Human 결정 — probe 연동 전 dot-logger 단독 작업 선행:** 「이 프로젝트에서 할 수 있는걸 다 마치고 probe쪽에서 logger를 연동하는 세션을 진행」. ① 원격 ApplyConfig/Compare 운영 backend·CLI runtime metrics 구현(기존 API§15 범위), ② TASK-023 합성 행(P0 기준선·10ms 제어 off/on·P1~P3·P5·계측 비용)을 TASK-026 전에 generic 커널에서 실행 — 실앱 행은 TASK-026 뒤 유지, ③ 합성 측정 뒤 RT 준비/재시작 요청서를 제출하고 승인 후 RT 행 실행, ④ probe 연동 인계 패키지 작성 후 dot-probe 저장소 세션에서 TASK-026 진행. 24h soak(TASK-024)는 연동 뒤. dot-probe 저장소는 이 세션에서 수정하지 않는다.

- **2026-10-31 Human 승인·권한 위임:** TASK-012 기존 로컬 구현 수락 및 REV-019 개선·검증 후 마감 → TASK-013 착수 승인. 앞으로 승인된 목표의 구현·검증·리뷰 마감/작업 전환 판단과 작업 단위별 커밋·푸시는 AI가 수행·보고한다. 같은 범위 재승인은 받지 않는다. [위임 기준](../01_cowork_protocol/decision_authority_matrix.md#프로젝트-오너의-권한-위임--2026-10-31). 미실행 검증·native 공개 후 조건은 유지한다.

- **TASK-009~022 현재 계약:** 각 Task 문서와 [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안)을 따른다. 미수락 잔여(native T3·실앱·PC/RT 성능·장기)는 각 Task와 GAP-002/004에 유지된다. 이 문서에 있던 요약 원문은 [상태 아카이브](state_archive.md) #023으로 옮겼다.

- **2026-10-26 Human 승인:** 「권고 기준선 채택 — 장비·호환 검증은 계획대로 진행」. 상세설계·ADR-008/009·추가 시험 목표·MS 계획을 채택했다. 검증 이월은 면제가 아니다.

- **ADR-010 Accepted / 2026-10-26 Human 승인:** MCAP 0.26.0 채택. 0.25 선택 zstd 경로의 보안 공지를 해소했다. Tokio는 1.51.6 patch. 최종 audit 취약점/unsound 0, `paste` 유지보수 경고 1(GAP-005).

- Gate 1·2 Pass, ADR-003~006 기준선 유지. 1 GB/s 원본+10만 소형/s·32생산자/16파일/128채널·R1 2 GiB 및 512 MiB/4 GiB 비교는 조건부 목표이며 제품 기본값·달성 결과가 아니다.
- **ADR-007 Accepted:** dot-probe 내장형 Ports and Adapters + 제한된 생산자/소비자 파이프라인. 별도 데몬은 첫 범위에 넣지 않는다. 프로세스 장애까지 격리하지 않음.
- **클라이언트 형태 확정:** Rust API + CLI, dot-probe 기존 화면 연동. 명령/통신 상세도 ADR-009 기준선 채택 완료.
- **실제 장치 대체 환경 확정 — ADR-011 Accepted / 2026-10-31:** Human이 현재 Ubuntu PC 실측을 실제 장치 시험으로 대체 수락하고 실물 UPS 없이 시뮬레이션 결과로 판정하도록 승인했다. native는 GitHub 공개 완료 후 진행한다. **2026-11-02 Human 재확인: 현재는 일반 Ubuntu이며 RT 커널로 실행 중이 아니다**(uname:7.0.0-42-generic). 2026-10-31 RT 패키지 설치 관측은 과거 이력이며 현재 설치/부팅 가능 상태를 보장하지 않는다. RT 검증이 필요해지면 AI가 준비 상태·필요 설치/부팅 전환·재시작·시험 계획을 먼저 정리해 Human에게 요청하고 승인 후 진행한다. 부팅 뒤 실행 커널/CONFIG_PREEMPT_RT/capability를 확인해야 RT 행을 실행한다. 상세 [TASK-022](../04_implementation/tasks/TASK-022.md)·[ADR-011](../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md).
- RT 무대기·별도 중요도·중요 우선 기본값, 보존 모드 옵션과 처리/저장 예산 분리 유지. FR-018은 선택 옵션·기본 off, 전체 약3초는 신호 발생→차단 예산.
- 플랫폼 Tier 1/2 유지. affinity는 간섭 감소이며 완전 격리/보편적 성능 보장 아님. Tier 2 고급 기능 미지원은 capability로 표시한다.
- FR-015는 장비 측정 auto preset만 2026-11-04 TASK-028 S2로 부분 채택(선택·기본 off). 질문 프리셋 TASK-018은 별도 범위 승인 전 착수하지 않고 Must 완료에 몰래 넣지 않는다.

## 활성 Task 요약

| Task | 제목 | 담당 | 상태 | 갱신 | 다음 액션 |
|---|---|---|---|---|---|
| TASK-004 | 원본 풀·수명·무해제 반환 | AI | In Progress | 2026-10-31 | 로컬 Pass·Human 승인 유지; native GAP-004는 공개 완료 후(T3) 실행 |
| TASK-005 | 분배·출력 격리·예산 | AI | In Progress | 2026-10-31 | EV-004 로컬 Pass·Human 승인 유지; native는 공개 완료 후(T3) 실행 |
| TASK-006 | 완료 추적·명시 sync·종료 | AI | In Progress | 2026-10-31 | EV-009 최신 수정분 로컬 Pass·Human 승인; native는 공개 완료 후(T3) 실행 |
| TASK-007 | MCAP 봉투·writer·인덱스 | AI | In Progress | 2026-10-31 | EV-011 로컬 Pass·Human 승인·REV-008 Closed; native는 공개 완료 후(T3) 실행 |
| TASK-008 | 회전·순환·이벤트 보호 | AI | In Progress | 2026-10-31 | EV-012/013 Human 승인·현재 PC 실측, native 공개 후(T3) |
| TASK-009 | 중단 복구·IO 장애 | AI | In Progress | 2026-10-31 | EV-014/015 로컬 Pass·Human 승인; 현재 PC 대체 실측·모의 장애, native 공개 후 |
| TASK-010 | 시간·채널 조회/active snapshot | AI | In Progress | 2026-10-31 | EV-016/017 Human 승인·현재 PC 조회/IO 실측, native 공개 후(T3) |
| TASK-011 | 재생 시간과 취소 제어 | AI | In Progress | 2026-10-31 | EV-020 Human 승인·현재 PC 실제 시계/IO 실측, native 공개 후 |
| TASK-012 | gRPC 구독·ACK·관리 서버 | AI | In Progress | 2026-10-31 | EV-023 수락 유지·REV-022 Closed/EV-026, native/실측 잔여 |
| TASK-013 | Rust 클라이언트와 CLI | AI | In Progress | 2026-10-31 | EV-025 수락 유지·REV-021 Closed·REV-022 Closed/EV-026, native T3·실앱/성능 후속 |
| TASK-014 | 운영 계측·유한 집계 | AI | In Progress | 2026-10-31 | EV-029 거절 범주·이력 복구 조치 수락/REV-025 Closed; 비용≤5%·PC/RT/native 후속 |
| TASK-015 | 여유·병목·보존 시간 리포트 | AI | In Progress | 2026-10-31 | EV-029 압력/시도 부하·host 출력 정렬 조치 수락/REV-025 Closed; PC/RT 오차/native·실앱 후속 |
| TASK-016 | revision 설정 변경과 전후 비교 | AI | In Progress | 2026-11-03 | EV-030/031·운영 backend EV-044/REV-044 Closed; 실앱·PC/RT/native 후속 |
| TASK-017 | 예산 제한 간편 점검 | AI | In Progress | 2026-11-01 | 고정 routed RunCheck EV-035로 로컬 기능 잔여 처리; 지속 앱/화면·통합 실측/native 수락 대기 |
| TASK-019 | InfluxDB 어댑터와 충돌·장애 | AI | In Progress | 2026-11-01 | S1~S3 EV-037~039 로컬 수락; REV-038 Closed·통합/환경/native 후속 |
| TASK-020 | tracing 어댑터와 통합 예제 | AI | In Progress | 2026-11-02 | S1~S3 EV-040~042 로컬 수락·REV-041 Closed; 통합 TASK-026/환경/native 대기 |
| TASK-021 | UPS 긴급 계획·모의 시험 | AI | In Progress | 2026-11-02 | S1~S3 EV-043 로컬 구현/UPS 모의 수락·REV-042 Closed, native T3·통합 후속; 다음 TASK-026 |
| TASK-023 | P0~P5 성능·RT·자원 검증 | AI | In Progress | 2026-11-04 | S2 — TASK-027 facade 기본 worker 경로로 logger on 측정(S1 EV-046 완료) |
| TASK-027 | 기본 실행 계층(facade)과 기본 worker | AI | In Progress | 2026-11-07 | S3a [EV-048](../05_verification/verification_evidence.md#ev-048)·S3b [EV-049](../05_verification/verification_evidence.md#ev-049)·S3c [EV-050](../05_verification/verification_evidence.md#ev-050)·S3e [EV-051](../05_verification/verification_evidence.md#ev-051) 로컬 Pass(F1~F13) — S1~S3 로컬 완료, Human 리뷰 대기; 잔여 TASK-023 성능·TASK-029 계수·T3, S3d는 T4 |
| TASK-022 | 실제 장비·플랫폼 검증 프로필 | AI | In Progress | 2026-10-31 | local-pc-v1 수치 Approved·선행 harness 준비, RT 행 부팅/capability 확인 후 |

## 코드리뷰 현황 (Reviews)

| ID | 범위 | 상태 | 현재 처리/후속 |
|---|---|---|---|
| REV-001~017 | 코어·파일·조회·재생 | Closed | 로컬 조치·Human 수락 완료, native/PC 실측 이월은 유지 |
| REV-018~022 | 원격·client·CLI | Closed | EV-026까지 수락. RV-049→TASK-022, RV-053→TASK-020 |
| REV-023~028 | 계측·리포트·설정변경 자가/독립 리뷰 | Closed | EV-027~031 로컬 수락 마감, 비용·오차·실앱/native 후속 |
| REV-029~041 | 간편 점검·DB·tracing/화면 예제 | Closed | EV-031~042 로컬 수락, 실앱/환경 후속은 각 Task 참조 |
| REV-042~050 | UPS·운영 backend·facade S2~S3·preset 자가/독립 리뷰 | Closed | EV-043~052 로컬 조치·마감(독립 REV-043·045 포함). 원문 [상태 아카이브](state_archive.md) #024 |
| [REV-051](../05_verification/reviews/REV-051_FACADE_PRESET_PERF_INDEPENDENT_REVIEW.md) | GAP-008·TASK-023 S1·TASK-027 S2~S3e·TASK-028 S1 누적 독립 코드리뷰 | Closed | Conditional Pass 8건(P1 1·P2 5·P3 2) 전부 수용·조치 [EV-053](../05_verification/verification_evidence.md#ev-053) `60ea9bb`, 2026-11-07 Human 「승인」 |

이전 회차별 대상·판정·처리 이력은 [리뷰 문서](../05_verification/reviews/)와 [상태 아카이브](state_archive.md)의 #014·#024 이관을 따른다.

## 다음 시작점

1. **단독 작업(2026-11-03 결정, 2026-11-04 MS-008/009로 확장):** ~~운영 backend~~ EV-044·~~REV-045~~·~~GAP-008~~ EV-045·TASK-023 S1 EV-046 완료 → ~~TASK-027 S1~~ 승인 → ~~S2~~ EV-047(`1af172c`·`64c4d23`) → ~~S3a~~ EV-048(`2a805ef`: `health()`·계측·CPU 고정·GAP-009/010) → ~~S3b~~ EV-049(`91387ba`: tracing/log 연결 +2줄) → ~~S3c~~ EV-050(`c284354`: remote 관측 사본·상태 조회) → ~~S3e~~ EV-051(`ad5dadf`: api-consumer·feature matrix) → ~~TASK-028 S1~~ EV-052(`21cc198` 정확 배정·`4465be3` preset/plan) → ~~[REV-051](../05_verification/reviews/REV-051_FACADE_PRESET_PERF_INDEPENDENT_REVIEW.md) 조치~~ EV-053(`60ea9bb`, Closed) → **다음 TASK-023 S2**(S3d slot 직접 encode는 2026-11-07 Human 승인으로 S3 제외, 재설계 조건은 이월 T4; facade 경로 P1 256B·P2 8MiB·P3 혼합, 32생산자/16파일/128채널, P3 혼합은 `StorageWorkload` 확장 필요)/S3(120초×3·P5·계측 비용, P4 배정 확인) → RT 요청/승인 후 RT 행 → TASK-028 S2 auto preset → TASK-029 → MS-009(TASK-030~032) → probe 연동 인계 패키지.
2. **TASK-026 실제 첫 적용·화면 연동(dot-probe 저장소 세션):** 별도 저장소 지침과 기존 composition/화면을 확인하고 logger off/on 동일 제어 시나리오부터 TDD로 진행한다. 아직 미착수다. TASK-020 S1~S3·TASK-021 S1~S3는 로컬 수락, TASK-026은 [진입점](../04_implementation/tasks/TASK-026.md)을 따르며 MS-008/009 뒤 facade 경로로 연동한다. TASK-018은 질문 프리셋 부분만 FR-015 승인 대기다.
3. **UPS 수락 완료/환경 후속:** TC-011 현재 PC ext4의 승인된 모의3초·실제 sync/중단/복구는 EV-043으로 수락했다. 실물 UPS는 필요하지 않다. 지속 앱/화면/off-on·P0~P6/RT/24h·native는 TASK-026/022~024·공개 후 T3다. 첫 적용 전이라 T1 미도래다.
4. 설정/비교 backend·worker, RV-049/TASK-022, PC/RT 비용/오차·장기·native/OS·Gate4/5 미완료. RV-053의 유한 화면 예제는 EV-042로 소비했으나 장기 tail/구독 간 복원은 미수락이다. RT 부팅·T2/T3 조건 유지.

## 승인·재개 제약

기존 구현·검증·수락·작업 전환·작업 단위별 커밋/푸시 위임은 유지한다. 새 범위·공개 전환·선택 기능 자동 착수로 확대하지 않는다. native 공개 후(T3)·PC/RT/장기/실앱·설정/비교·지속 실앱 통합·Gate4/5 미완료 조건은 유지한다. 상세 승인/이전 에디터 진단 정정·과거 재개 메모는 [상태 아카이브](state_archive.md)의 #014 이관과 관련 Task/API를 따른다. 개발1.96.0·MSRV1.85·Miri nightly-2026-10-26 고정, 전역 rustup default/사용자 에디터 설정은 변경하지 않는다.

## 선택 로딩

- [도메인](../03_design_artifacts/domain_model.md), [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안), [데이터](../03_design_artifacts/data_model.md), [CLI](../03_design_artifacts/ui_spec.md).
- [MS 목록](../04_implementation/milestone_registry.md): 9개 MS, [Task 목록](../04_implementation/task_registry.md): 32개 Task(1개 선택). 선행조건·변경 대상·첫 실패/DoD·상대 인일 추정(계획 승인, 납기 확정 아님).
- [요구 추적표](../05_verification/test_case.md#3-요구사항설계구현테스트-추적-체크리스트): INT 1 + FR 18 + NFR 8 + CON 7 = 34행. MS별 구현/테스트 체크·코드/EV 연결. TC-001의 TASK-002 하위 EV-001 로컬/native, TC-002 소형 EV-002 로컬/native·원본 EV-003 로컬, TC-003 분배 EV-004·완료/제어 EV-005 로컬 Pass; 후속/native/장비 시험 잔여.
- [2026-10-26 조사](imported_context/research_2026-10-26_detailed_design.md): 공식 자료와 dot-probe 코드 관측, 기술 후보/사실/미검증 분리.
- 이전 결정 근거가 필요할 때만 ADR-003~006·요구 §12~17·Story 상세·개인 로그를 추가로 읽는다.

## 이월 백로그

다음 T = T5 / 다음 B = B1. 트리거 감시는 AI 책임이다.

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| T1 | ROS 2 파일 호환·토픽 연동 필요/범위 검토 | dot-probe 첫 적용 완료가 확인된 때 | INT-001 / OQ-008 / TASK-026 | 2026-10-25 Human Q5 |
| T2 | 공개 전환 전 라이선스·GitHub 운영정책·공개할 이력/로그 검토 | 공개 전환 또는 첫 외부 배포를 요청할 때 | INT-001 / 공개 준비 | 2026-10-28 Human 논의 |
| T3 | 이월 native 검증 실행·결과 반영 | GitHub 저장소 공개 완료가 확인된 때 | GAP-004 / ADR-011 / TASK-004~017·019~021 | 2026-10-31 Human 검증 시점 결정 |
| T4 | facade 소형 기록 slot 직접 encode 별도 설계(scratch encode + byte ring 복사 대체) | TASK-023 S2 P1(256B) 측정에서 encode·복사 비용이 의미 있게 나온 때 | NFR-001 / API §18.3 / TASK-027 S3d / TASK-023 S2 | 2026-11-07 Human 「승인」·백로그 기록 요청 |

범위 밖 백로그 B: 없음. 현재 T1/T2/T3/T4는 미도래(T4는 TASK-023 S2 P1 결과로 판정). 공개 후 native 실행 시점 지정은 공개 전환 자체 요청이 아니므로 T2를 지금 발동하지 않는다. OQ를 T/B에 중복 등록하지 않는다.

## 열린 결정과 검증 공백

| ID | 상태/현재 제약 | 다음 행동 |
|---|---|---|
| OQ-002 | ADR-011로 현재 PC 실측·UPS 시뮬레이션 수락 확정. local-pc-v1 수치 Approved(EV-021), UPS 하위 EV-043 수락·성능/RT 실측 잔여 | 실앱/성능 harness·RT 부팅 확인 후 현재 PC 성능/TC-012·장기 통합 실행 |
| OQ-003 | 계약/US-002 AC6~10 승인. 장비별 보호량·실제 앱/장비 수명 수락은 미검증 | TASK-004~009·021·022 및 장비 검증 |
| OQ-004 | 구조·클라이언트·형식/DB/통신 채택. patch/MSRV 로컬/native CI 확인(EV-001), 실제 DB·장비 호환 잔여 | TASK-019·022 호환 확인 |
| OQ-005 | 계측 비용 ≤5%·안정 추정 오차 ≤20% 및 점검/변경/시험 계획 승인. FR-015 제안 | 관련 Task 실행 검증, TASK-018은 별도 범위 승인 후 |
| ONB-001 | lim8603·solo 확인 완료, 공식 산출물 생성 세부 미정 | docs 공식 생성 전에만 확인 |
| GAP-001·002·004·005 | PC 환경 확정·프로필/후속 TC 실행 잔여, TASK-004~017·019~021 native 공개 후(T3) 이월·paste 경고. GAP-003 Closed | [검증 근거](../05_verification/verification_evidence.md) 참조 |
| Closed GAP·RV | GAP-006~010·RV-006~035 로컬 조치/수락 완료(native는 GAP-004 이월) | 원문 [상태 아카이브](state_archive.md) #023 |

리스크 RSK-001~007은 [risk_register](../02_project_definition/risk_register.md) 유지. 특히 borrowed API의 실제 앱 수명 적합성, MCAP active reader/인덱스 예산, DB timestamp 충돌, gRPC 복사 비용은 해당 Task에서 검증한다.

## 최근 작업·이력

- 2026-11-07 #003: REV-051 RV-120~127을 TDD로 조치(Writer Drop reaper 경쟁·유휴 spin·시작 실패 정리·오버플로·facade 메모리 예산·IO 실패 분류·공개 표면·perf CLI). 1.96/1.85 각 493 Pass — [EV-053](../05_verification/verification_evidence.md#ev-053), [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-07_003.md).
- 2026-11-07 #002: 직전 독립 리뷰(REV-045) 이후 누적 변경분(`77013e7..49df7d3`, 32파일) 전수 독립 코드리뷰 수행, [REV-051](../05_verification/reviews/REV-051_FACADE_PRESET_PERF_INDEPENDENT_REVIEW.md) Open 등록(Conditional Pass: P1 1·P2 5·P3 2, 제품 코드 무수정) — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-07_002.md).
- 2026-11-07 #001: TASK-027 S3(health·계측·CPU 고정·tracing·remote·api-consumer, EV-048~051)과 TASK-028 S1(정확 배정·preset·plan, EV-052)을 로컬 완료했고 Human 결정 4건(S3d 제외·T4, remote 의미, preset 구성)을 반영했다 — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-07_001.md).
- 2026-11-05 #001: TASK-027 S1 ADR-012·API §18 승인과 S2 facade·기본 worker 구현(EV-047) — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-05_001.md).

2026-11-04 이전 이력과 위 세션의 상세 이력 원문은 [상태 아카이브](state_archive.md) #023·#022, R1/R2 이관 원문은 #020/021을 따른다.

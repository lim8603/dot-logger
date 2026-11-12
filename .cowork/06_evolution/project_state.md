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
| 활성 Task | TASK-030 In Progress — 착수 결정 4건 확정(API §21.1), S1 표준 export 진행. 직전 TASK-029 로컬 완료·승인 |
| 미처리 리뷰 | 없음 — REV-054 Closed(2026-11-12 Human 「승인」) |
| 상태 | Green(로컬) — REV-054 Closed. 가드 자가 시험 18 Pass·가드 Pass, 1.96.0/1.85.0 각 561 Pass(Rust 무변경). TASK-029 기존 승인·장기/실앱/native 후속 유지 |
| 언어 | 대화·작업 문서 한국어. 공식 산출물 언어는 생성 전 확인 |
| 보고 형식 | 요약정리 → 파일 변경표 → 클래스 변경표 → 리뷰 포인트 — [소통 규칙](../01_cowork_protocol/communication_convention.md) |
| 갱신 | 2026-11-12 / REV-054 반입·조치 / AI |
| 세션 상태 | REV-054 Closed, TASK-030 결정 4건 확정. 다음: S1 표준 export |
| 최근 세션 | [2026-11-12 #002](../members/lim8603/workspace/session_logs/session_2026-11-12_002.md) — REV-054 반입·조치 |

## 현재 결정과 제약

- **2026-11-12 Human 결정 — TASK-030 착수 4건(권고안):** MCAP `sequence` = Envelope 하위 32비트, `log_time`·`publish_time` = real, mono·u64 sequence·producer·level·importance·revision·source는 보조 메타 채널 1개(protobuf `RecordMeta`), ROS 2 encoding T1 유지. [API §21.1](../03_design_artifacts/interface_contract.md#211-human-결정--2026-11-12).
- **2026-11-12 Human 결정 — TASK-029 S4 범위:** `dot_probe_pipeline`(InfluxDB·설정 비교)은 facade로 옮기지 않고 고급 경로 예제로 유지, facade 전용 `dot_probe_facade` 예제 추가. DB·설정 변경의 facade 편입은 TASK-026에서 판단([EV-075](../05_verification/verification_evidence.md#ev-075)).
- **2026-11-12 Human 결정 — TASK-029 착수·L1:** NFR-008 「facade 공개 항목」은 이름 있는 항목(타입·trait·자유 함수, variant·메서드 제외)으로 세고, 변경 감지는 rustdoc JSON(nightly-2026-10-26) + 스크립트·snapshot. L1 개념 초과는 `Shutdown::is_success()` 추가로 해소. [API §20](../03_design_artifacts/interface_contract.md#20-task-029-레시피접점-회귀-가드--2026-11-12).
- **2026-11-11 Human 결정 — TASK-028 S2(착수 3건 권고안·「승인」):** 짧은 측정 + 보수 추정(지속 = 순간 × 0.5, 수락 ≤ 1.2 × S3 S), JSON 프로필(feature `calibrate`), 흡수·메모리·부하 경고. 측정 도중 취소 시험 미실시. [API §19.8·19.9](../03_design_artifacts/interface_contract.md#198-task-028-s2-auto-preset-계약--2026-11-11), [EV-071](../05_verification/verification_evidence.md#ev-071).
- **완료된 Human 결정(11-07~11-11: TASK-023 S2·S3 마감·⑪·T5·CRC vendor·rustix·TASK-027 S3c/S3d·TASK-028 S1):** 원문은 [상태 아카이브](state_archive.md) #026, 계약은 API §18~19·ADR-010 개정 1·EV-054~069. T4·T6 이월은 아래 백로그.

- **2026-11-05 Human 결정 — TASK-027 S1 facade 계약:** ADR-012 Accepted(공유 `CaptureCore`·소유 `Writer`·빌림 `Producer<'_>`, Writer Drop은 반납함 이동만), 손실 값 필수·성능 값 기본값, `Logger::drop` 기한 종료, NFR-008 수치(L1 ≤20줄·≤6개념 등), `writer.try_log(channel, &msg)`. [API §18](../03_design_artifacts/interface_contract.md#18-기본-실행-계층facade-계약안--2026-11-05).

- **2026-11-04 Human 결정 — MS-008/009:** 소비처 접점 경량화(TASK-027~029)와 MCAP 표준·viewer·대시보드(TASK-030~032) 등록, NFR-008 추가, FR-015는 auto preset만 TASK-028 S2로 부분 채택. **순서:** TASK-027 → TASK-028 S1 → TASK-023 S2/S3·RT 요청 → TASK-028 S2 → TASK-029 → MS-009 → probe 인계 → TASK-026. [MS-008](../04_implementation/milestones/MS-008.md)·[MS-009](../04_implementation/milestones/MS-009.md).

- **2026-11-03 Human 결정 — probe 연동 전 dot-logger 단독 작업 선행:** 「이 프로젝트에서 할 수 있는걸 다 마치고 probe쪽에서 logger를 연동하는 세션을 진행」. ① 원격 ApplyConfig/Compare 운영 backend·CLI runtime metrics 구현(기존 API§15 범위), ② TASK-023 합성 행(P0 기준선·10ms 제어 off/on·P1~P3·P5·계측 비용)을 TASK-026 전에 generic 커널에서 실행 — 실앱 행은 TASK-026 뒤 유지, ③ 합성 측정 뒤 RT 준비/재시작 요청서를 제출하고 승인 후 RT 행 실행, ④ probe 연동 인계 패키지 작성 후 dot-probe 저장소 세션에서 TASK-026 진행. 24h soak(TASK-024)는 연동 뒤. dot-probe 저장소는 이 세션에서 수정하지 않는다.

- **2026-10-31 Human 승인·권한 위임:** TASK-012 기존 로컬 구현 수락 및 REV-019 개선·검증 후 마감 → TASK-013 착수 승인. 앞으로 승인된 목표의 구현·검증·리뷰 마감/작업 전환 판단과 작업 단위별 커밋·푸시는 AI가 수행·보고한다. 같은 범위 재승인은 받지 않는다. [위임 기준](../01_cowork_protocol/decision_authority_matrix.md#프로젝트-오너의-권한-위임--2026-10-31). 미실행 검증·native 공개 후 조건은 유지한다.

- **TASK-009~022 현재 계약:** 각 Task 문서와 [API §13~17](../03_design_artifacts/interface_contract.md#13-상세-api-설계안)을 따른다. 미수락 잔여(native T3·실앱·PC/RT 성능·장기)는 각 Task와 GAP-002/004에 유지된다. 이 문서에 있던 요약 원문은 [상태 아카이브](state_archive.md) #023으로 옮겼다.

- **2026-10-26 Human 승인:** 「권고 기준선 채택 — 장비·호환 검증은 계획대로 진행」. 상세설계·ADR-008/009·추가 시험 목표·MS 계획을 채택했다. 검증 이월은 면제가 아니다.

- **ADR-010 Accepted / 2026-10-26 Human 승인:** MCAP 0.26.0 채택. 0.25 선택 zstd 경로의 보안 공지를 해소했다. Tokio는 1.51.6 patch. 최종 audit 취약점/unsound 0, `paste` 유지보수 경고 1(GAP-005).

- Gate 1·2 Pass, ADR-003~006 기준선 유지. 1 GB/s 원본+10만 소형/s·32생산자/16파일/128채널·R1 2 GiB 및 512 MiB/4 GiB 비교는 조건부 목표이며 제품 기본값·달성 결과가 아니다.
- **ADR-007 Accepted:** dot-probe 내장형 Ports and Adapters + 제한된 생산자/소비자 파이프라인. 별도 데몬은 첫 범위에 넣지 않는다. 프로세스 장애까지 격리하지 않음.
- **클라이언트 형태 확정:** Rust API + CLI, dot-probe 기존 화면 연동. 명령/통신 상세도 ADR-009 기준선 채택 완료.
- **실제 장치 대체 환경 확정 — ADR-011 Accepted / 2026-10-31:** Human이 현재 Ubuntu PC 실측을 실제 장치 시험으로 대체 수락하고 실물 UPS 없이 시뮬레이션 결과로 판정하도록 승인했다. native는 GitHub 공개 완료 후 진행한다. 기본 부팅은 generic(7.0.0-43). RT 행이 필요하면 AI가 요청서를 내고 Human 승인 뒤 `scripts/rt-boot.sh once`(1회 RT 부팅)·확인·`back` 절차로 진행한다(2026-11-11 실행, EV-070). 상세 [TASK-022](../04_implementation/tasks/TASK-022.md)·[ADR-011](../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md).
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
| TASK-023 | P0~P5 성능·RT·자원 검증 | AI | In Progress | 2026-11-09 | S2 완료(EV-054·055·057·058), S3 도구·⑩ 조치(EV-059), ⑪ 비차단(T5) → 축소 S3 1회차 실행 [EV-064](../05_verification/verification_evidence.md#ev-064): P1·제어·P2 효율 Pass, P3 효율 66% 미달(T5 도래), 후속 [EV-065](../05_verification/verification_evidence.md#ev-065): P3 미달 재현 안 됨(T5 미확정), raw 거절은 풀 사이징, 계측 비용 2.5% |
| TASK-027 | 기본 실행 계층(facade)과 기본 worker | AI | In Progress | 2026-11-07 | S3a [EV-048](../05_verification/verification_evidence.md#ev-048)·S3b [EV-049](../05_verification/verification_evidence.md#ev-049)·S3c [EV-050](../05_verification/verification_evidence.md#ev-050)·S3e [EV-051](../05_verification/verification_evidence.md#ev-051) 로컬 Pass(F1~F13) — S1~S3 로컬 완료, Human 리뷰 대기; 잔여 TASK-023 성능·TASK-029 계수·T3, S3d는 T4 |
| TASK-029 | 연결 레시피·문서·접점 회귀 가드 | AI | In Progress | 2026-11-12 | S1~S4 로컬 완료·Human 승인 [EV-073](../05_verification/verification_evidence.md#ev-073)~[EV-075](../05_verification/verification_evidence.md#ev-075). 잔여 native T3 |
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
| [REV-052](../05_verification/reviews/REV-052_SYNC_BOUNDARY_PERF_REVIEW.md) | REV-051 조치·TASK-023 S2a/S2b 누적 독립 코드리뷰(`49df7d3..93db6e3`) | Closed | Conditional, P1 2·P2 1(RV-128~130) 전부 수용·조치 [EV-056](../05_verification/verification_evidence.md#ev-056) `97e2b12`·`f8f2859`, 정정 `b2899a3`(EV-057). 2026-11-08 Human 「승인」 |
| [REV-053](../05_verification/reviews/REV-053_COMPLETION_CALIBRATION_REVIEW.md) | REV-052 조치·TASK-023 S2/S3·T5·RT harness·TASK-028 S2 누적 리뷰(`93db6e3..8dbd999`) | Closed | Conditional, P1 1·P2 1·P3 1(RV-131~133) 전부 수용·조치 [EV-072](../05_verification/verification_evidence.md#ev-072) `aa62e04` — 훑기 단일 소유 claim, 비 Linux 대체 경로·시험 분리, core-only rustdoc CI. 2026-11-12 Human 「승인」 |
| [REV-054](../05_verification/reviews/REV-054_INTEGRATION_GUARD_REVIEW.md) | REV-053 조치·TASK-029 S1~S4 누적 리뷰(`8dbd999..2fe4e49`) | Closed | Conditional, P2 2건(RV-134~135) 수용·조치 [EV-076](../05_verification/verification_evidence.md#ev-076) `64e9d69` — snapshot 계약 조건(trait 제약·연관 항목 타입·impl 대상/제약) 직렬화, 개념 계수 import 해석·glob/미해석 실패. 2026-11-12 Human 「승인」 |

이전 회차별 대상·판정·처리 이력은 [리뷰 문서](../05_verification/reviews/)와 [상태 아카이브](state_archive.md)의 #014·#024 이관을 따른다.

## 다음 시작점

0. **리뷰 후속:** 없음 — [REV-054](../05_verification/reviews/REV-054_INTEGRATION_GUARD_REVIEW.md) Closed([EV-076](../05_verification/verification_evidence.md#ev-076), 2026-11-12 Human 「승인」). 비 Linux CLI 및 기존 native 검증은 T3다.
1. **단독 작업(2026-11-03 결정, MS-008/009):** TASK-023 S3·T5·RT 행(EV-064~070)과 TASK-028 S2(EV-071) 완료. TASK-029 로컬 완료·Human 승인(EV-073~075). **다음 MS-009 TASK-030**(MCAP 표준 호환) — 착수 시 Task 문서의 Human 확인 4건(MCAP sequence u32↔u64 매핑·`log_time` mono/real·level/importance/producer 보존·ROS 2 encoding T1 유지)부터. S2 native 표준 모드는 ADR 채번·승인 후 → MS-009(TASK-030~032) → probe 연동 인계 패키지. 이전 서사 원문은 [상태 아카이브](state_archive.md) #026.
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

다음 T = T7 / 다음 B = B1. 트리거 감시는 AI 책임이다.

| ID | 항목 | 트리거 | 관련 | 출처 |
|---|---|---|---|---|
| T1 | ROS 2 파일 호환·토픽 연동 필요/범위 검토 | dot-probe 첫 적용 완료가 확인된 때 | INT-001 / OQ-008 / TASK-026 | 2026-10-25 Human Q5 |
| T2 | 공개 전환 전 라이선스·GitHub 운영정책·공개할 이력/로그 검토(vendor `mcap` MIT 전문 고지 보완 포함 — ADR-010 개정 1) | 공개 전환 또는 첫 외부 배포를 요청할 때 | INT-001 / 공개 준비 | 2026-10-28 Human 논의 |
| T3 | 이월 native 검증 실행·결과 반영 | GitHub 저장소 공개 완료가 확인된 때 | GAP-004 / ADR-011 / TASK-004~017·019~021 | 2026-10-31 Human 검증 시점 결정 |
| T4 | facade 소형 기록 slot 직접 encode 별도 설계(scratch encode + byte ring 복사 대체) | TASK-023 S2 P1(256B) 측정에서 encode·복사 비용이 의미 있게 나온 때 | NFR-001 / API §18.3 / TASK-027 S3d / TASK-023 S2 | 2026-11-07 Human 「승인」·백로그 기록 요청 |
| T6 | Envelope payload 복사 제거 설계(헤더·payload 분리 쓰기 — 형식 또는 vendor mcap 쓰기 API 변경). 측정: 복사 1회 −21~24%, 제거 상한 추정 +37~45% | 처리량 요구가 이 PC 기준 약 3 GB/s(logger tmpfs 상한)에 가까워질 때, 또는 Human 요청 시 | NFR-001 / [EV-067](../05_verification/verification_evidence.md#ev-067) / ADR-010 개정 1 | 2026-11-11 Human 「권고대로」 |

범위 밖 백로그 B: 없음. **T5 → 해소(2026-11-11, EV-066~068): 원인 규명, ⑫ 조치 `234ea62`, 무압축 chunk CRC 1회 `942eecf`, 잔여 개선은 T6.** T6 미도래. 트리거 ①은 EV-065에서 재현되지 않음. T1~T4는 미도래(T4: 복사 p50 21 ns = 시계 오버헤드). T4 입력: S2b smoke(측정 아님)에서 256B `try_log` p50 143 ns 중 복사 p50 21 ns(시계 오버헤드와 같은 수준) — 정식 판정은 TASK-023 S3. 공개 후 native 실행 시점 지정은 공개 전환 자체 요청이 아니므로 T2를 지금 발동하지 않는다. OQ를 T/B에 중복 등록하지 않는다.

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

- 2026-11-12 #002: REV-054 반입·조치·Closed([EV-076](../05_verification/verification_evidence.md#ev-076) `64e9d69`) — 가드 snapshot 계약 조건·개념 import 해석, 자가 시험 18개 — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-12_002.md).
- 2026-11-12 리뷰 전용: [REV-054](../05_verification/reviews/REV-054_INTEGRATION_GUARD_REVIEW.md) Open — P2 2건·Conditional, 1.96.0/MSRV 각 561 Pass·완료 추적 Miri 8 seeds, 제품 코드 미수정.
- 2026-11-12 #001: REV-053 조치·Closed([EV-072](../05_verification/verification_evidence.md#ev-072)), TASK-029 S1~S4 로컬 완료·승인(표면 가드·레시피 L1~L3·README/rustdoc·`dot_probe_facade`, [EV-073](../05_verification/verification_evidence.md#ev-073)~[EV-075](../05_verification/verification_evidence.md#ev-075)) — [세션 로그](../members/lim8603/workspace/session_logs/session_2026-11-12_001.md).
- 2026-11-11 리뷰 전용: [REV-053](../05_verification/reviews/REV-053_COMPLETION_CALIBRATION_REVIEW.md) Open — RV-131~133, Conditional(P1 1·P2 1·P3 1). 1.96.0/MSRV 전체 회귀 각각 559 passed·5 ignored, 동시 정리 실패 재현·비 Linux 타입 오류·core-only rustdoc 실패 확인. 제품 코드 미수정.

이전 이력과 상세 원문은 [상태 아카이브](state_archive.md) #027·#026·#025·#023·#022, R1/R2 이관 원문은 #020/021을 따른다.

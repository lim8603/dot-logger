# TASK-019 — InfluxDB 어댑터와 충돌·장애

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-006 |
| Story | US-001·002·003·004 |
| 요구사항 | FR-004 NFR-003 NFR-005 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-01 |
| 다음 액션 | S1 유한 write 로컬 수락; S2 배포 버전 pin·제한 수치 query·실제 DB 대조 |
| 의존 | TASK-006·TASK-007 |
| 예상 소요 | 2~3 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료. TASK-006/007의 로컬 구현·위임 수락을 사용하며 공개 후 native/실측 잔여 상태의 후속 개발 승인을 따른다.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

구현 경로: [`crates/dot-logger/src/adapters/influxdb/`](../../../crates/dot-logger/src/adapters/influxdb/mod.rs), [`tc_008_influxdb.rs`](../../../crates/dot-logger/tests/tc_008_influxdb.rs).

첫 실패/리뷰 조건: **동일 timestamp 충돌·timeout 이후 unknown·부분 batch 오류**.

- [x] S1 기능 부재 compile Red 및 이종 수치 field type 충돌 재현 Red 확인.
- [x] S1 최소 구현 후 TC-008 로컬 TCP 대역 하위 시나리오를 검증했다. 실제 DB/TC 전체는 미완료다.
- [x] S1 영향 회귀·두 Rust·선택 feature/독립 core 소비자·문서 일치 검증(EV-037).

## 완료 기준

- [ ] 실제 선택 DB version write/query 대조·재시도 상한·secret 비노출·파일 영향 계측.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-008](../../05_verification/test_case.md#tc-008). 제품 코드·로컬 시험: [EV-037](../../05_verification/verification_evidence.md#ev-037). TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.


## S1 유한 projection·HTTP write — 2026-11-01 #010

[API §14.6](../../03_design_artifacts/interface_contract.md#146-task-019-s1-influxdb-v2-유한-write--2026-11-01)·[REV-035](../../05_verification/reviews/REV-035_TASK-019_INFLUX_WRITE.md). ADR-009의 v2/선택 feature/일반 경로 방향 안의 구현이다. OQ-004의 실제 DB 호환 미확정은 v2 선택과 모순되지 않으며 S2 검증으로 유지한다. 배포 버전 채택·요구/오류 의미 변경은 이번에 결정하지 않았다.

- 고정 session/measurement/device/채널·typed fields·원 real ns/receipt, 유한 batch/event/재시도/TTL·공유 external 예약.
- 명시 `RejectNonIncreasing`: 호스트가 새 독점 namespace를 공급해야 한다. 같은 series의 equal/older 시각을 생산자/배치 간 거절한다. out-of-order 지원/기존 DB 충돌 탐지는 아직 없다. 원 timestamp 변경이나 recipe/sequence tag 추가 없음.
- 전체 preflight 실패 시 IO/상태 변경 없음. 시도한 batch는 Unknown/취소 뒤에도 high-water 유지. immutable encoded body 재시도·partial/timeout/transport Unknown·204 API ACK.
- token 민감 header·redacted 오류·HTTPS/literal-loopback HTTP·redirect/proxy/암묵 retry off. body와 예약은 실제 마지막 HTTP body owner까지 유지하며 다음 batch는 Busy.
- 신규10개 TCP/입력/예산 시험+1개 body 수명 단위 시험. 실제 DB server·write/query·파일/RT 비용 수락이 아니다.

| 단계 | 상태 | 다음 결과 |
|---|---|---|
| S1 유한 typed projection·HTTP write | 로컬 구현/검증·위임 수락 | EV-037/REV-035 Closed |
| S2 실제 DB 및 제한 수치 query | 미착수 | 승인 계열 v2의 배포 버전 pin·전용 격리 namespace·write/query 대조·서버 partial/type/동일 시각 동작 검증 |
| S3 Router Delivery·완료·파일 격리 | 미착수 | 원 record schema/identity projection·DatabaseAcknowledged/Unknown·원본/예약 수명·실패 DB가 파일 경로에 주는 영향 계측 |
| 통합/환경 수락 | 미완료 | TASK-020/026 실앱·TASK-022 실제 PC·공개 후 native(T3) |

S1은 Task 전체 완료가 아니며 MS-006 In Progress·TC-008 Partial·Gate4/5 Not Ready를 유지한다.

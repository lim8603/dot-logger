# TASK-030 — MCAP 표준 호환(표준 도구 판독)

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-003·005 |
| 요구사항 | FR-003 FR-005 FR-010 NFR-004 NFR-007 NFR-008 |
| 설계 | [데이터](../../03_design_artifacts/data_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) |
| ADR | ADR-009·010 Accepted. [ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Accepted — S2 native 단일 형식·손실·revision 저장 계약 승인(2026-11-16) |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-11-16 |
| 다음 액션 | S2 단계 A — native pair 쓰기/읽기·구형 경로 정리의 첫 실패 시험(S1 Done, ADR-013 승인 완료) |
| 의존 | TASK-007·TASK-009·TASK-010·TASK-011·TASK-027 |
| 예상 소요 | S1 1~2 + S2 3~4 인일, 리뷰 대기 제외 |

## 목표

현재 모든 채널이 `dot_logger.Envelope` protobuf로 저장되어([writer.rs](../../../crates/dot-logger/src/storage/writer.rs)) 표준 MCAP 도구가 사용자 payload를 디코딩하지 못한다. 사용자 schema·encoding 그대로 판독되는 파일을 제공한다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 표준 export | `dot-logger export --standard`가 `dot-logger/1` 파일을 사용자 schema 채널로 변환, 독립 MCAP reader 대조·Foxglove/Lichtblick 판독 확인, 손실 필드 명시 | **Done** 2026-11-15 — 로컬 Pass([EV-077](../../05_verification/verification_evidence.md#ev-077)), Foxglove 판독 Human 확인(아래 S1 리뷰 기록). Lichtblick 미실행 |
| S2 | native 표준 쓰기 모드 | ADR-013 승인안에 따라 native 단일 형식으로 기록·조회·재생·복구. 배포 전 개발 형식의 판독/legacy 쓰기 보존은 제품 범위에서 제외(ADR-013 D1 수정안). **ADR 범위에 함께 넣는 것(2026-11-15):** TASK-035 손실 기록 형식, TASK-034 실행 중 설정 변경을 위한 「revision 변경 시 출력 파일 회전」과 한 출력 안의 여러 revision 조회·복구(API §22.8) | **In Progress — ADR-013 Accepted(2026-11-16), 다음 단계 A 첫 실패 시험. 제품 구현 미착수** |

## 변경 대상과 첫 검증

계획 경로: `crates/dot-logger/src/storage/`, `crates/dot-logger-cli`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **표준 모드/export 파일에서 독립 reader가 사용자 schema로 payload를 원문 일치 디코딩하고, Envelope 고유 필드(producer·u64 sequence·level·importance·revision·mono/real)는 보존 위치 또는 손실을 명시**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-014`의 표준 판독 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## Human 결정 — 2026-11-12

착수 확인 4건 + 구현 중 확인 2건(⑤ facade `Schema::descriptor`, ⑥ `payload_schema_name` 채널 metadata) 모두 권고안([API §21.1](../../03_design_artifacts/interface_contract.md#211-human-결정--2026-11-12)): ① MCAP `sequence` = 하위 32비트, u64는 보조 메타 채널 ② `log_time`·`publish_time` = real, mono는 보조 ③ level·importance·producer·revision·source는 보조 메타 채널 1개(protobuf `RecordMeta`) ④ ROS 2 encoding은 T1 유지.

## S1 리뷰 기록 — 2026-11-15

| 항목 | 결과 |
|---|---|
| `robot/blob`(`vendor-blob`, descriptor 없음) | Foxglove `Unsupported encoding vendor-blob`(2026-11-13 Human 판독). 처리: Human 「권고대로」 — 원문·encoding 유지, 한계 기록([API §21.2](../../03_design_artifacts/interface_contract.md#212-s1-구현-반영--2026-11-12)) |
| CLI summary `"records":0` | 결함 아님. `Output::finish`가 모든 명령 summary에 출력한 `record` 줄 수를 넣는데 export는 `record` 줄을 내지 않는다. 변환 메시지 수는 `export` 행 `messages`. CLI README에 의미를 추가 |
| `robot/pose`·`_dot_logger/meta` | Foxglove Topics 판독(Human, 2026-11-15, 재생성 파일 pose 300·blob 10·meta 310): `robot/pose` = `test.Pose`(protobuf) 300건·99.58 Hz·`.x`/`.y` float32, `_dot_logger/meta` = `dot_logger.RecordMeta`(protobuf) 310건·102.90 Hz·필드 channel·producer·sequence(uint64)·mono·real(int64)·revision·level·importance·source{ticks, clock_domain} 전부 인식. `robot/blob`(channel 3)만 `Unsupported encoding vendor-blob` — 수락한 한계. GUI 값 표시(plot)는 보지 않았고 payload 값 디코딩은 EV-077 protoc 대조로 확인됨 |

**S1 판정: Done(2026-11-15).** 남은 미실행: Lichtblick 판독, 공식 `mcap` Python 대조, 큰 파일 export 시간·메모리, native(T3).

## 완료 기준

- [ ] S1 완료 이력 보존·S2 native 단일 경로 검증, fixture/예제/consumer 갱신 및 구형 profile 명시 거절(ADR-013 수정안).
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 손상/부분 파일·복구·예산 경계를 두 형식에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-014](../../05_verification/test_case.md#tc-014). S1 구현·검증은 EV-077, 후속 REV-055 RV-136·139·140 조치는 EV-082에 기록한다.

## S2 저장 설계안 — 2026-11-16

[ADR-013](../../03_design_artifacts/adrs/ADR-013_native_standard_loss_revision_storage.md) Accepted(2026-11-16 Human 「승인」). D1은 배포 전 native 단일 형식 전환·legacy 보존/선택 API 제외, D2는 TASK-035 누적 계수/미확인 상태, D3는 r1→r2→r1도 허용하는 단일 active writer 회전·source별 revision 조회/복구다. 단일 형식 전환의 설계는 승인됐고 현재 구현에는 아직 반영하지 않았다. 세부 형식·예산·실패/검증 계획은 ADR 본문을 따른다.

2026-11-16 Human의 미배포 지적을 반영해 초안의 기존 파일/명시 legacy 쓰기 호환 전제를 제거했다. 과거 S1 성과와 EV는 이력으로 유지하며, ADR 전체는 이어진 Human 「승인」으로 Accepted가 됐고 제품 구현을 진행할 수 있다.

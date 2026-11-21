# [REV-030] TASK-017 S2 합성 파일 실행·정리 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #005 / AI 개발 자가 점검. 독립 리뷰가 아니다.
- 요청: Human 「다음 작업 이어서 진행해줘」. 기록된 TASK-017 S2를 구현·검증하고 기존 실행/마감·분리 커밋/푸시 위임 적용.
- 대상: 소스5d3349bd3a4b5e398e0064cd4a061759fd6f725d, diagnostics/{mod,run,file}.rs·tc_010_file_check.rs 4개.
- 기준: [code-review 스킬](../../../.agents/skills/code-review/SKILL.md)·API §15.3/15.4·TC-010.
- 상태/판정: Closed / Pass — 신규 미해결 P1/P2/P3 없음. [EV-032](../verification_evidence.md#ev-032) S2 파일 scope만 기존 위임으로 로컬 수락.

## 요약표

| ID | 중요도 | 위치·계약 | 문제/제안 | 처리·검증 | 상태 |
|---|---|---|---|---|---|
| 신규 RV 없음 | — | TASK-017 S2 | 실제 파일 실행/예산·settlement·소유 경로 정리·결과 분류 구현 | API/settlement/identity Red→Green·EV-032, S3 후속 구분 | Closed |

## [리뷰어] 검토

| 영역 | 판단·근거 |
|---|---|
| RT/동시성 | 신규 경로는 명시 일반 호출이다. capture/라우팅/완료 원자 연산·unsafe 변경 없음. 한 호출 worker·Rc/RefCell 관리 상태만 사용하고 새 thread를 만들지 않는다. 기존 workspace RT allocator 회귀 Pass |
| TDD/완료 | API 부재 Red 뒤 최소 구현. 관측 실패/시계 역전 뒤 footer가 생략되는 경계와 교체된 파일 identity의 synced 오인 반례를 assertion Red→Green으로 보강. 요청한 부하/중단 이유와 최종 결과를 별도로 보존 |
| 메모리 | 등록/records/간격 검증과 WriterConfig의 checked 예약, payload/run/path 보수 비용을 preflight. shared output writer lease 실제 사용/반환을 모든 통합 경로에서 대조. OS cache/host runtime/allocator 내부·RSS를 실제 예약 증명으로 바꾸지 않음 |
| 쓰기/reserve | footer와 per-record chunk/index/seek 재쓰기의 보수 예약을 선차감. 실제 Write 수락량을 별도로 누적하고 fresh free-reserve/예약/총 상한을 쓰기마다 검사. payload/파일 길이/SSD write amplification을 서로 혼동하지 않음. 짧은 쓰기·부분 OS 오류·seek 재쓰기·cap/reserve 거절 단위3개/Miri24 |
| IO/수명 | 정상/예산/취소·관측/시계 오류 후 기존 prefix footer 시도→실제 file/directory sync→모든 writer/clone/File Drop→cleanup. poisoned writer는 footer 성공을 만들지 않고 실제 남은 파일의 sync/close/정리만 시도. sync는 admission 기한을 넘을 수 있고 OS IO 강제 취소 보장 없음 |
| namespace | 새 create_dir/create_new만 사용하고 기존 경로/symlink 보존. trusted parent 독점 통제 전제. dev/inode 확인 후 이번 파일/디렉터리만 삭제, 재귀 삭제 없음. foreign 파일/교체 경로는 보존하고 sync/cleanup을 실패로 표시. ancestor 동시 교체에 대한 sandbox를 주장하지 않음 |
| 오류/상태 | primary/첫 footer·sync/cleanup 오류를 분리하고 OS 원인 유지. Pass/Fail/Aborted/Inconclusive와 written/synced/file_complete/cleaned를 구분. footer 공간까지 사라지면 완료를 추정하지 않고 Fail/부분 범위 |
| API/측정 | 기존 WriterBudget session/revision 일치·단일 calling worker·Unix 파일 경계만 지원. 네트워크/receiver/DB는 무실행 Inconclusive, 생산 완료 ACK나 capacity Profile을 만들지 않음. synthetic seed/실제 mono/real0 등 입력 scope를 명시. 새 dependency/wire/프로토콜/공유 문서 구조 변경·ADR 승격 없음 |

## [처리담당] 처리·검증

- API 부재 exit101/E0432 Red. 관측 오류/역전 뒤 footer 미완료 반례2개와 교체 경로의 synced_records 오인 반례1개를 먼저 실패시킨 뒤 수정했다. witness는 EV-032의 raw archive에 보존했다.
- 신규 실제 파일15개·BoundedSink3개. 기존 S1 계획5개 및 core lib를 포함한 Rust1.96/MSRV1.85 관련40개 각각 Pass. workspace/all-features347개(문서 시험 포함), fmt/clippy/rustdoc -D warnings Pass.
- Miri nightly-2026-10-26 strict provenance, BoundedSink3종×8seeds=24 Pass. 실제 filesystem/lifecycle 시험을 Miri 결과로 대체하지 않으며 별도 OS 통합15개로 확인했다.
- 실제 MCAP prefix/완성 파일 독립 MessageStream·payload/revision 읽기, 실제 단조 시계 실행, 운영 bytes 불변, writer 예산 회수를 확인했다. free-space probe는 기능 fixture이며 실제 운영 OS probe/장비 성능 인증은 후속이다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | crates/dot-logger/src/diagnostics/run.rs | 유한 합성 workload·명시 실행/중단·file scope 보고 |
| 생성 | crates/dot-logger/src/diagnostics/file.rs | 실제 파일 소유/identity·sync/cleanup·쓰기 상한과 내부3개 |
| 수정 | crates/dot-logger/src/diagnostics/mod.rs | 로컬 실행 API 공개·scope 문서 |
| 생성 | crates/dot-logger/tests/tc_010_file_check.rs | 실제 file IO/lifecycle·입력/오류/소유권15개 |
| 생성/수정 | 본 리뷰·API/UI·TASK/MS·TC/EV/project_state·실행 근거 | S2 로컬 수락/다음 S3·미검증 경계 |

## 클래스(타입) 변경표

| 구분 | 소속 | 타입 | 역할 |
|---|---|---|---|
| 생성 | run.rs | FileWorkload·FileCheck | 반복 seed의 유한 합성 입력과 단일 채널 |
| 생성 | run.rs | CheckRuntime·FileCheckRuntime | session clock/free/cancel/pacing port와 실제 clock 연결 |
| 생성 | run.rs | CheckOutcome·CheckStop·CheckStage·CheckIssue·CheckStartError·CheckReport | 파일 scope 상태/원인·원본 오류·실제/부분 완료 |
| 생성 private | run.rs | Shared·Admission | 단일 관리 호출의 예약/실제 Write·진행 상태 |
| 생성 private | file.rs | OwnedFile·BoundedSink | 소유 namespace/실제 handle·sync/close/정리·물리 쓰기 backstop |
| 수정 | diagnostics 모듈의 CheckPlan | run_file 메서드 | S1 무부하 계획 뒤 별도 명시 로컬 실행 |

## 미검증/다음 행동

S3 운영 free probe/capability·Report/profile/CLI/runtime/원격 backend·실제 capture/routing/receiver/실앱, PC/RT 비용/오차/처리량·UPS·장기/native(T3)/다른 OS·fresh audit는 미완료다. trusted namespace의 외부 동시 교체/host callback panic·프로세스 kill 후 자동 정리/재시작을 보장하지 않는다. 짧은 로컬 합성 file Pass로 Task/MS/TC 전체·Gate4/5를 올리지 않는다. FR-015/TASK-018 미착수 유지.

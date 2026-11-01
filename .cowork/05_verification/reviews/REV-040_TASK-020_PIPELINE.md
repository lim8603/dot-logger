# REV-040 — TASK-020 S2 통합·설정 비교 예제 개발 자가 점검

## 개요

- 일자/담당: 2026-11-01 #015 / AI 개발 세션 자가 점검. [code-review 스킬](../../../.agents/skills/code-review/SKILL.md) 적용, 독립 리뷰 세션과 구분한다.
- 대상: `dot_probe_pipeline`·shared example support·pipeline 시험·client Cargo/lock·DB harness·README. 최종 소스와 실행 형상은 [결과](../../06_evolution/imported_context/task_020_s2_2026-11-01/results.json)에서 고정한다.
- 종합 판정: **Pass — S2 로컬 기능 예제 범위**. 미해결 P1/P2/P3 지적 없음.
- 상태: **Closed** — Open → In Progress → 검증/자가 점검 Resolved → 기존 위임으로 S2 로컬 수락. Task/MS/TC 전체 완료가 아니다.
- 소스: `aab94f4` (136개 파일/commit blob 대조).
- 근거: [EV-041](../verification_evidence.md#ev-041).

## 리뷰 요약

| ID | 중요도 | 문제/처리 | 판정 | 상태 |
|---|---|---|---|---|
| RV-082 | P2 | JSON phase 출력에 원 session/epoch·환경/workload/target/boundary/input ID 누락. 결과 파일만으로 출처를 대조할 수 없어 모든 식별자를 추가 | 수용·실제 누락 Red→Green | Closed |

새 미해결 RV는 없다. 다음 경계를 확인하고 예제·계약·실행 보고서에 반영했다.

| 검토 항목 | 확인/처리 |
|---|---|
| 원 identity/수명 | 한 capture receipt·metadata·payload를 file/DB/subscription에 유지, 원 guard 실제 종료 뒤 회수 |
| revision | DB initial revision 계약 유지·변경 후 DB route off, file writer는 revision별 별도 예약 |
| 비교 | 가상 시계·fanout 차이 명시, Comparison workload 차이/Unavailable 보존 |
| 종료/성공 | Ok(report)와 프로필 Pass 구분, 전체 선택 경계/query 검증으로 CLI 0/3/4 분류 |
| 자원 | bounded 큐·읽기·입력·deadline, actual server join/runtime Drop·임시 파일 명시 정리·예약0 |

## [리뷰어] 상세 점검

- **RT/원자성:** 일반 경로 예제만 추가했다. `crates/dot-logger/src`와 client 제품 src·공개 wire·core 수집/원자성/unsafe는 변경하지 않았다. 예제 Temp의 AtomicU64는 파일명 구분용 Relaxed이며 create_new/create_dir 실패로 충돌을 거절한다. RT 경로에 logger 설치/락/format/IO/콜백을 넣지 않는다.
- **소유권/실제 IO:** RoutedWriter의 native attempt가 dedicated runtime 실제 정리까지 원 Delivery를 유지하고 publish 뒤 Drop한다. subscriber는 원 Delivery를 RemoteHub에 인계하며 완전 조립만으로 ACK하지 않는다. 실제 bounded 소비 큐가 받으면 ACK하고, 마지막 큐 Full은 연결 종료/원 Unknown으로 정리한다. 서버 task는 명시 생성·join하며 daemon·재연결·재전송은 없다.
- **파일/정리:** 자기 소유 unique 임시 디렉터리·create_new 파일 두 개만 사용한다. revision별 writer의 동일 output 예산·타깃 lease를 지키고 footer/flush/file sync/close 후 독립 MCAP MessageStream·제품 봉투 decoder로 metadata/bytes를 비교한다. 2MiB 읽기 cap·512B payload·30개 phase history 상한을 유지한다. completion은 Written이며 sync 호출로 Synced를 위조하지 않는다. 정상 반환 전 파일 정리 오류를 전달하고 Drop은 오류 경로 보완이다.
- **입력/비교:** 동일 합성 생성 입력30건씩, 수락30→15·의도적 생략0→15, DB 할당20→0이다. Report는 실제 카운터의 안정 revision window와 정확 from/to의 quiescent target_totals 차분을 사용한다. 가상 session clock(0~10s/11~21s)을 계획/출력/README에 명시한다. 실제 처리량/CPU/RSS/capacity/RT를 측정한 것으로 표시하지 않는다. before/after fanout 변경은 workload 차이라 Comparison은 IncomparableConditions를 보존한다.
- **예산/실패:** file/receiver는 DB transport Unknown 중에도 45건 보존을 대조한다. 순차 일반 호스트이므로 DB 지연과 독립된 지속 처리율은 보장하지 않는다. cancel/기한은 신규 입력을 멈추며 actual 정리는 별도다. server shutdown timeout은 abort/await 후 Runtime Drop, 원래 작업 오류에서도 정리를 수행한다. 종료 후 external 예약0을 검사한다.
- **의존/기능 격리:** 새 `integration-example`은 기존 remote/influxdb/tracing-adapter만 명시 enable한다. log/mcap은 예제·검증용 dev 의존이고 기존 lock 버전/checksum은 전부 동일하다. 독립 core/default client의 normal/build 그래프에 네트워크·tracing·압축을 추가하지 않는다. `log`는 기존 MCAP 간접 의존이므로 신규 오염으로 판정하지 않았다.
- **보안/범위:** 토큰은 환경 변수에서만 읽고 Config/endpoint/secret을 보고서에 넣지 않는다. plan은 환경 조회·파일/DB·bind가 없다. explicit run만 fresh exclusive measurement를 사용한다. harness는 기존 digest-pinned 일회 container·tmpfs·loopback·자원 제한 및 own cleanup을 재사용한다. 기존 DB 데이터 삭제·공개·CI/계정/과금 조회는 없다.

## [처리담당] 검증/수락

- RV-082 수용: [직렬화 출처 누락 Red](../../06_evolution/imported_context/task_020_s2_2026-11-01/red-provenance.log)(Null/session mismatch) 뒤 원 Snapshot에서 JSON session/epoch·비교 조건을 내보내도록 수정했다. 일반/실제 DB·CLI를 최종 형상으로 재검증했다.
- API/공유 모듈 부재 compile Red(exit101) → 최소 wiring → 일반4개/실제 DB1개 Green. 일반 시험은 DB transport 실패·마지막 큐 Full/Unknown·초기 cancel/0기한·잘못된 DB 설정/비밀값 비노출·nested runtime 거절을 검증했다.
- Rust1.96/MSRV1.85 workspace각396개+doctest17개 Pass, 기존 실제 DB4개+새 실제 DB1개는 일반 회귀 ignore. 별도 각 toolchain의 고정 OSS2.9.1 harness에서 core4개+pipeline1개와 CLI 실행 Pass·cleanup0.
- 실제 정상 CLI: before 입력30/파일30/DB20/수신30, after 입력15·skip15/파일15/DB0/수신15. 독립 파일45·수신45·numeric receipt/time/value10·raw 회수20·external0, functional_pass=true.
- DB 실패 CLI exit3에서도 파일45·수신45·external0, missing DB env exit4, 무환경 plan exit0를 대조했다. client default/remote/MSRV default·독립 graph·fmt/clippy/rustdoc Pass. 새 core 변경이 없어 기존 EV-040의 core16조합 근거를 유지하고 무관한 전체 feature 반복은 하지 않았다.
- 새 Miri/성능/RT/native/실제 dot-probe·S3 화면 시험은 미실행이다. S1/S2만 수락, S3가 다음 시작점이다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | `crates/dot-logger-client/examples/dot_probe_pipeline.rs` | 무부하 계획·명시 실행·JSON/exit |
| 생성 | `examples/support/pipeline/mod.rs`, `remote.rs` (client 예제 하위) | 유한 입력·실제 파일/DB/구독·설정/비교·정리 |
| 생성 | `crates/dot-logger-client/tests/pipeline.rs` | 공유 코드의 일반4개/실제DB1개 |
| 수정 | client Cargo·Cargo.lock | 선택 integration-example·기존 log/mcap dev edges |
| 수정 | `scripts/check-influxdb.py` | owned DB에서 pipeline 시험/CLI 추가 실행 옵션 |
| 수정 | root/client README | 실행·가상 시계·DB route off·미수락 경계 |

## 클래스(타입) 변경표

| 구분 | 소속 | 타입 | 역할 |
|---|---|---|---|
| 생성 | example support/mod.rs | `Options`, `ReceiverMode` | 유한 기한·query·소비 큐 시나리오 |
| 생성 | example support/mod.rs | `Phase`, `PipelineReport` | 원 카운터/조건·비교·전체 경계 수락 판정 |
| 생성(내부) | example support/mod.rs | `Temp` | 자기 소유 임시 파일 수명·정리 |
| 생성(내부) | example support/remote.rs | `Remote` | 실제 gRPC·원 delivery/큐 ACK·server/runtime 정리 |

제품 라이브러리 타입 변경 없음. `Error`는 예제 내부 일반 오류 alias다.

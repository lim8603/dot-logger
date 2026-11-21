# REV-020 — TASK-013 착수·클라이언트 연결 수명 API 개발 자가 점검

## 개요

- 일자/담당: 2026-10-31 #007 / AI, code-review 스킬 기준 적용.
- 대상: 소스 `86b50ef`, dot-logger-client의 receiver 수명 첫 단위 및 workspace 등록. 실제 transport/CLI는 범위 밖이다.
- 성격: 개발 자가 점검/조치 자산. 독립 Read-Only 리뷰 세션이 아니다.
- 판정: Pass — 검토 범위에서 새 P1/P2 미해결 지적을 확인하지 못했다. 전체 제품/Task 통과를 뜻하지 않는다.
- 상태: Closed — Human의 구현/검증·마감 판단 위임에 따라 EV-024 검증 뒤 첫 단위를 AI가 수락했다. 전체 TASK-013 완료 승인은 아니다.

## 처리담당의 검토 근거

- RT/unsafe: 모든 API는 일반 경로며 자체 unsafe·runtime/socket/worker 시작 없음. 기본 client와 독립 core consumer의 네트워크 의존 격리를 metadata와 실제 실행으로 확인했다.
- 연결: Arc identity가 살아 있는 old token/owner와 새 연결을 구분한다. 잘못된/중복 begin이 기존 연결이나 예약을 교체하지 않고, stale/foreign end가 현재 receiver를 닫지 않는다. 동일 server subscription ID 재사용도 old fragment/ready/ACK와 구분한다.
- 자원: core QueryBudget/Reassembler를 사용한다. disconnect는 incomplete 정리만 하고 외부 PendingRecord가 살아 있으면 예약을 보존한다. consumer 거절/TTL/세대 오류는 원래 owner를 반환한다. identity/receipt/transport metadata는 별도 host headroom임을 문서화했다.
- ACK: consumer 인계 전에는 ACK가 없고, 인계 후에도 connection identity를 갖는다. take_ack는 현재 연결을 재검사한다. 이미 추출/전송된 ACK나 인계 데이터를 소급 취소하지 않는다. 추출 뒤 실제 socket send 수명은 다음 transport 단위의 필수 검증이다.
- 오류/API: 고정 ClientError와 core RemoteError를 사용하고 credential/payload를 오류에 출력하지 않는다. 공개 API에 tonic/prost/wire 내부 타입을 노출하지 않는다. 반환 owner를 보존하기 위해 handoff의 result_large_err만 국소 허용했다.

## 파일 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | [client Cargo.toml](../../../crates/dot-logger-client/Cargo.toml)·[lib.rs](../../../crates/dot-logger-client/src/lib.rs) | 기본 core-only·opt-in remote client crate/export |
| 생성 | [receiver.rs](../../../crates/dot-logger-client/src/receiver.rs) | 연결별 소유·조각/인계/세대/ACK identity 경계 |
| 생성 | [connection_lifetime.rs](../../../crates/dot-logger-client/tests/connection_lifetime.rs) | 실제 공개 API 사용 통합3개 |
| 수정 | workspace Cargo.toml·Cargo.lock | 로컬 package1개 등록, 외부 package/version 추가0 |

## 클래스/타입 변경표

| 구분 | 소속 | 타입 | 역할 |
|---|---|---|---|
| 생성 | receiver.rs | ClientReceiver·LiveConnection(private) | 현재 연결/receiver 소유 및 lifecycle |
| 생성 | receiver.rs | ConnectionId | 실제 transport 소유자가 보관하는 opaque identity |
| 생성 | receiver.rs | PendingRecord·Acknowledgement | 원래 연결 및 payload owner/인계 receipt 수명 |
| 생성 | receiver.rs | ReceiverConfig·ClientError | 명시 records/bytes/TTL 및 고정 lifecycle 오류 |

## 검증·다음 행동

[EV-024](../verification_evidence.md#ev-024): 공개 API 부재 Red → 통합3개 Green, source74개/검사12개 exit0·commit blob74개 일치. 두 Rust1.85.0/1.96.0 workspace/기본client/core consumer/lint/doc/fmt, client Miri3×8=24 Pass다. 기존 remote 조치의 각16feature/Miri25 근거는 EV-023이며 현재 통합 workspace 회귀도 Pass다.

다음은 actual transport/ACK send 수명·endpoint/TLS/token/버전·CLI JSON/exit/Ctrl-C·query→replay/오프라인 파일/예제다. native 공개 후T3·실앱/LAN/RSS·PC/RT/UPS·Gate4/5 미완료를 유지한다. 독립 코드리뷰를 수행했다고 주장하지 않는다. 새 RV ID는 채번하지 않았다.

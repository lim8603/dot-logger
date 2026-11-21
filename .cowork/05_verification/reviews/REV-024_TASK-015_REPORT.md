# [REV-024] TASK-015 여유·병목·보존 리포트 개발 자가 점검

## 개요

- 일자: 2026-10-31 #012 / AI 개발 세션 자가 점검. 독립 리뷰 세션이 아니다.
- 대상: telemetry/{capacity,report,mod}.rs·TC-009 리포트/실제 window 연결·공개 consumer·allocator·README.
- 근거: 승인된 API§15·TASK-015 착수/로컬 수락·게시 위임. 새 stack/프로토콜·설정 자동 변경 없음.
- 상태: Closed — EV-028 source100개/검사13개·Miri168·소스95a63a6·commit blob 일치 후 기존 위임으로 AI 로컬 수락.
- 판정: 로컬 Pass. 아래 지적은 실패 재현 후 조치·회귀 검증했으며 신규 미해결 P1/P2 없음. 독립 리뷰/Human 직접 재리뷰·실측을 주장하지 않는다.

## 요약표

| ID | 중요도 | 문제·위치 | 처리 | 상태 |
|---|---|---|---|---|
| RV-056 | P2 | profile 유효 시각을 다른 session clock에서 재사용 가능 | Profile.session_id 및 build 조건 일치 검사 | Closed |
| RV-057 | P2 | filesystem free가 늘면 출력 quota 소진을 놓침 | free 감소와 written-deleted quota 증가를 독립 계산 후 빠른 한계 | Closed |
| RV-058 | P2 | u64 ns 초과 한계가 quota AtLimit 판정을 가림 | BeyondRange를 유한 한계와 비교하고 AtLimit 우선 | Closed |
| RV-059 | P3 | 용량 대비 모델로만 판단한 Overloaded의 Observed 분류 | 실제 손실/한계는 Observed, 모델만 초과는 Estimated(MatchingCapacity) | Closed |

## 상세 내역

### RV-056·057

[리뷰어] 같은 수치의 monotonic 유효 시각은 session이 다르면 재사용할 수 없다. filesystem 여유가 늘어도 대상별 파일 quota는 증가할 수 있어 서로 같은 증가율을 쓰면 안 된다.

[처리담당] 수용. `Profile.session_id`·장비/부하/revision/target/boundary 일치 검사를 추가했다. disk filesystem/출력 quota는 별도 순증가율로 계산한다. boundary-red.log에서 두 assertion 실패를 먼저 확인한 뒤 tc_009_report의 회귀로 조치했다.

### RV-058

[리뷰어] 수백 년 이상 예측의 표현 한계는 다른 자원이 이미 full이라는 사실을 지워서는 안 된다.

[처리담당] 수용. Forecast::BeyondRange로 유한 양수/미측정/NoGrowth를 구분하고 first()에서 AtLimit·유한 한계를 보존했다. range-red.log로 재현 후 quota full/pressure와 buffer 초과 범위 회귀를 통과했다.

### RV-059

[리뷰어] 실제 손실이 없는 상태에서 검증 용량에 비춘 계산 초과만으로 Observed 손실/포화와 같은 provenance를 붙이면 근거가 혼동된다.

[처리담당] 수용. 모델만 초과는 Estimated(MatchingCapacity)로 바꿨다. provenance-red.log assertion 실패 후 조치했고 skip은 offered/loss에서 제외된다.

## 검토 결과

| 영역 | 판단 |
|---|---|
| RT/원자성 | 새 코드는 관리 경로의 순수 유한 계산. Producer·원자 ordering·private unsafe·수명/Drop·wire·dependency 변경 없음 |
| 예산 | 고정 Copy 타입/정수 계산만 사용. heap/worker/IO 없음. allocator 1024회 할당0/해제0·독립 공개 consumer로 검증 |
| 단위/수량 | global ingress records/payload bytes와 target assignment/물리 점유/실제 file bytes 구분. accepted+rejected checked 합계, u128 ratio, signed net growth와 ns 올림 |
| 근거/누락 | session/revision/epoch/실제 window·환경/부하·target/boundary 보존. 미측정/만료/다른 조건·포화는 명시 사유. loss 안정 큐 Overloaded, intentional skip 별도 |
| 수명/취소 | background 실행/IO/새 자원 소유가 없어 build 취소·종료 프로토콜 추가 불필요. 기존 owner/reader/worker 종료 계약은 변경 없음 |
| 통합 책임 | 호스트가 같은 window의 물리 gauge/출력 delta를 제공하고 source overflow/reset/foreign 관측을 먼저 거절해야 함. profile의 실제 검증 근거·환경/부하 ID 관리도 호스트 책임. 수집/지연 계측/자동 backend 구현 완료로 확대하지 않음 |

## 미검증

PC/RT 실제 IO 예측 오차≤20%·계측 비용≤5%·다른 OS/native(T3)·실앱/물리 gauge 수집/CLI runtime backend·TASK-016/017·MS-005/TC-009 전체/Gate4/5는 미완료. 합성 simulation은 모델 산술 검증이다.

## 최종 검증·수락

[EV-028](../verification_evidence.md#ev-028) 소스95a63a6·source100개/검사13개 exit0·Miri168·디스크/commit blob hash 일치. 두 Rust 각16feature/workspace·core-client·공개 consumer/RT allocator·clippy/rustdoc/fmt Pass. 위임으로 RV-056~059 및 본 개발 자가 점검을 Closed로 마감한다. 새 H/J/ADR 승격 대상 없음, 실제 PC/RT 오차·비용/native/실앱 후속 유지.

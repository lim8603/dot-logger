# TASK-031 — 원격 소비처 dot-logger-viewer와 시각화 bridge

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-009 |
| Story | US-003·004 |
| 요구사항 | FR-005 FR-006 FR-010 NFR-001 NFR-007 NFR-008 |
| 설계 | [인터페이스](../../03_design_artifacts/interface_contract.md) · [CLI/화면](../../03_design_artifacts/ui_spec.md) |
| ADR | ADR-007·009 Accepted. 시각화 방식 기술 결정(S1, tech_stack 또는 ADR) |
| 담당 | AI |
| 상태 | Planned |
| 마지막 갱신일 | 2026-11-04 |
| 다음 액션 | S1 — Foxglove WebSocket 프로토콜·SDK crate·Lichtblick 호환/라이선스 조사와 결정 |
| 의존 | TASK-012·TASK-013·TASK-030(S1) |
| 예상 소요 | 2~3 인일, 리뷰 대기 제외 |

## 목표

같은 저장소의 별도 crate `dot-logger-viewer`가 로봇 프로세스의 기존 gRPC(Subscribe/Query/Replay/Status)를 소비해 Foxglove WebSocket 프로토콜로 내보낸다. 이미지·점군·그래프 시각화는 Foxglove/Lichtblick에 맡기고 직접 구현하지 않는다.

```
[로봇] 앱 + dot-logger(remote) ──gRPC──▶ [운영 PC] dot-logger-viewer ─ Foxglove WS ─▶ Foxglove / Lichtblick
[오프라인] MCAP ─ TASK-030 export/표준 모드 ─▶ Foxglove / Lichtblick
```

- viewer는 `dot-logger-client`에만 의존하고 코어 내부를 직접 쓰지 않는다(client API 충분성 검증).
- 로봇 프로세스에는 추가 데몬·코드가 없다(ADR-007 유지). 원격 서버의 loopback 기본·TLS/인증 계약을 따른다.

## 작업 단계 (Step, S)

| 단계 | 목표 | 종료 상태 (검증 가능) | 상태 |
|---|---|---|---|
| S1 | 기술 조사·결정 | 프로토콜/SDK 버전·라이선스·Lichtblick 호환 근거, tech_stack 반영안 Human 승인 | Planned |
| S2 | 실시간 bridge | Subscribe→WS 채널, Gap/Unknown·종료/재연결 표시, 유한 큐·느린 소비자 처리 | Planned |
| S3 | 기록 소스 | Query/Replay 기반 시간 이동·배속, 파일 열기 | Planned |

## 변경 대상과 첫 검증

계획 경로: `crates/dot-logger-viewer/`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **viewer가 느린 WS 소비자·원격 Gap·서버 종료에서 유한 자원으로 Gap/Unknown을 표시하고, 로봇 측 구독 자원을 실제 해제**.

- [ ] 요구·설계와 연결한 실패 테스트(문서 작업은 리뷰)에서 의도한 실패를 확인한다.
- [ ] 최소 구현 후 관련 `TC-014`의 viewer 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [ ] 실시간·기록 소스 bridge와 Foxglove 계열 도구 판독.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-014](../../05_verification/test_case.md#tc-014). 제품 코드·테스트 실행 결과: 없음.

# REV-049 — TASK-027 S3c remote 선택 연결 개발 자가 점검

## 개요

- 일자/담당: 2026-11-07 #001 / AI 개발 세션 자가 점검. 독립 리뷰가 아니다.
- 대상: 소스 `d595122` — `logger/remote.rs`(`Remote`·`Tap`·`Runtime`·`default_limits`), `host.rs`(`Links`, IO tap, command 처리, Closed 전 정지), `budget.rs`(`Extra`·`resolve`), `mod.rs`(`remote`·`remote_with`·`remote_addr`·`StartError::Remote`), `health.rs`(remote thread 고정 결과), `routing`(`Delivery::is_raw` crate 내부), TC-013 F13(`dot-logger-client/tests/facade_remote.rs`).
- 상태: **Closed** — 결함 수정이 필요한 항목은 없다.
- 판정: **Pass — 로컬 기능 범위.** 근거: [EV-050](../verification_evidence.md#ev-050).

## 리뷰 요약

| ID | 중요도 | 위치·계약 | 문제 | 처리 | 상태 |
|---|---|---|---|---|---|
| RV-110 | P1 | remote 정지와 shutdown | tonic graceful 종료는 클라이언트가 연결을 닫을 때까지 기다린다. 클라이언트 runtime이 멈춰 있으면 host가 listener join에서 막혀 shutdown이 `Unresponsive`(15초)가 됐다(첫 구현에서 F13 재현) | 정지 신호 뒤 2초 유예 후 runtime을 내려 남은 연결을 끊는다. F13이 shutdown 4초 미만·성공을 단언 | 조치 |
| RV-111 | P2 | 메모리 partition 표기 | core `Resource`에 external 항목이 없어 remote 예약 부족이 `Total`로 보인다 | 공개 enum을 바꾸지 않기로 하고 §18.11에 명시. remote 한도를 줄이거나 총량을 늘리라는 rustdoc 안내 | 문서화 |
| RV-112 | P3 | 사본 처리 위치 | 첫 output IO thread가 batch마다 구독 수만큼 offer(lock·payload 복사)한다. 구독자가 있으면 파일 쓰기 처리량에 영향이 있을 수 있다 | 구독이 없으면 비용은 구독 목록 조회 1회. 영향 측정은 TASK-023(필요 시) | 문서화 |
| RV-113 | P3 | Compare 응답 | telemetry가 켜져 있으면 Compare가 Gone(snapshot 없음), 꺼져 있으면 Unsupported로 다르다 | 기존 `OperationsHost` 의미를 그대로 따른 결과. §18.11에 명시 | 문서화 |
| RV-114 | P3 | 종료 지연 상한 | 멈춘 클라이언트가 있으면 shutdown이 최대 2초 더 걸린다(기한 계산 밖) | 유예 상수를 문서화. 기한 안 종료가 중요한 사용처는 클라이언트 쪽 종료를 먼저 수행 | 문서화 |

## [리뷰어] 점검

- **범위/권한:** 2026-11-07 Human 결정(관측용 사본·상태 조회만·기본값+명시) 범위 안. 새 의존성·Cargo.lock 변경·unsafe 없음. feature 없이는 `Remote`/`Tap`이 빈 구현이며 feature 16조합 두 toolchain 확인.
- **완료 계약:** remote는 routing target이 아니므로 required 완료·`is_success` 판정에 영향이 없다(F13: 구독 중 shutdown 성공).
- **시작 원자성:** bind·한도·예산 오류는 namespace 생성 전에 반환되고 디렉터리가 남지 않는다(F13). host 시작 실패 시 `Runtime` Drop이 listener를 멈추고 join한다.
- **시험 신뢰도:** F13 4개. 첫 구현의 종료 교착을 F13이 재현했고(RV-110), tap을 끄는 돌연변이에서 구독 수신이 TimedOut으로 실패함을 확인했다. F13·TC-013 binary 10회 반복 통과.

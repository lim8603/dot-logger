# REV-065 — TASK-031 S3 원격 기록 재생 자가검토

| 항목 | 내용 |
|---|---|
| 일자 | 2026-11-18 |
| 대상 | `3ee2381..bc51394`, ADR-014 개정 |
| 판정 | 로컬 Pass, [EV-094](../verification_evidence.md#ev-094) |
| 상태 | Closed — 위임받은 개발 검토 범위 |

| 검토 경계 | 확인 |
|---|---|
| 원격 계약 | 기존 Query/ReadBlob의 타입·시간 범위·고정 스냅샷·부분 blob·연결 소유권을 사용한다. core·wire·RT 경로 변경은 없다. |
| 시간 이동 | 진행 중일 때는 새 세대로 이동하고, 처음 요청한 snapshot capability를 한 번 받아 재사용한다. 이전 blob 결과와 대기 출력은 송신 전에 차단한다. |
| 범위·수명 | 한 소켓당 한 페이지·blob·대기 출력, 페이지 수와 쓰기 기한을 검토했다. 소켓이 닫히면 연결을 Drop한다. 호스트의 TTL/명령 소유권이 실제 read 정리 시점을 정한다. |
| 소비 경로 | 시간/메시지의 동일 mono 시간축과 WS 데이터 순서, Lichtblick 역방향 이동 후 재생, 독립 WS 배속 대조를 확인했다. |

공개 API는 client 타입으로 구성하며 제품 경로에 신규 `unsafe`는 없다. 코드 검토에서 열린 지적은 없다. Foxglove Desktop 연결 및 LAN 정량은 [TASK-031](../../04_implementation/tasks/TASK-031.md)의 잔여 검증이다.

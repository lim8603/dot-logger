# REV-064 — TASK-031 S3 MCAP 등록 배치 자가검토

| 항목 | 내용 |
|---|---|
| 일자 | 2026-11-18 |
| 대상 | `338f600..a3296dc` |
| 판정 | 로컬 Pass, [EV-093](../verification_evidence.md#ev-093) |
| 상태 | Closed — 기존 위임에 따른 개발 검토 수락 |

쓰기 옵션이 바꾸는 범위는 초기 Schema/Channel 배치다. 메시지 인덱스와 CRC, 완료 경계, `standard/2` profile, 등록 ID는 유지된다. 새 파일의 초기 등록 종료 후 첫 실제 메시지의 청크 위치를 계산한다. 읽기에서는 청크 내부·외부 등록에 같은 길이·ID·스키마 검증을 적용하며 DataEnd 뒤 summary를 등록 원본으로 해석하지 않는다.

합성 초기 baseline, 손상·안전 오프셋·복구·새 파일 실제 Lichtblick, 기존 원본의 독립 복구/판독을 확인했다. RT 수집 코드나 `unsafe` 변경은 없다. 원격 재생 경로 결정은 [TASK-031](../../04_implementation/tasks/TASK-031.md)에 남아 있다.

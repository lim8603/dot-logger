# REV-042 — TASK-021 UPS 긴급 계획·모의 시험 개발 자가 점검

## 개요

- 일자/담당: 2026-11-02 #001 / AI 개발 세션 자가 점검. [code-review 스킬](../../../.agents/skills/code-review/SKILL.md) 적용. 독립 리뷰가 아니다.
- 대상: `control::emergency`·`platform::power`·기존 `ControlOwner` 통합·UPS 공유 실행/예제·TC-011·할당/경합 시험.
- 상태: **Closed** — Open → In Progress → RV-084/085 조치·최종 검증 Resolved → 기존 위임으로 로컬 구현/UPS 시뮬레이션 수락.
- 판정: **Pass — 로컬 구현·승인된 UPS 시뮬레이션**, 새 미해결 지적 없음. 실물 UPS는 ADR-011 및 Human 재확인에 따라 수락 선행조건이 아니다.
- 근거: [EV-043](../verification_evidence.md#ev-043)·[검사](../../06_evolution/imported_context/task_021_2026-11-02/checks.json).

## 리뷰 요약

| ID | 중요도 | 위치·계약 | 문제 | 처리 제안 | 판정 결과 | 상태 |
|---|---|---|---|---|---|---|
| RV-084 | P2 | `ControlOwner::poll_emergency` 최종 기한 | poll 도중 더 이른 알람이 게시되면 시작 시각의 이전 기한으로 protected_complete를 판정할 수 있음 | 최종 결과 전에 deadline 재관측·필요한 동일 coordinator 갱신 | 수용·Barrier로 실제 경합 Red→Green | Closed |
| RV-085 | P2 | 시뮬레이션 파일시스템 | 초기 temp 경로가 tmpfs여서 SSD sync 측정으로 수락할 수 없음 | 명시 trusted-parent 및 승인 ext4 재실행 | 수용·환경 관측 뒤 `/data/build` 실측으로 대체 | Closed |

## [리뷰어] 점검

- **범위/권한:** 기존 ADR-008/009의 startup 계획·atomic mailbox·가장 이른 기한·동일 종료 coordinator를 구현했다. 새 기술 계열/worker/wire/UPS 장치 프로토콜 채택은 없다. 새 ADR 승격 대상이 아니다. 몇 분도 절대 기한으로 표현하며, 수락 시험은 기존3초/안전여유200ms/8·64MiB/지연0·50·250·1000ms를 유지한다.
- **RT/할당:** 비활성은 추가 heap 예약/전이0, 활성 startup metadata만 checked control 예약한다. notify는 원자 연산/시각 변환/입력 close만 수행하고 할당/해제/IO/락/사용자 콜백/재시도 루프가 없다. RT 생산자에 JSON/IO를 추가하지 않는다. 신호는 OS 비동기 signal handler 밖의 host 경로에서 전달한다.
- **원자성/수명:** generation/receipt/정상 종료 coordinator는 유지한다. 알람의 event ID·생성/수신 최소 시각과 가장 이른 cutoff는 독립 관측임을 명시하고 잘못된 multiword 쌍을 만들지 않는다. metadata를 먼저 게시하고 deadline AcqRel/Acquire로 소비한다. 별도 송신자들은 fetch_min으로 기한을 늘릴 수 없다. 새 입력/일반 sync를 닫고 queued ordinary sync는 Expired로 끝내지만 이미 시작한 guard는 실제 완료까지 보존한다.
- **파일/완료:** rank/동일 rank target ID 순서, file별 barrier lifecycle과 payload limit/미관측을 구분한다. missing bytes는0이 아니며 이전 실패/Expired 상세는 보호 성공을 막는다. 선택하지 않은 optional target은 취소하되 실제 guard를 회수하지 않는다. 필수 Synced target을 누락한 계획을 거절한다. 실제 file sync를 완료한 adapter만 Synced를 게시하고 IO 오류는 부분 범위로 남긴다.
- **snapshot 정책:** 신호 후 추가 snapshot 수락량은0이다. 미리 수락한 protected raw/input만 frozen cutoff로 처리하고 기존 풀 quota/파일 framing 예산을 유지한다. 새로운 마지막 상태 입력 경로를 암묵적으로 무제한 허용하지 않는다.
- **모의 중단/복구:** 실제 MCAP·sync, footer 없는 active 파일·잘린 꼬리, child의 destructor 없는 exit17, parent의 사전 외부 oracle 및 별도 read-only source→create_new 복구/receipt·metadata·payload 대조. 이전 completion을 재생성하지 않는다. 소유 child는 오류/기한에도 kill+wait하며 고정 소유 파일만 정리한다.
- **한계:** 현재 generic Linux PC의 합성 보호 부하다. 실행 중 OS IO 강제 종료·보편적3초 보장/물리 전원 소실 내구성·실제 UPS 프로토콜·장기/RT/native를 주장하지 않는다. 실물 UPS는 미완료 필수 입력이 아니다.

## [처리담당] RV-084

- 수용. `control::tests::earlier_alarm_during_report_cannot_leave_protected_complete_using_old_deadline`의 test-only Barrier scheduler로 report 중3초→500ms 알람을 실제 게시했다. 수정 전 보고서가3초를 반환해 실패했다([Red](../../06_evolution/imported_context/task_021_2026-11-02/red-report-race.log)).
- 최종 deadline 재관측과 동일 coordinator의 보수적 재poll 후 protection을 판정하도록 수정했다. 이후에 게시된 사건은 다음 poll의 snapshot으로 처리한다. InFlight 상태/원본 소유권을 초기화하지 않는다.
- 최종 source `a5fafcd`·검증파일139개 blob 일치, 두 Rust workspace각434/16features·Miri112·RT0/0·ext4 보호량/지연16조건·child 복구를 EV-043으로 수락/Closed 처리했다.

## [처리담당] RV-085

- 환경 관측에서 `/tmp`=tmpfs를 확인했다. 작은 파일/제어 회귀의 기능 근거는 유지하되 SSD 동기화 수락 근거로 쓰지 않는다.
- 예제 실행은 `--run <trusted-parent>`를 명시하도록 수정했다. 최종8/64MiB×지연 프로필과 프로세스 중단/복구를 승인된 `/data/build` ext4에서 두 Rust로 재실행하고 source/filesystem/명령을 EV-043에 고정한다.

## 변경표

| 구분 | 파일 | 내용 |
|---|---|---|
| 생성 | `control/emergency.rs`·`platform/power.rs` | startup 계획·알람 mailbox·순위/기한/부분 결과 |
| 수정 | `control/mod.rs`·`control/tests.rs`·`platform.rs` | 기존 정상 종료/sync 통합·단축 경합 재현 |
| 생성 | `examples/ups_simulation.rs`·`examples/support/ups/mod.rs` | 실제 파일/명시 parent·고정 프로필·프로세스 중단/복구 |
| 생성/수정 | `tests/tc_011_emergency.rs`·`tests/rt-allocation/src/tests.rs` | TC-01111개·알람/거절 할당0/0 |
| 수정 | README·Task/MS/API/TC/EV/상태 | 수락·미실행/후속·실물 필수 아님 명시 |

| 구분 | 파일 | 타입 | 역할 |
|---|---|---|---|
| 생성 | `platform/power.rs` | `PowerAlarm`, `PowerNotice` | 원 발생/차단 기한·통지 결과 |
| 생성 | `control/emergency.rs` | `EmergencyFile`, `EmergencyConfig`, `EmergencyFileState`, `EmergencyFileReport`, `EmergencyReport` | 계획·개별 barrier/양·전체 시간/보호/정리 사실 |
| 생성(내부) | `control/emergency.rs` | `ProtectedFile`, `EmergencyState` | 고정 파일 예약·원자 mailbox/관측 |
| 수정 | `control/mod.rs` | `ControlOwner` | optional startup 계획과 기존 coordinator 연결 |
| 생성(예제/내부) | `support/ups/mod.rs` | `Mode`, `CaseReport`, `Temp`, `ChildOwner`, `FaultFile` | 유한 fault/관측·소유 경로/프로세스 정리·실제 IO fault |

원문 로그는 byte-exact gzip과 읽기용 뷰로 보존했다. 물리 UPS 확보나 전원 차단 하드웨어를 미해결 승인/필수 입력으로 남기지 않는다. native T3·RT/장기/실앱은 별도 이월이다.

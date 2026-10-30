# TASK-022 — 실제 장비·플랫폼 검증 프로필

| 항목 | 값 |
|---|---|
| Intent / Milestone | INT-001 / MS-007 |
| Story | US-001·002·003·004·005 |
| 요구사항 | NFR-001 NFR-002 NFR-003 NFR-007 |
| 설계 | [도메인](../../03_design_artifacts/domain_model.md) · [인터페이스](../../03_design_artifacts/interface_contract.md) · [데이터](../../03_design_artifacts/data_model.md) |
| ADR | ADR-003~009 Accepted |
| 담당 | AI |
| 상태 | In Progress |
| 마지막 갱신일 | 2026-10-31 |
| 다음 액션 | PC/UPS 수락 확정·환경 수집 완료 → RT 부팅 확인·P0/제어 부하/기한/경로/예산 구체화 |
| 의존 | TASK-001 |
| 예상 소요 | 1~2 인일, 장비/리뷰 대기 제외 |

## 진입 조건

- TASK-001의 Gate 3 및 해당 ADR·상세 계약 승인, 위 의존 작업 완료.
- 실장비가 필요한 판정은 TASK-022의 승인 프로필과 유효 환경을 사용한다. 대역 검증과 구분한다.

## 변경 대상과 첫 검증

계획 경로: `test-profiles/; .cowork/05_verification/test_strategy.md`. 아직 생성된 코드 경로가 아니다.

첫 실패/리뷰 조건: **문서 검토: 장비/OS/CPU/SSD/FS/NIC/UPS·제어 주기·deadline 누락 탐지**.

- [x] 문서/환경 검토: 장비 미확정과 RT 설치/실행 혼동을 확인하고 ADR-011로 수락 기준·관측 사실을 구분했다.
- [ ] 최소 구현 후 관련 `TC-012`와 해당 케이스의 필요한 하위 시나리오를 통과시킨다.
- [ ] 리팩토링 후 영향 경로 회귀와 문서/예제의 일치를 확인한다.

## 완료 기준

- [x] Human 검증 장비·UPS 방법 확정 — 현재 PC 실측을 실제 장치 시험으로 대체, UPS 시뮬레이션 수락(ADR-011).
- [ ] P0 도구/부하·제어 주기/deadline·저장 경로/예산과 수치 수락 프로필 고정, 실행 커널별 환경 확인. GitHub native는 공개 완료 후 이월.
- [ ] 대상 revision·환경·명령·결과/미실행을 기록하고 요구사항 추적표의 해당 MS 칸만 갱신한다.
- [ ] 실패/포화·취소/종료·예산 경계를 해당 범위에서 검증한다.
- [ ] Human 리뷰 완료. 새 H/J 결정이 필요하면 근거와 대안을 기록한다.

테스트 정의: [TC-012](../../05_verification/test_case.md#tc-012). 제품 코드·테스트 실행 결과: 없음. TC 전체가 다른 Task까지 포함하면 이 Task의 하위 검증만 완료로 표시한다.

## 확정 장비·수락 방법 — 2026-10-31

Human 결정 [ADR-011](../../03_design_artifacts/adrs/ADR-011_local_pc_simulated_ups_verification.md)으로 현재 PC의 실측을 실제 장치 테스트 대체 근거로 수락한다. UPS는 실물을 제공받지 않고 시뮬레이션 결과로 판정하며 GitHub native는 저장소 공개 완료 후 수행한다. 본 Task는 환경/수락 프로필 준비로 착수하며 종합 성능 시험은 아직 실행하지 않았다.

| 항목 | 관측·수락 조건 |
|---|---|
| PC/OS | AMD Ryzen7 H255, 8코어16스레드 / Ubuntu26.04.1 LTS x86_64 |
| 메모리 | OS 인식59,750,248,448B(약55.6GiB). logger 예산과 OS cache/host 앱 메모리는 분리 |
| 실행 커널 | 7.0.0-42-generic / CONFIG_PREEMPT_DYNAMIC=y, CONFIG_PREEMPT_RT 미설정 |
| 설치된 RT 커널 | 7.0.0-38-realtime / CONFIG_PREEMPT_RT=y. 부팅 뒤 uname/sysfs/config를 다시 확인하고 RT 행 시험 |
| SSD/FS | CT1000T500SSD8: 루트 ext4(relatime). HP EX950512GB 2개: /data/build ext4(noatime), /data/db ext4. 시험별 실제 경로 고정 필요 |
| UPS | 시뮬레이션 수락: 모의 원점·전달 지연·약3초 기한·최대 보호량·진행 IO·실제 sync/중단·복구/receipt 대조 |
| native | 공개 완료 후 source hash 고정·6 contracts+audit+raw-memory. 현재 미실행 이월 |
| 남은 프로필 | 제어 주기/deadline·대표 입력/보호량·P0 도구·NIC/원격 범위·CPU affinity/우선순위·시험 경로/예산을 실행 전에 선언 |

[환경 관측 원문](../../06_evolution/imported_context/verification_policy_2026-10-31/environment.json). 이 기록은 환경 확인이며 제품 성능·RT 지연·UPS 합격 증거가 아니다. 실물 UPS/별도 장치 미제공을 착수 차단 사유로 사용하지 않는다. 커널/부트 설정 변경·재부팅은 수행하지 않았다.

# 테스트 케이스와 요구사항 추적

- INT-001 / 2026-10-26 / 상세 시험계획 Approved, **제품 테스트 전부 Not Run**.
- TDD 방식은 승인됨. 아래 세부 기대값은 ADR-008·009/추가 AC와 함께 기준선으로 승인됐다. FR-015 관련 시험은 별도 범위 승인 후 활성화한다.
- TC는 12개 검증 묶음이며 각 파라미터·실패 시나리오는 실행 때 독립 결과를 남긴다. 한 하위 시험 Pass를 TC 전체나 요구 전체 Pass로 표시하지 않는다.

## 1. 운영 규칙

설계 검토 [x]는 Human 합의, 구현 [x]는 해당 MS의 코드/리뷰 완료, 테스트 [x]는 요구된 시나리오와 환경의 유효 EV가 있을 때만 표시한다. 일부 구현·모의 UPS 성공·교차 빌드만으로 나머지를 체크하지 않는다. 코드/계약 변경으로 근거가 무효가 되면 해당 검증을 재검증 상태로 되돌리고 과거 EV는 보존한다.

테스트 파일 후보는 `tests/tc_NNN_*.rs`이며 실제 이름·명령은 구현 Task에서 고정한다. 제품 코드가 없으므로 지금 존재/실행된 파일로 오인하지 않는다. MS 인수 때 구현에는 commit/파일, 테스트에는 TC 하위 시나리오·EV/환경을 기록한다.

## 2. 케이스

<a id="tc-001"></a>
### TC-001 — 설정·공개 API·문서 계약

- 관련: MS-001·002·007 / FR-001 / NFR-004 / CON-001~007
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 설정 fixture: 총 64 MiB, quota 합 65 MiB; required 미지원 capability; default/각 feature 조합 | 재현 가능한 설정/입력·예산 기록 |
| 2 | 설정 검증·독립 consumer crate build·CLI API 문서 대조 | 잘못된 구성 시작 전 거절, 선택 의존성 없이 파일 API 사용 가능, 추적 누락 0 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

수집 범위·artifact/기술/승인 일치 리뷰와 compile-fail 포함. 구현 전 문서 검사와 이후 실행을 분리.

<a id="tc-002"></a>
### TC-002 — RT 수집·포화·원본 수명

- 관련: MS-002 / FR-002·007·008·009·012 / NFR-001·005
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | producer 2개 각각 slots=8, Important reserve=2; 원본 슬롯 4개; 할당/해제 감시 | 재현 가능한 설정/입력·예산 기록 |
| 2 | Normal 6개→추가 Normal→Important 2개→추가 Important; 거절/생략 loan Drop; 회수 중지→재개 | 예약 보호·즉시 거절/소유권 반환, RT allocation/deallocation 0, 재개 후 모든 슬롯 정확 1회 회수 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

FIFO·중요도 매핑·동일 취급 모드(예약 0)·일반 deadline·owner보다 오래 사는 핸들 compile-fail·메모리 순서 모델/해석 검증. p99는 TC-012에서 별도.

<a id="tc-003"></a>
### TC-003 — fanout·완료·설정과 종료 경합

- 관련: MS-002 / FR-003·007·012·013 / NFR-003·005
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 필수 파일 A, 선택 B, 완료 window=8, seq1 pending·seq2 완료 | 재현 가능한 설정/입력·예산 기록 |
| 2 | B 정지/포화·A sync 실패·조회 timeout·이력 축출·shutdown과 record 경쟁 | A/B 별도 상태, highest seq로 전체 완료 오판 없음, expired/unknown 구분, 안전한 owner 유지 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

입력 수락/대상 분배와 최종 상태 대조, 완료 추적 포화 시 신규 수락 거절, 예산 상한.

<a id="tc-004"></a>
### TC-004 — MCAP·시각·스키마·인덱스

- 관련: MS-003 / FR-002·003·013 / NFR-002·004·005
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 소형 256 B·원본 8 MiB, u32 초과 receipt, wall clock 역행, 두 schema version | 재현 가능한 설정/입력·예산 기록 |
| 2 | 다중 파일 쓰기→chunk 마감→sync→독립 reader 읽기, unknown encoding·잘못된 길이 주입 | receipt/원본 정확 일치, mono/real/source 보존, CRC/인덱스 검증, unknown payload opaque 반환 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

chunk target 초과와 index 최대점유 계측. encoding 호환/구버전 golden 파일. 구현 선택 crate만의 roundtrip 외 독립 판독 포함.

<a id="tc-005"></a>
### TC-005 — 보존·회전·복구·디스크 장애

- 관련: MS-003 / FR-003·011 / NFR-001·003·005·007
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 작은 disk quota·짧은 pre/post 창·겹친 event·reader pin; partial chunk/CRC 손상 파일 | 재현 가능한 설정/입력·예산 기록 |
| 2 | bytes/time/count/index 회전 각각 유발; ENOSPC/write/sync/rename 실패; 프로세스 중단 후 복구 | 보호 파일 무단 삭제 없음, 원본 보존·완전한 구간만 복구, 손상/미완료 범위 보고, RT 대기 없음 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

Linux/Windows/macOS 파일 공유/rename/durability 차이는 네이티브 실행. process kill은 실제 전원 차단과 다름.

<a id="tc-006"></a>
### TC-006 — 조회·동시 쓰기·재생

- 관련: MS-004 / FR-005·010 / NFR-002·005
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | active/closed 세그먼트, mono/real 역행·동일 시각 tie, 대용량 blob, controllable clock | 재현 가능한 설정/입력·예산 기록 |
| 2 | 기록 중 1,000건/256 KiB 페이지→cursor→만료; replay 0.5/1/2배속·pause·seek·느린 소비자 | safe offset 밖 비노출, 페이지 상한·결정적 tie-break, seek 뒤 old generation 제거, 제한 prefetch/핀 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

SourceTime domain 불명은 오류. 손상 index 재생성·범위 bounds. P5 응답 p95는 TC-012.

<a id="tc-007"></a>
### TC-007 — 원격·Rust API·CLI 사용자 경로

- 관련: MS-004·006 / FR-005·006·010·013 / NFR-002·004·007
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | loopback server/client, 느린 구독자, 8 MiB 원본, 불량 조각/미인증 요청 | 재현 가능한 설정/입력·예산 기록 |
| 2 | query/subscribe/replay→ACK 전 disconnect→재연결; CLI JSON/exit code/Ctrl-C | 완전 record 확인 뒤 ACK, 불명·Gap·cursor 만료 명시, reassembly/출력 bytes 상한, 비밀값 비노출 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

네이티브 실제 소켓·TLS/권한, dot-probe 연동 예제 포함. 표시 렌더링 시간은 완료 지표와 구분.

<a id="tc-008"></a>
### TC-008 — DB 매핑·충돌·재시도

- 관련: MS-006 / FR-004 / NFR-003·005
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 선택 InfluxDB 실제 버전과 전용 namespace, 동일 series/timestamp 2개, timeout/부분 실패 서버 | 재현 가능한 설정/입력·예산 기록 |
| 2 | 수치·이벤트 write/query 대조→충돌/out-of-order→응답 전 단절/제한 retry | 무단 병합을 전량 성공으로 표시하지 않음, 파일 원본 유지, API ack/조회 확인/unknown 분리 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

원본 bytes를 DB로 몰래 전송하지 않음. retry 배치 불변·queue/TTL 상한. v3 호환은 별도 프로필.

<a id="tc-009"></a>
### TC-009 — 운영 리포트·변경·전후 비교

- 관련: MS-005 / FR-014·017 / NFR-006
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 검증 용량 없음/만료·계단 입력·counter reset·출력 정지/회복·샘플링 필터 | 재현 가능한 설정/입력·예산 기록 |
| 2 | 관측→필터/route patch→producer별 ack→before/after 비교 | 배출 뒤 카운터 정확 일치, observed/validated/estimated 분리, 손실 안정 상태는 Overloaded, 조건 차이 표시 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

5초 이상 안정 예측 구간 오차 ≤20%, 기본 계측 처리량 overhead ≤5%는 2026-10-26 승인된 시험 목표이며 달성은 미검증임.

<a id="tc-010"></a>
### TC-010 — 간편 점검·선택 추천

- 관련: MS-005 / FR-016 / NFR-005·006; FR-015는 제안
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 임시 전용 경로, 시간 10초·쓰기 32 MiB·메모리 16 MiB 예산, 수신자 부재 | 재현 가능한 설정/입력·예산 기록 |
| 2 | plan만 실행→run→예산 중 먼저 도달/취소/공간 reserve 위협 | plan에는 부하 없음, 중단 뒤 IO 정리·운영 파일 불변, 결과에 실제 범위·중단/불충분 이유 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

FR-015 승인 시만 unknown hardware/사용자 설정 보존/자동 적용 금지 검증. 수치는 단위 fixture이지 제품 기본값 아님.

<a id="tc-011"></a>
### TC-011 — UPS 옵션·기한·실제 차단

- 관련: MS-006·007 / FR-018 / NFR-001·003
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | off 및 on 계획, rank 1/2·snapshot quota·최대 미동기화량, 전달 지연/안전여유 | 재현 가능한 설정/입력·예산 기록 |
| 2 | off 통지→on 모의 신호→중복/더 이른 기한→큰 IO 중 신호→실물 UPS 차단/재기동 | off는 전이/전용 예약 없음, deadline 연장 없음, 중요 순위/부분 결과, 진짜 차단 뒤 외부 receipt oracle 대조 |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

모의 제어 검증과 실제 신호 발생→차단 약3초의 장비별 검증을 별도 행/EV로 기록. 장비 없음은 Not Run.

<a id="tc-012"></a>
### TC-012 — 플랫폼·실사용 성능·장시간 종합

- 관련: MS-007 / NFR-001~007 / 주요 FR 종단
- 상태: Ready(절차 승인; 필요한 환경은 해당 Task에서 확보) / 최근 실행: Not Run / EV: 없음

| 단계 | 입력·동작 | 기대 결과 |
|---|---|---|
| 1 | 승인된 OS/CPU/SSD/FS/NIC/제어 주기·P0 baseline, R1/P0~P6와 예산 512 MiB/2 GiB/4 GiB | 재현 가능한 설정/입력·예산 기록 |
| 2 | P1~P5 완전 경로 반복→P6 24시간/장애→3 OS 파일 교환→dot-probe 조절 시나리오 | 요구 §12 수치·무손실 정상 부하·deadline miss 추가 0 목표·유한 자원·OS 기능 일치, 불충족별 Fail |
| 3 | 정리·카운터/원본/자원 대조 | 잔여 IO/메모리 수명 안전, 손실/미완료 범위 명시 |

P3 최소60초 및 RAM2배 이상, 마지막 sync까지 시간 포함. P4 receiver ACK, P5 P3동시. 실행 안 한 조합은 미검증.

## 3. 요구사항–설계–구현–테스트 추적 체크리스트

각 행의 MS/TASK/TC 링크는 계획이다. 코드/EV가 없으므로 구현·테스트 칸은 모두 미완료다. 요구별 여러 MS의 칸은 각각 체크해 부분 완료를 유지한다. FR-015는 제안 상태이며 Must에서 제외하되 행은 유지한다. ADR-008·009 및 상세 기준선 승인에 따라 설계 검토 칸만 체크했다. 제품 구현·테스트 칸은 계속 미완료다.

| 요구사항 | 의도·AC | 상세설계 | 설계 검토 | 구현 체크 — MS/TASK·코드 근거 | 테스트 체크 — MS/TC·EV | 상태/공백 |
|---|---|---|---|---|---|---|
| INT-001 | 기록 활용과 운영 조절의 전체 가치 | [설계](../03_design_artifacts/domain_model.md#1-규모에-맞춘-아키텍처) | [x] 기준선 승인 | MS-001: 설계 완료([TASK-001](../04_implementation/tasks/TASK-001.md)); 제품 구현 해당 없음<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md)<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-005: [TC-009](#tc-009)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| FR-001 | 재사용 라이브러리/수명주기·US-001 AC1 | [설계](../03_design_artifacts/interface_contract.md#13-상세-api-설계안) | [x] 기준선 승인 | [ ] MS-002: [TASK-002](../04_implementation/tasks/TASK-002.md), [TASK-006](../04_implementation/tasks/TASK-006.md)<br>[ ] MS-004: [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md)<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001)<br>[ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | 설계 승인·코드/EV 없음 |
| FR-002 | 로그·수치·원본·US-001 AC2 | [설계](../03_design_artifacts/data_model.md#1-논리-스키마) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md), [TASK-004](../04_implementation/tasks/TASK-004.md)<br>[ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | 설계 승인·코드/EV 없음 |
| FR-003 | 파일·완료·복구·US-002 AC2~4 | [설계](../03_design_artifacts/data_model.md#3-인덱스동시-읽기회전) | [x] 기준선 승인 | [ ] MS-002: [TASK-006](../04_implementation/tasks/TASK-006.md)<br>[ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md), [TASK-008](../04_implementation/tasks/TASK-008.md), [TASK-009](../04_implementation/tasks/TASK-009.md) | [ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-003: [TC-005](#tc-005) | 설계 승인·코드/EV 없음 |
| FR-004 | 시계열 DB·US-003 AC2~4 | [설계](../03_design_artifacts/data_model.md#5-db-projection과-장애-의미) | [x] 기준선 승인 | [ ] MS-006: [TASK-019](../04_implementation/tasks/TASK-019.md), [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-006: [TC-008](#tc-008) | 설계 승인·코드/EV 없음 |
| FR-005 | 기록 조회·US-003 AC2/5 | [설계](../03_design_artifacts/interface_contract.md#14-조회재생원격-계약안) | [x] 기준선 승인 | [ ] MS-004: [TASK-010](../04_implementation/tasks/TASK-010.md), [TASK-012](../04_implementation/tasks/TASK-012.md), [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-004: [TC-006](#tc-006)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| FR-006 | Rust API+CLI·기존 화면 연동 | [설계](../03_design_artifacts/ui_spec.md#2-명령-후보와-결과) | [x] 기준선 승인 | [ ] MS-004: [TASK-012](../04_implementation/tasks/TASK-012.md), [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | 설계 승인·코드/EV 없음 |
| FR-007 | 중요/동등 정책·US-001 AC3/4 | [설계](../03_design_artifacts/domain_model.md#4-수집소유권회수의-상세안) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md), [TASK-005](../04_implementation/tasks/TASK-005.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-002: [TC-003](#tc-003) | 설계 승인·코드/EV 없음 |
| FR-008 | RT 무대기·일반 기한 대기 | [설계](../03_design_artifacts/interface_contract.md#13-상세-api-설계안) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md) | [ ] MS-002: [TC-002](#tc-002) | 설계 승인·코드/EV 없음 |
| FR-009 | 별도 중요도/레벨 mapping | [설계](../03_design_artifacts/interface_contract.md#17-첫-검증-구성과-상세-리뷰-경계) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md)<br>[ ] MS-005: [TASK-016](../04_implementation/tasks/TASK-016.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-005: [TC-009](#tc-009) | 설계 승인·코드/EV 없음 |
| FR-010 | 자체 시간 재생·US-003 AC1/5 | [설계](../03_design_artifacts/interface_contract.md#14-조회재생원격-계약안) | [x] 기준선 승인 | [ ] MS-004: [TASK-011](../04_implementation/tasks/TASK-011.md), [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-004: [TC-006](#tc-006)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | 설계 승인·코드/EV 없음 |
| FR-011 | 연속/순환/이벤트·US-002 AC1 | [설계](../03_design_artifacts/data_model.md#4-보존과-복구) | [x] 기준선 승인 | [ ] MS-003: [TASK-008](../04_implementation/tasks/TASK-008.md) | [ ] MS-003: [TC-005](#tc-005) | 설계 승인·코드/EV 없음 |
| FR-012 | 다중 생산자·US-001 AC1~3 | [설계](../03_design_artifacts/domain_model.md#3-실행-구조와-병목-경계) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md), [TASK-004](../04_implementation/tasks/TASK-004.md), [TASK-005](../04_implementation/tasks/TASK-005.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| FR-013 | 다중 파일/route·US-001 AC1 | [설계](../03_design_artifacts/domain_model.md#3-실행-구조와-병목-경계) | [x] 기준선 승인 | [ ] MS-002: [TASK-005](../04_implementation/tasks/TASK-005.md), [TASK-006](../04_implementation/tasks/TASK-006.md)<br>[ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md)<br>[ ] MS-004: [TASK-012](../04_implementation/tasks/TASK-012.md)<br>[ ] MS-005: [TASK-016](../04_implementation/tasks/TASK-016.md) | [ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-005: [TC-009](#tc-009) | 설계 승인·코드/EV 없음 |
| FR-014 | 병목·여유·잔여시간·US-004 | [설계](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) | [x] 기준선 승인 | [ ] MS-005: [TASK-014](../04_implementation/tasks/TASK-014.md), [TASK-015](../04_implementation/tasks/TASK-015.md) | [ ] MS-005: [TC-009](#tc-009)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| FR-015 | 제안: 질문 프리셋·설정 추천 | [설계](../03_design_artifacts/ui_spec.md#2-명령-후보와-결과) | [ ] 별도 제안 | [ ] MS-005: [TASK-018](../04_implementation/tasks/TASK-018.md) | [ ] MS-005: [TC-010](#tc-010) | 제안·범위 미승인 |
| FR-016 | 내장 간편 점검·US-004 | [설계](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) | [x] 기준선 승인 | [ ] MS-005: [TASK-017](../04_implementation/tasks/TASK-017.md) | [ ] MS-005: [TC-010](#tc-010)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| FR-017 | 개발자 조절·전후 비교·US-004 | [설계](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) | [x] 기준선 승인 | [ ] MS-005: [TASK-016](../04_implementation/tasks/TASK-016.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md) | [ ] MS-005: [TC-009](#tc-009)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | 설계 승인·코드/EV 없음 |
| FR-018 | 기본 off·UPS 약3초·US-002 AC6~10 초안 | [설계](../03_design_artifacts/interface_contract.md#16-동기화긴급-동작의-구체안) | [x] 기준선 승인 | [ ] MS-006: [TASK-021](../04_implementation/tasks/TASK-021.md)<br>[ ] MS-007: [TASK-024](../04_implementation/tasks/TASK-024.md) | [ ] MS-006: [TC-011](#tc-011)<br>[ ] MS-007: [TC-011](#tc-011)<br>[ ] MS-007: [TC-012](#tc-012) | 추가 AC 승인·실물 미검증 |
| NFR-001 | RT 무대기·affinity·p99 진단 | [설계](../03_design_artifacts/domain_model.md#7-플랫폼과-보장의-범위) | [x] 기준선 승인 | [ ] MS-002: [TASK-003](../04_implementation/tasks/TASK-003.md), [TASK-004](../04_implementation/tasks/TASK-004.md)<br>[ ] MS-003: [TASK-009](../04_implementation/tasks/TASK-009.md)<br>[ ] MS-006: [TASK-021](../04_implementation/tasks/TASK-021.md)<br>[ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-003: [TC-005](#tc-005)<br>[ ] MS-006: [TC-011](#tc-011)<br>[ ] MS-007: [TC-011](#tc-011)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| NFR-002 | RAM 이후 활용 완료·P0~P5 | [설계](../03_design_artifacts/interface_contract.md#14-조회재생원격-계약안) | [x] 기준선 승인 | [ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md)<br>[ ] MS-004: [TASK-010](../04_implementation/tasks/TASK-010.md), [TASK-012](../04_implementation/tasks/TASK-012.md)<br>[ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-004: [TC-006](#tc-006)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| NFR-003 | 24h·완료/복구·장애 | [설계](../03_design_artifacts/data_model.md#4-보존과-복구) | [x] 기준선 승인 | [ ] MS-002: [TASK-006](../04_implementation/tasks/TASK-006.md)<br>[ ] MS-003: [TASK-008](../04_implementation/tasks/TASK-008.md), [TASK-009](../04_implementation/tasks/TASK-009.md)<br>[ ] MS-006: [TASK-019](../04_implementation/tasks/TASK-019.md), [TASK-021](../04_implementation/tasks/TASK-021.md)<br>[ ] MS-007: [TASK-024](../04_implementation/tasks/TASK-024.md) | [ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-003: [TC-005](#tc-005)<br>[ ] MS-006: [TC-008](#tc-008)<br>[ ] MS-006: [TC-011](#tc-011)<br>[ ] MS-007: [TC-011](#tc-011)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| NFR-004 | Rust 재사용·버전·호환 | [설계](../03_design_artifacts/domain_model.md#2-패키지와-의존-방향) | [x] 기준선 승인 | [ ] MS-002: [TASK-002](../04_implementation/tasks/TASK-002.md)<br>[ ] MS-004: [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md)<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007) | 설계 승인·코드/EV 없음 |
| NFR-005 | bytes/slots/인덱스/작업 예산 | [설계](../03_design_artifacts/domain_model.md#5-자원-예산과-설정) | [x] 기준선 승인 | [ ] MS-002: [TASK-004](../04_implementation/tasks/TASK-004.md), [TASK-005](../04_implementation/tasks/TASK-005.md)<br>[ ] MS-003: [TASK-007](../04_implementation/tasks/TASK-007.md), [TASK-008](../04_implementation/tasks/TASK-008.md)<br>[ ] MS-004: [TASK-010](../04_implementation/tasks/TASK-010.md)<br>[ ] MS-005: [TASK-017](../04_implementation/tasks/TASK-017.md)<br>[ ] MS-007: [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-002: [TC-002](#tc-002)<br>[ ] MS-002: [TC-003](#tc-003)<br>[ ] MS-003: [TC-004](#tc-004)<br>[ ] MS-003: [TC-005](#tc-005)<br>[ ] MS-004: [TC-006](#tc-006)<br>[ ] MS-005: [TC-010](#tc-010)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| NFR-006 | 관측/검증/추정·계측 비용 | [설계](../03_design_artifacts/interface_contract.md#15-설정운영-계측점검-계약안) | [x] 기준선 승인 | [ ] MS-005: [TASK-014](../04_implementation/tasks/TASK-014.md), [TASK-015](../04_implementation/tasks/TASK-015.md), [TASK-016](../04_implementation/tasks/TASK-016.md), [TASK-017](../04_implementation/tasks/TASK-017.md)<br>[ ] MS-007: [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-005: [TC-009](#tc-009)<br>[ ] MS-005: [TC-010](#tc-010)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| NFR-007 | 3 OS·Tier별 검증·US-005 | [설계](../03_design_artifacts/domain_model.md#7-플랫폼과-보장의-범위) | [x] 기준선 승인 | [ ] MS-002: [TASK-002](../04_implementation/tasks/TASK-002.md)<br>[ ] MS-003: [TASK-009](../04_implementation/tasks/TASK-009.md)<br>[ ] MS-004: [TASK-013](../04_implementation/tasks/TASK-013.md)<br>[ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-024](../04_implementation/tasks/TASK-024.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001)<br>[ ] MS-003: [TC-005](#tc-005)<br>[ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| CON-001 | Rust | [설계](../03_design_artifacts/tech_stack.md) | [x] 기준선 승인 | [ ] MS-002: [TASK-002](../04_implementation/tasks/TASK-002.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001) | 설계 승인·코드/EV 없음 |
| CON-002 | Linux/macOS/Windows·Linux RT | [설계](../03_design_artifacts/domain_model.md#7-플랫폼과-보장의-범위) | [x] 기준선 승인 | [ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-024](../04_implementation/tasks/TASK-024.md) | [ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| CON-003 | dot-probe 첫 적용 | [설계](../03_design_artifacts/ui_spec.md#1-사용자-흐름) | [x] 기준선 승인 | [ ] MS-006: [TASK-020](../04_implementation/tasks/TASK-020.md), [TASK-026](../04_implementation/tasks/TASK-026.md)<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | [ ] MS-004: [TC-007](#tc-007)<br>[ ] MS-006: [TC-007](#tc-007)<br>[ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |
| CON-004 | 공식 조사·기록 | [설계](../03_design_artifacts/adrs/ADR-009_storage_transport_toolchain.md) | [x] 기준선 승인 | MS-001: 설계 완료([TASK-001](../04_implementation/tasks/TASK-001.md)); 제품 구현 해당 없음 | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001) | 설계 승인·코드/EV 없음 |
| CON-005 | 조사 후 한 번에 한 질문 | [설계](../03_design_artifacts/domain_model.md#9-설계에서-남은-결정) | [x] 기준선 승인 | MS-001: 설계 완료([TASK-001](../04_implementation/tasks/TASK-001.md)); 제품 구현 해당 없음 | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001) | 설계 승인·코드/EV 없음 |
| CON-006 | ROS2는 첫 적용 후 T1 | [설계](../03_design_artifacts/domain_model.md#9-설계에서-남은-결정) | [x] 기준선 승인 | MS-001: 설계 완료([TASK-001](../04_implementation/tasks/TASK-001.md)); 제품 구현 해당 없음<br>[ ] MS-007: [TASK-025](../04_implementation/tasks/TASK-025.md) | MS-001: [설계 리뷰 완료](quality_gate.md); 제품 TC 실행 해당 없음<br>[ ] MS-002: [TC-001](#tc-001)<br>[ ] MS-007: [TC-001](#tc-001) | 설계 승인·코드/EV 없음 |
| CON-007 | SSD·장비별 저장 예산 | [설계](../03_design_artifacts/domain_model.md#5-자원-예산과-설정) | [x] 기준선 승인 | [ ] MS-007: [TASK-022](../04_implementation/tasks/TASK-022.md), [TASK-023](../04_implementation/tasks/TASK-023.md) | [ ] MS-007: [TC-012](#tc-012) | 설계 승인·코드/EV 없음 |

## 4. MS 인수 시 갱신 순서

1. Task 상세와 registry 상태를 코드·리뷰 결과에 맞춘다.
2. TC 하위 결과에 revision·환경·명령·기대/실제·결과 파일을 기록하고 verification_evidence에 EV를 등록한다.
3. 위 요구 행에서 해당 MS의 구현/테스트 칸만 체크한다. 미실행 환경·장비는 GAP로 연결한다.
4. MS Done은 관련 필수 요구의 해당 범위와 Human 리뷰가 완료된 때만 표시한다. Gate 4/5는 별도 판정한다.

최근 제품 실행 결과: 없음. 문서의 ID/링크 검사는 제품 테스트 통과 증거가 아니다.

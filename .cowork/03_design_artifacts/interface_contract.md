# 핵심 인터페이스 계약

- 관련 Intent: INT-001
- 상태: Approved — 2026-10-26 상세설계·ADR-008/009 기준선 채택. 공개 함수 표기는 설계 계약이며 컴파일/수명·실제 호환 검증은 구현 Task에서 수행한다.
- 해석 우선순위: §13~17과 ADR-008·009가 현재 상세 동작 계약이며, 검증 방법/시점은 2026-10-31 ADR-011을 우선한다. §1~12의 초기 제안/미정 표현은 여기서 구체화한 범위에서 대체된다. 장비별 수치·알고리즘 증명은 미검증이다.
- 승인 기준선: ADR-003~009, US-001~005(US-002 AC6~10 포함)
- 후속 요구: FR-018 — 선택 옵션·기본 비활성·전체 약3초/중요 파일 우선 기록 계약 유지. 2026-10-31 ADR-011로 UPS는 시뮬레이션 판정, 실제 장치 시험은 현재 PC 실측으로 대체 수락한다.
- 갱신: 2026-10-27 / AI

## 1. 제공자·소비자와 실행 경계

| 경로 | 제공자/호출 주체 | 처리 책임 | 완료 의미 |
|------|-----------------|-----------|-----------|
| 등록·설정 | 애플리케이션/관리 클라이언트 | 로거 관리 경로 | 검증·등록/설정 적용 결과 |
| 기록 제시 | 등록된 생산자 | 제한된 수집 경로 | 수락 또는 거절. 저장 완료가 아님 |
| 완료 확인 | 애플리케이션/관리 클라이언트 | 백그라운드 출력/완료 관리 | 요청한 대상/완료 경계까지의 상태 |
| 조회·재생·구독 | 분석/모니터링 클라이언트 | 조회/전달 경로 | 소비 가능한 응답/전달. GUI 표시 완료와 구분 |
| 관측·조절 비교 | 개발자/클라이언트 | 계측·보고 경로 | 조건과 근거가 포함된 관측/검증/추정 보고 |
| 점검·종료 | 관리 클라이언트 | 관리·백그라운드 경로 | 완료/실패/시간 제한·미완료 범위를 포함한 결과 |

RT 호출은 기록 제시까지로 제한하는 안이다. 대기가 필요한 완료 확인·종료·점검·조회는 일반/관리 경로에 둔다. Rust 라이브러리 경계를 먼저 검토하고 원격 프로토콜·실행 배치는 ADR-007·009를 따른다.

## 2. 작업별 계약 검토안

| 작업 | 입력 의미 | 반환/관측 의미 | 검토할 제약 |
|------|-----------|---------------|-------------|
| 생산자/채널 등록 | 목적·스키마·출력 매핑·정책·크기/자원 예산 | 검증된 핸들·설정 revision 또는 구성 오류 | RT 실행 전 등록, 종료 중 등록·등록 수 상한 |
| 기록 가능 여부 확인 | 핸들·레벨/필터·선택 정보 | 현재 정책에서 기록 대상인지 | 공간을 예약한 의미는 아님. 실제 제시는 거절될 수 있음 |
| 소형 기록 제시 | 핸들·시각/순서·작은 구조화 payload·보존 중요도 | 수락 식별 또는 거절 이유 | RT 무대기. payload 생성·포맷·할당과 호출 비용 경계 정의 |
| 원본 기록 제시 | 유효 크기/메타데이터·제한된 원본 소유권 핸들 | 수락 시 소유권 이관, 거절 시 원본 반환을 우선 검토 | 최종 참조 해제/회수·거절 경로의 추가 작업과 수명 보장 |
| 완료 확인 | 기록/범위·대상 집합·원하는 완료 경계·제한 시간 | 대상별 완료/실패/폐기/미처리와 시간 제한 | 일반 경로. 일부 대상만 완료된 결과를 표현 |
| 조회·구독·재생 | 시간/채널/범위·필터·페이지/스트림 상한·재생 제어 | 읽을 수 있는 기록·결측/완료 범위·소비 흐름 | 동시 쓰기 가시성·취소·느린 소비자·메모리/IO 상한 |
| 보고서 조회 | 설정/장비/관측 구간·경로/채널 선택 | 부하·완료·지연·손실·예산·기준 유효성·추정 | 수집 근거와 불확실성을 표시. 계산은 RT 밖에서 수행 |
| 설정 제안/적용 | 현재 revision·개발자 조절 의도·대상 범위 | 검증된 변경안·효과 가정·명시적 적용 결과/새 revision | 고정값 유지·실행 중 변경 범위·되돌리기·기준 만료 |
| 간편 점검 | 재사용할 부하 프로필·완료 경로·시간/CPU/메모리/쓰기/공간 예산 | 실제 시험 범위·통과/실패/중단·추정과 추천 | 명시적 실행·임시 자원 관리·운영 기록 보호·상대 수신자 부재 |
| 종료 | 배출·동기화 범위·종료 제한 시간 | 최종 완료/미완료·실패 범위 | RT 밖에서 수행. 강제 종료/프로세스 중단의 보장 범위 별도 |
| 전원 이상 긴급 기록 | 예고 신호·차단 기한/남은 예산·사전 등록 중요 파일 계획 | 요청 인수와 파일별 영속 완료/실패/미완료 | FR-018. 전체 약 3초에 전달 지연·종료 여유 포함. 상세는 §12 검토안 |

위 작업명은 역할 설명이며 코드 API가 아니다. 어댑터가 요구하는 런타임/네트워크 기능을 수집 API의 필수 의존성으로 정하지 않았다.

## 3. 입력과 소유권·실패

### 3.1 등록·입력·동기 반환

이 절부터의 상세 동작은 권고안이다. 등록은 일반/관리 경로에서 수행하고 생산자·채널·스키마·최대 입력 크기·출력 계획·중요도 정책·자원 상한을 검증한다. RT 핸들은 하나의 직렬 호출 주체가 소유하는 안이다. 여러 스레드가 같은 논리 채널을 쓰려면 생산자 핸들을 각각 등록한다. 핸들의 임의 복제나 동시에 호출하는 공유 핸들은 아직 지원 계약으로 확정하지 않는다.

| 반환 분류 | 의미 | 원본 소유권 / 식별 |
|-----------|------|--------------------|
| Accepted | 처리 책임과 제한된 수집 공간을 로거에 이관 | 원본 핸들 소비, 수락 식별자 반환. 출력 완료 보장 아님 |
| Skipped | 활성 필터·샘플링 등 설정에 따른 의도적 생략 | 원본 핸들 반환, 수락 식별자 없음 |
| Rejected | 잘못된 입력·자원 한계·종료 등으로 수락하지 못함 | 원본 핸들 반환, 수락 식별자 없음 |

거절 이유는 최소한 `InvalidHandle`, `SchemaMismatch`, `TooLarge`, `Capacity(scope)`, `Contended`, `Closing`을 구분하는 안이다. `scope`는 수집 슬롯·원본 bytes·완료 추적 등 제한된 코드로 표현한다. RT 반환에 오류 문자열 포맷·동적 할당·사용자 콜백을 요구하지 않는다. 경합 재시도도 정해진 연산 한도 안에서 끝나야 하며 그 한도와 알고리즘은 후속 검증 사항이다. 등록된 계약 밖의 입력을 필터 생략으로 숨기지 않는다.

RT 수집 호출 내부는 사전 준비된 메모리만 사용하고 동적 할당·블로킹 잠금·OS I/O를 실행하지 않는 방향으로 검증한다. 제한된 복사·자원 판정·계측·동기화 연산까지 전체 경로 비용에 포함한다. 무대기 반환 계약과 특정 OS/장비에서의 호출 지연 목표 달성은 별개다.

소형 입력은 등록된 크기 상한 안의 바이트를 수집 공간으로 복사하는 안이다. 호출 뒤에는 애플리케이션 입력을 빌려 두지 않는다. 문자열 포맷·직렬화·사용자 payload 생성 비용은 호출 전 준비 비용으로 별도 계측한다. 원본 입력은 전체 바이트 복사 대신 §3.2의 제한된 소유권 핸들을 소비한다. 사전 기록 가능 여부 확인은 힌트이며 공간 예약·수락 보장이 아니다.

목표의 256 B는 소형 기록 시험 조건이며 API의 고정 최대 payload가 아니다. 32생산자·16파일·128채널 역시 검증 규모이며 제품의 고정 최대 개수로 변환하지 않는다. 실제 슬롯/bytes/객체 크기 상한은 등록/구성 계약에서 정한다.

### 3.2 원본 수명과 회수

RT 입력의 우선 검토안은 **사전 등록한 제한된 풀의 대여 핸들**이다. 생산자는 대여 중에만 내용을 쓰고, 수락 후에는 읽기·쓰기·재사용하지 않는다. 분배 이후에는 출력들이 불변 원본을 공유한다. 일반 `Vec`·`Arc`·외부 SDK 버퍼를 무조건 RT 입력으로 받는 계약은 두지 않는다. 외부 버퍼는 일반 경로에서 복사/등록하거나 해당 수명·회수 방식을 검증한 어댑터로 연결하는 안이다. 센서 SDK에서 풀로 입력하는 과정과 파일/네트워크 출력까지 완전한 zero-copy를 보장하지 않는다.

```mermaid
stateDiagram-v2
    [*] --> Free
    Free --> Loaned: 제한된 풀에서 대여
    Loaned --> Loaned: 거절 또는 생략 시 핸들 반환
    Loaned --> Accepted: 수락 시 소유권 이관
    Accepted --> InUse: 출력별 불변 참조
    Accepted --> ReclaimPending: 출력 전 폐기 또는 실패
    InUse --> ReclaimPending: 모든 참조와 진행 중 IO 해제
    Loaned --> ReclaimPending: 미제시 원본 반납
    ReclaimPending --> Free: 안전한 회수 경로에서 재사용 가능 처리
```

| 상황 | 처리 책임과 수명 |
|------|------------------|
| 거절·필터 생략 | 같은 핸들을 호출자에게 돌려준다. 내부 예약은 되돌리며 숨은 원본 참조를 남기지 않는다. 재제시/반납은 호출자가 선택한다. |
| 정상 수락·출력 분기 | 로거가 원본 수명을 책임진다. 각 출력의 참조와 복사/작업 버퍼를 예산에 포함한다. |
| 한 출력의 실패·폐기 | 그 출력이 더 이상 접근하지 않을 때만 참조를 해제한다. 다른 출력의 원본을 무효화하지 않는다. |
| 조회 취소·완료 대기 시간 초과 | 대기/조회만 끝낸다. 수락 기록 처리나 진행 중 IO를 암묵적으로 취소하지 않는다. |
| 종료·강제 중단 요청 | 신규 수락을 닫고 남은 참조/IO를 추적한다. 종료 시간이 지났다고 사용 중 메모리를 재사용하지 않는다. |
| 호출자 핸들·관리 객체 해제 | 풀에 접근 가능한 핸들이 남아 있으면 기반 메모리도 살아 있어야 한다. 최종 풀 소멸은 관리 경로에서 수행하도록 수명 계약을 검증한다. |

대여 핸들의 반납/Drop은 RT에서 큰 payload 해제·외부 SDK 반납 콜백·무한 재시도를 실행하지 않는 계약을 검토한다. 생산자 측에서 핸들이 소멸(Drop)되더라도 즉각적인 메모리 해제(OS deallocation)를 절대 수행하지 않고, 비소유/비차단 역방향 회수 큐(Recycle Queue)로 인계되어 백그라운드 워커에 의해 풀로 환원되는 RAII 안전 래퍼 구조를 검증한다. 회수 대기열이 찼을 때 동기 해제나 누수를 택하는 폴백을 두지 않는다. 슬롯별 반납 상태 또는 모든 대여 슬롯을 감당하는 회수 용량 등 **반납 경로가 포화돼도 반환 책임을 잃지 않는 구조**를 구현 전에 증명한다. 미제시 대여분·실패 후 반환분·회수 대기분도 계속 점유로 센다. 실제 자료구조·메모리 순서·관리 객체의 최종 해제 방식은 미정이며, 검증 전 RT 안전성 달성으로 표시하지 않는다.

근거: Rust의 [Arc 공식 문서](https://doc.rust-lang.org/std/sync/struct.Arc.html)는 마지막 참조 소멸 시 내부 값도 Drop됨을 설명한다. 따라서 공유 포인터를 사용했다는 이유만으로 최종 해제 비용이 RT 밖으로 이동했다고 판단할 수 없다. 제한된 풀/반납 구조는 이 프로젝트의 설계 제안이며 Arc 채택 결정이 아니다.

### 3.3 수락 시점과 설정·출력 계획

수락 판단과 공개는 하나의 논리적 경계를 갖는 안이다. 수락 식별자를 반환할 때는 기록 내용/원본 핸들·고정된 설정 revision·출력 계획·수집 및 추적 예산의 인수가 완료돼 있어야 한다. 중간 예약 실패는 모두 되돌린 뒤 거절하며 부분 수락을 동기 성공으로 반환하지 않는다. 구체 동시성 알고리즘은 미선정이다.

출력 계획에는 대상별 필수/선택 여부와 지원 완료 경계를 넣는다. **수락은 해당 계획의 처리 시도 인수이며 모든 출력 큐 공간이나 향후 성공의 원자적 예약이 아니다.** 후속 분배에서 필수 대상이 포화/실패하면 해당 대상의 실패/폐기를 명시하고 전체 요청을 성공으로 만들지 않는다. 선택 대상의 포화도 숨기지 않는다. 필수 대상 전체의 공간을 미리 예약하는 대안은 더 강한 초기 인수 조건을 주지만 RT 판단 비용과 출력 간 결합이 늘어 권고안에서는 보류한다.

한 기록에는 하나의 불변 revision/출력 계획을 연결하고 재시도·조회에도 유지한다. 설정 변경으로 과거 수락 기록의 대상을 바꾸지 않는다. 이전 설정/출력 계획의 수명도 제한 예산에 포함하고 회수 전 revision을 무제한 쌓지 않는다. 전역 동시 적용·실행 중 변경 가능 항목은 OQ-005로 남는다.

### 3.4 타임스탬프 삼원화 및 시계 동기화 계약

센서 퓨전 및 다중 생산자 분석을 위해 기록 봉투 메타데이터는 3가지 시간축을 선택적으로 다룬다.

| 시간축 | 정의 및 출처 | 용도 및 보장 |
|--------|--------------|--------------|
| `MonoTime` | 시스템 단조 시계 (`CLOCK_MONOTONIC` 또는 OS 동등물) | 필수. 제어 루프 주기 분석, 정렬, 로거 내부 지연 측정. 역행 없음 보장 |
| `RealTime` | 벽시계 (`CLOCK_REALTIME` 또는 OS 동등물) | 필수. 외부 시스템 이벤트 연관 및 절대 시간 조회. NTP/PTP 슬루/점프로 역행/도약 가능 |
| `SourceTime` | 센서 하드웨어/드라이버 생성 타임스탬프 | 선택. 카메라 셔터 시점, LiDAR 패킷 시각 등 정밀 센서 퓨전용 |

- NTP/PTP 시계 동기화로 인한 `RealTime`의 불연속 변화(점프)가 발생하더라도 `MonoTime`은 영향을 받지 않으며, 세션 메타데이터에 시계 기준점(Base Clock Offset)과 보정 이벤트를 기록해 재생/조회 시 시간축 왜곡을 보정할 수 있게 한다.
- 시간 기반 재생(Playback)의 기본 정렬 기준은 `MonoTime`을 권장하며, 벽시계 기반 질의 시에는 `RealTime` 인덱스를 사용한다.

### 3.5 스키마 식별 및 진화(Schema Evolution) 계약

데이터 구조의 변경과 장기 보존 로그의 판독 호환성을 위해 스키마 메타데이터 계약을 검토한다.

- **스키마 등록**: 모든 채널은 등록 시 `(schema_id, schema_version)` 및 필드 기술자(Descriptor/Schema Text)를 로깅 세션 메타데이터에 기록한다.
- **하위 호환 디코딩**: 신규 필드 추가는 선택적/기본값 허용 방식으로 설계하여 구버전 판독기가 신규 로그의 미인식 필드를 건너뛰고(Skip), 신규 판독기가 구버전 로그의 누락 필드를 기본값으로 채울 수 있도록 한다.
- **불가역 변경 격리**: 필드 타입 변경 등 호환 불가능한 변경은 새로운 `schema_id`를 발급하거나 주 버전을 올려 이전 로그 판독과의 혼선을 방지한다.

## 4. 완료 상태와 무결성

| 상태 | 의미 | 보장하지 않는 것 |
|------|------|------------------|
| 수락 | 로거가 해당 기록 처리를 받아들임 | 모든 대상 완료·영속 저장 |
| 파일 쓰기 완료 | 지정한 파일 쓰기 경계에 도달 | 전원 장애 영속성·다른 출력 완료 |
| 영속 동기화 완료 | 지정 범위와 필요한 메타데이터 동기화 결과 확인 | 장치 계약 밖의 고장 보장·미동기화 꼬리의 무손실 |
| 원격 활용 전달 완료 | 지정 수신 프로그램의 검증/소비 전달 경계 확인 | GUI 렌더링·다른 DB/파일 완료 |
| 실패/정책 폐기/미처리 | 경로별 원인·대상·기록/범위가 끝나지 않은 상태 | 임의로 성공 또는 측정된 처리량에 포함 |

### 4.1 식별·순서·완료 확인

수락 식별자는 `(session_id, producer_id, accepted_seq)`의 의미를 갖는 작은 비소유 토큰으로 제안한다. `accepted_seq`는 해당 생산자에서 수락될 때만 증가한다. 센서 프레임 번호/애플리케이션 발생 순서는 별도 입력 메타데이터이며 생략·거절은 반환/카운터로 센다. 세션 재시작이나 생산자 재등록 때 식별자를 재사용하지 않고 sequence 소진 시에는 관리 경로에서 갱신하거나 명시적으로 거절한다.

생산자별 수락 순서는 추적하되 전체 생산자·채널·출력의 전역 순서를 보장하지 않는 안이다. 출력이 순서대로 보이도록 요구하면 실패/폐기된 구간도 함께 표시해야 한다. 최고 완료 sequence만으로 모든 이전 기록의 완료를 추정하지 않는다. 큰 원본과 작은 기록의 처리 완료 순서도 다를 수 있다.

완료 조회는 기록 토큰 또는 **등록된 생산자 집합과 생산자별 수락 상한을 고정한 범위**를 받는다. 범위를 만든 뒤의 수락/신규 생산자는 자동 포함하지 않는다. 전 생산자를 같은 순간에 잘라 내는 전역 원자적 스냅샷은 보장하지 않는다. 조회에는 대상 선택·대상별 완료 경계·대기 기한을 명시하고 폴링/대기는 일반 경로에서만 수행한다. 조회 자체가 추가 전송/동기화를 시작하지 않는다. 요청한 경계가 기록의 처리 계획에 없으면 미지원/계획 오류를 반환하며, 명시적 동기화 실행 요청은 별도 관리 계약에서 정한다.

### 4.2 대상별 진행·결과와 대기 결과

| 구분 | 표현할 의미 |
|------|-------------|
| 진행 상태 | 대상별 최고 확인 경계와 아직 진행 중인 범위 |
| 완료 | 요청한 경계에 도달한 기록/범위 |
| 실패 | 더 진행할 수 없는 원인·대상·범위와 이미 확인한 경계 |
| 정책 폐기 | 구성된 정책에 따라 종료한 대상/범위. 수락 거절과 별도 집계 |
| 원격 결과 불명 | 전송 뒤 연결 단절 등으로 수신 활용 여부를 확인하지 못함. 미전달/성공으로 단정하지 않음 |
| 이력 만료 | 상세 조회 보존 범위를 벗어나 확인 불가. 성공/실패/아직 미처리로 변환하지 않음 |
| 대기 시간 초과/조회 취소 | 조회 작업의 결과이며 기록의 최종 처리 상태가 아님. 마지막 상태를 함께 반환 |
| 미지원/잘못된 범위 | 대상이 제공하지 않는 경계·알 수 없는 대상/세션·유효하지 않은 토큰. 대기 전에 오류 반환 |

쓰기 완료 후 동기화에 실패하면 쓰기 완료 사실을 유지하고 영속 동기화 실패를 별도로 표시한다. `Pending → Written → Synced` 하나로 모든 대상을 모델링하지 않는다. DB/원격의 경계는 어댑터가 선언하고, 수신 애플리케이션의 확인이 없으면 원격 활용 완료를 반환하지 않는다. 재시도로 생길 수 있는 중복/불명 상태를 성공으로 숨기지 않으며 exactly-once 전달은 아직 보장하지 않는다.

범위 결과는 요청한 대상·경계와 완료/실패/폐기/미처리/불명/만료 범위를 돌려주는 안이다. 필수 대상의 모든 요청 범위가 해당 경계에 도달해야 필수 범위 성공이다. 선택 대상도 요청에 포함하면 그 상태를 함께 보고한다. 예를 들어 파일은 쓰기 완료, DB는 실패, 원격은 대기 중이면 전체 출력 완료로 응답하지 않는다.

파일 쓰기 완료는 해당 기록의 인코딩 바이트를 사용자 공간 버퍼에서 OS 쓰기 경계까지 넘긴 의미로 검토한다. 파일 영속 동기화는 그 기록/범위를 포함하는 동기화 작업의 성공과 연결한다. Rust [File::sync_all 공식 문서](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all)는 내용/메타데이터 동기화를 별도 연산으로 설명하며 File Drop의 오류는 무시됨을 명시한다. 파일 Drop을 정상 종료/동기화 성공으로 취급하지 않는다. 파일명·디렉터리·회전/인덱스 복구까지 포함한 영속 계약, OS별 구현과 장치 보장은 별도 검토 대상이다.

### 4.3 제한된 완료 이력과 수량 대조

상세 토큰/범위 조회에는 보존 시간·항목/bytes·동시 대기자 상한을 두고 유효 조회 범위를 노출한다. 완료 구간은 연속 범위로 압축하고 결측/실패 구간도 제한된 예산으로 관리하는 안이다. 구간 분열로 추적 예산이 부족하면 신규 수락을 거절하거나 계약된 이력 만료를 명시한다. 미처리 기록 상태를 몰래 버려 성공으로 만들지 않는다. 토큰을 보관한다고 원본 메모리나 상세 이력이 영구 보존되는 것은 아니다. 정확한 상한·집계/만료 방식은 구현 전 확정한다.

고정된 생산자/대상 집합·revision·대조 구간에서 다음 수량을 연결한다.

- 제시 수 = 수락 수 + 의도적 생략 수 + 수락 거절 수.
- 대상별 배정 수 = 해당 대상의 최종 성공 수 + 최종 실패 수 + 정책 폐기 수 + 진행 중 수 + 결과 불명 수. 최종 성공은 선언한 대상 완료 경계 기준이며, 쓰기/동기화 누적 카운터를 더해 기록 수로 쓰지 않는다.
- 대상에 배정되지 않은 기록은 그 대상의 분모에서 제외한다. fan-out 합계는 수락 수보다 클 수 있다.

상세 이력 만료 뒤에도 누적 집계의 범위/손실 여부를 명시한다. 원격 결과 불명을 나중에 해소하면 같은 기록의 상태를 전이하며 중복으로 최종 결과를 더하지 않는다. 실행 중 비동기 스냅샷의 일시적 불일치와 정지/배출 뒤 확정 대조를 구분하고, 애플리케이션이 API에 제시하지 않은 생성 데이터는 로거 자체 집계로 검증했다고 주장하지 않는다.

### 4.4 디스크 공간 부족(ENOSPC) 및 I/O 장애의 Fail-Safe 격리

저장 매체 가득 참(`ENOSPC`, Disk Full)이나 파일 시스템 쓰기 실패는 로깅 시스템의 치명적 상황이지만, 메인 로봇 제어 시스템의 생존을 위협해서는 안 된다.

| 장애 상황 | 로거 백그라운드 동작 | RT 생산자/상위 제어 영향 |
|-----------|----------------------|--------------------------|
| 디스크 풀 (`ENOSPC`) | 영향받는 파일 대상에 `IoFailure(ENOSPC)` 상태 마킹. 백그라운드 재시도 중단 및 손상 방지 파일 닫기 | RT 호출에 패닉·블로킹 전파 금지. 수집 큐 포화 시 정책에 따라 새 입력 거절 또는 카운터 집계 |
| 파일 권한/잠금 충돌 | 대상별 실패 기록, 리포트에 장애 알림 발행 | 정상 출력은 계속 처리하되 공유 CPU/장치 경합의 영향은 별도 계측 |
| 순환 로깅 회전 실패 | 오래된 파일 삭제 실패 시 신규 파일 강제 생성 중단 (디스크 고갈 방지) | 자원 한계에 도달하면 신규 수락을 `Capacity(DiskFull)`로 명시적 거절 |

- 복구 가능한 I/O 오류는 `Result`와 대상별 상태로 처리하며 RT에 panic/대기를 전파하지 않는다. 내장형 구조는 panic=abort·OOM·메모리 손상의 프로세스 장애까지 격리하지 못한다(ADR-007).
- 장애가 감지되면 관측 리포트에 즉각 `StorageDegraded` 지표를 노출하고, 복구(디스크 공간 확보 등) 전까지는 지정된 안전 정책(신규 입력 거절 또는 메모리 내 손실 카운팅)을 따른다.

## 5. 운영 관측·기준 능력·개발자 조절

보고서에는 최소한 설정 revision·측정 구간·생산자/채널/대상 범위·입력/완료량·단위·미처리량·지연·손실/의도적 생략·측정 기준의 출처/유효성을 연결한다. 기준선 목표, 일상 관측, 지정 시험으로 검증된 능력, 모델 추정, 미측정/기준 만료를 구분한다.

버퍼 bytes/슬롯·작업/회수 예산과 실제 원본의 수명 연장을 함께 집계한다. 회전/삭제·압축·중복 출력 후 실제 파일 증가와 파일 시스템 여유를 관측하고 버퍼 포화·출력 정지 가정·디스크 여유·순환 보존 창의 시간을 각각 표시한다.

개발자의 주기/필터·샘플링·원본/필드·출력 변경을 지원하는 API와 설정 단위를 검토한다. 변경안의 이유·절감과 기록 정보 손실·적용 가능한 범위·필요한 재점검을 제공하고 명시적으로 적용한다. 변경 전후의 비교는 조건 차이와 설정 revision을 남기며, 출력만 줄인 효과와 앱에서 기록 생성 작업을 줄인 효과를 구분한다. 자동 설정 변경/자동 제한은 현재 기본 범위에 없다.

## 6. 열려 있는 설계 결정

| 관련 미결 | 정할 내용 | 확정 시점 |
|-----------|-----------|-----------|
| OQ-002 | 실제 dot-probe 장비·제어 주기/부하·소형/원본 타입·크기·단계별 수치 | 실제 장비 대응/검증 설계 및 해당 계약 리뷰 |
| OQ-003 | ADR-008/§13~17 계약 승인. 잔여: 장비별 quota·보호량/허용 손실 수치와 수명/동시성 증명 | TASK-003~009·021·022 및 실제 검증 |
| OQ-004 | ADR-007·009 배치/기술 채택. patch·MSRV 로컬 확인(EV-001). 잔여: native OS/CPU/DB 호환 | TASK-002·019·022 |
| OQ-005 | 설정·리포트·점검 및 계측 목표 승인. FR-015는 별도 제안 | 관련 구현/검증 Task, TASK-018은 범위 승인 후 |

[요구사항 §15.4](../02_project_definition/requirement_spec.md#154-미결을-설계에서-확정할-보완-계획)의 승인된 수집 계획을 따른다. 채택한 계약/기술은 ADR-008·009를 따른다. 미확정 실장비 수치를 임의로 채우지 않는다.

## 7. 관련 기준

- [도메인·모듈 초안](domain_model.md)
- [요구사항](../02_project_definition/requirement_spec.md)
- [승인 Story/AC](../02_project_definition/user_story_registry.md)
- [ADR-005](adrs/ADR-005_design_baseline_and_deferred_decisions.md)


## 8. OS 지원·기능 탐색 계약 (Tier 1 vs Tier 2)

Linux·macOS·Windows에서 공통 수집·기록·조회/재생·리포트/점검·개발자 조절 계약을 제공하되, 플랫폼 역할을 **Tier 1 (Target / Full SLA)**과 **Tier 2 (Compatibility / Best-Effort)**로 분리한다.

- **Tier 1: Linux (PREEMPT_RT / 일반 Linux)**: 실제 로봇 탑재 타겟. 승인된 조건부 실시간 호출 지연(p99 ≤5 µs), 대용량 처리(1.0 GB/s), 활성 구성의 전원 차단 긴급 동기화(FR-018)를 목표로 장비별 검증하며, 백그라운드 워커의 CPU 코어 격리(Pinning) 및 RT 우선순위 설정을 지원한다.
- **Tier 2: Windows, macOS**: 개발자 워크스테이션 및 분석기. 기능적 호환성(API 동작, 데이터 수집/파일 생성, 크로스 파일 판독, 모니터링 클라이언트)을 제공·네이티브 검증하며, 엄격한 실시간 SLA 및 성능 측정 대상에서 제외(Best-effort)한다.

플랫폼/권한에 따라 지원하지 않는 고급 스레드 설정(예: CPU 코어 격리)·계측 지표는 지원 여부와 사유로 표현하는 안이다. 미지원 요청을 조용히 성공 처리하거나 해당 지표를0·정상 여유로 변환하지 않는다. 가능한 공통 경로와 성능/보장 범위를 설명하고, 플랫폼 전용 API/핸들을 일반 호출자의 필수 입력으로 정하지 않는다.

완료 경계와 단위·시각 의미는 공통으로 문서화하고 OS별 파일/영속 동기화·열린 파일 회전/삭제·오류·공간 조회의 실행 동작을 확인한다. R1은 Linux PREEMPT_RT 참조 프로필이며 다른 OS의 측정값·성능 보장으로 재사용하지 않는다. 보고서의 장비/설정/기준 프로필 식별에는 OS와 실행 환경도 포함한다.

- **OS별 영속 동기화 프리미티브**: Linux는 `fdatasync`/`fsync`, macOS는 `fcntl(fd, F_FULLFSYNC)`, Windows는 `FlushFileBuffers`를 기본 매핑 후보로 하며, 각 플랫폼 시스템 콜의 지연 특성과 전원 장애 내구성 보장 차이를 문서화하고 벤치마크 프로필에 반영한다.
- **파일 락 및 공유 플래그 (특히 Windows)**: Windows 환경에서 로거가 기록 중인 파일에 모니터링 클라이언트나 점검 도구가 접근할 때, 또는 순환 로깅 중 이전 세그먼트를 회전/삭제할 때 배타적 락으로 인한 `Access Denied` 실패가 발생하지 않도록 파일 생성 시 `FILE_SHARE_READ | FILE_SHARE_DELETE` (Unix open 플래그와 동등한 의미는 아니며 OS별 검증 필요) 플래그를 기본 적용한다.
- **백그라운드 워커 CPU 코어 격리 (Affinity/Pinning)**: 로거 설정(`WorkerThreadingConfig`)을 통해 백그라운드 I/O 워커들이 동작할 CPU 코어 목록(예: Core 0, 1)을 지정할 수 있다. Tier 1 Linux에서는 `sched_setaffinity`를 통해 RT 제어 코어(Core 2, 3 등)에서 worker 실행을 분리하며 IRQ·공유 자원 간섭은 별도 검증하고, Tier 2 OS에서는 플랫폼 지원 수준에 따라 적용하거나 기본 스케줄러에 위임한다.

[ADR-006](adrs/ADR-006_cross_platform_linux_macos_windows.md), [US-005](../02_project_definition/user_stories/US-005_cross_platform_support.md).

## 9. 중요도·예산과 포화 동작 권고안

### 9.1 기본 동작과 대안

ADR-003의 중요 기록 우선 기본값을 구체화하는 **2026-10-26 승인된 상세 계약**이다. 옵션 지원 방향은 유지하며 RT 호출의 공간/저장 무대기는 기존 확정 계약이다.

| 쟁점 | 권고안 | 대안과 비용 |
|------|--------|-------------|
| 초기 중요도 | `Normal`·`Important` 두 등급, 로그 레벨과 별도 지정 | 다단계는 표현력이 높지만 예약·공정성 조합이 늘어 후속 필요 확인 |
| 중요도 생략 | 명시값 → 채널에 설정한 분류/기본값 → Normal. 레벨 기반 채널은 등록된 매핑으로 분류 | 생략 자체를 오류로 만들면 명시성은 높지만 일반 기록의 사용 비용 증가 |
| 수집 포화 | 중요 기록용 slots와 bytes를 보호하고, 허용 공간이 없으면 새 입력 거절 | 수락된 저중요도 기록 축출은 중요 입력을 더 받을 수 있지만 분배/참조 회수와 완료 이력 처리가 복잡 |
| 같은 중요도 포화 | 먼저 수락한 기록을 유지하고 새 입력 거절 | 오래된 기록 폐기는 최신 관측에 유리할 수 있으나 수락 후 폐기와 순서 공백 처리가 추가됨 |
| 중요 기록까지 포화 | 무대기 거절과 원인 집계. 무손실 보장 없음 | 일반 호출의 제한 시간 대기는 별도 옵션. RT용으로 자동 전환하지 않음 |
| 느린 선택 출력 | 그 출력의 제한 예산 안에서만 유지하고 한계 도달 시 대상별 정책 폐기 보고 | 모든 출력 완료까지 무제한 원본 유지하면 공유 메모리가 고갈됨 |

레벨 기반 매핑을 선택했는데 매핑/채널 기본값이 불완전하면 등록 오류로 처리하는 안이다. Important 지정은 우선 수락 기회를 뜻하며 모든 출력 성공·영구 보존·일정 지연 보장은 아니다. 같은 등급의 원본 대용량 기록이 소형 중요 이벤트용 예약분을 소진하지 않도록 생산자/채널 또는 payload 종류별 예산 분할도 검토한다. 실제 예약 비율·최대 크기·스케줄링 공정성 수치는 장비/부하 확인 전 채우지 않는다.

### 9.2 예산 차감과 출력 장애

관리 예산은 수집 봉투 slots/bytes, 풀 할당·대여/로거 보유 원본 bytes, 출력별 참조/복사·작업 버퍼, 완료 추적/대기자, 회수 대기를 포함한다. 같은 원본 할당은 전역 물리 점유로 한 번 세고 출력별 보유 원인/참조를 별도로 보고한다. 출력이 별도 복사하면 그 바이트를 추가한다. 풀 사전 할당량과 사용 중/재사용 가능량을 구분하고 애플리케이션 외부 할당까지 전체 프로세스 상한에 포함됐다고 주장하지 않는다.

예약 보호는 수집 봉투뿐 아니라 실제 병목 자원에도 적용돼야 한다. 풀의 대여 단계부터 채널/중요도에 맞는 예약분을 보호하고, 대여할 때의 자원 분류와 제시할 때의 분류가 일치하는지 확인하는 안이다. 일반 입력이 중요 풀을 먼저 빌려 보호를 우회할 수 없어야 한다. 자원별 허용 검사를 모두 통과해야 수락하며 슬롯만 남거나 bytes만 남으면 거절한다. 모든 자원에서 Important 예약분을 일반 입력이 잠식하지 않도록 구성 검증한다. 숫자/예약 구현·부족 자원별 판정 순서는 후속 사항이다.

출력 정지 시 필수 대상도 무제한 재시도하지 않는다. 대상별 시도/시간/미처리 bytes 상한과 종료 결과를 사전 구성하고, 소진하면 실패/정책 폐기로 정리하는 안이다. 선택 출력은 해당 출력의 참조를 포기할 수 있지만 이미 진행 중인 IO 접근은 안전하게 끝나거나 취소 확인을 받아야 한다. 다른 출력도 공유 CPU/메모리/SSD 영향을 받을 수 있으며 완전한 성능 격리를 보장하지 않는다.

필수 파일에 실패한 수락 기록을 일반 메모리에서 끝없이 보관하거나 별도 비상 디스크로 자동 우회하지 않는다. 재전송·영속 스풀 같은 추가 수단은 필요성과 예산을 별도 결정한다. 수락 후 실패는 입력의 동기 거절로 소급하지 않는다.

## 10. API 사용 흐름과 종료 검토안

다음은 의미를 검토하기 위한 표기이며 컴파일 가능한 Rust 선언이나 최종 함수명이 아니다. 실제 타입/수명·오류 열거형·고정 크기는 계약 합의와 알고리즘 검증 뒤 정한다.

```text
설정: init_logger(session_config, worker_threading_config) -> Session 또는 오류
      - worker_threading_config: { cpu_affinity: Option<Vec<usize>>, priority: Option<Priority> }
관리: register_producer(channel_plan, limits) -> Producer 또는 구성 오류
준비: try_acquire(pool, channel_class) -> Loan 또는 대여 불가
RT:   try_record_small(producer, metadata, bounded_bytes)
        -> Accepted(receipt) | Skipped(reason) | Rejected(reason)
RT:   try_record_raw(producer, metadata, loan)
        -> Accepted(receipt) | Skipped(reason, loan) | Rejected(reason, loan)
관리: capture_scope(producer_set) -> 생산자별 수락 상한이 고정된 범위
일반: poll_completion(receipt_or_scope, targets_and_boundaries) -> 대상별 상태
일반: wait_completion(receipt_or_scope, targets_and_boundaries, deadline)
        -> Complete | Incomplete | TimedOut(snapshot) | QueryError
관리: shutdown(drain_plan, deadline) -> 완료·실패·미처리·회수 잔여 보고
```

수락 receipt는 원본을 소유하지 않는다. 거절된 loan은 재제시하거나 반납할 수 있다. 재제시는 같은 시도 안의 무한 재시도가 아니며 제어 경로 밖의 명시적 정책으로 다룬다. 소형 입력은 호출 뒤 계속 빌리지 않으므로 반환할 loan이 없다. 일반 호출의 제한 시간 수락 대기는 별도 API/명시 옵션으로 설계하고 RT 핸들에 대기 동작을 숨기지 않는다.

운영 예: 카메라 원본을 제시해 Accepted를 받으면 해당 메모리를 다음 프레임에 재사용할 수 없다. 로거가 사용하는 동안은 풀에서 다른 핸들을 얻는다. 풀 고갈로 대여에 실패하거나 제시가 Rejected면 입력 누락/거절을 구분해 집계한다. 파일 쓰기 완료와 원본 풀 복귀는 서로 다른 상태다. 원본 참조는 동기화보다 먼저 해제될 수도, 다른 출력 때문에 더 오래 유지될 수도 있다.

종료는 `Running → Closing → Closed`의 관리 상태로 검토한다. Closing 전환과 경합한 제시는 수락 경계에 따라 완전 수락하거나 원본을 반환하며 거절한다. Closed 성공은 선언된 배출/동기화 계획과 안전한 정리가 끝났을 때만 반환한다. 제한 시간에 도달하면 미처리 기록·대상·회수/IO 잔여와 계속 살아 있는 관리 책임을 보고한다. 진행 중 IO가 있는 메모리를 무효화하거나 백그라운드 객체를 조용히 버리지 않는다. 프로세스 중단 후 복구 가능 범위와 허용 손실은 형식/동기화 설계에서 별도 확정한다.

## 11. 리뷰 시나리오·승인 범위·잔여 사항

### 11.1 계약 대조 시나리오

아래는 **설계 리뷰 항목**이며 실행된 테스트 기록은 아니다. 세부 AC의 현재 승인 근거는 ADR-008이다. 코드가 없는 현재는 모두 실행 미검증이다.

| 소비자/조건 | 확인할 동작 | 후속 실행 검증 |
|-------------|-------------|----------------|
| 소형/원본 생산자 정상·생략·입력 오류 | 소형 입력을 호출 뒤 빌리지 않음, 원본은 수락만 소비하고 그 외 동일 핸들 반환 | 내용/수명·카운터 대조 |
| slots 여유·bytes 부족 및 반대 | 자원별 명시 거절, 부분 예약/참조가 남지 않음 | 경계값·경합·메모리 상한 |
| 일반 입력 포화 뒤 Important 입력 | 보호한 예약 자원을 사용할 수 있음. 예약분 소진 후 Important도 거절 | 혼합/편중 부하·제어 주기 영향 |
| 풀 반납·회수 경로 포화 | 거절된 핸들의 Drop/반납도 RT에서 동기 해제/콜백·누수로 우회하지 않음 | 최종 참조·관리 객체 해제·회수 중단 |
| 파일 정상·DB 실패·원격 정지 | 대상별 부분 완료와 참조 수명 유지, 무제한 재시도 없음 | 장애 주입·팬아웃·실제 소비 확인 |
| sequence 1 진행 중·2 완료 | 최고 완료 2를 연속 완료로 보고하지 않음 | 범위/결측·실패/폐기 대조 |
| 쓰기 완료 뒤 동기화 실패·회전 | 쓰기 성과를 보존하고 동기화/복구 범위는 별도 보고 | OS별 파일/이름/인덱스 복구 |
| 대기 취소·시간 초과·이력 만료 | 수락 데이터는 계속 추적, 조회 한계/만료를 성공으로 바꾸지 않음 | 제한된 이력·대기자·메모리 |
| 설정 교체·종료와 동시 제시 | 한 revision/계획에 속한 완전 수락 또는 원본 반환 거절 | 경쟁 조건·기존 설정 회수·잔여 IO |
| 보고서·조회/재생 | 제시/수락/거절·대상별 결과와 결측 구분, 읽을 수 있는 범위만 노출 | 집계 정합성·실제 기록 조회·부하 영향 |
| UPS 예고·중요 파일 긴급 기록 | 신호 발생부터 전체 약 3초 안에서 우선순위/최대 보호량·전달 지연·진행 중 IO를 함께 판정 | 실제 전원 차단·재기동 복구, §12 검토안 |

Linux 일반/PREEMPT_RT·macOS·Windows에서 해당 경로를 네이티브 실행하고 OS/장비·설정 revision·부하와 함께 검증한다. 현재 공통 의미만 설계했으며 어느 OS의 지연/영속성 달성도 확인하지 않았다.

### 11.2 이번에 검토할 계약 묶음

Human 검토 대상은 (1) 제한된 원본 풀/소유권 반환·RT 밖 회수, (2) 두 중요도와 예약 보호·신규 거절 기본 동작, (3) 생산자별 수락 식별·revision/출력 계획 고정과 대상별 부분 완료·제한된 조회 이력이다. §9.1의 대안 비용과 함께 검토한다. 기존 ADR-003의 옵션·기본값과 ADR-004의 저장 예산 분리는 유지한다.

2026-10-26 ADR-008·009와 §13~17을 기준선으로 승인받았다. 장비별 quota·보호량·허용 손실 수치와 동시성/회수 알고리즘의 실제 안전성·성능은 구현/검증 Task에서 확인한다. 설계 승인과 제품 실행 증거를 구분한다.

## 12. 전원 이상 시 중요 파일 우선 기록

### 12.1 요구와 예산

**FR-018 기능 방향은 Human 요청이며, 약 3초는 UPS 신호 발생부터 실제 전원 차단까지 전체 시간이다.** 로거 수신 후 3초를 추가로 확보한 것으로 계산하지 않는다. 시간·요구의 기준 본문은 [요구사항 §17](../02_project_definition/requirement_spec.md#17-전원-이상-시-중요-파일-우선-긴급-기록)이다. 아래 API와 동작은 상세 검토안으로서 이전 §11.2 계약 묶음과 함께 검토할 대상이다.

**Human 확정: 기능은 선택 옵션이며 기본값은 비활성이다.** 활성 상태에만 아래 긴급 계획/예산을 적용한다. 비활성 시 긴급 전환·전용 자원 예약을 하지 않으며 일반 기록/종료 계약을 유지한다. 통지 API에는 비활성/미설정 결과를 표현해 인수 성공으로 오인하지 않도록 검토한다. 활성화 구성의 유효성 검증·설정 revision은 관리 경로에서 처리하는 안이며 실행 중 변경과 빌드 기능 분리는 미정이다.

전달/감지 지연과 종료/전원 차단 안전 여유를 뺀 남은 시간 안에 중요 범위의 쓰기·필수 메타데이터·영속 동기화를 완료하도록 설계한다. 보호 범위는 파일/우선순위와 최대 미동기화량으로 제한하며 로거 전체 메모리 배출과 구분한다. 중요 파일의 기본 목록·순위·같은 순위 안의 순서와 정확한 시간 여유는 미정이다.

### 12.2 입력·실행·완료 계약 검토안

| 경계 | 검토안 |
|------|--------|
| 사전 등록 | 중요 파일 ID/순위·보호 범위·최대 미동기화 bytes·필수 복구 메타데이터/완료 경계·신호 전달 지연 상한·안전 여유를 관리 경로에서 검증 |
| 신호 입력 | 장치/OS 어댑터가 원인·신호 발생/수신 시각·검증한 차단 기한 또는 남은 예산을 전달. 하드웨어 시각과 로거 단조 시각의 대응/오차도 명시 |
| 빠른 통지 | 사전 준비한 제어 요청을 게시하고 인수 여부 반환. 인수 성공은 긴급 파일 저장 완료가 아님. 대기/할당/파일 IO를 비동기 OS 신호 핸들러나 RT 호출 안에서 하지 않음 |
| 제어 자원 | 데이터 큐 포화 때도 긴급 통지/계획을 표현할 수 있게 별도 제한된 제어 자원 검토. 기존 관리 대기열 뒤에 무기한 지연되지 않도록 설계 |
| 긴급 진입 | 관리 경로가 기존 Closing/종료 처리와 조정해 고정된 긴급 기록 범위/계획을 만듦. 일반 신규 입력 제한과 마지막 중요 상태 수락 범위는 사전 설정한 별도 예산으로 검토 |
| 우선 실행 | 파일 긴급 순위에 따라 중요 범위를 먼저 처리. 파일 내 앞선 바이트/인덱스 등 복구 의존성도 포함. 기록의 Normal/Important 분류와 별도의 파일 순위를 사용 |
| 비필수 작업 | 조회·압축·원격/DB 등 비필수 작업은 긴급 정책에 따라 유예/축소하되 진행 중 IO와 참조를 안전하게 유지. 중요 로컬 파일 성공이 외부 수신자 응답을 기다리지 않도록 검토 |
| 신호 중복/종료 경합 | 동일 사건을 중복 종료로 실행하지 않고 더 이른 유효 기한을 반영. 중복 수신마다 예산을 3초로 다시 시작하지 않음. 전원 복구 신호만으로 정상 재개할지는 별도 결정 |
| 결과 | 파일/범위별 쓰기·영속 동기화 완료와 실패/미완료, 기한·소요시간·사용한 계획 revision 보고. 전원 차단으로 결과 보고가 없으면 완료 불명으로 다루고 재기동 때 확인 |

#### 긴급 3초 단계별 부하 차단(Load Shedding/Fast Reject) 프로토콜

UPS 신호 감지부터 실제 전원 차단까지의 전체 예산(약 3초) 내에서 핵심 파일의 온전한 보존을 보장하기 위해, 긴급 통지 접수 즉시 3단계 부하 차단을 적용한다.

```text
[UPS 신호 수신]
  │
  ├─ 1단계 (Fast Reject): 신규 일반/대용량 로그 입력 즉시 차단 (Rejected: EmergencyPowerOff)
  │                      (사전 지정된 필수 상태/오류 스냅샷 슬롯만 엄격한 상한 내 수락)
  │
  ├─ 2단계 (Shedding):    원격 네트워크 전송, 시계열 DB 연동, 비중요/선택 파일 Flush 즉시 포기
  │                      진행 중인 저우선 I/O 취소 요청 및 리소스 반환
  │
  └─ 3단계 (Sync Focus):  사전 등록된 순위에 따라 1순위 중요 파일의 쓰기 버퍼 즉시 플러시
                         OS 영속 동기화(fsync/FlushFileBuffers) 집중 실행 후 안전 종료 마진 확보
```

개념 API는 `notify_power_fail(pre_registered_plan, deadline_evidence) -> request_status`와 관리 경로의 상태 조회로 검토한다. RT 생산자가 종료/동기화 완료를 기다리지 않는 기존 계약은 유지한다. 정확한 타입·스레드·큐/장치 어댑터는 미선정이다.

일반 파일의 큰 쓰기/동기화가 이미 진행 중일 수 있으므로 우선순위 변경만으로 시간을 보장하지 않는다. 긴급 경로의 최악 지연을 고려해 평소 처리 단위·중요 파일의 미동기화량을 제한하고, 필요하면 주기적 영속 동기화·별도 자원 배치를 비교한다. 이는 아직 주기/배치 채택 결정이 아니다. 유효한 시간 예산이 없거나 보호 범위가 예산을 넘으면 지원/완료를 성공으로 표시하지 않는다. 기한 초과만으로 진행 중 IO가 안전하게 취소된 것으로 간주하지 않는다.

실제 UPS 어댑터 선택을 위한 참고로 [NUT 구성 문서](https://networkupstools.org/docs/user-manual.chunked/ar01s06.html)는 UPS 통신/안전 종료 구성을 설명한다. 현재 NUT를 채택하거나 이 문서만으로 3초 신호 전달/저장 성능을 입증한 것은 아니다. 파일 쓰기와 영속 동기화의 구분은 §4.2의 Rust 공식 문서 근거를 따른다.

### 12.3 검증·잔여 결정

US-002 AC-6~10의 승인된 추가 수락 기준을 따른다. 기본 비활성의 일반 경로를 확인하고, 활성 구성은 여러 순위 파일·최대 보호량·큰 일반 IO 진행 중·신호 지연/중복·종료 경합·디스크 장애·CPU/IO 편중에서 신호 발생→전원 차단까지 계측해 재기동 후 파일/내용/복구 범위를 확인한다. OS/UPS/SSD·파일 시스템별 실행과 실제 차단 검증은 아직 없다. 우선순위/새 입력 제한·복구 경계는 OQ-003, 장비/보호량은 OQ-002, 어댑터와 계측/수락 조건은 OQ-004·005로 보완한다. 기능 추가는 이전 세 가지 핵심 API 권고안의 자동 승인이 아니다.

## 13. 상세 API 설계안

2026-10-26 추가 및 Human 기준선 채택 완료. §1~12의 초기 논의를 아래 계약으로 구체화했다. ADR-008·009에 따라 새 기본값·오류·소유권·통신을 채택한다. 컴파일/실제 수명 검증은 TASK-003·004에서 한다.

| API 후보 | 입력/결과 | 실행 경계 |
|---|---|---|
| `SessionBuilder::validate/build` | limits·target·schema·worker·durability 계획 → owner 또는 구조화된 설정 오류 | 일반 경로, 모든 quota 합계·최대 단일 원본 검증·예열 |
| `owner.scope(callback)` | scope 안에서 producer/loan을 빌려 사용, owner보다 오래 유지 불가 | RT 스레드 생성 전에 등록. 최종 해제는 scope 종료 후 관리 스레드 |
| `Producer::try_record_small` | 고정 metadata + `&[u8]` → receipt/생략/거절 | `&mut self`, 하나의 생산자·고정 상한 복사 |
| `Producer::try_acquire` | 크기 등급·importance → `Loan<'session>` 또는 Capacity | 유한 풀에서 하나 대여, 외부 SDK 버퍼 아님 |
| `Producer::try_record_raw` | metadata + loan → receipt 또는 이유와 같은 loan | 성공 때만 이동. pool/producer/class 불일치 거절 |
| `Loan::drop` | 슬롯 상태를 ReturnPending으로 게시 | payload free·callback·blocking 없이 반환 의무 보존 |
| `GeneralProducer::record_until` | 입력 + 단조 deadline → receipt 또는 Timeout/Closing | 일반 핸들만 사용, 매 시도 기한 확인. RT API에 대기 옵션 없음 |
| `completion.poll/wait` | receipt 또는 생산자별 고정 상한 + target/boundary | 일반 경로. `Expired`, `UnsupportedBoundary`, `TimedOut(snapshot)` 분리 |
| `control.sync` | 대상/생산자별 scope + deadline | 각 writer의 barrier까지 들어온 기록을 마감·sync. 단순 poll은 sync를 시작하지 않음 |
| `control.shutdown` | drain·sync 계획 + deadline → Closed 또는 Incomplete(report, owner) | timeout이면 owner/IO 메모리 유지. 후속 poll/join으로 정리 |

`SessionOwner`의 무거운 Drop을 RT에서 안전하다고 선언하지 않는다. borrowed producer/loan만 RT-safe Drop 대상이다. scoped API 제약 때문에 dot-probe의 장기 thread 통합이 곤란하면 TASK-004에서 owner 기반 대안을 재검토하고, 미검증 `Arc`로 수명을 우회하지 않는다.

고정 오류 코드는 `InvalidInput`, `SchemaMismatch`, `TooLarge`, `Capacity(resource)`, `RevisionBusy`, `Closing`, `EmergencyPowerOff`, `Unsupported`, `IoFailure`, `DeadlineExceeded`를 포함한다. 동적 진단은 일반 경로의 별도 리포트다. no-op tracing 어댑터도 RT 안전하다고 자동 선언하지 않는다.

TASK-002 구현 범위: `crates/dot-logger/src/config.rs`의 `SessionBuilder::new(config).validate(&PlatformCapabilities)`가 총 메모리·생산자별 소형/원본 고정 slot·중요 예약·worker stack·CPU 목록·required capability를 검증하고 읽기 전용 `ValidatedConfig`를 반환한다. quota/slot 크기는 명시 설정이며 R1 수치를 기본값으로 쓰지 않는다. 지원 미확인·실패·미지원은 구분하고 optional 상태도 결과에 보존한다. 이 API는 worker/IO를 시작하지 않는다. `build`·예열·실제 플랫폼 탐색/적용·target/schema/durability와 실제 metadata 할당 예산 검사는 해당 자원을 구현하는 후속 Task에서 확장한다. 위 표의 owner/수집/완료/종료 API는 설계 계약이며 아직 구현하지 않았다. 로컬 검증은 EV-001, native CI는 GAP-004다.

TASK-003 구현 범위(2026-10-27): `capture::CaptureOwner::new(&ValidatedConfig, session_id, revision, channels)`와 `scope`는 소형 ingress만 구성한다. owner가 owning endpoint를 보유하고 `Producer::try_record_small`/`GeneralProducer::record_until`은 빌린 핸들이다. `CaptureConsumer::try_read`는 일반 경로의 대역 소비 API다. payload·봉투·endpoint/rtrb descriptor를 input, owner·고정 채널 metadata를 control 예산에 검사한 뒤 사전 할당·예열한다. 중요 예약은 생산자별 미소비 Normal 슬롯 수를 제한하고, 각 record의 최대 크기로 Important bytes도 보호한다. `ChannelConfig`의 전체 6레벨 mapping은 불완전한 매핑을 타입으로 배제한다. channel/schema/크기 오류는 필터보다 먼저 반환한다. 수락 sequence는 owner의 scope 재진입에도 유지하고 소진 시 거절한다.

`CaptureReceipt`는 소형 ingress 식별자이며 전체 출력 계획/완료 추적 예산을 인수하는 최종 세션 API의 완료를 뜻하지 않는다. 현재 `Capacity`는 소형/원본 ingress에 한정된 오류이며 최종 완료 추적 예산 오류는 후속 Task다. 초기 revision은 고정이며 설정 변경은 후속 Task다. ingress `control.close`는 신규 입력을 닫고 수락 기록을 읽을 수 있게 남긴다. 이는 drain/sync/worker join을 수행하는 최종 shutdown이 아니다. 제품 코드 unsafe 금지·무할당/무해제 로컬 검증은 [EV-002](../05_verification/verification_evidence.md#ev-002--task-003-소형-수집예약일반-대기)에 기록했다. Human 리뷰와 변경분 3 OS × 2 Rust native CI는 2026-10-27 완료됐다. 실제 RT/장비 판정은 남아 있다.

TASK-004 구현 범위(2026-10-27, Human 구현 리뷰 승인): `CaptureOwner`가 선택 원본 풀까지 할당한다. `Producer::try_acquire(length, importance)`는 producer별 고정 크기 등급에서 유효 길이를 고정한 `pool::Loan`을 반환한다. 대여 단계부터 중요 예약 bytes/slots를 보호한다. `try_record_raw`는 pool/producer/중요도 불일치도 필터보다 먼저 검사하며, 거절은 `RawRecordError { reason, loan }`, 생략은 `RawRecordOutcome::Skipped(loan)`, 수락은 `Accepted(CaptureReceipt)`다. 수락은 payload 복사 없이 같은 producer FIFO에 들어간다. 소형/원본 수락 sequence는 하나이며 ingress 인수만 뜻한다.

`CaptureConsumer::try_take_raw`는 불변 `RawRecord`를 반환한다. 선두가 소형이면 `PayloadKindMismatch`로 남겨 FIFO를 유지한다. 일반 복사가 필요한 소비자는 `try_read`로 두 payload 종류를 읽을 수 있다. 짧은 버퍼는 원본 상태/큐를 바꾸지 않는다. `RawRecord`의 마지막 borrow/guard 종료 뒤 ReturnPending으로 게시하며, `reclaim_raw(limit)`가 유한 스캔으로 Free로 바꾼다. 알림은 1슬롯 coalescing mailbox이며 권위 있는 반환 상태는 슬롯에 있다. 원본 generation은 61 bits·소진 시 Retired/GenerationExhausted이며 식별자를 재사용하지 않는다. 풀 미설정은 Unsupported다. 예산/수명/Miri/allocator 검증은 [EV-003](../05_verification/verification_evidence.md#ev-003--task-004-원본-풀수명반환)이며 제품 unsafe 예외는 private pool/storage 한 모듈로 제한한다. 완료 window·출력 성공·fanout·실제 shutdown·장비 성능 판정은 후속 범위다.

TASK-005 구현 범위(2026-10-28, Human 리뷰 승인): `routing::Router::new(plan, ordered_consumers, RoutingConfig)`는 같은 capture 세션의 모든 producer·실제 small/raw slots/slot bytes·초기 revision·채널 route·대상 ID와 concrete output 예약을 검증한다. `Router::scope`에서 유일 `Dispatcher`와 대상별 `TargetConsumer`를 빌린다. `Dispatcher::turn(now, events)`은 비감소 단조시간·유한 retry/회수/공정 순회를 사용하고, `TurnReport.events` 길이의 event prefix와 `fault`를 보고한다. 실패 head는 ingress에 남고, 이전에 처리한 기록의 event도 report에서 보존한다. `Enqueued`는 출력 FIFO 진입이며 쓰기/완료/영속 보장이 아니다. 필수 대상 drop을 전체 성공으로 바꾸지 않는다. 빈 route는 명시 `Unrouted`다.

`TargetConsumer::try_next(now)`는 불변 `Delivery` 또는 해당 queued 항목의 `Cancelled`/`Expired` event를 한 건 반환한다. 출력이 끝날 때까지 guard를 유지해야 하며 사용자 복제 API는 없다. raw fanout은 같은 원본 pointer를 공유하고, 소형 fanout은 별도의 일반 경로 복사 한 건을 공유한다. pending/queued/진행 IO의 대상별 보유 slots/bytes와 distinct 공유 header/copy bytes를 분리 집계한다. 이벤트 버퍼/공유 예산 포화는 신규 분배를 유예하고, 대상 포화·시도/기한 소진은 `DispatchEvent`로 명시한다. 취소 후 reader를 구동하지 않으면 유한 큐 보유가 남는다. `cancel_target`은 실제 진행 IO를 무효화하지 않고, `close`/owner Drop은 성공한 shutdown을 뜻하지 않는다. 초기 route만 구현하며 설정 변경·완료 추적/수락 예산 통합은 후속 Task다. [EV-004](../05_verification/verification_evidence.md#ev-004--task-005-분배출력-격리예산)에서 실행 범위를 확인한다.

### 13.1 완료 추적의 실제 상한

생산자별 `accepted_seq mod window_size` 슬롯에 sequence generation과 대상별 bitset/상태를 저장한다. 모든 필수 추적 예산을 먼저 확보한 뒤 수락한다. 미완료 슬롯을 재사용하지 않는다. window가 가득 차면 `Capacity(Completion)`으로 거절한다. 완료 뒤 상세 이력의 보존 한도에 따라 축출하고 그 이전 receipt는 `Expired`다. 최고 sequence만으로 완료 공백을 지우지 않는다.

2026-10-28 RV-002 수정: producer별 완료 보호량 R은 소형/원본 `important_slots` 중 큰 값이다. window W가 R보다 작으면 시작 전에 거절하고, W=R이면 Normal을 수락하지 않는다. Normal은 자기 slot 외에 다음 R개 cyclic slot이 Empty/Terminal일 때만 수락해 Important의 다음 수락 기회를 남긴다. 단순 빈 슬롯 수만 세면 중간의 pending slot이 다음 sequence를 막으므로 연속 재사용 가능성을 검사한다. 이미 Important가 점유했거나 일반 보고가 Updating 중이면 거절될 수 있다. RT는 최대 R개 상태 조회와 기존 단발 예약 CAS를 수행하며 추가 할당·시각 확인·재시도 loop는 없다. 예약이 0인 TreatEqually는 기존 동작이다.

범위 결과는 bounded page로 반환한다. 각 페이지의 complete/failed/dropped/pending/unknown/expired 구간을 구분하며 상세 구간 개수가 응답 상한을 넘으면 cursor를 제공한다. 전체 성공 판정에는 모든 요청 범위의 결과가 필요하다. 수락 토큰은 원본 메모리를 소유하지 않는다.

### 13.2 TASK-006 실제 API와 완료/종료 경계 — 2026-10-28

Human 리뷰 승인. `CaptureOwner::new_tracked(plan, session_id, revision, channels, routing, CompletionConfig)`는 초기 route와 producer별 완료 window·control bytes를 검증한다. 같은 source memory 분할을 Router/ControlOwner 생성 때 대조하고 pool layout mismatch도 거절한다. `CaptureScope::completion`/`control`로 borrowed 관리 핸들을 얻는다. `try_record_small/raw`는 수락 전에 completion slot을 단발 예약하고 부족/게시 중이면 `CompletionCapacity`로 거절한다. raw 거절은 같은 loan이며 일반 `record_until`은 이 포화도 deadline 안에서 재시도한다. 실제 RT 지연은 미검증이다.

`Completion::poll(receipt,target,boundary)`는 `RecordStatus { confirmed, outcome }`를 반환한다. 지원되지 않는 경계·잘못된 receipt/session/target은 오류다. Outcome은 Complete/Pending/Failed/Dropped/Unknown/Expired/NotRouted/Busy다. Synced 확인은 Written/Enqueued를 포함하고, 후속 실패는 이미 확인한 경계를 지우지 않는다. `Delivery::report(update)`는 guard를 유지한 채 구체 결과를 보고하며 Busy면 같은 guard로 재시도한다. 보고 없는 guard Drop은 Unknown으로 관측한다. 원본/소형 payload 접근 종료와 최종 완료 경계는 동일하지 않다.

RV-001 수정: FIFO 공개 직후 writer가 실패를 보고하고 Enqueued feedback이 나중에 도착할 수 있다. 이 선행 enqueue 사실만 terminal 결과와 병합하며 실패/폐기/불명을 성공으로 바꾸지 않는다. 뒤늦은 Written/Synced가 terminal 실패를 덮는 전이는 계속 거절한다.

`Completion::capture_scope(caller_ranges)`는 producer별 상한을 고정하고 `scope(first/last ranges)`는 명시 부분 범위를 검증한다. `query(scope,requests)`는 동시 query permit을 얻고 `page(caller_items)`가 설정한 page_items 이내 prefix와 누적 summary를 반환한다. 전 page를 읽기 전 is_complete는 false다. `wait(deadline,cancel,buffer)`는 페이지 사이마다 기한/취소를 확인하고 마지막 유한 page·summary를 반환하며 기록 처리나 sync를 바꾸지 않는다. clock 기반 최대 history_retention 또는 window 재사용으로 상세 결과가 Expired가 되며 terminal 집계는 남는다. `poll_at`의 caller 시간은 같은 단조 도메인의 시간 순서로 제공한다.

`ControlOwner::new(plan,capture_control,ControlConfig,origin)`은 별도 controller metadata를 남은 control 예산에서 예약한다. 명시 `request_sync(frozen_scope,targets,now,deadline)`는 명령 인수일 뿐이다. `next_sync(token,target,now,dispatcher,caller_cutoffs)`는 구체 입력/대상 IO/feedback의 배출을 확인한 뒤 유일 `SyncWork`를 준다. writer가 실제 동기화한 후 complete(Synced/Failed/Unknown)하고 `poll(now)`가 유한 receipt metadata에 결과를 적용한다. 이후 수락 기록을 과거 barrier로 Synced 처리하지 않으며 이미 만료된 detail을 성공으로 복원하지 않는다. barrier 결과와 범위 내 과거 실패/이력 만료는 별도로 조회한다. 진행 IO는 deadline 후에도 유지하며 live/queued/applying 상태에서 request slot 재사용을 거절한다.

`begin_shutdown(ShutdownPlan{sync_files},now,deadline,dispatcher)`는 신규 입력을 close하고, pre-close admission/loan publication이 끝난 뒤 cutoffs를 동결한다. Synced가 설정된 계약에서 sync_files=false는 close 전에 거절한다. `shutdown_ranges`와 전용 `shutdown_sync`로 동결 범위와 실제 sync 작업을 구동한다. `poll_shutdown`은 Closing/Incomplete(report)/Closed(report)이며 owner를 소비하지 않는다. 실제 IO guard/loan·queued/retry·feedback·회수 잔여 또는 pre-close call이 있으면 Closed 성공을 반환하지 않는다. deadline 이후에도 같은 owner로 cleanup하고 결과의 deadline 준수 여부를 보존한다. Closed의 is_success는 필수 대상 실패/불명과 sync 실패를 검사하며 optional 실패는 전체 totals에 남는다. 앱 worker/thread join은 애플리케이션 scoped host가 수행한다. 실제 파일/OS primitive·장비 durability·처리량 보장은 [EV-005](../05_verification/verification_evidence.md#ev-005--task-006-완료-추적명시-sync종료)의 대역 결과로 승격하지 않는다.

RV-003 수정: shutdown sync가 시작 전에 만료되면 실제 입력/출력/IO/loan/feedback 및 선행 일반 sync application을 먼저 배출한다. 그 뒤 전용 shutdown slot의 유한 application cursor로 Written 이후 미확인 Synced를 Unknown으로 종결한다. 명령 snapshot의 Expired·deadline 미준수·Written 확인은 보존하고 Closed도 실패 보고다. 일반 sync 요청만의 만료는 이 처리를 실행하지 않으며 이후 새 요청으로 동기화할 수 있다. 진행 중 guard를 무효화하거나 뒤늦은 실제 성공을 Unknown으로 선점하지 않는다. 수정 근거는 [EV-007](../05_verification/verification_evidence.md#ev-007)이다.

RV-005 수정 — 2026-10-29: 일반 `request_sync`의 진입부터 슬롯 게시/오류 반환까지 단발 원자적 진입 표시로 추적한다. 동시에 게시 중인 일반 요청이 있으면 기존 `Busy`로 무대기 반환하며, close 이후 요청은 `Closing`으로 거절한다. 진입 표시·열림 확인·shutdown close·정지 확인을 SeqCst로 순서화해, close 전 열림을 확인한 요청이 게시/반환되기 전에는 shutdown 범위 동결과 최종 정리를 확정하지 않는다. 이미 진입한 요청의 게시 이후에는 기존 명령/IO/metadata 수명과 deadline 계약을 따른다. 요청이 검증 오류/포화로 끝나도 RAII가 표시를 해제한다. `Closing`/`Incomplete`에서 dispatcher/readers/writers와 같은 owner를 계속 구동하고 `Closed`가 반환되면 보고의 `is_success`를 별도로 확인한다. 새 오류 enum·RT 수집 연산은 추가하지 않는다.

### 13.3 TASK-007 실제 파일 API — 2026-10-29

`Router::writer_budget(target)`은 source session/revision·target 및 같은 Router의 실제 잔여 output 예산을 묶은 opaque `WriterBudget`을 반환한다. 같은 target의 활성 `SegmentWriter`는 하나이고 여러 대상/finished index의 예약을 공유 합계로 검사한다. startup 경합은 Busy, 잔여 부족은 Budget이다. `WriterConfig`의 max payload·record·chunk·metadata 상한과 명시 `write_buffer_bytes` 및 보수적 reservation을 IO 전에 검증한다. 쓰기 버퍼는 logger 공유 예산에 포함하며 0이면 직접 쓴다. seek/flush 전에 버퍼를 배출하고 position 조회는 논리 위치를 반환한다. Drop은 버퍼를 재시도하거나 배출하지 않는다. 호출자가 제공하는 empty seekable `SyncSink`만 사용하며 path 생성이나 worker 시작은 하지 않는다. 표준 File 구현은 명시 sync_all을 호출한다.

`append(record,payload)`는 frozen session/revision/channel/schema·producer별 sequence·길이/시각을 검증하고 봉투를 기록한다. 인코딩과 file IO는 일반 경로다. 레코드/chunk 한계는 NeedRotation으로 거절하고 자동 삭제/회전은 수행하지 않는다. `flush_chunk` 성공 뒤에만 safe offset과 보조 인덱스를 공개한다. IO 실패 뒤에는 Poisoned이며 이전 safe offset은 유지한다. `finish`는 footer/summary까지 쓰고 `FinishedSegment`에 index/예산 소유권을 옮기며, finish/Drop을 Synced로 간주하지 않는다.

`write_batch(&[&Delivery])`는 호출자가 보유한 유한 guard 묶음을 chunk target에 따라 누적하고 마지막 chunk를 한 번 flush한 후 Written을 순서대로 보고한다. 배치 크기/최대 보유 지연은 호스트가 정하며 제품 기본값은 가정하지 않는다. 빈 묶음은 IO를 하지 않는다. `BatchWriteError`는 append 수락 prefix(`appended`), 마지막 batch flush 성공으로 보장하는 prefix(`written`), 성공한 보고 prefix(`reported`)를 보존한다. append/flush 실패에서는 Written을 보고하지 않는다(`written=reported=0`, 앞서 자동 마감된 chunk도 보수적으로 별도 확인). append 거절 뒤의 수락 prefix는 명시 flush 후 report할 수 있고, IO 실패는 Poisoned다. 성공한 batch flush 뒤 보고 Busy이면 `[reported..written]`의 같은 guard로 `Delivery::report(Update::Confirmed(Boundary::Written))`만 재시도한다. append된 prefix를 다시 쓰지 않는다.

`write_delivery(&Delivery)`는 단일 guard의 append/완전 chunk flush 뒤 Written을 보고하는 저지연 경로다. Busy 뒤에는 위와 같이 report만 재시도한다. Expired 등 그 외 완료 오류는 StorageError::Completion으로 보존한다.

`SegmentWriter::sync_work`와 `FinishedSegment::sync_work`는 같은 private 검증/오류 매핑으로 session/target·파일 coverage를 확인한 뒤 실제 sync를 수행하고 Synced/Failed를 보고한다. IO 전 검증 거절·metadata Busy·기존 Poisoned는 `SyncWorkError { error, work: Some(work) }`로 guard를 반환하므로 올바른 파일/시점에 재시도할 수 있다. 실제 flush/sync 실패는 Failed를 보고하고 `work: None`이다. 반환한 work를 버리면 기존 Drop 계약대로 Unknown이다. query permit은 coverage의 직접 poll 경로를 제한하지 않는다. 다른 세그먼트의 pending/만료 범위를 단일 새 파일 sync로 확인하지 않는다. 다중 세그먼트 조정은 TASK-008 후속이다.

제품 타입은 외부 MCAP/prost wire 타입을 공개 API에 노출하지 않는다. `encode_record`/`decode_record`는 버전 봉투·입력/해독 길이 상한을 검사하고 사용자 payload는 opaque bytes로 유지한다. 현재 압축은 off, CRC/index는 on이다. 초기 파일/독립 판독·golden 근거는 [EV-010](../05_verification/verification_evidence.md#ev-010), 리뷰 수정·배치/재시도·버퍼/할당 회귀는 [EV-011](../05_verification/verification_evidence.md#ev-011), 실제 OS/장비 내구성과 성능은 미검증이다.

### 13.4 TASK-008 회전·보존 API — 2026-10-30

`SegmentLog<SegmentStore>`는 일반 경로의 단일 소유자다. 같은 대상의 delivery를 producer FIFO로 전달하며, 단일 writer와 공유하는 target lease를 회전 사이에도 유지한다. `RetentionConfig`는 세 보존 모드·disk/reserve·file bytes/age·index bytes·event/pre/post/quota·pin count/bytes/deadline을 명시한다. 파일 bytes와 index bytes는 pinned MCAP의 보수적 수량 상한으로 판정해 실제보다 일찍 회전할 수 있다. 설정이 최대 단일 record와 최소 인덱스를 수용하지 못하면 IO 전에 거절한다.

`append(record,payload,now_ns)`는 session/revision/schema/producer sequence·길이/시각을 사전 검증하고 bytes/time/records/index 한도 전에 회전한다. `rotate`는 chunk/footer 마감→실제 file sync→시스템 생성 이름 확정→directory sync→closed catalog 공개→다음 active 파일 순서다. 연속 모드는 용량/기간 한계에서 멈추고, 순환/이벤트 모드는 오래된 비보호 closed 파일만 삭제한다. active/이벤트/실제 읽기 핀 때문에 확보가 불가능하면 Capacity다. 삭제 후 directory sync만 실패해도 unlink된 파일은 현재 catalog에서 제외하고 writer를 Poisoned로 멈춘다. 재기동의 durable namespace 판정은 TASK-009다.

`protect_event(at,expires,now)`는 active chunk를 flush하고 retained 파일의 session mono min/max와 요청 pre/post 창을 대조한다. `EventReport.actual`은 파일별 레코드 min/max 범위와 요청 창의 교집합이며 개별 레코드의 연속 존재나 gap-free coverage를 뜻하지 않는다. 부족한 과거 창과 post 대기 상태를 별도 표시한다. 겹친 창은 unique segment bytes로 한 번만 차감하며, 겹친 active는 파일 상한 전량을 예약한다. 향후 post 범위의 append도 quota를 먼저 검사한다. `release_event`/`expire_events`만 보호를 해제하고 자동 만료로 삭제하지 않는다.

`pin`은 published closed 파일 한 개의 읽기 bytes/deadline을 제한한다. `consume_pin`은 남은 bytes와 기한을 검사하며, 만료 후에도 실제 IO 종료와 `release_pin` 전까지 삭제 보호를 유지한다. Drop한 pin을 자동 해제로 추정하지 않는다. `ReadPin`은 `must_use`이며 오류/취소 경로에서도 보관 후 실제 읽기가 모두 끝나면 명시 반환한다. 토큰 유실 시 해당 catalog가 살아 있는 동안 보호와 pin slot이 남아 `PinLimit`/`Capacity`가 발생할 수 있다. 기한만으로 IO 종료를 증명할 수 없으므로 자동 강제 회수는 하지 않는다. 리더 수명 관리·중단/종료 확인 후 반환 및 고립 회수 조건 검토는 TASK-010 후속이다([REV-010 RV-012](../05_verification/reviews/REV-010_TASK-008_CODE_REVIEW.md)).

`write_batch`는 회전된 이전 파일과 마지막 완전 chunk를 대상으로 Written을 보고한다. `RetentionBatchError`는 원래 Capacity/EventQuota/IO/Completion 원인과 appended/written/reported prefix를 보존한다. 부분 append 거절 시 accepted prefix를 다시 쓰지 않고 명시 flush/report한다. 최종 flush 후 보고만 실패하면 report만 재시도한다. `sync_work`는 모든 과거 rotation 파일이 이미 동기화됐다는 사실과 같은 owner의 producer별 수락 범위·active 실제 sync·directory sync를 함께 확인한다. IO 전 거절/Busy는 원 work를 반환하고 실제 IO 실패만 Failed다. 보존 삭제는 과거 Synced 관측을 취소하거나 파일 영구 보존을 약속하지 않는다.

`SegmentStore`는 fresh filesystem 여유 관측·create/finalize/remove·directory sync capability를 제공하는 host port다. logger catalog/event/pin/인코더 상태는 공유 output 예산에 포함하며, OS cache·host adapter 자체 상태와 외부 프로세스의 공간 경쟁은 별도다. 표준 `FileStore`는 신선한 session/target namespace와 Unix directory sync를 제공하고 다른 OS는 Unsupported로 거절한다. OS별 어댑터/native·실장비 durability는 미검증이다. `finish`/Drop은 control의 Synced 성공을 만들지 않는다.

### 13.5 TASK-009 중단 복구 API — 2026-10-30

`recover_segment(source, budget, channels, options, sink, cancel)`는 일반 경로에서 변경이 멈춘 `Read + Seek` 원본과 독립된 빈 `SyncSink`를 사용한다. 호출자는 원래 session/revision·생산자 수·동일 순서의 채널/사용자 schema 등록을 제공한다. 호스트가 원본을 읽기 전용으로 열고 복구 중 불변성을 보장한다. 자동 startup namespace 발견/기존 target 재개나 worker 기동은 하지 않는다.

`RecoveryOptions`는 출력 `WriterConfig`와 `RecoveryConfig { max_source_bytes, max_record_bytes, max_chunks }`를 명시한다. source 크기 초과는 출력 쓰기 전 거절한다. 고정 body buffer·metadata/decoder 임시 메모리·참조/sequence registry를 `RecoveryOptions::reservation`으로 공유 output 예산에서 추가 예약하며, 별도 writer 예약과 함께 검사한다. 파일 길이/u64 overflow를 검사한 뒤 고정 buffer 안에서만 읽는다. host source/sink buffer·allocator 내부·OS cache는 별도다.

현재 제품 writer의 `dot-logger/1` 무압축 형식을 대상으로 header/magic·record 길이/opcode·nonzero chunk CRC·schema/channel 선행 참조·등록 metadata·봉투 식별자/시각·생산자별 증가 sequence·chunk 시각 범위를 검증한다. 완전한 chunk 전체를 검증한 뒤에만 해당 chunk의 메시지를 출력한다. 중간 오류가 있는 chunk의 앞선 메시지도 제외하며 이후 바이트를 재동기화하지 않는다. 지원하지 않는 압축·CRC 부재·record/chunk 한계도 명시 stop 원인이다. 손상/누락된 옛 summary/인덱스를 신뢰하지 않고 새 표준 summary/보조 인덱스를 만든다. 복구 봉투는 현재 버전의 알려진 필드를 재인코딩하며 사용자 payload는 opaque bytes로 보존한다. 원본 파일은 수정하지 않는다. 형식 판단 근거는 [MCAP 규격](https://mcap.dev/spec)과 고정된 mcap0.26.0 writer/독립 판독 시험이다.

`RecoveryReport`는 원본 길이·마지막 완전 검증/복사 chunk 끝(`verified_end`, 없으면0)·반개구간 제외 suffix·chunk/record 수·stop 원인을 보고한다. 제외 suffix에는 새로 만드는 옛 message index/summary도 포함하며 전부 손실된 메시지라는 의미가 아니다. 보고된 count/offset은 실패 시에도 앞서 완전히 복사한 prefix만 나타낸다. 원본 CRC 손상/절단/입력 한계에서 얻은 prefix는 성공한 복구 파일로 반환할 수 있지만, IO·writer 예산/용량 오류·취소는 `RecoveryError { error, report }`이고 출력을 성공으로 게시하지 않는다. 원 OS 오류는 보존한다. 취소는 유한 record/chunk 경계와 최종 file sync 이후에 확인한다. 진행 중 OS IO를 강제로 취소하거나 deadline을 보장하지 않는다.

`RecoveredSegment`는 새 파일의 finish+실제 file sync를 마친 `FinishedSegment`와 보고서다. 과거 프로세스의 Written/Synced 응답을 추정하거나 completion registry에 복원하지 않는다. `recover_to_store`는 create→복구/file sync→close→finalize→directory sync를 구동한다. capability 미지원은 생성 전 거절하며 모든 단계 실패를 원인/앞선 prefix와 반환한다. 실패한 active/rename 파일은 조사용으로 보존하고 자동 삭제·재시도·catalog 공개를 하지 않는다. FileStore의 Unix 지원/배타적 namespace/기존 경로 거절 계약을 유지한다. 대상 IO 실패→기존 completion 보고 및 StorageDegraded 계측 연결은 호스트/후속 worker·TASK-014 범위다. 실제 전원 차단·OS별 장애·재기동 durable namespace 수락은 TASK-022 이후 검증이며 복구 primitive 통과로 완료하지 않는다.

## 14. 조회·재생·원격 계약안

| 작업 | 계약 |
|---|---|
| `Query` | session + time_basis(mono/real/source) + [from,to) + 채널/레벨 + snapshot + cursor. source 축은 clock domain·변환 정보 없으면 명시 거절 |
| 소형 페이지 | 기본 시험 상한 1,000건 또는 256 KiB 중 먼저 도달. 커다란 원본은 descriptor와 blob handle을 반환해 페이지 초과 방지 |
| 원본 읽기 | blob ID + offset + bounded length, 만료·보존 삭제는 Gone. 무제한 전체 파일 응답 없음 |
| 정렬 | 오프라인은 `(mono, producer_id, accepted_seq)`로 결정적 정렬. 벽시계 조회는 real + tie-break. live는 수락 순서/부분 순서와 late 여부를 노출 |
| Snapshot | segment/offset·설정 revision 고정. 후속 기록은 다음 snapshot. 핀 만료 시 cursor 만료 오류 |
| Replay | start/pause/resume/seek/stop, 유한 양수 배속. mono 기준 상대 스케줄, seek 때 generation 증가·기존 전송 취소/폐기 구분 |
| 늦은 replay 소비자 | offline은 제한 prefetch 안에서 재생 시계를 늦추고 지연 보고, live는 해당 구독만 drop/disconnect. RT 생산자까지 대기 전파 없음 |
| Subscribe | 필터·최대 records/bytes/age·원본 opt-in. overflow는 Gap(범위/이유) 후 계속 또는 disconnect 정책을 시작 때 선택 |
| 원격 완료 | receiver가 전체 record 조립·검증·소비 큐 인계한 뒤 receipt ACK. 네트워크 send 완료와 구별 |

gRPC/Protobuf를 통신 후보로 한다. `GetStatus`, `Query`, `ReadBlob`, `Replay`, `Subscribe`, `ApplyConfig`, `RunCheck`, `Compare`, `GetCompletion`을 서비스 메서드 후보로 둔다. bulk payload는 최대 256 KiB 조각, 레코드 ID/offset/총 길이/checksum과 완료 ACK를 갖는다. 연결별 동시에 조립할 원본 수·bytes·TTL을 협상해 메모리를 제한한다. 재연결 후 미확인 전송은 Unknown이며 자동 exactly-once를 제공하지 않는다. HTTP/2 흐름제어가 애플리케이션 큐 전체의 예산을 대신하지 않는다.

네트워크 서버는 기본 비활성, 활성화 시 기본 loopback이다. 비-loopback은 인증/TLS 구성을 명시하고 dot-probe의 기존 보안 정책과 연결한다. 파일 읽기·관측 권한과 설정·점검 권한을 구분한다. 원격 요청에서 임의 파일 경로·무제한 데이터/실행 예산을 받지 않는다. 서비스 token/인증 설정은 운영 로그에서 제외한다.

### 14.1 TASK-010 로컬 API — 2026-10-30

RV-021~023 조치: 동일 질의(시간축·범위·채널·최소 level)가 연속되면 사전 예약된 정렬 index를 재사용하며 질의가 달라지면 같은 버퍼에서 재구성한다. 이전 cursor의 위치/필터·취소/만료·결정적 순서는 유지한다. `Payload::Inline` 좌표는 해당 Page 안에서만 유효하며 원본 파일 위치가 아니다. 바이트는 같은 Page의 record를 `Page::inline`에 전달해 얻는다(범위 검사는 있으나 Page identity 검증은 없음).

`query::{QueryBudget, QueryConfig, QuerySource, Snapshot, Query, Cursor, Page, QueryRecord, Payload, BlobHandle, TimeBasis, QueryError}`는 일반 경로의 동기식 primitive다. 현재 고정 session/revision·동일 순서의 등록·한 target의 유한 segment 목록을 사용한다. 호스트가 각 파일의 독립 read-only descriptor를 열고 snapshot 생존 중 공개된 원본 prefix의 불변성을 보장한다. query가 파일 경로 문자열을 입력받거나 worker를 시작하지 않는다.

`QueryBudget::new(plan)`는 session당 한 번 생성해 external 예약을 공유한다. 현재 query가 이 partition을 사용하며 다른 DB/remote/replay adapter 도입 시 합계 예약을 연결해야 한다. `QueryConfig::reservation(channels)`는 frame/decode scratch·유한 record/order index·페이지 metadata/inline bytes·blob read buffer·등록 검증 임시 메모리의 보수적 requested-byte 상한이다. source adapter 자체 메모리·OS cache/allocator 내부는 별도다. 동시 snapshot 합계·source bytes/segment/record/metadata/producer·누적 read bytes와 개별 blob read 길이를 제한한다. QueryConfig::read_bytes는 모든 source의 safe_offset 합계(초기 scan의 보수적 차감)와 이후 page inline/blob 재읽기 전체를 포함해야 한다. prefix 합계만 예약하면 scan 직후 payload 재읽기 여유가 없고 반복 page/blob 호출도 다시 차감된다.

`Snapshot::new`는 `Read+Seek` source별 frozen safe offset 안에서만 `dot-logger/1`의 무압축·nonzero chunk CRC·schema/channel/봉투/sequence/시각을 검증한다. source size/overflow·잘못된 framing/참조는 실패하며 부분 snapshot을 반환하지 않는다. offset0은 아직 공개 chunk가 없는 빈 active 범위다. 완전 검증 chunk만 index하며 뒤의 미완성/후속 기록을 노출하지 않는다. 현재 closed 파일도 data section을 bounded scan하여 조회 index를 재구성한다. 옛 summary/보조 index를 정답으로 신뢰하지 않으며 표준 summary를 이용한 scan 생략 최적화는 구현하지 않았다. 이 동작의 IO/지연은 장비 성능 달성 근거가 아니다.

`Query`는 mono/real/명시 source clock domain의 native ticks에서 `[from,to)`와 선택 채널·최소 level을 고정한다. source domain 누락/불일치는 ClockDomain이며 unrelated clock/UTC 변환을 추정하지 않는다. order는 `(선택 축 시각,producer_id,accepted_seq,segment_seq,payload_offset)`다. 벽시계 역행을 허용한다. offline query이며 live late 표시/subscribe는 TASK-012다.

`start`의 opaque Cursor는 snapshot 식별자·불변 filter·위치를 보유한다. `page`는 immutable cursor를 입력받아 최대 `page_records` 및 `page_bytes` 안의 borrowed Page와 next Cursor를 반환한다. bytes에는 `size_of::<QueryRecord>()`의 metadata/descriptor 및 실제 inline payload를 포함한다. wire encoding 상한은 TASK-012에서 별도 검사한다. inline 한도를 넘거나 단독 페이지에 들어가지 않는 payload는 원본 길이/receipt와 snapshot-local BlobHandle로 반환한다. `read_blob`는 이 snapshot의 핸들·payload 상대 범위·개별/누적 한도만 허용한다. 다른 snapshot 핸들·만료된 blob은 Gone, 다른 cursor는 Cursor 오류, 만료된 cursor는 Expired다. source lifetime/핀 deadline 중 가장 이른 시각에 만료한다. 관측된 시각의 최대값을 기억하여 이미 만료된 cursor/blob을 과거 인수로 되살리지 않는다. 취소는 요청을 끝내며 이미 진행 중인 OS IO를 강제 종료하지 않는다.

`SegmentLog::{active_boundary,active_query_source,query_source}`는 complete-chunk offset 또는 published closed 길이를 동결하고 reader가 소유하는 읽기 permit을 제공한다. active permit은 같은 sequence가 회전해 closed가 돼도 삭제를 막는다. QuerySource는 reader를 먼저 Drop하고 permit을 나중에 반환하며 오류/취소/만료만으로 보호를 일찍 해제하지 않는다. catalog는 실제 종료가 게시된 슬롯만 다음 관리 작업에서 회수한다. 저수준 ReadPin의 명시 release 계약은 유지하고, 토큰 유실/`mem::forget`/종료를 확인할 수 없는 reader에 대한 강제 회수는 도입하지 않았다. reservation authority도 permit이 보유하여 catalog가 먼저 사라져도 잔여 reader metadata의 예산이 유지된다. `QuerySource::active` 직접 생성은 호스트가 truncation/replacement/삭제 방지를 별도로 보장하는 standalone writer/오프라인용이다.

초기 구현 근거는 [EV-016](../05_verification/verification_evidence.md#ev-016), REV-014 조치의 최신 로컬 검증/hash는 [EV-017](../05_verification/verification_evidence.md#ev-017)이다. 실제 장비 p95/IO 간섭·native/OS 교환·replay/remote/CLI는 별도 후속이다.


### 14.2 TASK-011 로컬 재생 API — 2026-10-30

`replay::{Replay, Speed, ReplayState, ReplayPoll, ReplayFrame, ReplayPayload, Transition, ReplayError}`는 `Snapshot<Read+Seek>`을 소유하는 일반 동기식 offline API다. 자동 timer/worker/전송을 시작하지 않으며 RT 수집에 대기를 전파하지 않는다. host는 snapshot/pin과 같은 단조 ns clock domain의 now를 제공하고 Replay는 최대 관측값으로 역행을 막는다. `new`는 mono Query만 허용하고 시작 시각에 query.from을 anchor로 사용한다. leading gap을 포함한 원래 mono 간격과 `(mono,producer,seq,segment,offset)` 순서를 보존한다. 실시간/SourceTime 재생 변환은 추정하지 않는다.

`Speed::new(numerator:u32, denominator:u32)`는 양수 유리수 배속(0.5/1/2 포함)을 표현하며 0 성분은 거절한다. `Speed`의 `Eq/PartialEq`는 입력 성분을 비교하고 `is_equivalent(&other)`는 u64 교차곱으로 비율을 비교한다(1/1과 2/2는 성분 불일치·비율 일치). `Speed::REALTIME`은 1/1이며 `numerator()`/`denominator()`는 약분 없이 입력 성분을 돌려주고 Display는 `분자/분모`(예: `6/4`)로 표시한다. `ReplayPoll`·`ReplayFrame`·`ReplayPayload`는 일반 경로 진단을 위한 Debug를 제공한다. 정수 u128 산술과 deadline ceil을 사용하고 u64 deadline 표현을 넘으면 Limit으로 중단한다. `pause`/`resume`는 시계 진행을 멈추고 이어가며 반복 호출은 idempotent다. `set_speed`는 현재 정수 ns media 위치를 유지하고 전환 때 1ns 미만 소수부는 내림한다.

`poll(now_ns, consumer_ready, cancel)`는 최대1건만 prefetch한다. 기존 Snapshot의 사전 예약 page/order/blob 버퍼와 추가 heap 없는 controller metadata를 사용하며 shared external 예약은 그대로다. 아직 due가 아니거나 소비자가 준비되지 않았으면 `Waiting { due_ns, late_ns }`, paused면 읽기 없이 Paused다. Ready는 원 receipt/metadata·borrowed inline 또는 blob descriptor·generation·late_ns를 반환한다. 늦은 실제 전달만큼 offline 시계를 늦춰 후속 간격을 보존한다. blocked 상태에서 pause/배속 전환으로 media 위치가 미전달 레코드를 넘었어도, 전달 시 그 레코드 위치로 재기준화해 나머지를 몰아서 보내지 않는다. repeated blocked poll은 원본을 재읽거나 read 예산을 다시 소비하지 않는다. 데이터/범위가 없거나 끝나면 End이며 빈 범위에서 손실 건수를 추정하지 않는다.

`seek(from_ns, now_ns, cancel)`는 upper bound/채널/level을 보존하고 lower bound를 바꾼다. 검증 성공 뒤 generation을 checked 증가시키며 기존 미전달 prefetch를 폐기한다. 이미 반환된 동기식 전달은 취소한 것으로 보고하지 않는다. borrowed frame이 살아 있으면 Rust borrow가 seek/stop을 막는다. 외부 adapter가 복사한 전송은 generation을 확인해 직접 폐기해야 하며 네트워크 in-flight 취소는 TASK-012다. 잘못된 range/generation 한도 거절은 이전 상태를 보존한다. seek는 pause를 유지하고 End에서 새 재생을 시작할 수 있다.

큰 payload는 원본 descriptor를 유지하고 `read_blob(generation, handle, offset, length, now_ns, cancel)`로 같은 Snapshot의 개별/누적 예산 안에서 읽는다. 이전 seek generation은 StaleGeneration으로 거절한다. inline/page 재사용과 blob의 Gone/원 IO/한도 판정은 query 경로를 사용한다. lifecycle 검사에서 이미 관측한 만료/취소는 `ReplayError::Query(Expired/Cancelled)`로 terminal stop한다. 실제 IO 중 발생한 원인도 반환 후 terminal stop하며 실제 reader close 뒤에만 pin과 reservation을 반환한다. `stop`/Drop은 미전달 descriptor를 버리고 실제 reader를 닫는다. stop은 idempotent·terminal이며 End는 마지막 blob read를 위해 stop/Drop까지 snapshot을 보유한다.

`ReplayError`의 Display는 variant별 설명을 출력하며 Query는 기존 QueryError 표시와 원 IO 정보를 유지한다. 오류 분류/terminal 정리 및 배속 전환 동작은 유지한다. 검증은 [TASK-011](../04_implementation/tasks/TASK-011.md)과 EV-018·[EV-019](../05_verification/verification_evidence.md#ev-019)·[EV-020](../05_verification/verification_evidence.md#ev-020)에 기록한다. 실제 timer 지터·p95/수집 IO 간섭·native/OS/장비·remote/live/CLI는 별도 후속이다.

### 14.3 TASK-012 원격 어댑터·host 관리 경계 — 2026-10-31

`adapters::remote`는 선택 `remote` feature의 일반 경로다. `ServerConfig`는 기본 disabled/loopback/익명 read-only, `RemoteServer::bind`와 host 소유 Tokio runtime의 `serve(shutdown)`으로 명시 시작한다. 외부 bind는 인증 credential과 실제 TLS identity가 모두 있어야 한다. read/manage token을 분리하고 ApplyConfig/RunCheck는 Manage만 허용한다. 잘못된 인증 헤더는 익명으로 낮추지 않는다. 원격 입력에는 파일 경로를 받지 않고 native capability ID/고정 필터·기한/예산만 사용한다. secret 타입은 Debug를 제공하지 않으며 전송 오류는 credential을 출력하지 않는다. 테스트 PEM은 fixture 전용이며 운영 인증 정보가 아니다.

[logger.proto](../../proto/logger.proto)의 `dot_logger.remote.v1.Logger`는 GetStatus/Query/ReadBlob/Replay/GetCompletion/ApplyConfig/RunCheck/Compare·Subscribe·Ack를 제공한다. major1, 16-byte LE session/schema ID, 16-byte LE signed query bounds, enum validation·encoded message cap을 적용한다. `scripts/check-remote-protocol.py`로 checked-in prost 필드/태그/반복·optional 종류를 schema와 대조하며 protoc는 설치 시 compiler 검증도 한다. schema를 빌드 때 자동 생성하거나 core에 네트워크 의존을 강제하지 않는다.

`RemoteHub::new(&QueryBudget, RemoteLimits)`는 query와 같은 세션 external authority에서 유한 예약을 먼저 얻는다. TCP 연결/handshake·동시 stream·command·subscription·queued+awaiting-ACK records/bytes·encoded message·record·age를 제한한다. 조각 payload는 최대256KiB이며 message cap에서 envelope 여유1024B를 뺀 크기도 적용한다. HTTP2 header/window/reset/frame·동시 연결/요청 한도를 명시한다. adapter buffer 예약은 Requested bytes의 보수적 상한이며 host callback/source/sink buffer 및 Tokio/TLS/HTTP2/runtime/allocator 내부·OS socket/cache headroom은 별도 worker/host 예산으로 기록한다. HTTP2 흐름제어를 전체 관리 메모리 한도로 주장하지 않는다.

`RemoteHub::subscriptions`로 유한 ID 목록을 관측하고 `offer`/`offer_delivery`가 **일반 경로에서** payload를 복사한다. 채널/레벨/원본 opt-in을 검사하며 큐와 ACK 대기 항목의 bytes/records를 함께 차감한다. Gap 정책은 신규 기록을 버리고 구독 내 연속 transfer position의 first/last·reason=overflow를 보낸다. Gap 전까지 들어온 기록이 먼저 전달되며 미통지 gap이 있으면 신규 기록도 같은 연속 gap에 합친다. Disconnect 정책/record age/idle lease는 해당 stream만 종료하고 pending을 Unknown으로 한다. live mono 역행은 late로 표시하며 원본 receipt/revision/clock/schema를 유지한다. 미등록 생산자별 순서를 시간만으로 전역 순서라고 꾸미지 않는다.

한 전송은 모든 조각을 stream에 제공한 뒤에도 Pending이며, **같은 실제 TCP 연결**에서 receipt/transfer/generation이 일치하는 Ack만 받는다. `Reassembler`는 공유 external 예약·동시 원본 수/bytes·TTL 안에서 total/record identity·generation·정확한 offset·전체 CRC를 검증한다. `ReceivedRecord`는 소비자가 handoff를 수락할 때만 Ack를 만든다. 거절/Drop/TTL/이전 generation은 ACK가 아니며 permit을 실제 소유자 수명까지 보유한다. `discard_generations`는 제한된 generation gate를 올려 incomplete·늦은 조각·ready지만 미인계인 이전 세대를 거절한다. 이미 소비자에게 인계되었거나 네트워크 in-flight인 bytes를 취소했다고 주장하지 않는다. seek를 처리하는 host는 해당 구독의 `RemoteHub::discard_generations`와 수신 generation gate를 함께 갱신한다.

`RemoteDelivery::poll`은 원래 Delivery guard를 보관하면서 ReceiverAcknowledged/Unknown/명시 Dropped를 실제 completion에 게시한다. 보고 Busy는 guard를 유지하고 다음 host turn으로 넘긴다. 바이트 send 또는 Query 조회가 receiver 완료를 만들지 않으며 unreported Drop의 기존 Unknown 계약을 유지한다. 파일의 Written/Synced와 별도 경계다.

REV-019 보강: 연결 한도 초과 TCP는 기존처럼 닫되 이후 accept 재폴링은 10ms timer를 기다려 연속 backlog에 의한 자기 wake 반복을 억제한다. 한도/기존 연결 permit과 shutdown 의미는 유지하며 실제 CPU 점유율 상한을 보장하지 않는다. Bearer credential은 bind 시 길이와 고정256B 배열로 정규화하고 기존 lock의 subtle2.6.1로 고정 크기 bytes/길이를 비교한다. 이는 비교 primitive의 보강이며 전체 RPC·토큰 목록 탐색의 constant-time이나 공격 불가능을 보장하지 않는다.

수신자는 실제 transport 연결별 Reassembler를 소유한다. 서로 다른 구독의 generation gate는 `records`개까지 누적되고 동일 구독 갱신은 추가 슬롯을 쓰지 않는다. 미완성 원본 만료가 gate를 지우지 않는다. gate `Limit`이면 연결을 종료하고 새 연결에 새 receiver를 만든다. 재연결 시 이전 조각 라우팅/미인계 owner를 정리하고 이전 ACK를 새 연결로 보내지 않는다. 이전 owner/IO의 예산은 실제 종료까지 유지한다. 늦은 조각·ready handoff 방어를 위해 gate를 자동 삭제하지 않는다. TASK-013 transport가 이 수명을 연결한다.

관리 unary RPC는 유한 `CommandInbox`로 native `ManagementRequest`를 전달한다. host가 `Command::request`에 따라 실제 Snapshot/page/blob·Replay·Completion 또는 등록된 관리 기능을 실행하고 `respond`로 native reply를 돌려준다. 미구현 기능은 Unsupported로 명시한다. Query/ReadBlob/Replay의 handle/TTL·page/item/blob 권한/reader 종료는 host가 기존 primitive와 제한된 capability 목록으로 관리한다. timeout/취소는 대기만 끝내며 host Command가 살아 있으면 permit/예약을 회수하지 않는다. `cancelled`를 actual IO 종료로 해석하지 않는다. CLI/실제 설정 적용·RunCheck/Compare backend 자체는 TASK-013/016/017 및 관련 Task다. 관리 응답의 type/record 수·encoded bytes·소유한 Vec capacity 및 요청 blob length도 제한하며 과대 응답은 Limit이다.

초기 로컬 근거와 자가 리뷰는 [TASK-012](../04_implementation/tasks/TASK-012.md)·[REV-018](../05_verification/reviews/REV-018_TASK-012.md)·[EV-022](../05_verification/verification_evidence.md#ev-022)에 기록한다. 실제 LAN 지연/처리량·network/runtime 전체 RSS·dot-probe 통합/운영 보안 정책·다른 OS/native는 별도 검증이며 core 소유권/RT·실제 저장 경로를 바꾸지 않는다.

## 15. 설정·운영 계측·점검 계약안

`validate_config(base_revision, patch)`는 차이·예산·정보 손실·재시작 필요 여부를 반환한다. `apply_config`는 명시 요청만 수행하고 revision 불일치면 Conflict다. 라우팅/필터는 producer 경계에서 적용하며 모든 생산자가 같은 순간에 바뀐다는 보장을 하지 않는다. 응답은 posted revision과 per-producer applied revision을 분리한다. old plan을 처리 중이면 추가 업데이트는 RevisionBusy다.

RT는 producer-local 고정 카운터만 갱신하고, 관리 경로는 고정 atomic snapshot을 읽어 1초마다 집계하는 안이다. 상세 지연 histogram은 샘플링 또는 시험 모드로 분리한다. 채널 ID는 등록된 유한 집합이고 문자열 label·record ID를 지표 label로 늘리지 않는다. 기본 이력은 10/60/300초 추세에 필요한 고정 ring이며 raw 관측 무제한 보관은 없다.

보고서 값은 `Observed`, `Validated(profile_id)`, `Estimated(assumptions)`, `Unavailable(reason)`, `Stale`로 구분한다. 부하율의 분모는 동일 장비/기능/부하 조합의 검증된 완료 능력이다. 포화 예상은 가용 bytes와 slots 각각의 양의 순증가율로 계산해 먼저 오는 한계를 표시한다. 거절/폐기로 큐가 정체한 경우 Overloaded를 유지한다. 순환 보존 창과 디스크 소진 시간을 다른 필드로 둔다.

`RunCheck`는 dry-run 계획에서 시간·CPU worker 수·RAM·총 쓰기 bytes·디스크 reserve·상대 수신자·완료 경계를 먼저 보여준다. 실행은 명시 요청이다. 어느 예산이 먼저 소진되거나 운영 오류/공간 reserve 침범·취소가 오면 신규 부하를 중단하고 진행 중 IO를 정리한다. 결과는 Pass/Fail/Aborted/Inconclusive이며 짧은 점검을 24시간/UPS 검증으로 바꾸지 않는다. 사용자가 지정한 전용 시험 디렉터리만 만들고 지운다.

`Compare`는 before/after의 profile/revision/window·완료/지연/손실·의도적 생략·CPU/메모리를 대조한다. 다른 장비·부하/측정 경계는 비비교 가능으로 표시한다. 입력 자체를 조절하는 코드는 사용자 앱의 책임이며 로거의 샘플링은 기록량만 바꾼다. FR-015 추천은 제안 기능으로 명시하고 자동 적용을 하지 않는다.

## 16. 동기화·긴급 동작의 구체안

정상 파일 동기화는 explicit 또는 interval 옵션이다. 시험 프로필은 1초 요청 주기를 쓰되 실제 sync 완료 지연/미동기화 bytes를 계측한다. 제품 데이터 손실 상한으로 해석하지 않는다. 기본 모드·disk quota는 명시 설정이 필요하다. compression은 첫 성능 기준선에서 off, chunk CRC와 인덱스는 on이다.

FR-018 활성화는 재시작 때만 허용하는 안이다. 시작 전에 중요 파일 rank, 최대 미동기화 bytes, 신호 지연 상한, 안전 여유, 현재 장비의 검증 프로필을 받는다. 검증 프로필이 없으면 시험용 활성화임을 표시하고 Supported/3초 보장을 반환하지 않는다. 같은 rank는 target ID 순으로 결정적으로 처리한다.

긴급 통지는 사전 할당 atomic mailbox에 사건 ID와 **가장 이른 deadline**을 게시한다. 중요 snapshot용 사전 예약 슬롯 외의 신규 입력을 닫고, 선택 출력의 신규 작업을 중지한다. 실행 중 OS IO는 즉시 취소됐다고 가정하지 않으며 원본 참조를 유지한다. 이미 진행 중인 파일 sync의 최악 지연도 전체 예산에 포함한다. 정상 종료와 긴급 종료는 동일 coordinator가 상태 전이한다. 긴급 종료 뒤 자동 Running 복귀는 하지 않는다.

`available = poweroff_deadline - signal_received - safety_margin`이며 signal_received가 늦어졌으면 예산도 줄어든다. 전체 판정에는 신호 생성→수신의 지연을 포함한다. 예상 drain/sync가 available보다 크면 Unachievable과 파일별 partial 결과를 보고한다. 비활성에서는 긴급 전용 예약/전이가 없고 Disabled를 반환한다.

## 17. 첫 검증 구성과 상세 리뷰 경계

- 수집: 2 importance, Normal 기본, explicit → channel policy → Normal, Important 예약 slots/bytes, 신규 거절, producer별 FIFO.
- 원본/완료: owner가 메모리 소유, 슬롯별 반납 표시, bounded 완료 window, partial/expired/unknown 구분.
- 파일: MCAP·chunk CRC/index, 제한 회전, active safe offset, 복구 파일 별도 생성, 명시 quota/보존 설정.
- 활용: Rust API + CLI 범위는 승인. query/subscribe/replay·DB/gRPC 상세도 기준선 채택 완료.
- 관측/계측의 승인된 합격선: 안정 합성 입력에서 카운터 정합성은 배출 후 정확 일치, 안정 추세의 포화/소진 시간 추정 오차 ≤20%(5초 이상 예측 구간), 기본 계측 활성/비활성 완료 처리량 차이 ≤5%. RT 호출 목표는 계측 포함해서 기존 §12 목표로 판정한다. 이 추가 목표는 2026-10-26 Human 승인, 실제 달성은 미검증이다.
- 검증 장비 확정(ADR-011): 현재 PC 실측으로 실제 장치 시험을 대체 수락하며 UPS는 시뮬레이션으로 판정한다. TASK-022의 실행 커널/제어 주기·deadline·부하 프로필과 실행 증거로 판정한다. native는 공개 완료 후 진행하며 미실행을 Pass로 표시하지 않는다.

TASK-004 게시 후 native CI는 GitHub 계정 결제/지출 한도로 8개 job 모두 실행 전 차단됐다. 해당 revision의 제품/native 시험은 Not Run이며 재실행 조건은 EV-003·GAP-004에서 추적한다.

## 검증 방법·시점 갱신 — 2026-10-31

[ADR-011](adrs/ADR-011_local_pc_simulated_ups_verification.md)에 따라 §12/16 및 UPS·복구의 실물/실제 차단 필수 시험은 모의 신호·기한/지연·실제 파일 IO와 중단/재개/복구 시뮬레이션으로 대체 수락한다. 실제 장치 시험은 현재 Ubuntu PC 실측을 사용하고 GitHub native는 공개 완료 후 진행한다. 이전 실물 필수/장비 미확정 표기는 이 범위에서 대체하며 API·예산·순위·partial/오류 의미는 유지한다. 실행 중 generic/설치된 PREEMPT_RT는 TASK-022에 구분하며 커널별 시험 결과를 혼합하지 않는다.

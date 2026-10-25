# ADR-001 — Rust 재사용 라이브러리와 Linux + PREEMPT_RT 목표 환경

## 상태·날짜

`Superseded` · 2026-10-25 · 결정자: Human

대체: [ADR-006](ADR-006_cross_platform_linux_macos_windows.md). 아래 본문은 초기 결정 이력이며 현재 지원 범위는 Rust·Linux/macOS/Windows와 Linux PREEMPT_RT 프로필이다.

## 맥락과 결정 근거

Human은 `dot-logger`를 다른 프로젝트에서 사용하는 Rust 라이브러리로 제작하고, Linux + PREEMPT_RT에서 로보틱스의 실시간 고성능 로깅·텔레메트리를 제공하도록 지정했다. 첫 적용 대상은 `dot-probe`다.

주 언어와 목표 실행 환경은 API·의존성·실시간 검증 장비 및 운영에 장기 영향을 주므로 ADR로 기록한다. 대안 언어·OS를 비교한 결과로 포장하지 않고, Human이 제시한 프로젝트 제약을 기록한다.

## 결정

- 로깅 서버의 재사용 단위를 Rust 라이브러리로 제작한다.
- 로깅 서버의 목표 환경은 Linux + PREEMPT_RT다.
- 실시간 고성능·24시간 동작 신뢰성·파일 관리·시계열 DB 연동·기록 데이터 조회를 요구한다. 상세 수치와 수락 기준은 순차 확인한다.
- 모니터링 클라이언트는 라이브러리 또는 서비스로 제작하며, 구체 형태와 실행 환경은 미정이다.
- 대표 프로젝트·개발 방향을 먼저 조사하고 Human에게 한 번에 하나씩 필요한 사항을 확인한다.

## 영향과 미결 범위

Rust와 PREEMPT_RT만으로 호출 지연 상한이나 무손실을 보장하지 않는다. 호출 경로·부하·장애 가정과 타겟 환경에서의 검증이 필요하다.

Rust 버전/MSRV, CPU 아키텍처, 큐, 로깅 프레임워크, 파일 형식, DB, 프로토콜, 별도 서비스 유무는 이 결정에 포함되지 않는다. 조사 후보는 미승인이다. `dot-probe`의 기존 결정도 자동으로 dot-logger의 결정으로 취급하지 않는다.

## 관련 문서

- [INT-001](../../02_project_definition/intents/INT-001_realtime_robotics_logging.md)
- [기술스택](../tech_stack.md)
- [요구사항](../../02_project_definition/requirement_spec.md)

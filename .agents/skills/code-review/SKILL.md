---
name: code-review
description: >-
  dot-logger 프로젝트의 Rust 코딩 컨벤션, RT(실시간) 제약, 동시성/원자성 안전성, 품질 게이트 및 소통 규칙에 따라
  변경 사항과 코드를 정밀하게 검토하고 표준 보고 형식으로 결과를 출력하는 코드리뷰 스킬.
  Rust 코드 리뷰, PR 검토, 변경점 품질 검사, 자가 리뷰 요청 시 사용한다.
---

# dot-logger 코드리뷰 스킬 (Code Review Skill)

이 스킬은 `dot-logger` 프로젝트의 고유한 실시간(RT) 제약, Rust 언어 및 동시성 안전성, 품질 게이트, 그리고 프로젝트 소통 규칙에 맞춘 코드 리뷰 절차를 정의합니다.

---

## 1. 코드리뷰 핵심 검토 기준 (Checklist)

### A. RT(실시간) 경로 제약 (CRITICAL)
- [ ] **동적 메모리 할당/해제 금지**: RT 호출/거절/Drop 경로에서 `Box`, `Vec`, `String` 할당 또는 deallocation이 발생하는지 확인.
- [ ] **Blocking & I/O 금지**: RT 수집 경로에서 Mutex lock 대기, 채널 blocking, 파일/네트워크 I/O가 없는지 확인 (lock-free/무대기 큐 또는 원자적 상태 전이 준수).
- [ ] **문자열 포맷팅 금지**: RT 경로에서 `format!`, 동적 에러 문자열 생성 없이 고정 에러 코드/`Result`를 반환하는지 확인.
- [ ] **무한 루프/무한 재시도 금지**: bounded retry 또는 즉시 drop/rejection 처리 여부 확인.

### B. Rust 언어 및 동시성/메모리 안전성
- [ ] **`unsafe` 최소화 및 Safety 주석**: `unsafe` 블록은 최소화되었으며, 불변식(Invariants), 메모리 순서(`Ordering`), 소유권/수명 증명이 주석으로 상세히 기재되어 있는가?
- [ ] **원자적 메모리 순서(Memory Ordering)**: `Relaxed`, `Acquire`, `Release`, `SeqCst` 선택이 정확하며 data race나 순서 역전 버그가 없는가?
- [ ] **오류 처리 건전성**: 프로덕션/라이브러리 경로에서 `unwrap()`, `expect()`, `panic!`을 남용하지 않는가?
- [ ] **자원 누수 및 Drop 순서**: 역전된 완료 순서나 비정상 종료 시 버퍼 누수나 좀비 태스크가 발생하지 않는가?

### C. 아키텍처 및 API 경계
- [ ] **공개 API 오염 방지**: 외부 크레이트의 내부 runtime/wire 타입이 공개 API로 노출되지 않았는가?
- [ ] **기능 격리**: Core 수집 코어와 DB/Remote feature 의존성이 철저히 분리되었는가?
- [ ] **TDD 및 테스트 충분성**: 버그 수정 시 재현 테스트가 선행되었으며, 경계값 및 실패 케이스 테스트가 작성되었는가?

---

## 2. 코드 검증 명령어 (Verification)

리뷰 시 필요한 경우 다음 명령어를 실행하여 정적 분석 및 테스트 결과를 확인합니다:

1. **포맷 검사**: `cargo fmt --check`
2. **정적 분석**: `cargo clippy --all-targets -- -D warnings`
3. **단위 및 통합 테스트**: `cargo test`
4. **동시성/Unsafe 검증 (필요시)**: `cargo test -- --nocapture` 또는 Miri 대상 테스트 실행

---

## 3. 리뷰 결과 보고 형식 (필수 준수)

프로젝트 소통 규칙(`communication_convention.md`)에 따라 다음 순서로 결과를 보고합니다:

1. **요약정리**: 목적, 전체 평가(Pass / Conditional / Reject), 핵심 발견점 및 위험도 요약
2. **파일 변경표**: 생성/수정/삭제 구분, 파일 경로(clickable link), 변경 내용 설명
3. **클래스(타입) 변경표**: Rust의 `struct`, `enum`, `trait` 단위로 구분, 소속 파일, 역할 및 변경 내용
4. **리뷰 포인트**:
   - **Critical / P1**: 즉시 수정이 필요한 안전성/RT 위반/버그
   - **Warning / P2**: 성능, 메모리 순서, 코드 스타일 개선 권고사항
   - **Note / P3**: 문서화, 주석 보강 및 기타 참고사항
   - **다음 액션**: 필요한 테스트, 승인 사항 또는 수정 방향 제안


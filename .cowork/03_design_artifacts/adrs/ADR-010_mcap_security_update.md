# ADR-010 — 압축 의존성 보안 수정에 따른 MCAP 0.26 채택

- 상태: Accepted / 날짜: 2026-10-26
- 승인: Human 「MCAP 0.26.0으로 변경 승인 (권장)」
- 관련: INT-001 / MS-002 / TASK-002 / NFR-004·007
- 관계: ADR-009의 MCAP 0.25 계열 선택만 대체. 다른 기술·계약은 유지.

## ADR 필요성 판단

보안과 의존성 호환·운영 유지보수가 함께 얽힌 변경이다. ADR-009에서 채택한 파일 라이브러리 계열을 바꾸므로 별도 승인·추적을 남긴다.

## 맥락과 결정 요인

TASK-002에서 기존 dot-probe lockfile과 같은 MCAP 0.25.0, zstd 0.13.3, zstd-safe 7.3.0으로 전체 feature를 검사했다. Rust 1.85.0/1.96.0 기능 시험은 통과했으나 RustSec 검사에서 취약점 3건(RUSTSEC-2026-0391·0403·0398)과 zstd-safe 안전성 경고 10건이 남았다. 이는 해당 의존 버전의 공지이며 현재 MCAP 경로에서 모든 영향 함수가 실행됨을 증명한 것은 아니다.

기본 무압축 구성에는 zstd가 포함되지 않지만 승인된 선택 압축 기능을 제공하려면 해당 의존성을 해소해야 한다. Tokio 1.51.0의 RUSTSEC-2026-0386은 같은 계열 수정판 1.51.6으로 해소했다.

## 검토한 옵션

| 선택 | 이점 | 비용·한계 |
|---|---|---|
| MCAP 0.26.0 채택 | zstd 0.14.3·zstd-safe 8.2.0 사용 가능, 취약점·안전성 경고 해소 | 기존 dot-probe의 MCAP 0.25와 서로 다른 crate 버전, native·통합 재검증 필요 |
| MCAP 0.25.0 유지·압축 결정 보류 | 기존 계열 유지, 기본 무압축 경로는 zstd 미포함 | 선택 feature의 보안 검사는 실패로 남고 승인 기능의 구현을 완료할 수 없음 |
| 자체 보안 패치·포크 | 0.25 API 유지 가능 | 보안 수정 유지·검증 비용 추가. 이 작업에서 구현하지 않음 |

## 결정

Human 승인으로 MCAP **0.26.0**을 채택한다. `Cargo.toml`과 `Cargo.lock`에 고정하고 `compression-zstd`는 계속 선택 기능·기본 off로 둔다. 임시 복사본에서 Rust 1.85.0 전체 feature 시험 19개와 보안 검사를 먼저 확인했고, 본 작업 트리에서도 최종 feature/MSRV·정적 검사를 수행한다.

## 결과와 검증 경계

- API에 MCAP의 외부 타입을 노출하지 않으므로 호출자 핵심 타입은 그대로다. 파일 형식·CRC/index·압축 기본값 계약도 유지한다.
- `paste` 1.0.15의 RUSTSEC-2024-0436(유지보수 중단)은 MCAP 두 버전 모두의 간접 빌드 의존성으로 남는다. 취약점·안전성 경고와 구분해 EV/GAP에 기록하고, 보안 CI는 취약점 및 `unsound`에서 실패하며 경고 자체는 숨기지 않는다.
- 라이브러리 호환/메모리 MCAP 왕복은 실제 파일 내구성·전원 차단·dot-probe 통합 완료의 근거가 아니다. TASK-007·019·022·024·026에서 해당 범위를 검증한다.
- 상세 실행·고정 버전·보안 DB revision은 [검증 근거](../../05_verification/verification_evidence.md)의 EV-001을 따른다.

## 출처

- [RustSec zstd 공지](https://github.com/RustSec/advisory-db/blob/main/crates/zstd/RUSTSEC-2026-0391.md)
- [RustSec zstd-safe 공지](https://github.com/RustSec/advisory-db/blob/main/crates/zstd-safe/RUSTSEC-2026-0403.md)
- [RustSec Tokio 공지](https://github.com/RustSec/advisory-db/blob/main/crates/tokio/RUSTSEC-2026-0386.md)
- crates.io 원본 `mcap` 0.25.0/0.26.0 manifest, RustSec DB `a78282a1414d6c28cc228c4eae3570451affb0dc`.

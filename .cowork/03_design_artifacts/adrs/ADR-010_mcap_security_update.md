# ADR-010 — 압축 의존성 보안 수정에 따른 MCAP 0.26 채택

- 상태: Accepted / 날짜: 2026-10-26
- 승인: Human 「MCAP 0.26.0으로 변경 승인 (권장)」
- 관련: INT-001 / MS-002 / TASK-002 / NFR-004·007
- 관계: ADR-009의 MCAP 0.25 계열 선택만 대체. 다른 기술·계약은 유지.

## ADR 필요성 판단

보안과 의존성 호환·운영 유지보수가 함께 얽힌 변경이다. ADR-009에서 채택한 파일 라이브러리 계열을 바꾸므로 별도 승인·추적을 남긴다.

## 맥락과 결정 요인

TASK-002에서 기존 dot-probe lockfile과 같은 MCAP 0.25.0, zstd 0.13.3, zstd-safe 7.3.0으로 전체 feature를 검사했다. Rust 1.85.0/1.96.0 기능 시험은 통과했으나 RustSec 검사에서 취약점 3건(RUSTSEC-2026-0391·0403·0398)과 zstd-safe 안전성 경고 10건이 남았다. 이는 해당 의존 버전의 공지이며 현재 MCAP 경로에서 모든 영향 함수가 실행됨을 증명한 것은 아니다.

기본 무압축 구성에는 zstd가 포함되지 않지만 승인된 선택 압축 기능을 제공하려면 해당 의존성을 해소해야 한다.

> **2026-11-21 참고(F-09 점검):** 이후 제품 writer는 무압축으로 고정됐고, 압축 chunk의 조회·복구는 범위 밖이다([ADR-013](ADR-013_native_standard_loss_revision_storage.md) §5, 2026-11-15 Human 결정 B1·UC-23). `compression-zstd`는 MCAP 의존성의 zstd 지원만 켠다. 이 ADR의 버전 결정(MCAP 0.26.0)은 그대로 유효하다. Tokio 1.51.0의 RUSTSEC-2026-0386은 같은 계열 수정판 1.51.6으로 해소했다.

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

## 개정 1 — 무압축 chunk CRC 1회 계산 패치(2026-11-11)

- **결정:** Human 「권고대로」(2026-11-11, T5 후속 ②) — MCAP 0.26.0을 `vendor/mcap-0.26.0`에 두고 `[patch.crates-io]`로 쓴다. 최초 패치는 `src/write.rs` `ChunkWriter` 한 곳이다(2026-11-16 reader 결함 조치 보완은 아래 참조). 무압축 chunk에서는 compressed CRC 계층을 만들지 않고 uncompressed CRC를 data section CRC 결합에 재사용한다.
- **근거:** 0.26은 무압축 chunk의 같은 바이트를 CRC로 두 번 훑는다. 복구가 chunk CRC를 필수로 쓰므로 옵션으로 끌 수 없다. 파일 바이트와 모든 CRC 값은 그대로이며, tmpfs P3 무제한에서 +4~6%(raw 4 스트림)·+5%(8 스트림)이다.
- **검증:** 패치본과 원본에서 상류 시험 결과가 같다(lib 130 Pass, 패키지에 없는 fixture 때문에 통합 시험 7개는 양쪽 모두 실패). dot-logger에 chunk·data section CRC 정확성 시험을 추가했다. 상세는 [EV-068](../../05_verification/verification_evidence.md#ev-068).
- **결과·비용:** 위 옵션 표의 「자체 패치·포크」 비용이 생긴다. `cargo audit`은 path 패키지를 검사하지 않으므로 의존성 검토 때 `mcap` 공지를 수동으로 확인한다. crates.io 패키지에 LICENSE 파일이 없어, 공개 준비(T2, 2026-11-21)에서 상류 MIT 전문을 `vendor/mcap-0.26.0/LICENSE`로 보완했다. 상류가 1회 계산으로 바뀌면 패치를 제거한다. vendor 경로·manifest 정리 범위는 `vendor/mcap-0.26.0/VENDORED.md`에 있다.

### 승인된 계약의 결함 조치 보완 — 2026-11-16 / REV-055 RV-136

Human 「권고대로 조치 시작하자」에 따라 vendor reader의 무압축 chunk 경계 초과 패닉을 고치고, 기존 API §21.2의 완전한 chunk 단위 반출 계약을 구현했다. 새 저장 형식·압축 정책 결정은 아니다. `src/sans_io/linear_reader.rs`의 `prevalidate_chunk_crcs`는 CRC가 0이어도 chunk 전체 길이와 내부 레코드 파싱을 선검증한다. 비영(非零) CRC는 기존대로 검증하며, CRC 0 입력의 무결성 보장은 구조 검증까지다. caller 크기 상한은 압축·해제 크기 모두에 적용한다. 기본 streaming reader에도 chunk 내부 경계 검사를 넣어 언더플로와 다음 레코드 침범을 막았다.

상류 패키지에 패치 source를 덮은 lib 시험 **131 Pass**. 제품 Red→Green·두 Rust 회귀 및 한계는 [EV-082](../../05_verification/verification_evidence.md#ev-082), 정확한 vendor 범위는 `VENDORED.md`에 기록한다. native(T3)·장기·실앱 검증은 기존 이월을 유지한다.

## 개정 2 — 배포 가능한 vendor 구조(2026-11-21)

- **문제:** 패치한 MCAP를 워크스페이스 `[patch.crates-io]`로 적용해 왔다. Cargo는 `[patch]`를 최상위 워크스페이스에서만 적용하므로, git 의존성이나 crates.io로 dot-logger를 쓰는 외부 프로젝트는 원본 mcap 0.26.0을 받는다. 워크스페이스 밖 소비자 프로젝트에서 `WriteOptions::chunk_registrations`가 없어 `error[E0599]`로 컴파일이 실패하는 것을 확인했다(2026-11-21). 내부 `tests/api-consumer`는 워크스페이스 안이라 잡지 못했다.
- **결정(2026-11-21 Human 결정(1 A·2 권장대로·3 미리보기·4 TASK-026 뒤·5 권장대로)의 1 A):** vendor 복사본을 이름을 바꾼 워크스페이스 crate(`dot-logger-mcap` 후보)로 두고, dot-logger는 `mcap = { package = "dot-logger-mcap", path = …, version = … }` 별칭으로 의존한다. `[patch.crates-io]`는 없앤다. crates.io 게시 때 이 crate를 먼저 게시한다. MIT 고지와 원본 출처·`local-changes.patch`는 유지한다. 상류(foxglove/mcap)에 변경을 제안하는 일은 선택이며, 반영되면 이 crate를 걷어낸다.
- **유지되는 것:** MCAP 0.26.0 기준, 무압축 고정, 세 가지 로컬 변경(CRC 1회, reader 경계 강화, 선행 등록 옵션), 신뢰할 수 없는 입력 경계(RV-180).
- **구현·검증:** [TASK-036](../../04_implementation/tasks/TASK-036.md). 워크스페이스 밖 소비자 빌드를 CI에 넣어 재발을 막는다.

## 출처

- [RustSec zstd 공지](https://github.com/RustSec/advisory-db/blob/main/crates/zstd/RUSTSEC-2026-0391.md)
- [RustSec zstd-safe 공지](https://github.com/RustSec/advisory-db/blob/main/crates/zstd-safe/RUSTSEC-2026-0403.md)
- [RustSec Tokio 공지](https://github.com/RustSec/advisory-db/blob/main/crates/tokio/RUSTSEC-2026-0386.md)
- crates.io 원본 `mcap` 0.25.0/0.26.0 manifest, RustSec DB `a78282a1414d6c28cc228c4eae3570451affb0dc`.

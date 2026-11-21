# ADR-016 — viewer 내장 웹 운영 대시보드(읽기 전용)

- **상태:** Accepted — 2026-11-21 Human 「A. viewer 내장 웹 (Recommended)」·「읽기 전용 (Recommended)」
- **범위:** INT-001 / MS-009 / TASK-032, FR-006·FR-014·FR-016·FR-017·NFR-006·NFR-008
- **관련:** [ADR-007](ADR-007_embedded_ports_adapters_pipeline.md) · [ADR-009](ADR-009_storage_transport_toolchain.md) · [ADR-014](ADR-014_viewer_live_websocket.md) · [API §15.5·15.8](../interface_contract.md#158-운영-backendstatusapplycompare--2026-11-03) · [화면 §7](../ui_spec.md#7-task-032-운영-대시보드--2026-11-21)

## 맥락

TASK-032는 Foxglove 계열 도구가 보여주지 않는 dot-logger 고유의 운영 관점을 제공한다. 운영 관점은 최근 표본과 10/60/300초 window, 대상별 완료·실패·폐기·Unknown, snapshot 전후 비교, 점검 계획이다. 데이터는 이미 원격 관리 응답으로 나간다: `dot_logger.status.v1`, `dot_logger.compare.v1`, `dot_logger.check.v1`. 비교한 UI 방식은 세 가지다.

| | A. viewer 내장 웹 | B. native GUI | C. Foxglove/Lichtblick 확장 패널 |
|---|---|---|---|
| 의존성 | 새 crate 없음. hyper `server`·`http1`이 tonic을 통해 이미 잠겨 있다 | winit·GPU 백엔드 등 다수 추가, MSRV·audit 부담 | Node/npm 툴체인과 두 번째 언어 |
| 3 OS | 브라우저 | OS별 빌드·디스플레이 라이브러리 | 앱 설치와 PC별 패널 설치, 앱 간 API 차이 |
| 보안 | ADR-014의 loopback·Origin·credential 원칙 재사용 | 프로세스 내부 | 변경 동작에 WS services가 필요(현재 읽기 전용) |
| 시험 | JSON 계약을 Rust 시험으로 고정 | GUI 자동 시험 곤란 | 별도 JS 시험 체계 |

## 결정

1. **방식:** `dot-logger-viewer`가 명시 옵션을 받으면 loopback 전용 HTTP 리스너를 하나 더 연다. 정적 페이지 하나(HTML·CSS·JS)는 바이너리에 내장한다. 빌드 도구와 외부 CDN을 쓰지 않으므로 오프라인 운영 PC에서도 동작한다. 그래프는 페이지 안의 작은 SVG로 그린다. 기존 Foxglove WS 경로와 계약은 바꾸지 않는다.
2. **데이터 경로:** 브라우저는 viewer의 JSON 경로만 호출한다. viewer는 기존 `dot-logger-client` 관리 요청으로 상류에 GetStatus·Compare·PlanCheck를 보낸다. GetStatus 조회는 1초 이상 간격으로 한 번에 하나만 진행하므로 상류 명령 슬롯을 최대 1개 쓴다. 상류 credential은 viewer 프로세스 안에만 있고 브라우저에 주지 않는다.
3. **읽기 전용:** 대시보드는 ApplyConfig·RunCheck를 노출하지 않는다. 변경과 점검 실행은 기존 CLI(`config apply`, `check run`)가 맡는다. viewer에는 Manage credential이 필요 없다. 필요해지면 권한·명시 확인·CSRF 방지를 포함한 별도 결정으로 추가한다.
4. **접근 경계:** HTTP 리스너는 loopback에만 bind한다. ADR-014와 같은 Origin 규칙(loopback 또는 `--allow-origin`으로 명시한 값)을 적용한다. DNS rebinding을 막기 위해 `Host`가 loopback 이름·주소인지 검사한다. 읽기 경로는 GET만 받는다. 요청·응답 크기와 동시 연결 수는 유한하게 둔다.
5. **값 구분:** JSON과 화면은 원래 값 상태(Observed·Validated·Estimated·Stale·Unavailable와 사유)를 그대로 보존한다. 응답에 없는 값을 0이나 정상으로 채우지 않는다. 상류 연결이 끊기거나 권한·시간 초과·Unsupported로 실패하면, 마지막 성공 값을 그 시각과 함께 남기고 실패 사유와 실패 시각을 따로 표시한다. 이 규칙이 TASK-032의 첫 검증 조건이다.

## 결과와 비용

- 운영자는 `dot-logger-viewer` 하나로 Foxglove 연결용 WS와 대시보드 URL을 함께 얻는다. 추가 설치가 없다.
- 화면 구현(HTML·JS)은 저장소 안에서 직접 유지한다. JS는 표시만 하는 얇은 층으로 두고, 상태 판정은 Rust 쪽 JSON 계약 시험으로 검증한다. 브라우저 렌더링 자체의 자동 시험은 이 결정의 범위 밖이며 수동 확인 기록으로 대신한다.
- facade 호스트에서는 GetStatus만 채워진다. Compare는 snapshot이 없어 Gone, PlanCheck는 Unsupported다. 화면은 이 경우를 기능 미제공으로 표시한다.
- 변경 동작이 빠지므로 브라우저 쪽 공격 면과 Manage credential 보관이 생기지 않는다. 대신 설정 변경은 CLI로 해야 한다.

## 구현 기록 — 2026-11-21

제품 `704d055`(TASK-032 S2)는 다음과 같다.

- **HTTP 처리:** hyper 대신 이미 잠긴 `httparse`(hyper가 쓰는 파서)로 GET만 받는 작은 처리부를 두었다. 연결마다 요청 하나를 처리하고 닫는다. keep-alive·chunked 같은 기능이 필요 없어 표면이 더 작다. 외부 crate 수는 그대로(lockfile 패키지 203개)이며 의존 관계 한 줄만 추가됐다.
- **의존성:** viewer는 GetStatus 요청 타입 때문에 workspace의 `dot-logger`를 일반 의존성으로 쓴다(이전에는 개발 의존성).
- **경로·형식:** [화면 §7.1](../ui_spec.md#71-s2-구현--2026-11-21)에 있다. [EV-107](../../05_verification/verification_evidence.md#ev-107)

S3(`277de2d`)에서는 Compare·PlanCheck 조회를 같은 상류 작업자로 보낸다. 상태 조회와 합쳐 동시에 하나만 진행하므로 결정 2의 상한(명령 슬롯 1개)을 유지한다. 대기열은 4개이고, 넘치면 `viewer_busy`다. 형식은 [화면 §7.2](../ui_spec.md#72-s3-구현--2026-11-21)에 있다. [EV-108](../../05_verification/verification_evidence.md#ev-108)

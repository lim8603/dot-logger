# REV-070 — TASK-032 운영 대시보드 독립 코드리뷰

2026-11-21 · Codex · **Conditional / Open** · P1 1건, P2 4건

제품 범위는 `5daf22b..277de2d`의 S2 상태 화면·HTTP(`704d055`)와 S3 Compare·PlanCheck(`277de2d`)다. [ADR-016](../../03_design_artifacts/adrs/ADR-016_viewer_embedded_web_dashboard.md), [화면 명세 §7](../../03_design_artifacts/ui_spec.md#7-task-032-운영-대시보드--2026-11-21), [viewer README](../../../crates/dot-logger-viewer/README.md#operations-dashboard)의 사용 계약과 대조했다.

## 리뷰 항목

| ID | 중요도 | 위치·계약 | 영향 | 처리 제안 | 처리 판정 | 상태 |
|---|---|---|---|---|---|---|
| RV-166 | P1 | dashboard.rs 173·260행, credential 경계 | 잘못 입력한 상류 주소의 민감 정보가 상태 응답에 포함됨 | 시작 시 주소 검증, 표시용 주소 구성 | 대기 | Open |
| RV-167 | P2 | dashboard.js 354~366행, 화면 갱신 | 상태 렌더링 예외 후 폴링 중단 | 렌더링까지 오류 처리, 다음 갱신 보장 | 대기 | Open |
| RV-168 | P2 | dashboard.rs 508행, Host 검사 | HTTP 기본 포트에서 정상 브라우저 요청 거절 | 생략된 포트를 80으로 해석 | 대기 | Open |
| RV-169 | P2 | dashboard.js 241~253행, 조회 결과 | 이전 조회가 최신 선택의 결과를 덮음 | 조회 종류별 최신 요청 식별 | 대기 | Open |
| RV-170 | P2 | dashboard.rs 293~300·318~322행, 조회 주기 | 재연결 직후 GetStatus 간격이 1초 미만일 수 있음 | 실제 RPC 시작을 기준으로 다음 조회 예약 | 대기 | Open |

## 리뷰어 상세 및 수락 기준

1. **RV-166.** `run`은 CLI의 `--upstream` 원문으로 설정을 만든다. `serve`는 이를 즉시 `Status`에 복사하고 JSON의 `upstream`으로 내보낸다. 별도 작업자의 `RemoteClient::connect`가 userinfo·query를 거절해도 이미 복사한 원문은 계속 표시된다. 운영자가 인증 정보가 포함된 URL을 붙여넣으면 연결 실패 중에도 브라우저에 전달된다. 환경변수 토큰은 별도 필드로 처리된다. 리스너 공개 전에 기존 주소 검증을 공유하고, 화면에는 검증된 scheme·host·port만 직렬화한다. 잘못된 주소를 거절할 때 오류 메시지와 상태 문서에 원문이 남지 않는지 확인한다.
2. **RV-167.** Rust의 `received`는 상태 보고서의 schema 이름까지만 확인한다. JS는 응답을 즉시 `latest`로 저장한 뒤 `try` 밖에서 렌더링한다. 호스트와의 형식 불일치로 snapshots 또는 windows가 예상 타입과 다르면 렌더링이 예외로 끝나고 다음 타이머를 등록하지 못한다. 이전 성공 값도 이미 교체되어 있다. 소비하는 필드의 타입을 확인하고, 마지막 유효 상태와 오류를 분리하며 다음 갱신을 보장한다. 정상 → 형식 오류 → 정상 순서에서 오류 표시와 자동 복구를 확인한다.
3. **RV-168.** `loopback_host`는 port 구분자가 필수다. 운영자가 허용된 고정 주소의 80번 포트를 선택하면 브라우저는 HTTP 기본 포트를 생략할 수 있어 첫 페이지부터 400을 받는다. authority로 해석하고 포트 생략 시 80을 적용한 뒤 loopback 및 실제 리스너 포트를 검사한다. localhost·IPv4·괄호 IPv6의 기본 포트 성공과 비기본 포트 불일치 거절을 함께 확인한다.
4. **RV-169.** `lookup`은 Compare/Preview를 누를 때마다 요청하고, 완료되는 모든 요청이 같은 결과 영역을 갱신한다. 앞선 계획 조회가 대기 중에 새 계획 조회가 먼저 실패하면, 늦게 도착한 앞선 성공 결과가 최신 실패 표시를 지운다. 같은 종류의 최신 요청만 결과를 표시하게 하고, 순서를 뒤집은 두 응답으로 최종 계획 ID와 실패 표시를 확인한다.
5. **RV-170.** 타이머는 연결을 시작하기 전에 tick을 소비한다. 첫 연결이나 재연결에 1초 이상 걸린 뒤 GetStatus가 빠르게 끝나면 다음 tick은 이미 도래해 두 RPC가 연달아 전송될 수 있다. 작업자 직렬화는 동시 호출 수를 제한하지만 전송 시각 간격은 보장하지 않는다. 연결 뒤 실제 GetStatus 시작 시각으로 기한을 잡고, 연결 지연 뒤 호스트에서 관측한 두 시작 간격이 1초 이상인지 검증한다.

## 검토와 검증

- HTTP는 loopback 리스너, Host 단일 값, Origin 정책, GET 경로, 본문 선언 거절, 8 KiB 헤더·읽기 기한, write·drain 기한, 연결 8개, CSP와 no-store를 확인했다. Compare·PlanCheck는 정확한 10진 u64 인자만 허용하며 변경 명령 경로는 없다.
- 작업자 하나가 상태 및 조회 요청을 직렬 처리한다. 조회 대기열은 4개, HTTP 조회 대기는 설정 기한의 6배다. Gone·Unsupported는 연결을 유지한다. 정상 stop에서는 연결 작업을 정리한 뒤 worker를 abort한다. CLI 종료는 dashboard 태스크를 abort하고 프로세스 runtime 종료로 남은 태스크가 정리된다.
- Rust 상태 캐시는 실패 시 마지막 성공 값과 수신 시각을 보존한다. 조회 실패 응답에는 일부 보고서가 없다. 화면은 DOM 텍스트로 데이터를 표시하며 변화량은 문자열 분자·분모에서 BigInt로 환산한다. 같은 snapshot ID 목록에서는 선택 상자를 유지한다.
- 단일 슬롯 시험은 첫 GetStatus를 보류하고 300 ms 후 동시 요청 유무를 확인한다. 조회가 실제로 대기열에 들어간 시점을 동기화하면 부하가 큰 환경에서도 해당 조건을 검증할 수 있다. operations_host는 600초 루프 뒤 서버 종료를 알리고 대기한다. live_host는 상태 조회용 telemetry를 활성화한다.
- Rust 1.96.0에서 `cargo fmt --all --check`, `cargo clippy --locked -p dot-logger-viewer --all-targets -- -D warnings`, `cargo test --locked -p dot-logger-viewer --all-targets` 통과. Rust 1.85.0에서도 같은 viewer 시험을 수행했다. 두 버전 각각 16건 통과(대시보드 8건 포함), 예제 두 개 빌드. 브라우저 화면과 전체 워크스페이스의 이전 검증은 [EV-107](../verification_evidence.md#ev-107)·[EV-108](../verification_evidence.md#ev-108)에 있다. 이번 지적은 코드 경로 대조 결과다.

## 처리담당 기록

RV-166~170의 처리 판정·변경 함수·검증 근거는 후속 개발 세션에서 항목별로 기록한다. TASK-032 수락은 조치 검증 후 판단한다.

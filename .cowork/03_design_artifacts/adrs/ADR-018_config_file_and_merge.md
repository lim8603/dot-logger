# ADR-018 — 코드와 옵션 파일이 같은 설정 타입으로 모인다: TOML·병합표·출력 전체 교체

- **상태:** Accepted. 2026-11-28 Human 「모두 권고대로 승인」(MS-013 §2.1 C7, §2.2 P4·P5, §2.3 H7, §9.2 Q5·Q8). 같은 날 [REV-083](../../05_verification/reviews/REV-083_MS013_R10_DESIGN_REVIEW.md) Pass.
- **범위:** INT-001 / [MS-013](../../04_implementation/milestones/MS-013.md) 단계 3([TASK-051](../../04_implementation/tasks/TASK-051.md)). 요구: FR-020.
- **관련:** [ADR-003](ADR-003_configurable_policies_and_defaults.md)·[ADR-009](ADR-009_storage_transport_toolchain.md)(의존성 기준)·[ADR-012](ADR-012_facade_owned_writer_handles.md)·[ADR-017](ADR-017_output_model_copy_observer.md). 설계 본문은 MS-013 §5.5다.

## 맥락

R1은 재빌드 없이 시작할 때 옵션 파일로 출력 조합을 바꾸기를 요구한다. C7(2026-11-27 Human)은 코드 지정과 파일 지정을 **둘 다** 지원하고 비밀 값은 경로·환경 변수로만 참조하도록 정했다. 지금 facade는 `LoggerBuilder` 코드만 받는다. 채널의 출력은 파일 선언 순서 index(`outputs(&[usize])`)로 정한다. 파일이 출력을 바꾸면 숫자 index가 조용히 다른 파일을 가리킬 수 있다(REV-077 RV-241).

## 결정

1. **하나의 설정 타입:** `LoggerConfig`는 feature 없이 제공한다. 담는 항목은 budget, 출력(이름·종류·종류별 설정), 채널 정책(토픽별 출력 이름·`minimum`·`important_from`), writer quota, 스위치 초기 상태다. 코드는 `LoggerBuilder::config(LoggerConfig)`로 넣는다. 같은 내용이면 해석된 구성과 `Plan`이 같다. 값의 출처는 `LoggerBuilder::explain() -> PlanSources`로 본다(공개 `Plan` 필드는 불변).
2. **파일 형식:** TOML, `version = 1`, 모르는 키 거절(`deny_unknown_fields`). 읽는 중 1 MiB 상한(`Read::take`). 선택 feature `config-file`의 `from_toml_path`·`from_toml_str(text, base_dir)`로만 읽는다. 상대 경로는 설정 파일 디렉터리 기준이며, base가 없으면 거절한다.
3. **병합표(H7):** budget은 필드별, preset은 비어 있는 손실 값만, 채널 정책은 토픽·필드별, writer quota는 이름별로 뒤 호출이 덮는다. **출력은 목록 전체 단위다.** `config()`에 출력이 하나라도 있으면 그때까지 선언된 출력을 **모두 교체**하고, 없으면 유지한다. `config()` 뒤의 코드 출력 선언은 목록에 추가되고, 같은 이름이 두 번 나오면 `Duplicate`다.
4. **숫자 index와 교체(Q8, RV-241):** 출력 교체가 일어난 구성에서 숫자 index를 쓴 채널이 있으면 `start()`·`plan()`이 `InvalidOutputs`로 거절한다. 그 채널은 이름(`outputs_named`·파일 채널 정책)으로 지정해야 한다. 교체가 없으면 숫자 index 의미는 지금 그대로다. 이름 없는 코드 출력의 자동 이름(`file#<N>`)은 참조할 수 없고, 사용자 이름에도 쓸 수 없다.
5. **비밀과 오류 노출:** 비밀 값을 직접 쓰면 `SecretValue`로 거절한다(경로·환경 변수 참조만 허용). `ConfigFileError { kind, line, column }`의 `Display`·`Debug`는 kind와 위치만 보여 주고, 원문과 값은 노출하지 않는다. `source()`는 `None`이다. 오류 우선순위는 파싱 → 스키마 → 병합 → `start()` 검증이다.
6. **범위 밖:** 실행 중 다시 읽기(hot reload)는 하지 않는다. 실행 중 변경은 레벨·샘플링과 전송 스위치(ADR-019)뿐이다.

## 결과와 비용

- 파일 스키마가 공개 계약이 된다(K4). 키는 필요한 것만 두고 `version`으로 바꿔 간다.
- 새 의존성은 `toml`(feature `config-file` 전용)이다. 단계 3 S0에서 MSRV 1.85·`cargo audit`·라이선스를 확인한 뒤 [tech_stack](../tech_stack.md)에 올린다. feature 조합 4종을 CI에서 확인하고, 해당 feature가 없을 때 그 출력 종류는 `Unsupported`다.
- 같은 바이너리가 파일만 바꿔 파일만 / 네트워크만 / 혼합으로 시작할 수 있다. 이것이 단계 3의 수락 시험이다.

## 검증

[TASK-051](../../04_implementation/tasks/TASK-051.md)과 [TC-015](../../05_verification/test_case.md#tc-015--출력-구성네트워크-전용전송-스위치) 3단계에서 확인한다. 항목은 병합표 경우별, 같은 바이너리 + 파일 3개, 교체 + 숫자 index 거절, 자동 이름 참조 거절, 오류 노출, 읽기 상한, feature 조합이다.

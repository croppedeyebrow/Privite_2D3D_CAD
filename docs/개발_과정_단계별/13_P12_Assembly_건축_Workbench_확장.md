# P12 — Assembly·건축 Workbench 확장

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 파트 참조/간섭, 건축 벽·개구부·IFC 조사
> 원문 추적: v1 03

---

## P10~P12 — 후속(조건부)

- **P10:** model-linked orthographic drawing, hidden-line removal, sheets, PDF, dimensions, DXF adapter 선택.
- **P11:** fillet/chamfer, edge remapping, pattern/mirror, PBR materials/lights, PNG, glTF.
- **P12:** assembly reference model, interference, 3D PMI, architectural workbench with own data semantics/IFC.

## 단계 목표

파트 참조/간섭, 건축 벽·개구부·IFC 조사. 이 단계는 이후 작업의 선행 계약이며 완료 여부는 체크박스가 아니라 **실제 테스트 증거**로 판정합니다.

## 선행 조건·범위

- 대상: Assembly·건축 Workbench 확장
- 선행 관계: `개발_과정_단계별/00_개발_로드맵_마일스톤.md`의 DAG를 기준으로 하며 의존성 미충족이면 착수하지 않습니다.
- 위험/승인: 별도 프로젝트 수준, R4 게이트

## 세부 실행 태스크

- [ ] 기존 문서 요구사항 ID 및 영향받는 crate/API를 추적하고 인터페이스 변경을 명시합니다.
- [ ] 정상 흐름·입력 오류·중단/취소·파일/커널/렌더 자원 실패를 테스트 케이스로 작성합니다.
- [ ] 핵심 알고리즘/형상/포맷 특성에 대한 fixture와 테스트 오라클을 정의합니다.
- [ ] UI와 CLI 공통으로 호출할 Application Service 및 Result 오류 코드를 정의합니다.
- [ ] Rust 단위·통합·회귀 테스트, UI 동작 증거, 로그/메트릭을 확보합니다.
- [ ] 이전 파일 포맷·Undo/Redo·모델 revision·캐시 무효화 영향을 기록합니다.
- [ ] 라이브러리 추가·native ABI 변화 시 SBOM/라이선스/배포 사항을 검토합니다.

## 인수·완료 게이트

- 산출물: **별도 ADR/스코프, pilot documents**
- 테스트 실행 및 오류 로그를 `검증_산출물/`에 보관합니다.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`를 실행하고 네이티브 플랫폼 조건을 기록합니다.
- 미완료 기능은 릴리즈 노트에 명시하며 구현된 것처럼 표시하지 않습니다.

## 단계에서 학습할 개념

- Rust: 모듈/trait/Result/ownership, 비동기 worker, 테스트 설계.
- CAD: 기하·구속·B-Rep·파일 포맷 중 해당 주제의 오류 경계를 설명할 수 있어야 합니다.
- 운영: 재현 가능한 빌드와 rollback 경로를 문서로 제시합니다.

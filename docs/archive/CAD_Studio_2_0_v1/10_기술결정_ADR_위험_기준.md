# 10. Architecture Decision Records(ADR) & Engineering Risk Gates

상태 기준: `PROPOSED`(기획 제안), `SPIKE`(기술실험 필요), `ACCEPTED`(검증 후 결정), `REJECTED`(대안 검토 후 기각). 현재 아래 항목은 전부 **PROPOSED 또는 SPIKE**입니다.

## ADR-001: B-Rep을 원본으로, Mesh를 캐시로

- **문제:** 3D 시각화 mesh와 정밀 솔리드 편집 구조가 상이함.
- **결정 제안:** Parametric Feature+Sketch가 source of truth, kernel B-Rep precise shape, tessellated mesh은 non-authoritative.
- **대안:** mesh-native modeling only(정밀 기계 CAD 설계 목적과 불일치).
- **검증:** 구멍 있는 부품 boolean, volume, STEP export, tessellation 품질, 재생성.
- **상태:** PROPOSED.

## ADR-002: OCCT C++ vs Truck pure Rust

| 평가 축 | OCCT | Truck | PoC 점수(미측정) |
|---|---|---|---|
| 정밀 B-Rep / STEP | 성숙한 지원 문서·도구 | Rust CAD B-Rep 지향 | 실제 fixture 테스트 필요 |
| Boolean/Fillet | 광범위 연산 제공 | 버전별 기능·경계 테스트 필요 | 미측정 |
| Windows 배포/빌드 | CMake/MSVC C++ bridge 비용 | Rust 중심 빌드 가능성 | 미측정 |
| Kernel crash 격리 | C++ 예외/포인터/프로세스 전략 필요 | Rust라도 logic panic/OOM 가능 | 미측정 |
| License | OCCT LGPL-2.1 + exception | Truck Apache-2.0 | 법무/notice 확인 필요 |
| Topology naming | OCAF/TNaming 역사 기반 기법 존재 | custom strategy 검증 필요 | 미측정 |

**가설:** 실제 범용 기계 단일 파트 구현이 목표라면 OCCT 우선 PoC. **자동 도입 확정이 아님.** Truck가 필요 조건을 충족하고 유지비가 현저히 낮으면 대안 채택. Vendor adapter 미리 구축하되 지나친 abstraction보다 필요한 feature 최소 집합 추구.

**필수 통과:** (1) C++/Rust ownership-safe interop (2) 판재+구멍 Boolean (3) STEP (4) fillet/edge mapping P1 가능성 (5) invalid input/crash handling (6) license packaging.

## ADR-003: Rust unsafe 정책 예외

- **기존:** 모든 영역 unsafe 금지.
- **변경 제안:** `#![forbid(unsafe_code)]`를 core/feature/sketch/command/IO 등의 순수 Rust crates에는 유지. FFI-wrapper는 최소 범위 허용 및 엄격한 code review/testing. C++ 종속 코드는 독립 `native`와 적절한 C++/Rust bridge에서 격리.
- **이유:** C++ OCCT를 Rust에서 실사용하려면 기존 workspace 절대 unsafe 금지는 조정이 필요함.
- **상태:** PROPOSED, architecture 승인 항목.

## ADR-004: Sketch Solver

- **가정:** 수치 solver를 직접 구현하면 기하 구속 진단/수렴/과잉구속 해결이 길어짐.
- **비교:** 기성 solver(libslvs 등) vs 소형 Rust solver 연구, constraint 종류 축소.
- **라이선스 주의:** SolveSpace 라이브러리 공식 설명은 GPLv3/일부 라이선스 적용 제한을 지적. 제품 배포 방식과 호환 여부를 도입 전 검토.
- **PoC:** 8~10 constraint, fully/over/under 상태, 500 shape 반복, Windows build, error telemetry.
- **상태:** SPIKE.

## ADR-005: GUI egui + wgpu

- **결정 제안:** eframe/egui widget shell + `egui_wgpu` graphics integration and native wgpu 3D viewport. 기존 GUI 구조를 유지하고 scene renderer만 교체/확장.
- **검증:** wgpu/egui version compatibility, 3D depth/picking, DPI, device loss, integrated GPU performance.
- **대안:** OCCT viewer를 native HWND/Qt 등에 탑재(통합 난이도/배포 비용 비교). 이 방식은 MVP가 아니라 tradeoff contingency.
- **상태:** PROPOSED.

## ADR-006: Native Save vs STEP

- **결정 제안:** versioned own `.cadstudio` 프로젝트 포맷은 Feature+Sketch+History+Units+References 보존, STEP은 B-Rep interchange, STL/glTF은 mesh.
- **근거:** STEP을 열 수 있다는 사실은 원본 parametric design history가 재구성된다는 뜻이 아님.
- **검증:** version migration, STEP roundtrip, broken cache rebuild, crash durability.
- **상태:** PROPOSED.

## ADR-007: 2D Drafting / Sketch Workbench 분리

- **결정 제안:** Drafting geometry(주석·시트·레이어)와 Sketch geometry(plane·constraints·feature link) 역할 구분.
- **이유:** 2D 벡터 그림과 3D parametrically driven sketch를 구별하지 않으면 의미·변경 규칙·undo 책임이 꼬임.
- **상태:** PROPOSED.

## ADR-008: 병렬성/내장 GPU 대응

- **결정 제안:** UI event loop, CPU model worker, GPU render thread/pipeline의 책임 분리. OCCT thread-safety가 검증되기 전 모델 worker에서 **native kernel 단일 실행 직렬화**.
- **예외:** read-only derived mesh/cache는 thread-safe proof가 있을 때 병렬화.
- **게이트:** UI responsiveness, kernel races, memory pressure, stale rendering selection.
- **상태:** PROPOSED.

## 현재 가장 높은 우선순위 기술 위험 Top 8

| 우선순위 | 위험 | 검증할 것 | 실패 시 대안 |
|---|---|---|---|
| 1 | 커널/FFI/라이선스 | Windows build/STEP/bool/packaging | Truck PoC 또는 R2 범위 축소 |
| 2 | 스케치 solver 품질/라이선스 | constraint DOF, overconstraint, native ABI | solver 교체 / 지원 constraint 축소 |
| 3 | Topological naming | upstream edit references unchanged or flagged | face-selection follow-up을 P1로 격리 |
| 4 | Feature regeneration/Undo | failed operations transactional | last-valid snapshot/rollback 방식 단순화 |
| 5 | 저장 복구 | NTFS file lock/crash during overwrite | safe copy/backup + manual recovery |
| 6 | GPU 통합 및 메모리 | egui+wGPU adapter/perf integrated GPU | 낮은 품질 preset, cached redraw |
| 7 | STEP·DXF 지원 오해 | paid vs free modules / mapping capability | 독립 라이브러리/adapter, DXF 후순위 |
| 8 | 단일 개발자 구현 폭주 | minimal usable part workflow | R2 고정, R3+ 동결 |

## 소스 근거(공식/프로젝트 문서)

1. OCCT 개요 및 STEP/B-Rep 기능: https://occt3d.com/dev/doc/overview/html/index.html
2. OCCT LGPL exception: https://github.com/Open-Cascade-SAS/OCCT/blob/master/README.md
3. OCCT naming docs: https://www.occt3d.com/dev/doc/overview/html/occt_user_guides__ocaf.html
4. Rust Truck repository: https://github.com/ricosjp/truck
5. wgpu API: https://wgpu.rs/doc/wgpu/
6. egui_wgpu callback/integration: https://docs.rs/egui-wgpu/latest/egui_wgpu/
7. cxx bridge/ownership: https://cxx.rs/
8. SolveSpace solver licensing note: https://solvespace.github.io/solvespace-web/library.html
9. glTF 2.0 metallic-roughness: https://github.com/KhronosGroup/glTF/blob/main/specification/2.0/Specification.adoc

> 기술 문서는 2026-10-25 기준 참고. 최신 API/버전/라이선스는 구현·배포 직전에 고정 버전으로 재확인해야 합니다. 이 문서는 법률적 라이선스 판정서가 아닙니다.

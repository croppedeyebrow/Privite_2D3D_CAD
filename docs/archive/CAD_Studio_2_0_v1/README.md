# CAD Studio 2.0 — 통합 상세 개발 명세 v1.0

- 기준일: 2026-10-25
- 상태: **설계 초안 / 기술 PoC 검증 전**. 구현 완료 또는 라이브러리 호환성 검증 완료를 뜻하지 않습니다.
- 목표: **2D 드래프팅 + 2D 구속 스케치 + 3D 파라메트릭 솔리드 + 실시간 3D 뷰포트 + 출력**이 하나의 문서 모델을 공유하는 **오프라인 우선 Windows CAD**.
- 원본 5개 파일의 기획 의도(로컬·Rust·Command·Undo/Redo·검증·복구·CLI)는 유지하고, 3D 생산성을 위해 데이터 모델 및 커널 경계를 확장했습니다. 원본 첨부파일은 변경하지 않았습니다.

## 문서 탐색

| 문서 | 무엇을 결정하는가 |
|---|---|
| [00_프로젝트_기획서.md](00_프로젝트_기획서.md) | 제품 비전, 대상 업무, 단계별 범위, 비기능 요구사항 |
| [01_개발_목적_및_세부_기획.md](01_개발_목적_및_세부_기획.md) | 사용자 업무 시나리오 및 FR 요구사항과 수용 기준 |
| [02_아키텍처_시스템_언어_구조.md](02_아키텍처_시스템_언어_구조.md) | 모듈, 데이터 흐름, 책임·의존성, 스레드/프로세스 모델 |
| [03_개발_테스크_분할.md](03_개발_테스크_분할.md) | 선후행 관계와 게이트가 있는 개발 WBS |
| [04_UI_구성_기획.md](04_UI_구성_기획.md) | 도면/모델/렌더 워크벤치의 UI·입력 상태·오류 UX |
| [05_2D스케치_및_기하학_명세.md](05_2D스케치_및_기하학_명세.md) | 스케치 좌표, 스냅·구속조건·치수·도면 연결 |
| [06_3D커널_파라메트릭_모델링_명세.md](06_3D커널_파라메트릭_모델링_명세.md) | B-Rep, Feature DAG, Boolean, 형상 참조 및 재생성 |
| [07_3D뷰포트_GPU렌더링_명세.md](07_3D뷰포트_GPU렌더링_명세.md) | 테셀레이션, 카메라, Picking, PBR, GPU 자원 수명 |
| [08_저장_호환성_복구_명세.md](08_저장_호환성_복구_명세.md) | native format, autosave, STEP/DXF/STL/glTF, migration |
| [09_테스트_성능_보안_배포_명세.md](09_테스트_성능_보안_배포_명세.md) | 테스트 계층, 벤치마크, 크래시/보안/배포 검증 |
| [10_기술결정_ADR_위험_기준.md](10_기술결정_ADR_위험_기준.md) | OCCT vs Truck, FFI, 라이선스, 미확정 기술 검증 |

## 핵심 수용 시나리오 — R2 종료 게이트

1. 사용자가 XY 스케치 평면에 **중심 위치가 치수로 고정된 사각형 + 원형 구멍** 프로파일을 만듭니다.
2. 모든 자유도를 구속하고(fully constrained), 치수를 수정하면 스케치가 예상대로 재계산됩니다.
3. 닫힌 프로파일을 **Extrude**하여 3D 부품을 만듭니다. 재료를 제거하는 Hole/Cut을 적용합니다.
4. 부품을 회전·확대·선택하고, 면/모서리 속성을 보고, 질량이 아닌 **부피/바운딩 박스**를 검사합니다.
5. 파일을 저장·닫기·재실행·복원하여 설계 의도(스케치/구속조건/피처 이력)가 유지됩니다.
6. STEP으로 내보내어 독립 CAD 도구에서 열고, 기본 기하/부피/바운딩 박스를 비교합니다.
7. STL 메시로 출력하고 glTF로 시각화 메시/재질을 출력합니다. 메시 출력과 원본 B-Rep의 의미 차이를 명시합니다.
8. 모델 피처 하나가 실패하면 기존 마지막 성공 형상과 원인을 보여주고 사용자의 파일을 손상시키지 않습니다.

## 판정 원칙

- **정밀 편집 가능한 설계 원본:** `Sketch + Constraints + Feature History + B-Rep`.
- **렌더 전용 데이터:** 삼각형 메시, 노멀, 인덱스, GPU 버퍼는 언제든 재생성 가능한 캐시입니다.
- **CAD 설계 정확도 ≠ 화면 시각 품질:** 별도 테스트, 별도 실패 복구, 별도 파일 호환 범위를 둡니다.
- **예정 기능은 구현으로 간주하지 않음:** 각 기능에는 P0/P1/P2 구분과 테스트 증거가 필요합니다.

## 외부 공식 문서(기술 평가의 출발점)

- Open CASCADE Technology: https://occt3d.com/dev/doc/overview/html/index.html
- OCCT Topological Naming: https://www.occt3d.com/dev/doc/overview/html/occt_user_guides__ocaf.html
- OCCT source/license: https://github.com/Open-Cascade-SAS/OCCT
- Truck CAD kernel: https://github.com/ricosjp/truck
- wgpu: https://wgpu.rs/doc/wgpu/
- egui_wgpu: https://docs.rs/egui-wgpu/latest/egui_wgpu/
- cxx bridge: https://cxx.rs/
- glTF 2.0 spec: https://github.com/KhronosGroup/glTF/blob/main/specification/2.0/Specification.adoc
- SolveSpace constraint library/licensing: https://solvespace.github.io/solvespace-web/library.html

> 위 링크는 도입 확정 또는 기능 지원 검증을 의미하지 않습니다. 배포 시 버전·라이선스·종속성 고정을 별도로 수행합니다.

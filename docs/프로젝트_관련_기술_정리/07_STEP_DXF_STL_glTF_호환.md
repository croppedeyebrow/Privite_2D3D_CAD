# 외부 CAD 교환의 손실과 정확도

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 실제 CAD 구현 시 알아야 할 기술 원리·역할·실패 요건

---

## 4. 외부 교환 형식 매트릭스

| 포맷 | P0/P1/P2 | 입력/출력 목적 | 보존·손실 주의 |
|---|---|---|---|
| SVG | P0 out | 2D 도형/뷰 출력 | CAD 피처·구속 정보 손실 |
| PDF | P1 out | 인쇄/도면 배포 | 구조적 편집 정보 제한, 폰트 확인 |
| DXF | P1 in/out | 2D 도면 교환 | 도형 subset, hatch/spline/block/text mapping 별도 |
| STEP (AP203/214/242 subset) | P0 in/out | **정밀 3D B-Rep** 전달 | 원본 피처 tree/sketch·구속 복원 보장 안 됨 |
| STL | P0 out | 3D 인쇄/mesh workflow | 삼각형 메시; 단위/재질/설계 파라미터 보존 안 됨 |
| glTF 2.0/GLB | P1 out | PBR 렌더용 장면·시각화 | 정밀 B-Rep·피처 tree 보존 안 됨 |
| IGES | P2 | 일부 표면/곡면 교환 | 면 연결·오류·참조 손실 경고 |
| DWG | P2 research | 업계 2D 교환 | 기본 제공 확정 금지, SDK 라이선스/호환 이슈 |
| IFC | P2 separate | 의미적 BIM/건축 교환 | 단순 3D mesh를 IFC BIM으로 간주 금지 |

**OCCT 기본 오픈소스 지원과 유료/별도 SDK를 혼동하지 않습니다.** 특히 DXF는 선택한 라이브러리/SDK의 실제 지원·라이선스를 확인한 뒤 구현합니다. STEP AP242의 확장 PMI를 전체 지원한다고 주장하지 않습니다.

## 5. Import/Export Adapter 계약

- Import preflight: file type signature, max size, archive bomb detection, max counts, parser timeout/limits, memory/time budget, path normalization.
- Parse -> units/coordinate conversion -> shape/curve validity -> mapping report -> staged document -> commit.
- Imported foreign 3D solids는 editable parametric feature tree가 아니라 **ImportedBody + direct transform/Boolean subset**으로 관리할 수 있으며, sketch/history reconstitution은 P2 별도 역설계 기능.
- STEP export: topology validity, units, tolerance semantics, assembly vs single part constraints check.
- STL export: tessellation angle/deflection, watertight/normal/orientation, mm conversion/unit note.
- glTF export: `meters` 기반 coordinate/scene unit conversion, coordinate convention and node transforms, normals/PBR material mapping.
- DXF: 지원 R-code/entities/version list + unsupported entity loss report; unknown record pass-through은 안전성 검증 후만 허용.
- Export report: output path, format/version, source revision, object counts, warnings, SHA-256/checksum.

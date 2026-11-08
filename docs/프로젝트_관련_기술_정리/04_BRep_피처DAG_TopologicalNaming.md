# 피처 재계산과 형상참조

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 실제 CAD 구현 시 알아야 할 기술 원리·역할·실패 요건

---

## 1. 형상 모델의 정밀도 계층

1. **Parametric intent**: Sketch entities, constraint graph, feature parameters and references.
2. **Topological B-Rep**: Vertex / Edge / Wire / Face / Shell / Solid / Compound + underlying analytic / spline curves and surfaces.
3. **Tessellation**: Rebuildable triangle mesh with per-face/edge association; visualization/export cache only.
4. **Scene presentation**: camera, material, light, selection outlines; 변경되어도 1~2번 손상 금지.

삼각형 mesh만으로 설계 원본을 대체하지 않습니다. STL/glTF export는 3~4 계층 일부를 전달하며 원본 Feature Tree 보존을 보장하지 않습니다. STEP import도 일반적으로 원본 Sketch/Feature History 복원을 보장하지 않습니다.

## 3. Feature DAG

```text
Parameters(width=100,height=60,thickness=10,hole_r=5)
   └ Sketch_Base [plane: XY]
      ├ Rectangle constraints
      └ Circle center/radius constraints
          ↓
     Extrude_Base 10 mm ──────────┐
                                  ├ Cut_ThroughHole ──> Body_001
     Sketch_Hole [face support] ─┘
                               ↓
                Fillet_Edges(P1, topo ref)
```

- Feature kinds R2: CreateSketch, Extrude, Revolve, Boolean Fuse/Cut/Common, SetParameter, Suppress/Pause(가능 시).
- Feature kinds R3: Fillet, Chamfer, Mirror, Pattern, Shell, Loft/Sweep(추가 검증 후).
- Input graph DAG must be acyclic; explicit dependencies, `dirty propagation`, topo order recompute.
- 단계당 geometry kernel operation result에 `validity`, `operation history`, diagnostics, cache key.
- upstream parameter 변경으로 재생성 실패 시 **transaction rollback** 또는 명시된 `last-valid` 상태 유지; 재계산 실패한 결과를 정상 파일의 최신 shape로 조용히 덮지 않음.

## 4. Topological Naming 문제 — 실사용 CAD의 핵심 위험

B-Rep Boolean/Fillet 후 TopoDS Face/Edge 순서가 바뀌거나 분할·소멸될 수 있습니다. `face_index=7` 같은 참조는 후속 모델 수정을 견디지 못합니다.

**최소 정책:**

- `SemanticFeatureRef` = original sketch/profile/feature ID + lineage/selection context.
- `SubshapeRef` = producing feature + topological operation history (generated/modified/deleted) + geometric signature (surface type, normal, centroid, adjacency, area/bounds) + ambiguity status.
- 단일 후보일 때만 deterministic rebind; 복수 후보 또는 누락이면 `AmbiguousReference`/`LostReference`로 사용자 지정 재선택 요청.
- 커널의 history map(지원되는 경우)을 우선 사용, 면의 좌표 서명은 추가 검증 신호이지 완전한 식별자가 아님.
- 위험한 자동 치유/임의 대체는 **제조 설계 오류**를 만들 수 있으므로 금지.

**실험:** Extrude Base 변경·Cut 신규 추가·Fillet 실패·Feature Suppress/Unsuppress·Sketch plane rotate에서 대상 face 참조 유지/실패 동작을 정량 기록.

## 5. B-Rep 연산별 입력/출력 계약

| 연산 | 전제 조건 | 성공 결과 | 대표 실패 |
|---|---|---|---|
| MakeFace | closed valid wire/holes | orientable face | self intersection, zero area |
| Extrude | valid profile, finite nonzero length | solid 또는 명시 shell | wrong direction, unsupported multiple regions |
| Revolve | valid profile + axis | solid | degeneracy on axis, self-overlap |
| Boolean | valid operands, contact/overlap semantics | valid solid or valid empty set | tangency, tolerance healing fail |
| Fillet(P1) | valid target edges + radius | valid edge transitions | edge disappears, radius too large |
| Chamfer(P1) | valid reference edges | valid beveled body | non-manifold/tiny edge |
| Tessellate | valid shape + quality/units | triangles+normals+subshape ids | degenerate triangles, OOM |

공통 검사: `isValid`, closed/manifold/solid count, finite BBox, nonnegative and plausible volume, topology errors, operation logs, exact input references, elapsed time. 양/음 볼륨은 orientation 및 참조 모델 기준 검토. 음수 부피를 일괄 정답 판정 금지.## Feature 재계산 규약

```text
사용자 치수 편집 → Typed Command → staging Document
→ DAG affected descendants 계산 → solver 검사
→ dedicated kernel worker에서 재계산 → B-Rep validity
→ topology reference reconciliation → commit OR rollback
→ cache invalidation → tessellation → GPU snapshot publish
```

**실패 규칙:** 순환 참조·해결 불가능한 구속·커널 연산 실패·면 참조 모호성·피처 유효성 실패는 기존 Document revision을 유지하고 오류 객체와 원인을 반환합니다. 커널의 스레드 안전성이 입증되기 전에는 모델 워커를 직렬화합니다.

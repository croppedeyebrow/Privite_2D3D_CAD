# wgpu 그래픽스, 메시 및 CAD picking

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: wgpu 그래픽스, 메시 및 CAD picking
> 원문 추적: 7

---

## 1. CAD Renderer와 PBR Renderer는 같은 RenderScene을 사용하되 목표가 다름

- **CAD work viewport (R2/P0):** 기하 형태 읽기·edge outline·높은 입력 반응성·정확한 pick/selection·측정. Shaded, edges-on, wireframe, orthographic/perspective, orientation cube.
- **Visualization preview (R3/P1):** appearance-focused PBR shading, multiple lights/environment, shadows(성능 검증), tone mapping, supersampling, PNG snapshot, glTF export.
- **Offline photorealistic path tracing:** 초기 범위 제외; GPU/메모리/노이즈/샘플 비용이 별도 제품 규모.

`cad_render`는 **CPU-side scene description**만, `cad_viewport_wgpu`는 GPU 장치·pipeline·shader·buffer lifetime만 책임집니다.

## 2. 데이터 파이프라인

```text
Document revision r / current valid B-Rep shapes
  ├─→ Topology-aware tessellation {deflection_mm, angle_rad, normal_mode}
  │     ├─ Vertices/Indices/Normals/Tangents(Optional)/UV(Optional)
  │     ├─ FaceId -> triangle ranges  (for face picking)
  │     ├─ EdgeId -> line ranges      (for edge picking)
  │     └─ ObjectId/FeatureId provenance
  ├─→ RenderScene snapshot {scene_version=r, transforms, materials, visibility}
  │     └─→ GPU scene cache / buffer upload  → wgpu render passes → window surface
  └─→ Selection: GPU ID-pass or CPU ray-acceleration → Kernel/Topology revalidation
```

**필수 불변조건:** `geometry doc revision != render revision`일 때 낡은 mesh가 재계산된 shape의 새로운 face로 선택됐다고 표시되지 않도록 pick commit에서 revision guard.

## 3. 카메라·좌표계·정밀도

- Model canonical coordinate: right-handed, mm, f64.
- GPU: f32 vertex buffer(카메라 기준 origin rebasing) + view/projection matrix; large site coordinates 지원 테스트.
- Orthographic camera: 치수·도면 조작 기본; perspective for visual review.
- Orbit(camera target), pan(view plane), zoom(distance or orthographic scale), zoom-to-cursor, fit selection/model.
- `world↔screen`, view orientation cube, near/far clipping, depth precision behavior 명시; 기본 wgpu depth range 확인.
- World matrix composition: `ParentWorld * LocalTransform`; negative scale orientation/normals validity 테스트.

## 4. 렌더 패스(R2 기본, R3 고급)

| 순서 | R2 CAD viewport | R3 visualization |
|---|---|---|
| 1 | Clear/depth | environment/IBL setup |
| 2 | Solid shaded pass | physically based material pass |
| 3 | Visible edge overlay | shadows/AO(성능 제한) |
| 4 | Selection outline/gizmo | transparency/compositing |
| 5 | Grid/axes/dimensions | tone mapping/AA |
| 6 | UI overlay egui | image capture PNG |

렌더/재질 구현을 위해 모델을 불필요하게 GPU로 계속 복제하지 않도록 geometry/material/revision별 캐시. 재질만 바꾸면 topology tessellation 불필요.

## 5. Picking/Highlighting 설계

- Mouse pointer → camera ray. Object-level candidate broad-phase AABB/BVH → triangle ray or ID-buffer pick → `FaceId` / `EdgeId` 참조와 현재 shape topology 매핑 대조.
- Edge의 시각적 굵기와 실제 3D picking tolerance를 분리; 화면 픽셀 반경 사용.
- 선택 모드: Body / Face / Edge / Vertex. `FaceId` 는 영속 저장의 유일 식별자가 아니라 현재 형상 revision의 선택 토큰.
- Hover cost throttle, drag outline incremental update, 그리드/축 pick mask.
- GPU ID buffer가 플랫폼마다 다르게 동작하면 CPU-raycast fallback을 유지.

## 6. Tessellation 정책

- CAD B-Rep은 Surface/Curve 정확한 형상. Tessellation은 시각적 정밀도-성능의 절충.
- `MeshQuality {linear_deflection_mm, angular_deflection_rad, max_triangles, adaptive}`. 화면 확대 상태에 따라 LOD 적용(차후), export 품질과 viewport 품질은 분리.
- Zero-area triangle/NaN/invalid normal/crossing surface 등의 검사.
- Face-to-triangle association은 material/selection 구현에 필수.
- 변형 없는 모델에 repeated tessellation 피하고 cache key = B-Rep revision + quality settings + kernel version.
- STL export는 별도 설정/메시 무결성 체크하며 viewport mesh를 무조건 재사용하지 않음.

## 7. PBR(물리기반 재질, R3)

- `Material`: base_color, metallic, roughness, normal_map(optional), emissive, alpha_mode.
- glTF 2.0 `metallic-roughness`와 최대한 일치하는 내부 프리셋 구성. CAD 원본의 물성/질량 밀도와 visual material을 분리(알루미늄처럼 보여도 실제 소재 지정과 동일하지 않음).
- Color management: linear workflow, sRGB texture conversion, normal maps in linear, output tone mapping, image export gamma 확인.
- Lights: ambient/env/HDRI, directional/key/fill, roughness reflections. 내장 GPU에서 shadows/SSAO와 고해상도 IBL은 성능 프리셋으로 제한.
- Snapshot: resolution, sample/quality preset, transparent bg 여부, reproducibility(metadata with shader version, environment, camera).
- glTF는 visualization mesh/scene exchange용, STEP은 precision geometry용. 사용자가 export 목적을 혼동하지 않도록 UI 설명.

## 8. 장치/이벤트/성능

- App start: adapter selection, D3D12/Vulkan backend support detection, GPU capabilities log and user-facing fallback error.
- Window resize/DPI change: surface configuration, depth target regeneration, memory release.
- GPU device-lost/out-of-memory: surface recreation attempt, degraded mode/clear error; document untouched.
- CPU work and GPU frame pacing independent: viewport only necessary when interaction/revision change; costly feature rebuilding on background model worker.
- 프로파일 metrics: frame time(ms), 50/95p frame time, triangles, draw calls, GPU memory estimate, cache miss, tessellation time, picking latency, model rebuild latency.
- Benchmarks on integrated GPU class Windows laptop; initial goal **~100k visible triangles at usable 1080p response**. 수치는 약속 아닌 실험용 초기 목표, P06에서 실제 상한 확정.

## 9. Renderer acceptance tests

- V-01: 원통 곡면의 faceting 품질/노멀/모서리 정렬 확인.
- V-02: model change 동안 stale face pick result 차단.
- V-03: Orbit 360°, fit, orthographic toggling 후 domain revision unchanged.
- V-04: Window resize/DPI scale, minimized/recreated surface 안정.
- V-05: Shader compilation failure/device loss 시 autosave/data integrity 유지.
- V-06: R3 PNG 1920×1080, material metallic/roughness 값의 시각적 대비 및 파일 유효성.

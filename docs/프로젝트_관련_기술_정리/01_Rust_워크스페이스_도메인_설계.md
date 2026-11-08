# Rust 핵심 타입·워크스페이스

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 실제 CAD 구현 시 알아야 할 기술 원리·역할·실패 요건

---

## 2. 제안 Cargo workspace

```text
cad_studio/
├── Cargo.toml                     # lockfile 커밋, version pin, feature flags
├── apps/
│   ├── cad_app/                   # Desktop shell / UI states / keymap
│   └── cad_cli/                   # validate / inspect / export / recover
├── crates/
│   ├── cad_types/                 # IDs, Length/Angle, coordinate primitives, Error
│   ├── cad_core/                  # Document, Entity, Layer, Body, Sketch, Feature refs
│   ├── cad_geometry/              # 2D/3D analytic geometry, predicates, transforms
│   ├── cad_sketch/                # Sketch constraints + solver port
│   ├── cad_tolerance/             # Dimension, tolerance, trace, standards policy
│   ├── cad_command/               # Transaction, undo/redo, dirty set
│   ├── cad_feature/               # Feature DAG, regeneration, referential repair
│   ├── cad_kernel_api/            # KernelPort trait/types and result codes
│   ├── cad_kernel_occt/           # Dedicated C++ build/FFI wrapper **unsafe boundary**
│   ├── cad_tessellation/          # BRep to triangle mesh, face/edge IDs, cache
│   ├── cad_render/                # RenderScene/RenderPrimitives independent of GPU
│   ├── cad_viewport_wgpu/         # GPU pipelines, camera, picking, PBR
│   ├── cad_io/                    # Native archive, migrations, autosave, backups
│   ├── cad_exchange/              # Format ports: STEP/STL/glTF/DXF/SVG/PDF
│   ├── cad_batch/                 # job runner, deterministic reporting
│   └── cad_testing/               # Fixture helpers/shape checks
├── native/occt_bridge/            # C++ façade, CMake config; tests
├── schemas/                       # JSON schemas + fixture files
├── tests/e2e/                      # Goldens, crash/integration scenarios
└── docs/
```

> 모든 crate를 Phase 0에 빈 형태로 한 번에 생성할 필요는 없습니다. **경계와 인터페이스를 먼저 확정한 뒤 단계별 추가**합니다. 의존성 사이클은 금지합니다.

## 3. 각 계층의 책임

| 계층 | 책임 | 책임이 아닌 것 |
|---|---|---|
| cad_core | 도메인 aggregate, 안정 ID, 참조 제약 | OCCT TShape/GUI 포인터 보유 |
| cad_command | 불변조건 사전검증, 트랜잭션, inverse/snapshot Undo | 직접 GPU buffer 갱신 |
| cad_geometry | Rust 자체 기하 알고리즘, snap/hit-test, 단위·수치 정책 | 본격 솔리드 Boolean을 처음부터 독자 구현 |
| cad_sketch | plane-local entities, DOF, solver status, constraint graph | 3D mesh를 정밀 스케치로 취급 |
| cad_feature | DAG, dirty propagation, regen staging, topology refs | 렌더링 FPS 관리 |
| cad_kernel_api | `ShapeHandle` 추상화, 실패 코드, 독립 mock | C++ 헤더 누출 |
| cad_kernel_occt | 커널 연산 및 리소스·예외·스레드 제한 캡슐화 | Document·UI 상태 소유 |
| cad_tessellation | B-Rep faceting + face→triangle association | 설계 원본 데이터 관리 |
| cad_render | RenderScene과 layer/style/material 가시성 | 모델 영속화·편집 |
| cad_viewport_wgpu | draw/pick/camera/GPU resource | shape validity 검사 |
| cad_io | schema DTO, atomic save, autosave, migration | STEP 재수입을 native-load라고 간주 |
| cad_exchange | 포맷별 손실·단위·지원 매핑 | 내부 stable IDs를 외부 handle로 대체 |

## 4. 주요 도메인 스키마(개념 정의)

```rust
// Rust 유사 의사코드입니다. 확정 API 및 컴파일되는 코드가 아닙니다.
struct Document {
    id: DocumentId,
    schema_version: u32,
    units: Units,
    params: ParameterTable,
    sketches: Map<SketchId, Sketch>,
    draftings: Map<DrawingId, Drawing>,
    bodies: Map<BodyId, BodyDefinition>,
    features: FeatureDag,
    materials: Map<MaterialId, Material>,
    sheets: Map<SheetId, Sheet>,
    revision: u64,
}
struct Sketch { id: SketchId, plane: PlaneRef, geometry: Vec<SketchEntity>,
                constraints: Vec<Constraint>, solver_status: SolverStatus }
struct Feature { id: FeatureId, kind: FeatureKind, inputs: Vec<FeatureRef>,
                 params: ParameterRefs, status: FeatureStatus }
struct ShapeRevision { id: ShapeRevisionId, source: FeatureId,
                       brep_cache_key: CacheKey, valid: bool }
```

- **DocumentId/FeatureId/EntityId**는 JSON/native 파일에서 유지되는 논리 ID.
- **ShapeHandle/BRep**는 커널 런타임의 불투명 객체·영속 캐시 또는 바이너리 형태.
- **TopologySelection**은 `FeatureId + SubshapeSelector + geometric signatures + ambiguity status`로 표현하며, FaceIndex를 영구 저장하지 않습니다.
- **Undo/Redo**는 명령만 복제하면 안 될 수 있어 canonical input command + before/after snapshot 또는 inverse 값을 조합하고 재생성 결과를 검증합니다.## 핵심 계약(설계용 인터페이스)

```rust
// 개념적 의사코드: 구체적인 crate 타입/컴파일 성공을 약속하지 않습니다.
pub trait DocumentCommandHandler {
    fn execute(&mut self, expected_revision: u64, command: DrawingCommand)
        -> Result<CommittedRevision, CadError>;
}

pub trait SolidKernelPort {
    fn extrude(&self, profile: ProfileRef, depth_mm: f64)
        -> Result<ShapeRevision, KernelError>;
    fn cut(&self, base: ShapeRef, tool: ShapeRef)
        -> Result<ShapeRevision, KernelError>;
    fn validate(&self, shape: ShapeRef) -> Result<ShapeValidity, KernelError>;
}
```

- `expected_revision`으로 비동기 stale result와 동시 수정 충돌을 식별합니다.
- Geometry/Kernel 결과는 staging 모델에서 검증 후 commit합니다.
- UI는 Document를 직접 mutate하지 않으며 읽기용 snapshot으로 렌더링합니다.
- 외부 STEP ID, DXF handle, OCCT face index는 내부 StableID의 대체값이 아닙니다.

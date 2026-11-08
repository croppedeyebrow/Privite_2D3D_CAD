# in-process BE 도메인과 명령·동시성

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: in-process BE 도메인과 명령·동시성
> 원문 추적: 2

---

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
- **Undo/Redo**는 명령만 복제하면 안 될 수 있어 canonical input command + before/after snapshot 또는 inverse 값을 조합하고 재생성 결과를 검증합니다.

## 5. 명령과 재생성 처리

```text
UI  -> Draft Input (temporary)
    -> Typed Command {expected_revision, affected_ids, parameters}
    -> Permission / Locked-Layer / units / validity precheck
    -> Apply on staging document
    -> Calculate dirty feature subtree (no commit yet)
    -> Kernel recompute on model worker
    -> Shape validity + referential integrity
    -> atomic commit Document revision N+1 OR rollback
    -> undo journal + autosave enqueue
    -> Render scene snapshot/revision published
    -> GPU upload scheduled (cache derived)
```

- 계산이 오래 걸릴 때 UI는 **상태 불변 스냅샷**을 계속 표시하고 취소/상태 피드백 제공.
- stale async result: `expected_revision` mismatch이면 결과 폐기 또는 명시적 재계산.
- 커널 호출의 thread safety가 입증되기 전에는 **전용 단일 모델 워커에서 직렬화**; C++ 예외를 Rust로 넘기지 않고 Result로 변환.
- FFI 비용이 높은 fine-grained 함수 남발 대신 coarse-grained Feature operation 처리.
- fatal native crash 격리는 프로세스 외 워커가 필요: 초기에는 C++ unit tests/validation/보수적 한계를 두고, 외부 파일 import 등 위험한 단계의 별도 프로세스 격리 PoC를 병행.

## 6. 언어·런타임 정책

- Rust: UI, 도메인, command, sketch graph, persistence, CLI, renderer.
- C++(종속 코드/작은 façade): OCCT B-Rep kernel, Boolean, STEP, tessellation. **Rust만 사용한다는 이전 방향을 바꾸는 제안**이며 10번 ADR 게이트 필요.
- WGSL: GPU 셰이더.
- Python/웹 서버/클라우드: 초기 필수 아님. 필요한 경우 batch scripting을 CLI/API로 먼저 지원.
- `#![forbid(unsafe_code)]`는 안전한 pure-Rust crates에 유지, **`cad_kernel_occt`/GPU FFI의 예외를 격리·문서화**. workspace 전체 unsafe 전면 금지와 OCCT 연동은 동시에 충족될 수 없음.

## 핵심 계약(설계용 인터페이스)

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


## Feature 재계산 규약

```text
사용자 치수 편집 → Typed Command → staging Document
→ DAG affected descendants 계산 → solver 검사
→ dedicated kernel worker에서 재계산 → B-Rep validity
→ topology reference reconciliation → commit OR rollback
→ cache invalidation → tessellation → GPU snapshot publish
```

**실패 규칙:** 순환 참조·해결 불가능한 구속·커널 연산 실패·면 참조 모호성·피처 유효성 실패는 기존 Document revision을 유지하고 오류 객체와 원인을 반환합니다. 커널의 스레드 안전성이 입증되기 전에는 모델 워커를 직렬화합니다.

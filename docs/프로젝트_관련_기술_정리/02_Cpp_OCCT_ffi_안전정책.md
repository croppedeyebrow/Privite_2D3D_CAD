# OCCT/Truck와 FFI 위험

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 실제 CAD 구현 시 알아야 할 기술 원리·역할·실패 요건

---

## 2. KernelPort 추상화(예시 인터페이스)

```rust
// 설계용 의사코드. 실제 OCCT/Truck API가 아님.
trait CadKernel {
    fn create_face(&mut self, profile: &ClosedRegion, policy: &GeoPolicy) -> KernelResult<ShapeKey>;
    fn extrude(&mut self, profile: ShapeKey, direction: Vector3, mm: f64) -> KernelResult<ShapeKey>;
    fn revolve(&mut self, profile: ShapeKey, axis: Axis3, radians: f64) -> KernelResult<ShapeKey>;
    fn boolean(&mut self, a: ShapeKey, b: ShapeKey, op: BoolOp) -> KernelResult<ShapeKey>;
    fn validate(&self, shape: ShapeKey) -> KernelResult<ShapeReport>;
    fn properties(&self, shape: ShapeKey) -> KernelResult<MassProperties>;
    fn tessellate(&self, shape: ShapeKey, quality: MeshQuality) -> KernelResult<MeshWithIds>;
    fn export_step(&self, shapes: &[ShapeKey], units: Units) -> KernelResult<Vec<u8>>;
}
```

**추가할 계약:** Rust가 가진 shape ID와 native shape lifetime, `Drop`/thread confinement, exception->error mapping, cancellation semantics, size limits, shape check, return ownership. 커널별 지원 함수는 capability table을 명시하고 UI에서 미지원 기능 비활성화.

## 6. 커널 기술 후보

### 우선 후보: OCCT

- 장점: 성숙한 B-Rep modeling, boolean, STEP/IGES, model validity, tessellation.
- 비용: C++ build + Rust FFI, OS/VC++ runtime/installer, native crash risks, license obligations, object ownership, topological naming engineering.
- 권고: Rust `cad_kernel_api` trait, `cad_kernel_occt` 전용 crate와 `native/occt_bridge`만 C++; 앱·도메인·UI Rust 유지.

### 대안 후보: Truck (pure Rust)

- 장점: Rust-centric 개발 경험, 일부 모델링 및 B-Rep 개념 모듈화.
- 검증 미완: 요구되는 Boolean, STEP, 오류복구, fillet 등 특정 버전의 **실제 동작**을 PoC로 점수화해야 함.
- 초기부터 커널 이식성을 최적화하기보다 **제품의 필요한 Feature API 최소 공통계약**부터 고정.

## FFI 오류·메모리 정책

- C++ 예외는 boundary 안에서 포착하고 tagged error로 변환합니다.
- Rust에 native raw pointer를 영속 저장하지 않습니다.
- 객체 소유권과 스레드 접근은 명시적 handle lifetime/locking으로 검증합니다.
- native DLL 및 빌드 아티팩트에 대한 플랫폼별 라이선스 고지를 검토합니다.
- 커널이 crash하면 같은 프로세스의 Rust UI도 종료할 수 있으므로 단계적 worker process isolation을 검토합니다.

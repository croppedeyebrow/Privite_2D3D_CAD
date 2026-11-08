# 기하·수치 안정성·구속 솔버

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 실제 CAD 구현 시 알아야 할 기술 원리·역할·실패 요건

---

## 2. geometry 기본 타입

- `Point2(x,y)`, `Vector2`, `LineSegment2`, `InfiniteLine2`, `Circle2`, `Arc2`, `Polyline2`, `Spline2(P1)`, `Transform2`.
- `Point3(x,y,z)`, `Vector3`, `Plane3(origin, normal, x_axis, y_axis)`, `Transform3`.
- 저장 단위 mm, f64 좌표. 화면/GL shader f32는 별도 변환 및 camera-relative 좌표 사용.
- 내부 angle 단위 radians, UI deg 병기. 법선/축 정규화 및 오른손 좌표계 테스트.
- 유효값: 모든 실수 finite, radius>0, zero-length line 거부/경고, 행렬 invertibility 검사.

## 3. 수치 오차와 robust predicates

- 전역 상수 `epsilon = 1e-6` 하나로 모든 연산 처리 금지. `ToleranceContext`에 절대·상대·각도/스냅·모델 범위·커널 해상도 따로 보관.
- float equality가 아닌 거리/방향/교점 결과의 `None`, `Unique`, `Overlapping`, `NearDegenerate` 같은 명시적 결과형.
- 고정밀 기하 계산과 UI snap 반경(화면 pixel), kernel B-Rep tolerance는 서로 다른 도메인.
- 속성 테스트: parallel/collinear intersection, tiny segments, 1km 규모 모델 좌표, almost tangent arcs, reversed polyline winding.

## 5. Sketch Constraint Graph

**대상:** point/segment/arc/circle + Constraint. 초기 관계는 coincident, fixed, horizontal, vertical, parallel, perpendicular, equal length/radius, distance, angle, radius, concentric(연동 지원 범위 검증 후만 노출).

**상태:**
- `UnderConstrained {estimated_dof}`
- `FullyConstrained`
- `OverConstrained {conflicting_constraint_ids}`
- `Redundant {constraint_ids}`
- `SolveFailed {diagnostic}`

주의: solver마다 정확한 DOF/충돌 집합 제공 능력이 다르므로 라이브러리 API가 미지원일 때 **추정과 확정 진단을 구분 표시**합니다.

### 초기 Solver 채택 게이트

1. 실제 라이브러리 통합 난이도(Windows 빌드 및 배포).
2. 기초 constraint 8~10개 조합, 구속 중복/충돌/수렴 실패 탐지.
3. 속도(30/100/500 기하 객체 포함 stress case).
4. 라이선스와 정적/동적 링크 정책.
5. 공차 및 반복 실행 결과 안정성.

**SolveSpace/libslvs**는 검토 대상이나 공식 사이트에 GPLv3 및 링크 제약 안내가 있으므로, 상용/비공개 프로그램을 배포할 계획이라면 라이선스 검증 전 채택하지 않습니다. 라이선스 불일치 시 다른 라이브러리 또는 제한된 자체 solver로 범위를 조정합니다.

## 6. 닫힌 프로파일 검증

- endpoint linkage, `wire.is_closed`, self-intersection, duplicate segment, orientation, hole containment/overlap.
- outer loop + nested inner loops를 분리하여 **구멍이 있는 면 영역** 생성.
- 끊긴 윤곽은 gap endpoint와 mm 거리 표시.
- Recompute 전에 feature semantics(단일 solid or multi-solid allowed)를 판정.
- valid 2D sketch라도 뒤 단계 extrude/cut 실패 가능하므로 kernel shape validity를 추가 적용.

# GUI 정보구조 및 입력·상태·뷰포트

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: GUI 정보구조 및 입력·상태·뷰포트
> 원문 추적: 4

---

## 1. 정보 구조

**워크벤치(모드)별 기능을 분리하되 단일 Document와 Command Bus를 공유**합니다.

- **Drafting 2D:** 독립 2D 도면/레이어/치수/공차/도곽/인쇄.
- **Sketch:** 선택 Plane에서 기하와 구속조건 편집, fully constrained 시각화.
- **Part Design:** Sketch와 Feature Tree 기반 Extrude/Revolve/Cut, 면/모서리 선택, 재계산 상태.
- **Render:** 동일 Part의 재질/조명/카메라, 고해상도 이미지 출력(후속).
- **Drawing (P1):** 3D Part 링크 정면/평면/측면 투상도, 도면 레이아웃/Hidden Lines.
- **Assembly/Architecture (P2):** 별도 워크벤치로 확장. 현재 메뉴 비활성 표시 가능하지만 기능 완성처럼 오해 금지.

## 2. 기본 화면

```text
┌─────────────────────────────────────────────────────────────────────────────────────┐
│ CAD Studio | File Edit View Insert Sketch Part Inspect Render Tools Help | ● Saved │
├───────────────────┬───────────────────────────────────────┬─────────────────────────┤
│ WORKBENCH / TREE  │ 2D/3D VIEWPORT                         │ PROPERTIES / DIAGNOSTICS│
│ Document          │                                       │ Selected object         │
│  ├ Parameters     │  Orientation cube  ○  camera          │ dimensions / constraints│
│  ├ Sketches       │                                       │ references / material   │
│  ├ Bodies         │            [ precise model ]           │ DOF / Validity / History│
│  │  ├ Extrude1    │                                       │ Errors / Warnings       │
│  │  └ Cut1        │                                       │                         │
│  ├ Drawings       │                                       │                         │
│  └ Materials      │                                       │                         │
├───────────────────┴───────────────────────────────────────┴─────────────────────────┤
│ COMMAND: Extrude | XYZ: 12.50, -3.00, 0.00 mm | Snap: ON | Rebuild: OK | GPU: D3D12│
└─────────────────────────────────────────────────────────────────────────────────────┘
```

## 3. 툴바 / 메뉴 / 단축키

| 범주 | 커맨드 | 기본 처리 |
|---|---|---|
| View | Fit (F), Orthographic/Perspective, Top/Front/Right, Orbit/Pan/Zoom | 카메라만 변경; Document dirty=false |
| Edit | Undo Ctrl+Z, Redo Ctrl+Y/Ctrl+Shift+Z, Delete, Copy | typed Command / 재생성 필요시 연계 |
| Draft | Line, Arc, Circle, Polyline, Text, Dim, Layer | 좌표입력/스냅 반영 |
| Sketch | Create sketch, Constraints, Show DOF, Close Sketch | 로컬 좌표 & solver 상태 |
| Part | Extrude, Revolve, Cut, Boolean | 입력 유효성/preview/commit |
| Inspect | Measure, BBox, Volume, Validate, Select subshape | 수치 단위·정확성/경고 표시 |
| File | Save/Save As/Recovery/Import/Export | 포맷별 손실 표기 |
| Render(P1) | Material, Lights, Camera, Preview, PNG | domain visual attrs와 GPU render state 분리 |

단축키는 텍스트 속성/치수 입력창 포커스일 때 무단 발동 금지. ESC는 1) active interaction 취소 2) selection 해제 순서. 3D 조작(orbit)은 설계 Command가 아니므로 Undo stack에 들어가지 않습니다.

## 4. 상호작용 상태 머신

```text
Idle
  ├─ Select -> Hover -> Selected -> Manipulation Preview -> Confirm Command/Cancel
  ├─ Sketch Tool -> first point -> geometry preview -> next point -> constraint suggestion -> Commit
  ├─ Feature Tool -> pick sketch/face/axis -> parameter draft -> staged preview -> Accept/Cancel
  └─ Inspect -> pick entity/face/edge -> read-only properties
```

- Feature Preview는 staging/revision-tagged 결과만 표시; 사용자가 취소하면 Document revision 미변경.
- 수정 중 다른 명령으로 이동할 때 입력 draft 유지/폐기 확인.
- invalid sketch 또는 boolean 실패에 대해 오류 원인과 해당 tree node·형상 highlight.
- topology ambiguous/lost ref는 에러/경고로 구분하고 자동으로 임의의 다른 Face에 재연결하지 않음.

## 5. 좌표 입력과 스냅 UX

- 왼쪽 아래 월드좌표 mm와 현재 Sketch local 좌표 별도 표시.
- 커서 이동·카메라 depth와 실제 스냅된 3D 좌표는 반드시 구분.
- 휠 zoom/마우스 pan/orbit, X/Y/Z 축 기즈모, Grid/Snap 토글.
- 스냅 우선순위: 명시 참조 > endpoint > intersection > midpoint/center > grid > free; 화면 픽셀→월드 허용오차 변환에 upper/lower bound 적용.
- 단위 입력: `10`, `10 mm`, `1 cm` 등 허용 범위를 계약으로 정의하고, 원본 파라미터의 식(expression) 보존(P1).

## 6. 패널별 명세

**Feature Tree:** Sketch/Body/Feature DAG(시각화는 사용자 친화적 순서), `OK/Dirty/Computing/Failed/Suppressed/Outdated` 상태. 오류 클릭 시 원인과 affected descendants 표시.

**Properties:** 안정 ID / Feature parameters / Sketch DOF / Planar geometry / Face properties / 단위 / computed volume/BBox. 기하 상태를 텍스트 편집으로 직접 훼손하지 못하게 Command validation.

**Issue & Recovery:** `Error`, `Warning`, `Info` 분리; 오류코드·오브젝트·위치·재현 메타데이터; `Retry`, `Rebind face`, `Rollback feature`, `Open backup as copy` 등 컨텍스트 액션.

**Save Status:** `Saved`, `Modified`, `Autosave pending`, `Saving`, `Autosaved`, `Save failed`, `Recovered copy`. 현재 파일/복구 사본을 혼동하지 않도록 제목/상태 별도.

**Rendering(P1):** CAD shading vs final preview 모드 명확 분리. 실시간 viewport의 목적은 정확한 선택과 작업 피드백; 고품질 PBR 프리뷰는 작업 UI 지연 방지하도록 quality preset.

## 7. 접근성/한글/제어

- 단축키·키보드 탭 순서, focus ring, 색깔만으로 오류 상태 구분하지 않음.
- 현지화: SI/mm, 한글 텍스트 렌더/폰트 fallback; SVG/PDF/STEP 변환시 문자 손실 보고.
- 크기 조절 가능한 패널/고 DPI, 설정·워크벤치 레이아웃 사용자 환경설정으로 저장(설계 모델과 분리).
- File overwrite/unsaved changes/대량 export 작업은 범위·영향 명시 후 확인.

## 8. UI 검수용 작업 시나리오

- UAT-01: 스케치 치수 100→120mm 바꾼 후 extrude 부품 업데이트, 오류 없는 상태 배지.
- UAT-02: 잘못된 윤곽에서 Extrude 오류 발생→수정 후 retry, 중간에 원본 shape 유실 없음.
- UAT-03: 3D Face 선택 후 body 이동/뷰 회전에도 올바른 참조/속성 표시.
- UAT-04: 작업 중 강제종료→복구 사본 오픈→원본 덮어쓰기 없이 비교.
- UAT-05: STEP export 화면에서 **설계 피처 히스토리 손실 가능성**과 mm 단위 확인.

## 워크벤치 화면별 계약

| 워크벤치 | 기본 패널 | 도구 동작 | 오류 정책 |
|---|---|---|---|
| Drafting | Layers / Properties / Validation | 2D line/arc/polyline/dimension | 잠긴 레이어 수정 차단 |
| Sketch | Constraints / Solver / DOF | profile/constraint/solve | 중복/모순 구속 원인 표시 |
| Part | Feature Tree / Param / Body | Extrude/Revolve/Cut | 실패 피처 강조·롤백 |
| Rendering(R3) | Camera / Material / Light | orbit/light/preset/export | GPU 오류·드라이버 상태 표시 |

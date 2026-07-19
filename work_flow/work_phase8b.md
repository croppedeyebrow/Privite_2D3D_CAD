# Rust로 CAD 엔진 만들기 — Phase 8b: 도구 시스템과 undo/redo

## 들어가며

Phase 8a는 화면에 도면을 "보여주는" 것까지였다. 도구 모음의 버튼은 전부 비활성 상태였고, 캔버스를 클릭해도 아무 일도 일어나지 않았다. `cad_command`가 Phase 4에서 이미 완성해 둔 undo/redo도 `cad_app`에서는 단 한 번도 호출되지 않았다.

Phase 8b는 이 공백을 채운다 — 사용자가 실제로 도형을 그리고, 선택하고, 옮기고, 되돌릴 수 있게 만드는 단계다. `04_UI_와이어프레임_정책.md`가 정의한 상호작용 흐름을 그대로 구현 대상으로 삼았다.

```text
도구 선택 -> 입력 수집 -> 미리보기 -> 확정 command -> 검증 -> 렌더 갱신 -> 자동 저장 예약
```

("자동 저장 예약"은 `cad_io`가 연결되는 8d까지 그대로 비워둔다.)

---

## 1. Phase 8b의 목적

8a 문서의 "다음 Phase 예정 범위"에 이미 다음을 적어뒀다.

- 선택 도구, 그리기 도구(선/사각형/원/호/텍스트), 이동 도구
- undo/redo를 `CommandHistory`에 연결
- `cad_geometry::snap_candidates`/`nearest_point`를 포인터 입력에 연결

이번 Phase는 이 약속을 그대로 지키는 것이 전부다. 새로운 도메인 규칙은 만들지 않았다 — 여섯 개 도구 전부, Phase 1~4에서 이미 검증까지 끝난 `cad_core`/`cad_geometry`/`cad_command`의 기존 함수를 조합하기만 한다.

---

## 2. `tool.rs`를 다시 순수 함수로

Phase 8a에서 `camera.rs`를 `egui` 타입 없이 만든 이유를 "GUI 프레임워크가 바뀌어도 이 파일은 거의 그대로 남는다"고 적었다. 이번에도 같은 전략을 썼다.

```text
src/
├─ tool.rs   Tool, DrawState, 도형 생성 함수, hit_test, snap_point — egui 없음
└─ app.rs    tool.rs를 egui 이벤트(클릭, 드래그, 스크롤)에 연결
```

`tool.rs`의 모든 함수는 `(f64, f64)` 튜플과 `cad_core`/`cad_geometry`/`cad_render` 타입만 다룬다. 덕분에 "도형이 올바르게 만들어지는가", "스냅이 올바른 점을 찾는가" 같은 핵심 로직을 실제 창을 띄우지 않고 `cargo test`만으로 검증했다 — 이번 Phase의 테스트 20개 중 11개가 `tool.rs`에 있다.

---

## 3. 도형 생성 — 클릭 좌표에서 `EntityGeometry`로

### 3.1 사각형: 드래그 방향과 무관하게 정규화

사용자는 사각형을 왼쪽 위에서 오른쪽 아래로 그릴 수도, 반대로 그릴 수도 있다. `cad_core::Rectangle`은 원점(좌하단)과 양수 width/height만 허용하므로, 두 클릭 좌표를 그대로 저장할 수 없다.

```rust
pub fn rectangle_geometry(a: (f64, f64), b: (f64, f64)) -> EntityGeometry {
    EntityGeometry::Rectangle(Rectangle {
        origin: Point2::new(a.0.min(b.0), a.1.min(b.1)),
        width: LengthMm((a.0 - b.0).abs()),
        height: LengthMm((a.1 - b.1).abs()),
    })
}
```

`min`/`abs` 조합으로 두 점의 순서를 정규화한다. `rectangle_geometry_normalizes_corners_dragged_up_left` 테스트가 "거꾸로" 드래그해도 같은 결과가 나오는지 확인한다.

### 3.2 호: 3클릭 정의와 CCW 감음(wrap-around)

호 도구는 중심 → 시작점 → 끝점, 세 번 클릭한다. 반지름과 시작각은 시작점에서, 끝각은 끝점에서 얻는다(끝점까지의 거리는 무시 — 끝점은 각도만 정의한다).

```rust
pub fn arc_geometry(center: (f64, f64), start: (f64, f64), end: (f64, f64)) -> EntityGeometry {
    let radius = cad_geometry::distance(Point2::new(center.0, center.1), Point2::new(start.0, start.1));
    let start_angle = (start.1 - center.1).atan2(start.0 - center.0);
    let end_angle = (end.1 - center.1).atan2(end.0 - center.0);

    let mut sweep = end_angle - start_angle;
    while sweep <= 0.0 {
        sweep += std::f64::consts::TAU;
    }
    // ...
}
```

`cad_core::Arc`는 "sweep_angle이 양수면 반시계 방향"이라는 규칙을 갖고 있다(Phase 1). 그런데 `end_angle - start_angle`은 음수가 나올 수도 있다 — 예를 들어 시작각이 90°, 끝각이 0°라면 단순 차는 `-90°`다. `while sweep <= 0.0 { sweep += TAU }`는 이 값을 "반시계로 감아서" 항상 양수인 sweep으로 바꾼다.

> ⚠️ **이 방식의 한계**
>
> 3클릭만으로는 "반대쪽" 호(우회전 방향의 큰 호)를 그릴 방법이 없다. 항상 시작점에서 끝점까지
> 반시계 방향으로 가는 짧은/긴 쪽 중 하나로 고정된다. `arc_geometry_wraps_when_end_angle_precedes_start_angle`
> 테스트가 이 감음 동작 자체는 검증하지만, "다른 방향을 선택하고 싶다"는 요구는 이번 범위에서
> 다루지 않았다 — 남은 과제에 기록했다.

### 3.3 텍스트: 고정 높이

`text_geometry`는 항상 `DEFAULT_TEXT_HEIGHT_MM`(5mm)로 텍스트를 만든다. 글자 높이를 사용자가 지정하는 UI는 아직 없다 — 속성 편집 자체가 8c의 범위이기 때문이다.

---

## 4. 스냅과 선택 — Phase 2의 약속을 지키는 지점

### 4.1 `snap_point`

그리기 도구로 클릭할 때, 클릭한 좌표를 그대로 쓰지 않고 주변 엔티티의 스냅 후보(끝점, 중점, 중심 등)에 가까우면 그 후보로 "달라붙게" 만든다.

```rust
pub fn snap_point(drawing: &Drawing, raw: (f64, f64), tolerance_mm: f64) -> (f64, f64) {
    let target = Point2::new(raw.0, raw.1);
    let mut candidates = Vec::new();
    for entity in &drawing.entities {
        if cad_render::is_layer_visible(drawing, entity.layer_id) {
            candidates.extend(cad_geometry::snap_candidates(&entity.geometry));
        }
    }
    cad_geometry::nearest_point(target, &candidates, LengthMm(tolerance_mm))
        .map_or(raw, |p| (p.x.0, p.y.0))
}
```

`cad_geometry::snap_candidates`와 `nearest_point`는 Phase 2에서 이미 만들어 뒀지만, Phase 2 문서의 "남은 과제"에 "아직 포인터 이벤트와 연결되지 않았다"고 적어뒀던 함수들이다. 이번에 처음으로 실제 사용자 입력에 연결됐다.

허용 오차(`tolerance_mm`)는 화면 픽셀 단위(`PICK_TOLERANCE_PX = 8.0`)를 현재 줌 값으로 나눠서 구한다 — 확대할수록 스냅이 더 정밀해지고, 축소할수록 더 넉넉해진다.

```rust
let tolerance_mm = f64::from(PICK_TOLERANCE_PX / self.camera.zoom);
```

캔버스에서는 스냅이 실제로 발생했을 때(원래 좌표와 스냅된 좌표가 다를 때) 노란 원 마커를 그려 사용자에게 알려준다.

### 4.2 `hit_test` — 선택도 같은 함수로

선택 도구가 "지금 클릭한 위치에 어떤 엔티티가 있는가"를 판단하는 방식도 스냅과 똑같다 — 각 엔티티의 스냅 후보 중 가장 가까운 것을 찾고, 그 후보를 가진 엔티티를 선택한다.

```rust
pub fn hit_test(drawing: &Drawing, world_point: (f64, f64), tolerance_mm: f64) -> Option<EntityId> {
    // snap_point와 같은 순회, 다른 목적: 가장 가까운 엔티티의 id를 반환
}
```

> 📌 **왜 새 기하 함수를 만들지 않았는가**
>
> "클릭한 곳에서 가장 가까운 도형을 찾는" 정확한 방법은 점-선분 거리, 점-사각형 거리 같은
> 새로운 기하 계산이 필요하다. 하지만 `cad_geometry`에는 아직 그런 함수가 없다.
> 대신 이미 있는 `snap_candidates`(끝점, 중점, 중심, 꼭짓점)를 재사용해서,
> "도형의 특징점 근처를 클릭해야 선택된다"는 더 단순한 규칙으로 8b를 완성했다.
> 도형의 테두리 아무 곳이나 클릭해서 선택하는 정교한 히트 테스트는 필요성이 확인되면 추가한다.

`hit_test`도 숨긴 레이어의 엔티티는 건너뛴다 — 화면에 안 보이는 것을 선택할 수 있으면 안 되기 때문이다. 이 검사에 `cad_render::is_layer_visible`을 그대로 재사용했다(아래 6절).

---

## 5. 선택한 엔티티 이동 — 화면 좌표로 누적하는 이유

드래그하는 동안 실제 `Project`를 매 프레임 바꾸지 않는다. 대신 `DrawState::Moving`이 드래그의 누적 이동량을 **화면 픽셀 단위**로 들고 있다가, 드래그가 끝나는 순간에만 월드 좌표로 변환해 `MoveEntity` command 하나를 실행한다.

```rust
Moving {
    entity_id: EntityId,
    screen_delta: (f32, f32),
},
```

```rust
if response.dragged() {
    if let DrawState::Moving { screen_delta, .. } = &mut self.draw_state {
        let delta = response.drag_delta();
        screen_delta.0 += delta.x;
        screen_delta.1 += delta.y;
    }
}

if response.drag_stopped() {
    if let DrawState::Moving { entity_id, screen_delta } = self.draw_state {
        let dx = f64::from(screen_delta.0 / self.camera.zoom);
        let dy = f64::from(-screen_delta.1 / self.camera.zoom);
        self.execute(DrawingCommand::MoveEntity { id: entity_id, dx: LengthMm(dx), dy: LengthMm(dy) });
    }
}
```

> 💡 **왜 프레임마다 `MoveEntity`를 실행하지 않는가**
>
> `CommandHistory`는 `execute`를 호출할 때마다 undo 이력을 하나 쌓는다(Phase 4).
> 드래그 한 번에 프레임마다 command를 실행하면, "1픽셀 이동"이 undo 스택에 수십~수백 개
> 쌓이게 된다 — 사용자가 undo를 한 번 눌렀을 때 기대하는 "직전 이동 전체 되돌리기"와
> 맞지 않는다. 드래그가 끝나는 순간 딱 한 번만 `MoveEntity`를 실행해야 undo 한 번이
> 사용자의 동작 한 번과 대응한다.

이동 중에는 실제 데이터가 안 바뀌므로, 화면에는 원본 엔티티 위에 드래그 오프셋만큼 이동한 **강조 색 사본**을 겹쳐 그려서 "움직이는 것처럼" 보이게 했다(`draw_selection_highlight`). 드래그가 끝나야 진짜 위치가 바뀐다.

---

## 6. `cad_render`의 내부 함수를 공개 API로

선택 강조와 이동 미리보기를 그리려면 엔티티 하나만 골라 화면에 그리는 기능이 필요했다. `cad_render::build_render_model`은 프로젝트 전체를 변환할 뿐, 엔티티 하나만 변환하는 함수는 없었다 — 있긴 했지만 `entity_primitive`라는 이름의 **private** 함수였다.

```rust
// 이전: cad_render 내부에서만 쓰이던 private 함수
fn entity_primitive(geometry: &EntityGeometry) -> RenderPrimitive { .. }

// 변경: 공개 API로 승격, 이름도 역할에 맞게 변경
pub fn geometry_primitive(geometry: &EntityGeometry) -> RenderPrimitive { .. }
```

`is_layer_visible`도 같은 이유로 공개했다 — `hit_test`/`snap_point`가 "화면에 실제로 보이는 것만" 대상으로 삼으려면, `cad_render`가 렌더링 여부를 판단하는 것과 정확히 같은 함수를 써야 두 계층의 판단이 어긋나지 않는다.

이렇게 하면 `cad_app`은 "엔티티 하나를 어떻게 그림으로 바꾸는지"를 다시 구현하지 않고, `build_render_model`이 쓰는 것과 **완전히 같은 매핑**을 재사용한다 — 전체 렌더링과 미리보기가 절대 다르게 보일 일이 없다.

---

## 7. 텍스트 입력 팝업과 즉시 모드의 borrow 문제

텍스트 도구는 클릭한 자리에 `egui::Window`를 띄워 내용을 입력받는다. 처음 구현에서 막힌 지점이 있었다.

```rust
// 컴파일 안 됨: draw_state를 빌린 채로 self.add_entity(...)를 호출할 수 없다
if let DrawState::TextPending { origin, content } = &mut self.draw_state {
    // ... 창을 그리다가 확인 버튼을 누르면
    self.add_entity(tool::text_geometry(*origin, content.clone())); // 에러: self가 이미 빌려져 있음
}
```

`egui`의 즉시 모드 특성상, 텍스트 입력창은 **매 프레임** 현재 값을 읽고 다시 써야 한다(타이핑한 글자가 다음 프레임에도 남아있으려면). 그런데 그 "다시 쓰기"를 위해 `&mut self.draw_state`를 붙잡고 있으면, 확인을 눌렀을 때 `self.add_entity`(다른 `&mut self` 메서드)를 호출할 수 없다.

해결책은 상태를 먼저 **값으로 복제**해서 borrow를 끊는 것이었다.

```rust
fn text_input_popup(&mut self, ctx: &egui::Context) {
    let DrawState::TextPending { origin, content } = self.draw_state.clone() else { return };
    let mut content = content; // 로컬 변수 — self를 안 붙잡음

    // ... 창을 그리며 content를 편집, confirmed/cancelled 플래그만 기록

    if confirmed { self.draw_state = DrawState::Idle; self.add_entity(..); }
    else if cancelled { self.draw_state = DrawState::Idle; }
    else { self.draw_state = DrawState::TextPending { origin, content }; } // 타이핑 내용 저장
}
```

읽기(clone)와 쓰기(재대입) 사이에는 `self`를 전혀 빌리지 않으므로, 그 사이에 `self.add_entity`를 자유롭게 호출할 수 있다. 매 프레임 `String`을 복제하는 비용은 텍스트 입력 상자 하나 분량이라 무시할 만하다.

---

## 8. undo/redo 연결

`CommandHistory`에는 `execute`/`undo`/`redo`가 이미 있었지만, "지금 undo할 게 있는가"를 밖에서 알 방법이 없었다 — 버튼을 항상 활성 상태로 둘 수밖에 없었다. `cad_command`에 두 메서드를 추가했다.

```rust
pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
```

툴바 버튼, 메뉴 항목, 단축키(`Ctrl+Z`/`Ctrl+Y`/`Ctrl+Shift+Z`) 세 곳 모두 같은 `self.undo()`/`self.redo()`를 호출한다.

```rust
let undo = i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift;
```

`modifiers.command`는 `egui`가 제공하는 플랫폼 추상화로, Windows/Linux에서는 Ctrl, macOS에서는 Cmd에 대응한다 — 플랫폼별 분기를 직접 작성하지 않아도 됐다.

command 실행이나 undo/redo가 실패하면(예: 검증 실패) 그 오류를 `self.status_message`에 저장하고 오른쪽 패널에 그대로 노출한다 — `CommandError`에 이번에 `Display`를 구현해 둔 덕분에 `err.to_string()` 한 줄로 충분했다.

---

## 9. Clippy가 잡아낸 것

### `match_same_arms`

`RenderPrimitive::Line`과 `RenderPrimitive::Dimension`이 정확히 같은 방식(시작점-끝점을 잇는 선)으로 그려지고 있었는데, 두 개의 별도 `match` 갈래로 남아 있었다.

```rust
// 수정 후: 두 패턴을 |로 합침
RenderPrimitive::Line { start, end } | RenderPrimitive::Dimension { start, end, .. } => {
    painter.line_segment([to_screen(*start), to_screen(*end)], stroke);
}
```

### `derivable_impls`

`DrawState`의 `Default` 구현이 `#[derive(Default)]` + `#[default]` 어트리뷰트로 대체 가능하다는 지적이었다 — Rust 1.62부터 enum에도 `#[default]`로 기본 variant를 표시할 수 있다.

```rust
#[derive(Clone, Debug, PartialEq, Default)]
pub enum DrawState {
    #[default]
    Idle,
    // ...
}
```

---

## 10. 별도 문제 — 한글이 전부 깨져 보임

이 Phase의 코드를 커밋한 뒤, 실제로 실행해서 확인하던 중 사용자가 "글자가 다 ㅁㅁ으로 깨져서 보인다"고 보고했다.

### 원인

`egui`가 기본으로 내장하는 폰트(Hack, Ubuntu-Light)는 라틴 문자만 지원한다. 이 앱의 UI 텍스트는 메뉴부터 도구 이름까지 전부 한글이므로, 글리프가 없는 모든 문자가 대체 기호(tofu box)로 표시된 것이다 — Phase 8a~8b 내내 텍스트 자체는 `cargo test`로 검증할 수 없는 영역이었고, 실제 창을 열어보지 않았다면 이 문제를 코드만 보고는 알 수 없었다.

### 해결

새 폰트 파일을 프로젝트에 추가하는 대신(바이너리 자산과 라이선스를 새로 추적해야 함), Windows에 이미 설치된 한글 폰트(맑은 고딕)를 실행 시점에 읽어서 `egui`에 등록했다.

```rust
fn install_korean_font(ctx: &egui::Context) {
    const CANDIDATES: [&str; 2] = ["C:/Windows/Fonts/malgun.ttf", "C:/Windows/Fonts/NGULIM.TTF"];
    let Some(bytes) = CANDIDATES.iter().find_map(|path| std::fs::read(path).ok()) else {
        return; // 후보가 전부 없으면 조용히 포기 — 나머지 UI는 여전히 동작한다
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("korean".to_owned(), egui::FontData::from_owned(bytes));
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "korean".to_owned());
    fonts.families.entry(egui::FontFamily::Monospace).or_default().push("korean".to_owned());
    ctx.set_fonts(fonts);
}
```

`main.rs`에서 `eframe::run_native`의 앱 생성 클로저 안, `CadApp`을 만들기 전에 호출한다.

```rust
Box::new(|creation_context| {
    install_korean_font(&creation_context.egui_ctx);
    Ok(Box::new(CadApp::default()))
})
```

`Proportional` 폰트 목록의 맨 앞에 넣었기 때문에, 맑은 고딕이 한글뿐 아니라 라틴 문자까지 우선 담당하게 된다 — 폰트가 바뀌어도 글자가 깨지는 것보다는 통일된 모양이 낫다고 판단했다.

> ⚠️ **이 해결책은 Windows 전용이다**
>
> 파일 경로를 하드코딩했기 때문에 다른 운영체제에서는 후보가 전부 없어 한글이 다시 깨진다.
> 지금은 이 프로젝트가 Windows 환경만 대상으로 하고 있어 문제가 되지 않지만,
> 크로스 플랫폼을 지원하게 되면 폰트 파일을 직접 번들하거나 각 OS의 시스템 폰트 경로를
> 분기하는 작업이 필요하다 — 남은 과제에 기록한다.

---

## 11. 테스트

`tool.rs`에 11개, `cad_command`에 기존 undo/redo 테스트를 `can_undo`/`can_redo` 확인으로 확장했다.

| 테스트 | 검증 내용 |
|---|---|
| `line_geometry_builds_expected_line` | 선 생성 |
| `rectangle_geometry_normalizes_corners_dragged_up_left` | 역방향 드래그 정규화 |
| `circle_geometry_uses_distance_to_edge_as_radius` | 반지름 계산 |
| `arc_geometry_sweeps_positive_ccw_between_start_and_end` | 기본 CCW 스윕 |
| `arc_geometry_wraps_when_end_angle_precedes_start_angle` | 감음(wrap-around) 동작 |
| `text_geometry_uses_default_height` | 기본 높이 |
| `hit_test_finds_entity_within_tolerance` / `_returns_none_outside_tolerance` | 히트 테스트 |
| `hit_test_skips_entities_on_hidden_layers` | 숨긴 레이어 제외 |
| `snap_point_snaps_to_a_nearby_candidate` / `_returns_raw_point_when_nothing_in_range` | 스냅 |

`cad_command::add_entity_can_be_undone_and_redone`은 이번에 `can_undo()`/`can_redo()`가 매 단계에서 올바른 값을 반환하는지도 함께 확인하도록 확장했다.

---

## 12. 검증 결과

```text
cargo fmt --all -- --check
통과

cargo test --workspace
전체 통과
- cad_app: 20/20 (+11)
- cad_command: 8/8 (기존 테스트 확장)
- workspace total: 107

cargo clippy --workspace --all-targets -- -D warnings
경고 없음 (match_same_arms, derivable_impls, useless_conversion 수정 후 통과)

바이너리 실행: 사용자가 직접 실행해서 도구 동작을 확인
한글 폰트 버그: 사용자 보고 → 원인 파악 → 수정 → 재검증까지 완료
```

---

## 13. Phase 8b에서 사용한 개념

### 값 복제로 borrow 충돌 피하기

`self.draw_state.clone()`으로 읽기와 쓰기 사이의 借용 기간을 끊어, `&mut self`를 필요로 하는 다른 메서드를 자유롭게 호출했다.

### 화면 공간에서 누적, 커밋 시점에만 도메인 command로 변환

드래그 중에는 픽셀 단위로만 상태를 쌓고, 확정될 때 딱 한 번 도메인 command(`MoveEntity`)로 변환해 undo 단위를 사용자의 실제 조작 단위와 맞췄다.

### `#[default]` 어트리뷰트

enum의 특정 variant를 `#[derive(Default)]`의 기본값으로 지정해 수동 `impl Default` 없이 같은 동작을 얻었다.

### 런타임 폰트 로딩

컴파일 타임에 폰트를 번들하는 대신, 실행 중에 `std::fs::read`로 시스템 폰트를 읽어 `egui::FontDefinitions`에 등록했다 — 새 바이너리 자산을 추가하지 않고도 문제를 해결했다.

### 크로스 크레이트 API 재노출

`cad_render`의 private 헬퍼(`entity_primitive`)를 공개 API(`geometry_primitive`)로 승격해, `cad_app`이 같은 로직을 다시 구현하지 않고 재사용하게 했다.

---

## 14. Phase 8b 완료 결과

`cad_app`은 이제 실제로 도면을 만들 수 있는 프로그램이다.

```text
클릭 -> snap_point -> DrawState 누적 -> 확정 시 add_entity -> CommandHistory::execute
                                                                  (cad_core + cad_geometry 검증)
드래그 선택 -> hit_test -> screen_delta 누적 -> MoveEntity
Ctrl+Z / Ctrl+Y -> CommandHistory::undo / redo
```

Phase 1(검증), 2(기하 계산, 스냅), 4(command, undo/redo), 5(렌더링)가 전부 이번 Phase에서 실제 마우스/키보드 입력과 만났다.

---

## 15. 남은 과제

### 히트 테스트가 특징점 근처에서만 동작

도형의 테두리 아무 곳이나 클릭해서 선택할 수 없다 — 끝점/중점/중심 근처만 클릭해야 한다(4.2절). 점-도형 거리 계산이 `cad_geometry`에 추가되면 개선할 수 있다.

### 호 도구가 한쪽 방향만 그린다

3클릭 정의로는 반사(reflex) 방향의 호를 그릴 수 없다(3.2절). 필요해지면 네 번째 입력(방향 토글)을 추가한다.

### 한글 폰트가 Windows 전용

시스템 폰트 경로가 하드코딩되어 있다(10절). 크로스 플랫폼 지원 시 재검토가 필요하다.

### 텍스트 높이가 고정값이다

속성 편집이 없어 5mm 고정이다 — 8c에서 해결한다.

### 여전히 파일이 없다

새 프로젝트/열기/저장은 8d 그대로 남아 있다.

---

## 16. 다음 Phase

다음 단계는 8c — 패널이다.

### 8c 예정 범위

- 오른쪽 속성 패널: 선택된 엔티티의 값(좌표, 반지름, 텍스트 내용 등)을 command로 수정
- 왼쪽 레이어 패널: 가시성/잠금 토글을 실제 command로 연결 (지금은 읽기 전용 체크박스)
- 검증 결과 패널의 항목 클릭 시 해당 엔티티 선택 연동

---

## 마무리

Phase 8b는 코드로 예정했던 범위(도구, 선택, 이동, undo/redo)를 그대로 완성했지만, 정작 이 문서에서 가장 길게 다룬 문제(한글 폰트)는 계획에 없던 것이었다. `cargo test`와 clippy를 전부 통과한 코드가 실제로는 화면에 아무 글자도 제대로 보여주지 못하고 있었다는 사실은, 8a 문서에서 남겼던 경고("제대로 보인다는 사람이 눈으로 봐야 아는 것")가 실제로 일어난 사례였다. 사용자가 직접 실행해서 알려주지 않았다면 이 문제는 한동안 발견되지 않았을 것이다.

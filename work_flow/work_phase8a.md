# Rust로 CAD 엔진 만들기 — Phase 8a: `cad_app` 뼈대와 캔버스 렌더링

## 들어가며

Phase 7에서 `cad_cli`를 완성하면서 `01_제품_기획_정책.md`의 "GUI와 CLI에서 동일한 엔진 재사용"이 처음으로 두 번째 진입점을 통해 증명됐다. 하지만 CLI는 어디까지나 headless 도구다. 실제로 마우스로 도형을 그리고, 눈으로 도면을 보고, 화면에서 직접 조작하는 것 — CAD 프로그램의 본체는 결국 GUI다.

Phase 8은 이 프로젝트에서 가장 큰 단일 Phase가 될 것이 분명했다. 그래서 한 번에 밀어붙이는 대신 네 단계로 쪼갰다.

```text
8a. 뼈대       레이아웃 + cad_render 연결 + pan/zoom             ← 이번 문서
8b. 도구 시스템 선택/그리기 도구 + command 발행 + undo/redo
8c. 패널       속성 편집, 레이어 조작, 검증 결과 표시
8d. 파일/치수  새 프로젝트·열기·저장, 치수선 배치 규칙 확정
```

이 문서는 8a — "지금까지 만든 6개 계층을 실제로 화면에 띄워본다" — 를 다룬다.

---

## 1. Phase 8a의 목적

`04_UI_와이어프레임_정책.md`가 정의한 기본 화면 구조를 실제 창으로 옮기고, `cad_render`가 만드는 `RenderPrimitive`를 캔버스에 그리는 것이 이번 단계의 전부다. 도구로 도형을 그리거나 속성을 편집하는 기능은 아직 없다 — 오직 "지금까지 쌓아온 파이프라인이 실제로 화면까지 이어지는가"만 확인한다.

```text
04_UI_와이어프레임_정책.md의 6분할 레이아웃
┌─────────────────────────────────────────────────────────┐
│ 메뉴                 도구 모음                 프로젝트 │
├──────────────┬──────────────────────────────┬───────────┤
│ 도구/레이어  │          도면 캔버스          │ 속성/검증 │
├──────────────┴──────────────────────────────┴───────────┤
│ 명령 입력 / 프롬프트                         상태 표시 │
└─────────────────────────────────────────────────────────┘
```

---

## 2. 설계 결정 사항 — GUI 의존성 재확인

`cad_app/src/main.rs`에는 Phase 0 이전부터 다음 한 줄이 남아 있었다.

```rust
println!("CAD Studio application scaffold: GUI dependency pending approval");
```

workspace `Cargo.toml`에는 이미 `egui = "0.29"`, `eframe = { version = "0.29", .. }`가 고정되어 있었지만, 이 메모는 "버전은 정해졌어도 실제로 연결하는 시점에 한 번 더 확인하라"는 의도로 읽혔다. `05_사용자_승인_정책.md`가 "GUI framework, rendering backend 교체"를 승인 대상으로 명시하고 있기도 해서, 코드를 쓰기 전에 확인부터 했다.

**결정: 진행.** egui/eframe 0.29를 `cad_app`에 연결하고 8a 구현을 시작했다. `02_아키텍처_정책.md`에도 이 결정을 문서화했다.

```diff
 cad_app -> cad_command -> cad_core
+cad_app -> cad_render -> cad_core
 cad_command -> cad_geometry
```

```text
## GUI framework

`cad_app`은 `egui`/`eframe`(workspace 고정 버전)을 사용한다. rendering backend 교체는
여전히 승인 대상이다.
```

---

## 3. `cad_app`을 세 모듈로 나누기

```text
src/
├─ main.rs    진입점, eframe::run_native 호출만
├─ camera.rs  pan/zoom 순수 수학 (egui 타입 없음)
├─ demo.rs    데모 프로젝트 시드
└─ app.rs     eframe::App 구현, 레이아웃, 캔버스 렌더링
```

> 💡 **왜 `camera.rs`는 `egui`를 import하지 않는가**
>
> `Camera`는 두 튜플(`(f32, f32)`)만 다루는 순수 함수 모음이다. `egui::Pos2` 같은 GUI 타입을
> 전혀 쓰지 않았다. 덕분에 `cargo test`가 실제 창을 띄우거나 렌더링 컨텍스트를 만들지 않고도
> pan/zoom 수학을 검증할 수 있다 — Phase 2에서 `cad_geometry`를 도메인 로직과 분리했던 것과
> 같은 이유다. GUI 프레임워크가 나중에 바뀌더라도 이 파일은 거의 그대로 남는다.

---

## 4. Camera — 좌표계 변환과 커서 방향 줌

### 4.1 두 좌표계

```text
World (cad_core)      millimetre 단위, Y축 위로 증가 (수학 표준)
Screen (egui)          pixel 단위, Y축 아래로 증가
```

```rust
pub fn world_to_screen(&self, canvas_center: (f32, f32), world: (f64, f64)) -> (f32, f32) {
    (
        canvas_center.0 + self.offset.0 + (world.0 as f32) * self.zoom,
        canvas_center.1 + self.offset.1 - (world.1 as f32) * self.zoom,  // Y 반전
    )
}
```

Y를 반전하는 이유는 `cad_io`의 SVG 출력(Phase 6)과 동일하다 — 그대로 그리면 도면이 상하로 뒤집힌다.

### 4.2 커서를 향한 줌

스크롤로 확대/축소할 때, 화면 중앙이 아니라 **마우스 포인터가 가리키는 지점을 고정한 채** 확대되어야 자연스럽다. 이를 위해 확대 전후로 "포인터 아래 있던 월드 좌표"가 같은 화면 위치에 남도록 오프셋을 보정한다.

```rust
pub fn zoom_at(&mut self, canvas_center: (f32, f32), anchor_screen: (f32, f32), factor: f32) {
    let world_before = self.screen_to_world(canvas_center, anchor_screen);
    self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    let screen_after = self.world_to_screen(canvas_center, world_before);
    self.offset.0 += anchor_screen.0 - screen_after.0;
    self.offset.1 += anchor_screen.1 - screen_after.1;
}
```

순서가 중요하다.

```text
1. 지금 줌 값으로 "포인터 아래 월드 좌표"를 구한다
2. 줌 값을 바꾼다
3. 새 줌 값으로 그 월드 좌표가 어디에 그려지는지 다시 구한다
4. 1번 결과와 3번 결과의 차이만큼 오프셋을 보정한다
```

`zoom_at_keeps_the_anchor_point_fixed_on_screen` 테스트가 이 불변 조건(줌 전후로 포인터 아래 월드 좌표가 같다)을 직접 검증한다.

---

## 5. 데모 프로젝트 — 아직 파일을 열 수 없어서

Phase 8d 이전까지는 "새 프로젝트"도 "열기"도 없다. 그런데 캔버스가 뭔가를 그리는지 확인하려면 도면 데이터가 있어야 한다. `seed_demo_project()`가 6종 도형과 치수 하나, 그리고 **숨긴 레이어**를 포함한 프로젝트를 코드로 직접 만든다.

```rust
let hidden_layer = LayerId::new(2);
let mut hidden = Layer::new(hidden_layer, "hidden-demo");
hidden.visible = false;
```

숨긴 레이어를 일부러 넣은 이유는, Phase 5에서 만든 "숨긴 레이어 제외" 로직(`cad_render::build_render_model`)이 GUI 안에서도 실제로 작동하는지 눈으로 확인하기 위해서다 — 캔버스에 원이 하나 안 보인다면 그 레이어 필터링이 살아있다는 뜻이다.

`seed_demo_project_is_internally_valid` 테스트는 이 데모 데이터가 `cad_core::Drawing::validate()`를 실제로 통과하는지 확인한다 — Phase 4의 command 검증 로직이 지금 당장은 연결되어 있지 않지만(8b에서 연결), 최소한 데이터 자체는 처음부터 유효하게 만들었다.

> ⚠️ **`seed_demo_project`는 임시 코드다**
>
> 이 함수는 실제 기능이 아니라 8a~8c 개발 중 화면을 채우기 위한 장치다. Phase 8d에서
> "새 프로젝트"/"열기"가 생기면 이 함수는 제거되거나 테스트 전용으로만 남을 것이다.

---

## 6. 6분할 레이아웃

`eframe::App::update`는 매 프레임 호출된다. 여기서 여섯 영역을 순서대로 그린다.

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    self.menu_bar(ctx);
    Self::tool_bar(ctx);
    self.left_panel(ctx);
    self.right_panel(ctx);
    let status = self.canvas(ctx);
    self.status_bar(ctx, &status);
}
```

`egui`는 영역을 도킹 패널(`TopBottomPanel`, `SidePanel`, `CentralPanel`)로 표현한다. 호출 순서가 곧 배치 순서다 — 위/아래/좌/우 패널을 먼저 예약하고 남은 공간을 `CentralPanel`(캔버스)이 채운다.

이번 단계에서 왼쪽(레이어)과 오른쪽(검증) 패널은 실제 데이터를 보여주지만 상호작용은 없다.

```rust
// 왼쪽: 레이어 목록은 보여주지만 체크박스는 비활성 상태 (8c에서 활성화)
ui.add_enabled(false, egui::Checkbox::new(&mut visible, &layer.name));

// 오른쪽: cad_core::Drawing::validate()를 그대로 호출해서 보여준다
let report = self.project.drawing.validate();
```

오른쪽 패널의 검증 결과는 8a임에도 이미 "가짜가 아니다" — `04_UI_와이어프레임_정책.md`의 "공차 계산 결과와 계산 trace를 숨기지 않는다"는 원칙을 처음부터 지키기 위해, 플레이스홀더 텍스트 대신 실제 `ValidationReport`를 그대로 연결했다.

---

## 7. 캔버스 — `RenderPrimitive`를 `Painter`로

`cad_render::build_render_model`이 반환하는 7종 `RenderPrimitive`(6종 도형 + Dimension)를 각각 `egui::Painter`의 그리기 호출로 변환한다.

```rust
match primitive {
    RenderPrimitive::Line { start, end } => {
        painter.line_segment([to_screen(*start), to_screen(*end)], stroke);
    }
    RenderPrimitive::Circle { center, radius } => {
        painter.circle_stroke(to_screen(*center), (*radius as f32) * camera.zoom, stroke);
    }
    // ...
}
```

`Arc`는 `egui`에 전용 도형이 없어서, 시작각부터 끝각까지 32개 점을 샘플링해 선으로 연결했다 — Phase 6의 SVG 출력에서 arc를 `<path>`로 근사했던 것과 같은 방식이다.

```rust
const STEPS: u32 = 32;
let points: Vec<egui::Pos2> = (0..=STEPS)
    .map(|i| {
        let t = start_angle + sweep_angle * (f64::from(i) / f64::from(STEPS));
        to_screen((center.0 + radius * t.cos(), center.1 + radius * t.sin()))
    })
    .collect();
```

`Dimension`도 Phase 5·6과 같은 결정을 그대로 따라 시작점-끝점을 잇는 단순한 선(다른 색상)으로만 표시한다 — 실제 치수선 레이아웃은 8d에서 확정한다.

---

## 8. Clippy가 잡아낸 세 가지

이번에도 처음 작성한 코드가 컴파일은 됐지만 clippy(pedantic 포함)에서 세 가지가 걸렸다.

### `unused_self`

```rust
// 수정 전
fn tool_bar(&mut self, ctx: &egui::Context) { .. }   // self를 안 씀

// 수정 후
fn tool_bar(ctx: &egui::Context) { .. }               // 연관 함수로
```

도구 모음이 아직 비활성 버튼만 그리기 때문에 `&mut self`가 필요 없었다 — 8b에서 도구 클릭이 실제로 상태를 바꾸게 되면 다시 메서드로 돌아갈 가능성이 높다.

### `cast_possible_truncation`

`f64`(월드) → `f32`(화면) 변환이 파일 전체에 반복된다. 개별 캐스팅마다 `#[allow]`를 붙이는 대신, 모듈 상단에 한 번만 명시했다.

```rust
//! World coordinates are `f64` ...; screen coordinates are `f32` ...
//! The narrowing casts between them are intentional ...
#![allow(clippy::cast_possible_truncation)]
```

> 📌 **왜 개별 `#[allow]`가 아니라 모듈 단위로 껐는가**
>
> 이 변환은 버그가 아니라 이 모듈의 존재 이유(월드 좌표를 화면 좌표로 바꾸는 것) 그 자체다.
> 같은 이유로 반복되는 경고를 코드 전체에 흩뿌리는 것보다, 모듈 상단에 "왜 안전한지"를
> 한 번 설명하고 범위를 명확히 제한하는 편이 읽기 좋다고 판단했다.

### `float_cmp`

```rust
// 수정 전
assert_eq!(camera.zoom, MIN_ZOOM);

// 수정 후
assert!((camera.zoom - MIN_ZOOM).abs() < f32::EPSILON);
```

Phase 3에서 겪었던 것과 같은 종류의 지적이다 — `.clamp()`가 만든 값이라 사실상 정확히 같겠지만, "부동소수점은 직접 비교하지 않는다"는 원칙을 테스트 코드에도 예외 없이 적용했다.

---

## 9. 실제로 확인한 것과 확인하지 못한 것

바이너리를 빌드해 3초간 백그라운드로 실행한 뒤, 프로세스가 살아있는지(=패닉 없이 창이 뜬 상태인지)와 stderr에 에러가 없는지를 확인했다.

```text
started pid ...
STILL_RUNNING
---stderr---
(empty)
```

패닉이나 에러 로그 없이 프로세스가 계속 살아 있었다는 것은 창이 정상적으로 열렸다는 강한 신호다. 하지만 이 확인 방법으로는 **레이아웃이 눈으로 보기에 올바른지, 도형이 실제로 올바른 위치에 그려지는지는 알 수 없다.**

> ⚠️ **화면을 직접 보지 못했다는 한계**
>
> 이 프로젝트를 진행하는 동안 AI 에이전트는 사용자의 화면을 볼 수 없다. 프로세스 생존 여부와
> 로그만으로 "크래시하지 않는다"는 확인할 수 있지만 "제대로 그려진다"는 확인할 수 없다.
> 이 문서를 작성한 시점 기준으로, 실제 렌더링 결과에 대한 시각적 확인은 사용자에게 요청한
> 상태다. 8b로 넘어가기 전 이 부분의 피드백을 반영해야 한다.

---

## 10. 테스트

### `camera.rs` (6개)

| 테스트 | 검증 내용 |
|---|---|
| `default_camera_centers_the_origin_at_canvas_center` | 기본 카메라에서 원점이 캔버스 중앙에 오는지 |
| `world_to_screen_flips_y` | Y축이 실제로 반전되는지 |
| `screen_to_world_is_the_inverse_of_world_to_screen` | 왕복 변환이 원래 값으로 돌아오는지 |
| `pan_shifts_the_offset` | 팬이 오프셋을 그대로 이동시키는지 |
| `zoom_at_keeps_the_anchor_point_fixed_on_screen` | 커서 방향 줌의 핵심 불변 조건 |
| `zoom_is_clamped_to_the_allowed_range` | 줌이 min/max를 벗어나지 않는지 |

### `demo.rs` (3개)

| 테스트 | 검증 내용 |
|---|---|
| `seed_demo_project_is_internally_valid` | 데모 데이터가 `cad_core`의 검증을 통과하는지 |
| `seed_demo_project_includes_a_hidden_layer_entity` | 숨긴 레이어 테스트용 엔티티가 실제로 있는지 |
| `seed_demo_project_covers_every_geometry_variant` | 6종 도형이 전부 포함되는지 |

---

## 11. 검증 결과

```text
cargo fmt --all -- --check
통과

cargo test --workspace
전체 통과
- cad_app: 9/9 (신규)
- workspace total: 126

cargo clippy --workspace --all-targets -- -D warnings
경고 없음 (unused_self, cast_possible_truncation, float_cmp 수정 후 통과)

바이너리 실행: 3초간 패닉/에러 없이 프로세스 생존 확인 (시각적 검증은 사용자 확인 대기)
```

---

## 12. Phase 8a에서 사용한 개념

### GUI 상태와 순수 로직의 분리

`Camera`를 `egui` 타입 없이 튜플만으로 구현해, GUI 프레임워크 없이도 단위 테스트가 가능하게 했다. Phase 2(`cad_geometry`)에서 이미 썼던 전략을 GUI 계층에도 그대로 적용한 것이다.

### 프레임 기반 즉시 모드 GUI

`egui`는 매 프레임 전체 UI를 다시 선언하는 즉시 모드(immediate mode) 방식이다. `update()`가 호출될 때마다 `self.project.drawing.validate()`를 다시 실행해 검증 패널을 그리는 것도 이 모델을 따른 것이다 — 상태를 캐싱하지 않고 매번 다시 계산해도 이 정도 규모에서는 비용이 무시할 만하다.

### 모듈 단위 lint 허용

파일 전체에 걸쳐 의도적으로 반복되는 패턴에 대해 `#![allow(...)]`를 모듈 상단에 한 번만 선언해, 개별 지점에 흩어진 `#[allow]` 없이도 clippy와 공존했다.

### 화면-월드 좌표 변환의 왕복 불변 조건

`screen_to_world(world_to_screen(x)) == x`, `zoom_at` 이후에도 앵커 포인트가 고정된다는 불변 조건을 테스트로 명시적으로 검증했다 — GUI 코드라 해도 순수 수학 부분은 여전히 도메인 로직처럼 테스트할 수 있다는 것을 보여준다.

---

## 13. Phase 8a 완료 결과

```text
Project (지금은 demo.rs가 만든 하드코딩 데이터)
    ↓ cad_render::build_render_model   ← Phase 5
    ↓ egui::Painter                     ← 이번 Phase
실제 화면
```

`02_아키텍처_정책.md`가 그린 전체 흐름의 마지막 구간("렌더 모델 -> 화면")이 이번에 실제로 이어졌다. 지금까지 문서로만 존재하던 `04_UI_와이어프레임_정책.md`의 레이아웃도 처음으로 코드가 됐다.

---

## 14. 남은 과제

### 도구가 아직 아무것도 하지 않는다

도구 모음의 버튼은 전부 비활성 상태다. 클릭해도 아무 일도 일어나지 않는다 — 8b의 범위다.

### 레이어 체크박스가 읽기 전용이다

레이어 가시성을 눈으로 볼 수는 있지만 토글할 수 없다. `Layer::visible`을 바꾸려면 command를 거쳐야 하는데(`04_UI_와이어프레임_정책.md`: "속성 패널에서 값을 수정할 때도 직접 필드 변경을 하지 않고 command를 발생"), 이 배선은 8c에서 만든다.

### undo/redo가 연결되지 않았다

`cad_command::CommandHistory`는 Phase 4에서 이미 완성되어 있지만, `cad_app`은 아직 어떤 command도 실행하지 않는다 — 애초에 사용자가 도면을 바꿀 방법이 없기 때문이다. 8b에서 도구가 command를 발행하기 시작하면 undo/redo 버튼도 함께 활성화한다.

### 파일이 없다

앞서 언급했듯 새 프로젝트도, 열기도, 저장도 없다. `cad_io`는 Phase 6에서 이미 완성되어 있지만 아직 `cad_app`과 연결되지 않았다 — 8d의 범위다.

### 시각적 검증 대기 중

10번 절에서 설명한 대로, 실제 렌더링이 의도한 대로 보이는지는 사용자 확인이 필요하다.

---

## 15. 다음 Phase

다음 단계는 8b — 도구 시스템이다.

### 8b 예정 범위

- 선택 도구: 클릭으로 엔티티 선택 (도메인 상태 변경 없음, UI 상태만)
- 그리기 도구: 선/사각형/원/호/텍스트 — 클릭·드래그 입력을 수집해 `DrawingCommand::AddEntity`로 발행
- 이동 도구: 선택된 엔티티를 드래그하면 `DrawingCommand::MoveEntity` 발행
- undo/redo 버튼과 단축키를 `CommandHistory`에 연결
- `cad_geometry::snap_candidates`/`nearest_point`를 포인터 입력에 연결 (Phase 2에서 예고했던 지점)

`04_UI_와이어프레임_정책.md`의 상호작용 흐름을 그대로 구현하는 단계다.

```text
도구 선택 -> 입력 수집 -> 미리보기 -> 확정 command -> 검증 -> 렌더 갱신 -> 자동 저장 예약
```

("자동 저장 예약"은 `cad_io`가 연결되는 8d까지는 no-op으로 남을 것이다.)

---

## 마무리

Phase 8a는 새로운 도메인 개념을 하나도 추가하지 않았다 — `Camera`조차 CAD 도메인이 아니라 화면 표시를 위한 보조 개념이다. 그런데도 이 Phase가 중요했던 이유는, Phase 1부터 7까지 각자 독립적으로 검증해온 계층들이 실제로 하나의 화면 위에서 만나는 것을 처음으로 확인했기 때문이다.

동시에 이번 Phase는 이 프로젝트의 검증 방식이 가진 한계를 가장 분명하게 드러낸 지점이기도 하다. `cargo test`와 프로세스 생존 확인만으로는 "코드가 죽지 않는다"는 것까지만 알 수 있다. "제대로 보인다"는 사람이 눈으로 봐야 아는 것이고, 8b로 넘어가기 전에 그 확인이 필요하다.

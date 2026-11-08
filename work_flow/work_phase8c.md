# Rust로 CAD 엔진 만들기 — Phase 8c: 속성 편집, 레이어 토글, 그리기 중 수치 표시

## 들어가며

Phase 8b가 끝난 시점의 `cad_app`은 도형을 그리고 선택하고 옮길 수 있었지만, 일단 그려진 도형은 그대로 고정이었다. 텍스트 높이는 항상 5mm, 원의 반지름은 처음 클릭한 그대로 — 값 하나를 바꾸려면 지우고 다시 그리는 수밖에 없었다. 레이어 패널의 체크박스도 눈으로 보기만 할 뿐 클릭해도 아무 일도 일어나지 않았다.

Phase 8c는 "그려진 뒤에도 조정할 수 있게" 만드는 단계다. 여기에 이번 Phase를 시작하며 사용자가 직접 요청한 것 하나를 더했다 — 그리기 도중 지금 만들고 있는 도형의 치수(길이, 폭, 반지름 등)를 숫자로 보여주는 기능이다.

---

## 1. Phase 8c의 목적

Phase 8b 문서의 "다음 Phase 예정 범위"에 다음 세 가지를 적어뒀다.

- 오른쪽 속성 패널: 선택된 엔티티의 값을 command로 수정
- 왼쪽 레이어 패널: 가시성/잠금 토글을 실제 command로 연결
- 검증 결과 패널의 항목 클릭 시 해당 엔티티 선택 연동

이번 Phase에서 앞의 두 가지는 완성했다. 세 번째(검증 항목 클릭 → 선택 연동)는 시간 관계상 이번 범위에서 빠졌다 — 8절과 다음 Phase 목록에 남겨둔다.

추가로, 작업을 시작하며 사용자가 "선/직사각형 등을 그릴 때 수치가 보였으면 좋겠다"고 요청했다. `04_UI_와이어프레임_정책.md`의 "좌표와 단위를 항상 확인할 수 있어야 한다"는 원칙과 정확히 일치하는 요청이라, 계획에 없었지만 이번 Phase에 포함했다.

---

## 2. 그리기 중 수치 표시

### 2.1 각도 감음(wrap-around) 로직을 공유 함수로 뽑아내기

호의 스윕 각도를 "반시계로 감아서 항상 양수로 만드는" 로직(Phase 8b)이 `arc_geometry`에 있었다. 그리기 중 미리보기에서도 같은 계산(반지름, 각도)이 필요했으므로, 계산 로직을 별도 함수로 뽑아 재사용했다.

```rust
fn ccw_sweep(start_angle: f64, end_angle: f64) -> f64 {
    let mut sweep = end_angle - start_angle;
    while sweep <= 0.0 {
        sweep += std::f64::consts::TAU;
    }
    sweep
}
```

`arc_geometry`(확정된 도형을 만드는 함수)와 `preview_measurement`(아직 확정되지 않은 도형의 수치만 계산하는 함수) 둘 다 이 함수를 쓴다 — 도형이 실제로 만들어지기 전에 보여주는 숫자와, 확정될 때 실제로 저장되는 각도가 같은 계산에서 나온다는 것이 보장된다.

### 2.2 도구별 수치

```rust
pub fn preview_measurement(draw_state: &DrawState, hover: (f64, f64)) -> Option<String> {
    match draw_state {
        DrawState::LineStart(start) => Some(format!("길이: {:.2} mm", dist(*start, hover))),
        DrawState::RectangleStart(start) => Some(format!(
            "폭: {:.2} mm, 높이: {:.2} mm",
            (hover.0 - start.0).abs(), (hover.1 - start.1).abs()
        )),
        DrawState::CircleCenter(center) | DrawState::ArcCenter(center) => {
            Some(format!("반지름: {:.2} mm", dist(*center, hover)))
        }
        DrawState::ArcStart { center, start } => {
            let sweep = ccw_sweep(/* ... */);
            Some(format!("반지름: {:.2} mm, 각도: {:.1}°", dist(*center, *start), sweep.to_degrees()))
        }
        _ => None,
    }
}
```

`DrawState`가 `Idle`이거나(아직 아무것도 그리는 중이 아님) 선택 도구일 때는 `None`을 반환해 아무것도 표시하지 않는다. 이 함수는 `tool.rs`의 다른 함수들과 마찬가지로 `egui` 없이 순수하게 동작해, 5개 케이스 전부(선/사각형/원/호 1단계/호 2단계) 단위 테스트로 검증했다.

### 2.3 캔버스에 표시

스냅된 커서 위치 옆(오른쪽 위로 살짝 띄운 자리)에 노란색 텍스트로 그린다.

```rust
if let Some(snapped) = snapped_hover {
    if let Some(text) = tool::preview_measurement(&self.draw_state, snapped) {
        let (x, y) = self.camera.world_to_screen(center, snapped);
        painter.text(egui::pos2(x + 14.0, y - 14.0), egui::Align2::LEFT_BOTTOM, text, .., ..);
    }
}
```

스냅 마커(Phase 8b, 노란 원)와 같은 조건문 블록 안에 있어서, 두 힌트(스냅 위치 + 치수)가 그리기 도구를 쓰는 동안 항상 함께 보인다.

---

## 3. `cad_command`에 세 개의 command 추가

속성 편집과 레이어 토글 모두 "기존 값을 다른 값으로 통째로 바꾼다"는 점에서 Phase 4의 `SetEntityLayer`와 똑같은 모양이다. 그 패턴을 그대로 반복했다.

```rust
SetEntityGeometry { id: EntityId, geometry: EntityGeometry },  // 속성 패널 전체 교체
SetLayerVisible { id: LayerId, visible: bool },                 // 레이어 가시성
SetLayerLocked { id: LayerId, locked: bool },                   // 레이어 잠금
```

세 command 모두 `apply`/`inverse` 양쪽에 대칭적으로 추가했다 — `inverse`는 command를 적용하기 **전**의 값을 읽어서 되돌릴 command를 만든다(Phase 4에서 정립한 패턴).

```rust
DrawingCommand::SetEntityGeometry { id, .. } => {
    let current_geometry = /* id로 엔티티를 찾아 지금 geometry를 복제 */;
    DrawingCommand::SetEntityGeometry { id: *id, geometry: current_geometry }
}
```

이 세 command는 새로 만든 `find_layer_mut` 헬퍼(`find_entity_mut`와 대칭)를 함께 썼다는 것 말고는 완전히 기존 패턴의 반복이다. 특히 `SetEntityGeometry`는 이미 Phase 4에서 만들어 둔 `validate_drawing`(구조 검증 + `cad_geometry` 도형 검증)을 그대로 통과한다 — 속성 패널에서 원의 반지름을 0 이하로 바꾸려고 하면, 다른 command와 똑같이 거부되고 자동으로 되돌려진다. 검증 로직을 새로 만들 필요가 전혀 없었다.

---

## 4. 속성 패널 — 세 번째로 반복된 "초안 후 커밋" 패턴

### 4.1 왜 또 이 패턴인가

Phase 8b의 드래그 이동에서 "프레임마다 command를 실행하면 undo 스택이 의미 없이 커진다"는 이유로 화면 좌표 누적 방식을 썼다. 속성 패널의 `DragValue`도 마우스를 누른 채 끄는 동안 프레임마다 값이 바뀌는 위젯이라 똑같은 문제가 생긴다. 그래서 같은 해법을 다시 썼다.

```rust
struct PropertyEdit {
    entity_id: EntityId,
    draft: EntityGeometry,  // 편집 중인 사본
    editing: bool,          // 지금 드래그/타이핑 중인가
}
```

각 숫자 필드는 매 프레임 `draft`를 갱신하지만, `SetEntityGeometry`는 상호작용이 끝나는 순간(`drag_stopped` 또는 `lost_focus`)에만 실행된다.

```rust
fn number_field(ui: &mut egui::Ui, outcome: &mut FieldEditOutcome, label: &str, value: &mut f64) {
    let response = ui.add(egui::DragValue::new(value).speed(0.5).suffix(" mm"));
    if response.dragged() || response.has_focus() { outcome.editing = true; }
    if response.drag_stopped() || response.lost_focus() { outcome.commit = true; }
}
```

### 4.2 선택이 바뀌었을 때만 초안을 새로 만들기

선택된 엔티티가 바뀌면 `draft`를 새로 복제해야 하지만, **지금 편집 중인 필드가 있다면 덮어쓰면 안 된다** — 그러면 타이핑 도중 값이 사라진다. `sync_property_edit`이 이 조건을 정리한다.

```rust
fn sync_property_edit(&mut self) {
    let Some(id) = self.selection else { self.property_edit = None; return };

    let currently_editing = self.property_edit.as_ref()
        .is_some_and(|edit| edit.entity_id == id && edit.editing);
    if currently_editing {
        return; // 진행 중인 편집을 덮어쓰지 않는다
    }

    self.property_edit = /* 현재 프로젝트의 entity.geometry를 복제해 새 draft로 */;
}
```

편집 중이 아닐 때는 매 프레임 실행돼도 상관없다 — 어차피 `draft`가 이미 프로젝트의 실제 값과 같기 때문에, 다시 복제해도 아무것도 달라지지 않는다. `editing`이 `false`가 되는 순간(=상호작용이 끝난 순간)에만 실질적으로 "다시 동기화"가 의미를 가진다.

> ⚠️ **놓친 경우 하나**
>
> 만약 사용자가 캔버스에서 같은 엔티티를 드래그로 이동하는 동안 속성 패널이 열려 있다면,
> 속성 패널의 숫자는 이동이 끝나기 전까지 갱신되지 않는다(`editing`이 캔버스 드래그와는
> 별개 상태이기 때문). 실제로 두 조작을 동시에 하는 경우는 드물 것이라 판단해 이번
> 범위에서는 감수하기로 했다 — 남은 과제에 기록한다.

### 4.3 도형별 필드

| 도형 | 필드 |
|---|---|
| Line | 시작 x/y, 끝 x/y |
| Rectangle | 원점 x/y, 폭, 높이 |
| Circle | 중심 x/y, 반지름 |
| Arc | 중심 x/y, 반지름, 시작각(°), 스윕각(°) |
| Text | 내용, 원점 x/y, 높이 |
| Polyline | (아직 미지원 — "폴리라인 속성 편집은 아직 지원하지 않습니다") |

Arc의 각도 필드는 라디안(`cad_core`의 내부 표현)이 아니라 도(degree)로 보여준다 — 사용자가 라디안으로 각도를 입력하는 것은 비현실적이다. 위젯에는 지역 변수로 변환한 값만 전달하고, 편집이 끝나면 다시 라디안으로 변환해 `arc.start_angle`/`sweep_angle`에 써넣는다.

```rust
fn angle_field_degrees(ui: &mut egui::Ui, outcome: &mut FieldEditOutcome, label: &str, angle: &mut AngleRad) {
    let mut degrees = angle.0.to_degrees();
    // ... DragValue(&mut degrees) ...
    angle.0 = degrees.to_radians();
}
```

Polyline을 지원하지 않는 이유는 단순하다 — 그리기 도구 목록(`Tool::DRAWING_TOOLS`)에 애초에 Polyline이 없어서, 지금 이 앱으로는 Polyline 엔티티를 만들 방법 자체가 없다. 존재할 수 없는 것을 편집하는 UI를 만들 필요는 없었다.

---

## 5. borrow 문제를 또 같은 방법으로 풀기

`right_panel`도 Phase 8b의 텍스트 입력 팝업과 같은 문제에 부딪혔다 — 패널을 그리는 동안(`&mut self`가 필요 없는 순수 읽기/렌더링) `self.execute(...)`(`&mut self` 필요)를 호출할 수 없다.

이번에도 같은 해법을 썼다: 필요한 데이터를 전부 지역 변수로 먼저 꺼내고, 패널 클로저는 그 지역 변수만 참조하게 하고, 클로저가 끝난 뒤에 결과를 보고 `self`를 다시 빌려 command를 실행한다.

```rust
fn right_panel(&mut self, ctx: &egui::Context) {
    self.sync_property_edit();

    let report = self.project.drawing.validate();          // 소유권 있는 값으로 복사
    let status_message = self.status_message.clone();
    let selected = /* (EntityId, 종류 라벨) 튜플로 미리 추출 */;
    let mut draft = self.property_edit.as_ref().map(|e| e.draft.clone());
    let mut outcome = FieldEditOutcome::default();

    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        // 여기서는 지역 변수(report, selected, draft)만 사용 — self를 안 건드림
        outcome = edit_geometry_fields(ui, geometry);
    });

    // 클로저가 끝난 뒤에만 self를 다시 빌린다
    if outcome.commit {
        self.execute(DrawingCommand::SetEntityGeometry { id, geometry });
    }
}
```

`left_panel`의 레이어 토글도 동일한 구조다 — 체크박스 클릭 결과를 `Option<(LayerId, bool)>` 지역 변수에 잠깐 담아뒀다가, 패널이 다 그려진 뒤에 `self.execute`를 호출한다. 이걸로 Phase 8b부터 세 번째 반복되는 패턴이 됐다 — "즉시 모드 GUI 클로저 안에서 `&mut self` 전체가 필요한 작업은 하지 않는다"는 규칙이 이 프로젝트의 GUI 코드 전반에 자리잡은 셈이다.

---

## 6. 테스트

`tool.rs`에 5개(`preview_measurement`), `cad_command`에 3개(`SetEntityGeometry` 2개 + `SetLayerVisible`/`SetLayerLocked` 1개)를 추가했다.

### `tool.rs`

| 테스트 | 검증 내용 |
|---|---|
| `preview_measurement_reports_line_length` | 선 길이 |
| `preview_measurement_reports_rectangle_width_and_height` | 사각형 폭/높이 |
| `preview_measurement_reports_circle_radius` | 원 반지름 |
| `preview_measurement_reports_arc_radius_and_angle` | 호 반지름 + 각도 |
| `preview_measurement_is_none_when_idle` | 대기 상태에서는 표시 안 함 |

### `cad_command`

| 테스트 | 검증 내용 |
|---|---|
| `set_entity_geometry_replaces_geometry_and_can_be_undone` | 속성 교체 + undo |
| `set_entity_geometry_rejects_invalid_result_and_rolls_back` | 잘못된 값(퇴화 선분)이 거부되고 롤백되는지 |
| `set_layer_visible_and_locked_can_be_undone` | 레이어 토글 + undo |

---

## 7. 검증 결과

```text
cargo fmt --all -- --check
통과

cargo test --workspace
전체 통과
- cad_app: 25/25 (+5)
- cad_command: 11/11 (+3)
- workspace total: 115

cargo clippy --workspace --all-targets -- -D warnings
경고 없음 (첫 시도에 통과)

바이너리 실행 및 사용자 직접 확인 완료
```

---

## 8. Phase 8c에서 사용한 개념

### 계산 로직 공유로 일관성 보장

미리보기 숫자와 실제 확정되는 도형이 같은 함수(`ccw_sweep`)에서 나오게 해서, "미리보기에는 91°로 보였는데 확정하니 89°가 됐다" 같은 불일치를 애초에 불가능하게 만들었다.

### `Option<(A, B)>`로 여러 값을 함께 존재/부재시키기

`selected: Option<(EntityId, &'static str)>`처럼 관련된 두 값을 튜플로 묶어 `Option`에 넣으면, "선택된 게 없다"는 상태가 두 값 중 하나만 없는 불가능한 상태를 만들지 않는다.

### 같은 패턴의 세 번째 반복

Phase 8b의 드래그 이동, 텍스트 팝업에 이어 속성 패널까지, "즉시 모드 GUI에서 `&mut self` 충돌을 피하는 법"이 이번에는 고민 없이 바로 적용됐다 — 앞서 두 번 겪은 문제라 세 번째는 설계 단계에서부터 그 형태로 코드를 짰다.

---

## 9. Phase 8c 완료 결과

```text
그리기 도구 사용 중
    ↓ preview_measurement
캔버스에 실시간 치수 표시

선택 도구로 엔티티 선택
    ↓ sync_property_edit
오른쪽 패널에 편집 가능한 필드
    ↓ 상호작용 종료 시
SetEntityGeometry (undo 가능)

왼쪽 패널 체크박스 클릭
    ↓
SetLayerVisible / SetLayerLocked (undo 가능)
```

Phase 8a에서 6분할 레이아웃의 뼈대만 세웠던 왼쪽·오른쪽 패널이, 이번 Phase로 전부 실제 기능을 가지게 됐다.

---

## 10. 남은 과제

### 검증 항목 클릭 → 선택 연동 (8c에서 빠짐)

Phase 8b 문서가 8c의 범위로 예고했지만 이번에 구현하지 못했다. 오른쪽 패널의 검증 오류 목록에서 항목을 클릭하면 해당 엔티티가 선택되는 기능은 다음 작업으로 넘긴다.

### 캔버스 드래그와 속성 패널 편집이 동시에 일어나는 경우

4.2절에서 설명한 대로, 캔버스에서 드래그로 옮기는 동안 속성 패널의 숫자는 갱신되지 않는다. 실제로 문제가 되는지 사용 중 확인이 필요하다.

### Polyline 속성 편집 없음

애초에 Polyline을 그리는 도구가 없어서 미룬 것과 같은 이유다. Polyline 그리기 도구가 추가되면 함께 다룬다.

### 치수 도구가 여전히 비활성

Phase 5·6에서 미룬 "치수선 배치 규칙"이 아직 확정되지 않아, 도구 모음의 "치수" 버튼은 계속 비활성 상태로 남아 있다.

---

## 11. 다음 Phase

다음 단계는 8d다. 자세한 목록은 대화에서 별도로 정리했다.

---

## 마무리

Phase 8c는 계획된 두 가지(속성 패널, 레이어 토글) 외에 사용자가 작업 도중 요청한 기능(그리기 중 수치 표시)을 함께 포함시킨 첫 Phase였다. 계획에 없던 요청이었지만 기존에 이미 있던 계산 로직(`ccw_sweep`, 각 도형의 거리 계산)을 재사용하는 것만으로 구현할 수 있었다 — Phase 2부터 꾸준히 지켜온 "계산 로직을 한 곳에 모아 재사용한다"는 원칙이 계획에 없던 요구에도 쉽게 대응할 수 있게 해준 사례였다.

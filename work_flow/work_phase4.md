# Rust로 CAD 엔진 만들기 — Phase 4: `cad_command` 확장과 검증 통합

## 들어가며

Phase 1~3에서는 세 계층이 각자의 역할을 준비했다.

```text
cad_core       상태와 Stable ID, 구조적 Validation
cad_geometry   도형 계산과 도형별 Validation
cad_tolerance  공차 계산과 누적
```

문제는 이 세 계층이 아직 실제 "사용자 조작"으로 묶여 있지 않았다는 점이다. `cad_core`의 `Drawing`은 `add_entity`/`remove_entity` 같은 메서드를 직접 호출해야만 바뀌었고, `cad_geometry`의 validation 함수들은 어디에서도 호출되지 않았다.

`02_아키텍처_정책.md`는 모든 도면 변경이 다음 흐름을 따라야 한다고 규정한다.

```text
사용자 입력 -> UI 이벤트 -> DrawingCommand -> Core Model -> 검증 -> 렌더 모델 -> 화면/출력/저장
```

Phase 4는 이 흐름의 중심인 `DrawingCommand`와 `CommandHistory`를 확장해서, "명령을 실행하면 관련된 모든 계층의 검증을 통과한 상태만 남는다"는 것을 보장하는 단계다.

---

## 1. Phase 4의 목적

`02_아키텍처_정책.md`는 `cad_command`의 책임을 "실행, 검증, undo, redo, 재생"으로 정의한다. Phase 3 시점까지 구현된 것은 다음 두 가지뿐이었다.

- 실행 (`AddEntity`, `DeleteEntity`만 지원)
- undo / redo

빠져 있던 것은 다음 세 가지다.

- **검증** — command 실행 결과가 `cad_core`와 `cad_geometry`의 규칙을 모두 만족하는지 확인
- **더 많은 command** — Layer/Dimension을 다루는 command, 엔티티를 이동·수정하는 command
- **재생(replay)** — command 로그만으로 프로젝트를 처음부터 재구성

Phase 4의 목표는 이 세 가지를 채우는 것이다.

---

## 2. 설계 결정 사항 — `cad_command`가 `cad_geometry`에 의존해도 되는가

작업을 시작하기 전에 `02_아키텍처_정책.md`의 의존성 다이어그램을 다시 확인했다.

```text
cad_app -> cad_command -> cad_core
cad_cli -> cad_io -> cad_core
cad_batch -> cad_command / cad_io / cad_core
cad_render -> cad_core
cad_geometry -> cad_core
cad_tolerance -> cad_core
```

`cad_command -> cad_core`만 있고 `cad_command -> cad_geometry`는 없었다. 그런데 이번에 만들 것은 다음 두 가지다.

- 엔티티를 이동시키는 `MoveEntity` command → 좌표 이동 계산이 필요
- command 실행 후 도형이 유효한지 검사하는 로직 → `cad_geometry`의 `validate_line`/`validate_circle` 등이 필요

두 요구 모두 `cad_geometry`의 함수를 그대로 재사용하면 가장 간단하게 풀린다. 문제는 이것이 `02_아키텍처_정책.md`가 "변경 승인 대상"으로 명시한 **crate 구조**에 해당할 수 있다는 점이었다.

> ⚠️ **왜 그냥 추가하지 않았는가?**
>
> 의존성 다이어그램은 단순한 참고 그림이 아니라 정책 문서의 일부다.
> 문서에 없는 의존성을 임의로 추가하면, 이후 이 문서만 보고 판단하는 사람(또는 다른 AI 에이전트)이
> 실제 코드와 다른 그림을 보게 된다.
> `06_AI_개발_작업_명령서.md`의 "범위를 넓히는 기능은 먼저 제안하고 승인을 받는다"에 따라 먼저 확인했다.

### 결정

두 가지 선택지를 제시했다.

1. `cad_command`가 `cad_geometry`에 의존하도록 하여 이동/검증 로직을 재사용
2. `cad_core`만 사용하고, 이동 계산을 `cad_command` 안에 최소한으로 직접 구현 (geometry validation 통합은 보류)

**결정: 1번, `cad_geometry` 의존성을 추가한다.** 실제로 `crates/cad_command/Cargo.toml`에는 이미 `cad_geometry`가 등록되어 있었다 — 하지만 문서에는 반영되어 있지 않았으므로, 이번에 `02_아키텍처_정책.md`의 다이어그램에 `cad_command -> cad_geometry`를 추가해 코드와 문서를 일치시켰다.

```diff
 cad_app -> cad_command -> cad_core
+cad_command -> cad_geometry
 cad_cli -> cad_io -> cad_core
```

---

## 3. Phase 3까지의 `cad_command`

```rust
pub enum DrawingCommand {
    AddEntity(Entity),
    DeleteEntity { id: EntityId },
}

pub enum CommandError {
    Core(CoreError),
}

pub struct CommandHistory {
    undo: Vec<HistoryEntry>,
    redo: Vec<DrawingCommand>,
}
```

`execute`는 다음 순서로 동작했다.

```text
1. inverse(project, &command)  — 되돌릴 명령을 미리 계산
2. apply(project, &command)    — 실제로 적용
3. undo 스택에 기록, redo 스택 비움
```

이 구조에는 검증 단계가 없었다. `add_entity`가 `cad_core`의 중복 ID 검사만 통과하면, 그 엔티티가 존재하지 않는 레이어를 참조하든, 길이가 0인 선이든 상관없이 그대로 반영됐다.

---

## 4. `cad_core`에 `remove_layer` 추가하기

새 command 중 `RemoveLayer`를 만들려는데, `cad_core::Drawing`에는 `remove_layer`가 없었다.

```rust
// 이미 있던 것
pub fn add_entity(&mut self, entity: Entity) -> Result<(), CoreError>
pub fn remove_entity(&mut self, id: EntityId) -> Result<Entity, CoreError>
pub fn add_layer(&mut self, layer: Layer) -> Result<(), CoreError>
// remove_layer 없음
pub fn add_dimension(&mut self, dim: Dimension) -> Result<(), CoreError>
pub fn remove_dimension(&mut self, id: DimensionId) -> Result<Dimension, CoreError>
```

`Entity`와 `Dimension`은 추가/삭제가 대칭인데 `Layer`만 추가만 가능했다. 기존 패턴을 그대로 따라 추가했다.

```rust
pub fn remove_layer(&mut self, id: LayerId) -> Result<Layer, CoreError> {
    let index = self
        .layers
        .iter()
        .position(|l| l.id == id)
        .ok_or(CoreError::MissingLayer(id))?;
    Ok(self.layers.remove(index))
}
```

`CoreError::MissingLayer`는 Phase 1부터 이미 정의만 되어 있고 실제로 어디서도 만들어지지 않던 variant였다. 이번에 처음 사용됐다.

> 💡 **왜 이걸 "새 기능"이 아니라 "보완"으로 봤는가?**
>
> `Entity`/`Dimension`은 이미 추가·삭제가 모두 가능했다. `Layer`만 추가만 가능했던 것은
> 설계 의도라기보다 Phase 1에서 미처 채우지 못한 빈틈에 가까웠다.
> 기존 함수와 완전히 같은 시그니처·에러 패턴을 따랐으므로 새로운 설계 판단이 필요하지 않았고,
> 그래서 별도 승인 없이 진행했다.

---

## 5. `DrawingCommand` 확장

6개 variant를 추가해 총 8개가 됐다.

```rust
pub enum DrawingCommand {
    AddEntity(Entity),
    DeleteEntity { id: EntityId },
    MoveEntity { id: EntityId, dx: LengthMm, dy: LengthMm },
    SetEntityLayer { id: EntityId, layer_id: LayerId },
    AddLayer(Layer),
    RemoveLayer { id: LayerId },
    AddDimension(Dimension),
    RemoveDimension { id: DimensionId },
}
```

| Command | 대응하는 사용자 조작 |
|---|---|
| `MoveEntity` | 캔버스에서 엔티티를 드래그해서 이동 (`01_제품_기획_정책.md`의 MVP 도구: 이동) |
| `SetEntityLayer` | 속성 패널에서 엔티티의 레이어를 변경 (`04_UI_와이어프레임_정책.md`: 속성 패널 수정도 command로) |
| `AddLayer` / `RemoveLayer` | 레이어 패널에서 레이어 추가/삭제 |
| `AddDimension` / `RemoveDimension` | 치수 도구로 치수를 배치/삭제 |

---

## 6. `apply`/`inverse` 패턴 다시 보기 — 상태를 미리 읽어두기

기존 `DeleteEntity`의 inverse는 단순히 "반대 명령"을 만드는 게 아니라, **삭제되기 전의 엔티티 전체를 기억해서** `AddEntity`로 되돌릴 수 있게 만든다.

```rust
DrawingCommand::DeleteEntity { id } => DrawingCommand::AddEntity(
    project.drawing.entities.iter()
        .find(|entity| entity.id == *id)
        .cloned()
        .ok_or(CoreError::MissingEntity(*id))?,
),
```

이 패턴이 가능한 이유는 `execute`가 `inverse`를 **`apply`보다 먼저** 호출하기 때문이다.

```rust
pub fn execute(&mut self, project: &mut Project, command: DrawingCommand) -> Result<(), CommandError> {
    let inverse = inverse(project, &command)?; // 아직 적용 전 상태를 읽는다
    apply(project, &command)?;                  // 이제 적용한다
    // ...
}
```

새로 추가한 `SetEntityLayer`의 inverse도 같은 방식이다. "어떤 레이어로 바꿀지"가 아니라 "지금 어떤 레이어에 있는지"를 먼저 읽어서 되돌릴 명령을 만든다.

```rust
DrawingCommand::SetEntityLayer { id, .. } => {
    let current_layer_id = project.drawing.entities.iter()
        .find(|entity| entity.id == *id)
        .ok_or(CoreError::MissingEntity(*id))?
        .layer_id;
    DrawingCommand::SetEntityLayer { id: *id, layer_id: current_layer_id }
}
```

`RemoveLayer`도 마찬가지로, 지우기 전에 `Layer` 전체를 복제해 `AddLayer`로 되돌릴 수 있게 한다.

`MoveEntity`만 예외적으로 상태를 읽지 않는다. 이동은 그 자체로 대칭이기 때문이다.

```rust
DrawingCommand::MoveEntity { id, dx, dy } => DrawingCommand::MoveEntity {
    id: *id,
    dx: LengthMm(-dx.0),
    dy: LengthMm(-dy.0),
},
```

`(dx, dy)`만큼 옮긴 것을 되돌리려면 `(-dx, -dy)`만큼 다시 옮기면 된다 — 대상 엔티티의 현재 내용을 알 필요가 없다.

---

## 7. `MoveEntity` 구현 — 6종 도형에 이동을 적용하기

`Entity`는 `Line`/`Polyline`/`Rectangle`/`Circle`/`Arc`/`Text` 중 하나를 담고 있다. 이동은 각 도형의 "위치를 나타내는 점"만 옮기고 나머지(반지름, 각도, 텍스트 내용 등)는 그대로 둬야 한다.

```rust
fn translate_geometry(geometry: &EntityGeometry, dx: LengthMm, dy: LengthMm) -> EntityGeometry {
    let shift = |p: Point2| cad_geometry::translate_point(p, dx, dy);
    match geometry {
        EntityGeometry::Line(line) => EntityGeometry::Line(Line {
            start: shift(line.start),
            end: shift(line.end),
        }),
        EntityGeometry::Circle(circle) => EntityGeometry::Circle(Circle {
            center: shift(circle.center),
            radius: circle.radius, // 반지름은 이동과 무관하다
        }),
        // Polyline, Rectangle, Arc, Text도 같은 방식
        // ...
    }
}
```

좌표 이동 공식 자체는 새로 만들지 않고 Phase 2의 `cad_geometry::translate_point`를 그대로 호출한다.

```rust
let shift = |p: Point2| cad_geometry::translate_point(p, dx, dy);
```

`shift`는 클로저(closure)로, `dx`와 `dy`를 캡처해서 "점 하나를 받아 옮긴 점을 반환하는 함수"처럼 동작한다. `Polyline`처럼 점이 여러 개인 도형에서는 `map`에 그대로 넘길 수 있다.

```rust
EntityGeometry::Polyline(polyline) => EntityGeometry::Polyline(Polyline {
    points: polyline.points.iter().copied().map(shift).collect(),
    closed: polyline.closed,
}),
```

`apply`에서는 이 함수를 엔티티의 `geometry` 필드에 대입한다.

```rust
DrawingCommand::MoveEntity { id, dx, dy } => {
    let entity = find_entity_mut(drawing(project), *id)?;
    entity.geometry = translate_geometry(&entity.geometry, *dx, *dy);
}
```

`find_entity_mut`는 기존 `apply` 안에서 반복되던 "id로 엔티티를 찾아 가변 참조를 얻는" 패턴을 함수로 뽑아낸 것이다.

```rust
fn find_entity_mut(drawing: &mut Drawing, id: EntityId) -> Result<&mut Entity, CoreError> {
    drawing.entities.iter_mut()
        .find(|entity| entity.id == id)
        .ok_or(CoreError::MissingEntity(id))
}
```

`MoveEntity`와 `SetEntityLayer` 둘 다 이 함수를 사용한다.

---

## 8. 검증 통합 — command 실행이 실패할 수 있게 만들기

### 8.1 왜 지금까지는 검증이 없었는가

Phase 1의 `Drawing::validate()`(레이어 참조 검사)와 Phase 2의 `cad_geometry::validate_*`(zero-length line 등)는 각자 만들어졌을 뿐, 어디에서도 호출되지 않았다. `03_백엔드_구조_정책.md`가 요구하는 "오류는 저장, 출력, 명령 실행을 막는다"는 command 실행 지점에서 실제로 검사가 이뤄져야 의미가 있다.

### 8.2 두 계층의 검증을 하나로 합치기

```rust
fn validate_drawing(drawing: &Drawing) -> Result<(), ValidationReport> {
    let mut report = drawing.validate(); // cad_core: 레이어 참조 검사

    for entity in &drawing.entities {
        let geometry_error = match &entity.geometry {
            EntityGeometry::Line(line) => cad_geometry::validate_line(line).err(),
            EntityGeometry::Circle(circle) => cad_geometry::validate_circle(circle).err(),
            // Polyline, Rectangle, Arc도 동일한 방식, Text는 검사할 게 없음
            EntityGeometry::Text(_) => None,
        };
        if let Some(error) = geometry_error {
            report.issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                target: ValidationTarget::Entity(entity.id),
                message: error.to_string(),
                suggestion: "Adjust the entity geometry so it satisfies validation rules.".to_owned(),
            });
        }
    }

    if report.has_errors() { Err(report) } else { Ok(()) }
}
```

`cad_core::Drawing::validate()`가 만든 `ValidationReport`에, `cad_geometry`의 `GeometryError`를 `ValidationIssue`로 변환해 그대로 추가한다. `GeometryError`가 이미 `Display`를 구현하고 있어서 `error.to_string()`만으로 사람이 읽을 수 있는 메시지를 얻는다 (Phase 2에서 이 trait를 구현해 둔 이유가 여기서 드러난다).

두 계층의 오류가 같은 `ValidationIssue` 형태로 합쳐지므로, 호출하는 쪽(`execute`)은 오류가 어느 계층에서 왔는지 신경 쓰지 않고 하나의 `ValidationReport`만 다루면 된다.

### 8.3 실패하면 되돌리기

```rust
pub fn execute(&mut self, project: &mut Project, command: DrawingCommand) -> Result<(), CommandError> {
    let inverse = inverse(project, &command)?;
    apply(project, &command)?;

    if let Err(report) = validate_drawing(&project.drawing) {
        apply(project, &inverse)?; // 검증 실패 → 즉시 원상복구
        return Err(CommandError::Validation(report));
    }

    self.undo.push(HistoryEntry { command, inverse });
    self.redo.clear();
    Ok(())
}
```

흐름을 그림으로 보면 다음과 같다.

```text
inverse 계산 (적용 전 상태 기준)
    ↓
command 적용
    ↓
검증
    ├─ 통과 → undo 이력에 기록, 완료
    └─ 실패 → inverse를 다시 적용해 원상복구 → Err 반환
```

이미 계산해 둔 `inverse`를 검증 실패 시 롤백에도 그대로 재사용한다는 점이 이 구조의 핵심이다 — 되돌리는 로직을 두 번 만들지 않아도 된다.

> 📌 **왜 `undo`/`redo`에는 검증을 넣지 않았는가?**
>
> `undo`는 이전에 이미 유효했던 상태로 되돌아가는 동작이고, `redo`는 과거에 한 번 `execute`를 통과했던 명령을 다시 적용하는 동작이다.
> 두 경우 모두 결과 상태가 과거에 이미 유효성이 확인된 상태이므로, 매번 다시 검증할 필요가 없다.
> 검증은 "새로운 상태가 처음 만들어지는 지점"인 `execute`에만 필요하다.

---

## 9. `replay` — 명령 기록으로 프로젝트 재구성하기

```rust
pub fn replay(commands: &[DrawingCommand]) -> Result<Project, CommandError> {
    let mut project = Project::default();
    let mut history = CommandHistory::default();
    for command in commands {
        history.execute(&mut project, command.clone())?;
    }
    Ok(project)
}
```

빈 프로젝트에서 시작해 명령 목록을 순서대로 `execute`하는 것이 전부다. 새로운 로직을 만들지 않고 이미 검증까지 포함된 `execute`를 그대로 재사용했다.

이 함수가 필요한 이유는 `01_제품_기획_정책.md`의 "자동 저장과 복구" 때문이다. 도면 파일 자체가 아니라 **명령의 기록(저널)**을 저장해두면, 다음 두 가지가 가능해진다.

- 저장된 최종 상태 파일이 손상됐을 때, 명령 기록으로 마지막 상태를 재구성
- 같은 명령 기록을 재생하면 항상 같은 결과가 나온다는 결정성 보장 (`01_제품_기획_정책.md`: "같은 입력에 같은 결과를 내는 결정성")

다만 이번 Phase에서는 함수만 준비했고, 실제로 명령 기록을 파일에 저장하는 부분은 아직 없다 — 이는 Phase 6(`cad_io`)의 몫이다.

---

## 10. 테스트

`crates/cad_core/src/lib.rs`에 2개, `crates/cad_command/src/lib.rs`에 8개를 추가했다.

### `cad_core`

| 테스트 | 검증 내용 |
|---|---|
| `remove_layer_returns_the_layer` | 레이어 삭제가 정상 동작하는지 |
| `remove_layer_reports_missing_layer` | 존재하지 않는 레이어 삭제 시 오류 반환 |

### `cad_command`

| 테스트 | 검증 내용 |
|---|---|
| `add_entity_can_be_undone_and_redone` | 기존 테스트 (유지) |
| `move_entity_translates_geometry_and_can_be_undone` | 이동 후 좌표가 바뀌고, undo로 원래 좌표로 돌아오는지 |
| `set_entity_layer_changes_layer_and_can_be_undone` | 레이어 변경과 undo |
| `add_and_remove_layer_can_be_undone_and_redone` | 레이어 추가/삭제와 undo/redo |
| `add_and_remove_dimension_can_be_undone` | 치수 추가/삭제와 undo |
| `execute_rejects_entity_with_missing_layer_and_rolls_back` | 존재하지 않는 레이어를 참조하는 엔티티 추가가 거부되고 상태가 원복되는지 |
| `execute_rejects_zero_length_line_and_rolls_back` | 길이가 0인 선 추가가 거부되고 상태가 원복되는지 |
| `replay_rebuilds_project_deterministically` | 명령 목록을 재생해 동일한 최종 상태를 얻는지 |

마지막 두 개(`execute_rejects_*`)가 이번 Phase의 핵심을 검증한다 — **잘못된 상태는 애초에 커밋되지 않는다.**

```rust
let result = history.execute(&mut project, DrawingCommand::AddEntity(entity));
assert!(matches!(result, Err(CommandError::Validation(_))));
assert!(project.drawing.entities.is_empty()); // 롤백되어 비어 있어야 한다
```

---

## 11. 검증 결과

```text
cargo fmt --all -- --check
통과

cargo test --workspace
전체 통과
- cad_core: 12/12
- cad_geometry: 24/24
- cad_command: 8/8
- cad_tolerance: 9/9
- workspace total: 53

cargo clippy --workspace --all-targets -- -D warnings
경고 없음
```

이번 Phase는 처음 작성한 코드가 컴파일·테스트·clippy를 모두 한 번에 통과했다 — Phase 1~3에서 반복적으로 겪었던 (포맷팅 재실행, clippy 지적 수정, 부동소수점 비교 실패) 과정 없이 끝난 첫 Phase였다.

---

## 12. Phase 4에서 사용한 Rust 핵심 개념

### 함수 추출을 통한 중복 제거

`apply` 안에서 반복되던 "id로 엔티티 찾기" 패턴을 `find_entity_mut` 함수로 뽑아냈다.

```rust
fn find_entity_mut(drawing: &mut Drawing, id: EntityId) -> Result<&mut Entity, CoreError>
```

### 클로저

이동량 `(dx, dy)`를 캡처해 여러 점에 반복 적용할 수 있는 함수처럼 사용했다.

```rust
let shift = |p: Point2| cad_geometry::translate_point(p, dx, dy);
```

### `Iterator::map`과 클로저 조합

`Polyline`의 모든 점에 같은 이동을 적용했다.

```rust
polyline.points.iter().copied().map(shift).collect()
```

### `if let Err(...)`을 이용한 실패 분기와 롤백

검증 실패라는 특정 조건에서만 되돌리기 로직을 실행했다.

```rust
if let Err(report) = validate_drawing(&project.drawing) {
    apply(project, &inverse)?;
    return Err(CommandError::Validation(report));
}
```

### 서로 다른 계층의 오류를 공통 타입으로 합치기

`GeometryError`(cad_geometry)를 `ValidationIssue`(cad_core)로 변환해 하나의 `ValidationReport`에 모았다.

```rust
message: error.to_string(),
```

### 기존 함수 재귀적 재사용

`replay`는 새 로직 없이 이미 검증을 포함한 `execute`를 반복 호출하는 것만으로 구현됐다.

```rust
history.execute(&mut project, command.clone())?;
```

---

## 13. Phase 4 완료 결과

Phase 4를 통해 `cad_command`는 단순히 엔티티를 추가/삭제하던 상태에서, 다음을 보장하는 실행 계층으로 확장됐다.

- Entity 이동, 레이어 재할당, Layer/Dimension 추가·삭제를 모두 command로 표현
- 모든 command 실행 결과가 `cad_core` + `cad_geometry`의 검증을 통과해야만 확정됨
- 검증 실패 시 자동 롤백으로 잘못된 상태가 절대 남지 않음
- 명령 로그만으로 프로젝트를 결정적으로 재구성(`replay`)

`02_아키텍처_정책.md`가 그린 흐름 중 "DrawingCommand -> Core Model -> 검증"까지가 이번 Phase에서 실제로 연결됐다.

```text
cad_app (아직 없음)
    ↓
DrawingCommand         ← Phase 4에서 8종으로 확장
    ↓
Core Model (cad_core)  ← Phase 1
    ↓
검증 (cad_core + cad_geometry)  ← Phase 4에서 실제로 연결
    ↓
렌더 모델 / 저장 (아직 없음)
```

---

## 14. 남은 과제

### `cad_tolerance`는 아직 연결되지 않음

`Dimension`에 공차(`ToleranceSpec`)를 적용하는 command가 없다. 현재 `Dimension` 구조체 자체에도 공차 필드가 없어서, 이를 추가하려면 `cad_core::Dimension`의 필드 확장이 먼저 필요하다 — 이는 데이터 구조 변경이므로 별도로 범위를 확인해야 한다.

### Undo/redo에 상한이 없음

`CommandHistory`의 undo 스택은 무한히 쌓인다. 개인용 데스크톱 도구의 MVP 범위에서는 문제가 되지 않지만, 장시간 작업 시 메모리 사용량을 점검할 필요는 있다.

### `cad_io`와 아직 연결되지 않음

`replay`는 명령 로그를 받는 함수만 준비됐고, 실제로 로그를 파일에 쓰고 읽는 부분은 없다. Phase 6에서 저장 파이프라인을 만들 때 연결한다.

---

## 15. 다음 Phase

다음 단계는 `cad_render` 확장이다.

### Phase 5 예정 범위

- 현재 `Line`만 지원하는 `RenderPrimitive` 생성 로직을 6종 `EntityGeometry` 전체로 확장 (Phase 0에서 컴파일 오류를 막기 위해 임시로 넓혀둔 부분을 정식으로 재검토)
- `Dimension`을 렌더 primitive로 변환하는 로직 추가
- 결정적인 렌더링 순서 보장

`cad_command`가 "무엇을 바꿀지"를 안전하게 결정하는 계층이라면, `cad_render`는 그 결과를 "어떻게 보여줄지"로 변환하는 계층이다.

---

## 마무리

Phase 4는 지금까지 따로 준비해온 `cad_core`, `cad_geometry`의 검증 로직을 실제로 연결한 단계다. 두 계층 모두 Phase 1, 2에서 이미 만들어져 있었지만 아무도 호출하지 않으면 죽은 코드나 다름없다.

이번 Phase에서 중요했던 것은 새로운 계산 공식이 아니라, **"검증을 어디서 실행하고, 실패하면 어떻게 되돌릴 것인가"라는 흐름을 설계한 것**이다. `inverse`를 `apply`보다 먼저 계산해 두는 기존 패턴 덕분에, 검증 실패 시 롤백을 위한 별도 로직을 새로 만들 필요가 없었다 — Phase 3에서 강조했던 "새 공식을 만들지 않고 기존 함수를 재사용한다"는 원칙이 이번에도 그대로 적용된 셈이다.

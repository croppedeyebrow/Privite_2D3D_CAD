# Rust로 CAD 엔진 만들기 — Phase 5: `cad_render` 확장

## 들어가며

Phase 4까지 완성된 흐름은 다음과 같았다.

```text
DrawingCommand -> Core Model (cad_core) -> 검증 (cad_core + cad_geometry)
```

`02_아키텍처_정책.md`가 정의한 전체 그림은 여기서 한 단계 더 나아간다.

```text
사용자 입력 -> UI 이벤트 -> DrawingCommand -> Core Model -> 검증 -> 렌더 모델 -> 화면/출력/저장
```

검증을 통과한 `Project`는 여전히 `cad_core`의 내부 표현일 뿐이다. 화면에 그리거나 SVG로 출력하려면 이 표현을 "그리기 쉬운 형태"로 한 번 더 변환해야 한다. 이 변환을 담당하는 것이 `cad_render`다.

Phase 5는 이 변환 계층을 완성하는 단계다.

---

## 1. Phase 5의 목적

`02_아키텍처_정책.md`는 `cad_render`의 책임을 "core model을 RenderPrimitive로 변환"으로 정의한다. Phase 4 시점에 이미 되어 있던 것과, 이번에 채워야 할 것은 다음과 같이 나뉜다.

| 상태 | 내용 |
|---|---|
| 이미 되어 있음 | `Line`/`Polyline`/`Rectangle`/`Circle`/`Arc`/`Text` 6종 Entity → `RenderPrimitive` 변환 |
| 없었음 | `Dimension` → `RenderPrimitive` 변환 |
| 없었음 | 레이어의 `visible` 상태 반영 (숨긴 레이어의 도형이 그대로 렌더 결과에 포함됨) |
| 확인 필요 | 렌더 결과 순서가 결정적인지 |

> 📌 **6종 Entity 변환은 왜 이미 되어 있었는가?**
>
> Phase 0에서 `cad_core`의 `EntityGeometry`가 6종으로 확장되면서 `cad_render`의 `match`가
> 컴파일조차 되지 않는 상태였다. 그때 컴파일을 통과시키기 위해 6종 전체를 한 번에 매핑해뒀다.
> Phase 4 문서의 "다음 Phase 예정 범위"에는 이 부분을 "재검토"라고 적어뒀지만,
> 실제로 다시 살펴본 결과 로직 자체는 이미 올바르게 구현되어 있어 손댈 필요가 없었다.
> Phase 5의 실제 작업은 Dimension과 레이어 가시성, 이 두 가지에 집중됐다.

---

## 2. 설계 결정 사항

### 2.1 Dimension을 얼마나 구체적으로 렌더링할 것인가

`Dimension`을 화면에 그리려면 실제로는 다음 요소가 필요하다.

```text
치수선(dimension line)     — 측정 구간과 나란하거나 offset만큼 떨어진 선
인출선(extension line)     — 측정 시작점/끝점에서 치수선까지 이어지는 선
화살표(arrowhead)          — 치수선 양 끝
치수 문자(text)            — 측정값과 공차
```

`cad_core::Dimension`이 가진 정보는 `start`, `end`, `offset`, `kind`(Linear/Aligned)뿐이다. 여기서 치수선과 인출선의 정확한 좌표를 계산하려면, "Linear일 때 offset이 수평 방향인지 수직 방향인지" 같은 시각적 규칙을 지금 정해야 한다. 이 규칙은 문서 어디에도 명시되어 있지 않다.

> ⚠️ **왜 지금 규칙을 정하지 않았는가?**
>
> 이 규칙은 순수하게 기하학적인 문제가 아니라 렌더링 방식(스크린 좌표계, 캔버스 라이브러리)과 얽혀 있다.
> 아직 GUI framework도 정해지지 않은 시점(Phase 8에서 결정 예정)에 규칙을 먼저 못박으면,
> 나중에 실제 캔버스에 그려보고 나서 다시 바꿔야 할 가능성이 높다.
> Phase 1의 Sheet 구조 결정과 같은 이유로, 확실하지 않은 것은 최소한으로 남겨두는 편을 택했다.

**결정**: `RenderPrimitive::Dimension`은 계산된 치수선 좌표가 아니라 `cad_core::Dimension`의 원본 데이터(`kind`, `start`, `end`, `offset`)를 그대로 전달한다. 실제 치수선/인출선 배치는 Phase 8에서 렌더링 백엔드와 함께 설계한다.

### 2.2 `cad_render`가 `cad_geometry`에 의존해야 하는가

Phase 4에서 `cad_command -> cad_geometry` 의존성을 추가했던 것과 비슷한 상황이 다시 생길 뻔했다. Dimension의 측정값(예: "100.00 mm")을 렌더 primitive에 포함하려면 `cad_geometry::distance(start, end)`가 필요해 보였기 때문이다.

이번에는 의존성을 추가하지 않기로 했다. 이유는 두 가지다.

1. 측정값에 공차까지 포함한 완전한 라벨(`03_백엔드_구조_정책.md`가 요구하는 "계산 trace")은 `cad_tolerance`가 필요하고, `Dimension`에는 아직 `ToleranceSpec` 필드 자체가 없다 (Phase 4 문서의 "남은 과제"에 기록). 지금 거리만 계산해 넣어봤자 반쪽짜리 라벨이 된다.
2. `cad_render`의 책임은 "그리기 위한 변환"이지 "측정값 계산"이 아니다. 측정값이 필요한 시점(예: 속성 패널)에 그 계층이 직접 `cad_geometry`를 호출하는 편이 책임 분리에 맞는다.

**결정**: `cad_render`는 `cad_core`에만 의존하는 현재 상태를 유지한다. `02_아키텍처_정책.md`의 의존성 다이어그램도 수정하지 않았다.

---

## 3. `RenderPrimitive::Dimension`

```rust
Dimension {
    kind: DimensionKind,
    start: (f64, f64),
    end: (f64, f64),
    offset: f64,
},
```

변환 코드는 다른 5종과 같은 패턴을 따른다 — `cad_core`의 타입을 primitive 전용 튜플/필드로 옮겨 담을 뿐, 계산은 하지 않는다.

```rust
for dimension in &drawing.dimensions {
    if is_layer_visible(drawing, dimension.layer_id) {
        primitives.push(RenderPrimitive::Dimension {
            kind: dimension.kind,
            start: (dimension.start.x.0, dimension.start.y.0),
            end: (dimension.end.x.0, dimension.end.y.0),
            offset: dimension.offset.0,
        });
    }
}
```

`DimensionKind`는 `cad_core`가 이미 `Copy`로 정의해둔 타입이라 그대로 재사용했다 — `cad_render`가 이미 `cad_core`에 의존하고 있으므로 새로운 타입을 만들 필요가 없었다.

---

## 4. 레이어 가시성 반영

### 4.1 문제

기존 코드는 `drawing.entities`를 그냥 순회해서, 사용자가 UI에서 레이어를 꺼도(`Layer::visible = false`) 렌더 결과에는 그 레이어의 도형이 계속 나타나는 상태였다. `04_UI_와이어프레임_정책.md`가 레이어 패널을 통해 가시성을 다루도록 정의하고 있는 이상, 렌더 계층이 이를 반영하지 않으면 그 UI 기능 자체가 의미가 없어진다.

### 4.2 구현

```rust
fn is_layer_visible(drawing: &Drawing, layer_id: LayerId) -> bool {
    drawing.layer(layer_id).is_none_or(|layer| layer.visible)
}
```

`Drawing::layer(id)`는 `Option<&Layer>`를 반환한다. 레이어를 찾았으면 그 레이어의 `visible` 값을 따르고, 레이어를 찾지 못했으면(이론적으로는 Phase 4의 검증이 이런 상태를 막지만, 방어적으로) 기본값을 "보인다"로 둔다.

`Option::is_none_or`는 다음 두 가지 경우를 한 번에 표현한다.

```text
None       → true (기본값 그대로)
Some(x)    → f(x) 의 결과
```

이 메서드가 없다면 다음과 같이 풀어 써야 한다.

```rust
match drawing.layer(layer_id) {
    None => true,
    Some(layer) => layer.visible,
}
```

`is_none_or`는 이 패턴을 한 줄로 줄여준다. `entity_primitive`를 호출하기 전에 이 함수로 걸러낸다.

```rust
for entity in &drawing.entities {
    if is_layer_visible(drawing, entity.layer_id) {
        primitives.push(entity_primitive(&entity.geometry));
    }
}
```

Dimension에도 동일한 필터를 적용했다.

---

## 5. 결정적인 렌더링 순서

`03_백엔드_구조_정책.md`는 "좌표와 출력 정렬 순서는 deterministic해야 한다"고 요구한다. `build_render_model`은 다음 순서를 고정한다.

```text
1. drawing.entities를 삽입 순서대로 순회
2. drawing.dimensions를 삽입 순서대로 순회
```

둘 다 `HashMap`이 아니라 `Vec`이므로 순회 순서는 항상 삽입 순서와 같다. 별도의 정렬 로직 없이도 "같은 프로젝트 상태 → 같은 렌더 결과 순서"가 보장된다. 이 점은 테스트(`render_order_follows_entities_then_dimensions`)로 직접 확인했다.

레이어 쌓임 순서(z-order)에 따라 그리는 순서를 바꾸는 기능은 아직 없다 — `cad_core::Layer`에 순서를 나타내는 필드 자체가 없어서, 이번 Phase의 범위를 넘어선다.

---

## 6. 테스트

`crates/cad_render/src/lib.rs`에 6개의 테스트를 추가했다 (이전까지 0개).

| 테스트 | 검증 내용 |
|---|---|
| `line_entity_maps_to_line_primitive` | Line 변환이 올바른지 |
| `circle_entity_maps_to_circle_primitive` | Circle 변환이 올바른지 |
| `dimension_maps_to_dimension_primitive` | Dimension이 원본 데이터 그대로 변환되는지 |
| `entities_on_hidden_layer_are_excluded` | 숨긴 레이어의 엔티티가 제외되는지 |
| `dimensions_on_hidden_layer_are_excluded` | 숨긴 레이어의 치수가 제외되는지 |
| `render_order_follows_entities_then_dimensions` | 엔티티 → 치수 순서가 항상 유지되는지 |

---

## 7. 검증 결과

```text
cargo fmt --all -- --check
통과

cargo test --workspace
전체 통과
- cad_render: 6/6
- workspace total: 59

cargo clippy --workspace --all-targets -- -D warnings
경고 없음
```

Phase 4에 이어 이번에도 코드를 한 번에 작성해서 fmt/test/clippy를 모두 통과했다.

---

## 8. Phase 5에서 사용한 Rust 핵심 개념

### `Option::is_none_or`

"값이 없으면 기본값, 있으면 조건 검사"를 한 줄로 표현했다.

```rust
drawing.layer(layer_id).is_none_or(|layer| layer.visible)
```

### `Vec::with_capacity`

결과 벡터의 최종 크기를 미리 알 수 있을 때 재할당을 피하기 위해 사용했다.

```rust
Vec::with_capacity(drawing.entities.len() + drawing.dimensions.len())
```

### 순수 변환 함수 분리

엔티티 하나를 primitive 하나로 바꾸는 로직을 `entity_primitive`라는 별도 함수로 분리해, `build_render_model`은 "무엇을 순회하고 무엇을 걸러낼지"에만 집중하게 했다.

```rust
fn entity_primitive(geometry: &EntityGeometry) -> RenderPrimitive
```

### 문서 주석으로 미확정 설계를 표시하기

아직 정해지지 않은 부분(치수선 배치)을 코드에 남겨 다음 Phase에서 놓치지 않도록 했다.

```rust
/// Raw dimension geometry. Laying out the actual dimension line and
/// extension lines from `start`/`end`/`offset` is a rendering-backend
/// decision left to the UI layer (Phase 8).
Dimension { /* ... */ }
```

---

## 9. Phase 5 완료 결과

`cad_render`는 이제 `cad_core`가 표현할 수 있는 모든 영속 도형(6종 Entity + Dimension)을 빠짐없이 렌더 가능한 형태로 변환하며, 레이어 가시성을 반영하고, 항상 같은 순서로 결과를 낸다.

```text
Project (cad_core)
    ↓ build_render_model
Vec<RenderPrimitive>   ← 6종 Entity + Dimension, 숨긴 레이어 제외, 결정적 순서
```

`02_아키텍처_정책.md`가 그린 흐름 중 "검증 -> 렌더 모델"까지 연결됐다.

---

## 10. 남은 과제

### 치수선 배치 규칙 미정

`RenderPrimitive::Dimension`은 원본 데이터만 전달한다. 실제 치수선/인출선/화살표 좌표 계산은 Phase 8에서 GUI framework를 정할 때 함께 설계해야 한다.

### 치수 라벨(측정값 + 공차) 없음

`Dimension`에 `ToleranceSpec`이 아직 연결되지 않아, "100.0 ±0.2" 같은 완전한 라벨을 만들 수 없다. `cad_tolerance`와 `Dimension`을 연결하는 작업이 먼저 필요하다.

### 레이어 쌓임 순서(z-order) 없음

`Layer`에 순서를 나타내는 필드가 없어, 현재는 항상 "엔티티 삽입 순서 → 치수"로만 그려진다. 레이어 순서 편집 기능이 필요해지면 함께 설계한다.

---

## 11. 다음 Phase

다음 단계는 `cad_io` 구현이다. 지금까지의 Phase와 달리 이번에는 시작 전에 반드시 사용자 승인이 필요하다.

### Phase 6 예정 범위 (⚠ 저장 형식 승인 필요)

- 저장 형식 확정 (`05_사용자_승인_정책.md`: "프로젝트 저장 형식 또는 DB schema 변경"은 승인 대상)
- `임시 파일 작성 -> flush -> 재검증 -> 원자적 교체 -> 백업 상태 갱신` 파이프라인 구현 (`03_백엔드_구조_정책.md`)
- 자동저장, 백업, 복구
- SVG 출력

`cad_render`가 만든 `RenderPrimitive`는 화면 표시뿐 아니라 SVG 출력의 입력으로도 재사용될 예정이다.

---

## 마무리

Phase 5는 지금까지의 Phase 중 가장 코드 양이 적었지만, 두 번의 "지금 정하지 않는다"는 결정을 내린 Phase이기도 하다 — 치수선의 시각적 배치 규칙과, `cad_geometry`로의 의존성 확장. 두 결정 모두 "아직 확정되지 않은 것(GUI framework, Dimension의 공차 필드)에 의존하는 설계를 미리 만들지 않는다"는 하나의 원칙에서 나왔다.

다음 Phase는 지금까지와 성격이 다르다. `cad_io`는 사용자의 실제 파일을 다루므로, 시작 전에 저장 형식 자체를 승인받아야 한다.

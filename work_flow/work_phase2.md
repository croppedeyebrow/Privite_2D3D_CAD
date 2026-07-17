# Rust로 CAD 엔진 만들기 — Phase 2: `cad_geometry` 기하 연산 계층 확장

## 들어가며

Phase 1에서는 CAD 프로젝트 전체에서 사용할 Stable ID와 공용 Validation 구조를 `cad_core`에 추가했다.

그러나 CAD 엔진에서 데이터 구조만 정의되어 있다고 해서 실제 도형을 계산할 수 있는 것은 아니다.

선을 그리거나 원을 표시하려면 다음과 같은 계산이 필요하다.

- 두 점 사이의 거리
- 선분과 Polyline의 길이
- 원의 둘레와 호의 길이
- 호의 시작점과 끝점
- 두 선분의 교차점
- 도형의 이동·회전·크기 변경
- 마우스 포인터와 가장 가까운 스냅 지점
- 잘못된 도형 데이터 검증

Phase 2에서는 이러한 계산을 담당하는 `cad_geometry`를 확장했다.

이번 작업의 핵심은 단순히 수학 함수를 추가하는 것이 아니라, 다음 원칙을 코드에 반영하는 것이었다.

1. 기하 계산은 가능한 한 **순수 함수**로 작성한다.
2. 부동소수점 값을 직접 `==`로 비교하지 않는다.
3. 길이와 각도에 적용할 epsilon을 명시적으로 분리한다.
4. 잘못된 도형은 전용 오류 타입으로 검출한다.
5. 계산 결과와 후보 선택 순서는 결정적이어야 한다.
6. 렌더링과 명령 계층이 같은 계산을 중복 구현하지 않도록 한다.

---

## 1. Phase 2의 목적

`02_아키텍처_정책.md`에서는 `cad_geometry`의 책임을 다음과 같이 정의하고 있다.

- 점과 선
- 원과 호
- 거리와 길이
- 교차 판정
- 기하 변환
- 스냅
- 도형 유효성 검사

또한 `03_백엔드_구조_정책.md`에서는 Geometry 계층에 다음 규칙을 요구한다.

- 계산 로직은 순수 함수로 구현한다.
- 부동소수점을 직접 비교하지 않는다.
- epsilon 정책을 명시한다.
- 길이가 0인 선을 검출한다.
- 반지름이 0 이하인 원과 호를 검출한다.
- 잘못된 호의 각도를 검출한다.

Phase 2의 목표는 이 정책들을 실제 함수와 테스트로 구현하여, 이후 `cad_render`, `cad_command`, `cad_io`, `cad_app`이 공통으로 사용할 기하 계산 기반을 만드는 것이다.

---

## 2. Phase 1까지의 `cad_geometry`

Phase 1까지 `cad_geometry`에는 다음 세 함수만 존재했다.

```rust
distance(...)
approx_eq(...)
line_is_degenerate(...)
```

즉, 사실상 `Line`만 다룰 수 있는 상태였다.

반면 `cad_core`에는 이미 다음 도형 타입이 정의되어 있었다.

- `Polyline`
- `Rectangle`
- `Circle`
- `Arc`
- `Text`

이 상태로 개발을 계속하면 `cad_render`와 `cad_command`가 필요한 계산을 각자 구현하게 될 가능성이 높다.

예를 들어 원의 둘레, 호의 끝점, Rectangle의 꼭짓점 계산을 각 crate가 독립적으로 구현하면 다음 문제가 발생할 수 있다.

```text
cad_render의 계산 방식
    ≠
cad_command의 계산 방식
    ≠
cad_app의 계산 방식
```

이러한 중복은 코드 양만 늘리는 것이 아니다.

- epsilon 기준이 달라질 수 있다.
- 각도 단위가 서로 다르게 해석될 수 있다.
- Validation 기준이 달라질 수 있다.
- 같은 도형이 계층마다 다른 결과를 낼 수 있다.
- 버그를 수정할 때 여러 crate를 함께 수정해야 한다.

따라서 기하 계산을 `cad_geometry` 한곳에 모으고, 다른 crate는 이 함수를 재사용하도록 설계했다.

---

# 3. 순수 함수로 기하 계산 분리하기

## 3.1 순수 함수란 무엇인가

순수 함수는 일반적으로 다음 조건을 만족한다.

1. 같은 입력을 받으면 항상 같은 결과를 반환한다.
2. 함수 외부의 상태를 변경하지 않는다.
3. 파일, 네트워크, 전역 변수 등의 외부 환경에 의존하지 않는다.

예를 들어 다음 거리 계산 함수는 순수 함수이다.

```rust
pub fn distance(a: Point2, b: Point2) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;

    (dx * dx + dy * dy).sqrt()
}
```

같은 두 점을 전달하면 언제나 같은 거리를 반환하며, 외부 상태를 수정하지도 않는다.

반면 다음 함수는 애플리케이션 상태를 직접 변경한다.

```rust
pub fn move_selected_entity(
    app_state: &mut AppState,
    dx: f64,
    dy: f64,
) {
    // 선택된 Entity와 애플리케이션 상태를 변경한다.
}
```

상태를 변경하는 함수가 나쁜 것은 아니다. 다만 Geometry 계층에서는 수학 계산과 상태 변경을 분리하는 것이 중요하다.

```text
cad_command
    ├─ 어떤 Entity를 이동할지 결정
    ├─ 명령 실행과 Undo/Redo 이력 관리
    └─ cad_geometry의 변환 함수 호출

cad_geometry
    └─ 입력 좌표를 바탕으로 새로운 좌표 계산
```

이 구조를 사용하면 다음 장점이 있다.

- UI 없이 계산만 단위 테스트할 수 있다.
- Undo/Redo 구현과 기하 공식을 분리할 수 있다.
- 렌더러와 명령 계층이 같은 함수를 재사용할 수 있다.
- 외부 상태에 따른 비결정적 버그가 줄어든다.
- 향후 병렬 계산이나 캐싱을 적용하기 쉬워진다.

---

# 4. 부동소수점과 epsilon 정책

## 4.1 왜 `f64`를 직접 비교하면 안 되는가

컴퓨터는 대부분의 실수를 이진 부동소수점으로 표현한다.

그 결과 사람이 보기에는 같은 값이어도 내부적으로는 미세한 차이가 생길 수 있다.

```rust
let value = 0.1 + 0.2;

assert!(value == 0.3);
```

위 비교는 기대와 다르게 실패할 수 있다.

기하 계산에서는 다음 연산을 자주 사용한다.

- 제곱근
- 삼각함수
- 회전
- 정규화
- 교차점 계산
- 여러 구간의 길이 합산

이 과정에서 작은 오차가 반복적으로 발생한다.

예를 들어 점 `(1, 0)`을 원점 기준으로 90도 회전하면 수학적으로는 `(0, 1)`이 되어야 한다.

하지만 실제 x 좌표는 다음과 비슷한 값이 될 수 있다.

```text
0.00000000000000006123
```

따라서 다음과 같은 직접 비교는 안전하지 않다.

```rust
rotated.x == 0.0
```

---

## 4.2 `approx_eq`로 근사 비교하기

이번 Phase에서는 부동소수점 비교가 다음 함수를 거치도록 했다.

```rust
pub fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() <= epsilon
}
```

이 함수는 두 값의 차이가 허용오차인 `epsilon` 이하인지 검사한다.

```rust
assert!(approx_eq(
    0.1 + 0.2,
    0.3,
    1.0e-9,
));
```

여기서 중요한 점은 epsilon을 함수 내부에 숨기지 않고 인자로 전달한다는 것이다.

```rust
approx_eq(a, b, epsilon)
```

이 방식은 호출 지점에서 어떤 정밀도 기준을 적용했는지 확인할 수 있게 한다.

---

## 4.3 길이와 각도의 epsilon 분리

이번 구현에서는 하나의 epsilon을 모든 계산에 사용하지 않았다.

```rust
pub const DEFAULT_EPSILON_MM: f64 = 1.0e-9;
pub const DEFAULT_ANGLE_EPSILON_RAD: f64 = 1.0e-9;
```

각 상수의 의미는 다음과 같다.

| 상수                        | 적용 대상                      |
| --------------------------- | ------------------------------ |
| `DEFAULT_EPSILON_MM`        | 좌표, 거리, 길이               |
| `DEFAULT_ANGLE_EPSILON_RAD` | 시작 각도, sweep angle 등 각도 |

길이와 각도는 서로 다른 물리량이다.

```text
길이: millimeter
각도: radian
```

현재 두 상수의 숫자가 같더라도 의미는 다르다.

하나의 상수를 공유하면 나중에 길이 판정 기준만 조정하려다가 각도 Validation까지 달라질 수 있다.

```rust
// 사용 목적이 모호한 방식
const EPSILON: f64 = 1.0e-9;
```

반면 단위와 목적을 이름에 포함하면 코드만 보고도 의미를 파악할 수 있다.

```rust
DEFAULT_EPSILON_MM
DEFAULT_ANGLE_EPSILON_RAD
```

> **Rust 개념 — 의미 있는 이름과 Newtype**
>
> 현재는 `f64`와 상수 이름으로 단위 의미를 구분했다.
> 향후 단위 안전성이 더 중요해진다면 다음처럼 Newtype을 적용할 수도 있다.
>
> ```rust
> struct Millimeters(f64);
> struct Radians(f64);
> ```
>
> 이렇게 하면 길이와 각도를 잘못 전달하는 실수를 컴파일 단계에서 막을 수 있다. 다만 현재 Phase에서는 API 복잡도를 불필요하게 늘리지 않기 위해 `f64`를 유지했다.

---

# 5. Geometry Validation 설계

## 5.1 구조체를 만들 수 있다고 유효한 도형은 아니다

Rust 타입으로 값을 생성할 수 있다고 해서 그 값이 기하학적으로 유효하다는 의미는 아니다.

다음 `Line`은 구조체 형태로는 정상이다.

```rust
Line {
    start: Point2 { x: 10.0, y: 10.0 },
    end: Point2 { x: 10.0, y: 10.0 },
}
```

그러나 시작점과 끝점이 같으므로 길이가 0인 퇴화 선분이다.

다음 값들도 구조체로는 표현할 수 있지만 유효한 도형은 아니다.

- 반지름이 음수인 원
- 반지름이 0인 호
- 폭이 0인 Rectangle
- 점이 하나뿐인 Polyline
- 모든 점이 같은 Polyline
- sweep angle이 0인 Arc

따라서 데이터 구조 정의와 별도로 Geometry Validation이 필요하다.

---

## 5.2 `GeometryError` enum

도형 검증 실패를 표현하기 위해 전용 오류 타입을 추가했다.

개념적으로는 다음과 같은 형태다.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryError {
    ZeroLengthLine,
    PolylineHasTooFewPoints,
    DegeneratePolyline,
    InvalidRectangleSize,
    InvalidCircleRadius,
    InvalidArcRadius,
    ZeroArcSweep,
}
```

문자열 대신 enum을 사용하는 이유는 타입 안전성 때문이다.

문자열로 오류를 표현하면 같은 오류가 여러 형태로 작성될 수 있다.

```rust
Err("negative radius")
Err("NegativeRadius")
Err("invalid circle radius")
```

enum을 사용하면 허용된 오류만 생성할 수 있다.

```rust
Err(GeometryError::InvalidCircleRadius)
```

상위 계층에서는 `match`를 이용해 오류 종류별로 처리할 수 있다.

```rust
match error {
    GeometryError::ZeroLengthLine => {
        // 선의 시작점과 끝점이 같다는 메시지를 표시한다.
    }
    GeometryError::InvalidCircleRadius => {
        // 반지름 입력 필드를 강조한다.
    }
    _ => {
        // 기타 Geometry 오류를 처리한다.
    }
}
```

Rust의 `match`는 모든 variant를 처리하도록 요구한다.

새로운 오류 variant가 추가되었을 때 기존 처리 코드가 불완전하면 컴파일러가 수정 지점을 알려준다.

---

## 5.3 `Result<T, E>`로 Validation 결과 표현하기

도형 검증 함수는 성공과 실패를 다음 타입으로 표현한다.

```rust
Result<(), GeometryError>
```

각 부분의 의미는 다음과 같다.

```text
Ok(())                 검증 통과
Err(GeometryError)     검증 실패
```

Validation 함수가 유효한 도형 자체를 새로 반환할 필요는 없으므로 성공 타입으로 빈 튜플 `()`을 사용한다.

예를 들면 다음과 같다.

```rust
pub fn validate_circle(
    circle: &Circle,
) -> Result<(), GeometryError> {
    if circle.radius <= 0.0 {
        return Err(GeometryError::InvalidCircleRadius);
    }

    Ok(())
}
```

`Result`를 사용하면 호출자는 실패 가능성을 무시할 수 없다.

```rust
validate_circle(&circle)?;
```

`?` 연산자는 오류가 발생하면 현재 함수에서 즉시 반환하고, 성공하면 다음 코드를 계속 실행한다.

---

## 5.4 `Display`와 `Error` trait 구현

`GeometryError`에는 다음 trait를 구현했다.

```rust
std::fmt::Display
std::error::Error
```

`Debug`와 `Display`는 목적이 다르다.

### `Debug`

개발자용 진단 표현에 가깝다.

```rust
println!("{error:?}");
```

### `Display`

사용자나 로그에서 읽기 좋은 표현을 제공한다.

```rust
println!("{error}");
```

예시는 다음과 같다.

```rust
impl std::fmt::Display for GeometryError {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            GeometryError::ZeroLengthLine => {
                write!(
                    formatter,
                    "line length must be greater than zero",
                )
            }
            GeometryError::InvalidCircleRadius => {
                write!(
                    formatter,
                    "circle radius must be greater than zero",
                )
            }
            _ => write!(formatter, "invalid geometry"),
        }
    }
}
```

그리고 `std::error::Error`를 구현하면 Rust 생태계의 일반적인 오류 처리 방식과 호환된다.

```rust
impl std::error::Error for GeometryError {}
```

이를 통해 상위 계층인 `cad_command`와 `cad_io`에서 `CoreError`와 `GeometryError`를 비슷한 방식으로 다룰 수 있다.

향후 `CommandError`에 `From<GeometryError>`를 구현하면 다음과 같이 `?`를 사용할 수 있다.

```rust
pub fn execute_create_circle(
    command: CreateCircleCommand,
) -> Result<(), CommandError> {
    validate_circle(&command.circle)?;

    Ok(())
}
```

---

# 6. 도형별 Validation 함수

## 6.1 Line Validation

```rust
pub fn validate_line(
    line: &Line,
    epsilon: f64,
) -> Result<(), GeometryError>
```

시작점과 끝점의 거리가 epsilon 이하라면 zero-length line으로 판단한다.

```text
distance(start, end) <= epsilon
```

부동소수점 좌표는 직접 비교하지 않는다.

```rust
// 권장하지 않는 방식
line.start == line.end
```

좌표를 직접 비교하면 아주 작은 계산 오차로 인해 퇴화 선분을 놓칠 수 있다.

---

## 6.2 Polyline Validation

```rust
pub fn validate_polyline(
    polyline: &Polyline,
    epsilon: f64,
) -> Result<(), GeometryError>
```

다음 조건을 검사한다.

1. 점이 두 개 미만인가
2. 모든 점이 사실상 같은 위치인가

점이 여러 개 있더라도 전부 같은 위치라면 유효한 경로가 아니다.

```text
[(1, 1), (1, 1), (1, 1)]
```

따라서 단순히 다음 조건만 검사해서는 부족하다.

```rust
polyline.points.len() >= 2
```

Polyline은 최소한 하나 이상의 유효한 구간을 가져야 한다.

---

## 6.3 Rectangle Validation

```rust
pub fn validate_rectangle(
    rectangle: &Rectangle,
) -> Result<(), GeometryError>
```

다음 조건을 검사한다.

```text
width > 0
height > 0
```

폭이나 높이가 0이면 면적이 없는 퇴화 도형이다.

현재 Rectangle 모델에서는 음수 크기도 허용하지 않는다.

---

## 6.4 Circle Validation

```rust
pub fn validate_circle(
    circle: &Circle,
) -> Result<(), GeometryError>
```

반지름은 반드시 0보다 커야 한다.

```text
radius > 0
```

반지름이 0인 원은 하나의 점으로 퇴화하며, 음수 반지름은 현재 Geometry 모델에서 의미를 갖지 않는다.

---

## 6.5 Arc Validation

```rust
pub fn validate_arc(
    arc: &Arc,
    angle_epsilon: f64,
) -> Result<(), GeometryError>
```

다음 조건을 검사한다.

- 반지름이 0보다 큰가
- sweep angle이 0으로 간주되지 않는가

Arc의 끝 각도는 다음과 같이 결정된다.

```text
end angle = start angle + sweep angle
```

sweep angle이 epsilon 이내로 0이라면 Arc의 시작점과 끝점이 같고 실제 길이도 0이 된다.

---

# 7. 측정 함수 구현

## 7.1 Polyline 길이

Polyline은 여러 점을 차례로 연결한 도형이다.

```text
P0 → P1 → P2 → P3
```

전체 길이는 인접한 점 사이 거리의 합이다.

```text
distance(P0, P1)
+ distance(P1, P2)
+ distance(P2, P3)
```

Rust에서는 `windows(2)`를 사용해 인접한 두 요소를 순회할 수 있다.

```rust
pub fn polyline_length(
    polyline: &Polyline,
) -> f64 {
    let segment_sum: f64 = polyline
        .points
        .windows(2)
        .map(|points| distance(points[0], points[1]))
        .sum();

    segment_sum
}
```

### `windows(2)`란 무엇인가

다음 배열이 있다고 가정한다.

```rust
let values = vec![1, 2, 3, 4];
```

`windows(2)`는 다음 슬라이스를 차례로 반환한다.

```text
[1, 2]
[2, 3]
[3, 4]
```

인덱스를 직접 관리하지 않아도 되므로 다음 오류를 줄일 수 있다.

- 마지막 인덱스 초과
- 잘못된 반복 범위
- 현재 점과 다음 점의 순서 오류

Polyline이 닫혀 있다면 마지막 점에서 첫 번째 점으로 돌아가는 구간도 추가한다.

```text
P0 → P1 → P2 → P3 → P0
```

---

## 7.2 Rectangle 둘레

축 정렬 Rectangle의 둘레는 다음과 같다.

```text
2 × (width + height)
```

```rust
pub fn rectangle_perimeter(
    rectangle: &Rectangle,
) -> f64 {
    2.0 * (rectangle.width + rectangle.height)
}
```

함수는 입력 Rectangle을 수정하지 않고 계산 결과만 반환한다.

---

## 7.3 Circle 둘레

원의 둘레는 다음 공식으로 계산한다.

```text
2πr
```

```rust
pub fn circle_circumference(
    circle: &Circle,
) -> f64 {
    2.0 * std::f64::consts::PI * circle.radius
}
```

Rust 표준 라이브러리는 π 상수를 제공한다.

```rust
std::f64::consts::PI
```

`3.14` 같은 값을 직접 작성하는 것보다 의도가 명확하고 정밀도도 일관된다.

---

## 7.4 Arc 길이

각도를 radian으로 저장한다면 호의 길이는 다음과 같다.

```text
radius × |sweep angle|
```

```rust
pub fn arc_length(arc: &Arc) -> f64 {
    arc.radius * arc.sweep_angle.abs()
}
```

`abs()`를 사용하는 이유는 sweep angle의 부호가 회전 방향을 나타낼 수 있기 때문이다.

다음 두 Arc는 방향은 다르지만 길이는 같다.

```text
+π/2 rad
-π/2 rad
```

---

## 7.5 Arc의 시작점과 끝점

중심 좌표가 `(cx, cy)`, 반지름이 `r`, 각도가 `θ`라면 원 위 좌표는 다음과 같이 계산한다.

```text
x = cx + r cos θ
y = cy + r sin θ
```

```rust
pub fn arc_start_point(arc: &Arc) -> Point2 {
    Point2 {
        x: arc.center.x
            + arc.radius * arc.start_angle.cos(),
        y: arc.center.y
            + arc.radius * arc.start_angle.sin(),
    }
}
```

끝점은 시작 각도에 sweep angle을 더해 계산한다.

```rust
pub fn arc_end_point(arc: &Arc) -> Point2 {
    let end_angle =
        arc.start_angle + arc.sweep_angle;

    Point2 {
        x: arc.center.x
            + arc.radius * end_angle.cos(),
        y: arc.center.y
            + arc.radius * end_angle.sin(),
    }
}
```

Rust의 `f64`는 다음과 같은 수학 메서드를 제공한다.

```rust
sin()
cos()
sqrt()
abs()
```

---

## 7.6 Rectangle 꼭짓점

Rectangle의 네 꼭짓점을 일정한 순서로 반환하는 함수를 추가했다.

```rust
pub fn rectangle_corners(
    rectangle: &Rectangle,
) -> [Point2; 4]
```

반환값을 `Vec<Point2>`가 아니라 `[Point2; 4]`로 표현할 수 있는 이유는 Rectangle의 꼭짓점 수가 항상 네 개로 고정되어 있기 때문이다.

```rust
[Point2; 4]
```

고정 크기 배열을 사용하면 다음 불변 조건을 타입에 표현할 수 있다.

> 이 함수는 반드시 네 개의 점을 반환한다.

반면 `Vec<Point2>`는 길이가 0일 수도 있고 5일 수도 있다.

가능하다면 고정된 규칙을 타입으로 표현하는 편이 안전하다.

꼭짓점 순서도 항상 시계 방향으로 고정했다.

```text
첫 번째 꼭짓점
    ↓
시계 방향으로 나머지 꼭짓점 반환
```

이 순서는 렌더링, 스냅, 선택 처리의 결과를 일관되게 유지한다.

---

# 8. 선분 교차 계산

## 8.1 무한 직선과 선분의 차이

두 점으로 정의되는 선을 다룰 때는 다음 두 개념을 구분해야 한다.

### 무한 직선

양방향으로 끝없이 이어지는 선이다.

### 선분

시작점과 끝점 사이에만 존재한다.

CAD에서 사용자가 생성한 `Line` Entity는 일반적으로 선분이다.

따라서 이번 함수는 무한 직선이 아니라 두 선분의 교차만 판정한다.

```rust
pub fn line_line_intersection(
    a: &Line,
    b: &Line,
) -> Option<Point2>
```

---

## 8.2 반환 타입으로 `Option<Point2>` 사용

두 선분은 항상 교차하지 않는다.

함수는 다음 두 상태를 표현해야 한다.

```text
교차함       → 교차점 반환
교차하지 않음 → 값 없음
```

Rust에서는 값이 없을 수 있는 결과를 `Option<T>`로 표현한다.

```rust
Option<Point2>
```

가능한 값은 다음과 같다.

```rust
Some(point)
None
```

호출자는 `if let`으로 결과를 처리할 수 있다.

```rust
if let Some(point) =
    line_line_intersection(&a, &b)
{
    println!(
        "intersection: ({}, {})",
        point.x,
        point.y,
    );
}
```

또는 `match`를 사용할 수 있다.

```rust
match line_line_intersection(&a, &b) {
    Some(point) => {
        println!("교차점: {point:?}");
    }
    None => {
        println!("교차하지 않음");
    }
}
```

`null`을 사용하는 언어와 달리, 호출자는 타입을 통해 값이 없을 가능성을 알 수 있다.

---

## 8.3 파라메트릭 선분 교차 공식

선분 A와 B는 다음과 같이 표현할 수 있다.

```text
A(t) = p + t r
B(u) = q + u s
```

각 기호의 의미는 다음과 같다.

- `p`, `q`: 각 선분의 시작점
- `r`, `s`: 각 선분의 방향 벡터
- `t`, `u`: 선분 위 위치를 나타내는 파라미터

두 선분이 교차하려면 같은 점을 가져야 한다.

```text
p + t r = q + u s
```

교차점이 실제 선분 내부에 있으려면 다음 조건도 만족해야 한다.

```text
0 ≤ t ≤ 1
0 ≤ u ≤ 1
```

`t` 또는 `u`가 범위를 벗어나면 무한 직선은 교차하지만 실제 선분은 교차하지 않는 것이다.

---

## 8.4 평행선과 epsilon

교차 계산의 분모가 0이면 두 방향 벡터가 평행하다.

그러나 부동소수점 계산에서는 다음처럼 직접 비교하지 않는다.

```rust
denominator == 0.0
```

대신 epsilon을 사용한다.

```rust
approx_eq(
    denominator,
    0.0,
    DEFAULT_EPSILON_MM,
)
```

분모가 epsilon 이내라면 평행하거나 일치하는 선분으로 보고 `None`을 반환한다.

이번 Phase에서는 겹치는 두 선분의 공통 구간을 별도로 표현하지 않았다.

다음 세 상태는 `Option<Point2>`만으로 모두 구분할 수 없기 때문이다.

- 교차하지 않음
- 하나의 점에서 교차
- 일정 구간이 겹침

향후 겹침 정보를 구분해야 한다면 다음과 같은 enum으로 확장할 수 있다.

```rust
pub enum LineIntersection {
    None,
    Point(Point2),
    Overlap {
        start: Point2,
        end: Point2,
    },
}
```

현재 MVP에서는 하나의 교차점만 필요하므로 범위를 넓히지 않았다.

---

# 9. 기하 변환

이번 Phase에서는 세 가지 기본 변환을 구현했다.

- 이동
- 회전
- 스케일

모든 함수는 기존 `Point2`를 직접 수정하지 않고 새로운 `Point2`를 반환한다.

---

## 9.1 이동

이동은 좌표에 delta를 더한다.

```rust
pub fn translate_point(
    point: Point2,
    delta: Point2,
) -> Point2 {
    Point2 {
        x: point.x + delta.x,
        y: point.y + delta.y,
    }
}
```

`Point2`가 `Copy`를 구현하고 있다면 값으로 전달해도 원래 값을 계속 사용할 수 있다.

```rust
let original = Point2 {
    x: 1.0,
    y: 2.0,
};

let moved = translate_point(
    original,
    Point2 {
        x: 3.0,
        y: 4.0,
    },
);
```

> **Rust 개념 — `Copy`**
>
> `Copy`를 구현한 작은 값 타입은 대입하거나 함수에 전달할 때 소유권이 이동되지 않고 값이 복사된다.
>
> 좌표처럼 작은 숫자 집합은 `Copy`와 잘 맞지만, 큰 `Vec`이나 `String`은 일반적으로 `Copy`를 구현하지 않는다.

---

## 9.2 회전

pivot을 기준으로 점을 회전하려면 다음 순서를 따른다.

1. pivot이 원점이 되도록 좌표를 이동한다.
2. 회전 행렬을 적용한다.
3. pivot 좌표를 다시 더한다.

먼저 pivot 기준 상대 좌표를 계산한다.

```text
translated_x = point.x - pivot.x
translated_y = point.y - pivot.y
```

회전 후 좌표는 다음과 같다.

```text
rotated_x = translated_x cos θ - translated_y sin θ
rotated_y = translated_x sin θ + translated_y cos θ
```

이를 Rust 함수로 표현하면 다음과 같다.

```rust
pub fn rotate_point(
    point: Point2,
    pivot: Point2,
    angle: f64,
) -> Point2 {
    let translated_x = point.x - pivot.x;
    let translated_y = point.y - pivot.y;

    let cosine = angle.cos();
    let sine = angle.sin();

    Point2 {
        x: pivot.x
            + translated_x * cosine
            - translated_y * sine,
        y: pivot.y
            + translated_x * sine
            + translated_y * cosine,
    }
}
```

`sin()`과 `cos()`를 지역 변수에 저장한 이유는 다음과 같다.

- 같은 값을 반복 계산하지 않는다.
- 회전 공식의 의미를 읽기 쉽게 만든다.
- 디버깅 시 중간 결과를 확인하기 쉽다.

---

## 9.3 스케일

스케일도 pivot을 기준으로 계산한다.

```rust
pub fn scale_point(
    point: Point2,
    pivot: Point2,
    scale_x: f64,
    scale_y: f64,
) -> Point2 {
    Point2 {
        x: pivot.x
            + (point.x - pivot.x) * scale_x,
        y: pivot.y
            + (point.y - pivot.y) * scale_y,
    }
}
```

스케일 값의 의미는 다음과 같다.

```text
1.0   원래 크기
2.0   두 배
0.5   절반
-1.0  pivot 기준 반전
```

현재 MVP UI는 이동 도구만 요구한다.

그러나 아키텍처 정책이 변환을 `cad_geometry`의 책임으로 명시하고 있으므로 회전과 스케일도 순수 함수로 준비했다.

실제 UI 도구로 노출할지는 Phase 8에서 별도로 결정한다.

---

# 10. 스냅 후보 추출

## 10.1 CAD에서 스냅이 필요한 이유

사용자가 마우스로 선의 끝점이나 원의 중심을 정확히 클릭하는 것은 어렵다.

CAD 프로그램은 포인터 주변에서 의미 있는 점을 찾아 자동으로 좌표를 맞춰 준다.

대표적인 스냅 지점은 다음과 같다.

- 선의 시작점
- 선의 끝점
- 선의 중점
- Polyline의 각 점
- Polyline 각 구간의 중점
- Rectangle의 꼭짓점
- 원의 중심
- Arc의 중심, 시작점, 끝점
- Text의 기준점

Phase 2에서는 전체 UI 스냅 시스템이 아니라, 도형별로 의미 있는 후보를 추출하는 순수 함수를 구현했다.

---

## 10.2 `EntityGeometry`와 enum dispatch

스냅 후보 함수는 도형 enum을 참조로 받는다.

```rust
pub fn snap_candidates(
    geometry: &EntityGeometry,
) -> Vec<Point2>
```

`EntityGeometry`가 다음과 같은 enum이라고 가정할 수 있다.

```rust
pub enum EntityGeometry {
    Line(Line),
    Polyline(Polyline),
    Rectangle(Rectangle),
    Circle(Circle),
    Arc(Arc),
    Text(Text),
}
```

도형마다 스냅 후보 계산 방식이 다르므로 `match`를 사용한다.

```rust
pub fn snap_candidates(
    geometry: &EntityGeometry,
) -> Vec<Point2> {
    match geometry {
        EntityGeometry::Line(line) => {
            // 시작점, 끝점, 중점
        }
        EntityGeometry::Polyline(polyline) => {
            // 모든 점과 구간 중점
        }
        EntityGeometry::Rectangle(rectangle) => {
            // 네 꼭짓점
        }
        EntityGeometry::Circle(circle) => {
            // 중심
        }
        EntityGeometry::Arc(arc) => {
            // 중심, 시작점, 끝점
        }
        EntityGeometry::Text(text) => {
            // origin
        }
    }
}
```

이 방식은 enum dispatch에 해당한다.

다형성을 trait object로 구현할 수도 있지만, 현재 지원하는 도형 집합이 명확하고 모든 도형을 빠짐없이 처리해야 하므로 enum과 `match`가 잘 맞는다.

새로운 도형 variant가 추가되면 기존 `match`가 불완전해지고 컴파일러가 수정해야 할 위치를 알려준다.

---

## 10.3 도형별 스냅 후보

| 도형        | 후보                  |
| ----------- | --------------------- |
| `Line`      | 시작점, 끝점, 중점    |
| `Polyline`  | 모든 점, 각 구간 중점 |
| `Rectangle` | 네 꼭짓점             |
| `Circle`    | 중심                  |
| `Arc`       | 중심, 시작점, 끝점    |
| `Text`      | `origin`              |

현재 Circle에는 중심점만 포함했다.

사분점이나 접점 스냅은 UI 요구가 구체화되는 시점에 추가할 수 있다.

---

## 10.4 `f64::midpoint`

선의 중점은 일반적으로 다음처럼 계산할 수 있다.

```rust
(a + b) / 2.0
```

Clippy 검사에서는 직접 중점 계산을 `f64::midpoint`로 교체하도록 안내했다.

```rust
let midpoint = Point2 {
    x: f64::midpoint(
        line.start.x,
        line.end.x,
    ),
    y: f64::midpoint(
        line.start.y,
        line.end.y,
    ),
};
```

`f64::midpoint`는 다음 장점이 있다.

- 코드가 중점 계산이라는 사실을 명확히 표현한다.
- `(a + b)`에서 중간 overflow가 발생할 가능성을 줄인다.
- 표준 라이브러리 API를 사용하므로 구현 의도가 통일된다.

---

# 11. 가장 가까운 스냅 후보 찾기

## 11.1 함수 역할

```rust
pub fn nearest_point(
    target: Point2,
    candidates: &[Point2],
    tolerance: f64,
) -> Option<Point2>
```

이 함수는 다음 절차를 수행한다.

1. target과 각 후보의 거리를 계산한다.
2. tolerance보다 먼 후보를 제외한다.
3. 남은 후보 중 가장 가까운 점을 선택한다.
4. 후보가 없으면 `None`을 반환한다.

---

## 11.2 왜 `&[Point2]`를 사용하는가

후보 목록은 다음 타입으로 받는다.

```rust
&[Point2]
```

이는 `Vec<Point2>`가 아니라 Point2의 slice를 빌린다는 의미다.

호출자는 다음 데이터를 모두 slice로 전달할 수 있다.

```rust
Vec<Point2>
[Point2; 4]
&[Point2]
```

예를 들어 다음이 가능하다.

```rust
let candidates = vec![
    point_a,
    point_b,
    point_c,
];

let nearest = nearest_point(
    target,
    &candidates,
    tolerance,
);
```

함수는 후보 목록을 읽기만 하므로 소유권을 가져올 필요가 없다.

> **Rust 개념 — Borrowing**
>
> `&[Point2]`는 후보 배열을 소유하지 않고 잠시 빌려 사용한다.
>
> 함수 호출이 끝난 뒤에도 원래 `Vec<Point2>`는 계속 사용할 수 있다. 불필요한 복사와 소유권 이동을 피할 수 있는 방식이다.

---

## 11.3 Iterator 체인

가장 가까운 점은 반복문으로 찾을 수도 있지만 Rust iterator를 사용하면 처리 단계를 선언적으로 표현할 수 있다.

개념적인 흐름은 다음과 같다.

```rust
candidates
    .iter()
    .map(...)
    .filter(...)
    .min_by(...)
```

각 메서드의 역할은 다음과 같다.

| 메서드     | 역할                         |
| ---------- | ---------------------------- |
| `iter()`   | 후보를 참조로 순회           |
| `map()`    | 후보와 거리 계산 결과를 연결 |
| `filter()` | tolerance 밖 후보 제거       |
| `min_by()` | 가장 작은 거리 선택          |

Iterator는 기본적으로 lazy하다.

즉 `min_by()` 같은 최종 소비 연산이 호출되기 전까지 실제 순회가 시작되지 않는다.

중간 `Vec`를 만들지 않고 한 번의 순회로 결과를 구할 수 있다.

---

## 11.4 `partial_cmp`가 필요한 이유

정수는 모든 값을 서로 비교할 수 있다.

그러나 `f64`에는 `NaN`이 존재한다.

`NaN`은 정상적인 대소 비교가 불가능하다.

```rust
f64::NAN < 1.0
f64::NAN > 1.0
f64::NAN == 1.0
```

따라서 `f64`는 전체 순서를 뜻하는 `Ord`가 아니라 부분 순서를 뜻하는 `PartialOrd`를 구현한다.

거리 비교에서는 다음과 같이 `partial_cmp`를 사용한다.

```rust
left_distance
    .partial_cmp(&right_distance)
    .expect(
        "distance values must not be NaN",
    )
```

`partial_cmp`는 비교할 수 없는 값이 포함되면 `None`을 반환한다.

`expect`는 이 경우 panic을 발생시킨다.

즉 이 함수는 좌표와 거리 계산값에 `NaN`이 들어오지 않는다는 불변 조건에 의존한다.

---

## 11.5 `# Panics` 문서화

Clippy의 `missing_panics_doc` 경고에 따라 공개 함수 문서에 `# Panics` 섹션을 추가했다.

```rust
/// Finds the nearest candidate within the tolerance.
///
/// # Panics
///
/// Panics if a calculated distance is NaN and
/// therefore cannot be compared using `f64::partial_cmp`.
pub fn nearest_point(
    target: Point2,
    candidates: &[Point2],
    tolerance: f64,
) -> Option<Point2> {
    // ...
}
```

공개 API 내부에서 다음 요소를 사용하는 경우 panic 가능성을 문서화해야 한다.

- `panic!`
- `unwrap()`
- `expect()`

Rust 문서 주석에서는 일반적으로 다음 섹션을 사용할 수 있다.

- `# Examples`
- `# Errors`
- `# Panics`
- `# Safety`

`Result`를 반환하는 함수는 `# Errors`에 실패 조건을 설명하고, panic 가능성이 있다면 `# Panics`에 별도로 작성한다.

---

## 11.6 결정적인 동률 처리

두 후보가 target에서 정확히 같은 거리에 있을 수 있다.

```text
candidate A ← 같은 거리 → target ← 같은 거리 → candidate B
```

이번 구현에서는 동률일 때 후보 목록에서 먼저 나온 점을 선택한다.

이 정책의 목적은 deterministic한 결과를 얻는 것이다.

결정적이라는 것은 같은 입력에 대해 항상 같은 출력을 반환한다는 의미다.

CAD 시스템에서는 다음 이유로 중요하다.

- 테스트 결과가 실행마다 달라지지 않는다.
- 스냅 선택이 프레임마다 바뀌지 않는다.
- 명령 실행 결과를 재현하기 쉽다.
- 포인터가 움직이지 않았는데 스냅 지점이 번갈아 선택되는 현상을 줄인다.
- 저장 및 출력 순서 정책과 일관성을 유지할 수 있다.

단순히 가장 가까운 점을 찾는 것뿐 아니라, 동률 처리 규칙까지 명시해야 완전한 동작 정책이 된다.

---

# 12. 이번 Phase에서 제외한 범위

## 12.1 원-원과 선-원 교차

이번 Phase에서는 선분과 선분의 교차만 구현했다.

다음 기능은 포함하지 않았다.

- 원과 원의 교차
- 선과 원의 교차
- Arc와 Line의 교차
- Arc와 Arc의 교차
- 겹치는 선분 구간 판정

이 기능들은 향후 다음 도구에서 필요할 수 있다.

- Trim
- Extend
- Fillet
- Chamfer
- 자동 분할
- 교차점 스냅

그러나 현재 MVP의 스냅과 선택 기능에는 당장 필요하지 않다.

필요하지 않은 기능을 미리 구현하면 다음 부담이 생긴다.

- API 설계 범위 증가
- 수치 안정성 문제 증가
- 테스트 경우의 수 증가
- 접선과 겹침 처리 정책 결정 필요
- 향후 요구사항 변경 시 재작업 가능성 증가

따라서 실제 사용 시점에 요구사항과 함께 설계하기로 했다.

---

## 12.2 Geometry Validation과 `ValidationReport` 연결

Phase 2의 `validate_*` 함수는 도형 단위 오류를 반환한다.

```rust
Result<(), GeometryError>
```

그러나 Phase 1에서 구현한 `cad_core::ValidationReport`에는 아직 자동으로 합쳐지지 않는다.

두 Validation 계층의 역할은 다르다.

### `cad_core` Validation

- 존재하지 않는 Layer 참조
- Drawing 내부 구조 정합성
- 객체 Stable ID를 포함한 문제 보고

### `cad_geometry` Validation

- zero-length line
- 0 이하의 반지름
- 잘못된 Rectangle 크기
- 잘못된 Arc sweep
- 퇴화 Polyline

향후 `cad_command` 계층에서 두 검증을 연결할 수 있다.

```text
CreateCircleCommand 실행
        ↓
cad_geometry::validate_circle
        ↓
실패 시 CommandError 또는 ValidationIssue 생성
        ↓
명령 실행 중단
```

종합 `ValidationReport`를 생성할 위치는 Phase 4에서 결정할 예정이다.

---

# 13. 테스트 전략

`cad_geometry`에는 기존 2개를 포함하여 총 24개의 테스트가 존재한다.

## 13.1 거리와 퇴화 판정

```text
calculates_distance_in_mm
detects_zero_length_line
```

기본 거리 공식과 epsilon 기반 퇴화 판정을 검증한다.

---

## 13.2 Validation

각 도형마다 정상 입력과 실패 입력을 모두 테스트한다.

```text
validate_line_*
validate_polyline_*
validate_rectangle_*
validate_circle_*
validate_arc_*
```

실패 케이스만 테스트하면 정상 도형까지 거부하는 잘못된 구현을 놓칠 수 있다.

Validation 테스트는 일반적으로 다음 두 방향이 필요하다.

```text
정상 입력   → Ok
잘못된 입력 → 기대한 GeometryError
```

---

## 13.3 측정 함수

```text
polyline_length_*
circle_circumference_matches_formula
arc_endpoints_match_start_and_sweep_angle
```

다음 내용을 검증한다.

- 열린 Polyline의 길이
- 닫힌 Polyline의 마지막-첫 점 구간
- 원의 둘레 공식
- Arc의 시작점과 끝점
- sweep angle 방향

---

## 13.4 교차

```text
line_line_intersection_finds_crossing_point
line_line_intersection_returns_none_for_parallel_lines
line_line_intersection_returns_none_outside_segment_extents
```

단순 교차뿐 아니라 다음 경계 조건을 검증한다.

- 평행한 선분
- 무한 직선은 만나지만 선분 범위 밖에서 만나는 경우
- 실제 선분 내부에서 교차하는 경우

기하 알고리즘은 정상 사례보다 경계 사례에서 버그가 발생하기 쉽다.

---

## 13.5 변환

```text
translate_point_shifts_by_delta
rotate_point_quarter_turn_about_origin
scale_point_scales_about_pivot
```

90도 회전 결과는 부동소수점 오차가 발생할 수 있으므로 직접 `==` 비교하지 않고 근사 비교를 사용한다.

---

## 13.6 스냅

```text
rectangle_corners_are_axis_aligned
snap_candidates_for_line_*
snap_candidates_for_text_*
nearest_point_*
```

스냅 테스트에서는 다음 항목을 검증한다.

- 후보 좌표
- 후보 반환 순서
- tolerance 적용
- tolerance 밖 후보 제외
- 같은 거리일 때 먼저 등장한 후보 선택

---

# 14. Rust 도구를 이용한 품질 검증

## 14.1 rustfmt

```bash
cargo fmt --all -- --check
```

Rust 표준 포맷 규칙에 맞는지 검사한다.

`--check`를 사용했기 때문에 파일을 자동으로 수정하지 않고, 포맷이 맞지 않으면 명령이 실패한다.

CI 환경에서 코드 스타일을 강제할 때 유용하다.

---

## 14.2 cargo test

```bash
cargo test --workspace
```

Workspace 전체 crate의 테스트를 실행한다.

검증 결과는 다음과 같다.

```text
cad_geometry: 24/24
workspace total: 36
```

특정 crate만 검사하지 않고 Workspace 전체를 테스트하여 다른 crate와의 호환성도 함께 확인했다.

---

## 14.3 Clippy

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Clippy는 Rust 코드의 잠재적 문제와 개선점을 검사하는 정적 분석 도구다.

옵션의 의미는 다음과 같다.

| 옵션            | 의미                                               |
| --------------- | -------------------------------------------------- |
| `--workspace`   | Workspace의 모든 crate 검사                        |
| `--all-targets` | library, binary, test, example 등 모든 target 검사 |
| `-D warnings`   | 모든 경고를 오류로 처리                            |

이번 Phase에서는 Clippy를 통해 두 가지를 수정했다.

### `manual_midpoint`

직접 계산한 중점을 표준 라이브러리 API로 교체했다.

```rust
// 수정 전
let midpoint = (a + b) / 2.0;

// 수정 후
let midpoint = f64::midpoint(a, b);
```

### `missing_panics_doc`

`partial_cmp().expect(...)`가 포함된 공개 함수에 `# Panics` 문서를 추가했다.

Clippy는 코드가 컴파일되는지만 확인하는 도구가 아니다.

더 명확하고 유지보수하기 좋은 Rust 코드를 작성하도록 돕는 정적 분석 도구다.

---

# 15. 검증 결과

```text
cargo fmt --all -- --check
통과

cargo test --workspace
전체 통과
- cad_geometry: 24/24
- workspace total: 36

cargo clippy --workspace --all-targets -- -D warnings
경고 없음
```

---

# 16. Phase 2에서 사용한 Rust 핵심 개념

## `Result<T, E>`

도형 Validation의 성공과 실패를 명시적으로 표현했다.

```rust
Result<(), GeometryError>
```

---

## `Option<T>`

교차점이나 스냅 후보처럼 값이 없을 수 있는 결과를 표현했다.

```rust
Option<Point2>
```

---

## Enum

도형별 오류와 도형 종류를 제한된 variant 집합으로 표현했다.

```rust
GeometryError
EntityGeometry
```

---

## Pattern Matching

도형 종류와 오류 종류에 따라 동작을 분기했다.

```rust
match geometry {
    // ...
}
```

---

## Trait

오류 타입에 Rust 표준 오류 동작을 부여했다.

```rust
Display
Error
```

---

## Borrowing과 Slice

후보 목록의 소유권을 가져오지 않고 참조만 사용했다.

```rust
candidates: &[Point2]
```

---

## Iterator

Polyline 구간 순회와 최단 거리 후보 탐색을 선언적으로 표현했다.

```rust
windows
map
filter
min_by
sum
```

---

## 고정 크기 배열

Rectangle이 정확히 네 개의 꼭짓점을 가진다는 조건을 타입에 반영했다.

```rust
[Point2; 4]
```

---

## 문서 주석

공개 API의 panic 조건을 `# Panics`로 문서화했다.

```rust
/// # Panics
```

---

## 표준 라이브러리 수학 API

수학 상수와 함수를 직접 구현하지 않고 표준 API를 사용했다.

```rust
std::f64::consts::PI
f64::midpoint
sin
cos
sqrt
abs
```

---

# 17. Phase 2 완료 결과

Phase 2를 통해 `cad_geometry`는 단순 거리 계산 모듈에서 CAD 도형 계산을 담당하는 독립적인 기하 계층으로 확장되었다.

이번 Phase에서 확보한 기반은 다음과 같다.

- 길이와 각도의 epsilon 정책
- 도형별 Geometry Validation
- Polyline, Rectangle, Circle, Arc 측정 함수
- Arc 시작점과 끝점 계산
- 선분-선분 교차 판정
- 이동·회전·스케일 변환
- 도형별 스냅 후보 추출
- tolerance 기반 최근접 후보 선택
- 결정적인 동률 처리
- 공통 Geometry 오류 타입
- 24개의 단위 테스트

핵심은 각각의 기능을 UI나 상태 관리와 결합하지 않고 순수 함수로 분리한 것이다.

이 구조 덕분에 이후 계층은 계산 로직을 다시 구현하지 않고 `cad_geometry`의 함수를 조합해 사용할 수 있다.

```text
cad_render
    └─ Arc endpoint와 Rectangle corner 재사용

cad_command
    └─ Geometry validation과 transform 재사용

cad_app
    └─ Snap candidate와 nearest point 재사용

cad_io
    └─ 저장 전 Geometry validation 재사용 가능
```

---

# 18. 남은 과제

## 교차 기능 확장

현재는 선분과 선분의 단일 교차점만 계산한다.

향후 다음 기능이 필요해지는 시점에 확장할 예정이다.

- Line–Circle
- Circle–Circle
- Arc–Line
- Arc–Arc
- 겹치는 선분
- 접선 판정

---

## Validation 통합

`cad_geometry`의 Validation 결과는 아직 `cad_core::ValidationReport`에 연결되지 않았다.

Phase 4에서 command 실행 전 검증 흐름을 설계할 예정이다.

---

## UI 스냅 연동

`snap_candidates`와 `nearest_point`는 아직 포인터 이벤트와 연결되지 않았다.

Phase 8에서는 다음 흐름으로 연결할 예정이다.

```text
화면 좌표 입력
    ↓
월드 좌표 변환
    ↓
주변 Entity 탐색
    ↓
snap_candidates
    ↓
nearest_point
    ↓
스냅 표시 및 좌표 보정
```

---

# 19. 다음 Phase

다음 단계는 `cad_tolerance` 확장이다.

## Phase 3 예정 범위

- Worst-case 공차 누적 계산
- 계산값과 표시값 분리
- 표시 전용 반올림
- 공차 방향과 부호 처리
- 부동소수점 계산 정책 유지

Phase 2가 도형의 공간적 계산을 담당했다면, Phase 3에서는 제조와 도면에서 중요한 치수 공차 계산 규칙을 구현한다.

---

# 마무리

Phase 2는 눈에 보이는 UI 기능을 추가한 단계는 아니다.

하지만 CAD 시스템에서 반복적으로 사용될 수학 계산과 검증 기준을 하나의 crate에 모았다는 점에서 중요한 기반 작업이다.

특히 Rust의 다음 특성을 활용해 기하 계층의 안전성을 높였다.

- `Result`와 `Option`을 통한 실패 가능성 명시
- enum과 pattern matching을 통한 도형·오류 분기
- borrowing을 통한 불필요한 소유권 이동 방지
- iterator를 통한 안전한 구간 처리
- `Display`와 `Error` trait를 이용한 오류 표준화
- 고정 크기 배열을 이용한 불변 조건 표현
- Clippy를 이용한 수치 계산과 문서 품질 개선

이제 상위 계층은 도형 계산을 다시 구현하지 않고 `cad_geometry`가 제공하는 순수 함수를 조합해 사용할 수 있다.

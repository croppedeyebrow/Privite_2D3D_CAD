# Rust로 CAD 엔진 만들기 — Phase 3: `cad_tolerance` 공차 누적과 표시 분리

## 들어가며

Phase 1에서는 `cad_core`에 Stable ID와 공용 Validation 타입을 마련했고, Phase 2에서는 `cad_geometry`가 점·선·원·호를 계산하고 검증할 수 있도록 확장했다.

이번 Phase 3의 대상은 `cad_tolerance`다. 도면에 그려진 치수는 하나의 정확한 숫자가 아니라 항상 허용 범위를 가진다.

```text
10.0 mm
```

라고 적혀 있어도 실제로 제조 가능한 값은 다음과 같은 범위다.

```text
9.8 mm ~ 10.2 mm
```

CAD Studio는 이 범위를 다음 네 가지 방식(`ToleranceSpec`)으로 표현하고 있었다.

- `None` — 공차 없음
- `Symmetric` — 대칭 공차 (`10.0 ± 0.2`)
- `Bilateral` — 상하 공차 (`10.0 +0.3/-0.1`)
- `Limit` — 한계 치수 (`9.8 ~ 10.2`)

Phase 2까지는 이 네 가지를 **개별적으로** 계산하는 `calculate` 함수만 있었다. 이번 Phase에서는 여기에 두 가지를 더한다.

1. 여러 치수를 이어 붙였을 때(스택업) 전체 공차가 얼마나 벌어지는지 계산하는 **누적(accumulate)**
2. 화면에 보여줄 반올림된 숫자와, 내부에서 계속 사용할 정밀한 숫자를 분리하는 **표시 규칙**

작업을 시작하기 전에 문서들 사이에서 충돌을 하나 발견했는데, 이 이야기부터 정리한다.

---

## 1. Phase 3의 목적

`03_백엔드_구조_정책.md`는 `cad_tolerance`에 대해 다음을 요구한다.

- 계산 결과는 `nominal`, `min`, `max`를 포함해야 하며 모든 계산은 trace를 남긴다.
- 공차 표시용 반올림값을 내부 계산에 재사용하지 않는다.
- MVP 누적 계산은 worst-case 방식으로 한다.

`02_아키텍처_정책.md`도 `cad_tolerance`의 책임을 "공차 범위, 누적, 끼워맞춤, 계산 trace"로 정의하고 있다.

Phase 3의 목표는 이 두 문서가 요구하는 worst-case 누적 계산과 표시/계산 분리 규칙을 실제 함수와 테스트로 만드는 것이다.

---

## 2. 문서 충돌: 공차 누적은 MVP인가

작업을 시작하기 전에 정책 문서 세 개를 다시 확인했는데, 서로 다른 말을 하고 있었다.

| 문서 | 내용 |
|---|---|
| `01_제품_기획_정책.md` | **후순위(비MVP)** 목록에 "공차 누적 계산" 포함 |
| `02_아키텍처_정책.md` | `cad_tolerance` 책임에 "누적" 포함 |
| `03_백엔드_구조_정책.md` | "**MVP** 누적 계산은 worst-case 방식으로 한다" |

`01` 문서는 누적 계산을 MVP 이후로 미루라고 하고, `03` 문서는 그것이 MVP에 포함된 것처럼 서술하고 있다. 두 문서가 서로 다른 결론을 가리키는 상태에서 임의로 하나를 선택하면, 나중에 "왜 이 기능이 이 시점에 들어갔는가"를 설명할 수 없게 된다.

> ⚠️ **왜 그냥 진행하지 않았는가?**
>
> `06_AI_개발_작업_명령서.md`는 "범위를 넓히는 기능은 먼저 제안하고 승인을 받는다"고 명시한다.
> 두 문서가 상충하는 상황은 범위가 넓어질 수도, 좁아질 수도 있는 상황이다.
> 어느 쪽으로 판단하든 사용자가 의도한 범위와 다를 위험이 있어, 판단을 미루지 않고 먼저 확인했다.

### 결정

사용자에게 두 가지 선택지를 제시했다.

1. 이번 Phase에 포함 (`03` 문서 우선)
2. 후순위로 미룸 (`01` 문서 우선, 표시/계산 분리만 진행)

**결정: 이번에 포함한다.** `03_백엔드_구조_정책.md`를 우선 기준으로 삼아 worst-case 누적 계산을 Phase 3 범위에 넣었다.

---

## 3. Phase 2까지의 `cad_tolerance`

Phase 3 시작 시점의 `cad_tolerance`는 다음 세 타입과 함수 하나로 이루어져 있었다.

```rust
pub enum ToleranceSpec {
    None,
    Symmetric { nominal: LengthMm, plus_minus: LengthMm },
    Bilateral { nominal: LengthMm, upper: LengthMm, lower: LengthMm },
    Limit { min: LengthMm, max: LengthMm },
}

pub struct ToleranceRange {
    pub nominal: LengthMm,
    pub min: LengthMm,
    pub max: LengthMm,
}

pub struct CalculationTrace {
    pub expression: String,
    pub result: ToleranceRange,
    pub warnings: Vec<String>,
}

pub fn calculate(spec: ToleranceSpec) -> CalculationTrace { /* ... */ }
```

`calculate`는 공차 스펙 **하나**를 받아 `nominal`/`min`/`max`와 사람이 읽을 수 있는 `expression`, 그리고 경고 목록을 반환한다. 이 구조 자체는 이미 "계산 결과는 nominal, min, max를 포함하고 trace를 남긴다"는 정책을 만족하고 있었다.

빠져 있던 것은 "여러 개의 스펙을 하나로 합치는 방법"과 "표시용 반올림과 계산용 정밀도를 분리하는 장치"였다.

---

## 4. Rust 복습 — 데이터를 담는 Enum

`ToleranceSpec`은 Rust enum의 각 variant가 서로 다른 필드를 가질 수 있다는 특징을 활용한다.

```rust
pub enum ToleranceSpec {
    None,
    Symmetric { nominal: LengthMm, plus_minus: LengthMm },
    // ...
}
```

`None`은 데이터가 없고, `Symmetric`은 `nominal`과 `plus_minus`라는 이름 있는 필드를 가진다. 다른 언어라면 이를 표현하기 위해 클래스 계층이나 nullable 필드가 잔뜩 있는 구조체를 만들어야 할 수도 있다.

```rust
// 다른 언어에서 흔한 방식 (의사 코드)
struct ToleranceSpec {
    kind: String,          // "none" | "symmetric" | "bilateral" | "limit"
    nominal: Option<f64>,
    plus_minus: Option<f64>,
    upper: Option<f64>,
    lower: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
}
```

이 방식은 `kind`가 `"symmetric"`인데 `upper`가 채워져 있는 것 같은, 타입 시스템이 막아주지 못하는 상태를 허용한다. Rust의 enum은애초에 그런 조합을 만들 수 없게 한다 — `Symmetric` variant에는애초에 `upper` 필드가 존재하지 않는다.

`match`로 분해할 때도 각 variant가 어떤 필드를 갖는지 컴파일러가 알고 있다.

```rust
match spec {
    ToleranceSpec::Symmetric { nominal, plus_minus } => { /* ... */ }
    ToleranceSpec::Limit { min, max } => { /* ... */ }
    // ...
}
```

---

## 5. `calculate` 다시 보기

`calculate`는 4가지 variant를 각각 `(nominal, min, max)` 튜플로 변환한다.

```rust
let (nominal, min, max) = match spec {
    ToleranceSpec::None => (0.0, 0.0, 0.0),
    ToleranceSpec::Symmetric { nominal, plus_minus } => (
        nominal.0,
        nominal.0 - plus_minus.0,
        nominal.0 + plus_minus.0,
    ),
    ToleranceSpec::Bilateral { nominal, upper, lower } => {
        (nominal.0, nominal.0 - lower.0, nominal.0 + upper.0)
    }
    ToleranceSpec::Limit { min, max } => (min.0.midpoint(max.0), min.0, max.0),
};
```

`Limit`의 경우 사용자가 `min`/`max`만 지정하므로, `nominal`은 둘의 중간값(`f64::midpoint`)으로 역산한다.

이후 `min > max`이면 경고를 추가한다. 이 경고는 이번 Phase에서도 그대로 유지했고, `accumulate`가 이 경고를 전파하는 데 사용된다.

```rust
let mut warnings = Vec::new();
if min > max {
    warnings.push("minimum exceeds maximum".to_owned());
}
```

---

## 6. Worst-case 공차 누적 설계

### 6.1 공차 누적(스택업)이란

여러 개의 치수를 이어 붙여 하나의 전체 길이를 만드는 경우, 각 구간의 공차가 전체 길이의 공차에 그대로 영향을 준다.

```text
구간 A: 10.0 ± 0.1 mm
구간 B: 20.0 ± 0.2 mm
───────────────────────
전체 길이 = A + B
```

각 구간이 허용 범위 안에서 얼마든지 벌어질 수 있으므로, 전체 길이의 공차도 계산해서 보여줘야 한다. 이 계산을 "공차 누적" 또는 "치수 스택업(dimension stack-up)"이라고 부른다.

### 6.2 Worst-case 방식 vs 통계적 방식

공차를 누적하는 방법은 여러 가지가 있다.

| 방식 | 계산 | 특징 |
|---|---|---|
| **Worst-case** | 각 구간의 min을 모두 더하고, max를 모두 더한다 | 모든 구간이 동시에 최악의 방향으로 벌어지는 경우까지 보장. 계산이 단순하고 항상 안전한 쪽으로 넓게 잡힘 |
| 통계적(RSS 등) | 표준편차 기반으로 제곱합의 제곱근을 사용 | 실제 제조 분포를 반영해 더 좁은 범위를 제시하지만, 통계적 가정이 필요하고 계산이 복잡함 |

`03_백엔드_구조_정책.md`는 "MVP 누적 계산은 worst-case 방식으로 한다"고 명시했으므로, 이번 Phase에서는 단순 합산만 구현한다. 통계적 방식은 다루지 않는다.

> 📌 **Worst-case가 "단순 합산"인 이유**
>
> 각 구간의 최솟값을 합치면 "모든 구간이 동시에 가장 작게 나온 경우"의 전체 길이가 되고,
> 각 구간의 최댓값을 합치면 "모든 구간이 동시에 가장 크게 나온 경우"의 전체 길이가 된다.
> 이 두 극단 사이에 실제로 나올 수 있는 모든 조합이 들어가므로, min/max를 그대로 더하는 것만으로 안전한 상한·하한을 얻을 수 있다.

### 6.3 `accumulate` 함수

```rust
pub fn accumulate(specs: &[ToleranceSpec]) -> CalculationTrace {
    let mut nominal_sum = 0.0;
    let mut min_sum = 0.0;
    let mut max_sum = 0.0;
    let mut warnings = Vec::new();
    let mut terms = Vec::new();

    if specs.is_empty() {
        warnings.push("no tolerance specs to accumulate".to_owned());
    }

    for spec in specs {
        let trace = calculate(*spec);
        nominal_sum += trace.result.nominal.0;
        min_sum += trace.result.min.0;
        max_sum += trace.result.max.0;
        warnings.extend(trace.warnings);
        terms.push(trace.expression);
    }

    CalculationTrace {
        expression: format!(
            "worst-case sum of [{}] -> {nominal_sum:.9} -> [{min_sum:.9}, {max_sum:.9}] mm",
            terms.join(", ")
        ),
        result: ToleranceRange {
            nominal: LengthMm(nominal_sum),
            min: LengthMm(min_sum),
            max: LengthMm(max_sum),
        },
        warnings,
    }
}
```

핵심 아이디어는 단순하다. **각 스펙을 기존 `calculate`로 계산한 뒤, 그 결과의 `nominal`/`min`/`max`를 그대로 더한다.** 새로운 공식을 만들지 않고 기존 함수를 재사용했다.

### 6.4 슬라이스로 입력 받기

`accumulate`는 `Vec<ToleranceSpec>`이 아니라 `&[ToleranceSpec]`(슬라이스)를 받는다.

```rust
pub fn accumulate(specs: &[ToleranceSpec]) -> CalculationTrace
```

호출하는 쪽은 `Vec`, 배열, 슬라이스 어느 것이든 그대로 전달할 수 있다.

```rust
let specs = vec![/* ... */];
accumulate(&specs);

let specs = [/* ... */];
accumulate(&specs);
```

함수가 데이터를 소유할 필요가 없다면 슬라이스로 받는 편이, 호출자에게 불필요한 소유권 이전이나 복사를 강요하지 않는다.

`for spec in specs`에서 `spec`의 타입은 `&ToleranceSpec`이다. `calculate`는 `ToleranceSpec`(참조가 아닌 값)을 받으므로 `*spec`으로 역참조해서 넘긴다.

```rust
let trace = calculate(*spec);
```

이것이 가능한 이유는 `ToleranceSpec`이 `Copy`를 derive하고 있기 때문이다.

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToleranceSpec { /* ... */ }
```

`Copy` 타입은 역참조 시 소유권을 "이동(move)"시키지 않고 값을 복사한다. 그래서 `specs` 슬라이스는 반복이 끝난 뒤에도 그대로 유효하다.

### 6.5 경고 전파와 trace 합성

`accumulate`는 각 스펙의 경고를 무시하지 않고 모아서 반환한다.

```rust
warnings.extend(trace.warnings);
```

`extend`는 다른 컬렉션의 요소들을 현재 `Vec`의 끝에 이어붙인다. 예를 들어 두 번째 구간에 `min > max`인 잘못된 `Limit` 스펙이 있으면, 그 경고가 누적 결과의 경고 목록에도 나타난다. 개별 계산의 문제를 상위 계산이 삼켜버리지 않도록 하기 위함이다.

`expression`도 각 스펙의 `expression`을 이어 붙여서 만든다.

```rust
terms.push(trace.expression);
// ...
terms.join(", ")
```

`join`은 `Vec<String>`의 각 요소 사이에 구분자를 넣어 하나의 `String`으로 합친다. 최종 `expression`을 보면 "각 구간이 어떻게 계산됐고, 그것들이 어떻게 합산됐는지"를 한 줄로 추적할 수 있다 — `03_백엔드_구조_정책.md`가 요구하는 "모든 계산은 trace를 남긴다"를 누적 계산에도 그대로 적용한 것이다.

---

## 7. 계산값과 표시값 분리

### 7.1 왜 반올림을 계산에 재사용하면 안 되는가

`03_백엔드_구조_정책.md`는 "공차 표시용 반올림값을 내부 계산에 재사용하지 않는다"고 못박고 있다. 이 규칙이 왜 필요한지 예를 들어본다.

화면에는 소수점 둘째 자리까지만 보여준다고 가정한다.

```text
9.333 mm  →  화면 표시: 9.33 mm
```

만약 이 반올림된 `9.33`을 다음 계산의 입력으로 다시 사용하면, 계산을 반복할수록 반올림 오차가 누적된다.

```text
1단계: 9.333... → 표시용 9.33 사용 → 다음 계산 입력
2단계: 9.33 기준으로 계산 → 또 반올림 → 다음 계산 입력
3단계: ...
```

이런 오차 누적은 특히 공차처럼 여러 구간을 이어 붙이는 계산에서 위험하다. 개별 오차는 작아도 여러 번 누적되면 실제 제조 허용 범위를 벗어난 결과를 "정상"이라고 판단하게 만들 수 있다.

그래서 이번 구현은 두 종류의 값을 명확히 분리했다.

```text
ToleranceRange { nominal, min, max }   → 항상 전체 정밀도(f64) 유지, 계산에만 사용
round_for_display / format_range_for_display → 화면에 보여줄 때만 사용, 반환값을 다시 계산에 넣지 않음
```

### 7.2 `round_for_display`

```rust
pub fn round_for_display(value: LengthMm, decimals: u32) -> f64 {
    let factor = 10f64.powi(i32::try_from(decimals).unwrap_or(i32::MAX));
    (value.0 * factor).round() / factor
}
```

동작 방식은 다음과 같다.

```text
decimals = 2
factor = 10^2 = 100

value = 9.3333...
value * factor = 933.33...
.round() = 933.0
933.0 / factor = 9.33
```

`decimals: u32`를 `i32::powi`가 요구하는 `i32`로 바꾸는 부분에서 `i32::try_from(decimals).unwrap_or(i32::MAX)`를 사용했다. `u32`가 표현할 수 있는 최댓값이 `i32::MAX`를 넘는 극단적인 경우에도 패닉 대신 `i32::MAX`로 대체하도록 했다 — 물론 실제로 소수점 20억 자리까지 반올림을 요청하는 호출은 없겠지만, 패닉 가능성을 명시적으로 없애는 편이 안전하다.

이 함수는 **새로운 `f64` 값을 반환할 뿐, 입력으로 받은 `LengthMm`을 변경하지 않는다.** `LengthMm`은 `Copy` 타입이므로애초에 원본을 변경할 방법도 없다 — 함수는 항상 값을 복사해서 받는다.

### 7.3 `format_range_for_display`와 캡처된 포맷 인자

```rust
pub fn format_range_for_display(range: &ToleranceRange, decimals: usize) -> String {
    format!(
        "{:.decimals$} [{:.decimals$}, {:.decimals$}] mm",
        range.nominal.0, range.min.0, range.max.0
    )
}
```

`{:.decimals$}`는 Rust의 포맷 문자열에서 정밀도(precision)를 **변수로** 지정하는 문법이다.

```text
{:.2}          → 소수점 둘째 자리로 고정
{:.decimals$}  → decimals 변수의 값만큼 소수점 자리 사용
```

Rust 2021 에디션부터 포맷 문자열은 스코프에 있는 변수 이름을 `{이름}` 형태로 직접 캡처할 수 있는데, 이 규칙은 폭(width)이나 정밀도(precision)에도 동일하게 적용된다. 그래서 `decimals`를 별도의 위치 인자로 전달하지 않아도 함수 인자 `decimals`를 그대로 참조할 수 있다.

이 함수도 `&ToleranceRange`를 **참조**로만 받고 새 `String`을 반환한다. 원본 `range`의 `nominal`/`min`/`max`는 함수 호출 전후로 완전히 동일하다 — 이는 아래 테스트에서 직접 확인한다.

```rust
let formatted = format_range_for_display(&range, 2);
assert_eq!(formatted, "10.00 [9.33, 10.33] mm");
// 원본은 그대로 전체 정밀도를 유지한다.
assert!((range.min.0 - (9.0 + 1.0 / 3.0)).abs() < 1.0e-12);
```

---

## 8. 부동소수점 비교의 함정 — 실제로 겪은 실패

테스트를 작성하는 과정에서 다음 코드가 실패했다.

```rust
let trace = accumulate(&specs); // 10.0±0.1 과 20.0±0.2 를 누적
assert_eq!(trace.result.min, LengthMm(29.7));
```

실행 결과는 다음과 같았다.

```text
assertion `left == right` failed
  left: LengthMm(29.700000000000003)
 right: LengthMm(29.7)
```

`9.9 + 19.8`을 계산했을 뿐인데 결과가 정확히 `29.7`이 아니었다. 이것은 버그가 아니라 이진 부동소수점의 근본적인 한계다 — `0.1 + 0.2 != 0.3`과 같은 종류의 문제다.

`cad_geometry`를 만들 때 이미 "부동소수점은 직접 비교하지 않는다"는 원칙을 세워뒀지만(Phase 2), 정작 `cad_tolerance`의 테스트 코드에서는 `assert_eq!`로 직접 비교를 하고 있었다. 이 실패는 그 원칙을 코드로 다시 한번 확인시켜준 사례다.

수정은 다음과 같이 오차 허용 비교로 바꿨다.

```rust
assert!((trace.result.min.0 - 29.7).abs() < 1.0e-9);
```

> 💡 **원칙과 실제 실수는 별개다**
>
> "부동소수점을 직접 비교하지 않는다"는 규칙을 `cad_geometry`의 프로덕션 코드에는 적용했지만,
> 다른 crate의 테스트 코드를 작성할 때는 습관적으로 `assert_eq!`를 먼저 썼다.
> 이 경험은 정책을 문서로 알고 있는 것과, 코드를 작성하는 매 순간 적용하는 것이 다르다는 점을 보여준다.
> 테스트가 실패한 덕분에 실제로 걸러졌다는 점에서, 테스트를 작성하는 목적이 다시 한번 확인된 셈이다.

---

## 9. 테스트

`crates/cad_tolerance/src/lib.rs`의 `tests` 모듈에는 총 9개의 테스트가 있다 (기존 1개 포함).

| 테스트 | 검증 내용 |
|---|---|
| `symmetric_tolerance_is_deterministic` | 대칭 공차의 min/max 계산 (기존) |
| `bilateral_tolerance_uses_upper_and_lower` | 상하 공차가 upper/lower를 올바르게 사용하는지 |
| `limit_tolerance_derives_nominal_as_midpoint` | 한계 치수에서 nominal이 중간값으로 역산되는지 |
| `calculate_warns_when_minimum_exceeds_maximum` | min > max일 때 경고가 붙는지 |
| `accumulate_sums_worst_case_min_and_max` | 두 대칭 공차를 누적하면 worst-case로 합산되는지 |
| `accumulate_propagates_warnings_from_each_spec` | 개별 스펙의 경고가 누적 결과에도 남는지 |
| `accumulate_warns_on_empty_input` | 빈 입력에 대해 경고를 반환하는지 |
| `round_for_display_does_not_change_source_value` | 반올림 결과와 원본 값이 분리돼 있는지 |
| `format_range_for_display_rounds_without_mutating_range` | 문자열 포맷팅이 원본 range를 바꾸지 않는지 |

---

## 10. 검증 결과

```text
cargo fmt --all -- --check
통과

cargo test --workspace
전체 통과
- cad_tolerance: 9/9
- workspace total: 44

cargo clippy --workspace --all-targets -- -D warnings
경고 없음
```

---

## 11. Phase 3에서 사용한 Rust 핵심 개념

### 데이터가 있는 Enum

`ToleranceSpec`의 각 variant가 서로 다른 필드 집합을 가지도록 표현했다.

```rust
Symmetric { nominal: LengthMm, plus_minus: LengthMm }
Limit { min: LengthMm, max: LengthMm }
```

### 슬라이스

소유권을 가져오지 않고 여러 스펙을 순회했다.

```rust
specs: &[ToleranceSpec]
```

### `Copy` 트레잇과 역참조

값 타입인 `ToleranceSpec`을 참조에서 복사해 함수에 그대로 넘겼다.

```rust
calculate(*spec)
```

### `Vec::extend` / `Vec::join`

여러 계산의 경고와 trace 문자열을 하나로 합쳤다.

```rust
warnings.extend(trace.warnings);
terms.join(", ")
```

### 캡처된 포맷 인자

변수 이름을 포맷 문자열의 정밀도 지정자로 직접 사용했다.

```rust
format!("{:.decimals$}", value)
```

### `TryFrom`과 `unwrap_or`

패닉 가능성을 없애기 위해 실패할 수 있는 변환에 안전한 대체값을 지정했다.

```rust
i32::try_from(decimals).unwrap_or(i32::MAX)
```

### 근사 비교

부동소수점 결과를 검증할 때 정확한 동등 비교 대신 오차 허용 비교를 사용했다.

```rust
(a - b).abs() < 1.0e-9
```

---

## 12. Phase 3 완료 결과

Phase 3를 통해 `cad_tolerance`는 개별 공차 하나만 계산하던 상태에서, 여러 공차를 이어 붙인 전체 스택업까지 계산할 수 있는 상태로 확장됐다.

이번 Phase에서 확보한 기반은 다음과 같다.

- worst-case 방식의 공차 누적 계산
- 누적 계산에서도 유지되는 trace와 경고 전파
- 표시용 반올림과 내부 계산값의 명확한 분리
- 근사 비교를 사용하는 부동소수점 테스트 관행

---

## 13. 남은 과제

### 통계적 누적 방식 미포함

이번 Phase는 worst-case 방식만 구현했다. RSS(제곱합의 제곱근) 같은 통계적 누적은 `01_제품_기획_정책.md`에서도 별도로 언급되지 않았고, 필요성이 확인되기 전까지는 추가하지 않는다.

### `cad_command`/`cad_io`와 아직 연결되지 않음

`calculate`와 `accumulate`는 아직 어떤 command나 저장 파이프라인에서도 호출되지 않는다. Phase 4(`cad_command` 확장)에서 Dimension에 `ToleranceSpec`을 적용하는 command를 만들 때 연결한다.

### UI 표시 연동 없음

`round_for_display`/`format_range_for_display`는 아직 어떤 화면 요소와도 연결돼 있지 않다. Phase 8(UI)에서 속성 패널에 공차 계산 결과와 trace를 보여줄 때 사용한다 (`04_UI_와이어프레임_정책.md`: "공차 계산 결과와 계산 trace를 숨기지 않는다").

---

## 14. 다음 Phase

다음 단계는 `cad_command` 확장이다.

### Phase 4 예정 범위

- Layer/Dimension 대상 command 추가
- 이동/수정 command 추가
- command 실행 시 `cad_core::ValidationReport`, `cad_geometry`의 validation, `cad_tolerance`의 계산을 한데 모으는 흐름 설계
- undo/redo가 새 command에도 적용되는지 확인
- replay 지원

Phase 3까지는 각 계층이 독립적으로 계산과 검증을 제공하는 단계였다면, Phase 4에서는 이 계층들을 실제 사용자 조작(command)으로 엮는다.

---

## 마무리

Phase 3는 코드 양으로는 크지 않지만, 두 가지를 분명히 남겼다.

첫째, 정책 문서끼리 상충할 때는 추측으로 넘어가지 않고 먼저 확인한다는 원칙을 실제로 적용했다 (`01`과 `03`의 "공차 누적" 범위 충돌).

둘째, "부동소수점을 직접 비교하지 않는다"는 규칙이 프로덕션 코드뿐 아니라 테스트 코드에도 예외 없이 적용돼야 한다는 것을, 실패한 테스트를 통해 다시 확인했다.

두 경험 모두 코드로는 드러나지 않지만, 이후 Phase에서 같은 실수를 반복하지 않게 해주는 기록이다.

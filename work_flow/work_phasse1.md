# Phase 1 — cad_core 기반 정책 구현

## 목적

`03_백엔드_구조_정책.md`에서 정의한 **Stable ID 정책**과 **Validation(오류/경고) 정책**을 `cad_core`에 반영하여, 이후 구현될 `cad_geometry`, `cad_tolerance`, `cad_command`, `cad_io`, `cad_app`에서 공통으로 사용할 기반을 마련한다.

> 💡 **왜 필요한가?**
>
> CAD 프로젝트의 모든 모듈이 동일한 ID 체계와 검증 방식을 사용해야 하기 때문이다.
> 만약 각 모듈이 서로 다른 방식으로 ID를 관리하거나 오류를 표현한다면,
> 저장·불러오기·Undo/Redo·명령 실행 시 일관성을 유지하기 어려워진다.

---

# 배경

## Phase 0 완료 상태

Phase 0에서는 `cad_core`의 `Entity`, `Layer`, `Dimension` 확장 과정에서 발생했던 Workspace 빌드 오류를 모두 해결하였다.

당시 `cad_core`에는 다음 Stable ID만 존재했다.

- `EntityId`
- `LayerId`
- `DimensionId`

하지만 정책 문서에서는 아래 ID들도 요구하고 있었다.

- `ProjectId`
- `DrawingId`
- `SheetId`

또한 프로젝트 전체에서 사용할 Validation 타입도 존재하지 않았다.

이번 Phase에서는 이러한 공백을 메우는 작업을 진행한다.

> 📌 **Stable ID란?**
>
> 프로그램이 실행되는 동안뿐 아니라 저장 후 다시 불러와도
> **절대로 변하지 않는 고유 ID**를 의미한다.
>
> 예를 들어 Entity가 삭제되거나 순서가 변경되어도
> `EntityId(25)`는 항상 동일한 객체를 의미해야 한다.

---

# 설계 결정 사항

## Sheet 범위 결정

정책 문서에는 `SheetId`라는 이름만 정의되어 있었지만,
실제 `Sheet` 구조가 무엇인지는 명시되어 있지 않았다.

예상되는 정보는 다음과 같다.

- 용지 크기
- 출력 영역
- 출력 배율
- 출력 대상

그러나 지금 이를 설계하면
향후 저장 포맷까지 변경될 가능성이 있었다.

### 최종 결정

- `SheetId`만 추가
- Project/Drawing에는 연결하지 않음
- SVG 출력 기능 구현 시 다시 설계

> ⚠️ **왜 지금 만들지 않았는가?**
>
> 아직 요구사항이 명확하지 않은 데이터를 먼저 설계하면
> 나중에 저장 포맷까지 수정해야 하는 일이 발생할 수 있다.
>
> 따라서 "ID만 먼저 준비"하는 것이 변경 비용이 가장 적다.

---

# 구현 내용

## 1. Stable ID 추가

기존과 동일한 `stable_id!` 매크로를 사용하였다.

```rust
stable_id!(ProjectId);
stable_id!(DrawingId);
stable_id!(EntityId);
stable_id!(LayerId);
stable_id!(DimensionId);
stable_id!(SheetId);
```

새롭게 추가된 ID

- ProjectId
- DrawingId
- SheetId

> 💡 **왜 매크로를 사용하는가?**
>
> Stable ID는 내부 구조가 모두 동일하다.
>
> 매번 같은 코드를 작성하기보다
> 매크로 하나로 생성하면 코드 중복을 줄일 수 있다.

---

## 2. Project / Drawing ID 연결

### Drawing

추가

- `id: DrawingId`
- `DEFAULT_DRAWING_ID = DrawingId::new(0)`

### Project

추가

- `id: ProjectId`
- `DEFAULT_PROJECT_ID = ProjectId::new(0)`

두 타입 모두 `Default`를 직접 구현하였다.

> 💡 **왜 Default를 직접 구현했는가?**
>
> Rust의 자동 Default는 우리가 원하는
> "항상 ID가 0부터 시작"이라는 정책을 보장하지 않는다.
>
> 따라서 프로젝트 정책을 명확히 유지하기 위해 수동 구현하였다.

---

## 3. Validation 타입 정의

Validation 정책을 코드로 표현하였다.

...

> 📌 **Validation이란?**
>
> 객체가 현재 규칙을 만족하는지 검사하는 과정이다.
>
> 예를 들어
>
> - 존재하지 않는 Layer 참조
> - 음수 반지름
> - 잘못된 Arc
>
> 등을 검사하게 된다.

...

> 💡 **왜 Error와 Warning을 나누는가?**
>
> 모든 문제가 프로그램 실행을 막아야 하는 것은 아니다.
>
> 예를 들어
>
> Error
>
> - 존재하지 않는 Layer
> - 저장 불가능
>
> Warning
>
> - 권장하지 않는 값
> - 자동 수정 가능
>
> 처럼 심각도를 구분하기 위함이다.

---

## 4. Drawing::validate()

이번 Phase에서는

"구조적 정합성"

만 검사한다.

### 검사 대상

- Entity의 Layer 존재 여부
- Dimension의 Layer 존재 여부

...

> ⚠️ **왜 Geometry Validation은 하지 않는가?**
>
> 현재 의존성 구조는
>
> ```
> cad_geometry
>        │
>        ▼
> cad_core
> ```
>
> 이다.
>
> Rust에서는 아래 계층이 위 계층을 참조할 수 있지만,
> 반대로 위 계층이 아래 계층을 참조할 수는 없다.
>
> 따라서 Geometry Validation은
> `cad_geometry`에서 구현하는 것이 올바른 구조이다.

---

# 테스트

...

> 💡 **왜 테스트를 작성하는가?**
>
> 이후 기능을 추가하면서 기존 기능이 망가지는
> Regression(회귀 버그)을 방지하기 위해서이다.

---

# 검증 결과

...

> 📌 **각 명령의 의미**
>
> `cargo fmt`
>
> → 코드 스타일 검사
>
> `cargo test`
>
> → 테스트 실행
>
> `cargo clippy`
>
> → 코드 품질 및 잠재적인 문제 검사

---

# 다음 Phase

...

> 💡 **Phase 2에서는 무엇이 달라지는가?**
>
> 지금까지는 "데이터 구조"를 만들었다.
>
> 다음 단계에서는
>
> "실제 기하학 계산"을 구현하게 된다.
>
> 예를 들어
>
> - 선의 길이 계산
> - 원의 반지름 검사
> - Arc 계산
> - Rectangle 생성
>
> 등이 추가될 예정이다.

# Rust로 CAD 엔진 만들기 — Phase 6: `cad_io` 저장/불러오기와 SVG 출력

## 들어가며

Phase 1~5에서 만든 것은 전부 메모리 안에서만 존재했다. `Project`를 아무리 정교하게 검증하고 렌더링해도, 프로그램을 껐다 켜면 사라진다. `01_제품_기획_정책.md`가 1순위 가치로 꼽는 "도면 데이터 손실 방지"는 이 문제를 정면으로 다루지 않으면 지켜질 수 없다.

Phase 6은 지금까지 유일하게 **시작 전에 사용자 승인이 필요했던** Phase다. `05_사용자_승인_정책.md`가 "프로젝트 저장 형식 또는 DB schema 변경"을 승인 대상으로 못박고 있기 때문이다. 실제 코드를 쓰기 전에 저장 형식부터 확정하고 승인을 받았다.

---

## 1. Phase 6의 목적

`02_아키텍처_정책.md`가 정의한 `cad_io`의 책임은 "저장, 불러오기, 백업, 마이그레이션, 출력"이다. `03_백엔드_구조_정책.md`는 저장 절차를 구체적으로 못박는다.

```text
메모리 검증 -> 임시 파일 작성 -> flush -> 재검증 -> 원자적 교체 -> 백업 상태 갱신
```

Phase 6 이전까지 `cad_io`는 `NotImplemented`만 반환하는 빈 껍데기였다. 이번 목표는 이 절차를 실제로 구현하고, `01_제품_기획_정책.md`의 MVP 항목인 자동저장/백업/복구/SVG 출력을 채우는 것이다.

---

## 2. 설계 결정 사항

작업을 시작하기 전, 저장 형식에 대해 두 차례 확인을 거쳤다.

### 2.1 기존에 이미 준비되어 있던 것

코드를 새로 짜기 전에 프로젝트에 이미 남아 있는 단서부터 확인했다.

```text
.gitignore
    *.cadproj              ← 프로젝트 파일 확장자가 이미 정해져 있었다
    *.autosave.cadproj      ← 자동저장 파일명 규칙도
    /backups/                ← 백업 폴더 위치도

Cargo.toml (workspace)
    serde = { version = "1", features = ["derive"] }
    serde_json = "1"        ← 직렬화 라이브러리도 이미 선택되어 있었다
    egui / eframe            ← (참고: GUI framework도 이미 정해져 있음, Phase 8에서 사용)

crates/cad_io/Cargo.toml
    cad_core, cad_render, serde, serde_json ← cad_io가 SVG 출력을 위해
                                                cad_render에 의존하는 것까지 이미 준비돼 있었다
```

> 📌 **왜 이걸 먼저 확인했는가?**
>
> 저장 형식은 승인 대상이지만, 이미 결정된 것을 다시 묻는 것은 비효율적이다.
> `.gitignore`와 `Cargo.toml`에 남은 흔적은 "이미 내려진 결정"이므로,
> 이를 무시하고 새로 제안하면 오히려 기존 의도와 어긋날 위험이 있었다.

### 2.2 `file_save_docs.md` 참조

사용자가 별도로 준비해 둔 `file_save_docs.md`(AutoCAD 파일 체계와 자체 CAD 저장 포맷 설계 참고 문서)를 확인한 뒤, 이 문서의 권장 사항에 맞춰 제안을 다듬었다.

| 항목 | 최종 결정 | 근거 |
|---|---|---|
| 포맷 지원 순서 | JSON 우선, SVG 함께 | 문서 17절 "1단계: JSON, 2단계: SVG" |
| 버전 필드 이름 | `format_version`이 아니라 **`schema_version`** | 문서 16.2절 용어 |
| 단위 명시 | 최상위에 `units: "millimeter"` | 문서 19.4절 "단위를 명시해야 한다" |
| Domain Model / DTO 분리 | **이번 Phase에서는 하지 않음** | 문서 16.4절은 권장하지만, JSON 예시(16.1절) 자체가 도메인 필드를 거의 1:1로 반영. DTO 분리가 실제로 필요한 시점은 DXF/DWG처럼 포맷 고유 개념(Handle 등)이 생기는 3~5단계 |
| 알 수 없는 객체 보존 | 이번 범위에서 제외 | 문서 19.3절은 권장하지만, `schema_version 1`은 최초 버전이라 "이전 버전이 모르는 데이터"가 아직 존재할 수 없음 |

> ⚠️ **DTO 분리를 미룬 것은 원칙을 어긴 것이 아닌가?**
>
> `file_save_docs.md`는 "내부 모델과 저장 모델도 분리하는 것이 좋다"고 권장하지만,
> 동시에 15.4절에서 "이 구조를 사용하면 내부 모델은 그대로 유지하면서 입출력 포맷만 확장할 수 있다"고
> 그 이유를 설명한다. 즉 DTO 분리의 목적은 *포맷이 여러 개일 때 서로 간섭하지 않게 하는 것*이다.
> 지금은 JSON 하나뿐이라 간섭할 대상이 없다 — 원칙을 어겼다기보다, 그 원칙이 아직 필요해지지 않은 것이다.
> DXF나 DWG를 지원하게 되는 시점에는 이 결정을 다시 검토해야 한다.

---

## 3. `cad_core`에 직렬화 능력 추가하기

### 3.1 Newtype은 "투명하게" 직렬화된다

`EntityId` 같은 stable ID는 내부적으로 `u64` 하나만 감싼 튜플 구조체(newtype)다.

```rust
pub struct EntityId(u64);
```

여기에 `Serialize`/`Deserialize`를 derive하면, serde는 이 구조체를 감싼 형태(`{"0": 5}`)가 아니라 내부 값 그대로(`5`)로 직렬화한다 — 필드가 하나뿐인 튜플 구조체를 "newtype"으로 인식하고 감싸는 껍데기를 생략하기 때문이다.

```json
{ "id": 5, "layer_id": 0 }
```

`LengthMm(f64)`, `AngleRad(f64)`도 같은 이유로 그냥 숫자로 직렬화된다. `Point2 { x: LengthMm, y: LengthMm }`는 결과적으로 `{"x": 0.0, "y": 10.0}`이 된다 — 내부에 newtype이 몇 겹 있든 JSON에는 드러나지 않는다.

### 3.2 `EntityGeometry`를 태그 있는 JSON으로

`EntityGeometry`는 여섯 가지 도형 중 하나를 담는 enum이다. serde의 기본 표현 방식(외부 태그, external tagging)을 그대로 쓰면 다음과 같은 모양이 된다.

```json
{ "Line": { "start": {...}, "end": {...} } }
```

`file_save_docs.md`의 JSON 예시는 `"type": "line"`처럼 필드 하나로 종류를 나타내는 내부 태그(internal tagging) 방식을 쓰고 있었다. 이 스타일을 그대로 따르기로 했다.

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntityGeometry {
    Line(Line),
    Polyline(Polyline),
    Rectangle(Rectangle),
    Circle(Circle),
    Arc(Arc),
    Text(Text),
}
```

결과는 다음과 같다.

```json
{ "type": "line", "start": { "x": 0.0, "y": 0.0 }, "end": { "x": 10.0, "y": 0.0 } }
```

`rename_all = "snake_case"`는 Rust의 `PascalCase` variant 이름(`Line`, `Rectangle`)을 JSON에서는 소문자(`line`, `rectangle`)로 바꿔준다. `DimensionKind`(`Linear`/`Aligned`)에도 같은 속성을 붙여 `"linear"`/`"aligned"`로 저장되게 했다.

이 태그 방식이 통하려면 각 variant가 감싸는 타입(`Line`, `Circle` 등)이 전부 필드를 가진 구조체여야 한다 — JSON 객체로 직렬화되는 타입이어야 `"type"` 태그를 그 객체 안에 끼워 넣을 수 있기 때문이다. 여섯 도형 모두 구조체이므로 문제없이 적용됐다.

---

## 4. 저장 파이프라인

### 4.1 참조용 구조체와 소유용 구조체를 분리하기

파일에 쓸 때와 읽을 때 서로 다른 보조 구조체를 만들었다.

```rust
#[derive(Serialize)]
struct ProjectFileRef<'a> {
    schema_version: u32,
    units: &'a str,
    project: &'a Project,
}

#[derive(Deserialize)]
struct ProjectFileOwned {
    schema_version: u32,
    units: String,
    project: Project,
}
```

저장할 때는 이미 메모리에 있는 `&Project`를 빌려서 감싸기만 하면 되므로 `ProjectFileRef`는 참조만 들고 있다 — `Project` 전체를 복제할 필요가 없다. 반대로 파일을 읽을 때는 JSON에서 새로운 값을 만들어내야 하므로 `ProjectFileOwned`는 소유한 값을 담는다. `Serialize`는 `ProjectFileRef`에만, `Deserialize`는 `ProjectFileOwned`에만 필요하므로 derive도 각각 하나씩만 붙였다.

### 4.2 `units` 필드를 실제로 검사하기

`units: String`을 그냥 저장만 하고 읽지 않으면 "죽은 필드"가 된다. 대신 `load_project`에서 실제로 검사한다.

```rust
if file.units != SUPPORTED_UNITS {
    return Err(IoError::UnsupportedUnits(file.units));
}
```

지금은 `"millimeter"` 외의 값이 나올 수 없지만(`03_백엔드_구조_정책.md`: "내부 길이 단위는 밀리미터다"), 이 필드가 존재하고 검사된다는 것 자체가 향후 다른 단위를 지원하게 될 때 마이그레이션의 기준점이 된다.

### 4.3 파이프라인 구현

```rust
fn write_project_atomically(project: &Project, path: &Path) -> Result<(), IoError> {
    validate_for_save(project)?;                          // 메모리 검증

    let json = serde_json::to_string_pretty(&ProjectFileRef { .. })?;

    let tmp_path = temp_path_for(path);
    {
        let mut handle = fs::File::create(&tmp_path)?;
        handle.write_all(json.as_bytes())?;                // 임시 파일 작성
        handle.sync_all()?;                                 // flush
    }

    let round_trip: ProjectFileOwned =
        serde_json::from_str(&fs::read_to_string(&tmp_path)?)?;
    if round_trip.project != *project {                     // 재검증
        let _ = fs::remove_file(&tmp_path);
        return Err(IoError::RoundTripMismatch);
    }

    fs::rename(&tmp_path, path)?;                            // 원자적 교체
    Ok(())
}
```

`03_백엔드_구조_정책.md`가 요구하는 5단계가 함수 하나에 그대로 순서대로 나타난다.

> 💡 **"재검증"을 `PartialEq` 비교로 구현한 이유**
>
> "재검증"이 정확히 무엇을 검사해야 하는지는 문서에 구체적으로 적혀 있지 않았다.
> 두 가지 해석이 가능했다.
>
> 1. 방금 쓴 파일이 다시 파싱되는가 (문법 검사)
> 2. 방금 쓴 파일이 저장하려던 내용과 **완전히 같은가** (내용 검사)
>
> `Project`, `Drawing`, `Entity` 등 관련 타입이 이미 Phase 1~4에서 전부 `PartialEq`를 derive하고 있었기 때문에,
> 2번(내용 검사)을 선택하는 데 추가 비용이 거의 없었다. 파싱만 확인하는 것보다
> "디스크에 있는 내용이 메모리의 내용과 정확히 일치한다"를 보장하는 편이
> `01_제품_기획_정책.md`의 "도면 데이터 손실 방지"에 더 부합한다고 판단했다.

`fs::rename`은 Rust 표준 라이브러리 문서에 따르면 Windows에서도 `MOVEFILE_REPLACE_EXISTING` 플래그를 사용하는 `MoveFileExW`로 구현되어 있어, 대상 파일이 이미 존재해도 원자적으로 교체된다 — Unix의 `rename()`과 동일한 보장을 크로스 플랫폼에서 얻을 수 있었다.

### 4.4 저장 실패 시 아무것도 남지 않는다

검증에 실패하면 `write_project_atomically`는 애초에 임시 파일조차 만들지 않고, 재검증에서 불일치가 발견되면 임시 파일을 지우고 실패를 반환한다. 두 경우 모두 원래 파일(`path`)은 손대지 않는다 — 테스트(`save_rejects_invalid_project_and_writes_nothing`)로 확인했다.

---

## 5. 자동저장과 백업

### 5.1 자동저장은 별도 파일, 백업은 안 만든다

```rust
pub fn autosave_project(project: &Project, original_path: &Path) -> Result<(), IoError> {
    write_project_atomically(project, &autosave_path(original_path))
}
```

`autosave_path`는 `house.cadproj`를 `house.autosave.cadproj`로 바꾼다. 자동저장은 `save_project`와 달리 백업을 만들지 않는다 — 자동저장은 짧은 간격으로 반복되므로, 매번 백업까지 만들면 `backups/` 폴더가 의미 없이 빠르게 차버린다. 사용자가 명시적으로 저장하는 순간만 백업 대상으로 삼았다.

### 5.2 백업 파일명이 겹치지 않도록 하기

처음에는 밀리초 단위 타임스탬프만으로 파일명을 만들려고 했다. 그런데 테스트에서 `save_project`를 연달아 여러 번(예: 백업 회전을 확인하려고 13번) 호출하면, 같은 밀리초 안에 두 번 이상 저장이 일어날 수 있다는 문제가 있었다. 그러면 이전 백업이 같은 이름으로 덮어써져서 실제로는 백업 개수가 줄어든다.

```rust
static BACKUP_SEQUENCE: AtomicU32 = AtomicU32::new(0);

fn backup_file_name(stem: &str) -> String {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis()).unwrap_or(0);
    let seq = BACKUP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{stem}_{millis:016}_{seq:06}.cadproj")
}
```

타임스탬프에 프로세스 전역 원자적 카운터(`AtomicU32`)를 이어 붙여, 같은 밀리초 안에 저장이 몇 번 일어나도 파일명이 겹치지 않게 했다. `{millis:016}`, `{seq:06}`처럼 자릿수를 고정해 0으로 채운 것은, 파일명을 문자열로 정렬했을 때 그 순서가 실제 시간 순서와 일치하게 만들기 위해서다 — 자릿수가 다르면 `"9".."1"`처럼 사전식 정렬이 실제 크기와 어긋날 수 있다.

> 📌 **왜 별도 타임스탬프 포맷팅 라이브러리를 쓰지 않았는가?**
>
> 사람이 읽기 좋은 날짜(`2026-07-18_14-30-00`)를 만들려면 `chrono`나 `time` 같은 외부 crate가 필요하다.
> `05_사용자_승인_정책.md`상 새 외부 crate 추가는 승인 대상이라, 이미 승인된 것(`serde`, `serde_json`) 안에서
> 해결할 수 있는 방법을 우선했다. `SystemTime`만으로는 사람이 읽기 편한 날짜 문자열을 만들 수 없어
> 밀리초 숫자를 그대로 사용했다 — 정렬과 유일성은 보장되지만 파일명만 보고 날짜를 알아보기는 어렵다는
> 트레이드오프가 있다.

### 5.3 오래된 백업 정리

```rust
fn prune_backups(backups_dir: &Path, stem: &str) -> Result<(), IoError> {
    let mut names: Vec<String> = /* backups_dir 안에서 이 stem의 백업 파일명만 모음 */;
    names.sort();
    if names.len() > MAX_BACKUPS {
        for name in &names[..names.len() - MAX_BACKUPS] {
            fs::remove_file(backups_dir.join(name))?;
        }
    }
    Ok(())
}
```

앞서 파일명을 고정 자릿수로 만들어 뒀기 때문에 `names.sort()`(문자열 정렬)만으로 시간 순 정렬이 보장된다. 별도의 날짜 파싱 없이 오래된 것부터 `MAX_BACKUPS`(10개)를 넘는 만큼만 지운다.

---

## 6. SVG 출력

### 6.1 좌표계 뒤집기

CAD 좌표계는 y축이 위로 증가하지만(수학 표준), SVG는 y축이 아래로 증가한다. 아무 처리 없이 그대로 그리면 도면이 상하로 뒤집혀 보인다. 모든 좌표에 `-y`를 적용해서 해결했다.

```rust
format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" .. />",
    start.0, -start.1, end.0, -end.1)
```

`viewBox`의 y 시작점도 같은 원칙으로 뒤집었다(`-max_y`부터 시작).

### 6.2 호(Arc)를 그릴 때 방향이 꼬였던 문제

SVG의 `<path>` 요소로 호를 그리려면 `sweep-flag`(방향)와 `large-arc-flag`(180도를 넘는지)를 지정해야 한다. 처음에는 "CAD의 sweep_angle이 양수면 SVG sweep-flag도 그냥 같은 방향이겠지"라고 단순하게 가정했는데, 실제로 좌표를 뒤집고 있다는 것을 감안하면 이 가정이 맞는지 다시 계산해봐야 했다.

CAD에서 각도 θ만큼 회전한 점은 `(cos θ, sin θ)`이고, 이걸 y축을 뒤집어 SVG에 넘기면 `(cos θ, -sin θ)`가 된다. 이 값은 `(cos(-θ), sin(-θ))`와 같다 — 즉 뒤집은 좌표는 "SVG 좌표계 안에서 각도 `-θ`만큼 회전한 점"과 같은 값이 된다.

```text
CAD 각도 θ 증가 (반시계, sweep_angle > 0)
    ↓ y 반전
SVG 좌표계 안에서의 각도는 -θ, 즉 θ가 커질수록 감소
```

SVG의 `sweep-flag = 1`은 "각도가 증가하는 방향"을 의미하므로, 우리가 넘기는 좌표 기준으로 각도가 오히려 *감소*하는 이 상황은 `sweep-flag = 0`에 해당한다.

```rust
fn svg_sweep_flag(sweep_angle: f64) -> u8 {
    u8::from(sweep_angle <= 0.0)
}
```

`sweep_angle > 0.0`(CAD 반시계)일 때 `false` → `0`, `sweep_angle <= 0.0`일 때 `true` → `1`이 되어 위 유도와 일치한다.

> ⚠️ **이 부분은 자동화된 테스트로 "보이는 결과"까지 확인하지 못했다**
>
> `svg_sweep_flag_matches_cad_ccw_convention` 테스트는 함수가 유도한 공식대로 값을 반환하는지만 확인한다.
> 실제로 브라우저에서 열어서 호가 시각적으로 올바른 방향으로 그려지는지는 확인하지 못했다.
> Phase 8에서 실제 도면을 그려보며 재검증이 필요한 부분으로 남겨둔다.

### 6.3 나머지 도형과 치수

Line, Polyline/Polygon, Rectangle, Circle, Text는 SVG의 대응 요소(`<line>`, `<polyline>`/`<polygon>`, `<rect>`, `<circle>`, `<text>`)에 직접 매핑된다. `Text`의 내용은 `&`, `<`, `>`를 이스케이프해서 깨진 XML이 나오지 않게 했다.

`Dimension`은 Phase 5에서 내린 결정(치수선/인출선 배치는 미확정)을 그대로 따라, 시작점과 끝점을 잇는 점선 하나로만 표시한다 — 실제 치수선다운 표현은 아니지만, 데이터가 어디 있는지는 확인할 수 있다.

---

## 7. 테스트

`cad_core`에 2개, `cad_io`에 10개를 추가했다.

### `cad_core`

| 테스트 | 검증 내용 |
|---|---|
| `entity_geometry_serializes_with_a_type_tag` | `EntityGeometry`가 `"type"` 태그를 가진 JSON으로 직렬화되는지 |
| `project_round_trips_through_json` | `Project`를 JSON으로 직렬화했다가 역직렬화하면 원본과 같은지 |

### `cad_io`

| 테스트 | 검증 내용 |
|---|---|
| `save_then_load_round_trips` | 저장 후 불러오면 원본과 동일한지 |
| `save_rejects_invalid_project_and_writes_nothing` | 검증 실패 시 아무 파일도 생기지 않는지 |
| `load_rejects_unsupported_schema_version` | 알 수 없는 `schema_version`을 거부하는지 |
| `autosave_path_uses_autosave_suffix` | 파일명 규칙이 올바른지 |
| `save_creates_a_backup_file` | 저장 시 백업이 생기는지 |
| `backups_are_pruned_to_max_count` | 13번 저장해도 백업이 10개로 유지되는지 |
| `export_svg_contains_expected_elements` | SVG 문서 구조가 올바른지 |
| `export_svg_skips_entities_on_hidden_layers` | 숨긴 레이어가 SVG에서도 제외되는지 |
| `svg_sweep_flag_matches_cad_ccw_convention` | 호 방향 플래그 공식 검증 |
| `svg_large_arc_flag_reflects_sweep_magnitude` | 180도 초과 여부 플래그 검증 |

---

## 8. 검증 결과

```text
cargo fmt --all -- --check
통과

cargo test --workspace
전체 통과
- cad_core: 14/14
- cad_io: 10/10
- workspace total: 91

cargo clippy --workspace --all-targets -- -D warnings
경고 없음 (pedantic 포함, 첫 시도에 통과)
```

작업 도중 `.gitignore`의 허점도 하나 발견했다 — `*.cadproj` 패턴은 임시 파일 `test.cadproj.tmp`와 매치되지 않아, 저장 중간에 생기는 임시 파일이 실수로 커밋될 수 있는 상태였다. `*.cadproj.tmp`를 추가해 막았다.

---

## 9. Phase 6에서 사용한 Rust 핵심 개념

### Newtype의 투명한 직렬화

필드가 하나뿐인 튜플 구조체는 감싸는 껍데기 없이 내부 값 그대로 직렬화된다.

```rust
pub struct EntityId(u64);  // JSON에서는 그냥 5
```

### `#[serde(tag = "type")]`를 이용한 내부 태그 enum

여러 variant를 갖는 enum을 "구분자 필드 하나 + 나머지 필드"로 평평하게 직렬화했다.

```rust
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntityGeometry { Line(Line), /* ... */ }
```

### 참조용/소유용 구조체 분리

같은 데이터를 감싸는 두 구조체를 목적에 따라 나눠, 저장 시 불필요한 복제를 피했다.

```rust
struct ProjectFileRef<'a> { project: &'a Project, .. }   // Serialize
struct ProjectFileOwned { project: Project, .. }          // Deserialize
```

### `AtomicU32`를 이용한 프로세스 전역 카운터

락 없이도 여러 호출 사이에서 겹치지 않는 값을 얻었다.

```rust
static BACKUP_SEQUENCE: AtomicU32 = AtomicU32::new(0);
BACKUP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
```

### `From<E>` 구현과 `?` 연산자

`std::io::Error`, `serde_json::Error`를 각각 `IoError`로 변환하는 `From`을 구현해, 여러 실패 지점을 `?` 하나로 전파했다.

```rust
impl From<io::Error> for IoError { .. }
fs::File::create(&tmp_path)?;   // 자동으로 IoError::Io로 변환
```

### 값 비교를 이용한 무결성 검사

새 개념은 아니지만, 이미 존재하던 `PartialEq` derive를 "재검증" 단계에 그대로 활용했다.

```rust
if round_trip.project != *project { /* 저장 실패 처리 */ }
```

---

## 10. Phase 6 완료 결과

`cad_io`는 빈 `NotImplemented` 스텁에서, `01_제품_기획_정책.md`의 MVP 저장 관련 항목을 모두 채우는 계층으로 바뀌었다.

```text
Project (메모리)
    ↓ save_project
.cadproj (JSON, 원자적 저장)
    ↓ (자동, 백업 없이)
.autosave.cadproj

.cadproj
    ↓ 저장 시마다
backups/*.cadproj (최근 10개)

Project
    ↓ export_svg / save_svg
SVG 문서
```

`02_아키텍처_정책.md`가 그린 전체 흐름(`사용자 입력 -> ... -> 검증 -> 렌더 모델 -> 화면/출력/저장`)의 마지막 구간이 이번에 채워졌다.

---

## 11. 남은 과제

### 호 렌더링의 시각적 검증 없음

sweep-flag 공식은 수학적으로 유도했지만, 실제 브라우저에서 열어 눈으로 확인하지 않았다. Phase 8에서 실물 도면을 그려볼 때 재확인이 필요하다.

### Domain Model / DTO 분리 없음

지금은 `cad_core` 타입이 직접 JSON으로 직렬화된다. DXF/DWG 지원이 실제로 필요해지는 시점에 `file_save_docs.md`가 권장하는 DTO 분리를 다시 검토해야 한다.

### 마이그레이션 함수 없음

`schema_version`이 지금은 `1`뿐이라 마이그레이션 로직(`v1 -> v2`)이 아직 없다. 스키마가 바뀌는 시점에 `file_save_docs.md` 16.3절의 패턴(버전별 `match` 후 변환)을 적용한다.

### `cad_command`의 `replay`와 아직 연결되지 않음

Phase 4에서 만든 `replay(commands)`는 명령 로그로 프로젝트를 재구성하지만, 그 로그 자체를 저장/불러오는 기능은 없다. 지금은 `Project` 스냅샷 자체를 저장하는 방식만 구현했다 — 명령 저널 방식은 필요성이 확인되면 별도로 추가한다.

---

## 12. 다음 Phase

다음 단계는 `cad_cli`다.

### Phase 7 예정 범위

- GUI 없이 프로젝트를 검증하는 명령 (`01_제품_기획_정책.md`: "CLI 검증 명령")
- 저장된 파일을 불러와 복구를 시도하는 명령
- `cad_io`/`cad_command`/`cad_core`를 그대로 재사용 — 새로운 도메인 로직은 만들지 않는다

---

## 마무리

Phase 6은 이번 프로젝트에서 처음으로 "사용자 승인"이 실제 게이트로 작동한 Phase였다. 두 번의 확인(기존 흔적 기반 제안 → `file_save_docs.md` 반영 후 재확인) 덕분에, 막상 구현할 때는 형식을 둘러싼 재작업이 전혀 없었다 — 코드를 쓰기 전에 합의된 스키마가 그대로 구현 전체를 관통했다.

동시에 이번 Phase는 "문서에 적힌 대로"만으로는 부족한 지점(재검증이 정확히 무엇을 의미하는지, 호의 회전 방향을 어떻게 계산할지)에서 직접 판단하고 그 근거를 남겨야 했던 Phase이기도 하다. 특히 SVG 호 방향 문제는 겉보기엔 사소해 보이지만 부호 하나만 틀려도 모든 호가 반대로 그려지는, 눈으로 확인하기 전까지는 틀렸는지조차 알기 어려운 종류의 버그였다.

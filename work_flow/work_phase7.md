# Rust로 CAD 엔진 만들기 — Phase 7: `cad_cli`로 headless 기능 완성하기

## 들어가며

Phase 6까지 만든 `cad_io`는 강력하지만 아직 아무도 쓸 수 없는 상태였다. `save_project`, `load_project`, `export_svg` 같은 함수는 전부 라이브러리 코드일 뿐, 사용자가 실제로 실행할 수 있는 프로그램이 아니었다. GUI(`cad_app`)는 아직 Phase 8에서나 만들어질 예정이고, 그 전까지는 지금까지 쌓아온 모든 계층(core → geometry → tolerance → command → render → io)을 실제로 손에 쥐고 써볼 방법이 없었다.

`02_아키텍처_정책.md`가 `cad_cli`의 책임으로 정의한 것이 정확히 이 공백을 메운다 — "GUI 없이 검증, 출력, 복구".

---

## 1. Phase 7의 목적

`01_제품_기획_정책.md`의 MVP 항목에는 "CLI 검증 명령"이 명시되어 있다. Phase 7의 목표는 이 요구를 채우되, 범위를 넓히지 않는 것이다.

> Phase 6 문서(`work_phase6.md`)의 "다음 Phase" 절에 이미 이렇게 적어뒀다.
>
> - GUI 없이 프로젝트를 검증하는 명령
> - 저장된 파일을 불러와 복구를 시도하는 명령
> - `cad_io`/`cad_command`/`cad_core`를 그대로 재사용 — 새로운 도메인 로직은 만들지 않는다

이 약속을 그대로 지켰다. Phase 7에서 추가한 도메인 로직은 사실상 없다 — 전부 기존 함수를 조합하는 얇은 배선(wiring)이다.

---

## 2. 기존 스텁이 이미 알려준 것

`cad_cli/src/main.rs`는 Phase 0 이전부터 다음과 같은 스텁으로 존재했다.

```rust
"--help" | "help" => println!("cad validate|export|batch-export|inspect|recover"),
other => eprintln!("command '{other}' is not implemented in the initial scaffold"),
```

이 한 줄이 사실상 CLI의 명세였다. Phase 1의 `.gitignore`/`Cargo.toml`에서 저장 형식의 힌트를 얻었던 것처럼, 이번에도 이미 남아 있는 설계 의도를 그대로 따랐다 — 새로 명령 목록을 고민하지 않고 `validate`, `export`, `batch-export`, `inspect`, `recover` 다섯 개를 그대로 구현 대상으로 삼았다.

`cad_cli/Cargo.toml`도 `cad_core`, `cad_io`, `cad_batch`에 대한 의존성이 이미 선언되어 있었다 — `cad_command`는 의도적으로 빠져 있었다. CLI가 하는 일(검증, 출력, 복구)은 전부 "읽기 + 저장"이지 "사용자 조작을 undo/redo 가능한 명령으로 실행"하는 것이 아니므로, `cad_command`가 필요 없다는 것도 이미 정해져 있던 셈이다.

---

## 3. `cad_io`에 한 조각 추가하기 — `latest_backup`

`recover` 명령은 가장 최근 백업을 찾아야 한다. 그런데 "백업이 어느 폴더에, 어떤 이름 규칙으로 있는지"는 이미 `cad_io`의 `backups_dir_for`/`backup_file_name`이 알고 있는 정보였다 — 이 규칙을 `cad_cli`에 다시 구현하면 두 crate가 같은 규칙을 따로 유지해야 하고, 나중에 파일명 규칙이 바뀌면 한쪽만 고치는 실수가 생길 수 있다.

```rust
#[must_use]
pub fn latest_backup(path: &Path) -> Option<PathBuf> {
    let backups_dir = backups_dir_for(path);
    let stem = path.file_stem()?.to_str()?;
    let prefix = format!("{stem}_");

    let mut names: Vec<String> = fs::read_dir(&backups_dir).ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(&prefix) && name.ends_with(".cadproj"))
        .collect();
    names.sort();
    names.pop().map(|name| backups_dir.join(name))
}
```

Phase 6에서 백업 파일명을 `{stem}_{millis:016}_{seq:06}.cadproj`처럼 고정 자릿수로 만들어 뒀던 덕분에, 여기서도 `names.sort()` 한 줄로 최신 백업을 안다 — 최댓값이 정렬된 목록의 마지막(`pop()`)에 온다.

> 💡 **"백업 조회"를 어느 crate에 둘 것인가**
>
> 이 함수는 `cad_cli`에서만 쓰인다. 그런데도 `cad_io`에 넣은 이유는,
> "누가 이 데이터를 소유하는가"와 "누가 이 데이터를 쓰는가"가 다르기 때문이다.
> 백업 폴더 구조는 `cad_io`가 결정한 규칙이므로, 그 규칙을 아는 코드도 `cad_io`에 있어야
> 규칙이 바뀔 때 한 곳만 고치면 된다.

---

## 4. `cad_batch` 채우기 — 있던 구조체를 그대로 쓰기

`cad_batch`에는 이미 이런 코드가 있었다.

```rust
#[derive(Debug, PartialEq)]
pub struct BatchReport {
    pub processed: usize,
    pub failed: usize,
}

pub fn empty_report() -> BatchReport {
    BatchReport { processed: 0, failed: 0 }
}
```

`processed`/`failed`라는 필드 이름 자체가 이미 "여러 개를 처리하고 성공/실패를 센다"는 용도를 말해주고 있었다. 새 구조체를 만드는 대신 이 구조체에 `failed_paths: Vec<PathBuf>` 하나만 보태고, 그 구조를 채우는 함수를 만들었다.

```rust
pub fn export_svg_dir(dir: &Path) -> BatchReport {
    let mut report = empty_report();
    let Ok(entries) = fs::read_dir(dir) else { return report; };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        // ".cadproj"로 끝나지만 ".autosave.cadproj"는 아닌 파일만
        if !is_project_file(&name) { continue; }

        let exported = cad_io::load_project(&path).ok()
            .map(|project| cad_io::save_svg(&project, &path.with_extension("svg")))
            .is_some_and(|result| result.is_ok());

        if exported { report.processed += 1; }
        else { report.failed += 1; report.failed_paths.push(path); }
    }
    report
}
```

`is_project_file`이 autosave 파일을 걸러내는 이유는, `"house.autosave.cadproj"`도 `.extension()`만 보면 `"cadproj"`로 끝나 `"house.cadproj"`와 구분되지 않기 때문이다. 파일명 전체가 `.autosave.cadproj`로 끝나는지 따로 검사해야 두 번 내보내는 실수를 막을 수 있다.

읽기 실패와 SVG 쓰기 실패를 굳이 구분하지 않고 하나의 `failed`로 합친 것은, 사용자 입장에서 "이 파일은 실패했다"는 사실이 중요하지 실패 지점의 세부 단계는 `batch-export`의 결과 요약에서는 크게 의미가 없다고 판단했기 때문이다.

---

## 5. `cad_cli` 설계 — 테스트 가능한 명령 함수

### 5.1 `ExitCode`를 직접 비교할 수 없는 문제

처음에는 각 명령 함수가 `std::process::ExitCode`를 직접 반환하도록 만들려고 했다. 그런데 `ExitCode`는 내부 값을 비교할 수 있는 API를 제공하지 않는다 — 테스트에서 `assert_eq!(result, ExitCode::SUCCESS)` 같은 코드를 쓸 수 없다.

대신 명령 함수들은 전부 `Result<(), String>`을 반환하게 했다.

```rust
fn run_validate(mut args: impl Iterator<Item = String>) -> Result<(), String> { .. }
fn run_export(mut args: impl Iterator<Item = String>) -> Result<(), String> { .. }
```

`main`만 이 결과를 `ExitCode`로 변환한다.

```rust
fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "--help".to_owned());
    match run(&command, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => { eprintln!("{message}"); ExitCode::FAILURE }
    }
}
```

이렇게 나누자 테스트에서는 `result.is_ok()` / `result.is_err()`로 명령의 성공·실패를 직접 검증할 수 있게 됐다. "출력 형식(종료 코드)"과 "명령의 실제 동작"을 분리한 셈이다.

### 5.2 바이너리 crate도 단위 테스트를 가질 수 있다

`cad_cli`는 라이브러리(`lib.rs`)가 아니라 실행 파일(`main.rs`)이다. 그래도 같은 파일 안에 `#[cfg(test)] mod tests`를 두면 `cargo test -p cad_cli`가 그대로 이 테스트들을 컴파일하고 실행한다 — 바이너리 crate라고 해서 테스트를 포기할 이유가 없었다.

```rust
fn args(values: &[&str]) -> impl Iterator<Item = String> {
    values.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>().into_iter()
}

#[test]
fn validate_succeeds_for_a_valid_project() {
    // ...
    let result = run_validate(args(&[path.to_str().unwrap()]));
    assert!(result.is_ok());
}
```

`std::env::args()`를 직접 읽는 대신 모든 명령 함수가 `impl Iterator<Item = String>`을 인자로 받도록 설계했기 때문에, 테스트에서는 진짜 명령줄 인자 대신 임의의 문자열 목록을 넣어줄 수 있었다.

---

## 6. `recover` 명령 — 원본을 절대 덮어쓰지 않기

### 6.1 복구 순서

```text
1. 원본 경로 그대로 불러와보기       → 성공하면 "복구 불필요"
2. 실패하면 autosave 경로 시도       → cad_io::autosave_path
3. 실패하면 최신 백업 시도            → cad_io::latest_backup
4. 셋 다 실패하면 오류 반환
```

### 6.2 왜 원본에 덮어쓰지 않는가

`file_save_docs.md`의 7.3절(복구 시 주의사항)은 이렇게 못박고 있다.

> - 원본 파일을 직접 덮어쓰지 않는다.
> - 복구 파일은 별도 이름으로 복사한다.

이 원칙을 그대로 코드에 반영했다. 복구에 성공하면 항상 `<이름>.recovered.cadproj`라는 새 파일을 만들고, 원본 경로는 절대 건드리지 않는다.

```rust
fn finish_recovery(project: &Project, original: &Path, source: &Path) -> Result<(), String> {
    let recovered_path = original.with_extension("recovered.cadproj");
    cad_io::save_project(project, &recovered_path)?;
    println!("recovered from {} -> wrote {} (original left untouched)", ..);
    Ok(())
}
```

`05_사용자_승인_정책.md`가 "사용자가 승인하지 않은 파일 삭제 또는 덮어쓰기"를 반드시 승인받을 작업으로 분류하고 있는 것과도 같은 방향이다 — `recover` 명령이 사용자의 명시적 확인 없이 원본을 대체해버리면, 잘못된 백업을 복구했을 때 되돌릴 방법이 없어진다. 새 파일로 남겨두면 사용자가 내용을 비교하고 직접 교체 여부를 정할 수 있다.

---

## 7. 테스트

### `cad_io` (+2)

| 테스트 | 검증 내용 |
|---|---|
| `latest_backup_finds_the_most_recent_one` | 여러 백업 중 최신 것을 찾는지 |
| `latest_backup_is_none_when_no_backups_exist` | 백업이 없으면 `None`인지 |

### `cad_batch` (신규 4개)

| 테스트 | 검증 내용 |
|---|---|
| `empty_report_starts_at_zero` | 기본값이 0인지 |
| `export_svg_dir_processes_valid_projects_and_skips_autosave` | autosave 파일을 중복 처리하지 않는지 |
| `export_svg_dir_counts_unreadable_files_as_failures` | 손상된 파일을 실패로 집계하는지 |
| `export_svg_dir_on_missing_directory_is_an_empty_report` | 없는 디렉터리는 오류가 아니라 빈 리포트인지 |

### `cad_cli` (신규 10개)

| 테스트 | 검증 내용 |
|---|---|
| `validate_succeeds_for_a_valid_project` / `validate_fails_for_a_missing_file` | 검증 성공/실패 |
| `export_writes_an_svg_file` | SVG 파일이 실제로 생성되는지 |
| `inspect_succeeds_for_a_valid_project` | 요약 정보 조회 |
| `recover_reports_no_action_when_original_is_valid` | 이미 정상이면 아무것도 만들지 않는지 |
| `recover_falls_back_to_autosave_without_touching_the_original` | autosave 복구, 원본 미생성 확인 |
| `recover_falls_back_to_latest_backup_when_autosave_is_missing` | 백업 복구 |
| `recover_fails_when_nothing_can_be_found` | 아무 소스도 없을 때 실패 |
| `batch_export_reports_success_for_a_directory_of_valid_projects` | 일괄 출력 |
| `unknown_command_is_an_error` | 잘못된 명령어 처리 |

---

## 8. 실제 바이너리로 직접 확인하기

Phase 1~6은 전부 `cargo test`로만 검증했다. Phase 7은 처음으로 **실제 실행 파일을 직접 실행**해서 확인한 Phase다. 손으로 만든 `.cadproj` 샘플 파일 하나로 다섯 명령을 전부 실행해봤다.

```text
$ cad_cli validate demo.cadproj
OK: demo.cadproj is valid (1 entities, 1 layers, 0 dimensions)

$ cad_cli inspect demo.cadproj
project id: ProjectId(0)
drawing id: DrawingId(0)
layers: 1
  - 0 (visible=true, locked=false)
entities: 1
dimensions: 0

$ cad_cli export demo.cadproj
exported demo.cadproj -> demo.svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -0 10 1">
  <line x1="0" y1="-0" x2="10" y2="-0" stroke="black" />
</svg>

$ cad_cli batch-export .
batch-export: 1 exported, 0 failed

$ rm demo.cadproj && cp demo.autosave.cadproj demo.autosave.cadproj  # (autosave만 남긴 상태)
$ cad_cli recover demo.cadproj
recovered from demo.autosave.cadproj -> wrote demo.recovered.cadproj (original left untouched)
```

`recover`의 마지막 줄 확인이 특히 중요했다 — `demo.cadproj`(원본)는 삭제된 채로 남아 있고, `demo.recovered.cadproj`만 새로 생겼는지 실제 파일 목록으로 확인했다. 단위 테스트가 같은 것을 이미 검증하고 있었지만, 실제 파일 시스템에서 눈으로 한 번 더 확인한 것은 "임시 디렉터리 안에서의 테스트"와 "실제 사용자 경로에서의 동작"이 다를 가능성을 배제하기 위해서였다.

---

## 9. 검증 결과

```text
cargo fmt --all -- --check
통과

cargo test --workspace
전체 통과
- cad_io: 12/12 (+2)
- cad_batch: 4/4 (+4)
- cad_cli: 10/10 (+10)
- workspace total: 117

cargo clippy --workspace --all-targets -- -D warnings
경고 없음 (map_unwrap_or 1건 수정 후 통과)

실제 바이너리 실행: validate / inspect / export / batch-export / recover 전부 확인
```

clippy가 지적한 것은 `Option::map().unwrap_or_else()` 패턴이었다.

```rust
// 수정 전
args.next().map(PathBuf::from).unwrap_or_else(|| Path::new(&path).with_extension("svg"))

// 수정 후
args.next().map_or_else(|| Path::new(&path).with_extension("svg"), PathBuf::from)
```
두 메서드 호출을 하나(`map_or_else`)로 합쳐 중간에 `Option`을 한 번 덜 감쌌다 뺐다 하는 것 — 의미는 같지만 더 직접적이다.

---

## 10. Phase 7에서 사용한 Rust 핵심 개념

### 제네릭 이터레이터 인자로 테스트 가능성 확보

`std::env::args()`에 직접 의존하지 않고 `impl Iterator<Item = String>`을 받아, 실제 명령줄과 테스트용 입력을 같은 함수로 처리했다.

```rust
fn run_validate(mut args: impl Iterator<Item = String>) -> Result<(), String>
```

### `?`로 여러 실패 지점을 하나의 흐름으로

`cad_io`의 각기 다른 실패(파일 없음, 파싱 실패, 검증 실패)를 `map_err`로 문자열 메시지로 통일한 뒤 `?`로 전파했다.

```rust
let project = cad_io::load_project(Path::new(&path))
    .map_err(|e| format!("failed to load {path}: {e}"))?;
```

### `let else`로 조기 반환

디렉터리를 읽을 수 없는 경우를 별도 분기 없이 짧게 처리했다.

```rust
let Ok(entries) = fs::read_dir(dir) else { return report; };
```

### `Option::is_some_and`

"값이 있고, 그 값이 조건을 만족하는가"를 한 번에 표현했다.

```rust
.map(|project| cad_io::save_svg(&project, &out))
.is_some_and(|result| result.is_ok())
```

### 바이너리 crate 내 단위 테스트

`main.rs` 안에서도 `lib.rs`와 동일하게 `#[cfg(test)] mod tests`가 동작한다는 것을 실제로 활용했다.

---

## 11. Phase 7 완료 결과

`cad_cli`는 빈 스텁에서 다음 다섯 가지를 실제로 수행하는 프로그램이 됐다.

```text
cad_cli validate <path>        cad_io::load_project 재사용
cad_cli export <path> [out]    cad_io::save_svg 재사용
cad_cli batch-export <dir>     cad_batch::export_svg_dir (신규, 기존 BatchReport 확장)
cad_cli inspect <path>         cad_io::load_project 재사용
cad_cli recover <path>         cad_io::{load_project, autosave_path, latest_backup, save_project} 조합
```

이번 Phase를 마치면서, `01_제품_기획_정책.md`가 요구한 "GUI와 CLI에서 동일한 엔진 재사용"이 처음으로 실제 두 번째 진입점(entry point)을 통해 증명됐다 — `cad_cli`는 `cad_core`/`cad_io`/`cad_batch`가 제공하는 함수를 그대로 호출할 뿐, 자신만의 도메인 로직을 갖지 않는다.

---

## 12. 남은 과제

### 프로젝트를 새로 만드는 명령이 없다

`validate`/`inspect`/`export`/`recover`는 모두 기존 파일을 대상으로 한다. 빈 프로젝트를 새로 만들어 저장하는 `new` 같은 명령은 없다 — 지금까지 스모크 테스트에서도 `.cadproj`를 손으로 작성해서 사용했다. GUI가 이 역할을 맡을 예정이라 이번 범위에서는 제외했지만, GUI 없이 새 프로젝트를 시작하고 싶은 경우를 위해 나중에 필요해질 수 있다.

### `batch-export`가 하위 디렉터리를 훑지 않는다

`fs::read_dir`는 지정한 디렉터리 바로 아래만 본다. 프로젝트를 폴더 구조로 정리해서 관리하는 사용자에게는 재귀 탐색이 필요할 수 있다.

### 명령줄 인자 파싱이 최소 수준이다

`--out <path>` 같은 이름 있는 옵션 없이 위치 인자만 지원한다. 옵션이 늘어나면 `clap` 같은 라이브러리 도입을 검토해야 하는데, 이는 새 외부 의존성 추가라 승인이 필요하다.

---

## 13. 다음 Phase

다음 단계는 `cad_app`(GUI)이다. `01_제품_기획_정책.md`가 요구한 "GUI와 CLI에서 동일한 엔진 재사용"이 CLI 쪽에서는 이번에 증명됐으니, 이제 같은 재사용을 GUI에서도 보여줄 차례다.

GUI framework는 이미 워크스페이스 `Cargo.toml`에 `egui`/`eframe`로 정해져 있어, Phase 8은 다른 Phase와 달리 framework 선택을 다시 승인받을 필요는 없다 — 다만 `04_UI_와이어프레임_정책.md`가 정의한 화면 구조와 상호작용 흐름을 실제 위젯으로 옮기는 작업 자체가 이번 프로젝트에서 가장 큰 단일 Phase가 될 가능성이 높다.

### Phase 8 예정 범위

- 기본 레이아웃: 메뉴 / 도구 모음 / 왼쪽 도구·레이어 패널 / 중앙 캔버스 / 오른쪽 속성·검증 패널 / 하단 명령 입력
- UI 상태(선택, 도구, 확대율)와 도메인 상태(`Project`) 분리
- 도구 선택 → 입력 수집 → 미리보기 → 확정 command → 검증 → 렌더 갱신 → 자동 저장 예약 흐름 구현
- Phase 6에서 미뤄둔 치수선 배치 규칙을 이번에 확정

---

## 마무리

Phase 7은 새로운 계산이나 데이터 구조를 거의 만들지 않은, 지금까지 중 가장 "배선(wiring)" 성격이 강한 Phase였다. 그런데도 의미가 작지 않았던 이유는, 이 Phase를 통해 처음으로 지금까지 쌓아온 6개 계층이 사람이 직접 실행할 수 있는 하나의 프로그램으로 이어졌기 때문이다. `cargo test`가 초록불이어도 실제 바이너리를 실행했을 때 다른 결과가 나올 가능성은 항상 있다 — 이번에는 그 간극이 없다는 것을 직접 확인하고 넘어갔다.

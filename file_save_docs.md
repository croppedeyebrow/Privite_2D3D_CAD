# CAD 파일 포맷·확장자·버전 관리 방식

## 1. 개요

AutoCAD의 파일 체계는 단순히 `.dwg` 파일 하나로 끝나지 않는다.

실제로는 다음 요소들이 함께 사용된다.

- 기본 도면 저장 포맷
- CAD 간 데이터 교환 포맷
- 템플릿
- 외부 참조
- 자동 저장 및 백업
- 출력 설정
- 폰트와 해치 패턴
- 자동화 스크립트와 플러그인
- DWG 버전 호환성

AutoCAD와 호환되는 CAD 프로그램이나 자체 CAD 엔진을 설계하려면, 각 파일의 역할과 버전 관리 방식을 분리해서 이해해야 한다.

---

# 2. 주요 파일 확장자

## 2.1 도면 및 데이터 파일

| 확장자 | 명칭                    | 설명                               | 주요 용도                            |
| ------ | ----------------------- | ---------------------------------- | ------------------------------------ |
| `.dwg` | Drawing                 | AutoCAD의 기본 바이너리 도면 파일  | 일반적인 도면 작성 및 저장           |
| `.dxf` | Drawing Exchange Format | CAD 프로그램 간 데이터 교환용 포맷 | CAD 호환, 데이터 변환, 외부 연동     |
| `.dwt` | Drawing Template        | 도면 템플릿 파일                   | 레이어, 문자, 치수, 출력 기준 재사용 |
| `.dws` | Drawing Standards       | 도면 표준 검사 파일                | 회사 또는 프로젝트 표준 검증         |

## 2.2 백업 및 임시 파일

| 확장자  | 설명             | 용도                           |
| ------- | ---------------- | ------------------------------ |
| `.bak`  | 이전 저장본 백업 | 도면 복구                      |
| `.sv$`  | 자동 저장 파일   | 비정상 종료 후 복구            |
| `.dwl`  | 도면 잠금 파일   | 편집 중인 사용자 정보 관리     |
| `.dwl2` | 확장 잠금 정보   | 사용자, 컴퓨터, 세션 정보 저장 |
| `.ac$`  | 임시 파일        | 작업 중 임시 데이터 저장       |

## 2.3 출력 및 스타일 파일

| 확장자 | 설명                       | 용도                             |
| ------ | -------------------------- | -------------------------------- |
| `.pc3` | Plotter Configuration      | 프린터 및 플로터 설정            |
| `.ctb` | Color-dependent Plot Style | 색상 기준 선 굵기 및 출력 스타일 |
| `.stb` | Named Plot Style           | 이름 기준 출력 스타일            |
| `.pmp` | Plotter Model Parameter    | 출력 장치별 세부 설정            |
| `.psv` | Plot Stamp Settings        | 출력 도장 정보                   |

## 2.4 폰트 및 도면 표현 파일

| 확장자                  | 설명                | 용도                     |
| ----------------------- | ------------------- | ------------------------ |
| `.shx`                  | Shape 또는 CAD Font | CAD 전용 문자와 형상     |
| `.lin`                  | Linetype Definition | 중심선, 점선 등의 선종류 |
| `.pat`                  | Hatch Pattern       | 해치 패턴 정의           |
| `.mln`                  | Multiline Style     | 다중선 스타일            |
| `.mnu`, `.cui`, `.cuix` | UI 설정             | 메뉴, 리본, 단축키 설정  |

## 2.5 자동화 및 확장 파일

| 확장자 | 설명                      | 용도                            |
| ------ | ------------------------- | ------------------------------- |
| `.scr` | Script                    | AutoCAD 명령 자동 실행          |
| `.lsp` | AutoLISP                  | CAD 자동화                      |
| `.fas` | Compiled AutoLISP         | 컴파일된 AutoLISP               |
| `.vlx` | Visual LISP Executable    | Visual LISP 애플리케이션 배포   |
| `.arx` | AutoCAD Runtime Extension | C++ 기반 AutoCAD 플러그인       |
| `.dbx` | ObjectDBX Extension       | 사용자 정의 객체 및 데이터 처리 |
| `.dll` | .NET Assembly             | C# 기반 AutoCAD 플러그인        |

---

# 3. DWG 포맷

## 3.1 DWG란

DWG는 AutoCAD의 기본 저장 포맷이다.

주요 특징은 다음과 같다.

- 바이너리 포맷
- 비교적 작은 파일 크기
- 빠른 읽기와 쓰기
- 객체 지향 데이터 구조
- 2D 및 3D 도면 지원
- 레이어, 블록, 치수, 문자, 스타일 저장
- 레이아웃과 뷰포트 저장
- 사용자 정의 객체 및 확장 데이터 지원

DWG에는 단순한 선과 원뿐 아니라 도면의 전체 작업 환경이 함께 저장된다.

## 3.2 DWG에 포함될 수 있는 정보

```text
DWG
├── 파일 헤더
├── 버전 정보
├── 클래스 테이블
├── 객체 테이블
├── 핸들 테이블
├── 레이어 테이블
├── 블록 테이블
├── 엔티티 데이터
├── 문자 및 치수 스타일
├── 레이아웃
├── 뷰포트
├── 사전 및 메타데이터
├── 확장 데이터
└── 미리보기 이미지
```

## 3.3 엔티티와 객체

AutoCAD에서는 일반적으로 화면에 표시되는 도형을 엔티티라고 부른다.

대표적인 엔티티는 다음과 같다.

- Line
- Polyline
- Circle
- Arc
- Ellipse
- Spline
- Text
- MText
- Dimension
- Hatch
- Block Reference
- Image Reference

반면 화면에 직접 표시되지 않더라도 도면을 구성하는 데이터는 객체로 관리된다.

예시는 다음과 같다.

- Layer
- Linetype
- Text Style
- Dimension Style
- Layout
- Dictionary
- Group
- Material

---

# 4. DXF 포맷

## 4.1 DXF란

DXF는 `Drawing Exchange Format`의 약자로, 서로 다른 CAD 프로그램 간 도면 데이터를 교환하기 위해 만들어진 포맷이다.

DXF는 ASCII 텍스트 또는 바이너리 형식으로 저장할 수 있지만, 일반적으로 텍스트 기반 ASCII DXF가 많이 사용된다.

## 4.2 기본 구조

DXF는 그룹 코드와 값의 조합으로 구성된다.

```text
0
SECTION
2
HEADER
9
$ACADVER
1
AC1032
0
ENDSEC
```

위 예시는 파일 헤더에서 DWG/DXF 버전 코드가 `AC1032`임을 나타낸다.

## 4.3 LINE 엔티티 예시

```text
0
LINE
8
Layer1
10
0.0
20
0.0
30
0.0
11
100.0
21
50.0
31
0.0
```

각 그룹 코드는 다음 의미를 가진다.

| 그룹 코드        | 의미             |
| ---------------- | ---------------- |
| `0`              | 엔티티 종류      |
| `8`              | 레이어 이름      |
| `10`, `20`, `30` | 시작점의 X, Y, Z |
| `11`, `21`, `31` | 끝점의 X, Y, Z   |

즉, 위 데이터는 다음 선을 의미한다.

```text
시작점: (0, 0, 0)
끝점:   (100, 50, 0)
레이어: Layer1
```

## 4.4 DXF의 장단점

### 장점

- 사람이 직접 읽을 수 있다.
- 파싱 및 디버깅이 비교적 쉽다.
- CAD 프로그램 간 호환성이 높다.
- Git에서 텍스트 차이를 확인할 수 있다.
- 자체 CAD 엔진의 입출력 포맷으로 구현하기 쉽다.

### 단점

- 파일 크기가 크다.
- DWG보다 읽기와 쓰기가 느릴 수 있다.
- 모든 사용자 정의 객체가 완벽하게 교환되지는 않는다.
- 프로그램마다 DXF 지원 수준이 다를 수 있다.

---

# 5. DWG와 DXF 비교

| 항목             | DWG            | DXF                  |
| ---------------- | -------------- | -------------------- |
| 기본 형식        | 바이너리       | 텍스트 또는 바이너리 |
| 파일 크기        | 비교적 작음    | 비교적 큼            |
| 처리 속도        | 빠름           | 상대적으로 느림      |
| 사람이 직접 읽기 | 어려움         | 가능                 |
| CAD 간 호환성    | AutoCAD 중심   | 비교적 높음          |
| Git Diff         | 사실상 어려움  | 텍스트 DXF는 가능    |
| 내부 정보 보존   | 높음           | 일부 손실 가능       |
| 직접 구현 난이도 | 높음           | 상대적으로 낮음      |
| 주요 용도        | 실제 작업 도면 | 데이터 교환 및 연동  |

---

# 6. DWT 템플릿

DWT는 새 도면을 생성할 때 사용하는 템플릿 파일이다.

회사나 프로젝트에서 반복적으로 사용하는 설정을 미리 정의할 수 있다.

## 6.1 템플릿에 포함할 수 있는 요소

- 단위
- 도면 한계
- 레이어
- 선종류
- 문자 스타일
- 치수 스타일
- 다중 지시선 스타일
- 표 스타일
- 블록
- 레이아웃
- 용지 크기
- 뷰포트
- 출력 스타일
- 회사 도곽
- 표준 제목란

## 6.2 사용 흐름

```text
회사표준.dwt
    ↓
새 도면 생성
    ↓
프로젝트도면.dwg
```

DWT를 사용하면 프로젝트 참여자들이 동일한 도면 규칙을 적용할 수 있다.

---

# 7. 백업 및 자동 저장

## 7.1 BAK 파일

AutoCAD에서 기존 DWG 파일을 다시 저장하면, 이전 저장 상태가 `.bak` 파일로 남을 수 있다.

```text
project.dwg
project.bak
```

복구할 때는 파일 확장자를 변경한다.

```text
project.bak
    ↓
project_recovery.dwg
```

단, 운영체제에서 파일 확장자 표시가 활성화되어 있어야 한다.

## 7.2 SV$ 파일

`.sv$` 파일은 AutoCAD의 자동 저장 파일이다.

비정상 종료나 프로그램 충돌이 발생했을 때 복구에 사용할 수 있다.

```text
project_1_1_1234.sv$
    ↓
project_recovery.dwg
```

자동 저장 파일의 실제 위치는 AutoCAD 설정에 따라 달라진다.

## 7.3 복구 시 주의사항

- 원본 파일을 직접 덮어쓰지 않는다.
- 복구 파일은 별도 이름으로 복사한다.
- 확장자 변경 후 `RECOVER` 명령으로 검사를 수행한다.
- 필요하면 `AUDIT` 명령으로 도면 오류를 검사한다.
- 참조 파일이 누락되었는지 확인한다.

---

# 8. 도면 잠금 파일

AutoCAD에서 DWG 파일을 열면 같은 디렉터리에 `.dwl`, `.dwl2` 파일이 생성될 수 있다.

```text
project.dwg
project.dwl
project.dwl2
```

잠금 파일은 다음 목적으로 사용된다.

- 현재 도면이 열려 있는지 확인
- 편집 중인 사용자 식별
- 컴퓨터 이름 및 세션 정보 관리
- 중복 편집 경고

AutoCAD가 정상적으로 종료되면 잠금 파일은 일반적으로 자동 삭제된다.

비정상 종료로 잠금 파일이 남은 경우, 실제로 아무도 도면을 열고 있지 않은지 확인한 뒤 제거해야 한다.

---

# 9. 외부 참조 Xref

## 9.1 Xref란

Xref는 `External Reference`의 약자로, 다른 DWG 파일을 현재 도면에서 외부 참조로 연결하는 기능이다.

예를 들어 건축 프로젝트에서는 다음과 같이 도면을 분리할 수 있다.

```text
Master.dwg
├── Architecture.dwg
├── Structure.dwg
├── Electrical.dwg
├── Mechanical.dwg
└── Plumbing.dwg
```

`Master.dwg`는 각 도면의 실제 객체를 복사하지 않고 파일 참조를 저장한다.

## 9.2 Xref의 장점

- 여러 작업자가 도면을 분리해서 작업할 수 있다.
- 원본 도면 변경이 상위 도면에 반영된다.
- 마스터 도면의 파일 크기를 줄일 수 있다.
- 건축, 구조, 전기, 설비 도면을 독립적으로 관리할 수 있다.
- 협업 구조가 명확해진다.

## 9.3 Xref 경로 방식

| 경로 방식 | 설명                          | 특징                          |
| --------- | ----------------------------- | ----------------------------- |
| 절대 경로 | 드라이브부터 전체 경로 저장   | 다른 PC로 이동 시 깨지기 쉬움 |
| 상대 경로 | 현재 도면 기준 상대 위치 저장 | 프로젝트 단위 이동에 유리     |
| 경로 없음 | 파일명만 저장                 | 동일 폴더 또는 검색 경로 의존 |

프로젝트 폴더를 함께 이동하거나 배포해야 한다면 상대 경로가 일반적으로 유리하다.

## 9.4 Attach와 Overlay

| 방식    | 설명                                                              |
| ------- | ----------------------------------------------------------------- |
| Attach  | 현재 도면에 참조하고, 상위 도면에서도 중첩 참조가 전달됨          |
| Overlay | 현재 도면에서만 참조하며, 상위 도면에는 중첩 참조가 전달되지 않음 |

복잡한 중첩 참조로 인한 순환 참조를 방지하려면 Overlay를 사용하는 경우가 많다.

---

# 10. Block과 Xref의 차이

| 구분             | Block                      | Xref                             |
| ---------------- | -------------------------- | -------------------------------- |
| 데이터 저장 위치 | 현재 DWG 내부              | 외부 DWG 파일                    |
| 원본 변경 반영   | 자동 반영되지 않음         | 다시 로드하면 반영               |
| 파일 독립성      | 현재 파일만으로 유지       | 참조 파일 필요                   |
| 주요 용도        | 반복 부품, 심볼, 상세 객체 | 도면 분리, 협업, 대규모 프로젝트 |
| 배포 편의성      | 높음                       | 참조 경로 관리 필요              |

Block은 현재 도면 내부에 데이터를 포함하고, Xref는 외부 파일을 링크한다.

---

# 11. DWG 버전 체계

## 11.1 내부 버전 코드

DWG와 DXF 파일은 AutoCAD 제품 연도와 별개로 내부 버전 코드를 사용한다.

| AutoCAD 계열      | DWG 내부 코드 |
| ----------------- | ------------- |
| Release 12        | `AC1009`      |
| Release 13        | `AC1012`      |
| Release 14        | `AC1014`      |
| AutoCAD 2000      | `AC1015`      |
| AutoCAD 2004      | `AC1018`      |
| AutoCAD 2007      | `AC1021`      |
| AutoCAD 2010      | `AC1024`      |
| AutoCAD 2013      | `AC1027`      |
| AutoCAD 2018 계열 | `AC1032`      |

제품 버전이 매년 출시되더라도 DWG 저장 포맷은 매년 변경되지 않는다.

여러 AutoCAD 제품 버전이 동일한 DWG 포맷을 공유할 수 있다.

## 11.2 버전 코드 확인

ASCII DXF에서는 `$ACADVER` 값을 통해 확인할 수 있다.

```text
9
$ACADVER
1
AC1032
```

DWG는 바이너리 파일이지만 파일 시작 영역에 버전 식별 정보가 포함된다.

---

# 12. 상위 및 하위 버전 호환성

## 12.1 상위 버전에서 하위 버전 열기

일반적으로 최신 AutoCAD는 이전 버전의 DWG 파일을 열 수 있다.

```text
AutoCAD 최신 버전
    ↓
이전 DWG 파일 열기
    ↓
최신 형식으로 저장 가능
```

## 12.2 하위 버전에서 상위 버전 열기

이전 AutoCAD는 자신보다 새로운 DWG 포맷을 직접 열지 못할 수 있다.

```text
새로운 DWG 포맷
    ↓
이전 AutoCAD
    ↓
직접 열기 실패 가능
```

이 경우 다음 방법을 사용한다.

- AutoCAD에서 하위 버전으로 `Save As`
- Autodesk DWG TrueView로 변환
- DXF로 내보내기
- 신뢰할 수 있는 CAD 변환 도구 사용

## 12.3 하위 버전 저장 시 주의사항

최신 도면을 오래된 DWG 버전으로 저장하면 다음 문제가 발생할 수 있다.

- 최신 객체가 프록시 객체로 변환
- 일부 객체가 기본 엔티티로 분해
- 최신 속성 손실
- 해치 또는 치수 표현 변경
- 사용자 정의 객체 정보 손실
- 재료 및 3D 데이터 단순화
- 최신 주석 기능 호환 문제

따라서 하위 버전 저장은 단순한 파일 확장자 변경이 아니라 데이터 변환 과정이다.

---

# 13. AutoCAD의 실무 버전 관리

AutoCAD 자체의 DWG 버전과 프로젝트의 변경 이력 관리는 서로 다른 문제다.

## 13.1 파일 포맷 버전 관리

파일 포맷 버전 관리는 다음 내용을 의미한다.

- `AC1027`
- `AC1032`
- AutoCAD 2013 형식
- AutoCAD 2018 형식

즉, 어느 AutoCAD 제품군에서 열 수 있는지를 관리한다.

## 13.2 도면 변경 이력 관리

도면 변경 이력 관리는 실제 업무에서 다음 방식으로 수행되는 경우가 많다.

```text
Project_A
├── 01_Working
├── 02_Review
├── 03_Approved
├── 04_Issued
└── 99_Archive
```

파일 이름 예시는 다음과 같다.

```text
A-101_FloorPlan_Rev00.dwg
A-101_FloorPlan_Rev01.dwg
A-101_FloorPlan_Rev02.dwg
```

또는 날짜를 포함할 수 있다.

```text
A-101_FloorPlan_2026-07-18.dwg
```

## 13.3 Revision 관리

도면 Revision은 일반적으로 다음 정보와 함께 관리한다.

| 항목      | 설명                      |
| --------- | ------------------------- |
| 도면 번호 | 도면의 고유 식별자        |
| Revision  | 수정 차수                 |
| 변경 날짜 | 변경이 승인된 날짜        |
| 작성자    | 도면 수정 담당자          |
| 검토자    | 검토 담당자               |
| 승인자    | 승인 담당자               |
| 변경 내용 | 수정된 내용 요약          |
| 배포 상태 | 검토용, 승인용, 시공용 등 |

도면 안의 제목란에도 Revision 이력을 기록하는 것이 일반적이다.

---

# 14. Git으로 DWG를 관리할 때의 한계

DWG는 바이너리 파일이므로 일반 소스 코드처럼 Git Diff를 확인하기 어렵다.

## 14.1 주요 문제

- 어떤 객체가 변경되었는지 텍스트 Diff로 확인하기 어렵다.
- 작은 수정에도 파일 전체가 변경된 것으로 보일 수 있다.
- 파일 크기가 크면 저장소 용량이 빠르게 증가한다.
- 여러 사람이 동시에 편집한 내용을 병합하기 어렵다.
- 충돌 발생 시 수동 병합이 사실상 어렵다.

## 14.2 Git 관리 전략

```text
repository
├── src/
├── docs/
├── samples/
├── drawings/
│   ├── source/
│   └── exports/
└── metadata/
```

권장 방식은 다음과 같다.

- DWG는 Git LFS로 관리
- DXF 또는 JSON 스냅샷을 함께 저장
- 도면 메타데이터를 별도 텍스트 파일로 관리
- 도면 번호와 Revision 규칙 통일
- 동일 DWG의 동시 편집 방지
- 자동 내보내기로 SVG, PDF, PNG 생성
- PR에서는 렌더링 결과를 비교

---

# 15. 자체 CAD 엔진 설계 관점

자체 CAD 프로그램을 개발할 때는 DWG 구조를 내부 도메인 모델과 직접 결합하지 않는 것이 중요하다.

권장 구조는 다음과 같다.

```text
CAD Application
├── cad_core
├── cad_geometry
├── cad_document
├── cad_command
├── cad_render
├── cad_io
│   ├── dxf
│   ├── dwg
│   ├── json
│   ├── svg
│   └── pdf
└── cad_app
```

## 15.1 cad_core

CAD의 핵심 도메인 타입을 정의한다.

```text
Point
Line
Polyline
Rectangle
Circle
Arc
Text
Layer
Block
Document
```

핵심 모델은 특정 저장 포맷에 의존하지 않아야 한다.

잘못된 예시는 다음과 같다.

```rust
struct Line {
    dwg_handle: u64,
    dxf_group_code_10: f64,
    dxf_group_code_20: f64,
}
```

권장 예시는 다음과 같다.

```rust
struct Line {
    start: Point,
    end: Point,
}
```

DWG의 Handle이나 DXF의 Group Code는 `cad_io`에서 처리한다.

## 15.2 cad_geometry

다음과 같은 순수 계산을 담당한다.

- 거리
- 교차
- 투영
- 변환
- 회전
- 스케일
- 스냅
- 경계 상자
- 포함 관계
- 충돌 판정

## 15.3 cad_document

도면 단위의 상태를 관리한다.

```text
Document
├── Entities
├── Layers
├── Blocks
├── Styles
├── Units
├── Metadata
└── Version
```

## 15.4 cad_io

외부 포맷을 내부 모델로 변환한다.

```text
DXF Reader
    ↓
cad_core 모델
    ↓
DXF Writer
```

```text
DWG Reader
    ↓
cad_core 모델
    ↓
DWG Writer
```

이 구조를 사용하면 내부 모델은 그대로 유지하면서 입출력 포맷만 확장할 수 있다.

---

# 16. 자체 Native 포맷 설계

DWG를 직접 구현하기 어렵다면, 초기에는 자체 JSON 또는 바이너리 포맷을 사용하는 것이 현실적이다.

## 16.1 JSON 예시

```json
{
  "schema_version": 1,
  "document": {
    "units": "millimeter",
    "layers": [
      {
        "id": "layer-001",
        "name": "0",
        "visible": true,
        "locked": false
      }
    ],
    "entities": [
      {
        "id": "entity-001",
        "type": "line",
        "layer_id": "layer-001",
        "start": {
          "x": 0.0,
          "y": 0.0
        },
        "end": {
          "x": 100.0,
          "y": 50.0
        }
      }
    ]
  }
}
```

## 16.2 스키마 버전

자체 파일 포맷에는 반드시 버전 필드를 넣는 것이 좋다.

```json
{
  "schema_version": 3
}
```

파일 로딩 과정은 다음과 같이 구성할 수 있다.

```text
Version 1 File
    ↓
V1 → V2 Migration
    ↓
V2 → V3 Migration
    ↓
Current Document Model
```

## 16.3 버전 변환 함수

```rust
fn load_document(raw: RawDocument) -> Result<Document, LoadError> {
    match raw.schema_version {
        1 => migrate_v1_to_current(raw),
        2 => migrate_v2_to_current(raw),
        3 => parse_current(raw),
        version => Err(LoadError::UnsupportedVersion(version)),
    }
}
```

## 16.4 저장 정책

내부 모델과 저장 모델도 분리하는 것이 좋다.

```text
Domain Model
    ↓
Serialization DTO
    ↓
JSON / Binary / DXF / DWG
```

예를 들어 화면 캐시, 선택 상태, 임시 계산값은 파일에 저장하지 않는다.

---

# 17. 권장 포맷 지원 순서

자체 CAD 프로젝트를 개발한다면 다음 순서가 현실적이다.

## 1단계: JSON

목적:

- 내부 모델 검증
- 저장 및 불러오기 구현
- 버전 마이그레이션 구조 구현
- 테스트 자동화

## 2단계: SVG

목적:

- 2D 렌더링 결과 내보내기
- 브라우저 기반 결과 확인
- 테스트 스냅샷 생성

## 3단계: DXF

목적:

- AutoCAD 및 다른 CAD와 데이터 교환
- Line, Circle, Arc, Polyline, Text 지원
- Layer 및 Block 매핑

## 4단계: PDF

목적:

- 배포 및 출력
- 읽기 전용 결과 제공
- 도면 검토

## 5단계: DWG

목적:

- 실제 산업용 CAD 호환
- ODA 또는 상용 SDK 연동
- 고급 객체와 버전 호환

---

# 18. 포맷별 책임 분리 예시

```text
cad_core
└── 도형과 도면의 의미

cad_geometry
└── 좌표와 기하 계산

cad_document
└── 도면 상태와 객체 관계

cad_io_json
└── 내부 저장 및 테스트

cad_io_dxf
└── CAD 교환 포맷

cad_io_dwg
└── AutoCAD 호환 포맷

cad_export_svg
└── 화면 및 웹 출력

cad_export_pdf
└── 문서 및 인쇄 출력
```

포맷 처리 코드가 `cad_core`나 `cad_geometry`에 들어가면 안 된다.

---

# 19. 설계 시 핵심 원칙

## 19.1 내부 ID와 외부 Handle 분리

```text
Internal Entity ID
≠
DWG Handle
≠
DXF Handle
```

내부 도메인 ID는 프로그램이 직접 관리하고, DWG/DXF Handle은 입출력 어댑터에서 변환한다.

## 19.2 파일 버전과 애플리케이션 버전 분리

```text
Application Version: 0.4.0
File Schema Version: 3
DXF Version: AC1032
```

세 버전은 서로 다른 의미를 가진다.

## 19.3 알 수 없는 객체 보존

지원하지 않는 객체를 읽었을 때 무조건 삭제하면 데이터가 손실된다.

가능하면 다음과 같은 형태로 보존한다.

```rust
enum Entity {
    Line(Line),
    Circle(Circle),
    Arc(Arc),
    Unknown(UnknownEntity),
}
```

## 19.4 단위 명시

파일에는 단위를 명시해야 한다.

```text
Millimeter
Centimeter
Meter
Inch
Foot
```

단위가 없으면 같은 좌표라도 실제 크기를 판단할 수 없다.

## 19.5 부동소수점 정책

CAD 좌표는 부동소수점 기반이므로 다음 규칙이 필요하다.

- 직접 동등 비교 금지
- epsilon 명시
- 좌표 정규화
- 저장 정밀도 정책
- 변환 누적 오차 관리
- 극단적으로 큰 좌표 처리

---

# 20. 정리

AutoCAD의 파일 체계는 다음과 같이 구분할 수 있다.

```text
도면 저장
├── DWG
├── DXF
└── DWT

복구 및 잠금
├── BAK
├── SV$
├── DWL
└── DWL2

표현 및 출력
├── SHX
├── LIN
├── PAT
├── PC3
├── CTB
└── STB

자동화 및 확장
├── SCR
├── LSP
├── VLX
├── ARX
├── DBX
└── DLL
```

DWG는 AutoCAD의 핵심 바이너리 포맷이고, DXF는 CAD 간 데이터 교환에 적합한 포맷이다.

AutoCAD의 버전 관리는 다음 두 가지를 구분해야 한다.

1. DWG 저장 형식의 버전
2. 실제 도면의 Revision과 변경 이력

자체 CAD 프로그램에서는 다음 원칙이 중요하다.

- 내부 도메인 모델과 외부 파일 포맷을 분리한다.
- `cad_core`는 DWG/DXF 구조를 알지 않도록 한다.
- 입출력은 `cad_io` 계층에서 처리한다.
- 초기에는 JSON, SVG, DXF 순서로 구현한다.
- 파일 스키마 버전과 마이그레이션 구조를 처음부터 설계한다.
- DWG 호환은 전용 SDK 또는 ODA 계열 기술을 검토한다.

이 구조를 따르면 도형 모델, 렌더링, 저장 포맷, 버전 변환을 독립적으로 확장할 수 있다.

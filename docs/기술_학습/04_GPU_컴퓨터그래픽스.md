# wgpu 실시간 렌더링

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 개념별 한/영 검색어와 숙달 기준

---

## 학습 세부 목차

| 번호 | 한국어 검색어/주제 | 영어 검색어 | 어디까지 이해·구현할 것인가 |
|---|---|---|---|
| 01 | 그래픽스 파이프라인 | GPU graphics pipeline vertex fragment depth buffer | GPU pipeline 구성과 resource lifetime |
| 02 | 카메라·투영 | orthographic perspective camera orbit | CAD 뷰포트 orthographic/perspective 구현 |
| 03 | Tessellation·정밀도 | BRep tessellation chordal deflection | B-Rep에서 메쉬 및 tolerance 설정 |
| 04 | Picking·ID buffer | GPU object picking face ID buffer | selection ID 보존과 hit 정확도 |
| 05 | PBR·색공간 | physically based rendering linear sRGB tone mapping | 미리보기/PNG 색상 차이 테스트 |

## 학습 검증 형식

1. 개념을 구조도/예제 데이터로 설명합니다.
2. 프로그램 최소 구현 예제를 만들고 오차·시간·자원 결과를 기록합니다.
3. 정상 결과와 실패 결과를 비교해 원인을 추적합니다.
4. 실제 `cad_*` crate 또는 설계 계약과 연결합니다.
5. 블로그/README에는 단순 코드 복사 대신 테스트 증거·트레이드오프를 포함합니다.

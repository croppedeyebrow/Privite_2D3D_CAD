# CAD 모델링 알고리즘

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 개념별 한/영 검색어와 숙달 기준

---

## 학습 세부 목차

| 번호 | 한국어 검색어/주제 | 영어 검색어 | 어디까지 이해·구현할 것인가 |
|---|---|---|---|
| 01 | Sketch Plane·Constraint | parametric sketcher degrees of freedom | 닫힌 profile·구속 상태 구현 |
| 02 | B-Rep·Topology | B-Rep face edge vertex shell solid | 원본 형상과 렌더 메쉬 역할 구분 |
| 03 | Boolean·Extrude·Revolve | solid modeling boolean extrude revolve | 오류 B-Rep 감지/rollback |
| 04 | Feature DAG·Recompute | parametric feature dependency graph recompute | dirty descendants 재계산 |
| 05 | Topological Naming | CAD topological naming problem persistent naming | 모호한 face reference 중단/복구 |

## 학습 검증 형식

1. 개념을 구조도/예제 데이터로 설명합니다.
2. 프로그램 최소 구현 예제를 만들고 오차·시간·자원 결과를 기록합니다.
3. 정상 결과와 실패 결과를 비교해 원인을 추적합니다.
4. 실제 `cad_*` crate 또는 설계 계약과 연결합니다.
5. 블로그/README에는 단순 코드 복사 대신 테스트 증거·트레이드오프를 포함합니다.

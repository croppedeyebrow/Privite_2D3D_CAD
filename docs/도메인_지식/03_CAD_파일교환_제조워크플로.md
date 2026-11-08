# 산업 CAD 파일/제조 흐름

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 해당 설계 도메인 지식과 CAD 제품 적용

---

## 제조 교환 체계

| 포맷 | 유용한 경우 | 잃기 쉬운 정보 | 검증 요점 |
|---|---|---|---|
| native `.cadstudio`(가칭) | 편집/구속/피처 이력 | 외부 시스템 호환 | reload/recompute |
| STEP | 정밀 형상 교환 | 내부 파라메트릭 의도 | 면·솔리드·단위·부피 |
| STL | 3D 프린팅/메시 | 정밀 원호·피처/단위 관례 | watertight mesh |
| DXF | 2D CAM/도면 | 일부 고급 엔터티·폰트 | layer/line/arc mapping |
| glTF | 실시간 시각화 | B-Rep/제조 치수 | PBR mesh/material mapping |
| PDF/SVG | 인쇄·공유 | 편집용 형상 의미 | 축척/글꼴/선가중치 |

## 검사 절차

외부 파일 ingest→단위 정규화→변환 로그→shape validity→비교 fixture→명시적 손실 보고서를 갖춥니다.

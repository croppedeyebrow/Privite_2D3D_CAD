# 기술 후보 PoC 점수표

> 문서상태: **템플릿 · 실행 데이터 미수집**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 구현/시험 후 값과 기록을 채우는 서식

---

## 측정 비교 서식(아직 실측값 없음)

| 항목 | OCCT+Rust FFI | Truck | 상태/측정법 |
|---|---|---|---|
| Windows clean build | 미측정 | 미측정 | commit/toolchain pin |
| Boolean/Fillet coverage | 미측정 | 미측정 | 동일 B-Rep input |
| STEP reader/writer | 미측정 | 미측정 | 독립 검증 앱 readback |
| Crash/exception semantics | 미측정 | 미측정 | fault injection |
| License & redistribution | 검토 전 | 검토 전 | 법적 요구사항 확인 |
| Feature topology naming | 미측정 | 미측정 | upstream edit |

## Solver·GPU 추가 후보 비교

- Solver: 지원 구속, DOF 상태, 수렴 한계, timeout, 재현성, 라이선스.
- GPU: 대상 장비별 adapter 인식, 3D picking, buffer 메모리, 장치소실 recover, 프레임 시간.
- 최종 ADR에 근거·최종 선택·대안·버전·변경 조건을 기록합니다.

# CLI·배치·향후 플러그인 설계

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: CLI·배치·향후 플러그인 설계
> 원문 추적: 2;8

---

## 7. 실행/검증 명령(예시 계약)

```text
cad validate model.cadstudio --strict
cad inspect model.cadstudio --entities --features
cad export model.cadstudio --format step --out part.step
cad export model.cadstudio --format stl --deflection-mm 0.1 --out part.stl
cad render model.cadstudio --preset studio --size 1920x1080 --out hero.png
cad recover model.cadstudio.autosave --out recovered.cadstudio
```

`render`·`--format step` 등의 실제 지원은 각 단계 구현 뒤에 활성화합니다. CLI는 코어의 별도 구현이 아니라 동일 application service 호출자로 유지합니다.

## CLI 안정 인터페이스

| 명령 | R1/R2 범위 | 오류 처리 |
|---|---|---|
| `cad validate` | 스키마·참조·형상 검사 | machine-readable report + exit code |
| `cad inspect` | 구조·피처·문제 조회 | 파일 손상 시 partial report |
| `cad export` | svg/step/stl(릴리즈 순서에 따라) | 중간 파일 삭제 정책 명시 |
| `cad recover` | recovery candidate→별도 출력 | 원본 덮어쓰기 금지 |
| `cad render` | R3 도입 시 PNG | headless GPU 미지원이면 명시적 실패 |

- CLI 입력은 UI와 동일한 application service로 처리합니다.
- 향후 스크립트/플러그인은 버전 고정 API, sandbox, resource budget, deterministic batch 계약이 마련된 후 검토합니다.

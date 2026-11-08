# CAD Studio — 개발 에이전트와 협업 규칙

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: AI/사람 공용 최소 개발 계약

---

## 최우선 기준

1. 사용자 설계 데이터 보존과 backward compatibility를 신규 기능보다 우선합니다.
2. Document의 영속 변경은 Typed Command 및 revision 검증을 거칩니다.
3. **CAD 설계 원본은 Document/Feature/Sketch/B-Rep**이며 GPU Mesh는 derived cache입니다.
4. UI 입력과 영속 도메인 모델을 분리합니다. GUI와 CLI가 Application API를 공유합니다.
5. `unsafe`가 필요한 C++/GPU boundary는 별도 crate로 한정하고, 해당 crate만 예외 정책을 적용합니다.
6. 라이브러리 신규 추가, 저장 스키마 변경, 데이터 이동·삭제·덮어쓰기, 커널 교체, 네트워크 사용은 승인 대상입니다.

## 변경 요청 템플릿

- 요구사항 ID / 영향 서브시스템 / 파일 형식 영향 / 하위 호환 / 반례 / 테스트 / 롤백.
- 작업 전: P00 구현 인벤토리와 테스트 기준선을 확인하고 미확인 코드는 **가정**으로 표시.
- 작업 후: `fmt`, `clippy`, `test`, fixture regression, 저장 round-trip, Undo/Redo, 스레드/렌더 leak 점검.

## 완료 증거

단순 빌드 성공은 완료가 아닙니다. 사용자 시나리오, 예외 입력, 위험 축소 수치, 실제 OS/GPU 검증 로그가 있어야 합니다.

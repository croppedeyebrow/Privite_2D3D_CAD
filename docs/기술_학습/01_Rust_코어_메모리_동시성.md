# Rust 코어와 안전한 FFI

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 개념별 한/영 검색어와 숙달 기준

---

## 학습 세부 목차

| 번호 | 한국어 검색어/주제 | 영어 검색어 | 어디까지 이해·구현할 것인가 |
|---|---|---|---|
| 01 | 소유권·대여·라이프타임 | Rust ownership borrowing lifetimes | 복잡한 Document&cache API에서 참조무효화를 방지 |
| 02 | 에러 처리·결과 타입 | Rust Result error enum thiserror | 커널 오류와 사용자 입력 오류를 타입으로 구분 |
| 03 | Trait·Generic·Cargo workspace | Rust traits generics cargo workspaces | 교체 가능한 KernelPort·SolverPort 구성 |
| 04 | 스레드·채널·Cancel | Rust worker thread channel cancellation | 긴 커널 작업의 취소/Revision 충돌 설계 |
| 05 | C++ FFI·불안전 코드 | Rust C++ cxx FFI unsafe safety invariant | 메모리 수명·예외·ABI·스레드 규칙 명시 |

## 학습 검증 형식

1. 개념을 구조도/예제 데이터로 설명합니다.
2. 프로그램 최소 구현 예제를 만들고 오차·시간·자원 결과를 기록합니다.
3. 정상 결과와 실패 결과를 비교해 원인을 추적합니다.
4. 실제 `cad_*` crate 또는 설계 계약과 연결합니다.
5. 블로그/README에는 단순 코드 복사 대신 테스트 증거·트레이드오프를 포함합니다.

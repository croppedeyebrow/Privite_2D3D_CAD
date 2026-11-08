# CI/CD·테스트·라이선스

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 실제 CAD 구현 시 알아야 할 기술 원리·역할·실패 요건

---

## 1. 테스트 피라미드

**Unit (초기부터):** geometry predicates, unit conversions, bbox, dimension tolerance, ID invariants, serial DTO, command failure rollback, solver constraint status.

**Property/fuzz:** invalid/random geometry, boolean near-degeneracy, command sequences/inverse, malformed JSON/ZIP/file sizes, finite-value rejection.

**Integration:** Rust/C++ bridge memory ownership, STEP read/write, sketch solver→B-Rep feature, tessellation face map, snapshot-to-GPU sync, cad_app/cad_cli 같은 core 결과.

**Golden:** schema fixtures v1/v2, canonical snapshots, 2D SVG/PDF, exported STEP/STL compared by shape validity & geometric measurements; binary byte-for-byte diff만으로 판단 금지.

**E2E:** Windows actual GUI mouse/keyboard, long document sessions, crash mid-save/recompute, invalid/imported model, undo+redo after feature rebuild.

**Cross CAD:** 외부 도구에서 STEP open/inspect/re-export, size/BBox/volume and shape count comparison. 하나의 동일 커널만 쓰는 내부 왕복 시험과 구분.

## 4. 보안 및 안정성 위협 모델

| 위험 | 완화책 | 테스트 |
|---|---|---|
| 악성 STEP/DXF/native 파일 | 크기·개수·깊이 상한, parsing limits, 검증, possible out-of-process importer | malformed/fuzz corpus |
| native C++ segfault/exception | C++ exception boundary/RAII, inputs preflight, kernel worker process PoC | fault injection |
| ZIP path traversal / zip bomb | canonical safe paths, uncompressed budget, no arbitrary extraction | crafted archive |
| 자동 저장 데이터 손실 | staging+backup+read-back+separate autosave | kill process at save stages |
| 종속성 공급망 | pinned lockfile/toolchain, SBOM, advisories, license check | CI report |
| 사용자 문서 외부 유출 | opt-in only telemetry/network and external API; default offline | network-disabled test |
| GPU driver fault | device reinit/clear message, no domain mutation | device-lost simulation |
| 스크립트/매크로(P2) | sandbox, file/network permissions, untrusted macro disabled by default | malicious script fixtures |

`unsafe` 선언을 `cad_kernel_occt` 전용 FFI boundary로 제한하고 code review checklist/ASan or equivalent native tests을 사용. Rust 전체가 메모리 안전이라는 마케팅 문구로 FFI 위험을 덮지 않음.

## 5. DevOps/CI/CD (Windows 우선)

- PR checks: fmt/clippy/unit, license/SBOM, schema migrations, automated headless CLI fixtures.
- Windows CI: exact Rust toolchain and MSVC/CMake C++ dependency build; cache key pinned to toolchain & OCCT version.
- Separate GPU tests: runner hardware variability 고려해 nightly/manual or dedicated GPU CI, deterministic CPU-side render scene test.
- Package: Windows `portable zip` and/or signed installer; included C++ DLL/VC++ redistributable requirements, license notices, crash log opt-in.
- Release manifest: app version, file schema, kernel version, GPU backend, supported formats, known limitations.
- Rollback: install 이전 앱이 새 형식을 읽지 못할 수 있으므로 **업그레이드 사본·원본 백업** 정책.
- Telemetry/auto-update/cloud sync 등 네트워크 기능은 별도 opt-in과 사전 승인 후만.

## 6. 릴리즈 승인 게이트

1. 기능 데모보다 **저장/복구/Undo/피처 실패 복원** 시험 통과 우선.
2. Crash and malformed external file corpus failing tests = release blocker.
3. STEP validation and units incorrect = R2 release blocker.
4. Topology reference change silently moves design intent = release blocker.
5. GPU 문제로 문서 corruption = release blocker.
6. License/SBOM 불명확 native library를 포함한 배포 금지.
7. Release notes에 지원/미지원 포맷 및 모델 복잡도 한계 명시.

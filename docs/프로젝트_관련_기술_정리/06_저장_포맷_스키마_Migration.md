# 파일/저장·스키마

> 문서상태: **기획 확정 전 · 구현 검증 필요**  
> 문서버전: v2.1-draft (2026-10-25)  
> 문서목적: 실제 CAD 구현 시 알아야 할 기술 원리·역할·실패 요건

---

## 1. 저장 정책 — 설계 의도를 보존하는 자체 포맷

**제안 확장자:** `.cadstudio` (가칭). R2에서 ZIP archive/container 형식 또는 versioned directory-package 중 하나를 PoC로 결정. 파일 extension 확정 전에 Windows atomic replacement/성능/보안을 테스트.

**구성 예시 (ZIP container 개념이며 구현 확정 아님):**

```text
part.cadstudio
├── manifest.json               # magic, schema_version, created_with, checksum/index
├── document.json               # units, ids, layers, parameters, persistent refs
├── sketches.json               # local planes, geometry, constraints
├── features.json               # DAG, history, statuses, bodies
├── drawings.json               # drafting & sheets
├── appearances.json            # materials/cameras, presentation attrs
├── native/geometry.brep        # OPTIONAL: kernel-specific cache/validated geometry
└── thumbnail.png               # OPTIONAL non-authoritative image
```

원본 진실의 우선순위는 `document/sketches/features`이며, B-Rep native blob의 완전한 재생성 가능성 및 커널간 호환성은 보장되지 않음. API나 피처 semantics가 바뀌면 자동 재생성 실패할 수 있으므로 **커널 버전/기하 캐시 포맷을 기록**하고 migration을 작성.

Native 프로젝트에는 command log 전체를 반드시 넣는 대신 **문서 스냅샷 + 필요한 history/undo 저널**의 트레이드오프 결정. autosave journal, crash rollback 기준을 분리.

## 2. schema 규칙

- `schema_version` 과 `app_version`, `kernel_name/kernel_version` 분리.
- stable opaque IDs, units, Coordinate Frame, tolerances, dependency DAG, references, report/source maps.
- forward unknown fields는 가능한 범위에서 보존; unsafe unknown geometry는 read-only placeholder + 경고.
- migration이 destructive할 경우 원본 read-only copy → derived upgraded file; silent overwrite 금지.
- validation before/after serialization: id uniqueness, DAG acyclic, references resolve, finite values, allowable object counts, resource limits.
- 파일 버전이 신버전이면 수정 금지/read-only 또는 명확한 unsupported 버전 메시지.

## 3. Atomic save + autosave

```text
Document snapshot (revision r)
 → validate data contract
 → write new temp in same filesystem/directory (random unique name)
 → flush + close + checksum / read-back parse
 → retain backup of last valid target (policy + availability)
 → atomic replacement where OS/filesystem supports required semantics
 → post-replacement verification + record saved revision
 → cleanup temp after safe commit
```

- Windows antivirus/indexer/파일 잠금/rename 실패 포함 retry/rollback 설계.
- OS·파일 시스템에 따라 atomic rename, durable flush 의미가 다르므로 **전 플랫폼 완전 무손실** 표현을 금지; crash fault injection과 실제 NTFS 실험으로 계약 확정.
- autosave는 원본과 다른 별도 경로, 수명/순환 개수와 dirty revision 추적. 실패한 autosave가 이전 유효 백업을 지우지 않게 함.
- 백업 순환(rotation)은 최신 정상 snapshot의 참조가 확보된 경우에만 실행.
- Restore는 사용자 승인 후 `Recovered Copy` 이름으로 열고 원본 자동 덮어쓰기 금지.
- 원치 않는 백그라운드 클라우드 통신·파일 업로드 없음.

## 6. Recovery / Migration QA

- E-01: 정상 문서 save/load/rebuild equals canonical domain state.
- E-02: truncated archive, corrupted CRC/checksum/JSON, duplicate ID, cyclic DAG → error with no filesystem mutation.
- E-03: v1 2D legacy document → v2 upgrade(copy) → geometry/layer/dimension preservation.
- E-04: file lock/disk full/permission denied/crash at each save stage → normal file/backup recoverable.
- E-05: kernel upgrade/rebuild failure → file read-only with last-valid geometry display or explicit inability.
- E-06: STEP file imported in independent CAD, back-convert then volume/BBox/face validity tolerances.
- E-07: STL exported with chosen tessellation settings and units, mesh orientation/gaps validated.## 영속 데이터 계약 및 버전

| 데이터 | 저장할 내용 | 재생성 가능 여부 | 안정 ID | 비고 |
|---|---|---|---|---|
| Document/Sketch/Feature | 파라미터, 평면, 의존성, 구속조건 | 아니오, 반드시 원본 보존 | 필수 | native authoritative data |
| B-Rep Shape | 커널 형상 결과, 유효성 메타데이터 | 예(커널/환경이 동일한 경우) | ShapeRevision | 재생성 가능한 캐시와 교차검증 필요 |
| Mesh/RenderScene | Vertex/Face/Material, GPU 데이터 | 예 | 원본 face/feature 매핑 | 설계 원본이 아님 |
| Drawing/Sheet | 2D 시트, 레이어, 치수 객체 및 뷰 링크 | 일부 | 필수 | 출력 스타일/단위 포함 |
| Audit/Undo | 명령 식별자, precondition, 결과 revision | 정책별 | CommandId | 재실행 가능성 증거 |

- 스키마 버전은 앱 버전 및 커널 버전과 별도로 기록합니다.
- 임시 파일 저장·재읽기 검증·원자 교체는 파일 시스템에 따라 전원 손실 지속성 보장이 달라집니다. 필요 시 flush/fsync(디렉터리 포함), Windows API 동작을 검증합니다.
- 마이그레이션 실패는 기존 파일 불변·읽기 전용 로드·복구 안내로 종료합니다.
- 패키지 압축·검사·확장 단계에서 ZIP bomb, traversal, oversized geometry 제한을 적용합니다.

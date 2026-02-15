# 구현 로드맵

> 문서 상태: **Current (MVP 우선순위)** + **Target (장기 확장 계획)** 를 함께 포함한다.

## 전체 Phase 개요

```
Phase 1 ─── 기반 구축 (Spec + Core)
  │
Phase 2 ─── 코드 생성 엔진
  │
Phase 3 ─── Tauri 앱 + GUI
  │
Phase 4 ─── 런타임 + HMU
  │
Phase 5 ─── 양방향 변환 + OpenAPI
  │
Phase 6 ─── 테스트 + 디버깅
  │
Phase 7 ─── 마켓플레이스 + 생태계
```

## Phase 0: 구현 선행 기능 (권장)

핵심 구현을 시작하기 전에, 실패 비용을 줄이기 위한 선행 작업이다.

### 작업 순서

| # | 선행 기능 | 목적 | 연관 문서 |
|---|-----------|------|-----------|
| 0.1 | Spec 버전/마이그레이션 시스템 | 버전 변화 시 자동 업그레이드/롤백 기준 확보 | `architecture/spec-format.md` |
| 0.2 | Resolver 결정 규칙 고정 | `{ "ref": "..." }` 해석의 결정론 보장 | `architecture/spec-format.md` |
| 0.3 | JSON Schema + 에러 포맷 표준화 | GUI/CLI가 동일한 검증 오류를 해석하도록 통일 | `architecture/spec-format.md` |
| 0.4 | Incremental 의존성 그래프 | 변경 영향 범위를 정확히 계산 | `architecture/codegen.md` |
| 0.5 | HMU Atomic 적용/롤백 | 부분 적용으로 인한 런타임 불일치 방지 | `architecture/runtime.md` |
| 0.6 | 실행 프리플라이트 체크 | 런타임/포트/env 문제를 Run 전에 차단 | `architecture/runtime.md` |
| 0.7 | Golden E2E 샘플 프로젝트 | 회귀 기준(스펙→코드→실행→HMU) 확보 | `architecture/runtime.md`, `architecture/tauri-app.md` |
| 0.8 | 이벤트 Payload 표준 스키마 | Rust↔JS 이벤트 계약 안정화 | `architecture/tauri-app.md` |

### 완료 기준
- 0.1~0.8이 순서대로 구현되어, 각 단계 산출물이 다음 단계의 입력으로 재사용된다.
- MVP 구현(Phase 1~4) 착수 전에 최소 0.1~0.6이 완료된다.
- CI에서 Golden E2E 샘플 1개를 기준으로 기본 회귀가 통과한다.

## Phase 1: 기반 구축

**목표**: Rash 스펙 포맷을 확정하고, 핵심 데이터 구조를 구현한다.

### 산출물
- `rash-spec` crate: 스펙 JSON 파싱/직렬화/검증
- `rash-ir` crate: 중간 표현(IR) 데이터 구조
- `rash-valid` crate: 스펙 유효성 검사

### 작업 항목

| # | 작업 | 설명 |
|---|------|------|
| 1.1 | 스펙 JSON Schema 정의 | rash.config, route, schema, model, middleware, handler 각각의 JSON Schema 작성 |
| 1.2 | Rust 타입 정의 | 스펙 → Rust struct/enum 매핑 (serde 기반) |
| 1.3 | 스펙 파서 구현 | JSON 파일 → Rust 타입 파싱 + 오류 리포팅 |
| 1.4 | IR 정의 | 스펙과 코드 사이의 중간 표현 구조체 |
| 1.5 | Spec → IR 변환기 | 파싱된 스펙을 IR로 변환 |
| 1.6 | 참조 해석기(Resolver) | `{ "ref": "..." }` 형식의 스펙 간 참조 해석 |
| 1.7 | Validator | 참조 무결성, 타입 일관성, 필수 필드 검증 |
| 1.8 | AST 노드 타입 정의 | Tier 0~3 노드 enum + JSON 직렬화 |
| 1.9 | CLI 프로토타입 | `rash validate`, `rash init` 명령어 |

### 완료 기준
- 예제 프로젝트의 모든 스펙 파일이 파싱/검증을 통과
- IR 변환이 정보 유실 없이 완료
- 유닛 테스트 커버리지 80%+

---

## Phase 2: 코드 생성 엔진

**목표**: IR에서 실행 가능한 코드를 생성하는 엔진을 구현한다.

### 산출물
- `rash-codegen` crate: 코드 생성 엔진
- TypeScript + Express 지원 (첫 번째 타겟)
- DTO 변환 (JSON Schema → Zod/struct/Pydantic)

### 작업 항목

| # | 작업 | 설명 |
|---|------|------|
| 2.1 | `LanguageEmitter` trait 설계 | 언어별 코드 생성 인터페이스 |
| 2.2 | `FrameworkAdapter` trait 설계 | 프레임워크별 어댑터 인터페이스 |
| 2.3 | `TypeScriptEmitter` 구현 | TS 문법 코드 생성 |
| 2.4 | `ExpressAdapter` 구현 | Express 라우트/미들웨어/진입점 |
| 2.5 | 스키마 → Zod 변환 | JSON Schema → Zod 스키마 코드 |
| 2.6 | 모델 → Prisma 변환 | 모델 스펙 → Prisma schema |
| 2.7 | AST → TS 코드 변환 | 핸들러 AST → TypeScript 함수 |
| 2.8 | NativeBridge 처리 | Tier 3 노드의 네이티브 코드 보존 |
| 2.9 | 프로젝트 scaffold 생성 | package.json, tsconfig 등 |
| 2.10 | Rust 타겟 (2nd) | `RustEmitter` + `ActixAdapter` |
| 2.11 | Python 타겟 (3rd) | `PythonEmitter` + `FastAPIAdapter` |
| 2.12 | Go 타겟 (4th) | `GoEmitter` + `GinAdapter` |

### 완료 기준
- 예제 프로젝트에서 TS/Express 코드 생성 후 `bun run` 으로 실행 가능
- 생성된 코드가 올바른 HTTP 응답 반환
- 최소 2개 언어 × 2개 프레임워크 조합 동작

---

## Phase 3: Tauri 앱 + GUI

**목표**: 데스크톱 앱을 구축하고 핵심 GUI 컴포넌트를 구현한다.

### 산출물
- Tauri v2 앱 초기 설정
- SolidJS 프론트엔드 기본 레이아웃
- 핵심 편집기 컴포넌트

### 작업 항목

| # | 작업 | 설명 |
|---|------|------|
| 3.1 | Tauri + SolidJS 프로젝트 초기화 | `create-tauri-app` + SolidJS 설정 |
| 3.2 | IPC 명령 등록 | Rust ↔ JS 통신 레이어 |
| 3.3 | 메인 레이아웃 구현 | Sidebar + MainPanel + BottomPanel |
| 3.4 | Route Explorer | 파일 탐색기 스타일 라우트 트리 |
| 3.5 | Route Editor | HTTP 메서드별 설정 편집 |
| 3.6 | Schema Editor | 필드 추가/삭제/타입 변경 GUI |
| 3.7 | Model Editor | DB 컬럼/관계 편집 |
| 3.8 | Handler Editor (v1) | AST 노드 시각 편집 (블록 기반) |
| 3.9 | Code Preview | 실시간 코드 변환 미리보기 |
| 3.10 | Monaco 통합 | NativeBridge 코드 편집용 |
| 3.11 | 프로젝트 생성/열기 | 프로젝트 관리 UI |
| 3.12 | 상태 관리 | SolidJS Store 설계 + IPC 동기화 |

### 완료 기준
- GUI에서 라우트/스키마/모델/미들웨어를 CRUD할 수 있음
- 편집 내용이 JSON 스펙 파일에 정확히 저장됨
- 코드 미리보기가 실시간으로 갱신됨

---

## Phase 4: 런타임 + HMU

**목표**: 앱 내에서 서버를 실행하고, 변경 사항을 실시간 적용한다.

### 산출물
- `rash-runtime` crate: 프로세스 관리
- HMU 프로토콜 + 클라이언트 라이브러리
- 서버 컨트롤 UI

### 작업 항목

| # | 작업 | 설명 |
|---|------|------|
| 4.1 | ProcessManager 구현 | child process 생성/관리/종료 |
| 4.2 | 로그 스트리밍 | stdout/stderr → 프론트엔드 실시간 전달 |
| 4.3 | 포트 감지 | 서버 시작 시 포트 자동 감지 |
| 4.4 | 서버 컨트롤 UI | 시작/정지/재시작 버튼 + 상태 표시 |
| 4.5 | 로그 뷰어 | 실시간 로그 표시 (필터링, 검색) |
| 4.6 | IncrementalCodegen | 변경분 감지 + 점진적 코드 재생성 |
| 4.7 | HMU 프로토콜 구현 | IPC/Unix Socket 기반 메시지 교환 |
| 4.8 | HMU TS 클라이언트 | Bun/Node용 HMU 수신 라이브러리 |
| 4.9 | HMU Python 클라이언트 | Python용 HMU 수신 라이브러리 |
| 4.10 | HMU 실패 핸들링 | 실패 시 자동 재시작 + 에러 표시 |
| 4.11 | 환경 변수 관리 | 환경별 변수 설정 UI |
| 4.12 | 런타임 감지 | 시스템 설치 런타임 자동 감지 |

### 완료 기준
- GUI에서 "Run" 버튼으로 서버 실행 가능
- 핸들러 수정 후 HMU로 즉시 반영 (서버 재시작 없이)
- 실시간 로그가 GUI에 표시됨

---

## Phase 5: 양방향 변환 + OpenAPI

**목표**: 기존 코드 → Rash 스펙 역변환, OpenAPI 호환을 구현한다.

### 산출물
- 코드 → 스펙 역파싱 엔진
- OpenAPI 임포트/익스포트
- `rash-openapi` crate

### 작업 항목

| # | 작업 | 설명 |
|---|------|------|
| 5.1 | Tree-sitter 통합 | TS/Rust/Python/Go 파서 바인딩 |
| 5.2 | Express 역파싱 | Express 코드 → 라우트 스펙 추출 |
| 5.3 | FastAPI 역파싱 | FastAPI 코드 → 라우트 스펙 추출 |
| 5.4 | 핸들러 역파싱 | 코드 함수 → AST 변환 |
| 5.5 | 스키마 역파싱 | Zod/TypeScript/Pydantic → JSON Schema |
| 5.6 | NativeBridge 자동 태깅 | 매핑 불가 코드 → Bridge 노드 래핑 |
| 5.7 | OpenAPI → Spec 임포트 | OpenAPI 3.1 JSON/YAML → Rash 스펙 변환 |
| 5.8 | Spec → OpenAPI 익스포트 | Rash 스펙 → OpenAPI 3.1 문서 생성 |
| 5.9 | 임포트 UI | 기존 프로젝트/OpenAPI 파일 임포트 워크플로우 |

### 완료 기준
- 기존 Express 프로젝트를 Rash로 임포트 후 동일하게 실행 가능
- 생성된 OpenAPI 문서가 Swagger UI에서 정상 렌더링
- Rash → 코드 → Rash 라운드트립 시 정보 유실 최소

---

## Phase 6: 테스트 + 디버깅

**목표**: 내장 테스트 러너, API 테스트 UI를 구현한다.

### 산출물
- 내장 API 테스트 러너
- 테스트 스펙 포맷
- 디버깅 도구

### 작업 항목

| # | 작업 | 설명 |
|---|------|------|
| 6.1 | 테스트 스펙 포맷 정의 | 테스트 케이스 JSON 형식 |
| 6.2 | 테스트 러너 엔진 | HTTP 요청 → 응답 검증 로직 |
| 6.3 | 테스트 UI | Postman 스타일 테스트 편집/실행 |
| 6.4 | 변수 캡처/치환 | 테스트 간 변수 공유 메커니즘 |
| 6.5 | 테스트 리포트 | 통과/실패 시각화 |
| 6.6 | 라우트별 자동 테스트 생성 | 스펙 기반 테스트 스텁 자동 생성 |

### 완료 기준
- GUI에서 API 테스트를 작성하고 실행할 수 있음
- 테스트 결과가 시각적으로 표시됨
- 라우트 스펙에서 테스트 스텁 자동 생성

---

## Phase 7: 마켓플레이스 + 생태계

**목표**: 미들웨어 마켓플레이스, 플러그인 시스템을 구축한다.

### 산출물
- 미들웨어 마켓플레이스
- 플러그인 API
- 커뮤니티 인프라

### 작업 항목

| # | 작업 | 설명 |
|---|------|------|
| 7.1 | 플러그인 시스템 설계 | 미들웨어/어댑터 플러그인 API |
| 7.2 | 미들웨어 패키지 포맷 | 배포 가능한 미들웨어 번들 형식 |
| 7.3 | 마켓플레이스 백엔드 | 레지스트리 서버 |
| 7.4 | 마켓플레이스 UI | 검색/설치/관리 인터페이스 |
| 7.5 | 추가 프레임워크 어댑터 | NestJS, Axum, Django, Echo 등 |
| 7.6 | 프로젝트 템플릿 | 시작 템플릿 (CRUD API, Auth 등) |
| 7.7 | 문서 사이트 | rash.dev 문서 + 가이드 |

### 완료 기준
- 미들웨어를 마켓플레이스에서 검색하고 설치할 수 있음
- 커스텀 어댑터를 플러그인으로 추가할 수 있음
- 프로젝트 템플릿으로 빠르게 시작 가능

---

## MVP 정의

**MVP = Phase 1 + Phase 2 + Phase 3 + Phase 4**

MVP에서 가능한 것:
- Rash 앱을 열고 새 프로젝트를 생성
- GUI에서 라우트, 스키마, 모델, 미들웨어를 설계
- 핸들러 로직을 AST 편집기로 작성
- TypeScript/Express 코드를 자동 생성
- 앱 내에서 서버를 실행하고 로그 확인
- 코드 변경 시 HMU로 즉시 반영

MVP에서 미루는 것:
- 기존 코드 역파싱 (Phase 5)
- OpenAPI 임포트/익스포트 (Phase 5)
- 내장 테스트 러너 (Phase 6)
- 마켓플레이스 (Phase 7)
- Rust/Python/Go 타겟 완성도 (Phase 2에서 기본만, 완성은 이후)

참고:
- UI 구조 문서에 `TestRunner` 슬롯이 등장하더라도 MVP에서는 노출하지 않거나 비활성 상태로 유지한다.

## 마일스톤 타임라인

```
Phase 1 ████████░░░░░░░░░░░░░░░░░░░░░░  기반 구축
Phase 2 ░░░░░░░░████████░░░░░░░░░░░░░░  코드 생성
Phase 3 ░░░░░░░░░░░░░░░░████████░░░░░░  GUI
Phase 4 ░░░░░░░░░░░░░░░░░░░░░░░░██████  런타임
        ──────────────────────────────
                    MVP 완성 ──────▶│

Phase 5 ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░██████  양방향 변환
Phase 6 ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░████  테스트
Phase 7 ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░████  마켓플레이스
```

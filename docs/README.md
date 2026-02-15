# Rash

**서버 애플리케이션을 GUI로 설계하고, 관리하고, 실행하는 데스크톱 앱**

## 비전

Rash는 Express, Fastify, Django, Actix, FastAPI 같은 서버 프레임워크를 **코드가 아닌 시각적 인터페이스**로 대체한다. 라우트, 미들웨어, 핸들러, DTO, DB 스키마를 모두 GUI에서 관리하고, 내부적으로 AST/JSON 기반 DSL로 로직을 표현하며, 이를 TypeScript, Rust, Python, Go 등 여러 언어의 실행 가능한 코드로 1:1 양방향 변환한다.

## 핵심 가치

| 가치 | 설명 |
|------|------|
| **Visual-First** | 라우트 트리, 미들웨어 체인, DTO 스키마를 GUI로 직관 관리 |
| **Language-Agnostic** | 하나의 스펙 → TS/Rust/Python/Go 코드로 양방향 변환 |
| **Full-Lifecycle** | 설계 → 코드 생성 → 실행 → 테스트를 앱 하나에서 완결 |
| **OpenAPI-Native** | 모든 스펙이 OpenAPI 3.1과 자연스럽게 호환 |
| **Git-Friendly** | 디렉토리 기반 프로젝트 구조로 버전 관리에 최적화 |

## 타겟 유저

- 반복적인 서버 보일러플레이트에 지친 백엔드 개발자
- API 설계를 시각적으로 관리하고 싶은 테크 리드
- 여러 언어/프레임워크에 걸친 서버를 유지보수하는 팀
- 서버 개발을 빠르게 프로토타이핑하고 싶은 스타트업

## 기술 스택

| 계층 | 기술 |
|------|------|
| 데스크톱 앱 | [Tauri v2](https://tauri.app/) |
| 프론트엔드 | [SolidJS](https://www.solidjs.com/) + TypeScript |
| 백엔드 (앱 내부) | Rust |
| 코드 에디터 | Monaco Editor |
| 코드 생성 엔진 | Rust (trait 기반 Emitter/Adapter) |
| 서버 런타임 | Child Process (Bun/Node/Python/Cargo) |
| 스펙 포맷 | JSON (AST/DSL) |

## 문서 구조

```
docs/
├── README.md                          # 이 파일 - 프로젝트 개요
├── architecture/
│   ├── overview.md                    # 시스템 아키텍처 총괄
│   ├── spec-format.md                 # 프로젝트 파일/스펙 포맷
│   ├── ast-dsl.md                     # AST/DSL 노드 설계
│   ├── codegen.md                     # 코드 생성 파이프라인
│   ├── tauri-app.md                   # Tauri 앱 구조
│   └── runtime.md                     # 런타임 시스템
└── roadmap.md                         # 구현 로드맵
```

### 읽기 순서

1. **처음 접하는 경우**: 이 파일 → [아키텍처 총괄](architecture/overview.md) → [스펙 포맷](architecture/spec-format.md)
2. **DSL/AST에 관심**: [AST/DSL 설계](architecture/ast-dsl.md)
3. **코드 변환기 작업**: [코드 생성 파이프라인](architecture/codegen.md)
4. **앱 개발**: [Tauri 앱 구조](architecture/tauri-app.md)
5. **런타임/실행**: [런타임 시스템](architecture/runtime.md)
6. **로드맵 확인**: [구현 로드맵](roadmap.md)

## 용어 사전

| 용어 | 의미 |
|------|------|
| **Spec** | Rash 프로젝트의 JSON 기반 서버 정의 파일 |
| **AST Node** | 핸들러 로직을 표현하는 추상 구문 트리 노드 |
| **DSL** | 핸들러 내부 로직을 언어 독립적으로 표현하는 도메인 특화 언어 |
| **NativeBridge** | 특정 언어 생태계의 네이티브 라이브러리를 DSL에서 호출하는 메커니즘 |
| **Emitter** | 언어별 코드 출력기 (TypeScriptEmitter, RustEmitter 등) |
| **Adapter** | 프레임워크별 코드 변환기 (ExpressAdapter, ActixAdapter 등) |
| **HMU** | Hot Module Update - 코드 변경 시 실시간 서버 갱신 (프론트의 HMR에 대응) |
| **Tier** | AST 노드의 이식성 등급 (Universal → Domain → Utility → Bridge) |

## 라이선스

TBD

# 코드 생성 파이프라인

Rash의 코드 생성은 Spec → IR → Emitter → Framework Adapter → Code 순서로 진행된다. Rust의 trait 시스템을 활용하여 언어와 프레임워크를 조합할 수 있도록 설계한다.

> 문서 상태: **Current (MVP 구현 우선순위)** + **Target (최종 지원 범위)** 를 함께 표시한다.

## 파이프라인 개요

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│   Spec   │────▶│    IR    │────▶│ Emitter  │────▶│ Adapter  │────▶│   Code   │
│  (JSON)  │     │(Rust 구조)│    │ (언어별)  │     │(프레임워크)│    │  (파일)  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘     └──────────┘
     │                                                                    │
     └─────────────────── 역방향 파싱 ◀──────────────────────────────────┘
```

### 각 단계의 역할

| 단계 | 입력 | 출력 | 역할 |
|------|------|------|------|
| **Spec Parser** | JSON 파일들 | `IR` 구조체 | 스펙 JSON을 Rust 타입으로 파싱 |
| **IR** | 파싱된 스펙 | 정규화된 중간 표현 | 언어/프레임워크 독립적인 중간 표현 |
| **Emitter** | IR | 언어별 코드 조각 | 각 언어의 문법에 맞는 코드 생성 |
| **Adapter** | 코드 조각 | 프레임워크 특화 코드 | 프레임워크의 관례/구조에 맞게 조립 |
| **Writer** | 최종 코드 | 파일 시스템 | 프로젝트 구조에 맞게 파일 출력 |

## IR (Intermediate Representation)

IR은 스펙과 최종 코드 사이의 중간 표현이다. 스펙의 JSON 구조를 Rust의 강타입 구조체로 변환한 것이다.

```rust
/// 프로젝트 전체의 IR
pub struct ProjectIR {
    pub config: ProjectConfig,
    pub routes: Vec<RouteIR>,
    pub schemas: Vec<SchemaIR>,
    pub models: Vec<ModelIR>,
    pub middleware: Vec<MiddlewareIR>,
    pub handlers: Vec<HandlerIR>,
}

/// 하나의 라우트 IR
pub struct RouteIR {
    pub path: RoutePath,
    pub methods: HashMap<HttpMethod, EndpointIR>,
    pub tags: Vec<String>,
}

pub struct EndpointIR {
    pub operation_id: String,
    pub summary: Option<String>,
    pub handler_ref: HandlerRef,
    pub middleware: Vec<MiddlewareRef>,
    pub request: RequestIR,
    pub response: HashMap<StatusCode, ResponseIR>,
}

/// 핸들러 AST의 IR
pub struct HandlerIR {
    pub name: String,
    pub is_async: bool,
    pub params: Vec<ParamIR>,
    pub return_type: TypeIR,
    pub body: Vec<StatementIR>,
    pub max_tier: Tier,
    pub bridge_languages: HashSet<Language>,
}

/// AST Statement IR
pub enum StatementIR {
    Let { name: String, type_: Option<TypeIR>, value: ExprIR },
    Assign { target: ExprIR, value: ExprIR },
    Return { value: Option<ExprIR> },
    If { condition: ExprIR, then_: Vec<StatementIR>, else_: Option<Vec<StatementIR>> },
    For { binding: String, iterable: ExprIR, body: Vec<StatementIR> },
    While { condition: ExprIR, body: Vec<StatementIR> },
    Match { expr: ExprIR, arms: Vec<MatchArm> },
    TryCatch { try_: Vec<StatementIR>, catch_: CatchClause, finally_: Option<Vec<StatementIR>> },
    Throw { value: ExprIR },
    Expression { expr: ExprIR },
}

/// AST Expression IR
pub enum ExprIR {
    Literal(LiteralValue),
    Identifier(String),
    Binary { op: BinaryOp, left: Box<ExprIR>, right: Box<ExprIR> },
    Unary { op: UnaryOp, operand: Box<ExprIR> },
    Call { callee: Box<ExprIR>, args: Vec<ExprIR> },
    Member { object: Box<ExprIR>, property: String },
    Index { object: Box<ExprIR>, index: Box<ExprIR> },
    Object { properties: Vec<(String, ExprIR)> },
    Array { elements: Vec<ExprIR> },
    ArrowFn { params: Vec<String>, body: Vec<StatementIR> },
    Await { expr: Box<ExprIR> },
    Pipe { stages: Vec<ExprIR> },
    Template { parts: Vec<TemplatePart> },

    // Domain (Tier 1)
    DbQuery(DbQueryIR),
    DbMutate(DbMutateIR),
    HttpRespond(HttpRespondIR),
    CtxGet(CtxGetIR),
    Validate(ValidateIR),

    // Bridge (Tier 3)
    NativeBridge(NativeBridgeIR),
}
```

## LanguageEmitter Trait

각 언어별 코드 생성기는 `LanguageEmitter` trait을 구현한다.

```rust
pub trait LanguageEmitter {
    /// 이 Emitter가 담당하는 언어
    fn language(&self) -> Language;

    /// Statement IR → 코드 문자열
    fn emit_statement(&self, stmt: &StatementIR, ctx: &mut EmitContext) -> String;

    /// Expression IR → 코드 문자열
    fn emit_expression(&self, expr: &ExprIR, ctx: &mut EmitContext) -> String;

    /// 타입 IR → 언어별 타입 문자열
    fn emit_type(&self, type_: &TypeIR) -> String;

    /// 스키마 → DTO 코드 (Zod, struct, Pydantic 등)
    fn emit_schema(&self, schema: &SchemaIR, ctx: &mut EmitContext) -> String;

    /// 모델 → ORM 코드 (Prisma, SeaORM, SQLAlchemy 등)
    fn emit_model(&self, model: &ModelIR, ctx: &mut EmitContext) -> String;

    /// import 문 생성
    fn emit_imports(&self, imports: &[ImportIR]) -> String;

    /// 파일 확장자
    fn file_extension(&self) -> &str;

    /// 들여쓰기 스타일
    fn indent_style(&self) -> IndentStyle;
}
```

### 구현체 목록

| 언어 | 구현체 | 파일 확장자 |
|------|--------|-----------|
| TypeScript | `TypeScriptEmitter` | `.ts` |
| Rust | `RustEmitter` | `.rs` |
| Python | `PythonEmitter` | `.py` |
| Go | `GoEmitter` | `.go` |

### TypeScript Emitter 예시 (일부)

```rust
impl LanguageEmitter for TypeScriptEmitter {
    fn language(&self) -> Language { Language::TypeScript }

    fn emit_statement(&self, stmt: &StatementIR, ctx: &mut EmitContext) -> String {
        match stmt {
            StatementIR::Let { name, type_, value } => {
                let type_ann = type_.as_ref()
                    .map(|t| format!(": {}", self.emit_type(t)))
                    .unwrap_or_default();
                let val = self.emit_expression(value, ctx);
                format!("const {}{} = {};", name, type_ann, val)
            }
            StatementIR::If { condition, then_, else_ } => {
                let cond = self.emit_expression(condition, ctx);
                let then_body = self.emit_block(then_, ctx);
                match else_ {
                    Some(else_stmts) => {
                        let else_body = self.emit_block(else_stmts, ctx);
                        format!("if ({}) {{\n{}\n}} else {{\n{}\n}}", cond, then_body, else_body)
                    }
                    None => format!("if ({}) {{\n{}\n}}", cond, then_body),
                }
            }
            // ... 다른 Statement 처리
        }
    }

    fn emit_type(&self, type_: &TypeIR) -> String {
        match type_ {
            TypeIR::String => "string".into(),
            TypeIR::Number => "number".into(),
            TypeIR::Boolean => "boolean".into(),
            TypeIR::Array(inner) => format!("{}[]", self.emit_type(inner)),
            TypeIR::Optional(inner) => format!("{} | null", self.emit_type(inner)),
            TypeIR::Ref(name) => name.clone(),
            // ...
        }
    }

    fn file_extension(&self) -> &str { "ts" }
}
```

## FrameworkAdapter Trait

프레임워크별 관례를 적용하는 어댑터이다.

```rust
pub trait FrameworkAdapter {
    /// 이 Adapter가 담당하는 프레임워크
    fn framework(&self) -> Framework;

    /// 호환되는 언어
    fn compatible_language(&self) -> Language;

    /// 라우트 등록 코드 생성
    fn emit_route_registration(&self, route: &RouteIR, ctx: &mut EmitContext) -> String;

    /// 미들웨어 적용 코드 생성
    fn emit_middleware_apply(&self, mw: &MiddlewareRef, ctx: &mut EmitContext) -> String;

    /// 핸들러 함수 시그니처 생성
    fn emit_handler_signature(&self, handler: &HandlerIR, ctx: &mut EmitContext) -> String;

    /// 진입점 (main 함수 / app 초기화) 생성
    fn emit_entrypoint(&self, project: &ProjectIR, ctx: &mut EmitContext) -> String;

    /// 프로젝트 설정 파일 생성 (package.json, Cargo.toml 등)
    fn emit_project_config(&self, project: &ProjectIR) -> Vec<(String, String)>;

    /// DomainNode를 프레임워크 특화 코드로 변환
    fn emit_domain_node(&self, node: &ExprIR, ctx: &mut EmitContext) -> Option<String>;

    /// 요청 컨텍스트 접근 방식
    fn ctx_access_pattern(&self) -> CtxAccessPattern;
}
```

### 구현체 목록

| 프레임워크 | 언어 | 구현체 |
|-----------|------|--------|
| Express | TypeScript | `ExpressAdapter` |
| Fastify | TypeScript | `FastifyAdapter` |
| Hono | TypeScript | `HonoAdapter` |
| Elysia | TypeScript | `ElysiaAdapter` |
| NestJS | TypeScript | `NestJSAdapter` |
| Actix | Rust | `ActixAdapter` |
| Axum | Rust | `AxumAdapter` |
| FastAPI | Python | `FastAPIAdapter` |
| Django | Python | `DjangoAdapter` |
| Gin | Go | `GinAdapter` |
| Echo | Go | `EchoAdapter` |

### Express Adapter 예시 (일부)

```rust
impl FrameworkAdapter for ExpressAdapter {
    fn framework(&self) -> Framework { Framework::Express }
    fn compatible_language(&self) -> Language { Language::TypeScript }

    fn emit_route_registration(&self, route: &RouteIR, ctx: &mut EmitContext) -> String {
        let mut lines = Vec::new();
        for (method, endpoint) in &route.methods {
            let method_lower = method.to_string().to_lowercase();
            let mw_chain: Vec<String> = endpoint.middleware.iter()
                .map(|mw| self.emit_middleware_apply(mw, ctx))
                .collect();
            let mw_str = if mw_chain.is_empty() {
                String::new()
            } else {
                format!("{}, ", mw_chain.join(", "))
            };
            lines.push(format!(
                "router.{}(\"{}\", {}{});",
                method_lower, route.path, mw_str, endpoint.handler_ref
            ));
        }
        lines.join("\n")
    }

    fn emit_entrypoint(&self, project: &ProjectIR, ctx: &mut EmitContext) -> String {
        format!(r#"
import express from "express";
import {{ registerRoutes }} from "./routes";

const app = express();
app.use(express.json());

registerRoutes(app);

app.listen({port}, () => {{
  console.log("Server running on port {port}");
}});
"#, port = project.config.server.port)
    }
}
```

## 코드 생성 실행 흐름

```rust
pub struct CodeGenerator {
    emitter: Box<dyn LanguageEmitter>,
    adapter: Box<dyn FrameworkAdapter>,
}

impl CodeGenerator {
    pub fn new(language: Language, framework: Framework) -> Result<Self> {
        let emitter = create_emitter(language)?;
        let adapter = create_adapter(framework)?;

        // 호환성 검사
        if adapter.compatible_language() != language {
            return Err(Error::IncompatibleTarget(language, framework));
        }

        Ok(Self { emitter, adapter })
    }

    pub fn generate(&self, project: &ProjectIR) -> Result<GeneratedProject> {
        let mut output = GeneratedProject::new();
        let mut ctx = EmitContext::new();

        // 1. 스키마 (DTO) 생성
        for schema in &project.schemas {
            let code = self.emitter.emit_schema(schema, &mut ctx);
            let path = format!("src/schemas/{}.{}", schema.name, self.emitter.file_extension());
            output.add_file(path, code);
        }

        // 2. 모델 (ORM) 생성
        for model in &project.models {
            let code = self.emitter.emit_model(model, &mut ctx);
            let path = format!("src/models/{}.{}", model.name, self.emitter.file_extension());
            output.add_file(path, code);
        }

        // 3. 미들웨어 생성
        for mw in &project.middleware {
            let code = self.emit_middleware(mw, &mut ctx);
            let path = format!("src/middleware/{}.{}", mw.name, self.emitter.file_extension());
            output.add_file(path, code);
        }

        // 4. 핸들러 생성
        for handler in &project.handlers {
            let code = self.emit_handler(handler, &mut ctx);
            let path = format!("src/handlers/{}.{}", handler.name, self.emitter.file_extension());
            output.add_file(path, code);
        }

        // 5. 라우트 등록 코드 생성
        let route_code = self.emit_routes(&project.routes, &mut ctx);
        output.add_file(
            format!("src/routes/index.{}", self.emitter.file_extension()),
            route_code,
        );

        // 6. 진입점 생성
        let entry = self.adapter.emit_entrypoint(project, &mut ctx);
        output.add_file(
            format!("src/index.{}", self.emitter.file_extension()),
            entry,
        );

        // 7. 프로젝트 설정 파일 생성
        for (path, content) in self.adapter.emit_project_config(project) {
            output.add_file(path, content);
        }

        Ok(output)
    }
}
```

## 양방향 변환 (Code → Spec)

기존 코드를 Rash 스펙으로 가져오는 역파싱 기능이다.

### 전략

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Source   │────▶│  Parser  │────▶│ Analyzer │────▶│   Spec   │
│  Code    │     │(Tree-sit)│     │(패턴매칭) │     │  (JSON)  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
```

### 단계별 설명

1. **파싱**: Tree-sitter를 사용하여 소스 코드를 AST로 파싱
2. **프레임워크 감지**: 라우트 등록 패턴, import 문 등으로 프레임워크 식별
3. **구조 추출**:
   - 라우트 경로/메서드 추출
   - 핸들러 함수 경계 식별
   - 미들웨어 체인 추출
   - DTO/스키마 추출 (타입 정의, Zod 스키마, Pydantic 모델 등)
4. **AST 변환**: 추출된 코드 구조를 Rash AST 노드로 변환
5. **NativeBridge 태깅**: 매핑 불가능한 코드는 NativeBridge로 래핑

### 역파싱 예시

**입력 (Express 코드)**:
```typescript
app.get("/api/users/:id", authMiddleware, async (req, res) => {
  const user = await prisma.user.findUnique({ where: { id: req.params.id } });
  if (!user) return res.status(404).json({ error: "Not found" });
  res.json(user);
});
```

**출력 (Spec)**:
- `routes/api/users/[id].route.json` 생성
- `handlers/users.handler.json`에 `getUser` AST 추가
- 미들웨어 참조 `{ "ref": "auth" }` 연결

### 역파싱 제약 사항

| 구분 | 설명 |
|------|------|
| 완전 변환 | 표준 CRUD 패턴, 라우트 등록, DTO 타입 |
| 부분 변환 | 복잡한 비즈니스 로직 (일부 NativeBridge로 래핑) |
| 변환 불가 | 동적 라우트 등록, eval, 메타프로그래밍 |

## OpenAPI 3.1 호환

### Spec → OpenAPI 변환

Rash 스펙에서 OpenAPI 3.1 문서를 자동 생성한다.

```
[Rash Spec]
    │
    ▼
[OpenAPI Generator]
    │
    ├── routes → paths
    ├── schemas → components/schemas
    ├── middleware (auth) → securitySchemes
    ├── request/response → requestBody/responses
    └── tags → tags
    │
    ▼
[openapi.json / openapi.yaml]
```

### OpenAPI → Spec 임포트

기존 OpenAPI 문서를 Rash 프로젝트로 가져온다.

```
[openapi.json]
    │
    ▼
[OpenAPI Parser]
    │
    ├── paths → routes/
    ├── components/schemas → schemas/
    ├── securitySchemes → middleware/
    └── requestBody/responses → 스키마 참조 연결
    │
    ▼
[Rash Spec 파일들]
```

핸들러 본문은 빈 상태(스텁)로 생성되며, 사용자가 GUI에서 직접 채운다.

## Incremental 의존성 그래프

점진적 코드 생성의 정확도를 위해 `SpecDependencyGraph`를 먼저 구축한다.

### 그래프 노드/엣지

- 노드: `route`, `schema`, `model`, `middleware`, `handler`, `generated-file`
- 엣지: `A -> B` (A가 변경되면 B를 재생성해야 함)

예시 엣지:
- `schema:UserResponse -> handler:users.getUser`
- `handler:users.getUser -> generated-file:src/handlers/users.ts`
- `middleware:auth -> route:/v1/users`

### 변경 영향 계산 규칙

1. 변경된 스펙을 시작점으로 그래프 탐색(BFS/DFS)
2. `generated-file` 노드까지 도달한 모든 경로를 수집
3. 중복 파일을 제거하고 안정적인 순서(topological order)로 정렬
4. 결과를 `FileChangePlan`으로 반환

```rust
pub struct SpecDependencyGraph {
    // key: canonical spec id, value: dependents
    edges: HashMap<NodeId, HashSet<NodeId>>,
}

pub struct FileChangePlan {
    pub affected_specs: Vec<NodeId>,
    pub affected_files: Vec<PathBuf>,
    pub requires_full_regen: bool,
}
```

`requires_full_regen = true` 조건:
- 타겟 언어/프레임워크 변경
- 전역 설정(`rash.config.json.codegen`, 글로벌 middleware) 변경
- 그래프 무결성 실패(순환/누락)

## 점진적 코드 생성 (Incremental Codegen)

전체를 매번 재생성하지 않고, 변경된 부분만 재생성한다.

```rust
pub struct IncrementalCodegen {
    cache: CodegenCache,
}

impl IncrementalCodegen {
    /// 변경된 스펙 파일을 감지하고, 영향받는 코드만 재생성
    pub fn regenerate(&mut self, changes: &[SpecChange]) -> Result<Vec<FileChange>> {
        let mut file_changes = Vec::new();

        for change in changes {
            match change {
                SpecChange::RouteModified(route) => {
                    // 라우트 파일 + 연관 핸들러 재생성
                    file_changes.extend(self.regen_route(route)?);
                }
                SpecChange::SchemaModified(schema) => {
                    // 스키마 파일 + 이 스키마를 참조하는 모든 파일 재생성
                    file_changes.extend(self.regen_schema_and_dependents(schema)?);
                }
                SpecChange::HandlerModified(handler) => {
                    // 핸들러 파일만 재생성
                    file_changes.extend(self.regen_handler(handler)?);
                }
                // ...
            }
        }

        self.cache.update(&file_changes);
        Ok(file_changes)
    }
}
```

## 지원 타겟 매트릭스

범례:
- `MVP`: Phase 1~4에서 우선 구현
- `Planned`: 설계상 지원 대상 (Phase 5+ 또는 후속 릴리즈)

| | Express | Fastify | Hono | Elysia | NestJS | Actix | Axum | FastAPI | Django | Gin | Echo |
|-|---------|---------|------|--------|--------|-------|------|---------|--------|-----|------|
| **상태** | MVP | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned |
| **라우트** | MVP | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned |
| **미들웨어** | MVP | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned |
| **DTO/스키마** | MVP | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned |
| **ORM** | Prisma (MVP) | Prisma | Prisma | Prisma | TypeORM | SeaORM | SeaORM | SQLAlchemy | Django ORM | GORM | GORM |
| **인증** | MVP | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned |
| **WebSocket** | Planned | Planned | Planned | Planned | Planned | Planned | Planned | - | - | Planned | Planned |

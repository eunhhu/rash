# AST/DSL 설계

Rash의 핸들러 로직은 언어 독립적인 AST(추상 구문 트리) 형태로 저장된다. GUI에서 조작하면 내부적으로 JSON AST로 직렬화되며, 이 AST를 각 언어의 코드로 1:1 양방향 변환한다.

## 설계 원칙

1. **언어 독립성**: 어떤 프로그래밍 언어에도 매핑할 수 있는 범용 노드 구조
2. **표현력**: 일반적인 서버 로직(CRUD, 인증, 검증, DB 쿼리)을 충분히 표현
3. **이식성 투명성**: 각 노드가 어느 언어에서든 동일하게 동작하는지, 특정 언어에 종속되는지 Tier로 명시
4. **양방향 변환**: 코드 → AST 역파싱 가능. NativeBridge 노드로 네이티브 코드 보존

## Tier 시스템

AST 노드는 이식성(portability) 수준에 따라 4개 Tier로 분류된다.

```
┌──────────────────────────────────────────────────────┐
│ Tier 0: Universal                                     │
│ 모든 언어에서 동일하게 동작                           │
│ if, for, while, match, let, return, 산술/논리 연산    │
├──────────────────────────────────────────────────────┤
│ Tier 1: Domain                                        │
│ 서버 도메인에 특화, Rash가 각 언어 매핑을 제공        │
│ db.query, http.respond, ctx.get, validate, hash       │
├──────────────────────────────────────────────────────┤
│ Tier 2: Utility                                       │
│ 범용 유틸리티, 대부분의 언어에 대응 존재              │
│ json.parse, date.now, string.split, array.map         │
├──────────────────────────────────────────────────────┤
│ Tier 3: Bridge                                        │
│ 특정 언어/생태계 전용. 이 노드가 있으면 언어 고정     │
│ NativeBridge("npm:bcrypt"), NativeBridge("pip:redis") │
└──────────────────────────────────────────────────────┘
```

### Tier별 규칙

| Tier | 양방향 변환 | 언어 고정 | 예시 |
|------|------------|----------|------|
| 0 | 완전 가능 | X | `if`, `for`, `let`, `+`, `==` |
| 1 | 완전 가능 | X | `db.findOne`, `ctx.body`, `respond(200)` |
| 2 | 대부분 가능 | X (일부 경고) | `JSON.parse`, `Date.now` |
| 3 | 불가 (bridge 보존) | O | `require("bcrypt").hash(...)` |

**핵심 규칙**: Tier 3 노드가 하나라도 존재하면, 해당 핸들러는 특정 언어로 **고정**된다. GUI에서 언어 전환 시 경고를 표시한다.

## AST 노드 타입 계층

### 최상위 분류

```
AstNode
├── Statement (문)
│   ├── LetStatement          # 변수 선언
│   ├── AssignStatement       # 변수 할당
│   ├── ReturnStatement       # 반환
│   ├── IfStatement           # 조건분기
│   ├── ForStatement          # 반복 (for-in)
│   ├── WhileStatement        # 반복 (while)
│   ├── MatchStatement        # 패턴 매칭
│   ├── TryCatchStatement     # 예외 처리
│   ├── ThrowStatement        # 예외 발생
│   └── ExpressionStatement   # 표현식을 문으로 사용
│
├── Expression (식)
│   ├── Literal               # 리터럴 (string, number, boolean, null)
│   ├── Identifier            # 변수 참조
│   ├── BinaryExpr            # 이항 연산 (a + b, a == b)
│   ├── UnaryExpr             # 단항 연산 (!a, -a)
│   ├── CallExpr              # 함수 호출
│   ├── MemberExpr            # 멤버 접근 (a.b)
│   ├── IndexExpr             # 인덱스 접근 (a[0])
│   ├── ObjectExpr            # 객체 리터럴 { key: value }
│   ├── ArrayExpr             # 배열 리터럴 [1, 2, 3]
│   ├── ArrowFn               # 익명 함수 (콜백)
│   ├── AwaitExpr             # 비동기 대기
│   ├── PipeExpr              # 파이프 체이닝 (a |> b |> c)
│   └── TemplateString        # 템플릿 문자열
│
├── DomainNode (Tier 1: 서버 도메인 전용)
│   ├── DbQuery               # DB 쿼리
│   ├── DbMutate              # DB 변경 (insert/update/delete)
│   ├── HttpRespond            # HTTP 응답 반환
│   ├── CtxGet                # 요청 컨텍스트 접근
│   ├── Validate              # 스키마 검증
│   ├── HashPassword          # 비밀번호 해싱
│   ├── VerifyPassword        # 비밀번호 검증
│   ├── SignToken             # JWT 서명
│   ├── VerifyToken           # JWT 검증
│   ├── SendEmail             # 이메일 전송
│   ├── EmitEvent             # 이벤트 발행
│   └── LogMessage            # 로깅
│
└── BridgeNode (Tier 3: 네이티브 바인딩)
    └── NativeBridge           # 특정 언어 패키지 호출
```

## 노드 JSON 구조

모든 AST 노드는 다음 공통 구조를 가진다:

```json
{
  "type": "노드 타입",
  "tier": 0,
  "loc": { "line": 1, "col": 0 },
  "meta": {}
}
```

### Statement 예시

#### LetStatement
```json
{
  "type": "LetStatement",
  "tier": 0,
  "name": "user",
  "valueType": { "ref": "UserResponse", "nullable": true },
  "value": {
    "type": "AwaitExpr",
    "tier": 1,
    "expr": {
      "type": "DbQuery",
      "tier": 1,
      "model": "User",
      "operation": "findUnique",
      "where": {
        "id": {
          "type": "CtxGet",
          "tier": 1,
          "path": "params.id"
        }
      }
    }
  }
}
```

#### IfStatement
```json
{
  "type": "IfStatement",
  "tier": 0,
  "condition": {
    "type": "BinaryExpr",
    "tier": 0,
    "operator": "==",
    "left": { "type": "Identifier", "tier": 0, "name": "user" },
    "right": { "type": "Literal", "tier": 0, "value": null }
  },
  "then": [
    {
      "type": "ReturnStatement",
      "tier": 0,
      "value": {
        "type": "HttpRespond",
        "tier": 1,
        "status": 404,
        "body": {
          "type": "ObjectExpr",
          "tier": 0,
          "properties": {
            "message": { "type": "Literal", "tier": 0, "value": "User not found" },
            "code": { "type": "Literal", "tier": 0, "value": "NOT_FOUND" }
          }
        }
      }
    }
  ],
  "else": null
}
```

### DomainNode 예시

#### DbQuery
```json
{
  "type": "DbQuery",
  "tier": 1,
  "model": "User",
  "operation": "findMany",
  "where": {
    "role": { "type": "Literal", "tier": 0, "value": "admin" },
    "deletedAt": { "type": "Literal", "tier": 0, "value": null }
  },
  "orderBy": { "createdAt": "desc" },
  "skip": { "type": "Identifier", "tier": 0, "name": "offset" },
  "take": { "type": "Identifier", "tier": 0, "name": "limit" },
  "select": ["id", "email", "name", "role", "createdAt"]
}
```

#### HttpRespond
```json
{
  "type": "HttpRespond",
  "tier": 1,
  "status": 200,
  "headers": {
    "X-Total-Count": { "type": "Identifier", "tier": 0, "name": "total" }
  },
  "body": {
    "type": "ObjectExpr",
    "tier": 0,
    "properties": {
      "data": { "type": "Identifier", "tier": 0, "name": "users" },
      "total": { "type": "Identifier", "tier": 0, "name": "total" },
      "page": { "type": "Identifier", "tier": 0, "name": "page" },
      "limit": { "type": "Identifier", "tier": 0, "name": "limit" }
    }
  }
}
```

### NativeBridge 예시

```json
{
  "type": "NativeBridge",
  "tier": 3,
  "language": "typescript",
  "package": "npm:bcrypt",
  "import": { "name": "bcrypt", "from": "bcrypt" },
  "call": {
    "method": "bcrypt.hash",
    "args": [
      { "type": "Identifier", "tier": 0, "name": "password" },
      { "type": "Literal", "tier": 0, "value": 10 }
    ]
  },
  "returnType": "string",
  "fallback": {
    "description": "다른 언어로 변환 시, 이 bridge를 대체할 Domain 노드",
    "node": {
      "type": "HashPassword",
      "tier": 1,
      "input": { "type": "Identifier", "tier": 0, "name": "password" },
      "algorithm": "bcrypt",
      "rounds": 10
    }
  }
}
```

**NativeBridge의 핵심**: `fallback` 필드가 있으면, 해당 언어가 아닌 다른 언어로 변환할 때 fallback 노드를 사용한다. fallback이 없으면 언어가 완전히 고정된다.

## 핸들러 파일 (*.handler.json) 전체 예시

`GET /api/v1/users/:id` 핸들러의 전체 AST:

```json
{
  "$schema": "https://rash.dev/schemas/handler.json",
  "name": "getUser",
  "description": "ID로 사용자 조회",
  "async": true,

  "params": {
    "ctx": {
      "type": "RequestContext",
      "description": "요청 컨텍스트 (params, query, body, headers 등)"
    }
  },

  "returnType": { "ref": "HttpResponse" },

  "body": [
    {
      "type": "LetStatement",
      "tier": 0,
      "name": "userId",
      "value": {
        "type": "CtxGet",
        "tier": 1,
        "path": "params.id"
      }
    },
    {
      "type": "LetStatement",
      "tier": 0,
      "name": "user",
      "value": {
        "type": "AwaitExpr",
        "tier": 1,
        "expr": {
          "type": "DbQuery",
          "tier": 1,
          "model": "User",
          "operation": "findUnique",
          "where": {
            "id": { "type": "Identifier", "tier": 0, "name": "userId" }
          }
        }
      }
    },
    {
      "type": "IfStatement",
      "tier": 0,
      "condition": {
        "type": "BinaryExpr",
        "tier": 0,
        "operator": "==",
        "left": { "type": "Identifier", "tier": 0, "name": "user" },
        "right": { "type": "Literal", "tier": 0, "value": null }
      },
      "then": [
        {
          "type": "ReturnStatement",
          "tier": 0,
          "value": {
            "type": "HttpRespond",
            "tier": 1,
            "status": 404,
            "body": {
              "type": "ObjectExpr",
              "tier": 0,
              "properties": {
                "message": { "type": "Literal", "tier": 0, "value": "User not found" },
                "code": { "type": "Literal", "tier": 0, "value": "NOT_FOUND" }
              }
            }
          }
        }
      ],
      "else": null
    },
    {
      "type": "ReturnStatement",
      "tier": 0,
      "value": {
        "type": "HttpRespond",
        "tier": 1,
        "status": 200,
        "body": { "type": "Identifier", "tier": 0, "name": "user" }
      }
    }
  ],

  "meta": {
    "maxTier": 1,
    "languages": ["typescript", "rust", "python", "go"],
    "bridges": []
  }
}
```

### 위 AST에서 생성되는 코드

**TypeScript (Express)**
```typescript
export async function getUser(ctx: RequestContext): Promise<HttpResponse> {
  const userId = ctx.params.id;
  const user = await prisma.user.findUnique({ where: { id: userId } });

  if (user == null) {
    return ctx.json(404, { message: "User not found", code: "NOT_FOUND" });
  }

  return ctx.json(200, user);
}
```

**Rust (Actix)**
```rust
pub async fn get_user(ctx: RequestContext) -> impl Responder {
    let user_id = ctx.params().get("id");
    let user = User::find_by_id(user_id)
        .one(&ctx.db())
        .await?;

    match user {
        None => HttpResponse::NotFound().json(json!({
            "message": "User not found",
            "code": "NOT_FOUND"
        })),
        Some(user) => HttpResponse::Ok().json(user),
    }
}
```

**Python (FastAPI)**
```python
async def get_user(ctx: RequestContext) -> HttpResponse:
    user_id = ctx.params["id"]
    user = await User.get_or_none(id=user_id)

    if user is None:
        return JSONResponse(
            status_code=404,
            content={"message": "User not found", "code": "NOT_FOUND"}
        )

    return JSONResponse(status_code=200, content=user.dict())
```

## 타입 시스템

AST 노드에서 사용하는 타입은 다음과 같이 정의된다:

```json
{
  "primitives": ["string", "number", "boolean", "null"],
  "composites": {
    "array": { "items": "<Type>" },
    "object": { "properties": { "<key>": "<Type>" } },
    "map": { "key": "<Type>", "value": "<Type>" },
    "optional": { "inner": "<Type>" },
    "union": { "variants": ["<Type>", "<Type>"] }
  },
  "references": {
    "ref": "<SchemaName>",
    "model": "<ModelName>"
  }
}
```

### 타입 매핑표

| AST 타입 | TypeScript | Rust | Python | Go |
|----------|-----------|------|--------|-----|
| `string` | `string` | `String` | `str` | `string` |
| `number` | `number` | `f64` | `float` | `float64` |
| `integer` | `number` | `i64` | `int` | `int64` |
| `boolean` | `boolean` | `bool` | `bool` | `bool` |
| `null` | `null` | `None` (Option) | `None` | `nil` |
| `array<T>` | `T[]` | `Vec<T>` | `list[T]` | `[]T` |
| `optional<T>` | `T \| null` | `Option<T>` | `Optional[T]` | `*T` |
| `map<K,V>` | `Record<K,V>` | `HashMap<K,V>` | `dict[K,V]` | `map[K]V` |

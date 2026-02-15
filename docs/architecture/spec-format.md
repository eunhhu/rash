# 스펙 파일 포맷

Rash 프로젝트는 디렉토리 기반으로 저장된다. 각 파일은 JSON 형식이며, git으로 추적하기에 최적화되어 있다.

> 문서 상태: **Current (MVP 기준 규칙)** + **Target (최종 비전 확장)** 를 함께 다룬다.

## 프로젝트 디렉토리 구조

```
my-server/
├── rash.config.json              # 프로젝트 설정
├── routes/
│   ├── api/
│   │   └── v1/
│   │       ├── users.route.json          # GET/POST /v1/users (basePath와 결합)
│   │       ├── users/
│   │       │   └── [id].route.json       # GET/PUT/DELETE /v1/users/:id (basePath와 결합)
│   │       ├── posts.route.json
│   │       └── auth/
│   │           ├── login.route.json
│   │           └── register.route.json
│   └── ws/
│       └── chat.route.json               # WebSocket /ws/chat
├── schemas/
│   ├── user.schema.json                  # User DTO 정의
│   ├── post.schema.json
│   └── auth.schema.json
├── models/
│   ├── user.model.json                   # User DB 모델
│   ├── post.model.json
│   └── comment.model.json
├── middleware/
│   ├── auth.middleware.json              # JWT 인증 미들웨어
│   ├── cors.middleware.json
│   └── rate-limit.middleware.json
├── handlers/
│   ├── users.handler.json                # 핸들러 AST (DSL)
│   ├── posts.handler.json
│   └── auth.handler.json
└── .rash/
    ├── cache/                            # 코드 생성 캐시
    └── lock.json                         # 의존성 잠금
```

## rash.config.json

프로젝트의 최상위 설정 파일이다.

```json
{
  "$schema": "https://rash.dev/schemas/config.json",
  "version": "1.0.0",
  "name": "my-server",
  "description": "My awesome server",

  "target": {
    "language": "typescript",
    "framework": "express",
    "runtime": "bun"
  },

  "server": {
    "port": 3000,
    "host": "0.0.0.0",
    "protocol": "http",
    "basePath": "/api"
  },

  "database": {
    "type": "postgresql",
    "orm": "prisma"
  },

  "codegen": {
    "outDir": "./dist",
    "sourceMap": true,
    "strict": true
  },

  "middleware": {
    "global": [
      { "ref": "cors" },
      { "ref": "rate-limit", "config": { "windowMs": 60000, "max": 100 } }
    ]
  },

  "plugins": [],

  "meta": {
    "createdAt": "2026-01-15T00:00:00Z",
    "rashVersion": "0.1.0"
  }
}
```

### 버전/마이그레이션 정책

`rash.config.json.version`은 스펙 저장 포맷 버전이다. `meta.rashVersion`(앱 버전)과 구분한다.

- 앱이 프로젝트를 열 때, 현재 파서가 지원하는 스펙 버전인지 먼저 검사한다.
- 구버전 스펙이면 `MigrationRunner`가 순차적으로 업그레이드를 수행한다.
- 마이그레이션은 항상 `vN -> vN+1` 단위의 작은 단계로만 정의한다.
- 실패 시 파일을 덮어쓰지 않고, `.rash/migrations/<timestamp>/`에 백업 후 중단한다.

예시:
- 지원 범위: `1.x`
- 입력 프로젝트: `0.9.0`
- 실행 순서: `0.9 -> 1.0`
- 결과: 성공 시 `version: "1.0.0"`으로 갱신

```json
{
  "version": "1.0.0",
  "meta": {
    "rashVersion": "0.1.0",
    "lastMigratedFrom": "0.9.0"
  }
}
```

### target 필드 상세

| 필드 | 타입 | 설명 | 허용 값 |
|------|------|------|---------|
| `language` | `string` | 코드 생성 타겟 언어 | `typescript`, `rust`, `python`, `go` |
| `framework` | `string` | 타겟 프레임워크 | 아래 호환표 참고 |
| `runtime` | `string` | 실행 런타임 | `bun`, `node`, `deno`, `cargo`, `python`, `go` |

### 언어-프레임워크 호환표

| 언어 | 프레임워크 |
|------|-----------|
| TypeScript | `express`, `fastify`, `hono`, `elysia`, `nestjs` |
| Rust | `actix`, `axum`, `rocket` |
| Python | `fastapi`, `django`, `flask` |
| Go | `gin`, `echo`, `fiber` |

### `server.basePath` + Route `path` 해석 규칙

경로 중복(`/api/api/...`)을 방지하기 위해 다음 단일 규칙을 사용한다.

- `route.path`는 **basePath를 제외한 상대 경로**를 기본값으로 사용한다.
- 최종 URL은 `finalPath = normalize(server.basePath + route.path)` 로 계산한다.
- `server.basePath`가 비어 있거나 `/`이면 `route.path`를 그대로 사용한다.
- 경로는 항상 `/`로 시작해야 하며, 저장 시 중복 슬래시는 정규화한다.

예시:
- `basePath=/api`, `route.path=/v1/users` → `/api/v1/users`
- `basePath=/api`, `route.path=/v1/users/:id` → `/api/v1/users/:id`
- `basePath=/`, `route.path=/health` → `/health`

## Route 파일 (*.route.json)

라우트 파일은 하나의 URL 경로에 대한 HTTP 메서드별 핸들러를 정의한다.

```json
{
  "$schema": "https://rash.dev/schemas/route.json",
  "path": "/v1/users",
  "description": "사용자 관리 API",

  "methods": {
    "GET": {
      "operationId": "listUsers",
      "summary": "사용자 목록 조회",
      "handler": { "ref": "users.listUsers" },
      "middleware": [
        { "ref": "auth", "config": { "roles": ["admin", "user"] } }
      ],
      "request": {
        "query": {
          "ref": "ListUsersQuery"
        }
      },
      "response": {
        "200": {
          "description": "성공",
          "schema": { "ref": "UserListResponse" }
        },
        "401": {
          "description": "인증 실패",
          "schema": { "ref": "ErrorResponse" }
        }
      }
    },

    "POST": {
      "operationId": "createUser",
      "summary": "사용자 생성",
      "handler": { "ref": "users.createUser" },
      "middleware": [
        { "ref": "auth", "config": { "roles": ["admin"] } },
        { "ref": "validate", "config": { "schema": "CreateUserBody" } }
      ],
      "request": {
        "body": {
          "ref": "CreateUserBody",
          "contentType": "application/json"
        }
      },
      "response": {
        "201": {
          "description": "생성 완료",
          "schema": { "ref": "UserResponse" }
        },
        "400": {
          "description": "유효성 검사 실패",
          "schema": { "ref": "ValidationErrorResponse" }
        }
      }
    }
  },

  "tags": ["users"],
  "meta": {
    "createdAt": "2026-01-15T00:00:00Z",
    "updatedAt": "2026-01-20T00:00:00Z"
  }
}
```

### 동적 경로 파라미터

파일명에 `[param]` 표기법을 사용한다.

- `users/[id].route.json` → `/v1/users/:id` (basePath와 결합 시 `/api/v1/users/:id`)
- `posts/[postId]/comments/[commentId].route.json` → `/posts/:postId/comments/:commentId`

```json
{
  "path": "/v1/users/:id",
  "params": {
    "id": {
      "type": "string",
      "format": "uuid",
      "description": "사용자 UUID"
    }
  },
  "methods": {
    "GET": {
      "operationId": "getUser",
      "handler": { "ref": "users.getUser" },
      "response": {
        "200": { "schema": { "ref": "UserResponse" } },
        "404": { "schema": { "ref": "ErrorResponse" } }
      }
    }
  }
}
```

## Schema 파일 (*.schema.json)

JSON Schema 기반의 DTO 정의 파일이다. 이 스키마에서 Zod, TypeScript interface, Rust struct, Python dataclass 등으로 양방향 변환된다.

```json
{
  "$schema": "https://rash.dev/schemas/schema.json",
  "name": "User",
  "description": "사용자 정보",

  "definitions": {
    "CreateUserBody": {
      "type": "object",
      "required": ["email", "password", "name"],
      "properties": {
        "email": {
          "type": "string",
          "format": "email",
          "maxLength": 255,
          "description": "이메일 주소"
        },
        "password": {
          "type": "string",
          "minLength": 8,
          "maxLength": 128,
          "description": "비밀번호"
        },
        "name": {
          "type": "string",
          "minLength": 1,
          "maxLength": 100,
          "description": "사용자 이름"
        },
        "role": {
          "type": "string",
          "enum": ["admin", "user", "moderator"],
          "default": "user",
          "description": "사용자 역할"
        }
      }
    },

    "UserResponse": {
      "type": "object",
      "properties": {
        "id": { "type": "string", "format": "uuid" },
        "email": { "type": "string", "format": "email" },
        "name": { "type": "string" },
        "role": { "type": "string", "enum": ["admin", "user", "moderator"] },
        "createdAt": { "type": "string", "format": "date-time" },
        "updatedAt": { "type": "string", "format": "date-time" }
      }
    },

    "UserListResponse": {
      "type": "object",
      "properties": {
        "data": {
          "type": "array",
          "items": { "$ref": "#/definitions/UserResponse" }
        },
        "total": { "type": "integer" },
        "page": { "type": "integer" },
        "limit": { "type": "integer" }
      }
    },

    "ListUsersQuery": {
      "type": "object",
      "properties": {
        "page": { "type": "integer", "minimum": 1, "default": 1 },
        "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 },
        "sort": { "type": "string", "enum": ["createdAt", "name", "email"] },
        "order": { "type": "string", "enum": ["asc", "desc"], "default": "desc" },
        "search": { "type": "string" }
      }
    },

    "ErrorResponse": {
      "type": "object",
      "required": ["message", "code"],
      "properties": {
        "message": { "type": "string" },
        "code": { "type": "string" },
        "details": { "type": "object" }
      }
    }
  }
}
```

### 스키마 ↔ 코드 변환 예시

위 `CreateUserBody` 스키마에서 생성되는 코드:

**TypeScript (Zod)**
```typescript
export const CreateUserBody = z.object({
  email: z.string().email().max(255),
  password: z.string().min(8).max(128),
  name: z.string().min(1).max(100),
  role: z.enum(["admin", "user", "moderator"]).default("user"),
});
export type CreateUserBody = z.infer<typeof CreateUserBody>;
```

**Rust (serde)**
```rust
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateUserBody {
    #[validate(email)]
    #[validate(length(max = 255))]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[serde(default = "default_role")]
    pub role: UserRole,
}
```

**Python (Pydantic)**
```python
class CreateUserBody(BaseModel):
    email: EmailStr = Field(max_length=255)
    password: str = Field(min_length=8, max_length=128)
    name: str = Field(min_length=1, max_length=100)
    role: Literal["admin", "user", "moderator"] = "user"
```

## Model 파일 (*.model.json)

데이터베이스 테이블/컬렉션 정의이다. 스키마(DTO)와는 별도로 DB 레벨의 모델을 정의한다.

```json
{
  "$schema": "https://rash.dev/schemas/model.json",
  "name": "User",
  "description": "사용자 테이블",
  "tableName": "users",

  "columns": {
    "id": {
      "type": "uuid",
      "primaryKey": true,
      "default": "gen_random_uuid()"
    },
    "email": {
      "type": "varchar(255)",
      "unique": true,
      "nullable": false,
      "index": true
    },
    "passwordHash": {
      "type": "varchar(255)",
      "nullable": false
    },
    "name": {
      "type": "varchar(100)",
      "nullable": false
    },
    "role": {
      "type": "enum",
      "values": ["admin", "user", "moderator"],
      "default": "user",
      "nullable": false
    },
    "createdAt": {
      "type": "timestamp",
      "default": "now()",
      "nullable": false
    },
    "updatedAt": {
      "type": "timestamp",
      "default": "now()",
      "nullable": false,
      "onUpdate": "now()"
    },
    "deletedAt": {
      "type": "timestamp",
      "nullable": true
    }
  },

  "relations": {
    "posts": {
      "type": "hasMany",
      "target": "Post",
      "foreignKey": "authorId"
    },
    "profile": {
      "type": "hasOne",
      "target": "Profile",
      "foreignKey": "userId"
    }
  },

  "indexes": [
    { "columns": ["email"], "unique": true },
    { "columns": ["role", "createdAt"] },
    { "columns": ["deletedAt"], "where": "deletedAt IS NULL" }
  ],

  "hooks": {
    "beforeInsert": { "ref": "users.hashPassword" },
    "beforeUpdate": { "ref": "users.updateTimestamp" }
  }
}
```

## Middleware 파일 (*.middleware.json)

미들웨어 정의 파일이다. 미들웨어의 설정 스키마와 핸들러를 정의한다.

```json
{
  "$schema": "https://rash.dev/schemas/middleware.json",
  "name": "auth",
  "description": "JWT 인증 미들웨어",
  "type": "request",

  "config": {
    "type": "object",
    "properties": {
      "roles": {
        "type": "array",
        "items": { "type": "string" },
        "description": "허용되는 역할 목록. 비어있으면 인증만 확인."
      },
      "optional": {
        "type": "boolean",
        "default": false,
        "description": "true면 토큰 없어도 통과 (인증 정보만 추출)"
      }
    }
  },

  "handler": { "ref": "auth.verifyToken" },

  "provides": {
    "user": {
      "type": "object",
      "properties": {
        "id": { "type": "string" },
        "email": { "type": "string" },
        "role": { "type": "string" }
      },
      "description": "인증된 사용자 정보. 후속 핸들러에서 ctx.user로 접근."
    }
  },

  "errors": {
    "UNAUTHORIZED": {
      "status": 401,
      "message": "유효하지 않은 토큰"
    },
    "FORBIDDEN": {
      "status": 403,
      "message": "권한 없음"
    }
  }
}
```

## 참조 규칙

스펙 파일 간 참조는 `{ "ref": "..." }` 형태로 한다.

| 참조 대상 | 형식 | 예시 |
|-----------|------|------|
| 스키마 정의 | `SchemaName` | `{ "ref": "UserResponse" }` |
| 핸들러 함수 | `handler.function` | `{ "ref": "users.createUser" }` |
| 미들웨어 | `middlewareName` | `{ "ref": "auth" }` |
| 모델 | `ModelName` | `{ "ref": "User" }` |
| 외부 스키마 | `file#definition` | `{ "ref": "common.schema#Pagination" }` |

### Resolver 결정 규칙

Resolver는 동일 입력에 대해 항상 동일 결과를 반환해야 한다.

1. **타입 판별**: 참조가 쓰인 필드 문맥으로 대상 타입(스키마/핸들러/미들웨어/모델)을 먼저 고정한다.
2. **로컬 우선**: 동일 타입에서 현재 프로젝트 루트의 인덱스(`SpecIndex`)를 먼저 탐색한다.
3. **정규화 키 사용**: 대소문자/구분자 차이를 정규화한 canonical key로 조회한다.
4. **외부 참조 분리**: `file#definition`은 파일 로드 후 로컬 네임스페이스와 충돌 없이 별도 해석한다.
5. **모호성 금지**: 후보가 2개 이상이면 자동 선택하지 않고 `E_REF_AMBIGUOUS`를 반환한다.

에러 정책:
- 미존재 참조: `E_REF_NOT_FOUND`
- 타입 불일치 참조: `E_REF_TYPE_MISMATCH`
- 순환 참조: `E_REF_CYCLE`
- 중복 정의: `E_DUPLICATE_SYMBOL`

### 검증 스키마/에러 포맷 표준

검증기는 JSON Schema 오류와 도메인 규칙 오류를 동일 포맷으로 반환한다.

```json
{
  "ok": false,
  "errors": [
    {
      "code": "E_REF_NOT_FOUND",
      "severity": "error",
      "message": "Referenced schema 'UserResponse' was not found",
      "file": "routes/api/v1/users.route.json",
      "path": "$.methods.GET.response.200.schema.ref",
      "suggestion": "Create schema 'UserResponse' or fix the ref name"
    }
  ]
}
```

필수 필드:
- `code`: 기계가 처리할 수 있는 안정적 오류 코드
- `severity`: `error | warning | info`
- `file`: 상대 경로
- `path`: JSONPath
- `message`: 사용자 표시용 문장
- `suggestion`: 자동 수정/가이드 문구

## 파일 네이밍 규칙

| 파일 종류 | 네이밍 | 예시 |
|-----------|--------|------|
| 라우트 | `kebab-case.route.json` | `user-profile.route.json` |
| 스키마 | `kebab-case.schema.json` | `user.schema.json` |
| 모델 | `kebab-case.model.json` | `user.model.json` |
| 미들웨어 | `kebab-case.middleware.json` | `rate-limit.middleware.json` |
| 핸들러 | `kebab-case.handler.json` | `users.handler.json` |
| 동적 파라미터 | `[paramName].route.json` | `[id].route.json` |

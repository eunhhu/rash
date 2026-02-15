# 고급 기능: 컴포넌트화, 상태 시스템, 추상화

GUI 기반 서버 빌더가 코드 작성의 자율성을 대체하려면, 코드에서 자연스럽게 사용하는 **재사용**, **합성**, **상태 관리**, **추상화** 기능을 동등한 수준으로 제공해야 한다.

이 문서는 기존 AST-DSL(핸들러 단위의 imperative 로직)을 기반으로, 그 위에 쌓이는 프로젝트 레벨 기능들을 설계한다.

> 문서 상태: **Design (설계 논의 단계)** — 구현 전 합의용

---

## 1. 문제 정의

코드로 서버를 작성할 때 자연스럽게 하는 것들:

```typescript
// 1. 재사용 가능한 로직 추출
function paginate<T>(query: Query<T>, page: number, limit: number) { ... }

// 2. 합성 (composition)
const adminOnly = compose(authenticate, requireRole("admin"));

// 3. 상태/변수 스코프
const rateLimitStore = new Map<string, number>();  // 모듈 레벨
const config = env.get("MAX_RETRIES");             // 환경 변수
const cached = computeOnce(() => loadConfig());    // 계산된 값

// 4. 깊은 추상화
interface Repository<T> { findById(id: string): Promise<T>; ... }
class UserRepository implements Repository<User> { ... }
```

이것들이 GUI에서 불가능하면, 복잡한 서버를 만들 때 결국 코드로 돌아가게 된다.

---

## 2. 컴포넌트 시스템

### 2.1 개념

**컴포넌트(Component)** = 재사용 가능한 로직 블록. 핸들러, 미들웨어, 검증 로직, DB 쿼리 패턴 등 모든 종류의 로직을 컴포넌트로 만들 수 있다.

기존 `*.handler.json`이 "함수"라면, 컴포넌트는 **"재사용 가능한 함수"** 이다.

### 2.2 컴포넌트 파일 (*.component.json)

```
my-server/
├── components/
│   ├── pagination.component.json      # 페이지네이션 로직
│   ├── soft-delete.component.json     # 소프트 삭제 패턴
│   ├── audit-log.component.json       # 감사 로그 기록
│   └── cache-aside.component.json     # 캐시 어사이드 패턴
```

```json
{
  "$schema": "https://rash.dev/schemas/component.json",
  "name": "paginate",
  "description": "컬렉션에 페이지네이션을 적용하는 범용 컴포넌트",
  "version": "1.0.0",
  "category": "query",

  "typeParams": ["T"],

  "inputs": {
    "model": {
      "type": "string",
      "description": "대상 모델명"
    },
    "where": {
      "type": "object",
      "description": "필터 조건 (optional)",
      "optional": true
    },
    "orderBy": {
      "type": "object",
      "description": "정렬 기준 (optional)",
      "optional": true
    }
  },

  "contextInputs": {
    "page": {
      "source": "query.page",
      "type": "integer",
      "default": 1
    },
    "limit": {
      "source": "query.limit",
      "type": "integer",
      "default": 20
    }
  },

  "outputs": {
    "data": { "type": "array", "items": { "typeParam": "T" } },
    "total": { "type": "integer" },
    "page": { "type": "integer" },
    "limit": { "type": "integer" },
    "totalPages": { "type": "integer" }
  },

  "body": [
    {
      "type": "LetStatement",
      "tier": 0,
      "name": "skip",
      "value": {
        "type": "BinaryExpr",
        "tier": 0,
        "operator": "*",
        "left": {
          "type": "BinaryExpr",
          "tier": 0,
          "operator": "-",
          "left": { "type": "Identifier", "tier": 0, "name": "page" },
          "right": { "type": "Literal", "tier": 0, "value": 1 }
        },
        "right": { "type": "Identifier", "tier": 0, "name": "limit" }
      }
    },
    {
      "type": "LetStatement",
      "tier": 0,
      "name": "total",
      "value": {
        "type": "AwaitExpr",
        "tier": 1,
        "expr": {
          "type": "DbQuery",
          "tier": 1,
          "model": { "type": "Identifier", "tier": 0, "name": "$input.model" },
          "operation": "count",
          "where": { "type": "Identifier", "tier": 0, "name": "$input.where" }
        }
      }
    },
    {
      "type": "LetStatement",
      "tier": 0,
      "name": "data",
      "value": {
        "type": "AwaitExpr",
        "tier": 1,
        "expr": {
          "type": "DbQuery",
          "tier": 1,
          "model": { "type": "Identifier", "tier": 0, "name": "$input.model" },
          "operation": "findMany",
          "where": { "type": "Identifier", "tier": 0, "name": "$input.where" },
          "orderBy": { "type": "Identifier", "tier": 0, "name": "$input.orderBy" },
          "skip": { "type": "Identifier", "tier": 0, "name": "skip" },
          "take": { "type": "Identifier", "tier": 0, "name": "limit" }
        }
      }
    },
    {
      "type": "ReturnStatement",
      "tier": 0,
      "value": {
        "type": "ObjectExpr",
        "tier": 0,
        "properties": {
          "data": { "type": "Identifier", "tier": 0, "name": "data" },
          "total": { "type": "Identifier", "tier": 0, "name": "total" },
          "page": { "type": "Identifier", "tier": 0, "name": "page" },
          "limit": { "type": "Identifier", "tier": 0, "name": "limit" },
          "totalPages": {
            "type": "CallExpr",
            "tier": 0,
            "callee": { "type": "Identifier", "tier": 0, "name": "Math.ceil" },
            "args": [
              {
                "type": "BinaryExpr",
                "tier": 0,
                "operator": "/",
                "left": { "type": "Identifier", "tier": 0, "name": "total" },
                "right": { "type": "Identifier", "tier": 0, "name": "limit" }
              }
            ]
          }
        }
      }
    }
  ],

  "meta": {
    "maxTier": 1,
    "bridges": []
  }
}
```

### 2.3 컴포넌트 호출 (UseComponent 노드)

핸들러에서 컴포넌트를 사용할 때 새로운 AST 노드를 사용한다.

```json
{
  "type": "UseComponent",
  "tier": 0,
  "ref": "paginate",
  "typeArgs": ["User"],
  "inputs": {
    "model": { "type": "Literal", "tier": 0, "value": "User" },
    "where": {
      "type": "ObjectExpr",
      "tier": 0,
      "properties": {
        "role": { "type": "Identifier", "tier": 0, "name": "roleFilter" }
      }
    },
    "orderBy": {
      "type": "ObjectExpr",
      "tier": 0,
      "properties": {
        "createdAt": { "type": "Literal", "tier": 0, "value": "desc" }
      }
    }
  },
  "bind": "result"
}
```

**코드 생성 결과** (TypeScript):

```typescript
// 인라인 전개 방식 (기본)
const skip = (page - 1) * limit;
const total = await prisma.user.count({ where: { role: roleFilter } });
const data = await prisma.user.findMany({
  where: { role: roleFilter },
  orderBy: { createdAt: "desc" },
  skip,
  take: limit,
});
const result = { data, total, page, limit, totalPages: Math.ceil(total / limit) };
```

```typescript
// 함수 추출 방식 (codegen 옵션)
const result = await paginate<User>({
  model: "User",
  where: { role: roleFilter },
  orderBy: { createdAt: "desc" },
  page,
  limit,
});
```

### 2.4 코드 생성 전략

| 전략 | 설명 | 장점 | 단점 |
|------|------|------|------|
| **inline** (기본) | 컴포넌트 body를 호출 위치에 전개 | 오버헤드 없음, 디버깅 용이 | 코드 중복 |
| **extract** | 별도 함수로 추출, 호출로 대체 | DRY, 읽기 좋음 | 함수 호출 오버헤드 |
| **module** | 별도 파일로 분리 후 import | 모듈 분리, 테스트 용이 | import 관리 필요 |

`rash.config.json`에서 전역 또는 컴포넌트별로 전략을 설정:

```json
{
  "codegen": {
    "componentStrategy": "extract",
    "componentOverrides": {
      "paginate": "module",
      "audit-log": "inline"
    }
  }
}
```

---

## 3. 컴포지션 (합성)

### 3.1 미들웨어 컴포지션

여러 미들웨어를 하나의 단위로 합성한다.

```json
{
  "$schema": "https://rash.dev/schemas/middleware.json",
  "name": "admin-only",
  "description": "인증 + 관리자 권한 확인",
  "type": "composed",

  "compose": [
    { "ref": "auth" },
    { "ref": "require-role", "config": { "roles": ["admin"] } },
    { "ref": "audit-log", "config": { "action": "admin-access" } }
  ],

  "shortCircuit": true
}
```

`shortCircuit: true` — 체인 중 하나라도 실패하면 나머지를 실행하지 않음.

**코드 생성 결과** (Express):

```typescript
export const adminOnly = [
  authMiddleware,
  requireRole({ roles: ["admin"] }),
  auditLog({ action: "admin-access" }),
];

// 라우트에서 사용
router.delete("/users/:id", ...adminOnly, deleteUserHandler);
```

### 3.2 핸들러 컴포지션 (Pipeline)

핸들러 로직을 파이프라인으로 합성한다. 각 단계가 이전 단계의 출력을 입력으로 받는다.

```json
{
  "$schema": "https://rash.dev/schemas/handler.json",
  "name": "createUser",
  "async": true,
  "type": "pipeline",

  "pipeline": [
    {
      "step": "validate",
      "use": { "ref": "validate-body", "config": { "schema": "CreateUserBody" } },
      "bind": "validated"
    },
    {
      "step": "transform",
      "use": { "ref": "hash-field", "config": { "field": "password", "algorithm": "bcrypt" } },
      "input": { "type": "Identifier", "tier": 0, "name": "validated" },
      "bind": "transformed"
    },
    {
      "step": "persist",
      "use": { "ref": "db-insert", "config": { "model": "User" } },
      "input": { "type": "Identifier", "tier": 0, "name": "transformed" },
      "bind": "user"
    },
    {
      "step": "respond",
      "body": [
        {
          "type": "ReturnStatement",
          "tier": 0,
          "value": {
            "type": "HttpRespond",
            "tier": 1,
            "status": 201,
            "body": { "type": "Identifier", "tier": 0, "name": "user" }
          }
        }
      ]
    }
  ]
}
```

핸들러 `type` 필드:
- `"imperative"` (기본, 기존 body 배열 방식)
- `"pipeline"` (단계별 합성)

### 3.3 컴포넌트 오버라이드

컴포넌트를 기반으로 하되 일부를 재정의한다.

```json
{
  "$schema": "https://rash.dev/schemas/component.json",
  "name": "paginate-with-search",
  "extends": "paginate",

  "inputs": {
    "searchFields": {
      "type": "array",
      "items": { "type": "string" },
      "description": "검색 대상 필드 목록"
    }
  },

  "contextInputs": {
    "search": {
      "source": "query.search",
      "type": "string",
      "optional": true
    }
  },

  "override": {
    "where": {
      "description": "기존 where에 검색 조건을 병합",
      "strategy": "merge",
      "additions": {
        "OR": {
          "type": "ConditionalExpr",
          "tier": 0,
          "condition": {
            "type": "BinaryExpr",
            "tier": 0,
            "operator": "!=",
            "left": { "type": "Identifier", "tier": 0, "name": "search" },
            "right": { "type": "Literal", "tier": 0, "value": null }
          },
          "then": {
            "type": "CallExpr",
            "tier": 0,
            "callee": { "type": "Identifier", "tier": 0, "name": "$buildSearchFilter" },
            "args": [
              { "type": "Identifier", "tier": 0, "name": "search" },
              { "type": "Identifier", "tier": 0, "name": "$input.searchFields" }
            ]
          },
          "else": { "type": "Literal", "tier": 0, "value": null }
        }
      }
    }
  }
}
```

---

## 4. 변수 및 상태 시스템

### 4.1 스코프 계층

```
┌─────────────────────────────────────────────────┐
│ Environment (환경 변수)                          │
│   DATABASE_URL, JWT_SECRET, PORT                │
│                                                  │
│  ┌───────────────────────────────────────────┐  │
│  │ Project (프로젝트 레벨 상수/설정)          │  │
│  │   MAX_PAGE_SIZE=100, DEFAULT_LOCALE="ko"   │  │
│  │                                            │  │
│  │  ┌─────────────────────────────────────┐   │  │
│  │  │ Module (모듈/파일 레벨)              │   │  │
│  │  │   rateLimitStore, cachedConfig       │   │  │
│  │  │                                      │   │  │
│  │  │  ┌───────────────────────────────┐   │   │  │
│  │  │  │ Handler (핸들러 로컬)          │   │   │  │
│  │  │  │   userId, user, result         │   │   │  │
│  │  │  └───────────────────────────────┘   │   │  │
│  │  └─────────────────────────────────────┘   │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

### 4.2 프로젝트 변수 (Project Variables)

`rash.config.json`에 정의하는 프로젝트 레벨 상수:

```json
{
  "variables": {
    "MAX_PAGE_SIZE": {
      "type": "integer",
      "value": 100,
      "description": "페이지네이션 최대 크기"
    },
    "DEFAULT_LOCALE": {
      "type": "string",
      "value": "ko",
      "description": "기본 로케일"
    },
    "SUPPORTED_ROLES": {
      "type": "array",
      "value": ["admin", "user", "moderator"],
      "description": "시스템에서 허용하는 역할 목록"
    },
    "BCRYPT_ROUNDS": {
      "type": "integer",
      "value": 10,
      "envOverride": "BCRYPT_ROUNDS",
      "description": "bcrypt 해싱 라운드 수. 환경 변수로 재정의 가능."
    }
  }
}
```

AST에서 참조할 때:

```json
{
  "type": "Identifier",
  "tier": 0,
  "name": "$project.MAX_PAGE_SIZE"
}
```

코드 생성:

```typescript
// TypeScript
const MAX_PAGE_SIZE = 100;
// 또는 envOverride가 있으면:
const BCRYPT_ROUNDS = parseInt(process.env.BCRYPT_ROUNDS ?? "10");
```

### 4.3 Computed State (계산된 상태)

값이 다른 변수나 표현식에서 파생되는 상태:

```json
{
  "$schema": "https://rash.dev/schemas/state.json",
  "name": "app-state",
  "scope": "module",

  "definitions": {
    "jwtSecret": {
      "type": "string",
      "compute": {
        "type": "EnvGet",
        "tier": 0,
        "key": "JWT_SECRET",
        "required": true
      },
      "lifecycle": "init"
    },

    "dbPool": {
      "type": "object",
      "compute": {
        "type": "CallExpr",
        "tier": 1,
        "callee": { "type": "Identifier", "tier": 0, "name": "createDbPool" },
        "args": [
          { "type": "EnvGet", "tier": 0, "key": "DATABASE_URL", "required": true },
          { "type": "ObjectExpr", "tier": 0, "properties": {
            "maxConnections": { "type": "Literal", "tier": 0, "value": 10 },
            "idleTimeout": { "type": "Literal", "tier": 0, "value": 30000 }
          }}
        ]
      },
      "lifecycle": "init",
      "cleanup": {
        "type": "CallExpr",
        "tier": 0,
        "callee": { "type": "MemberExpr", "tier": 0,
          "object": { "type": "Identifier", "tier": 0, "name": "dbPool" },
          "property": "disconnect"
        },
        "args": []
      }
    },

    "activeUserCount": {
      "type": "integer",
      "compute": {
        "type": "AwaitExpr",
        "tier": 1,
        "expr": {
          "type": "DbQuery",
          "tier": 1,
          "model": "User",
          "operation": "count",
          "where": { "active": true }
        }
      },
      "lifecycle": "lazy",
      "ttl": 60000,
      "description": "활성 사용자 수. 60초 TTL 캐시."
    }
  }
}
```

### 4.4 Lifecycle 종류

| Lifecycle | 시점 | 재계산 | 용도 |
|-----------|------|--------|------|
| `init` | 서버 시작 시 1회 | 재시작 시 | DB pool, config 로드 |
| `lazy` | 첫 접근 시 계산 | TTL 만료 시 | 캐시, 계산 비용 큰 값 |
| `request` | 요청마다 | 매 요청 | 요청별 컨텍스트 데이터 |
| `reactive` | 의존 값 변경 시 | 자동 | 다른 상태에서 파생된 값 |

코드 생성 예시:

```typescript
// init lifecycle
let dbPool: Pool;
async function initState() {
  dbPool = await createDbPool(process.env.DATABASE_URL!, {
    maxConnections: 10,
    idleTimeout: 30000,
  });
}

// lazy + TTL lifecycle
let _activeUserCount: number | undefined;
let _activeUserCountTimestamp = 0;
async function getActiveUserCount(): Promise<number> {
  if (_activeUserCount === undefined || Date.now() - _activeUserCountTimestamp > 60000) {
    _activeUserCount = await prisma.user.count({ where: { active: true } });
    _activeUserCountTimestamp = Date.now();
  }
  return _activeUserCount;
}
```

### 4.5 Reactive State

상태 간 의존 관계를 선언적으로 정의:

```json
{
  "rateLimitConfig": {
    "type": "object",
    "compute": {
      "type": "ObjectExpr",
      "tier": 0,
      "properties": {
        "windowMs": { "type": "Literal", "tier": 0, "value": 60000 },
        "max": { "type": "Identifier", "tier": 0, "name": "$project.RATE_LIMIT_MAX" }
      }
    },
    "lifecycle": "reactive",
    "dependsOn": ["$project.RATE_LIMIT_MAX"]
  }
}
```

`reactive` lifecycle은 **GUI 내에서 변수 값을 변경했을 때 의존 트리를 따라 자동으로 갱신**하는 것이 주 목적이다. 런타임에서는 `init`처럼 1회 계산된다 (서버 환경에서 실시간 reactive는 과도).

---

## 5. 사용자 정의 함수 및 추상화

### 5.1 사용자 정의 함수 (UserFunction)

컴포넌트보다 가벼운, 순수 함수 수준의 재사용 단위:

```json
{
  "$schema": "https://rash.dev/schemas/function.json",
  "name": "formatCurrency",
  "description": "숫자를 통화 형식 문자열로 변환",
  "pure": true,

  "params": [
    { "name": "amount", "type": "number" },
    { "name": "currency", "type": "string", "default": "KRW" }
  ],

  "returnType": "string",

  "body": [
    {
      "type": "ReturnStatement",
      "tier": 0,
      "value": {
        "type": "CallExpr",
        "tier": 2,
        "callee": {
          "type": "MemberExpr",
          "tier": 0,
          "object": {
            "type": "CallExpr",
            "tier": 2,
            "callee": { "type": "Identifier", "tier": 2, "name": "Intl.NumberFormat" },
            "args": [
              { "type": "Literal", "tier": 0, "value": "ko-KR" },
              {
                "type": "ObjectExpr",
                "tier": 0,
                "properties": {
                  "style": { "type": "Literal", "tier": 0, "value": "currency" },
                  "currency": { "type": "Identifier", "tier": 0, "name": "currency" }
                }
              }
            ]
          },
          "property": "format"
        },
        "args": [
          { "type": "Identifier", "tier": 0, "name": "amount" }
        ]
      }
    }
  ],

  "meta": {
    "maxTier": 2
  }
}
```

### 5.2 함수 vs 컴포넌트 구분

| | UserFunction | Component |
|---|---|---|
| 파일 | `*.function.json` | `*.component.json` |
| 목적 | 순수 데이터 변환 | 부수 효과 포함 로직 (DB, HTTP, etc.) |
| Tier 범위 | 주로 0~2 | 0~3 |
| async | 불가 | 가능 |
| contextInputs | 없음 | 있음 (요청 컨텍스트에서 자동 주입) |
| 타입 파라미터 | 가능 | 가능 |
| 코드 생성 | 항상 함수로 추출 | inline/extract/module 선택 |

### 5.3 제네릭 패턴

컴포넌트와 함수 모두 타입 파라미터를 가질 수 있다:

```json
{
  "name": "findOrFail",
  "typeParams": ["T"],

  "inputs": {
    "model": { "type": "string" },
    "where": { "type": "object" }
  },

  "outputs": {
    "type": { "typeParam": "T" }
  },

  "body": [
    {
      "type": "LetStatement",
      "tier": 0,
      "name": "result",
      "value": {
        "type": "AwaitExpr",
        "tier": 1,
        "expr": {
          "type": "DbQuery",
          "tier": 1,
          "model": { "type": "Identifier", "tier": 0, "name": "$input.model" },
          "operation": "findUnique",
          "where": { "type": "Identifier", "tier": 0, "name": "$input.where" }
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
        "left": { "type": "Identifier", "tier": 0, "name": "result" },
        "right": { "type": "Literal", "tier": 0, "value": null }
      },
      "then": [
        {
          "type": "ThrowStatement",
          "tier": 0,
          "value": {
            "type": "CallExpr",
            "tier": 1,
            "callee": { "type": "Identifier", "tier": 1, "name": "NotFoundError" },
            "args": [
              { "type": "Identifier", "tier": 0, "name": "$input.model" }
            ]
          }
        }
      ],
      "else": null
    },
    {
      "type": "ReturnStatement",
      "tier": 0,
      "value": { "type": "Identifier", "tier": 0, "name": "result" }
    }
  ]
}
```

### 5.4 인터페이스/계약 (Contract)

컴포넌트의 입출력 계약을 별도로 정의하여 교체 가능한 구현을 만든다:

```json
{
  "$schema": "https://rash.dev/schemas/contract.json",
  "name": "Repository",
  "description": "데이터 접근 계층 계약",
  "typeParams": ["T"],

  "methods": {
    "findById": {
      "params": [{ "name": "id", "type": "string" }],
      "returnType": { "typeParam": "T", "nullable": true },
      "async": true
    },
    "findMany": {
      "params": [
        { "name": "where", "type": "object", "optional": true },
        { "name": "orderBy", "type": "object", "optional": true },
        { "name": "skip", "type": "integer", "optional": true },
        { "name": "take", "type": "integer", "optional": true }
      ],
      "returnType": { "type": "array", "items": { "typeParam": "T" } },
      "async": true
    },
    "create": {
      "params": [{ "name": "data", "type": { "typeParam": "T" } }],
      "returnType": { "typeParam": "T" },
      "async": true
    },
    "update": {
      "params": [
        { "name": "id", "type": "string" },
        { "name": "data", "type": "object" }
      ],
      "returnType": { "typeParam": "T" },
      "async": true
    },
    "delete": {
      "params": [{ "name": "id", "type": "string" }],
      "returnType": "boolean",
      "async": true
    }
  }
}
```

구현:

```json
{
  "$schema": "https://rash.dev/schemas/component.json",
  "name": "PrismaRepository",
  "implements": "Repository",
  "typeParams": ["T"],

  "inputs": {
    "model": { "type": "string" }
  },

  "methods": {
    "findById": {
      "body": [
        {
          "type": "ReturnStatement",
          "tier": 0,
          "value": {
            "type": "AwaitExpr",
            "tier": 1,
            "expr": {
              "type": "DbQuery",
              "tier": 1,
              "model": { "type": "Identifier", "tier": 0, "name": "$input.model" },
              "operation": "findUnique",
              "where": { "id": { "type": "Identifier", "tier": 0, "name": "id" } }
            }
          }
        }
      ]
    }
  }
}
```

핸들러에서 계약을 통해 참조:

```json
{
  "type": "LetStatement",
  "tier": 0,
  "name": "userRepo",
  "value": {
    "type": "UseComponent",
    "tier": 0,
    "ref": "PrismaRepository",
    "typeArgs": ["User"],
    "inputs": { "model": { "type": "Literal", "tier": 0, "value": "User" } }
  }
}
```

---

## 6. AST 확장 노드 요약

기존 AST 노드 계층에 추가되는 노드:

```
AstNode
├── Statement
│   └── (기존 유지)
│
├── Expression
│   ├── (기존 유지)
│   ├── UseComponent        # 컴포넌트 호출 (NEW)
│   ├── CallUserFunction    # 사용자 함수 호출 (NEW)
│   ├── StateGet            # 상태 변수 읽기 (NEW)
│   ├── EnvGet              # 환경 변수 읽기 (NEW)
│   ├── ProjectVar          # 프로젝트 변수 참조 (NEW)
│   └── ConditionalExpr     # 삼항 조건식 (NEW)
│
├── DomainNode
│   └── (기존 유지)
│
└── BridgeNode
    └── (기존 유지)
```

### 새 노드 타입별 Tier

| 노드 | Tier | 설명 |
|------|------|------|
| `UseComponent` | 0 | 컴포넌트 자체는 언어 독립, body의 maxTier가 실질 tier |
| `CallUserFunction` | 0 | 함수 호출 자체는 범용 |
| `StateGet` | 0 | 변수 참조 |
| `EnvGet` | 0 | `process.env` / `os.environ` 등으로 매핑 |
| `ProjectVar` | 0 | 상수 또는 envOverride 패턴으로 매핑 |
| `ConditionalExpr` | 0 | 삼항 연산자 |

---

## 7. GUI 통합

### 7.1 컴포넌트 팔레트

GUI에서 컴포넌트를 드래그&드롭으로 핸들러에 삽입하는 팔레트:

```
┌─────────────────────────────────────┐
│ Component Palette                    │
│                                      │
│ 📂 Query                            │
│   ├── paginate                       │
│   ├── paginate-with-search          │
│   └── find-or-fail                   │
│                                      │
│ 📂 Auth                             │
│   ├── hash-password                  │
│   └── verify-token                   │
│                                      │
│ 📂 Transform                        │
│   ├── soft-delete                    │
│   ├── audit-log                      │
│   └── format-response                │
│                                      │
│ 📂 Custom Functions                  │
│   ├── formatCurrency                 │
│   └── calculateTax                   │
│                                      │
│ [+ New Component]                    │
└─────────────────────────────────────┘
```

### 7.2 변수 탐색기

스코프별 변수를 트리 뷰로 표시:

```
┌─────────────────────────────────────┐
│ Variables Explorer                   │
│                                      │
│ 🌍 Environment                      │
│   ├── DATABASE_URL     ●            │
│   ├── JWT_SECRET       ●            │
│   └── PORT             3000         │
│                                      │
│ 📦 Project                          │
│   ├── MAX_PAGE_SIZE    100          │
│   ├── DEFAULT_LOCALE   "ko"         │
│   └── BCRYPT_ROUNDS    10 (env↑)   │
│                                      │
│ 📄 Module: users                    │
│   └── userRepo         Repository   │
│                                      │
│ 🔧 Handler: getUser                │
│   ├── userId           string       │
│   ├── user             User | null  │
│   └── result           PaginateResult│
│                                      │
│ [+ New Variable]                     │
└─────────────────────────────────────┘
```

`●` = 민감 정보 (값 마스킹)

### 7.3 파이프라인 에디터

핸들러를 파이프라인으로 구성할 때의 시각적 에디터:

```
┌─────────────────────────────────────────────┐
│ Pipeline: createUser                         │
│                                              │
│  ┌──────────┐    ┌──────────┐    ┌────────┐│
│  │ validate │───▶│ transform│───▶│ persist││
│  │ body     │    │ hash pw  │    │ db save││
│  └──────────┘    └──────────┘    └────────┘│
│       │               │              │      │
│       ▼               ▼              ▼      │
│   validated       transformed       user    │
│                                              │
│  ──────────────────────────────────────────  │
│                                              │
│  ┌──────────┐                               │
│  │ respond  │                               │
│  │ 201 user │                               │
│  └──────────┘                               │
│                                              │
│  [+ Add Step]                               │
└─────────────────────────────────────────────┘
```

---

## 8. 디렉토리 구조 확장

```
my-server/
├── rash.config.json
├── routes/
├── schemas/
├── models/
├── middleware/
├── handlers/
├── components/                    # NEW
│   ├── pagination.component.json
│   ├── soft-delete.component.json
│   └── cache-aside.component.json
├── functions/                     # NEW
│   ├── format-currency.function.json
│   └── calculate-tax.function.json
├── contracts/                     # NEW
│   └── repository.contract.json
├── state/                         # NEW
│   └── app-state.state.json
└── .rash/
```

파일 네이밍 추가:

| 파일 종류 | 네이밍 | 예시 |
|-----------|--------|------|
| 컴포넌트 | `kebab-case.component.json` | `pagination.component.json` |
| 함수 | `kebab-case.function.json` | `format-currency.function.json` |
| 계약 | `kebab-case.contract.json` | `repository.contract.json` |
| 상태 | `kebab-case.state.json` | `app-state.state.json` |

---

## 9. 코드 생성 영향

### 9.1 의존성 그래프 확장

`SpecDependencyGraph`에 추가되는 노드 타입:

```
기존: route, schema, model, middleware, handler, generated-file
추가: component, function, contract, state
```

새 엣지 예시:
- `component:paginate -> handler:users.listUsers`
- `function:formatCurrency -> component:format-response`
- `contract:Repository -> component:PrismaRepository`
- `state:app-state -> handler:*` (모듈 레벨 상태는 모든 핸들러에 영향)

### 9.2 IR 확장

```rust
pub struct ProjectIR {
    // 기존
    pub config: ProjectConfig,
    pub routes: Vec<RouteIR>,
    pub schemas: Vec<SchemaIR>,
    pub models: Vec<ModelIR>,
    pub middleware: Vec<MiddlewareIR>,
    pub handlers: Vec<HandlerIR>,

    // 추가
    pub components: Vec<ComponentIR>,
    pub functions: Vec<FunctionIR>,
    pub contracts: Vec<ContractIR>,
    pub state: Vec<StateIR>,
}
```

### 9.3 Emitter/Adapter 확장

`LanguageEmitter`에 추가되는 메서드:

```rust
pub trait LanguageEmitter {
    // 기존 메서드...

    /// 컴포넌트 → 함수/모듈 코드
    fn emit_component(&self, comp: &ComponentIR, strategy: ComponentStrategy, ctx: &mut EmitContext) -> String;

    /// 사용자 함수 → 함수 코드
    fn emit_user_function(&self, func: &FunctionIR, ctx: &mut EmitContext) -> String;

    /// 상태 초기화 코드
    fn emit_state_init(&self, state: &StateIR, ctx: &mut EmitContext) -> String;

    /// 계약 → 인터페이스/trait/protocol 코드
    fn emit_contract(&self, contract: &ContractIR, ctx: &mut EmitContext) -> String;
}
```

---

## 10. 로드맵 위치

| 기능 | Phase | 우선순위 |
|------|-------|---------|
| UserFunction (순수 함수) | Phase 2 | 높음 — 재사용의 기본 단위 |
| Component (부수 효과 포함) | Phase 3 | 높음 — 핸들러 중복 제거 |
| Project Variables | Phase 2 | 높음 — 설정 관리 기본 |
| State (init, lazy) | Phase 3 | 중간 — DB pool 등 필수 |
| Middleware Composition | Phase 3 | 중간 — 미들웨어 체이닝 |
| Pipeline Handler | Phase 4 | 중간 — 복잡한 핸들러 분해 |
| Contract/Interface | Phase 5 | 낮음 — 교체 가능한 추상화 |
| State (reactive) | Phase 5 | 낮음 — GUI 내 반응성 |
| Component Override/Extends | Phase 5 | 낮음 — 고급 재사용 패턴 |

---

## 11. 설계 결정 사항

### 11.1 컴포넌트 공유/패키지

**결정: 점진적 확장** — 로컬 → Git → 전용 레지스트리

| 단계 | 시점 | 방식 | 설명 |
|------|------|------|------|
| 1단계 | MVP~Phase 4 | **로컬 전용** | 프로젝트 내 `components/`만 사용. 외부 의존성 없음 |
| 2단계 | Phase 5+ | **Git 기반** | `rash add git@.../pagination.git` — 기존 인프라 활용, 버전은 git tag |
| 3단계 | 사용자 규모 확보 후 | **전용 레지스트리** | `rash add @rash/pagination` — 검색, 의존성 해결, 네임스페이스 |

근거:
- 처음부터 레지스트리를 만드는 건 과도한 투자. 사용자와 컴포넌트 생태계가 형성되기 전까지는 불필요.
- Git 기반 공유는 Terraform module 방식(`source = "git::https://..."`)의 검증된 패턴.
- 3단계 진입 조건: 커뮤니티 공유 컴포넌트가 50개 이상, 또는 사용자 요청이 충분할 때.

공유 시 추가 고려사항:
- **호환성 문제**: Tier 1 노드(DbQuery 등)를 사용하는 컴포넌트는 특정 DB/ORM에 종속 → 메타데이터에 `requires.database`, `requires.orm` 명시
- **전이적 의존성**: 컴포넌트가 다른 컴포넌트를 참조하면 재귀적으로 해결 필요 → 2단계에서 `rash.lock.json`에 의존 트리 기록
- **네임스페이스**: 3단계에서 `@org/component` 형태 (npm 스코프 컨벤션)

### 11.2 버전 관리

**결정: MVP에서는 메타데이터 전용, 공유 도입 시 semver 적용**

**현재 (Phase 2~4):**

`component.version` 필드는 **정보 제공용**으로만 사용. 호환성 검사 없음. 로컬 파일은 git history가 사실상 버전 관리.

```json
{
  "name": "paginate",
  "version": "1.0.0"
}
```

**공유 도입 후 (Phase 5+):**

semver를 적용하되, breaking change 기준을 명확히 정의:

```
Major (breaking):
  - inputs/outputs의 필드 제거 또는 타입 변경
  - contextInputs의 source 경로 변경
  - body에서 사용하는 DomainNode 변경 (DB 스키마 가정 변경)

Minor (feature):
  - inputs에 optional 필드 추가
  - outputs에 필드 추가
  - body 내부 최적화 (외부 동작 동일)

Patch:
  - 버그 수정
  - description/meta 변경
```

`extends`에서 부모 버전 범위 지정:

```json
{
  "extends": "paginate",
  "versionRange": "^1.0.0"
}
```

Resolver가 `paginate@1.x.x` 범위에서 호환 버전을 탐색. 로컬에서는 단일 파일이므로 현재 버전이 범위 내인지만 검증.

### 11.3 컴포넌트/함수 테스트

**결정: 함수는 `*.test.json` (스펙 레벨), 컴포넌트는 생성 코드 기반 테스트 자동 생성**

#### 함수 테스트 (`*.function.test.json`)

순수 함수는 입력/출력만 검증하면 되므로 mock 없이 스펙 레벨에서 테스트 가능.

```
functions/
├── format-currency.function.json
└── format-currency.function.test.json
```

```json
{
  "$schema": "https://rash.dev/schemas/function-test.json",
  "target": "formatCurrency",
  "tests": [
    {
      "name": "기본 KRW 포맷",
      "params": { "amount": 15000 },
      "expect": { "returns": "₩15,000" }
    },
    {
      "name": "USD 포맷",
      "params": { "amount": 99.99, "currency": "USD" },
      "expect": { "returns": "$99.99" }
    },
    {
      "name": "0원",
      "params": { "amount": 0 },
      "expect": { "returns": "₩0" }
    },
    {
      "name": "음수 금액",
      "params": { "amount": -5000 },
      "expect": { "returns": "-₩5,000" }
    }
  ]
}
```

함수 테스트 실행: 생성된 코드의 함수를 직접 호출하여 검증. 런타임별로 테스트 러너 통합.

#### 컴포넌트 테스트 (`*.component.test.json`)

부수 효과가 있는 컴포넌트는 mock이 필요. GUI에서 mock 데이터를 시각적으로 설정할 수 있도록 한다.

```
components/
├── pagination.component.json
└── pagination.component.test.json
```

```json
{
  "$schema": "https://rash.dev/schemas/component-test.json",
  "target": "paginate",
  "tests": [
    {
      "name": "기본 페이지네이션",
      "typeArgs": ["User"],
      "inputs": {
        "model": "User",
        "where": {}
      },
      "context": {
        "query.page": 1,
        "query.limit": 10
      },
      "mock": {
        "db.User.count": { "returns": 50 },
        "db.User.findMany": {
          "returns": [
            { "id": "1", "name": "Alice" },
            { "id": "2", "name": "Bob" }
          ]
        }
      },
      "expect": {
        "outputs.total": { "eq": 50 },
        "outputs.page": { "eq": 1 },
        "outputs.totalPages": { "eq": 5 },
        "outputs.data": { "type": "array", "length": 2 }
      }
    },
    {
      "name": "빈 결과",
      "typeArgs": ["User"],
      "inputs": {
        "model": "User",
        "where": { "role": "superadmin" }
      },
      "context": {
        "query.page": 1,
        "query.limit": 10
      },
      "mock": {
        "db.User.count": { "returns": 0 },
        "db.User.findMany": { "returns": [] }
      },
      "expect": {
        "outputs.total": { "eq": 0 },
        "outputs.data": { "eq": [] },
        "outputs.totalPages": { "eq": 0 }
      }
    },
    {
      "name": "두 번째 페이지",
      "typeArgs": ["User"],
      "inputs": {
        "model": "User",
        "where": {}
      },
      "context": {
        "query.page": 2,
        "query.limit": 5
      },
      "mock": {
        "db.User.count": { "returns": 12 },
        "db.User.findMany": {
          "args_match": { "skip": 5, "take": 5 },
          "returns": [
            { "id": "6", "name": "Frank" },
            { "id": "7", "name": "Grace" }
          ]
        }
      },
      "expect": {
        "outputs.page": { "eq": 2 },
        "outputs.totalPages": { "eq": 3 }
      }
    }
  ]
}
```

#### 테스트 코드 생성 전략

컴포넌트 테스트 스펙에서 실제 테스트 코드를 자동 생성:

```typescript
// 생성 결과: __tests__/components/paginate.test.ts
import { paginate } from "../src/components/paginate";
import { prismaMock } from "../__mocks__/prisma";

describe("paginate", () => {
  it("기본 페이지네이션", async () => {
    prismaMock.user.count.mockResolvedValue(50);
    prismaMock.user.findMany.mockResolvedValue([
      { id: "1", name: "Alice" },
      { id: "2", name: "Bob" },
    ]);

    const result = await paginate<User>({
      model: "User",
      where: {},
      page: 1,
      limit: 10,
    });

    expect(result.total).toBe(50);
    expect(result.page).toBe(1);
    expect(result.totalPages).toBe(5);
    expect(result.data).toHaveLength(2);
  });
});
```

#### 테스트 파일 네이밍

| 대상 | 테스트 파일 | 예시 |
|------|------------|------|
| 함수 | `*.function.test.json` | `format-currency.function.test.json` |
| 컴포넌트 | `*.component.test.json` | `pagination.component.test.json` |
| 핸들러 (API) | 기존 `*.test.json` 유지 | `user-api.test.json` (runtime.md 참조) |

### 11.4 GUI 시각화 수준

**결정: 하이브리드 3단계 줌 레벨** — 추상화 수준을 자유롭게 전환

#### 줌 레벨

```
[Level 1: 높은 추상화]    [Level 2: 중간 추상화]    [Level 3: 낮은 추상화]
파이프라인/흐름 뷰         구조화된 폼 에디터         인라인 코드/AST 뷰
```

| 레벨 | 대상 | 표현 방식 | 편집 | 용도 |
|------|------|----------|------|------|
| **L1: Flow** | 핸들러, 파이프라인 | 노드 → 노드 흐름도 | 단계 추가/제거/재배치 | 전체 구조 파악, 파이프라인 설계 |
| **L2: Form** | 컴포넌트 내부, 미들웨어 설정 | 구조화된 폼 (필드별 입력) | 입력값/설정 편집 | 일반적인 편집 작업 (기본 뷰) |
| **L3: Code** | 복잡한 표현식, 조건문 | 인라인 코드 에디터 | 직접 AST/코드 편집 | 세밀한 로직 조정 |

각 뷰는 항상 전환 가능하며, 우측 상단 토글로 접근:

```
┌─────────────────────────────────────────────────────┐
│ Handler: createUser                  [Flow][Form][</>]│
│                                                       │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐       │
│  │ validate │───▶│ hashPw   │───▶│ dbInsert │       │  ← L1: Flow
│  └────┬─────┘    └────┬─────┘    └────┬─────┘       │
│       │               │               │              │
│  ┌────▼──────────────────────────────────────────┐   │
│  │ Step: validate                                │   │
│  │   Schema: [CreateUserBody ▼]                  │   │  ← L2: Form
│  │   On Fail: [return 400    ▼]                  │   │
│  │   Error Format: [ValidationErrorResponse ▼]   │   │
│  └───────────────────────────────────────────────┘   │
│                                                       │
│  Expression: where condition                          │
│  ┌───────────────────────────────────────────────┐   │
│  │ user.role === "admin" && user.active !== false │   │  ← L3: Code
│  └───────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

#### 기본 뷰 정책

| 편집 대상 | 기본 뷰 | 이유 |
|-----------|---------|------|
| 핸들러 (pipeline) | L1: Flow | 파이프라인은 흐름이 핵심 |
| 핸들러 (imperative) | L2: Form | 문 단위 편집이 주 작업 |
| 컴포넌트 내부 | L2: Form | 입력/출력 설정이 주 작업 |
| 미들웨어 설정 | L2: Form | config 편집 |
| 함수 body | L3: Code | 순수 로직은 코드가 직관적 |
| 복합 표현식 | L3: Code | 조건문/연산은 코드가 빠름 |

#### 코드 프리뷰 (읽기 전용)

모든 레벨에서 **코드 프리뷰 패널**을 사이드바로 열 수 있음. 타겟 언어의 생성 코드를 실시간으로 미리 보여준다.

```
┌────────────────────────────┬──────────────────────┐
│ Handler: createUser (Form) │ Preview (TypeScript)  │
│                            │                       │
│ Step 1: validate           │ export async function │
│   Schema: CreateUserBody   │ createUser(ctx) {     │
│   On Fail: return 400      │   const validated =   │
│                            │     CreateUserBody    │
│ Step 2: hashPw             │     .parse(ctx.body); │
│   Field: password          │   validated.password  │
│   Algorithm: bcrypt        │     = await bcrypt    │
│                            │     .hash(...);       │
│ Step 3: dbInsert           │   const user = await  │
│   Model: User              │     prisma.user       │
│                            │     .create({...});   │
│ Step 4: respond            │   return ctx.json(    │
│   Status: 201              │     201, user);       │
│   Body: user               │ }                     │
└────────────────────────────┴──────────────────────┘
```

### 11.5 NativeBridge + 컴포넌트 다국어 전략

**결정: Contract 기반 다국어 구현 분리**

Bridge 노드(Tier 3)를 포함한 컴포넌트를 여러 언어에서 사용하려면, Contract를 통해 인터페이스를 통일하고 언어별 구현을 분리한다.

#### 구조

```
contracts/
└── email-sender.contract.json          # 언어 독립 인터페이스

components/
├── ses-email-sender.component.json     # TypeScript 구현 (npm:@aws-sdk/client-ses)
└── boto3-email-sender.component.json   # Python 구현 (pip:boto3)
```

#### Contract (언어 독립)

```json
{
  "$schema": "https://rash.dev/schemas/contract.json",
  "name": "EmailSender",
  "description": "이메일 전송 계약",

  "methods": {
    "send": {
      "params": [
        { "name": "to", "type": "string" },
        { "name": "subject", "type": "string" },
        { "name": "body", "type": "string" },
        { "name": "html", "type": "boolean", "default": false }
      ],
      "returnType": {
        "type": "object",
        "properties": {
          "messageId": { "type": "string" },
          "success": { "type": "boolean" }
        }
      },
      "async": true
    }
  }
}
```

#### 언어별 구현

```json
{
  "$schema": "https://rash.dev/schemas/component.json",
  "name": "SesEmailSender",
  "implements": "EmailSender",

  "meta": {
    "languages": ["typescript"],
    "maxTier": 3,
    "bridges": ["npm:@aws-sdk/client-ses"]
  },

  "methods": {
    "send": {
      "body": [
        {
          "type": "NativeBridge",
          "tier": 3,
          "language": "typescript",
          "package": "npm:@aws-sdk/client-ses",
          "import": { "name": "SESClient, SendEmailCommand", "from": "@aws-sdk/client-ses" },
          "call": {
            "method": "sesClient.send",
            "args": [{ "type": "Identifier", "tier": 0, "name": "$buildSendCommand(to, subject, body, html)" }]
          },
          "returnType": "object"
        }
      ]
    }
  }
}
```

```json
{
  "$schema": "https://rash.dev/schemas/component.json",
  "name": "Boto3EmailSender",
  "implements": "EmailSender",

  "meta": {
    "languages": ["python"],
    "maxTier": 3,
    "bridges": ["pip:boto3"]
  },

  "methods": {
    "send": {
      "body": [
        {
          "type": "NativeBridge",
          "tier": 3,
          "language": "python",
          "package": "pip:boto3",
          "import": { "name": "boto3", "from": "boto3" },
          "call": {
            "method": "ses_client.send_email",
            "args": [{ "type": "Identifier", "tier": 0, "name": "$build_send_params(to, subject, body, html)" }]
          },
          "returnType": "object"
        }
      ]
    }
  }
}
```

#### Resolver 동작

핸들러에서 Contract로 참조하면 Resolver가 타겟 언어에 맞는 구현을 자동 선택:

```json
{
  "type": "UseComponent",
  "tier": 0,
  "ref": "EmailSender",
  "inputs": {
    "to": { "type": "Identifier", "tier": 0, "name": "userEmail" },
    "subject": { "type": "Literal", "tier": 0, "value": "Welcome!" },
    "body": { "type": "Identifier", "tier": 0, "name": "emailBody" }
  },
  "bind": "sendResult"
}
```

Resolver 규칙:
1. `ref`가 Contract명인지 확인
2. `implements: "EmailSender"`인 컴포넌트 목록 수집
3. `meta.languages`에 현재 타겟 언어가 포함된 구현을 선택
4. 후보가 0개 → `E_NO_IMPL_FOR_LANGUAGE` 에러
5. 후보가 2개 이상 → `E_AMBIGUOUS_IMPL` 에러 (사용자가 명시적으로 선택 필요)
6. 정확히 1개 → 해당 구현으로 자동 바인딩

#### Bridge 미포함 컴포넌트는 분리 불필요

Tier 0~2만 사용하는 컴포넌트는 모든 언어에서 동일하게 동작하므로 Contract 분리 없이 직접 사용:

```
┌─────────────────────────────────────────────────────┐
│ Bridge 포함 여부에 따른 자동 판단                      │
│                                                      │
│ Component maxTier ≤ 2  →  단일 구현, 모든 언어 호환  │
│ Component maxTier = 3  →  Contract + 언어별 구현 필요 │
│                                                      │
│ Validator가 maxTier=3인 컴포넌트를 Contract 없이      │
│ 사용하면 E_BRIDGE_NEEDS_CONTRACT 경고를 발생          │
└─────────────────────────────────────────────────────┘
```

#### 타겟 언어 전환 시 동작

사용자가 `rash.config.json`의 `target.language`를 변경하면:

1. Validator가 모든 `UseComponent` 참조를 검사
2. Contract 기반 참조: 새 언어의 구현이 존재하는지 확인
3. 직접 Bridge 컴포넌트 참조: `E_BRIDGE_LANG_MISMATCH` 에러
4. 미구현 Contract: `E_NO_IMPL_FOR_LANGUAGE` 에러 + 구현 생성 가이드 제시

---

## 12. 결정 요약표

| 항목 | 결정 | Phase |
|------|------|-------|
| 컴포넌트 공유 | 로컬 → Git → 전용 레지스트리 (점진적) | Phase 5+ |
| 버전 관리 | MVP는 메타데이터만, 공유 시 semver + breaking 기준 명문화 | Phase 5+ |
| 함수 테스트 | `*.function.test.json` (입력/출력 검증) | Phase 2 |
| 컴포넌트 테스트 | `*.component.test.json` (mock 정의) + 테스트 코드 자동 생성 | Phase 3 |
| GUI 시각화 | 하이브리드 3단계 줌 (L1:Flow, L2:Form, L3:Code) + 코드 프리뷰 패널 | Phase 2~4 |
| NativeBridge 다국어 | Contract 기반 언어별 구현 분리, Resolver 자동 선택 | Phase 3 |

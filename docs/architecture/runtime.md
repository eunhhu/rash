# 런타임 시스템

Rash는 Tauri 앱 내에서 child process로 서버를 직접 실행하고 관리한다. 코드 변경 시 HMU(Hot Module Update)로 실시간 갱신하며, 내장 테스트 러너로 API를 검증한다.

> 문서 상태: **Current (MVP 런타임 핵심)** + **Target (Phase 6 테스트 러너/고급 운영)** 를 함께 포함한다.

## Child Process 관리

### 아키텍처

```
┌────────────────────────────────────────────────┐
│                Tauri App (Main Process)          │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │           ProcessManager                  │   │
│  │                                           │   │
│  │  ┌─────────────┐  ┌─────────────────┐   │   │
│  │  │ Process     │  │ PortDetector    │   │   │
│  │  │ Lifecycle   │  │ (stdout 파싱)    │   │   │
│  │  └─────────────┘  └─────────────────┘   │   │
│  │                                           │   │
│  │  ┌─────────────┐  ┌─────────────────┐   │   │
│  │  │ LogStream   │  │ HealthCheck     │   │   │
│  │  │ (pipe)      │  │ (주기적 ping)    │   │   │
│  │  └─────────────┘  └─────────────────┘   │   │
│  └──────────────────────────────────────────┘   │
│                     │                            │
│                     │ spawn / kill / signal       │
│                     ▼                            │
│  ┌──────────────────────────────────────────┐   │
│  │         Child Process (Server)            │   │
│  │                                           │   │
│  │  bun run dist/ts-express/src/index.ts     │   │
│  │  cargo run --manifest-path dist/rust-actix│   │
│  │  python dist/py-fastapi/main.py           │   │
│  └──────────────────────────────────────────┘   │
└────────────────────────────────────────────────┘
```

### ProcessManager

```rust
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct ProcessManager {
    app_handle: AppHandle,
    process: Option<RunningProcess>,
}

struct RunningProcess {
    child: Child,
    pid: u32,
    port: u16,
    language: Language,
    framework: Framework,
    started_at: DateTime<Utc>,
}

impl ProcessManager {
    /// 서버 시작
    pub async fn start(&mut self, project: &OpenProject) -> Result<u16> {
        // 이미 실행 중이면 정지
        if self.process.is_some() {
            self.stop().await?;
        }

        // 코드 생성 (또는 캐시 사용)
        let codegen_result = self.ensure_code_generated(project)?;

        // 런타임 명령 결정
        let (command, args, cwd) = self.resolve_runtime_command(project, &codegen_result);

        // 프로세스 시작
        let mut child = Command::new(&command)
            .args(&args)
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let pid = child.id().unwrap_or(0);

        // stdout/stderr 스트리밍
        self.stream_output(&mut child);

        // 포트 감지
        let port = self.detect_port(&mut child, project.config.server.port).await?;

        self.process = Some(RunningProcess {
            child,
            pid,
            port,
            language: project.config.target.language.clone(),
            framework: project.config.target.framework.clone(),
            started_at: Utc::now(),
        });

        // 프론트엔드에 상태 알림
        self.app_handle.emit("server:status", "running")?;

        Ok(port)
    }

    /// 서버 정지
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut process) = self.process.take() {
            // 우아한 종료 시도 (SIGTERM)
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                if let Some(pid) = process.child.id() {
                    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
                }
            }

            // 제한 시간 내 종료되지 않으면 강제 종료
            if tokio::time::timeout(std::time::Duration::from_secs(3), process.child.wait())
                .await
                .is_err()
            {
                process.child.kill().await?;
            }

            self.app_handle.emit("server:status", "stopped")?;
        }
        Ok(())
    }

    /// 서버 재시작
    pub async fn restart(&mut self, project: &OpenProject) -> Result<u16> {
        self.stop().await?;
        self.start(project).await
    }

    /// 런타임 명령 결정
    fn resolve_runtime_command(
        &self,
        project: &OpenProject,
        codegen: &CodegenResult,
    ) -> (String, Vec<String>, PathBuf) {
        let out_dir = codegen.output_dir.clone();

        match (&project.config.target.language, &project.config.target.runtime) {
            (Language::TypeScript, Runtime::Bun) => (
                "bun".into(),
                vec!["run".into(), "src/index.ts".into()],
                out_dir,
            ),
            (Language::TypeScript, Runtime::Node) => (
                "node".into(),
                vec!["--loader".into(), "ts-node/esm".into(), "src/index.ts".into()],
                out_dir,
            ),
            (Language::Rust, _) => (
                "cargo".into(),
                vec!["run".into()],
                out_dir,
            ),
            (Language::Python, _) => (
                "python".into(),
                vec!["-m".into(), "uvicorn".into(), "main:app".into(),
                     "--host".into(), "0.0.0.0".into(),
                     "--port".into(), project.config.server.port.to_string()],
                out_dir,
            ),
            (Language::Go, _) => (
                "go".into(),
                vec!["run".into(), ".".into()],
                out_dir,
            ),
        }
    }
}
```

### 실행 프리플라이트 체크

`start_server` 실행 전에 실패 가능 항목을 선검증한다.

검사 항목:
- 런타임 존재/버전 (`bun`, `node`, `python`, `cargo`, `go`)
- 출력 디렉토리 쓰기 권한
- 포트 충돌 여부 (`server.port`)
- 필수 환경 변수 존재 여부 (`DATABASE_URL`, `JWT_SECRET` 등)
- 프로젝트 스펙 유효성(`validate_spec`) 성공 여부

```rust
pub struct PreflightReport {
    pub ok: bool,
    pub checks: Vec<PreflightCheck>,
}

pub struct PreflightCheck {
    pub code: String,        // E_RUNTIME_NOT_FOUND, E_PORT_IN_USE ...
    pub status: CheckStatus, // pass | warn | fail
    pub message: String,
    pub suggestion: Option<String>,
}
```

정책:
- `fail`가 하나라도 있으면 프로세스를 시작하지 않는다.
- `warn`만 있는 경우 사용자 확인 후 시작 가능하다.
- 결과는 `server:preflight` 이벤트와 IPC 응답에 동일 포맷으로 전달한다.

### 로그 스트리밍

```rust
impl ProcessManager {
    fn stream_output(&self, child: &mut Child) {
        let app = self.app_handle.clone();

        // stdout
        if let Some(stdout) = child.stdout.take() {
            let app_clone = app.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = app_clone.emit("server:log", LogEntry {
                        timestamp: Utc::now(),
                        level: LogLevel::Info,
                        message: line,
                        source: LogSource::Stdout,
                    });
                }
            });
        }

        // stderr
        if let Some(stderr) = child.stderr.take() {
            let app_clone = app.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = app_clone.emit("server:log", LogEntry {
                        timestamp: Utc::now(),
                        level: LogLevel::Error,
                        message: line,
                        source: LogSource::Stderr,
                    });
                }
            });
        }
    }
}
```

## HMU (Hot Module Update)

### 개념

HMU는 프론트엔드의 HMR(Hot Module Replacement)에 대응하는 개념이다. 스펙이 변경되면 전체 서버를 재시작하지 않고, 변경된 모듈만 교체한다.

### 프로토콜

```
┌──────────┐                          ┌──────────────┐
│  Rash    │                          │ Server       │
│  App     │                          │ Process      │
│          │                          │              │
│  [스펙변경]                         │              │
│     │                               │              │
│  [변경감지]                          │              │
│     │                               │              │
│  [영향분석]                          │              │
│     │                               │              │
│  [증분코드젠]                        │              │
│     │                               │              │
│     ├─── HMU_UPDATE ───────────────▶│              │
│     │    { module, code, hash }     │  [모듈교체]   │
│     │                               │     │        │
│     │◀── HMU_ACK ──────────────────│     │        │
│     │    { status, hash }           │  [완료]      │
│     │                               │              │
│  [UI업데이트]                        │              │
└──────────┘                          └──────────────┘
```

### HMU 메시지 포맷

```json
// HMU_UPDATE: Rash → Server
{
  "type": "HMU_UPDATE",
  "id": "hmu_001",
  "timestamp": "2026-01-15T12:00:00Z",
  "modules": [
    {
      "path": "src/handlers/users.ts",
      "action": "replace",
      "content": "export async function getUser(ctx) { ... }",
      "hash": "sha256:abc123..."
    }
  ]
}

// HMU_ACK: Server → Rash
{
  "type": "HMU_ACK",
  "id": "hmu_001",
  "status": "success",  // "success" | "partial" | "failed"
  "applied": ["src/handlers/users.ts"],
  "failed": [],
  "requiresRestart": false
}
```

### HMU Atomic 적용/롤백 정책

HMU는 "부분 성공"보다 "일관성 유지"를 우선한다.

적용 단계:
1. **Stage**: 수신 모듈을 임시 영역에 로드/구문 검증
2. **Commit**: 모든 모듈 검증 성공 시 한 번에 교체
3. **Rollback**: 하나라도 실패하면 이전 모듈 세트로 즉시 복구

```json
{
  "type": "HMU_ACK",
  "id": "hmu_001",
  "status": "failed",
  "applied": [],
  "failed": ["src/handlers/users.ts"],
  "rolledBack": true,
  "requiresRestart": false
}
```

에스컬레이션 규칙:
- 같은 모듈에서 연속 `N`회(기본 3회) HMU 실패 시 `requiresRestart=true`
- 런타임이 atomic 교체를 지원하지 않으면 HMU를 건너뛰고 즉시 graceful restart

### HMU 통신 채널

| 런타임 | 채널 | 설명 |
|--------|------|------|
| Bun/Node | IPC (process.send) | 부모-자식 프로세스 IPC |
| Python | Unix Socket | `/tmp/rash-hmu-{pid}.sock` |
| Rust | Unix Socket | `/tmp/rash-hmu-{pid}.sock` |
| Go | Unix Socket | `/tmp/rash-hmu-{pid}.sock` |

### HMU 언어별 구현

각 언어의 생성 코드에 HMU 클라이언트가 포함된다.

**TypeScript (Bun/Node)**
```typescript
// 생성 코드에 포함되는 HMU 클라이언트
import { createHmuClient } from "@rash/hmu-client";

const hmu = createHmuClient({
  async onUpdate(modules) {
    for (const mod of modules) {
      // ESM 기준: 캐시 버스팅 import로 모듈 재로드
      await import(`${mod.path}?hmu=${Date.now()}`);
    }
    // 라우트 재등록
    await reregisterRoutes();
  },
});

// 참고: CJS 런타임을 별도 지원할 경우에만 require.cache 전략을 사용한다.
```

**Python (FastAPI)**
```python
# 생성 코드에 포함되는 HMU 클라이언트
import importlib
from rash_hmu import HmuClient

hmu = HmuClient()

@hmu.on_update
async def handle_update(modules):
    for mod in modules:
        # 모듈 리로드
        importlib.reload(importlib.import_module(mod.module_name))
    # 라우트 재등록
    reregister_routes(app)
```

### HMU 전체 흐름

```
1. 사용자가 GUI에서 핸들러 수정
   │
2. SolidJS Store 업데이트 → IPC invoke: write_handler
   │
3. Rust: 스펙 파일 저장
   │
4. Rust: 변경 감지 (diff 기반)
   │     ┌─ 라우트 변경: 해당 라우트 파일만 재생성
   │     ├─ 스키마 변경: 스키마 + 참조하는 핸들러 재생성
   │     ├─ 핸들러 변경: 핸들러 파일만 재생성
   │     └─ 미들웨어 변경: 미들웨어 + 이를 사용하는 라우트 재생성
   │
5. Rust: 점진적 코드 생성 (IncrementalCodegen)
   │
6. Rust: HMU 메시지 생성 및 전송
   │
7. Server Process: 모듈 교체
   │
8. Server Process: HMU_ACK 응답
   │
9. Rust: 프론트엔드에 결과 알림 (hmu:result 이벤트)
   │
10. SolidJS: UI에 성공/실패 표시
```

### HMU가 불가능한 경우

다음 변경은 서버 재시작이 필요하다:

| 변경 유형 | 이유 |
|-----------|------|
| 프레임워크 변경 | 전체 구조 변경 |
| DB 스키마 변경 | 마이그레이션 필요 |
| 글로벌 미들웨어 변경 | 앱 초기화 단계 변경 |
| 포트/호스트 변경 | 서버 재바인딩 필요 |
| 의존성 추가/제거 | 패키지 재설치 필요 |

이 경우 자동으로 graceful restart를 수행한다.

## 테스트 러너

### 내장 API 테스트

GUI에서 직접 API를 테스트할 수 있는 Postman/Insomnia 스타일의 테스트 러너를 제공한다.

```
┌─────────────────────────────────────────────┐
│ Test Runner                                  │
│                                              │
│ ┌──────────────────────────────────────────┐│
│ │ GET  /api/v1/users                       ││
│ │ ──────────────────────────────────────── ││
│ │ Headers:                                  ││
│ │   Authorization: Bearer {token}           ││
│ │ Query:                                    ││
│ │   page: 1                                 ││
│ │   limit: 20                               ││
│ │                                           ││
│ │ [Send Request]                            ││
│ │                                           ││
│ │ Response: 200 OK (45ms)                   ││
│ │ ┌────────────────────────────────────┐   ││
│ │ │ {                                   │   ││
│ │ │   "data": [...],                    │   ││
│ │ │   "total": 42,                      │   ││
│ │ │   "page": 1,                        │   ││
│ │ │   "limit": 20                       │   ││
│ │ │ }                                   │   ││
│ │ └────────────────────────────────────┘   ││
│ └──────────────────────────────────────────┘│
└─────────────────────────────────────────────┘
```

### 테스트 스펙

라우트별로 테스트 케이스를 정의할 수 있다.

```json
{
  "$schema": "https://rash.dev/schemas/test.json",
  "name": "User API Tests",
  "baseUrl": "http://localhost:3000",

  "setup": {
    "description": "테스트 전 환경 준비",
    "steps": [
      {
        "name": "Create test user",
        "request": {
          "method": "POST",
          "path": "/api/v1/auth/register",
          "body": {
            "email": "test@example.com",
            "password": "password123",
            "name": "Test User"
          }
        },
        "capture": {
          "token": "$.response.body.token",
          "userId": "$.response.body.user.id"
        }
      }
    ]
  },

  "tests": [
    {
      "name": "List users requires auth",
      "request": {
        "method": "GET",
        "path": "/api/v1/users"
      },
      "expect": {
        "status": 401
      }
    },
    {
      "name": "List users with auth",
      "request": {
        "method": "GET",
        "path": "/api/v1/users",
        "headers": {
          "Authorization": "Bearer {{token}}"
        },
        "query": {
          "page": "1",
          "limit": "10"
        }
      },
      "expect": {
        "status": 200,
        "body": {
          "$.data": { "type": "array" },
          "$.total": { "type": "number", "gte": 1 },
          "$.page": { "eq": 1 }
        },
        "headers": {
          "content-type": "application/json"
        },
        "maxResponseTime": 500
      }
    },
    {
      "name": "Get user by ID",
      "request": {
        "method": "GET",
        "path": "/api/v1/users/{{userId}}",
        "headers": {
          "Authorization": "Bearer {{token}}"
        }
      },
      "expect": {
        "status": 200,
        "body": {
          "$.id": { "eq": "{{userId}}" },
          "$.email": { "eq": "test@example.com" }
        }
      }
    }
  ],

  "teardown": {
    "steps": [
      {
        "name": "Delete test user",
        "request": {
          "method": "DELETE",
          "path": "/api/v1/users/{{userId}}",
          "headers": {
            "Authorization": "Bearer {{token}}"
          }
        }
      }
    ]
  }
}
```

### Golden E2E 샘플 프로젝트

회귀 기준점으로 `golden-user-crud` 샘플 프로젝트를 유지한다.

검증 시나리오(고정):
1. 스펙 로드
2. 코드 생성 (TS/Express MVP 타겟)
3. 서버 실행
4. 기본 API 테스트 실행
5. 핸들러 1개 수정 후 HMU 반영
6. 동일 테스트 재실행

통과 조건:
- 1차/2차 테스트 모두 성공
- HMU 후 프로세스 재시작 없이 응답 갱신 확인
- 로그/이벤트(`server:status`, `hmu:result`)가 표준 포맷으로 수집

### 테스트 실행 흐름

```
[사용자: "Run Tests" 클릭]
    │
    ▼
[서버 실행 확인 (없으면 자동 시작)]
    │
    ▼
[Setup 단계 실행]
    │  - 캡처된 변수 저장 (token, userId 등)
    ▼
[테스트 케이스 순차 실행]
    │  - 각 테스트: 요청 → 응답 검증 → 결과 기록
    │  - 변수 치환: {{token}} → 실제 값
    ▼
[Teardown 단계 실행]
    │
    ▼
[결과 리포트]
    │
    ├── 통과: 3/4
    ├── 실패: 1/4
    ├── 총 소요시간: 234ms
    └── 실패 상세:
        └── "List users with auth": expected status 200, got 500
```

## 환경 관리

### 환경 변수

```json
// rash.config.json의 environments 섹션
{
  "environments": {
    "development": {
      "PORT": "3000",
      "DATABASE_URL": "postgresql://localhost:5432/mydb_dev",
      "JWT_SECRET": "dev-secret"
    },
    "test": {
      "PORT": "3001",
      "DATABASE_URL": "postgresql://localhost:5432/mydb_test",
      "JWT_SECRET": "test-secret"
    },
    "production": {
      "PORT": "${PORT}",
      "DATABASE_URL": "${DATABASE_URL}",
      "JWT_SECRET": "${JWT_SECRET}"
    }
  }
}
```

### 런타임 감지

Rash는 시스템에 설치된 런타임을 자동 감지한다.

```rust
pub struct RuntimeDetector;

impl RuntimeDetector {
    /// 시스템에 설치된 런타임 목록 반환
    pub async fn detect_installed() -> Vec<DetectedRuntime> {
        let mut runtimes = Vec::new();

        // Bun
        if let Ok(output) = Command::new("bun").arg("--version").output().await {
            if output.status.success() {
                runtimes.push(DetectedRuntime {
                    name: "bun".into(),
                    version: String::from_utf8_lossy(&output.stdout).trim().into(),
                    path: which::which("bun").ok(),
                });
            }
        }

        // Node
        if let Ok(output) = Command::new("node").arg("--version").output().await {
            // ...
        }

        // Python
        if let Ok(output) = Command::new("python3").arg("--version").output().await {
            // ...
        }

        // Rust (cargo)
        if let Ok(output) = Command::new("cargo").arg("--version").output().await {
            // ...
        }

        // Go
        if let Ok(output) = Command::new("go").arg("version").output().await {
            // ...
        }

        runtimes
    }
}
```

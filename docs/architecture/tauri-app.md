# Tauri 앱 구조

Rash는 Tauri v2 기반 데스크톱 앱이다. Rust 백엔드가 핵심 엔진을 담당하고, SolidJS 프론트엔드가 GUI를 제공한다.

> 문서 상태: **Current (MVP UI/기능)** + **Target (Phase 6+ 확장 UI)** 를 함께 포함한다.

## 프로젝트 파일 구조

```
rash/
├── src-tauri/                     # Rust 백엔드
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs                # 진입점
│   │   ├── lib.rs                 # Tauri plugin 등록
│   │   ├── commands/              # IPC 명령 핸들러
│   │   │   ├── mod.rs
│   │   │   ├── project.rs         # 프로젝트 CRUD
│   │   │   ├── spec.rs            # 스펙 읽기/쓰기
│   │   │   ├── codegen.rs         # 코드 생성
│   │   │   ├── runtime.rs         # 서버 실행/정지
│   │   │   └── openapi.rs         # OpenAPI 임포트/익스포트
│   │   ├── state/                 # 앱 상태 관리
│   │   │   ├── mod.rs
│   │   │   ├── project_state.rs
│   │   │   └── runtime_state.rs
│   │   └── error.rs               # 에러 타입
│   └── crates/                    # 내부 라이브러리
│       ├── rash-spec/             # 스펙 파싱/직렬화
│       ├── rash-ir/               # 중간 표현
│       ├── rash-codegen/          # 코드 생성
│       ├── rash-valid/            # 검증
│       ├── rash-runtime/          # 런타임 관리
│       └── rash-openapi/          # OpenAPI 변환
│
├── src/                           # SolidJS 프론트엔드
│   ├── index.html
│   ├── App.tsx
│   ├── main.tsx                   # 진입점
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx        # 프로젝트 트리
│   │   │   ├── TopBar.tsx         # 상단 메뉴/상태바
│   │   │   ├── MainPanel.tsx      # 메인 편집 영역
│   │   │   └── BottomPanel.tsx    # 로그/터미널
│   │   ├── route/
│   │   │   ├── RouteExplorer.tsx  # 라우트 트리 (파일 탐색기 스타일)
│   │   │   ├── RouteEditor.tsx    # 라우트 상세 편집
│   │   │   ├── MethodPanel.tsx    # HTTP 메서드별 설정
│   │   │   └── ParamEditor.tsx    # 경로 파라미터 편집
│   │   ├── schema/
│   │   │   ├── SchemaEditor.tsx   # 스키마 시각 편집기
│   │   │   ├── FieldEditor.tsx    # 필드 하나의 타입/제약조건
│   │   │   └── SchemaPreview.tsx  # 변환 코드 미리보기
│   │   ├── model/
│   │   │   ├── ModelEditor.tsx    # DB 모델 편집기
│   │   │   ├── ColumnEditor.tsx   # 컬럼 편집
│   │   │   ├── RelationEditor.tsx # 관계 편집
│   │   │   └── ERDiagram.tsx      # ER 다이어그램
│   │   ├── handler/
│   │   │   ├── HandlerEditor.tsx  # 핸들러 AST 시각 편집기
│   │   │   ├── NodePalette.tsx    # 사용 가능한 AST 노드 팔레트
│   │   │   ├── AstNodeView.tsx    # 개별 AST 노드 렌더링
│   │   │   └── CodePreview.tsx    # 변환 코드 실시간 미리보기
│   │   ├── middleware/
│   │   │   ├── MiddlewareList.tsx # 미들웨어 목록
│   │   │   └── MiddlewareConfig.tsx # 미들웨어 설정
│   │   ├── runtime/
│   │   │   ├── ServerControl.tsx  # 서버 시작/정지 버튼
│   │   │   ├── LogViewer.tsx      # 실시간 로그
│   │   │   └── TestRunner.tsx     # API 테스트 UI (Phase 6, MVP에서는 숨김/비활성)
│   │   └── common/
│   │       ├── MonacoEditor.tsx   # Monaco 에디터 래퍼
│   │       ├── JsonViewer.tsx     # JSON 트리 뷰어
│   │       └── TabPanel.tsx       # 탭 컴포넌트
│   ├── stores/
│   │   ├── projectStore.ts        # 프로젝트 상태
│   │   ├── editorStore.ts         # 편집기 상태
│   │   ├── runtimeStore.ts        # 런타임 상태
│   │   └── uiStore.ts             # UI 상태 (패널 크기, 탭 등)
│   ├── ipc/
│   │   ├── invoke.ts              # Tauri invoke 래퍼
│   │   ├── events.ts              # Tauri 이벤트 리스너
│   │   └── types.ts               # IPC 타입 정의
│   ├── utils/
│   │   ├── ast.ts                 # AST 유틸리티
│   │   ├── schema.ts              # 스키마 유틸리티
│   │   └── format.ts              # 포맷팅 유틸리티
│   └── styles/
│       ├── global.css
│       └── themes/
│           ├── dark.css
│           └── light.css
│
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

## Rust 백엔드 상세

### Tauri 명령 등록

```rust
// src-tauri/src/lib.rs
use tauri::Manager;

mod commands;
mod state;
mod error;

pub fn run() {
    tauri::Builder::default()
        .manage(state::ProjectState::default())
        .manage(state::RuntimeState::default())
        .invoke_handler(tauri::generate_handler![
            // 프로젝트
            commands::project::create_project,
            commands::project::open_project,
            commands::project::close_project,
            commands::project::get_project_tree,

            // 스펙 CRUD
            commands::spec::read_route,
            commands::spec::write_route,
            commands::spec::delete_route,
            commands::spec::read_schema,
            commands::spec::write_schema,
            commands::spec::read_model,
            commands::spec::write_model,
            commands::spec::read_middleware,
            commands::spec::write_middleware,
            commands::spec::read_handler,
            commands::spec::write_handler,

            // 코드 생성
            commands::codegen::generate_code,
            commands::codegen::preview_code,
            commands::codegen::generate_single_file,

            // 런타임
            commands::runtime::start_server,
            commands::runtime::stop_server,
            commands::runtime::restart_server,
            commands::runtime::get_server_status,
            commands::runtime::get_server_logs,

            // OpenAPI
            commands::openapi::export_openapi,
            commands::openapi::import_openapi,

            // 유효성 검사
            commands::spec::validate_spec,
        ])
        .setup(|app| {
            // 이벤트 리스너 등록
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### IPC 명령 상세

#### 프로젝트 관리

```rust
// commands/project.rs
use tauri::State;
use crate::state::ProjectState;

#[tauri::command]
pub async fn create_project(
    name: String,
    path: String,
    target: TargetConfig,
    state: State<'_, ProjectState>,
) -> Result<ProjectInfo, String> {
    // 1. 디렉토리 생성
    // 2. rash.config.json 초기화
    // 3. 기본 디렉토리 구조 생성
    // 4. 상태 업데이트
}

#[tauri::command]
pub async fn open_project(
    path: String,
    state: State<'_, ProjectState>,
) -> Result<ProjectTree, String> {
    // 1. rash.config.json 파싱
    // 2. 모든 스펙 파일 인덱싱
    // 3. 프로젝트 트리 구성
    // 4. 상태 업데이트
}

#[tauri::command]
pub async fn get_project_tree(
    state: State<'_, ProjectState>,
) -> Result<ProjectTree, String> {
    // 현재 프로젝트의 파일 트리 반환
}
```

#### 스펙 CRUD

```rust
// commands/spec.rs

#[tauri::command]
pub async fn read_route(
    path: String,  // "api/v1/users"
    state: State<'_, ProjectState>,
) -> Result<RouteSpec, String> {
    // routes/api/v1/users.route.json 읽기 + 파싱
}

#[tauri::command]
pub async fn write_route(
    path: String,
    spec: RouteSpec,
    state: State<'_, ProjectState>,
) -> Result<(), String> {
    // 1. 검증
    // 2. JSON 직렬화
    // 3. 파일 쓰기
    // 4. 변경 이벤트 발행 (HMU 트리거용)
}

#[tauri::command]
pub async fn write_handler(
    name: String,
    ast: HandlerAst,
    state: State<'_, ProjectState>,
) -> Result<ValidationResult, String> {
    // 1. AST 유효성 검사 (타입 체크, tier 계산)
    // 2. JSON 직렬화
    // 3. 파일 쓰기
    // 4. 변경 이벤트 발행
}

#[tauri::command]
pub async fn validate_spec(
    state: State<'_, ProjectState>,
) -> Result<Vec<ValidationError>, String> {
    // 전체 프로젝트 스펙 검증
    // - 참조 무결성 (존재하지 않는 스키마 참조 등)
    // - 타입 일관성
    // - 필수 필드 확인
}
```

#### 코드 생성

```rust
// commands/codegen.rs

#[tauri::command]
pub async fn generate_code(
    state: State<'_, ProjectState>,
) -> Result<CodegenResult, String> {
    // 전체 프로젝트 코드 생성
    // 1. 스펙 → IR
    // 2. IR → Emitter + Adapter → Code
    // 3. dist/ 디렉토리에 출력
}

#[tauri::command]
pub async fn preview_code(
    spec_type: SpecType,  // "route" | "schema" | "handler" 등
    spec: serde_json::Value,
    language: Language,
    framework: Framework,
) -> Result<String, String> {
    // 단일 스펙의 코드 변환 미리보기
    // GUI에서 실시간으로 변환 결과를 보여줄 때 사용
}
```

### State 관리 (Rust 측)

```rust
// state/project_state.rs
use std::sync::Mutex;
use std::path::PathBuf;

pub struct ProjectState {
    pub inner: Mutex<Option<OpenProject>>,
}

pub struct OpenProject {
    pub path: PathBuf,
    pub config: ProjectConfig,
    pub index: SpecIndex,  // 스펙 파일 인덱스 (빠른 참조용)
}

pub struct SpecIndex {
    pub routes: HashMap<String, RouteSpec>,
    pub schemas: HashMap<String, SchemaSpec>,
    pub models: HashMap<String, ModelSpec>,
    pub middleware: HashMap<String, MiddlewareSpec>,
    pub handlers: HashMap<String, HandlerSpec>,
    pub ref_graph: RefGraph,  // 참조 관계 그래프
}
```

```rust
// state/runtime_state.rs
use std::sync::Mutex;
use tokio::process::Child;

pub struct RuntimeState {
    pub inner: Mutex<Option<RunningServer>>,
}

pub struct RunningServer {
    pub process: Child,
    pub pid: u32,
    pub port: u16,
    pub language: Language,
    pub framework: Framework,
    pub started_at: DateTime<Utc>,
    pub log_buffer: Vec<LogEntry>,
}
```

## SolidJS 프론트엔드 상세

### 상태 관리 설계

SolidJS의 `createStore`와 `createSignal`을 사용하여 반응형 상태를 관리한다.

```typescript
// stores/projectStore.ts
import { createStore } from "solid-js/store";

interface ProjectState {
  isOpen: boolean;
  path: string | null;
  config: ProjectConfig | null;
  tree: ProjectTreeNode[];
  activeFile: string | null;
  dirty: Set<string>;  // 수정된 파일 경로들
}

const [project, setProject] = createStore<ProjectState>({
  isOpen: false,
  path: null,
  config: null,
  tree: [],
  activeFile: null,
  dirty: new Set(),
});

export function useProject() {
  return {
    project,

    async openProject(path: string) {
      const tree = await invoke<ProjectTree>("open_project", { path });
      setProject({
        isOpen: true,
        path,
        config: tree.config,
        tree: tree.nodes,
      });
    },

    async saveFile(filePath: string, content: unknown) {
      await invoke("write_route", { path: filePath, spec: content });
      setProject("dirty", (d) => {
        const next = new Set(d);
        next.delete(filePath);
        return next;
      });
    },

    markDirty(filePath: string) {
      setProject("dirty", (d) => new Set([...d, filePath]));
    },
  };
}
```

```typescript
// stores/runtimeStore.ts
import { createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";

type ServerStatus = "stopped" | "starting" | "running" | "error";

const [status, setStatus] = createSignal<ServerStatus>("stopped");
const [logs, setLogs] = createSignal<LogEntry[]>([]);
const [port, setPort] = createSignal<number | null>(null);

// Tauri 이벤트로 런타임 상태 동기화
listen<LogEntry>("server:log", (event) => {
  setLogs((prev) => [...prev, event.payload]);
});

listen<ServerStatus>("server:status", (event) => {
  setStatus(event.payload);
});

export function useRuntime() {
  return {
    status,
    logs,
    port,

    async startServer() {
      setStatus("starting");
      setLogs([]);
      const result = await invoke<{ port: number }>("start_server");
      setPort(result.port);
    },

    async stopServer() {
      await invoke("stop_server");
      setStatus("stopped");
      setPort(null);
    },
  };
}
```

### IPC 래퍼

```typescript
// ipc/invoke.ts
import { invoke as tauriInvoke } from "@tauri-apps/api/core";

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await tauriInvoke<T>(cmd, args);
  } catch (error) {
    // Rust 에러를 프론트엔드 에러로 변환
    console.error(`IPC Error [${cmd}]:`, error);
    throw new IpcError(cmd, error as string);
  }
}
```

```typescript
// ipc/types.ts
// Rust 타입과 1:1 대응하는 TypeScript 타입

export interface ProjectConfig {
  version: string;
  name: string;
  description: string;
  target: TargetConfig;
  server: ServerConfig;
  database?: DatabaseConfig;
}

export interface RouteSpec {
  path: string;
  description?: string;
  methods: Record<HttpMethod, EndpointSpec>;
  params?: Record<string, ParamSpec>;
  tags?: string[];
}

export interface HandlerAst {
  name: string;
  async: boolean;
  params: ParamDef[];
  returnType: TypeRef;
  body: AstStatement[];
  meta: {
    maxTier: number;
    languages: string[];
    bridges: BridgeInfo[];
  };
}
```

### 컴포넌트 계층

```
App
├── TopBar
│   ├── ProjectSelector
│   ├── TargetSelector (언어/프레임워크)
│   └── ServerControl (시작/정지/상태)
├── MainLayout
│   ├── Sidebar
│   │   ├── RouteExplorer (라우트 트리)
│   │   ├── SchemaList
│   │   ├── ModelList
│   │   └── MiddlewareList
│   ├── MainPanel
│   │   ├── TabPanel
│   │   │   ├── RouteEditor
│   │   │   │   ├── MethodPanel (GET/POST/PUT/DELETE)
│   │   │   │   ├── ParamEditor
│   │   │   │   ├── RequestEditor (body/query/header)
│   │   │   │   └── ResponseEditor
│   │   │   ├── SchemaEditor
│   │   │   │   ├── FieldEditor (재귀적)
│   │   │   │   └── SchemaPreview
│   │   │   ├── ModelEditor
│   │   │   │   ├── ColumnEditor
│   │   │   │   ├── RelationEditor
│   │   │   │   └── ERDiagram
│   │   │   ├── HandlerEditor
│   │   │   │   ├── NodePalette
│   │   │   │   ├── AstNodeView (재귀적)
│   │   │   │   └── CodePreview
│   │   │   └── MonacoEditor (NativeBridge용)
│   │   └── SplitPane (좌: 에디터, 우: 미리보기)
│   └── BottomPanel
│       ├── LogViewer
│       ├── TestRunner (Phase 6, MVP에서는 비활성 슬롯)
│       └── Terminal
└── Modals
    ├── CreateProjectDialog
    ├── ImportOpenAPIDialog
    └── SettingsDialog
```

## Tauri 이벤트 시스템

Rust 백엔드에서 프론트엔드로 비동기 이벤트를 전송하는 경우:

| 이벤트 | 방향 | 용도 |
|--------|------|------|
| `server:log` | Rust → JS | 서버 stdout/stderr 실시간 전달 |
| `server:status` | Rust → JS | 서버 상태 변경 알림 |
| `spec:changed` | Rust → JS | 외부에서 스펙 파일 변경 감지 (file watcher) |
| `codegen:progress` | Rust → JS | 코드 생성 진행률 |
| `hmu:result` | Rust → JS | HMU 적용 결과 |
| `validation:error` | Rust → JS | 실시간 유효성 검사 결과 |

### 이벤트 Payload 표준 스키마

Rust와 SolidJS가 동일 계약으로 통신하도록 공통 envelope를 사용한다.

```typescript
export interface AppEventEnvelope<T> {
  eventId: string;          // uuid
  eventType: string;        // server:log, hmu:result ...
  timestamp: string;        // ISO8601
  projectId?: string;
  payload: T;
}
```

대표 payload:

```typescript
export interface ServerLogPayload {
  level: "trace" | "debug" | "info" | "warn" | "error";
  source: "stdout" | "stderr" | "runtime";
  message: string;
}

export interface HmuResultPayload {
  status: "success" | "failed";
  applied: string[];
  failed: string[];
  rolledBack?: boolean;
  requiresRestart: boolean;
}

export interface ValidationErrorPayload {
  code: string;
  file: string;
  path: string; // JSONPath
  message: string;
  suggestion?: string;
}
```

운영 규칙:
- 새로운 이벤트 추가 시 `ipc/types.ts`에 타입을 먼저 추가한 뒤 Rust emit을 연결한다.
- `eventType` 문자열과 payload 타입 이름은 1:1로 매핑한다.
- `validation:error`는 `spec-format`의 오류 포맷(`code`, `file`, `path`)을 그대로 재사용한다.

### Golden E2E 연동

- `TestRunner`는 `golden-user-crud` 샘플 프로젝트를 바로 실행할 수 있는 프리셋을 제공한다.
- CI용 스모크 테스트에서는 동일 프리셋을 사용해 UI/IPC/Runtime 경로를 함께 검증한다.

### 이벤트 발행 (Rust)

```rust
use tauri::Emitter;

// 서버 로그 전달
app_handle.emit("server:log", LogEntry {
    timestamp: Utc::now(),
    level: LogLevel::Info,
    message: line,
})?;

// 서버 상태 변경
app_handle.emit("server:status", "running")?;
```

### 이벤트 수신 (SolidJS)

```typescript
import { listen } from "@tauri-apps/api/event";
import { onCleanup, onMount } from "solid-js";

function LogViewer() {
  const runtime = useRuntime();

  onMount(() => {
    const unlisten = listen<LogEntry>("server:log", (event) => {
      // 로그 추가
    });

    onCleanup(async () => {
      (await unlisten)();
    });
  });

  return (
    <div class="log-viewer">
      <For each={runtime.logs()}>
        {(log) => <LogLine entry={log} />}
      </For>
    </div>
  );
}
```

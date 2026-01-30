# relay — REQUIREMENTS

本文件用于把当前项目在文档中已经明确写出的“目标/范围/约束/验收方式”固化为可执行的需求清单，便于后续迭代与回滚。

## 需求来源（当前唯一来源）

- `README.md`
- `AGENTS.md`
- （新增对齐来源）`tiann/hapi` 的公开文档（README、cli/README、server/README、WHY_NOT_HAPPY）
- （新增对齐来源）`slopus/happy` 的公开文档（DeepWiki 汇总；用于能力拆解与术语对齐）

## 术语

- **server**：中心服务（认证、路由、存储、WebSocket），供 PWA 与 hostd 连接。
- **hostd**：宿主机守护进程（PTY 进程运行器、事件 spooling，出站 WS 连接 server）。
- **cli**：本地命令行封装（通过 hostd 启动 runs，不直接拥有/管理工具进程）。
- **web**：Svelte PWA（runs 列表、实时日志、approve/deny/input）。
- **spool**：hostd 本地 SQLite 事件队列/重放机制，用于离线与断线重连。
- **hapi 对齐**：本项目的目标能力集合，参考 `https://github.com/tiann/hapi` 的功能与使用体验，并以本文件为唯一落地需求清单（避免每次回看 GitHub）。
- **run.tool**：会话的后端工具/模型标识（如 `codex | claude | iflow | ...`），来自 `run.started.data.tool`，并体现在 sessions 列表的 `tool` 字段。
- **op_tool**：待审批的“操作工具名”（如 `rpc.fs.read` / `rpc.fs.write` / `bash`），用于审批相关 UI 展示与风险提示。

## 目标（Goals）

- 在宿主机上运行 AI coding CLIs（如 Codex / Claude / iFlow 等），并通过移动端友好的 PWA 远程监控与控制。
- 以多组件方式交付：Rust workspace（server/hostd）+ Bun CLI（cli）+ Svelte PWA（web）。
- 基于事件模型通过 WebSocket 串联：PWA ↔ server ↔ hostd，并支持远程输入（remote input）。
- hostd 支持离线/断线重连后的事件重放（spool replay）。

## 非目标（Non-goals）

- 该仓库处于 active development；文档中标注为 “skeleton” 的部分不承诺完整功能，只要求能按 README 的方式启动/演示。
- cli 不直接拥有/管理工具进程（由 hostd 负责 PTY/进程相关能力）。
- 事件类型作为稳定 API：不允许在无版本升级的情况下删除/重命名既有事件字段（仅允许兼容性新增字段）。

## hapi 功能对齐（Alignment Requirements）

本节把 `tiann/hapi` 的能力拆解为本项目的需求清单，并按阶段逐步达成。

### 对齐原则

- **单用户 self-host（local-first）**：默认假设为个人使用；无需 Happy 那种多用户/云端复杂度。
- **远程可控**：从手机/浏览器远程监控与控制本机/多台机器上的会话。
- **多后端**：至少支持 `codex`、`claude`、`gemini`、`iflow` 四类后端（可扩展）。
- **最小可信配置**：
  - 鉴权 token 可通过环境变量或本地配置文件设置；环境变量优先。
  - hostd ↔ server 使用 TOFU（Trust On First Use）：首次连接将 `host_id` 绑定到 `sha256(host_token)`；后续必须匹配。
  - 开箱即用：hostd 首次启动可自动生成并保存 `host_id/host_token`，用户只需配置 server 地址即可连接。

## happy 功能对齐（补充对齐清单）

本节用于把 `slopus/happy`（Happy Coder）的能力拆解为可执行需求清单，避免每次回看 GitHub。

> 注：Happy 的官方实现以多端客户端（React Native/Expo）为主，并包含端到端加密与跨端同步。
> 本项目当前阶段以 `hapi` 的“remote workflow layer + PWA 控制”路径为主，因此会把部分能力标为后续或 NON-GOAL。

### happy 的关键模型（我们应对齐的“概念”）

- **Session 是一等公民**：会话绑定 `machine + working directory`，围绕 session 管理 UI/状态/历史。
- **Tool system + 权限控制**：会话可以触发工具调用（fs/git/rg/bash 等），用户可审批/拒绝。
- **Machine 管理**：多机器在线状态、能力、远程操作（RPC）。
- **实时同步**：客户端能实时看到 session 的输出/状态变化（不要求照搬 Happy 的同步实现）。

### happy 对齐原则（本项目落地口径）

- **保持现有架构**：继续使用 `server/hostd` + WS fan-out + HTTP API；不引入 Happy 的 React Native 体系。
- **认证维持现状**：继续用 `/auth/login` + JWT；不引入 Happy 的多用户体系。
- **不做 E2E 加密**（后续可选）：当前 `events` 存储与消息模型只保证 redaction（避免 secrets 落库），不承诺端到端加密。

### happy 能力拆解（按模块）

#### H1：Session 模型（对齐 Happy 的 session-based model）

- 目标：
  - 将 “run” 提升为 “session” 的表现层模型：每个 session 关联 `host_id + cwd + tool`，并有可持续的消息线程（messages）。
  - session 列表/最近会话（recent）可在 web 中查看（MVP：按 started_at 排序）。
- 非目标：
  - 不实现 Happy 的跨端加密同步与离线合并策略。
- 验收（dev）：
  - `GET /runs` 之外，提供 `GET /sessions`（或等价）能列出 session（MVP 可复用 runs 表，但对外语义为 session）。
  - web 端能按 session 维度选择/切换，并展示对应 messages。

#### H2：Messages / Rendering（对齐 Happy 的 “chat interface + markdown”）

- 目标：
  - messages 线程展示（已具备基础），并支持基本的 markdown 渲染（MVP：代码块/链接）。
- 非目标：
  - 不对齐 Happy 的完整富文本/语法高亮栈；先保证可读、可用。
- 验收（dev）：
  - web 中 Messages 视图可读（至少：换行、code block 不挤压）。

#### H3：Tool system（对齐 Happy 的 tool execution）

- 目标：
  - 工具调用要有明确的“工具名/参数/结果/错误”结构（避免仅靠 output 文本解析）。
  - 工具权限：需要审批的工具调用产生 `permission_requested`，并可在 web 上 approve/deny。
  - Codex/Claude：通过 MCP（`relay mcp`，stdio）把工具调用转发到 hostd local API，形成 `tool.call/tool.result` 事件；其中写入与执行类操作需要 PWA 审批。
- 非目标：
  - 不引入 Happy 的插件系统；工具清单固定/可扩展即可。
- 验收（dev）：
  - 至少覆盖只读工具：`fs.read/fs.search/git.status/git.diff`（已有雏形：local + ws-rpc）。
  - 覆盖可审批工具：`rpc.fs.write` 与 `rpc.bash`（或等价），能在 web 的会话详情中 approve/deny，并在 approve 后继续执行、deny 后返回错误。
  - 工具调用具备可审计事件（events 中可追踪一次调用的 request_id 与结果）。

#### H4：Machine 管理（对齐 Happy 的 machine list + remote operations）

- 目标：
  - host 列表、在线状态、last_seen（已具备基础）；并扩展为“机器能力”可发现（MVP：支持哪些 tools）。
- 非目标：
  - 不做复杂的机器注册/租户管理。
- 验收（dev）：
  - `GET /hosts` 返回 `online + last_seen_at`，并额外包含 `capabilities`（MVP 可 hardcode 或由 hostd 上报）。

#### H5：Task/TODO（对齐 Happy 的任务管理/Zen）

- 目标：
  - web 端提供 per-session 的 todo 列表（已具备 localStorage 版本的基础），并允许从输出提取建议。
- 非目标：
  - 不做 server-side 多端同步（保持浏览器本地即可）。
- 验收（dev）：
  - web 端 todo 刷新后仍存在；可一键把 `TODO:` 建议加入列表。

#### H6：Sync/Encryption（Happy 的核心差异点）

- 当前状态：NON-GOAL（明确不做）。
- 后续可选：若未来要支持多端同步，优先考虑：
  - 明确消息/工具调用结构后再做同步；
  - 先做 server-side 同步（受控）再考虑端到端加密。

### 功能矩阵（按组件归类）

#### CLI（对齐 hapi CLI）

- **会话启动**：提供 `relay codex|claude|gemini|iflow`（或等价子命令）在本机启动会话，并自动注册/关联到 server（无需手写 curl）。
- **远程控制**：提供发送输入、停止会话等命令（对齐“手机端操作”与“命令行兜底”两种路径）。
- **鉴权管理**：提供 `auth status/login/logout`（或等价）用于配置/保存 token（环境变量优先）。
- **诊断与自检**：提供 `doctor`（或等价）输出版本、连接性、daemon 状态、日志位置、常见依赖检查。
- **后台守护（daemon）**：支持将长会话托管到后台进程，提供 start/stop/status/list/logs 等管理命令。
- **多后端 runner**：根据后端选择不同的启动/交互策略（至少保证 PTY 交互、输出上报、远程输入可用）。
- **MCP bridge（Codex/Claude）**：提供 stdio MCP server（`relay mcp`）把 Codex/Claude 的工具调用转发到 hostd local API；其中 `fs_write`/`bash` 等写入/执行类操作必须走 `run.permission_requested` 并在 web 中审批。

#### Server（对齐 hapi-server）

- **统一鉴权**：提供供 CLI 与 web 使用的鉴权入口（当前为 `/auth/login` + JWT；后续可补 token 直登/单用户模式优化）。
- **会话与机器管理**：可查询 runs/sessions 与在线机器（hosts），并具备“在某机器上发起新会话”的能力。
- **实时更新通道**：web 端能实时看到输出、状态变化、审批请求等（当前为 WS fan-out；后续可补更稳定的订阅/重连策略）。
- **持久化**：SQLite 持久化 runs/events/权限请求等必要状态，支持审计/回放最小信息。
- **安全边界**：默认不存 raw input；存储 redacted + sha256；禁止 secrets 进入日志/持久化。

#### hostd

- **多会话 PTY 托管**：为每个 run 创建 PTY，捕获 stdout/stderr，上报事件；接收远程输入写入 stdin。
- **离线/断线重连**：spool 事件持久化与 replay，支持 `run.ack` 清理已确认事件。
- **本地控制面**：提供本地 unix socket API（创建 run、输入、停止、列出运行中会话）。
- **权限审批（后续阶段）**：将需要用户确认的动作上报为可审批事件，并支持 remote approve/deny。
- **文件与 git 能力（后续阶段）**：为 web/cli 提供文件浏览、搜索、git status/diff 等（默认只读、严格路径限制）。

#### Web / PWA

- **远程监控**：runs 列表、状态、实时日志。
- **远程控制**：发送输入、停止会话；展示 awaiting_input/权限请求并提供 approve/deny。
- **日志查看（排障）**：在 web 中可查看 server 日志（`/server/logs/tail`）与 hostd 日志（`rpc.host.logs.tail`）。
- **文件与 git（后续阶段）**：浏览文件、查看 git diff、搜索文件内容。
- **todo（后续阶段）**：对会话输出/消息提取 todo 并可跟踪进度。

## 功能需求（Functional Requirements）

### server（Rust）

- 提供认证（auth）。
- 提供路由（routing）。
- 使用 SQLite 作为存储（storage）。
- 提供 WebSocket：
  - 支持 PWA 连接。
  - 支持 hostd 连接。
- 提供会话消息 API（用于 web 的结构化消息视图）：`GET /sessions/:id/messages?limit=200`
  - 返回按时间排序的消息列表（MVP：events 表的视图）。
  - 对 `tool.call` / `tool.result` / `run.permission_requested` 等事件，响应中可选携带结构化 `data` 字段（来自 events.data_json 的解析结果）。
- 提供 server 日志 tail（排障）：`GET /server/logs/tail?lines=200&max_bytes=200000`（需要 JWT；默认 docker 写入 `/data/relay-server.log`，可用 `SERVER_LOG_PATH` 覆盖）。
- 提供密码哈希生成能力（Argon2），用于创建可登录的凭据（见 README 的 `--hash-password` 用法）。

### hostd（Rust）

- 作为宿主机守护进程：
  - 运行 PTY 交互进程（PTY runner）。
  - 以出站方式通过 WebSocket 连接 server（outbound WS）。
  - 支持 `run.stop` 的 `signal`：`int | term | kill`（其中 `int` 语义等价“Ctrl+C 中断”，用于取消当前生成/操作；`term/kill` 用于结束进程）。
  - Codex 运行模式（可选增强）：
    - 支持通过 `RELAY_CODEX_MODE=tui|structured|auto` 控制 codex 启动方式（默认 `tui`）。
    - `structured`：使用 `codex mcp-server`（兼容探测 `codex mcp serve`）启动 Codex MCP server，并通过 MCP `tools/call`（`codex` / `codex-reply`）驱动会话；输入仍走 `run.send_input`（按行作为 prompt）。
    - `auto`：自动探测 Codex MCP server 的启动参数并持久化到 `~/.relay/tool-mode-cache.json`；默认满足“运行 5 次或 24 小时”触发再次探测（可用 `RELAY_TOOL_MODE_AUTO_RUNS` / `RELAY_TOOL_MODE_AUTO_TTL_SECS` 覆盖）。
    - 探测超时可通过 `RELAY_CODEX_PROBE_TIMEOUT_MS` 配置（默认 5000ms）。
  - OpenCode 运行模式（MVP，结构化消息优先）：
    - 支持通过 `RELAY_OPENCODE_MODE=structured|tui` 控制 opencode 启动方式（默认 `structured`）。
    - `structured`：
      - hostd 在收到 `run.send_input` 后调用 `opencode run --format json`（必要时带 `--session <id>` 续聊）。
      - 解析 opencode 的 JSONL 事件并映射为 relay 事件：
        - `text` → `run.output`（markdown 原文）
        - `tool_use` → `tool.call` + `tool.result`（args/result 存在 `data_json`，供 web 富渲染）
        - `error` → `run.output`（stderr）
      - 为避免非 TTY 下的交互式权限提示，hostd 默认注入 `OPENCODE_PERMISSION={"*":"allow"}`（若用户已显式设置则不覆盖）。
        - 可通过 `RELAY_OPENCODE_PERMISSION_MODE=inherit` 禁用注入，仅继承环境变量（默认：auto allow-all，保持兼容）。
      - `run.started`（opencode structured）需携带权限模式元信息（用于 web 展示，不提供编辑）：
        - `permission_env_set`：布尔值，表示是否用户显式设置了 `OPENCODE_PERMISSION`。
        - `permission_mode`：`env | relay_auto_allow_all | inherit`。
      - 结构化事件的敏感信息处理：
        - `tool.call.args` 与 `tool.result.result.raw_part/title/output` 需经过 redaction 后再进入事件流/落库（避免 secrets 泄露）。
        - `tool.result.duration_ms` 可选；未知时不应伪造为 0。
      - `run.stop`：
        - `signal=int`：尝试 SIGINT 当前 opencode 子进程（取消当前生成），run 继续可输入。
        - `signal=term|kill`：结束 run 并发出 `run.exited`。
    - `tui`：按 PTY 方式启动（类似直接在终端里运行 opencode），输出以 xterm.js 渲染为主。
    - 二进制路径解析：支持 `RELAY_OPENCODE_BIN=/path/to/opencode` 与 shims 的 `~/.relay/bin-map.json`。
- 事件 spooling 与重放：
  - 将待发送事件持久化到本地 SQLite spool DB。
  - 断线后重连时重放未送达事件。
  - 可通过 `SPOOL_DB_PATH` 配置 spool DB 路径（默认：`data/hostd-spool.db`）。

### cli（Bun）

- 作为本地命令包装层，通过 hostd 启动 runs。
- 提供登录能力（示例：`bun run dev login ...`）。
- 提供通过 WebSocket 发送远程输入能力（示例：`bun run dev ws-send-input ...`）。
- 安装体验（macOS/Linux）：
  - `npm i -g @aipper/relay-cli` 在 `postinstall` 阶段 best-effort 下载并安装 `relay-hostd` 到 `~/.relay/bin`（失败不阻断安装，可手动 `relay hostd install`）。
  - `--yes/-y` 或 `RELAY_YES=1` 可跳过确认（用于自动安装/提示场景）。
  - Linux（systemd）：打包目录内置的 `client-init.sh` / `install-hostd-systemd-user.sh` 安装 user service 时，应能兼容 `XDG_CONFIG_HOME` 与 systemd user manager 环境不一致的场景（避免 `Unit relay-hostd.service does not exist`）。
- 常驻体验：
  - 运行 `relay codex/claude/iflow/gemini` 或本地 `relay runs/fs/git/local ...` 时，若本地 unix socket 不存在则自动拉起 `relay-hostd`（等价 `relay daemon start`）并等待就绪。
- 开箱即用（个人使用）：
  - `relay init --server http://<vps>:8787 --start-daemon` 可一条命令完成“写入 server 配置 + 常驻启动 hostd”（也可通过 `RELAY_START_DAEMON=1` 启用）。
  - Linux（systemd）：`relay init --server http://<vps>:8787 --install-systemd-user` 可一条命令完成“写入 server 配置 + 安装并启用 systemd user service”（也可通过 `RELAY_INSTALL_SYSTEMD_USER=1` 启用）。

### web（Svelte PWA）

- 提供移动端友好的 UI。
- 支持：
  - runs 列表与状态查看；
  - 实时日志（live logs）；
  - 审批与输入流转：approve/deny/input。
- 交互与信息展示（MVP）：
  - 登录成功后将 `token`（JWT）与用户名持久化到 localStorage；刷新页面不退出（可关闭）。
  - 提供“记住密码（本机）”开关；开启后将密码存入 localStorage，并在 token 失效时可自动重登（个人使用；共享设备不建议开启）。
  - 认证失败（HTTP 401 / token 失效）时自动清除 token 并回到登录页，提示“登录已过期”。
  - 首页以 session 维度展示列表（语义对齐 runs），待审批时每条显示“等待原因 + 会话工具/模型（run.tool）+ 待审批操作工具名（op_tool）”。
  - 列表仅展示状态，不提供 approve/deny，避免误操作。
  - 会话详情以消息流为主视图，并显示 `run_id`、`status`、`host_id`、`cwd`、host 在线状态（圆点 + 文本）、工具/模型（简单 SVG 图标，品牌色）、权限模式元信息；不展示 host 名称。
  - 工具/模型图标不提供点击说明。
  - 权限模式在详情页仅展示，不提供编辑。
  - 会话详情不展示开始/结束时间。
  - 会话详情显示最后活动时间（相对时间）。
  - 会话详情的状态标签颜色与列表保持一致。
  - 手机单列模式下，详情页提供“返回列表”按钮。
  - 输出页提供“全屏查看”按钮（横屏或大屏使用），全屏采用当前页覆盖层而非新路由，并提供“退出全屏”按钮；不支持 ESC/手势关闭；不隐藏顶部状态与 tabs。
  - 会话详情不提供复制 run_id/cwd 的快捷按钮。
  - 会话详情提供手动刷新消息/输出的按钮。
  - 会话详情空消息时不显示提示文案。
  - WebSocket 断开时，会话详情显示离线提示条，并提供“重连/刷新”按钮。
  - 会话处于错误状态时，详情页显示错误原因摘要（优先：exit_code + 最后 stderr 片段，stderr 截断 200 字符）。
  - 会话详情提供“停止会话”按钮（需要二次确认）。
  - 会话详情提供“中断（Ctrl+C）”按钮（不需要二次确认），用于发送 `run.stop signal=int`，尽量只中断当前生成而不结束会话。
  - 待审批时，以弹窗/浮层方式展示 approve/deny，弹窗展示会话工具/模型（run.tool）+ 待审批操作工具名（op_tool），并提供“查看完整参数”展开区与风险提示（读/写/执行类型）。
  - 风险类型可基于待审批操作工具名（op_tool）的静态映射（例如 `rpc.fs.read`=读，`rpc.fs.write`=写，`bash`=执行；允许带/不带 `rpc.` 前缀）。
  - 输入默认通过按钮触发弹出输入框；在“消息”tab 下提供底部输入框（Enter 发送 / Shift+Enter 换行），并保留“更多/弹窗”入口用于多行编辑。
  - 输入弹窗提供快捷输入按钮（固定：`y` / `n` / `continue`）。
  - 输入弹窗不保存输入历史。
  - 会话列表按机器分组展示；分组标题显示“在线状态圆点 + 机器名（缺失时用 host_id） + last_seen（始终显示）”。
  - 机器在线圆点不提供 tooltip。
  - 机器分组支持折叠/展开。
  - 机器分组折叠状态需要持久化（刷新后保持）。
  - 不提供“全部折叠/全部展开”快捷操作。
  - 分组内按最近活跃时间降序排序。
  - 会话状态展示为“文字 + 颜色标签”，包含：运行中 / 待审批 / 待输入 / 已结束 / 错误。
  - 错误判定：`exit_code != 0` 或出现 `tool.result ok=false`。
  - 会话列表仅在待审批时显示会话工具/模型（run.tool，品牌色 SVG 图标）与待审批操作工具名（op_tool）；`cwd` 仅在宽屏展示。
  - 会话列表覆盖全部历史会话，支持滚动触底加载更多（每页 20 条）。
  - 加载更多时显示加载中指示。
  - 会话列表空状态不显示文案（留空）。
  - 会话列表支持搜索/过滤（仅按会话标题/摘要；若无标题/摘要，临时按 `run_id` 过滤）。
  - 会话列表优先显示会话标题（若 server 提供）；无标题时不显示占位。
  - 会话列表显示摘要（优先使用 server 提供的摘要，最多 60 字符；缺失时不显示；待审批时隐藏摘要）。
  - 会话列表不单独显示“待输入”提示（由状态标签表达）。
  - 会话列表不展示 run_id。
  - 会话列表显示最后活动时间（相对时间），放在摘要下方小字。
  - 会话列表高亮当前选中会话（背景色 + 左侧彩色条）。
  - 待审批状态使用警示色（黄/橙系）。
  - 待输入状态使用橙色提示。
  - 错误状态使用红色提示。
  - 运行中状态使用绿色提示。
  - 已结束状态使用灰色提示。
  - 状态标签不提供 tooltip。
  - 会话列表不显示未读更新提示。
  - 顶部显示连接状态（圆点 + 文字）；WebSocket 断开时启用 10s 轮询刷新 runs/hosts。
  - 会话详情提供“事件 / 终端”顶部标签切换；默认进入“事件”视图；终端仅用于查看原始上下文。
  - 响应式断点：<=640（手机单列，列表→详情全屏切换）；641–1024（平板双栏，列表+详情）；>=1025（桌面双栏，列表宽度固定 320–360px）。
  - 输出页保留最近内容按可视高度动态裁剪：`visibleLines = floor(viewHeight / lineHeight)`，`bufferLines = clamp(visibleLines * 4, 200, 2000)`。
  - 输出页自动滚动暂停时显示“跳到最新”按钮。
  - 输出页提供“暂停/继续”自动滚动的显式控制按钮。
  - 输出页默认处于“继续（自动滚动）”状态。
  - 点击“跳到最新”会同时恢复自动滚动。
  - 自动滚动关闭时显示“已暂停”状态提示（输出区域右上角），点击可恢复自动滚动。
  - 输出页暂停自动滚动后，从最后一次手动滚动起 10 秒无动作则自动恢复（即使通过“暂停”按钮触发）；自动恢复不提示。
  - 输出页空内容不显示提示文案。
  - 输出页提供“复制全部输出”按钮。
  - 复制输出成功后显示简短 toast 提示。
  - 输出页不提供下载为文件功能。
  - 对 TUI 工具（`codex/claude/iflow/gemini`），输出页默认以“终端屏幕快照（方案 B v1）”展示：只展示当前屏幕（默认 80x24），不追加日志；不提供历史回看（后续再做 scrollback/transcript）。
  - 输出页提供搜索功能（仅本页搜索，不请求后端）。
  - 输出搜索支持高亮命中。
  - 输出搜索提供“上一处/下一处”跳转。
  - 输出搜索为大小写不敏感匹配，不提供切换。
  - 输出搜索不支持正则表达式。
  - 输出搜索显示命中数量（如 3/12），搜索框在输出页顶部，并带清空按钮。
  - 输出搜索框不显示占位提示文本。
  - 输出搜索高亮颜色为橙色。
  - 当前命中使用更深色强调。
  - 清空搜索不自动滚动回顶部。
  - 输出搜索不做实时匹配，需手动触发（按钮或 Enter 均可）；Enter 不用于跳转下一处。
  - 输出搜索支持键盘快捷键切换上一处/下一处（↑/↓），搜索框聚焦时仅用于搜索跳转。
  - 进入输出页时搜索框自动聚焦（含手机端），不影响自动滚动。
  - 待审批时，消息流内插入“审批请求卡片”（与弹窗并存）；卡片展示会话工具/模型（run.tool）+ 待审批操作工具名（op_tool）+ 参数摘要（最多 80 字符） + 发起时间。
  - 事件流以“turn（用户输入）→ parts（工具/系统/assistant 输出）”的结构展示（对齐 opencode share 的观感）；`tool.call/tool.result` 优先展示结构化 JSON（来自 messages API 的 `data` 字段），并可展开查看详情。
  - 事件视图默认不展示海量 `run.output`（避免淹没结构化事件）；默认仅对 `tool=opencode` 的会话保留 `run.output` 作为结构化 text 的承载。
  - 事件视图顶部提供“输出摘要（tail）”与“打开终端”入口；TUI 输出默认仅在终端视图查看。
  - 消息流区分用户/助手/系统角色的视觉样式（用户右对齐、助手左对齐、系统居中；系统消息更小字号与弱色，且无气泡仅文本；用户/助手使用气泡背景；用户用品牌色，助手用中性灰；用户/助手/系统均显示时间戳，格式为绝对时间，位置在气泡下方小字；气泡最大宽度 70%；长文本自动换行并保留换行）。

## 配置与安全（Configuration & Security Requirements）

- 禁止提交 secrets；运行时配置放在 `conf/env`（或 `.env`）并确保被 `.gitignore` 忽略。
- 日志默认短保留（3 days）。
- 输入（inputs）必须以 **redacted** 形式存储；默认关闭 raw input 存储。

## 构建/开发/测试（Build & Test Requirements）

- Rust workspace 支持：
  - `cargo fmt` / `cargo fmt --check`
  - `cargo clippy --all-targets --all-features`
  - `cargo test`
- CLI（Bun）支持：
  - `cd cli && bun install`
  - `cd cli && bun run dev`（若提供）
- Web（Svelte PWA）支持：
  - `cd web && bun install`
  - `cd web && bun run dev`
  - `cd web && bun run build`

## 验收标准（Acceptance Criteria）

以下验收项以“可运行、可验证”为最低标准（不额外扩展未在来源文档中声明的行为）。

- 能按 `README.md` 启动 `relay-server`（在 `conf/env` 配置完成后）。
- 能按 `README.md` 启动 `relay-hostd` skeleton，并连接到 server（通过 `SERVER_BASE_URL` 等环境变量配置）。
- 能按 `README.md` 启动 web skeleton（`cd web && bun run dev`）。
- 能按 `README.md` 方式使用 cli 完成：
  - `login` 获取 token；
  - `ws-send-input` 发送远程输入到指定 `run_id`。
- `hostd` spool replay 行为满足 README 的 E2E smoke test：
  - `scripts/e2e.sh` 能运行完成；
  - 对同一 `input_id` 发送两次输入时，在 SQLite 中满足幂等性断言（idempotency）。

## 分期里程碑（Roadmap & Acceptance）

本节用于把“对齐 hapi 的完整能力”拆成可在 30–60 分钟内推进的小迭代，并明确每期验收命令。

### M0（已具备）：最小闭环

- 目标：server + hostd + CLI 发送输入 + DB 幂等断言跑通。
- 验收：
  - `scripts/test.sh`（含 `scripts/e2e.sh`）通过；
  - `scripts/dev-up.sh --port 8787` 能拉起服务，CLI 可 `login`。

### M1（CLI 可用性）：一条命令启动会话

- 目标：CLI 不依赖手写 curl，即可创建 run 并打印 `run_id`；并支持发送输入/停止。
- 验收：
  - `scripts/dev-up.sh --port 8787`
  - `cd cli && bun run src/index.ts codex --sock ./.relay-tmp/relay-hostd-dev-8787.sock --cmd "echo Proceed?; cat"` 返回 `run_id`
  - `cd cli && bun run src/index.ts ws-send-input --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --text $'y\\n'` 输出 `sent`
  - `cd cli && bun run src/index.ts ws-stop --server http://127.0.0.1:8787 --token <jwt> --run <run_id>` 输出 `sent`

### M1-A（本轮优先）：项目目录直接运行 codex/claude/iflow

- 目标：对齐 hapi 的“在项目目录直接运行 codex/claude/iflow”体验：
  - 安装 shim 后，在任意项目目录执行 `codex` 会通过 relay/hostd 启动真实 codex 会话，且 `cwd=$PWD`。
- 关键约束（防递归）：
  - shim 会覆盖系统上的同名命令（这是预期行为）。
  - hostd 必须执行“真实二进制绝对路径”而不是 shim，否则会递归创建 run。
  - 因此安装 shim 时会记录真实路径到 `~/.relay/bin-map.json`，hostd runner 会优先读取该映射。
- 验收（dev）：
  - 启动：`scripts/dev-up.sh --port 8787`
  - 安装 shim：`bash scripts/install-shims.sh`
  - PATH 就绪后（例如 `export PATH="$HOME/.local/bin:$PATH"`），在任意项目目录执行：
    - `RELAY_HOSTD_SOCK=./.relay-tmp/relay-hostd-dev-8787.sock codex`
    - 期望：输出 `run-...`（run_id），并可在 web/cli 中看到该 run 的 output/awaiting/approve 等事件
  - 回滚：`bash scripts/install-shims.sh --uninstall`

（可选增强）安装 shim 同时自动写入 PATH：
- `bash scripts/install-shims.sh --auto-path`

### M2（Web 可用性）：远程查看 + 输入闭环

- 目标：web/PWA 具备登录、runs 列表、run 详情实时日志、输入框发送输入。
- 验收：
  - `cd web && bun install && bun run dev`（首次需要依赖安装）
  - 浏览器中可看到 run.output；输入后 run 有响应
  - web 可通过 `rpc.run.start` 在指定 host 上启动新 run，并自动切换到该 run（远程发起会话）
  - server 可列出在线 hosts（用于 web 下拉选择）：
    - `curl -s http://127.0.0.1:8787/hosts -H "Authorization: Bearer <jwt>"`
    - 结果中包含 `host-dev`，且 `online=true`（在 `scripts/dev-up.sh` 运行中）

### M3（权限审批）：approve/deny

- 目标：hostd 产生可审批事件，web/cli 能 approve/deny 并驱动会话继续或停止。
- 验收：
  - 触发审批场景（由 demo 命令或后端 runner 触发）
  - web 端收到请求并 approve/deny 生效
  - CLI 端可对指定 `request_id` 执行 approve/deny：
    - `cd cli && bun run src/index.ts ws-approve --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --request-id <uuid>`
    - `cd cli && bun run src/index.ts ws-deny --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --request-id <uuid>`

### M4（工作流增强）：文件/git/todo + daemon/doctor

- 目标：提供 hapi 风格的“远程工作流层”能力（文件、diff、todo、诊断与守护）。
- 验收：
  - CLI `doctor` 输出诊断信息（版本、连接性、日志路径）
  - daemon 可托管会话并列出/停止
  - web 端能浏览文件/查看 diff（默认只读）并展示 todo（MVP 提取）

#### M4-B（本轮优先）：daemon + doctor

- 验收（dev）：
  - `cd cli && bun run src/index.ts daemon start --server http://127.0.0.1:8787 --sock ./.relay-tmp/relay-hostd.sock`
  - `cd cli && bun run src/index.ts daemon status` 显示 `running=true`
  - `cd cli && bun run src/index.ts doctor` 显示 `ok=true`（至少 server.health/hostd.sock/hostd.api 通过）
  - `cd cli && bun run src/index.ts daemon logs` 输出日志路径
  - `cd cli && bun run src/index.ts daemon stop` 后 `daemon status` 显示 `running=false`

#### M4-A1（本地工作流能力）：fs/git（仅 run.cwd）

- 目标：提供本地只读的文件读取/搜索与 git status/diff（用于后续 web 远程查看的基础能力）。
- 安全边界：
  - 所有路径必须为相对路径，且必须落在该 `run_id` 的 `cwd` 目录内（禁止越界）。
- 验收（dev）：
  - 先启动一个在项目目录下的 run（`cwd` 指向你的 repo 目录）：
    - `cd cli && bun run src/index.ts codex --sock ./.relay-tmp/relay-hostd.sock --cmd "echo ready; cat" --cwd "$(pwd)"`
  - 读取文件：
    - `cd cli && bun run src/index.ts fs read --sock ./.relay-tmp/relay-hostd.sock --run <run_id> --path README.md`
  - 搜索（需要 `rg`）：
    - `cd cli && bun run src/index.ts fs search --sock ./.relay-tmp/relay-hostd.sock --run <run_id> --q "relay"`
  - git 状态/差异（需要 cwd 是 git repo）：
    - `cd cli && bun run src/index.ts git status --sock ./.relay-tmp/relay-hostd.sock --run <run_id>`
    - `cd cli && bun run src/index.ts git diff --sock ./.relay-tmp/relay-hostd.sock --run <run_id>`

- 会话列表（用于 daemon/doctor 与后续 UI）：
  - `cd cli && bun run src/index.ts runs list --sock ./.relay-tmp/relay-hostd.sock`

#### M4-A2（远程工作流能力）：web 通过 WS-RPC 查看 fs/git（仅 run.cwd）

- 目标：web/PWA 可通过 server→hostd 的 WS-RPC 读取文件、搜索、查看 git status/diff（能力基于 M4-A1，但由 web 远程触发）。
- 协议：见 `docs/protocol.md` 的 `WS-RPC (M4-A2)`。
- 安全边界：
  - 所有路径必须为相对路径，且必须落在该 `run_id` 的 `cwd` 目录内（禁止越界）。
  - 验收（dev）：
  - 启动 server/hostd：`scripts/dev-up.sh --port 8787`
  - 启动 web：`cd web && bun run dev -- --host 127.0.0.1 --port 5173`
  - 创建一个 `cwd` 指向 git repo 的 run（例如用 CLI `codex --cwd "$(pwd)"`）
  - web 页面中：
    - `Files` 输入 `README.md` 能读出内容
    - `Git` 点击 `Status`/`Diff` 能展示输出（在 git repo 内）

#### M4-TODO（会话进度）：todo 列表（web）

- 目标：web/PWA 提供每个 run 的 todo 列表，用于跟踪会话进度；支持手动维护，并提供基于 `run.output` 的 todo 建议提取。
- 约束：
  - 默认仅保存在浏览器（localStorage），不写入 server DB（后续如需多端同步再扩展）。
  - 验收（dev）：
    - 启动：`scripts/dev-up.sh --port 8787` + `cd web && bun run dev -- --host 127.0.0.1 --port 5173`
    - 在 web 选择某个 `run_id`：
      - 可手动新增 todo、勾选完成、删除
      - 刷新页面后 todo 仍存在（localStorage 持久化）
      - 在 output 出现 `TODO:` 或 `- [ ] ...` 风格文本时，`Suggestions` 能出现并可一键加入

### M5（会话/消息模型）：sessions + messages（对齐 hapi 的聊天线程）

- 目标：server 提供可分页的 messages API，web 用“消息线程”展示（而不是只拼接 output），并能在消息级别进行 approve/deny/input。
- 约束：
  - 不引入 Telegram。
  - 不强制切换到 hapi 的 Socket.IO/SSE 体系；允许继续使用现有 WS fan-out + HTTP API（后续如需再补）。
- 验收（dev）：
  - 启动：`scripts/dev-up.sh --port 8787 --rust-log info`
  - 登录拿到 `<jwt>`：`cd cli && bun run src/index.ts login --server "http://127.0.0.1:8787" --username admin --password "123456"`
  - 创建 run：`cd cli && bun run src/index.ts codex --sock ./.relay-tmp/relay-hostd-dev-8787.sock --cmd "echo hello; cat"`
  - messages API（示例）：
    - `curl -s "http://127.0.0.1:8787/runs/<run_id>/messages?limit=50" -H "Authorization: Bearer <jwt>"`
    - 期望：返回 JSON 数组，至少包含 assistant（output）与 user（input，redacted）两类消息
  - web 中选择 `<run_id>` 后可看到消息按“user/assistant/system”分段展示

### M6（后端适配）：codex（优先）+ 可测试 mock

- 目标：把 `codex` 作为一等公民后端：
  - hostd 对 `tool=codex` 走专用 runner（最少：启动命令/环境/输出解析/权限请求触发策略可独立演进）。
  - 提供仓库内置的 `mock-codex`，用于 e2e 回归（不依赖用户是否安装真实 codex）。
- 约束：
  - 真实 codex 的可用性依赖用户机器安装/配置；e2e 以 mock 为主，真实集成作为可选验收。
- 验收：
  - `bash scripts/test.sh --fast`：通过
  - `bash scripts/e2e.sh`：通过（包含 mock-codex 场景）
  -（可选，若本机已安装 codex）启动真实 codex 会话，并能远程 approve/deny/input：
    - 启动：`scripts/dev-up.sh --port 8787`
    - 启动 run：`cd cli && bun run src/index.ts codex --sock ./.relay-tmp/relay-hostd-dev-8787.sock`
    - 如 codex 不在 PATH：在 hostd 环境里设置 `RELAY_CODEX_BIN=/path/to/codex`

### M7（快速分发/安装）：可分发目录 + 一键启动（不做单可执行）

- 目标：提供“快速可用”的分发路径（先对齐 hapi 的体验收益，不追求同款三阶段单可执行）：
  - `scripts/package.sh` 生成一个可分发目录（含 `relay-server`、`relay-hostd`，以及可选 web 静态资源）。
  - 分发目录内提供 `up.sh` 一键启动 server+hostd（默认密码 `123456`）。
- 非目标：
  - 不实现 hapi 的 npm optionalDependencies 多平台自动下载；当前采用 `npm postinstall` best-effort 下载 `relay-hostd`（仅 darwin/linux），其他平台后续再补。
  - 不要求构建过程零依赖；web 资源允许在无依赖时跳过并提示。
- 验收（dev）：
  - 打包：`bash scripts/package.sh`
  - 在输出的分发目录中启动：`bash ./dist/<package>/up.sh --port 8787`
  - 期望：`curl -s http://127.0.0.1:8787/health` 返回 ok；并看到 host `host-dev` 在线：`curl -s http://127.0.0.1:8787/hosts -H "Authorization: Bearer <jwt>"`
  - 分发目录内可直接使用 Rust CLI 启动会话（无需 bun/node）：
    - `./dist/<package>/bin/relay codex --sock ./dist/<package>/data/relay-hostd.sock --cwd /path/to/project`
    - 期望：输出 `run_id`（如 `run-...`），并可在 web 或 ws-send-input 中操作该 run

## 可执行验证清单（Verification Checklist）

说明：以下命令均来自本仓库已存在脚本/配置，作为“每轮迭代必须可验证”的最低标准。

### 代码质量（Rust）

- `cargo fmt --check`：退出码 0
- `cargo clippy --all-targets --all-features`：退出码 0（允许 warning，但不应失败）
- `cargo test --workspace`：退出码 0

### 端到端（E2E）

- `scripts/e2e.sh`：输出包含 `"[e2e] ok: input idempotency"` 且退出码 0
-（保留日志排错）`KEEP_TMP=1 RUST_LOG=info bash scripts/e2e.sh`：保留临时目录并生成 `server.log/hostd.log`

### 开发演示（Dev）

- 启动：`scripts/dev-up.sh --port 8787 --rust-log info`
- 健康检查：`curl -s http://127.0.0.1:8787/health`
- 在线 hosts（需先登录获取 `<jwt>`）：
  - `curl -s http://127.0.0.1:8787/hosts -H "Authorization: Bearer <jwt>"`
- CLI 登录（示例，勿在日志/文档里硬编码生产密码）：
  - `cd cli && bun run src/index.ts login --server "http://127.0.0.1:8787" --username admin --password "123456"`
- CLI 一条命令启动 run（M1）：
  - `cd cli && bun run src/index.ts codex --sock ./.relay-tmp/relay-hostd-dev-8787.sock --cmd "echo Proceed?; cat"`

### Web（M2）

- 依赖安装：`cd web && bun install`
- 本地启动：`cd web && bun run dev -- --host 127.0.0.1 --port 5173`
- 构建：`cd web && bun run build`
  - 如遇到 PWA service worker 生成不兼容，可临时禁用：`RELAY_DISABLE_PWA=1 bun run build`
  - 如需强制启用 PWA：`RELAY_FORCE_PWA=1 bun run build`

## 回滚方案（Rollback）

- CLI 新增能力回滚：还原 `cli/src/index.ts` 到变更前版本。
- e2e 改动回滚：还原 `scripts/e2e.sh` 中创建 run 的方式（从 `bun ... codex` 改回原先 `curl --unix-socket .../runs`）。

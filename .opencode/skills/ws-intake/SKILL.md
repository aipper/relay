---
name: ws-intake
description: 使用时机：新需求需要逐条澄清、冻结问题时。触发词：需求澄清、intake、冻结问题、前期沟通。注意：需求已冻结直接进 ws-plan。
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在进入 `/ws-plan` 前，把新需求或中大型变更里的待确认问题逐条澄清并冻结。
- 采用“一题一线程”模式推进：每次只处理 1 个问题，允许该问题多轮往返，直到形成明确结论。
- 产出一份可被 `/ws-plan` 消费的轻量草案：`plan/<timestamp>-<slug>.intake.md`。

## Deep Interview 层（前置探询）

在逐题澄清前，先做一轮 Deep Interview 收集高维信息：

### 1. Why 探询
先问"为什么要做这个？背后的业务/用户价值是什么？"——理解动机后再谈方案。延伸探询：
- "如果不做这个，会有什么后果？"（量化不做的代价，确认 urgency）
- "这个需求最早是谁提出的？在什么场景下触发的？"（追溯原始 trigger，避免需求被转述变形）

### 2. 非目标（Non-goals）
显式记录什么不在本次范围内——防止 scope creep 和后续争论。
例如："这次不做用户权限管理，只做内容展示层""这个版本不做国际化"。
非目标与目标同等重要，需用户确认后写入 intake 草案。

### 3. 影响面（Stakeholders）
识别这个改动会影响谁：
- 终端用户（使用行为是否会变？）
- 其他模块或系统（API 耦合、数据依赖）
- 其他团队（是否需要跨团队协调？谁需要参与评审？）

### 4. 假设显式化
从当前理解中识别隐含假设，逐条列出并请用户确认。例如：你假设了用户已登录/有权限；你假设了数据规模不超过 X；你假设了这个功能只有 Y 场景用到。

### 5. 替代方案
在选定方案前，先问"是否考虑过其他方案？为什么选了当前这个？"——记录已探明的替代路径和淘汰理由。
如果用户明确没考虑过替代方案，在 intake 草案中标记 `Alternatives not explored` 作为风险项。

### 6. 约束挑战
对每条约束问"如果这条约束不存在会怎样？"——区分硬约束（不可变）和自设约束（可协商）。

### 7. 优先级
Must-have / Should-have / Nice-to-have 三层分类，明确 scope 底线。

### 8. 成功度量
不仅仅是"验收标准通过"，更要问"这个功能上线后，怎么判断它成功了？"——量化指标（DAU、转化率、响应时间、错误率等）。如果无法量化则记入风险。

### 9. 风险预判
识别 3-5 个最关键风险（技术方案、时间窗口、外部依赖、安全合规），写入 intake 草案。

输出：以上 9 项归入 intake 草案的 `Deep Interview` 小节。

## 发散-收敛两子阶段

当需求模糊、方向不明确时，intake 分两子阶段推进：

### 发散阶段（Explore）
- 目标：快速探索 2-3 个可能方向，不深入任何单一方向
- 输出：每个方向的 1-2 段摘要 + 关键风险 + 依赖
- 时限：不超过 3 轮对话
- **探码前问**：在向用户提问前，先使用 `explore` agent 探查代码库中是否已有答案。需要探查的维度包括：
  - 现有配置文件（`AI_PROJECT.md`、`REQUIREMENTS.md`、`AI_WORKSPACE.md`、`package.json`、`tsconfig.json` 等）
  - 已有实现模式（grep 关键词、查找类似功能的实现文件）
  - 已有文档或注释（相关模块的 README、代码注释）
  - 已有测试文件（理解测试模式和边界条件）
- 探查到的信息直接作为发散阶段的输入，不需要再向用户重复确认已经在代码库中找到的事实。

### 收敛阶段（Converge）
- 目标：从发散结果中选择 1 个方向，逐条冻结具体问题
- 回到标准"一题一线程"模式
- 把发散产出的摘要作为收敛的输入上下文

触发条件：用户说"不确定"/"多个方向"/"帮我分析" 等模糊表达。
非模糊需求不需要经过发散阶段，直接进入收敛。

## 核心原则：一次一个问题

- 每轮只推进 1 个 `Open Questions` 中的问题
- 当前问题未标记 `frozen` 或 `deferred` 前，不进入下一题
- 输出格式：`Current question → Why it matters → Options → Exit condition`
- 用户选择后立即写盘更新草案，再进入下一题

**队列保护**：若用户一次提出多个问题，必须显式建队（Queue），逐一处理——不可同时推进多个维度。队列格式：
- Queue: [Q1: ..., Q2: ..., Q3: ...]
- Current: Q1
- Status: [open/in_discussion/frozen/deferred] per item

执行要求：
1) 先读 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`，必要时先 `/ws-preflight`。
2) 若存在最新 `plan/*.intake.md`，先续写它；否则新建一份 intake 草案。
3) 把当前任务拆成 `Open Questions`，状态只允许 `open / in_discussion / frozen / deferred`。

4) **探码后问**：在对每个 Open Question 提问前，先 spawn `explore` agent 探查代码库。如果代码库中已有足以回答该问题的信息，在 intake 草案的相应章节记录该信息，并跳过该问题的提问。只有当代码库无法回答时，才把该问题交给用户。记录每个问题属于以下哪一类：
   - `codebase: answered` — 代码库已可回答，跳过提问
   - `codebase: partial` — 代码库有部分信息，补充提问未覆盖的部分
   - `user: required` — 必须问用户

5) 每次只推进 1 个当前问题，并显式输出：
   - `Current question:`
   - `Why it matters:`
   - `Current options / current understanding:`
    - `Exit condition:`
- 每个问题独立线程：在未标记 frozen/deferred 前，不进入下一题。问答往返不限轮数，但一次只处理一个维度的决策。
6) 若用户一次提出多个问题，必须显式排队：列出所有问题编号，按顺序逐个处理；不得同时推进多个问题的讨论。输出格式：`Queued questions: #1, #2, ...  | Current: #1`
7) 每轮都要把 intake 草案写盘，至少包含：
   - `Deep Interview` — 9 维分析：Why/非目标/影响面/假设/替代方案/约束/优先级/成功度量/风险
   - `Context`
   - `Codebase Knowns` — 来自代码库的已知信息：在提问前通过 explore 已确认的事实、现有模式、配置值等。标注每项的信息来源路径
   - `Open Questions`
   - `Resolved Questions`
   - `Frozen Decisions`
   - `Draft Scope`
   - `Draft Verify`
   - `Ready for ws-plan: yes/no`
8) 错误状态节点（Error States）：
   - 在 intake 草案中专设 `Error States` 小节，覆盖已知失败模式：
     - 网络/超时/第三方依赖不可用时的系统行为
     - 数据一致性（并发写入、部分失败、脏数据）
     - 输入校验边界（空、超大、非预期类型）
   - 若识别到需要回滚的场景：在 `Error States` 中写明回滚条件与回滚方式。
9) 回滚规范（Rollback Spec）：
   - 当 intake 涉及现有数据迁移、API 契约变更、配置漂移修复时，必须包含 `Rollback Plan` 小节。
   - `Rollback Plan` 至少包含：触发条件、回滚步骤、验证回滚成功的方式、副作用清单。
10) 若关键问题已冻结：`Next` 指向 `/ws-plan`；否则继续 `/ws-intake`。

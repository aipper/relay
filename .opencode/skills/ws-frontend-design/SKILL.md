---
name: ws-frontend-design
description: 使用时机：需要前端设计、UI/UX 实现时。触发词：设计、前端、UI、页面、视觉、界面。注意：非视觉实现请用 ws-dev。
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在 AIWS 约束下交付一个可运行、可验证、视觉方向明确的前端界面
- 先做信息层级与构图，再做组件细节；避免“先堆卡片再补样式”
- 在品牌页与产品页之间做正确取舍：品牌页重视觉锚点，产品页重可操作性

非目标（强制）：
- 不绕过 `$ws-preflight`、`REQUIREMENTS.md`、`AI_WORKSPACE.md`
- 不因为“追求设计感”而重写无关页面、改动无关设计系统或新增大面积依赖
- 不默认把已有产品后台改成营销页；dashboard / admin / workspace 优先 utility copy
- 不把 prompt 语言、设计说明、占位废话直接写进 UI

适用场景：
- 用户要做 landing page、品牌站、活动页、marketing 页面、demo、prototype、game UI
- 用户要把现有前端界面做成“视觉主导、层级清晰、记忆点强”的版本
- 用户明确要求美化、重做、提质、增强 art direction / hierarchy / motion

前置（建议顺序）：
1) 先运行 `$ws-preflight`。
2) 判断任务类型（必须选一个）：
   - `landing`：品牌/营销/活动页
   - `app-ui`：dashboard / admin / workspace / 工具界面
   - `polish-only`：不改信息架构，只做视觉提质
3) 判断设计边界（必须说明）：
   - `net-new`：全新页面，可建立完整视觉语言
   - `existing-system`：已有设计系统/品牌规范，优先复用
4) 若任务达到 medium / complex：先用 `$ws-plan` 落盘计划，再进入实现。

开始编码前，先写三项（不要跳过）：
- `Visual thesis:` 一句话写清 mood / material / energy
- `Content plan:` `hero -> support -> detail -> final CTA`（若是 app-ui，则改为 `workspace -> nav -> context -> action`）
- `Interaction thesis:` 2-3 个动效想法；说明它们如何改善层级/氛围/可感知性

设计默认值：
- 从构图开始，不从组件库开始
- 第一屏优先做成海报感（poster），不是文档感（document）
- 默认先找一个强视觉锚点：大图、主视觉平面、关键产品画面、主数据工作区
- 默认不做卡片墙；优先 section、column、divider、media block、list、plain layout
- 默认最多两套字体、一种强调色；若已有品牌系统，优先跟随现有 token
- 优先靠留白、尺度、裁切、对比、对齐建立层级，再考虑装饰

landing 规则：
1) 默认结构：
   - Hero：品牌/产品名、承诺、CTA、一个主视觉
   - Support：一个具体能力 / 证明点 / offer
   - Detail：氛围、流程、产品深度或故事
   - Final CTA：开始、注册、联系、访问
2) Hero 强约束：
   - 一个 section 只承载一个 dominant idea
   - 默认使用 full-bleed hero；只有内层文字列需要约束宽度
   - 品牌名优先级高于 headline；headline 高于 body；body 高于 CTA
   - 默认不要 hero cards、stat strips、logo clouds、pill soup、floating dashboards
   - headline 在 desktop 约 2-3 行；mobile 一眼读完
   - 若有固定 header，它占用首屏预算；不要让 header + hero 超出初始 viewport
   - 若去掉主视觉后首屏仍几乎成立，说明图像太弱

app-ui 规则：
- 默认偏克制：少颜色、少 chrome、清晰栅格、密度适中、信息可扫读
- 优先组织为：`primary workspace -> navigation -> secondary context/inspector -> action`
- 只有当 card 本身就是交互容器时才用 card；否则尽量改回 plain layout
- 不要把 routine product UI 做成营销落地页
- 文案优先 orientation / status / action：
  - 好例子：`Selected KPIs`、`Plan status`、`Last sync`
  - 差例子：首页口号、情绪化隐喻、执行摘要横幅

图像与媒体：
- 图像必须承担叙事任务，不能只是补背景
- 品牌页/空间页/生活方式产品优先真实感强的图，而不是抽象 3D / 假 dashboard
- 选图时优先有稳定明暗区，便于文字落位
- 避免图里自带抢戏的 logo、signage、碎字、边框 UI
- 若需要多个场景，优先多张图，不要拼贴大杂烩

文案：
- 用产品语言，不用设计评论语言
- headline 负责主要意义；supporting copy 通常一句话够了
- 每个 section 只负责一件事：explain / prove / deepen / convert
- 如果删掉 30% 文案后更清楚，就继续删

动效：
- 视觉型页面至少给 2-3 个“有感但克制”的动效：
  - 一个 hero 入场序列
  - 一个 scroll-linked / sticky / depth 效果
  - 一个 hover / reveal / layout transition
- 动效必须改善层级或氛围，不能只是热闹
- 要兼顾 mobile 流畅度；支持 `prefers-reduced-motion`

工程约束（强制）：
- 先读现有代码，再决定是否沿用已有 design tokens / 组件 / 动效库
- `existing-system` 场景下，优先复用已有视觉语言；不要无故“整站改头换面”
- 不新增字体、图片资源、动画库、运行时依赖，除非明确写出原因、来源、license/成本与回滚方式
- 所有文字覆盖在图像上时，必须保证对比度与点击区域可用
- 必须同时考虑 desktop / mobile；不要只调一个 viewport
- 未运行不声称已运行；验证命令优先引用 `AI_WORKSPACE.md`

硬规则：
- No cards by default
- No hero cards by default
- No generic SaaS card grid as first impression
- No more than one dominant idea per section
- No more than two typefaces without a clear reason
- No more than one accent color unless the existing product already has a strong system
- No decorative gradients behind routine product UI
- No busy imagery behind text
- No filler copy

实现检查（交付前自检）：
- 第一屏能否一眼看出品牌/产品是什么
- 是否存在一个明确视觉锚点
- 只扫标题是否能理解页面
- 每个 section 是否只有一个职责
- card 是否真有必要
- 动效是否真的提升层级/氛围
- 去掉装饰阴影后，页面是否仍然成立

输出要求：
- `Mode:` `landing | app-ui | polish-only`
- `Visual thesis:` 一句话
- `Changed:` 改动文件清单
- `Verify:` 实际运行命令 + 预期结果
- `Evidence:` 相关 `plan/...`、`.aiws/changes/<change-id>/...` 或截图/审计路径

# proposal: web-unit-more（stores/utils 剩余纯函数单测）

## Bindings
Change_ID: web-unit-more
Req_ID: WEB-UNIT-003
Problem_ID: PWA-UNIT-003
Contract_Row: WEB-UNIT-003
Plan_File: .aiws/plan/2026-09-17_19-40-00-web-unit-more.md
Evidence_Path: .aiws/changes/web-unit-more/evidence/
Change_Type: config-docs

## 目标
`web/src/lib/stores/utils.ts` 中 12 个无单测纯函数补单测：`uid/dataAny/isRecord/formatRelativeTime/formatAbsTime/connLabel/sessionTitle/sessionSummary/looksLikeTuiAnsi/isLikelyTuiToolName/codexStructuredEventText/renderOutputHtml`。`inferDefaultApiBaseUrl` 读 `window.location`，跳过。

## 非目标
不碰产品逻辑；不测 relay-store（runes+浏览器依赖，前序结论）；不碰 e2e。

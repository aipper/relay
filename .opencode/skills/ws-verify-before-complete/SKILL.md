---
name: ws-verify-before-complete
description: Thin wrapper for `aiws verify-bc`
---

# ws-verify-before-complete

## 完成前验证 Gate

进入 `ws-finish` / `ws-handoff` 前必须确认：

- [ ] `ws-spec-review` 已完成：`test -f .aiws/changes/<id>/review/spec-review.md` (→ PASS/FAIL)
- [ ] `ws-quality-review` 已完成：`test -f .aiws/changes/<id>/review/quality-review.md` (→ PASS/FAIL)
- [ ] `aiws validate .` stamp 存在：`ls .aiws/tmp/aiws-validate/*.json 2>/dev/null` (→ PASS/FAIL)
- [ ] 无未关闭 Critical blocker：`grep -c 'Critical' .aiws/changes/<id>/review/*.md` = 0 或已标记 resolved (→ PASS/FAIL)
- [ ] 评审-返工循环 handoff 记录：`test -f .aiws/changes/<id>/handoff-evidence.md` 且含 rework round 记录 (→ PASS/FAIL)

## Gate Result（结构化输出）

```
Gate: PASS / FAIL
Items:
1. spec-review: PASS
2. quality-review: PASS
3. validate-stamp: PASS
4. no-critical-blocker: PASS
5. rework-handoff: N/A (未经过返工循环)
Summary: <一句话总结>
Action: → ws-finish / → 需补齐: <缺失项>
```

任一项 FAIL 即阻断 finish；输出中必须给出具体缺失项与补救路径。

Thin skill wrapper. Delegates to `aiws verify-bc`. See `aiws verify-bc --help` for details.

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws verify-bc
elif command -v aiws >/dev/null 2>&1; then
  aiws verify-bc
else
  npx @aipper/aiws verify-bc
fi
```

## 执行要求

- 按 Gate Result 结构化输出逐项验证；任一项 FAIL 即阻断 finish。
- finish 前门禁证据：须确认存在 spec-review.md + quality-review.md（双审查）+ 有效 validate stamp + 已关闭所有 blocker；缺任一项即阻断 finish。

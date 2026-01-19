# Release (Gitea/GitHub + npm)

本项目的 `relay hostd install` / `npm i -g @aipper/relay-cli` 依赖“可下载的发布产物（Release assets）”：

- `relay-hostd-darwin-x64`
- `relay-hostd-darwin-arm64`
- `relay-hostd-linux-x64`
- `relay-darwin-x64`
- `relay-darwin-arm64`
- `relay-linux-x64`

默认下载地址（可通过 `RELAY_RELEASE_BASE_URL` 覆盖）会从 `cli/package.json#repository.url` 推导：

`<repo>/releases/download/v<version>/...`

如果 `repository.url` 使用 SSH 形式（例如 `git@github.com:owner/repo.git`），CLI 会自动转换为 `https://<host>/<owner>/<repo>` 来拼接下载地址。

示例（Gitea）：

`https://gitea.example.com/owner/repo/releases/download/v<version>/...`

## 发布流程（推荐）

1) 更新版本号（必须）

- `cli/package.json` 的 `version` 必须等于要发布的版本号（例如 `0.1.2`）

2) 打 tag 并推送

```bash
git tag v0.1.2
git push origin v0.1.2
```

3) 产出并上传 release assets（方式任选其一）：

- 手动构建（本仓库已有脚本示例）：`bash scripts/package-client.sh`（会在 `dist/` 下生成对应命名的二进制文件）
- 或使用你的 CI（Gitea Actions / GitHub Actions / 其他 CI）构建并上传到 Release（tag = `v<version>`）
- `npm publish` 发布 `@aipper/relay-cli`（需要配置 `NPM_TOKEN`）

## 必要的 Secrets

- `NPM_TOKEN`：有权限发布 `@aipper/relay-cli` 的 npm token（Actions secret）

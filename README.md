# 抵达 Focus

抵达 Focus 是面向 Windows 10/11 的本地优先桌面专注系统，用于连接长期项目、每日计划、深度专注、备忘录提醒与周期复盘。

## 技术栈

- Tauri 2 + Rust
- React 19 + TypeScript + Vite
- SQLite
- Vitest + React Testing Library + Rust tests

## 本地开发

```bash
# 安装依赖
pnpm install

# 启动 Web 开发预览
pnpm dev

# 启动 Tauri 桌面应用
pnpm tauri dev
```

## 验证

```bash
# 前端测试、类型检查和生产构建
pnpm test
pnpm typecheck
pnpm build

# Rust 测试与静态检查
cargo test --manifest-path src-tauri/Cargo.toml --offline --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --offline --locked --all-targets --features desktop-app -- -D warnings
```

## Windows 云构建

GitHub Actions 的 `Windows Build` 工作流会在 `windows-2022` runner 上运行测试，并生成以下 Windows x64 文件：

- NSIS 安装包 `Focus_<version>_x64-setup.exe`
- MSI 安装包 `Focus_<version>_x64_en-US.msi`
- 便携主程序 `arrive-focus.exe`
- 完整性校验文件 `SHA256SUMS.txt`

推送到 `main` 或手动运行工作流时，文件会保留为 14 天有效的 workflow artifact。推送与项目版本一致的 `v*` 标签时，文件会同步到对应 GitHub Release。

项目文档位于 `.monkeycode/docs/`，需求与设计规格位于 `.monkeycode/specs/`。

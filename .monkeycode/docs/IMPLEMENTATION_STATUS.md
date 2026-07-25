# 抵达 Focus 当前实施与验证

## 抵达 Focus 开发与验证

抵达 Focus 工程位于 ``，前端使用 pnpm，桌面核心位于 `src-tauri/`。

```bash
cd arrive-focus

# 前端测试
pnpm test

# TypeScript 类型检查
pnpm typecheck

# 前端生产构建
pnpm build

cd src-tauri

# Rust 单元、集成和文档测试
cargo test
```

新增固定界面文案时，先在 `src/i18n/messages.ts` 的简体中文资源中增加键，再补充英文资源；类型检查会验证资源键完整性。日期和时间展示统一使用 `useI18n()` 暴露的格式器。主窗口与小组件语言同步测试分别位于 `src/app/App.test.tsx` 和 `src/app/WidgetApp.test.tsx`，资源完整性、系统语言解析和格式化测试位于 `src/i18n/i18n.test.tsx`。

任务 13.2 验证基线为前端 24 个测试文件共 90 项测试通过，Rust 132 项单元测试与 6 项集成测试通过，其中备份恢复和桌面适配器各 3 项；TypeScript 类型检查、Vite 生产构建、`cargo fmt --check` 和包含 `desktop-app` feature 的严格 Clippy 检查通过。

任务 13.3 验证基线为前端 24 个测试文件共 90 项测试通过，Rust 136 项单元测试与 6 项集成测试通过；`pnpm typecheck`、`pnpm build`、`cargo fmt --check`、默认 Rust 测试和包含 `desktop-app` feature 的严格 Clippy 检查通过。Tauri 单实例插件、窗口 API 和 `ExitRequested` 编排由桌面 feature 编译门禁覆盖，完整 Windows 单实例与窗口生命周期自动化验收归入任务 13.4。

任务 13.4 验证基线为前端 25 个测试文件共 95 项测试通过，Rust 139 项单元测试与 6 项集成测试通过；`pnpm typecheck`、`pnpm build`、`cargo fmt --check`、默认 Rust 测试、包含 `desktop-app` feature 的严格 Clippy 和 `git diff --check` 均通过。新增覆盖 Dialog 显式自动焦点与恢复、3 秒交互就绪预算、焦点环、减少动效、文本缩放重排、第二实例激活顺序与失败短路，以及主窗口屏幕外状态修正。

任务 14.1 验证基线为前端 26 个测试文件共 100 项测试通过，Rust 141 项单元测试与 6 项集成测试通过；类型检查、生产构建、Rust 格式检查、默认 Rust 测试、包含 `desktop-app` feature 的严格 Clippy 和补丁检查通过。组件测试覆盖桌面运行时隔离、版本与更新说明、下载后安装确认、取消安装、确认后安装和检查失败脱敏；Rust 测试覆盖安装前持久化顺序与失败阻断。

任务 14.2 验证基线为前端 27 个测试文件共 104 项测试通过，Rust 141 项单元测试与 6 项集成测试通过；类型检查、生产构建、Rust 格式检查、默认 Rust 测试、包含 `desktop-app` feature 的严格 Clippy、补丁检查和 Tauri 合并配置构建均通过。`src-tauri/tauri-config.contract.test.ts` 固定 NSIS target、安装模式、双语选择、开始菜单目录、WebView2 bootstrapper、Windows 图标和 Authenticode 覆盖配置契约。

任务 14.3 验证基线为前端 27 个测试文件共 104 项测试通过，Rust 141 项单元测试与 7 项集成测试通过；`pnpm test`、`pnpm typecheck`、`pnpm build`、`cargo fmt --all -- --check`、`cargo test --offline --locked`、包含 `desktop-app` feature 的严格 Clippy 和 `git diff --check` 均通过。新增 `src-tauri/tests/desktop_core_flow.rs` 串联项目、任务、重复计划、今日汇总、小组件、通知、专注、日历统计和备份服务，验证 Release Acceptance 核心流程及关键幂等约束。

任务 14.4 验证基线为前端 28 个测试文件共 108 项测试通过，Rust 141 项单元测试与 7 项集成测试通过；类型检查、生产构建、Rust 格式检查、默认 Rust 测试、包含 `desktop-app` feature 的严格 Clippy 和补丁检查均通过。`scripts/windows-installer-smoke.contract.test.ts` 跨平台固定两版 NSIS 输入、静默参数、双版本启动探测、升级二进制替换、静默卸载和数据保留断言；完整 PowerShell 烟测在 Windows 10/11 发布机执行。

最终检查点 15 已通过：任务清单全部完成，前端 28 个测试文件共 108 项测试、Rust 141 项单元测试与 7 项集成测试再次通过；TypeScript 类型检查、Vite 生产构建、Rust 格式检查和包含 `desktop-app` feature 的严格 Clippy 均通过。Windows NSIS 实机安装升级烟测继续作为签名发布机门禁执行。

项目持久化修复后的验证基线为前端 30 个测试文件共 112 项测试、Rust 141 项单元测试与 7 项集成测试通过；`pnpm typecheck`、`pnpm build`、`cargo fmt --all -- --check`、`cargo test --offline --locked` 和包含 `desktop-app` feature 的严格 Clippy 均通过。项目定向测试位于 `src/features/projects/projectClient.test.ts` 与 `src/features/projects/ProjectWorkspace.test.tsx`，覆盖 command 参数、权威列表与详情加载、完整项目输入、写入失败状态保留和项目任务操作。

重复任务生产调度修复后的验证基线为前端 30 个测试文件共 114 项测试、Rust 143 项单元测试与 7 项集成测试通过；类型检查、生产构建、Rust 格式检查、默认 Rust 测试和包含 `desktop-app` feature 的严格 Clippy 均通过。服务层定向测试覆盖开放式规则跨日回填、规则时区、本地日界线和重复运行幂等；`App.test.tsx` 与 `WidgetApp.test.tsx` 覆盖 `today://changed` 刷新。

调整重复任务运行时时，应保持启动和恢复使用 `GenerationTrigger::Startup`，常驻 worker 使用 `GenerationTrigger::DayBoundary`，并维持“生成实例、提交 SQLite、广播 `today://changed`、扫描通知”的顺序。自动协调使用 UTC 时钟输入并按每条规则的 IANA 时区计算本地日期，测试应注入固定 `DateTime<Utc>`。

Widget 关闭生命周期修复后的验证基线为前端 30 个测试文件共 114 项测试、Rust 144 项单元测试与 7 项集成测试通过；类型检查、生产构建、Rust 格式检查、默认 Rust 测试和包含 `desktop-app` feature 的严格 Clippy 均通过。`desktop::lifecycle::tests::missing_widget_window_does_not_block_exit_persistence` 固定退出容错边界，桌面 feature 编译覆盖 Widget `CloseRequested` 的保存、隐藏和阻止销毁接线。

Widget Shell 层级恢复修复后的验证基线为前端 30 个测试文件共 114 项测试、Rust 145 项单元测试与 7 项集成测试通过；`pnpm typecheck`、`pnpm build`、`cargo fmt --all -- --check`、`cargo test --offline --locked`、包含 `desktop-app` feature 的严格 Clippy 和 `git diff --check` 均通过。`outcomes_define_window_layer_and_recovery_state` 固定 Shell outcome 到原生窗口层级的映射，`WidgetApp.test.tsx` 覆盖回退提示出现、恢复事件清除提示和监听器卸载。

通知发布失败重试修复后的验证基线为前端 30 个测试文件共 114 项测试、Rust 147 项单元测试与 7 项集成测试通过；类型检查、生产构建、Rust 格式检查、默认 Rust 测试、`desktop-app` feature 编译、严格 Clippy 和补丁检查均通过。定向测试覆盖失败投递重新预留、发布失败后重试成功、投递记录幂等，以及扫描游标仅在 reconciliation 成功后推进。

通知中断恢复修复后的验证基线为前端 30 个测试文件共 114 项测试、Rust 149 项单元测试与 7 项集成测试通过；默认 Rust 测试、`desktop-app` feature 编译和严格 Clippy 均通过。定向测试使用固定 UTC 时间覆盖 60 秒 lease 边界、活动 `pending` 保持 reconciliation 失败、过期 `pending` 原子接管、`sent` 永久去重和 P7 重复处理幂等。

任务跨窗口同步修复后的验证基线为前端 30 个测试文件共 114 项测试、Rust 151 项单元测试与 7 项集成测试通过；默认 Rust 测试、`desktop-app` feature 编译和严格 Clippy 均通过。`desktop::today_events::tests` 固定成功写入发送一次事件、失败写入保持零事件，desktop feature 编译覆盖 13 个任务、重复规则和实例写 command 的 `AppHandle` 注入与事件接线。

专注跨窗口同步修复后的验证基线为前端 30 个测试文件共 115 项测试、Rust 153 项单元测试与 7 项集成测试通过；`pnpm typecheck`、`pnpm build`、`cargo fmt --all -- --check`、`cargo test --offline --locked`、`cargo check --offline --locked --features desktop-app` 和包含 `desktop-app` feature 的严格 Clippy 均通过。`desktop::focus_events::tests` 固定成功状态变更发送一次事件、领域失败保持零事件；`WidgetApp.test.tsx` 覆盖跨窗口暂停状态与剩余时间即时更新。

项目状态跨窗口同步修复后的验证基线保持为前端 30 个测试文件共 115 项测试、Rust 153 项单元测试与 7 项集成测试通过；类型检查、生产构建、Rust 格式检查、`desktop-app` feature 编译和严格 Clippy 均通过。项目四类写 command 复用 `after_today_change`，`App.test.tsx` 固定 `today://changed` 同时重新读取项目摘要与当前 Today digest，`WidgetApp.test.tsx` 继续覆盖同一事件触发摘要刷新。

暂停项目专注资格修复后的验证基线为前端 30 个测试文件共 115 项测试、Rust 155 项单元测试与 7 项集成测试通过；`pnpm test`、`pnpm typecheck`、`pnpm build`、`cargo fmt --all -- --check`、`cargo test --offline --locked`、`cargo check --offline --locked --features desktop-app` 和包含 `desktop-app` feature 的严格 Clippy 均通过。`services::focus_service::tests::paused_project_blocks_task_and_recurring_instance_focus` 覆盖普通任务当前项目与重复实例快照项目，`desktop::tray::tests::tray_focus_candidates_skip_paused_projects` 固定托盘跳过暂停项目并继续选择后续候选；`domainError.test.ts` 与 `i18n.test.tsx` 覆盖稳定错误码的双语提示。

修改窗口生命周期时，应保持 `tauri-plugin-single-instance` 在 builder 插件链首位，第二实例、托盘和全局快捷键继续复用 `show_main_window()`。主窗口配置保持初始隐藏，并在数据库可用后调用 `restore_main_window()` 显示；主窗口几何运行态必须在恢复前注册，确保恢复产生的窗口事件可以安全防抖。显式退出入口统一调用 `desktop::lifecycle::request_exit()`，关闭到托盘只保存并隐藏主窗口。Widget 的关闭请求必须调用 `prevent_close()` 并隐藏窗口，保证后续显示、解锁、Shell 恢复和退出持久化仍有有效窗口实例。Shell outcome 应继续作为父窗口关系与 `always_on_top` 的共同权威来源；恢复桌面附着后同步广播 `widget://mode-restored`，保持前端提示与原生层级一致。

修改通知投递时，应保持“预留记录、调用系统 publisher、标记 sent 或 failed”的顺序。worker 仅在整批 reconciliation 成功后推进扫描游标；`failed` 和 lease 已过期的 `pending` 允许下一轮原子接管，活动 `pending` 保持窗口待处理，`sent` 继续拒绝重复预留。服务层应处理完当前窗口中的所有候选再返回首个发布错误或 in-flight 状态，避免单个候选阻断同批其他到时任务。

新增会改变项目摘要、今日任务、项目进度或重复实例的 Tauri 写 command 时，应通过 `after_today_change` 在领域写入成功后广播 `today://changed`。失败结果保持原领域错误并跳过广播，主窗口与 Widget 继续把该事件作为重新读取权威 SQLite 摘要的信号。

新增专注状态转换入口时，应通过 `after_focus_change` 在领域状态成功写入后广播 `focus://state-changed`。手动完成和自动到期还需发送 `focus://completed`，并同步广播最终 ready 状态；Widget 保留周期权威读取，用于事件丢失与系统恢复后的校准。

新增备忘录写入口时，应通过 `after_memo_change` 在 Repository 事务成功后广播空 payload 的 `memo://changed`。领域失败保持原错误并跳过广播，订阅方只用该事件触发 SQLite 权威数据重读。定向验证使用 `cargo test --offline --locked memo`，当前覆盖 47 项 memo 相关测试，其中 `commands::memo::tests` 使用真实内存 SQLite 验证写 command 编排，`desktop::memo_events::tests` 固定成功写入一次广播与失败零广播；`cargo check --offline --locked --features desktop-app` 验证 `AppHandle` 注入和 Tauri command 接线。

备忘录错误映射修改后运行 `pnpm test -- src/lib/domainError.test.ts` 与 `pnpm run typecheck`，验证全部稳定 `MEMO_*` 类别在简体中文和英文下返回安全操作提示。Rust 日志边界使用 `cargo test --offline --locked diagnostics` 验证诊断事件排除备忘录标题、正文、标签、搜索词和内部错误 message；当前前端全量测试为 32 个文件共 132 项，诊断定向测试为 3 项。

修改开始专注入口时，应保留 `FocusService::validate_target` 的统一资格校验：普通任务使用当前项目引用，重复实例使用快照项目引用，暂停项目返回 `FOCUS_PROJECT_PAUSED`。托盘候选筛选应跳过暂停项目并继续搜索，所有其他入口继续依赖服务层兜底，避免旧前端状态或直接 command 调用绕过项目状态。

主窗口状态定向测试位于 `src-tauri/src/domain/window.rs`、`src-tauri/src/desktop/main_window.rs`、`src-tauri/src/desktop/lifecycle.rs` 和 `src-tauri/src/repositories/preferences_repository.rs`，覆盖值域、物理到逻辑尺寸转换、SQLite 往返、无效状态回退和暂停专注退出持久化。屏幕外位置修正继续由 `desktop/widget_window.rs` 的示例测试与 P8 property-based test 覆盖，主窗口与小组件共享同一算法。

共享无障碍组件测试位于 `src/components/ui.test.tsx`，覆盖 Dialog 初始焦点、焦点循环、Escape 关闭、焦点恢复、唯一可读标题，以及 SegmentedControl 的 roving tabindex、方向键、Home 和 End。主题测试位于 `src/theme/theme.test.ts`，使用 OKLCH 到线性 sRGB 的转换验证每套明暗主题的正文、辅助文字、强调文字、主按钮和状态文字均达到 4.5:1。任务行、小组件和主导航测试分别验证包含业务上下文的操作名称、背景透明度边界和当前页面状态。

无障碍 CSS 契约测试位于 `src/styles/accessibility.contract.test.ts`，直接读取 `global.css` 验证 `:focus-visible`、`prefers-reduced-motion` 和 125% 文本缩放所依赖的重排、滚动边界。主窗口首次交互预算由 `src/app/App.test.tsx` 覆盖；单实例激活和窗口恢复定向测试位于 `src-tauri/src/desktop/main_window.rs`。

文本缩放适配依赖内容自然重排与可滚动边界。修改主页面、设置区、Dialog 或 Widget 布局时，应保留 `min-width: 0`、可换行操作区、视口约束的 Dialog 滚动和 Widget 根滚动；减少动态效果规则应继续停用装饰性动画与过渡，并保留即时状态反馈。

更新发布构建必须同时设置 `ARRIVE_FOCUS_UPDATE_ENDPOINT` 和 `ARRIVE_FOCUS_UPDATE_PUBLIC_KEY`。endpoint 使用 HTTPS，公钥内容来自 Tauri signer 生成结果；签名私钥通过发布环境的 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 提供。应用仓库和构建日志不得包含私钥。版本发布时同步更新 `package.json`、`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 的版本号。

Windows Authenticode 发布构建在 Windows 签名主机执行。先从 `src-tauri/tauri.windows-signing.conf.example.json` 生成被 Git 忽略的 `src-tauri/tauri.windows-signing.conf.json`，将占位 thumbprint 替换为已导入 Windows 证书存储的代码签名证书 SHA-1 thumbprint，再运行签名 bundle：

```bash
# 生成本地签名覆盖配置
cp src-tauri/tauri.windows-signing.conf.example.json src-tauri/tauri.windows-signing.conf.json

# 构建并签署可执行文件、NSIS 安装包和更新产物
pnpm tauri:build:windows:signed
```

无需发布签名材料的本地 Windows 验包使用独立入口。该命令显式启用 `desktop-app`，并通过公开覆盖配置关闭 updater 产物：

```bash
pnpm tauri:build:windows
```

Windows 打包入口修复后的验证基线为前端 30 个测试文件共 118 项测试、Rust 155 项单元测试与 7 项集成测试通过；`pnpm typecheck`、`pnpm build`、默认 Rust 测试、`cargo check --features desktop-app`、包含 `desktop-app` feature 的严格 Clippy、Rust 格式和 `git diff --check` 通过。`src-tauri/tauri-config.contract.test.ts` 固定无签名与签名脚本都携带 `desktop-app` feature、三处发布版本一致，并验证无签名覆盖配置关闭 updater 产物。

Tauri updater 插件即使在无签名验包中关闭更新产物，运行时仍要求 `plugins.updater` 是可反序列化对象。基础配置必须保留空 `endpoints` 数组和空 `pubkey` 字符串，构建时注入的发布公钥继续由 Rust plugin builder 覆盖。配置缺失会在主窗口显示前触发 `PluginInitialization("updater", ...)` panic，并以退出码 101 结束进程；配置契约测试固定该启动前置条件。

Linux 验包环境可使用 `cargo-xwin` 下载的 MSVC sysroot、Clang/LLD、NSIS 和一个向 Cargo 注入 sysroot include/library 路径的 runner 交叉生成 Windows x64 无签名产物。本次验证使用 `/tmp/opencode/cargo-msvc` runner：

```bash
CARGO_HTTP_MULTIPLEXING=false CARGO_HTTP_TIMEOUT=120 CARGO_NET_RETRY=2 \
  pnpm tauri build \
  --runner /tmp/opencode/cargo-msvc \
  --target x86_64-pc-windows-msvc \
  --features desktop-app \
  --config src-tauri/tauri.windows-unsigned.conf.json
```

应用产物位于 `src-tauri/target/x86_64-pc-windows-msvc/release/arrive-focus.exe`，NSIS 产物位于 `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/抵达 Focus_0.1.4_x64-setup.exe`，同目录的 `SHA256SUMS.txt` 提供两个文件的交付校验值。`file` 与 `llvm-readobj --file-headers` 已确认应用为 `PE32+` x86-64 且 `Subsystem` 为 `IMAGE_SUBSYSTEM_WINDOWS_GUI`；NSIS 文件已识别为 Nullsoft Installer self-extracting archive。两个文件的 PE 证书表均为空，符合无签名构建预期。交叉构建用于静态验包和提前发现编译问题，Windows 10/11 实机继续承担安装、启动、升级、卸载、WebView2 和数据保留验收。

签名覆盖配置保持 `digestAlgorithm` 为 `sha256`，时间戳服务使用 HTTPS。公开发布前在 Windows 10 22H2 x64 与 Windows 11 23H2 x64 验证安装目录页、桌面快捷方式复选框、开始菜单入口、缺少 WebView2 时的联网安装，以及可执行文件和安装包的 Authenticode 签名状态。

Windows 安装升级烟测需要两个版本不同的 NSIS `.exe` 产物，并使用一次性 Windows 测试用户。默认脚本会拒绝已有安装目录、已有 `%APPDATA%/com.arrive.focus` 数据目录和正在运行的 `arrive-focus.exe`：

```powershell
pnpm smoke:windows-installer -- `
  -BaselineInstallerPath C:\artifacts\baseline\arrive-focus-setup.exe `
  -UpgradeInstallerPath C:\artifacts\upgrade\arrive-focus-setup.exe
```

脚本使用隔离安装目录完成基线静默安装、首次启动、升级静默安装、升级版本启动和静默卸载，最终保留应用数据目录作为验收证据。发布机在烟测完成后按其临时用户或虚拟机回收流程清理环境。

新增或调整领域错误时，需要保持稳定错误码，并在 `src/lib/domainError.ts` 增加精确映射或确认现有类别映射适用；中英文文案同步维护在 `src/i18n/messages.ts`。生产组件统一调用 `domainErrorMessage`，避免直接展示 `DomainError.message` 或任意捕获异常的 `Error.message`。Rust command 统一使用 `CommandResult::from_result(module_path!(), value, version)`，诊断日志只记录脱敏上下文、错误码和字段名。前端 invoke 被 Tauri 拒绝时会调用 `diagnostic_command_failure`，在应用日志中记录经过单行、长度限制和字符过滤的 command 与拒绝原因；日志 IPC 自身失败时仍返回稳定的 `COMMAND_INVOCATION_FAILED`。

SQLite 必须在 `tauri::Builder` 构建 AppManager 前打开并通过 `.manage(database)` 注册。WebView 可以在 `setup` 完成前加载前端脚本，因此在 setup 内调用 `app.manage(database)` 会形成首批 command 与状态注册之间的竞争，并产生 `state not managed for field database`。`tauri-config.contract.test.ts` 固定数据库注册先于 setup。

错误映射定向测试执行 `pnpm exec vitest run src/lib/domainError.test.ts`。Rust 协议与日志脱敏测试位于 `src-tauri/src/lib.rs`，可执行 `cargo test command_failure_diagnostics` 和 `cargo test failure_result_uses_stable_shape`。

备忘录 schema 迁移位于 `src-tauri/migrations/0004_memo_center.sql`。数据库定向测试执行 `cargo test --offline --locked repositories::database::tests`，覆盖迁移幂等、批次回滚、既有通知保留、`memoReminder` 类型、字段 CHECK/UNIQUE 约束和备忘录关联级联约束。

备忘录领域模型位于 `src-tauri/src/domain/memo.rs`。定向验证执行 `cargo test --offline --locked domain::memo::tests`，覆盖空草稿、Unicode 长度边界、标签限制、camelCase 提醒协议、未来时间、频率组合、重复星期、日期范围、间隔、IANA 时区、夏令时不存在的本地时间和时钟格式。

前端备忘录 DTO 和 command client 位于 `src/features/memos/`。执行 `pnpm exec vitest run src/features/memos/memoClient.test.ts` 验证六个 command 的名称与参数映射，执行 `pnpm run typecheck` 验证 DTO 与前端调用类型。

备忘录核心服务位于 `src-tauri/src/services/memo_service.rs`。执行 `cargo test --offline --locked services::memo_service::tests` 验证创建、读取、更新、置顶时间和显示标题派生逻辑。

备忘录提醒时间计算与到期协调位于 `src-tauri/src/services/memo_reminder_service.rs`，共享日期选择函数位于 `src-tauri/src/domain/recurrence.rs`。执行 `cargo test --offline --locked memo_reminder_service` 可验证一次提醒 UTC 转换、每天/每周/每月自定义间隔、工作日、普通年与闰年月末收敛、结束日期、DST 缺失和重叠时刻、同批失败隔离及陈旧发生时间条件推进；当前共 12 项。

修改提醒扫描时，应保持“查询全部到期项、逐项投递、成功后条件推进”的顺序。投递失败保留原 `next_scheduled_for`，一次提醒成功后设为 completed，重复提醒从当前发生时间计算严格递增的下一时间；Repository 更新必须继续校验提醒 ID、active 状态和旧发生时间。通知租约与幂等投递记录由通知服务层接入，提醒扫描服务保持 publisher 回调边界。

备忘录通知协调位于 `src-tauri/src/services/notification_service.rs`，桌面周期接线位于 `src-tauri/src/desktop/notifications.rs`。执行 `cargo test --offline --locked notification_service::tests` 可定向验证首次投递、显示标题与提示音、失败记录和重试、活动 lease、过期 lease 接管、已发送中断恢复及任务通知既有行为。修改协调逻辑时保持 `(memoReminder, reminder.id, next_scheduled_for)` 唯一身份，并让 `AlreadySent` 路径完成提醒状态推进。

标签关联事务位于 `src-tauri/src/repositories/memo_repository.rs`。执行 `cargo test --offline --locked memo` 可联合验证标签规范化、大小写复用、关联替换、孤立标签清理和缺失备忘录回滚。

执行 `cargo test --offline --locked repositories::memo_repository::tests` 可定向验证删除级联、共享标签保留、稳定错误码，以及通过 SQLite trigger 注入清理失败后的事务回滚。

Property M2 使用 `proptest` 运行 64 组随机标签集合。执行 `cargo test --offline --locked property_tag_normalization_keeps_entities_and_links_unique`，验证大小写与首尾空白变体经过规范化和关联替换后，标签实体、规范名和 memo-tag 关联始终唯一。

Property M6 使用 `proptest` 运行 64 组随机标签共享图。执行 `cargo test --offline --locked property_memo_removal_is_atomic_for_random_association_graphs`，验证正常删除会精确保留共享标签和关联，注入孤立标签清理失败时会恢复目标备忘录、提醒、全部标签及关联。

`MemoRepository` 的定向测试执行 `cargo test --offline --locked repositories::memo_repository::tests`。测试覆盖 CRUD 详情往返、标签聚合、标签筛选实时计数、重复提醒反序列化、到期提醒稳定查询、发生时间条件推进、LIKE 特殊字符字面搜索、搜索筛选交集、稳定排序、核心字段、标签与提醒的联合回滚，以及删除和属性测试；当前共 17 项。

Property M3 使用 `proptest` 运行 64 组随机置顶状态、置顶时间、更新时间和 ID 集合。执行 `cargo test --offline --locked property_memo_list_sorting_is_stable`，验证正序或逆序写入后，两次列表查询都与置顶、置顶时间倒序、更新时间倒序、ID 升序的纯 Rust 基准完全一致。

Property M8 使用 `proptest` 运行 64 组随机标题、正文、标签命中位置和筛选关联。执行 `cargo test --offline --locked property_search_and_tag_filter_return_exact_intersection`，验证搜索结果、标签结果及二者交集分别与 Rust 集合基准完全一致，并覆盖 `%`、`_` 和反斜杠的字面搜索。

Repository 独立集成测试位于 `src-tauri/tests/memo_repository.rs`。执行 `cargo test --offline --locked --test memo_repository`，验证临时文件数据库中的 CRUD、搜索、标签、提醒、删除与重开持久化，并验证关联写入失败后的跨重开事务回滚；当前共 2 项。执行 `cargo test --offline --locked memo` 可运行 48 项 memo 相关单元与属性测试。

备忘录数据与 Repository 阶段质量门禁执行 `cargo test --offline --locked`。阶段 4 检查点包含 186 项库测试和 9 项集成/适配测试，共 195 项；同时执行 `cargo fmt --all -- --check` 与 `git diff --check`。

提醒规则任务 6.1 的验证基线为 198 项 Rust 库测试与 9 项集成/适配测试通过；`cargo fmt --all -- --check`、默认 `cargo check --offline --locked` 和 `git diff --check` 同步通过。

提醒扫描任务 6.2 的验证基线为 203 项 Rust 库测试与 9 项集成/适配测试通过；`cargo fmt --all -- --check`、`cargo check --offline --locked --features desktop-app`、`cargo clippy --offline --locked --all-targets -- -D warnings` 和 `git diff --check` 同步通过。

通知租约任务 6.3 的验证基线为 208 项 Rust 库测试与 9 项集成/适配测试通过；`cargo fmt --all -- --check`、`cargo check --offline --locked --features desktop-app`、`cargo clippy --offline --locked --all-targets --features desktop-app -- -D warnings` 和 `git diff --check` 同步通过。新增定向测试覆盖备忘录通知首次成功、失败重试、活动 lease、过期接管和 `AlreadySent` 中断恢复。

通知点击任务 6.4 的验证基线为 211 项 Rust 库测试与 9 项集成/适配测试通过；`cargo fmt --all -- --check`、`cargo check --offline --locked --features desktop-app`、`cargo clippy --offline --locked --all-targets --features desktop-app -- -D warnings` 和 `git diff --check` 同步通过。`cargo test --offline --locked memo_notification_activation` 定向覆盖有效 UUID 按“显示主窗口、发送事件”顺序激活、非法 ID 零副作用和窗口失败零事件。Windows target 编译可执行 `cargo check --locked --target x86_64-pc-windows-msvc --features desktop-app`，安装态还需验证通知主体点击、主窗口恢复和 `memo://open-requested` 到达。

提醒规则任务 6.5 的验证基线为 212 项 Rust 库测试与 9 项集成/适配测试通过；规则测试覆盖全部频率、严格未来时间边界、IANA 时区、DST 缺失与重叠时刻、自定义日/周/月间隔以及普通年和闰年月末日期。`cargo check --offline --locked --features desktop-app`、`cargo clippy --offline --locked --all-targets --features desktop-app -- -D warnings`、`cargo fmt --all -- --check` 和 `git diff --check` 同步通过。Linux 环境执行 Windows MSVC target 检查需要提供 `lib.exe` 等 MSVC 交叉工具链。

提醒幂等任务 6.6 的验证基线为 213 项 Rust 库测试与 9 项集成/适配测试通过。`cargo test --offline --locked property_m4` 使用 64 组随机提醒 ID、UTC 发生时间、重复协调次数和提示音配置，验证每次提醒发生最多形成一条 `sent` 投递记录并最多调用一次成功通知发布；完整 `desktop-app` 编译、Clippy、格式和 diff 门禁同步通过。

重复提醒推进任务 6.7 的验证基线为 214 项 Rust 库测试与 9 项集成/适配测试通过。`cargo test --offline --locked property_m5` 使用 64 组有效重复规则，覆盖每天、工作日、每周、每月、自定义间隔、周几集合、月末收敛和五个 IANA 时区；每组通过 SQLite Repository 执行一次成功投递后的条件推进，并独立验证下一发生时间严格递增且符合保存的本地规则。完整 `desktop-app` 编译、Clippy、格式和 diff 门禁同步通过。

通知 worker 任务 6.8 的验证基线为 214 项 Rust 库测试与 12 项集成/适配测试通过。桌面通知线程通过 `NotificationService::reconcile_reminder_worker_cycle` 统一协调任务与备忘录提醒；执行 `cargo test --offline --locked --test memo_notification_worker` 可使用磁盘 SQLite 验证权限拒绝后的跨进程重试与点击激活、过期 lease 重启接管，以及通知已发送但提醒未推进时的中断恢复和零重复发布。完整 `desktop-app` 编译、Clippy、格式和 diff 门禁同步通过。

备份格式任务 7.1 将导出版本升级为 2，并增加 `memos`、`memoTags`、`memoTagLinks` 和 `memoReminders` 四个稳定排序集合。版本 1 解析保留源格式号并在反序列化前确定性补齐四个空集合；执行 `cargo test --offline --locked backup` 可验证版本 2 导出、版本 1 转换及既有 P9 往返。验证基线为 215 项 Rust 库测试、12 项集成/适配测试和 32 个前端测试文件共 132 项测试通过，`pnpm run typecheck`、完整 `desktop-app` 编译、Clippy、格式和 diff 门禁同步通过。

备份恢复任务 7.2 在写入前校验备忘录和标签长度、规范标签名、集合唯一性、每条备忘录的标签与提醒数量、跨记录引用，以及提醒字段组合、状态、日期时间、IANA 时区和活动提醒下一发生时间。恢复继续使用现有单事务边界，按关联、提醒、备忘录、标签顺序清理旧数据，再按备忘录、标签、关联、提醒顺序写入；故障测试在备忘录写入阶段注入 SQL 失败，验证原项目、原备忘录和备份历史事务状态完整回滚。验证基线为 217 项 Rust 库测试、12 项集成/适配测试和 32 个前端测试文件共 132 项测试通过，`pnpm test`、`pnpm run typecheck`、`cargo test --offline --locked`、完整 `desktop-app` 编译、严格 Clippy、格式和 diff 门禁同步通过。

备份往返任务 7.3 新增 Property M7。执行 `cargo test --offline --locked property_m7` 会运行 64 组有效备忘录关系图，覆盖空集合、共享标签、不同标签组合、置顶状态、一次性与重复提醒、提醒状态和四个 IANA 时区，并通过源 SQLite 导出、JSON 解析、目标 SQLite 恢复和再次导出验证规范化备忘录业务模型等价。验证基线为 218 项 Rust 库测试、12 项集成/适配测试和 32 个前端测试文件共 132 项测试通过，`pnpm test`、`pnpm run typecheck`、`pnpm run build`、`cargo test --offline --locked`、完整 `desktop-app` 编译、严格 Clippy、格式和 diff 门禁同步通过。

备份兼容与故障集成任务 7.4 执行 `cargo test --offline --locked --test backup_restore`。4 项测试覆盖版本 1 导入为空备忘录集合、版本 2 损坏标签引用在写入前返回稳定错误、成功恢复四类备忘录数据及恢复前快照并在数据库重开后保持结果，以及在备忘录插入阶段注入 SQL trigger 后完整回滚项目、备忘录和备份历史。验证基线为 218 项 Rust 库测试、13 项集成/适配测试和 32 个前端测试文件共 132 项测试通过，前端测试、类型检查、生产构建、Rust 全量测试、完整 `desktop-app` 编译、严格 Clippy、格式和 diff 门禁同步通过。

备忘录页面任务 8.1 在 `src/app/App.tsx` 注册导航、页面状态和 Tauri 事件生命周期，在 `src/features/memos/MemoWorkspace.tsx` 建立后续页面实现的输入边界。`memo://changed` 推进数据修订号，`memo://open-requested` 生成带递增序号的打开请求；修改该生命周期时应保留异步监听器卸载保护和相同 ID 重复请求语义。定向验证执行 `pnpm exec vitest run src/app/App.test.tsx src/i18n/i18n.test.tsx`，完整验证基线为 32 个前端测试文件共 135 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录页面任务 8.2 在 `.memo-workspace` 中建立 360px 列表栏与弹性编辑栏，并把滚动限制在 `.memo-list-pane` 和 `.memo-editor` 内。布局结构测试位于 `src/features/memos/MemoWorkspace.test.tsx`，CSS 宽度和滚动契约位于 `src/styles/memo-layout.contract.test.ts`。完整验证基线为 34 个前端测试文件共 138 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录页面任务 8.3 使用 inline-size container query 在内容宽度低于 980px 时切换列表和编辑单栏。两个 Panel 保持挂载，通过 `data-mobile-view` 控制可见面板；通知打开请求进入编辑视图，可读返回按钮恢复列表。完整验证基线为 34 个前端测试文件共 140 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录页面任务 8.4 和 8.5 已完成加载骨架、首次空状态、筛选零结果、详情失效和权威刷新，并通过可注入 `MemoClient` 固定异步状态。`src/app/App.test.tsx` 覆盖导航及事件生命周期，`src/features/memos/MemoWorkspace.test.tsx` 覆盖双栏结构、单栏往返、加载、空状态、零结果和失效详情，`src/styles/memo-layout.contract.test.ts` 固定 360px 双栏、独立滚动和 980px 容器查询。任务组 8 验证基线为 34 个前端测试文件共 144 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录列表任务 9.1 使用 `src/features/memos/MemoListItem.tsx` 集中处理标题、两行正文、标签折叠、置顶状态、提醒摘要和更新时间。`src/features/memos/MemoListItem.test.tsx` 覆盖列表选择、前三个标签与剩余数量、图标文字状态、重复提醒频率以及完成状态；`src/styles/memo-layout.contract.test.ts` 固定两行截断与紧凑元数据布局。验证基线为 35 个前端测试文件共 151 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录搜索筛选任务 9.2 在 `MemoWorkspace` 中实现 200 毫秒搜索防抖、权威标签计数、单标签选择、搜索与标签交集查询、清除条件和失效标签收敛；`App` 持有最近 `MemoListQuery` 以跨主页面切换恢复条件。`MemoWorkspace.test.tsx` 覆盖延迟搜索、交集参数和清除条件，`App.test.tsx` 覆盖卸载再挂载后的查询恢复。验证基线为 35 个前端测试文件共 153 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录任务 9.3 和 9.4 在 `MemoEditor.tsx` 中实现标签添加、移除、大小写重复提示、10 个上限、置顶切换和保存状态，在 `MemoWorkspace` 中统一处理乐观详情、command 返回回写、权威列表刷新与失败回滚。`MemoEditor.test.tsx` 固定标签 trim、复用、移除、上限和置顶回调，`MemoWorkspace.test.tsx` 固定置顶失败时的安全错误及权威状态恢复。任务组 9 验证基线为 36 个前端测试文件共 157 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录编辑任务 10.1 在 `MemoEditor.tsx` 中以完整 `MemoInput` 维护标题、纯文本正文、标签和置顶草稿，按 Unicode 字符限制标题 200 字符和正文 20000 字符，并提供保存按钮与 Ctrl/Cmd+Enter。`MemoWorkspace` 使用 `draftOpen` 区分新草稿和未选中状态，首次保存调用 `memo_create`，成功后切换到返回的权威记录；已有记录调用 `memo_update`。定向测试执行 `pnpm exec vitest run src/features/memos/MemoEditor.test.tsx src/features/memos/MemoWorkspace.test.tsx`，覆盖字符边界、显式与快捷保存、新建首次保存和置顶失败回滚。验证基线为 36 个前端测试文件共 160 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录自动保存任务 10.2 使用 `MemoEditor` 的 500 毫秒草稿去抖和 `MemoWorkspace` 的单请求加最新值队列。保存期间继续接受输入，中间成功响应不会覆盖较新的本地草稿；新建期间的排队值在首次 `memo_create` 成功后通过 `memo_update` 写入同一记录。失败时保留标题与正文，恢复权威标签和置顶值，并显示“重新保存”入口。`MemoEditor.test.tsx` 使用 fake timers 固定 499/500 毫秒边界，`MemoWorkspace.test.tsx` 使用延迟 Promise 固定请求串行和最新草稿提交。验证基线为 36 个前端测试文件共 162 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录删除任务 10.3 复用共享 `Dialog` 展示显示标题及记录、标签关联、未发送提醒的删除影响。取消保持编辑上下文，确认调用 `MemoClient.remove`；失败保持详情并刷新权威列表，成功清空详情并返回列表。`MemoWorkspace.test.tsx` 覆盖取消零调用、失败安全文案与重试、成功清理详情和列表刷新。定向验证包含 12 项工作区测试和 6 项编辑器测试，`pnpm run typecheck` 同步通过。

Property M1 位于 `src/features/memos/memoSaveQueue.property.test.ts`，生产代码与测试共同使用 `LatestMemoSaveQueue`。属性测试运行 128 组、每组最多 80 个随机编辑或保存完成事件，排空队列后断言最后启动的保存值等于用户最后输入值。执行 `pnpm exec vitest run src/features/memos/memoSaveQueue.property.test.ts src/features/memos/MemoWorkspace.test.tsx` 可联合验证属性和 React 编排。

任务组 10 的组件测试覆盖新建、Unicode 长度边界、500 毫秒自动保存、Ctrl/Cmd+Enter、保存期间继续输入、失败草稿保留与重新保存、Property M1、删除取消、删除失败重试和成功删除。阶段验证基线为 37 个前端测试文件共 166 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。并行执行测试与生产构建时曾使既有日历测试超过 5 秒，单独复测与串行全量测试均通过。

任务组 11 的提醒 Dialog 位于 `src/features/memos/MemoReminderEditor.tsx`，权限提示位于 `MemoReminderPermissionNotice.tsx`。组件测试覆盖一次、每天、工作日、每周、每月、自定义间隔、渐进字段、未来时间、IANA 时区、结束日期、错误聚焦、已有规则修改、取消提醒和权限设置入口。推荐使用 `pnpm exec vitest run src/features/memos/MemoReminderEditor.test.tsx src/features/memos/MemoReminderPermissionNotice.test.tsx src/features/memos/MemoEditor.test.tsx` 定向验证；阶段全量门禁使用 `pnpm exec vitest run --maxWorkers=1 --minWorkers=1`，基线为 39 个文件共 176 项测试，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

任务组 12 使用 `src/i18n/i18n.test.tsx` 固定中英文资源键一致性，`src/components/ui.test.tsx` 固定 Dialog 焦点管理，`MemoEditor.test.tsx` 固定编辑流程 Tab 顺序、可读名称、tooltip 和提醒触发点恢复，`memo-layout.contract.test.ts` 固定 40px 操作目标与窄宽度单列提醒表单，`accessibility.contract.test.ts` 固定焦点环、125% 文本缩放所需回流边界和减少动态效果。已有记录列表新增常驻新建入口；详情读取与草稿切换同步更新权威 ref，相关工作区测试覆盖新建和即时置顶竞态。阶段全量门禁为 39 个文件共 179 项测试通过，`pnpm run typecheck`、`pnpm run build` 和 `git diff --check` 同步通过。

备忘录中心最终检查点通过：前端串行全量为 39 个测试文件共 179 项，Rust 为 218 项库测试与 13 项集成/适配测试，共 231 项。`pnpm run typecheck`、`pnpm run build`、`cargo fmt --all -- --check`、`cargo test --offline --locked`、`cargo check --offline --locked --features desktop-app`、`cargo clippy --offline --locked --all-targets --features desktop-app -- -D warnings` 和 `git diff --check` 全部通过。Vite 预览在端口 1420 保持运行并通过平台代理连接检测。Windows MSVC 目标与安装态通知点击继续在原生 Windows 发布环境验收。

备份定向单元测试可在 `src-tauri/` 执行 `cargo test backup`。P9 属性测试默认运行 64 组随机业务图，验证版本化 JSON 序列化、解析、SQLite 导入和再次导出的规范化模型及摘要等价。

独立恢复集成测试执行 `cargo test --offline --locked --test backup_restore`。测试覆盖未知格式版本、版本 2 损坏引用、版本 1 确定转换、磁盘数据库替换与重开、四类备忘录数据恢复、恢复前快照解析、SQL 故障注入、原数据回滚和快照历史保留。

桌面核心流程集成测试可在 `src-tauri/` 执行：

```bash
cargo test --offline --locked --test desktop_core_flow
```

该测试使用内存 SQLite 与正式领域服务，系统通知通过内存发布器记录；执行环境无需启动 Tauri 窗口或 WebView。

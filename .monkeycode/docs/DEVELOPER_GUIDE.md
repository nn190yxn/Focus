# 开发指南

## 环境要求

- Windows 10/11 x64 开发机。
- Node.js 22 LTS。
- pnpm 11。
- Rust stable 与 Cargo。
- Microsoft Visual Studio Build Tools 2022，包含 Desktop development with C++。
- WebView2 Runtime。

## 开发约定

- TypeScript 开启严格模式，领域类型集中定义并由命令客户端复用。
- Rust 命令只负责参数解析和调用领域服务，业务规则放在 `domain` 与 `services`。
- SQLite 变更通过编号迁移文件完成，每个迁移包含升级验证测试。
- 所有时间持久化为 UTC，日期边界显式使用用户当前时区。
- Tauri capability 采用最小权限清单，每个插件权限单独评审。
- UI 文案通过国际化资源键管理，简体中文作为首发基准语言。

## 可用脚本

```bash
# 启动前端开发服务器
pnpm dev

# 启动 Tauri 桌面开发环境
pnpm tauri dev

# 运行 TypeScript 检查
pnpm typecheck

# 运行前端单元测试
pnpm test

# 运行 Rust 测试
cargo test --manifest-path src-tauri/Cargo.toml

# 验证桌面 feature
cargo test --manifest-path src-tauri/Cargo.toml --features=desktop-app --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --features=desktop-app -- -D warnings

# 构建 Windows 安装包
pnpm tauri build --bundles nsis
```

默认 Cargo feature 只编译跨平台核心库和测试；桌面二进制要求 `desktop-app` feature。`build.rs` 同样按该 feature 启用 Tauri 构建步骤，使 Linux 环境可以验证 Rust 核心边界。Windows 桌面开发和 NSIS 构建由 Tauri CLI 自动启用桌面目标所需配置。

pnpm 11 在 `pnpm-workspace.yaml` 中仅允许 `esbuild` 执行依赖构建脚本。新增带安装脚本的依赖时需要单独评审并更新 `allowBuilds`。

SQLite 迁移位于 `src-tauri/migrations/`，由 `repositories::database::run_migrations` 按版本在单事务内执行。迁移测试使用内存数据库，当前覆盖重复执行、批次回滚、写事务回滚、重复实例唯一约束、通知投递唯一约束以及项目和任务 CRUD。

项目业务规则位于 `domain::project`，数据库编排位于 `services::project_service`，Tauri 边界位于 `commands::project`。P5 属性测试从随机任务状态和专注时长构造记录，验证活动数、完成数、完成比例和累计专注秒数始终等于定义值。

任务校验位于 `domain::task`，事务编排位于 `services::task_service`，Tauri 边界位于 `commands::task`。任务创建和编辑以调用方提供的本地日期校验计划日期，检查项与任务在单个数据库事务中保存；任务移除更新为 `removed` 状态以保留历史关联。任务列表筛选由 `TaskListFilter` 校验日期范围，通过参数绑定查询，并以计划日期、计划时间、优先级、创建时间和任务标识提供确定顺序。

前端任务类型位于 `src/features/tasks/types.ts`，命令封装位于 `taskClient.ts`。`TaskEditor` 和 `TaskRow` 保持展示与持久化边界分离：组件通过属性接收 DTO 和回调，页面或数据适配器负责调用 Rust command。组件测试文件使用 `// @vitest-environment jsdom` 单独启用浏览器环境，覆盖字段校验、项目与检查项编辑、五种视觉状态及独立操作区域。

今日工作台模型位于 `src/features/today/todayModel.ts`，负责本地日期、周日期、TodayDigest 装饰、视觉状态、分类分区和日程排序等纯函数；`TodayWorkspace.tsx` 负责三列布局和操作入口。`App.tsx` 根据运行环境选择数据源：Tauri 桌面环境调用 `today_get_digest`，浏览器开发预览使用内存样例。普通任务编辑前调用 `task_get` 获取完整检查项；重复实例读取对应规则并进入范围编辑器。

Rust 今日汇总 DTO 位于 `domain/today.rs`，跨来源查询位于 `repositories/today_repository.rs`，日期校验、时区转换、去重和排序位于 `services/today_service.rs`。重复任务模板必须从普通任务候选中排除；候选集只接纳今日 pending/completed 与历史 pending 来源。扩展来源类型时需要定义稳定来源标识，并将来源加入排序末级键。

实例只保存 UTC `scheduled_at`，今日服务通过关联规则的 IANA 时区转换为本地 `HH:MM`。时间解析或时区数据异常返回 `TODAY_DATA_INVALID`。排序顺序固定为逾期、有时间、计划时间、优先级、创建时间、来源类型和来源标识。

前端重复计划类型和命令封装位于 `src/features/recurrence/`。`RecurrenceEditor` 维护每天、工作日、每周与每月结构化输入，并通过 `recurrenceSummary.ts` 即时输出自然语言摘要。`RecurrenceScopeEditor` 负责仅本次与未来计划范围，未来变更必须将规则版本递增一。今日行操作必须依据 `sourceKind` 分发，禁止将重复实例标识传给普通任务命令。

重复规则模型、校验器和日期展开位于 `src-tauri/src/domain/recurrence.rs`。模式 JSON 使用 camelCase 字段并通过 `kind` 判别；每周规则的 `weekdays` 使用 ISO 1–7 表示周一至周日。IANA 时区使用 `chrono-tz` 的 `Tz` 解析器校验，日期和本地时间继续使用 `chrono`。新增规则模式或字段时需同步 schema 序列化测试、日期展开匹配逻辑和接口文档。

重复规则与实例持久化位于 `repositories/recurrence_repository.rs`，触发编排和本地时间到 UTC 的转换位于 `services/recurrence_service.rs`。启动与日界线触发采用 missing-only 写入；规则与时区变化采用 refresh-pending 写入。修改冲突 SQL 时必须维持两个约束：同一规则和日期仅有一个实例，非 `pending` 历史实例不会被规则刷新覆盖。

实例操作同样位于 `RecurrenceService`。完成和跳过使用条件状态更新；今日延后只更新原日期的 `scheduled_at`；顺延在单事务内更新源实例并插入或复用明日实例。所有可变实例 SQL 同时检查 `active_focus` 与 `focus_sessions`，因此 pending 状态并非可操作性的充分条件。暂停和结束规则只更新规则状态与版本，`ended` 状态禁止后续转换。

计划变更范围由 `RecurrenceChangeScope` 表达。`thisInstance` 保持规则不变，只重算指定实例时间；`future` 要求候选版本等于当前版本加一，并在同一事务内保存规则和刷新生成范围内的 pending、未开始实例。新增实例状态或专注关联方式时，需要同步所有可操作性 SQL 条件。

定向验证重复计划可运行 `cargo test --manifest-path src-tauri/Cargo.toml recurrence`。当前测试覆盖规则与范围序列化、日期展开稳定顺序、规则范围与暂停状态、月末归一化、规则和首批实例原子创建、实例唯一性、重复运行幂等、缺失日期补生成、完成与跳过幂等、同日延后、唯一明日实例复用、规则暂停与结束、仅本次和未来范围变更、已开始实例保护、DST 不存在时间归一化以及无效范围和未知规则错误。

P1 属性测试位于 `services/recurrence_service.rs`，使用 64 组 `proptest` 场景随机组合四类规则、有效间隔、星期集合、1–31 月日期、四个代表性 IANA 时区、可选本地时间、最多 120 天范围和 2–6 次重复调度。测试同时比较规则展开的期望日期、实例数量和唯一日期集合，覆盖需求 R13.3 与 R13.9。定向运行命令为 `cargo test --manifest-path src-tauri/Cargo.toml p1_repeated_generation_keeps_one_instance_per_rule_and_date`。

P2 属性测试位于 `domain/recurrence.rs`，使用 256 组 `proptest` 场景随机组合四类有效规则、开放或有限结束日期、五个代表性 IANA 时区、可选本地时间，以及跨越规则起点和终点的查询范围。测试重复调用纯函数 `scheduled_dates`，验证输出完全一致、严格升序、无重复，并且每个日期都位于查询范围与规则有效期的交集内，覆盖需求 R13.1–R13.3 与 R13.10。定向运行命令为 `cargo test --manifest-path src-tauri/Cargo.toml p2_rule_generation_is_deterministic_and_strictly_ordered`。

P3 属性测试位于 `services/recurrence_service.rs`，使用 64 组临时 SQLite 场景生成初始实例，并分别创建已完成、已跳过和已有专注记录的受保护实例。测试随后修改任务模板标题及未来规则的模式、执行时间和 IANA 时区，从规则起点刷新范围，再逐字段比较受保护实例的完整记录，覆盖需求 R13.4、R13.6 与 R13.12。定向运行命令为 `cargo test --manifest-path src-tauri/Cargo.toml p3_future_changes_preserve_processed_and_started_instances`。

P4 属性测试位于 `services/today_service.rs`，使用 64 组临时 SQLite 场景随机组合普通任务、项目任务与重复实例，以及过去、今日、未来日期和 pending、completed、removed、skipped、rescheduled 状态。每组固定包含四类核心来源，测试将查询结果与按资格规则计算的精确来源集合比较，并检查来源标识唯一、条目类型和逾期派生，覆盖需求 R13.7 与 R13.9。定向运行命令为 `cargo test --manifest-path src-tauri/Cargo.toml p4_digest_contains_every_eligible_source_once`。

定向验证今日汇总可运行 `cargo test --manifest-path src-tauri/Cargo.toml today_service`。当前测试覆盖普通任务、项目任务和重复实例聚合，重复模板排除，逾期派生，实例时区转换，项目摘要，processed 实例过滤和重复查询顺序稳定性。

专注状态与转换位于 `domain/focus.rs`，SQLite 映射位于 `repositories/focus_repository.rs`，目标校验和事务编排位于 `services/focus_service.rs`，七个 Tauri 命令位于 `commands/focus.rs`。运行状态的剩余时间始终由 `target_ends_at - now` 校准，暂停状态使用已持久化剩余秒数。完成和重置通过同一事务插入 `focus_sessions` 并移除 `active_focus`；系统恢复事件与显式 `focus_reconcile` 命令都调用 `FocusService::reconcile`。

`FocusRepository::complete_due` 在一个数据库事务中读取活动状态、判断到期、解析项目关联、写入 `deadline` 会话并清除活动记录。到期会话使用原 `target_ends_at` 作为结束时间，使进程延迟或长时间休眠后的统计日期保持稳定。该操作返回可选会话，调用方可据此判定是否需要广播完成事件。

前端专注类型、纯显示逻辑、命令客户端和工作区组件位于 `src/features/focus/`。运行态倒计时必须从 `targetEndsAt` 与当前时间计算，UI interval 只刷新展示并在归零后调用 `focus_reconcile`。空格快捷键需要避让 `input`、`textarea`、`select`、`button` 和可编辑元素；完成确认对话框打开期间保持快捷键停用。

定向验证专注状态机可运行 `cargo test --manifest-path src-tauri/Cargo.toml focus`。当前测试覆盖时长和目标校验、暂停与继续的剩余时间、中断计数、直接状态标签序列化、单活动轮次、提前完成实际秒数、到时完成保护、重置的取消历史、时间前进与回拨校准、暂停期间长时间恢复、重复恢复幂等及八线程并发到期处理。

P6 属性测试位于 `services/focus_service.rs`，使用 128 组临时 SQLite 场景随机组合 1–180 分钟计划时长与 0–24 小时非负毫秒跳变。测试从持久化活动轮次调用 `reconcile_at`，将运行态剩余秒数和到期后的零剩余统一与 `max(target_ends_at - now, 0)` 比较，同时验证目标时间保持稳定、到期生成完成会话且换算误差小于 2 秒，覆盖需求 R3.2、R3.6 与 R4.5。定向运行命令为 `cargo test --manifest-path src-tauri/Cargo.toml p6_reconciliation_tracks_the_persisted_deadline_within_two_seconds`。

小组件配置模型位于 `domain/widget.rs`，双表仓储位于 `repositories/widget_repository.rs`，生命周期服务与 Tauri 边界分别位于 `services/widget_service.rs` 和 `commands/widget.rs`。前端 `widgetClient` 读取配置，`widgetModel` 负责三档容量、今日进度、任务目标映射和权威截止时间格式化，`WidgetApp` 监听 `widget://config-changed` 并复用 Today、Task、Recurrence 与 Focus clients。定向验证可运行 `cargo test --manifest-path src-tauri/Cargo.toml widget` 和 `pnpm exec vitest run src/features/widget/widgetModel.test.ts src/app/WidgetApp.test.tsx`。

桌面贴附状态机位于 `desktop/shell_attachment.rs`，可通过 `ShellHostAdapter` 替身测试宿主发现、附着、解除附着、句柄失效恢复和回退提示去重。Windows API 调用集中在 `desktop/windows_shell.rs`，使用 `windows-sys` 的最小 `Win32_Foundation` 与 `Win32_UI_WindowsAndMessaging` feature；新增 Win32 行为时应继续把 `unsafe` 限制在该文件。`desktop/widget_shell.rs` 负责 Tauri HWND 与事件映射，监视周期为 2 秒。定向测试命令为 `cargo test --manifest-path src-tauri/Cargo.toml shell_attachment`。

跨模块 Windows 适配器测试位于 `src-tauri/tests/widget_windows_adapter.rs`。测试替身允许运行时替换 Shell host，并记录附着和解除附着调用；场景覆盖 Explorer 宿主替换后的重新附着、单段故障提示去重与恢复、后续独立故障提示，以及桌面附着失败回退浮窗后通过 `WidgetService::unlock` 恢复鼠标交互和缩放。定向运行命令为 `cargo test --manifest-path src-tauri/Cargo.toml --test widget_windows_adapter`。

窗口策略与几何转换位于 `desktop/widget_window.rs`。`WidgetWindowBehavior` 以实际附着模式决定置顶状态，以锁定状态同步决定鼠标穿透和原生缩放；`WidgetWindowGeometry` 将物理窗口尺寸按 DPI 转为逻辑尺寸。Tauri 全局窗口事件只记录 widget 的移动、缩放与 DPI 变化，后台线程在 180ms 静默期后读取最终几何信息并通过 `WidgetService` 保存。锁定交互由 `WidgetApp` 提交配置，托盘 `widget_unlock` 菜单项提供恢复入口。定向验证可运行 `cargo test --manifest-path src-tauri/Cargo.toml widget_window` 和 `pnpm test -- src/app/WidgetApp.test.tsx`。

系统托盘实现位于 `desktop/tray.rs`。`TrayMenuModel` 是与 Tauri 解耦的状态映射，Ready 状态读取 TodayDigest 第一条 pending 来源，Running 与 Paused 状态解析普通任务或重复实例快照标题。菜单刷新线程每秒调用 `FocusService::reconcile`，负责后台剩余时间校准和到期事务；首次完成会广播 `focus://completed`。托盘动作通过现有 Focus、Today、Widget 服务执行，并以 `focus://state-changed`、`tray://quick-task` 和 `tray://open-focus` 同步前端。定向验证可运行 `cargo test --manifest-path src-tauri/Cargo.toml tray` 和 `pnpm exec vitest run src/app/App.test.tsx`。

通知领域类型、候选和时间窗口位于 `domain/notification.rs`，偏好位于 `domain/settings.rs`，投递去重与到时查询位于 `repositories/notification_repository.rs`，编排位于 `services/notification_service.rs`。Tauri 发布、15 秒 worker、连续扫描游标与 Windows 设置入口位于 `desktop/notifications.rs`。Windows 权限由系统管理，发布器允许提交并在设置页显示系统管理状态；发送失败写入稳定错误码，数据库唯一键保持单事件单记录。定向验证可运行 `cargo test --manifest-path src-tauri/Cargo.toml notification` 和 `pnpm exec vitest run src/features/settings/NotificationSettingsPanel.test.tsx`。

P7 属性测试位于 `services/notification_service.rs`，使用 64 组临时 SQLite 场景随机选择专注完成、普通任务到时或重复实例到时事件，并组合来源标识、计划日期时间、2–8 次重复处理、提示音偏好和发布成功或失败结果。测试通过实际 `NotificationService`、通知仓储和 SQLite 唯一键验证每个通知身份只保留一条投递记录，成功发布最多一次，覆盖需求 R3.5、R5.1 与 R13.8。定向运行命令为 `cargo test --manifest-path src-tauri/Cargo.toml p7_repeated_event_processing_creates_at_most_one_notification_record`。

桌面快捷键领域偏好位于 `domain/settings.rs`，JSON 持久化位于 `repositories/preferences_repository.rs`，Tauri global-shortcut 注册与冲突回滚位于 `desktop/shortcuts.rs`，命令和 autostart 系统同步位于 `commands/desktop_integration.rs`。快捷键变更必须保持“候选新增注册成功后再释放旧组合”的顺序，并在注册或数据库写入失败时恢复原运行态。任务栏进度模型位于 `desktop/tray.rs`，由现有每秒专注校准线程同步 Windows 任务栏。定向验证可运行 `cargo test --manifest-path src-tauri/Cargo.toml --features desktop-app --lib shortcuts`、`cargo test --manifest-path src-tauri/Cargo.toml --features desktop-app --lib autostart` 和 `cargo test --manifest-path src-tauri/Cargo.toml --features desktop-app --lib taskbar_progress`。

通用设置模型、SQLite JSON 持久化、服务和 Tauri 边界分别位于 `domain/settings.rs`、`repositories/preferences_repository.rs`、`services/settings_service.rs` 与 `commands/settings.rs`。新增通用偏好字段时需要同步 Rust 默认值与 Patch、TypeScript `GeneralPreferences`、设置面板、`settings://changed` 完整载荷和双窗口主题接入。语言偏好在国际化资源接入前只负责持久化；数据设置入口在备份模块完成前保持禁用。定向验证可运行 `cargo test --manifest-path src-tauri/Cargo.toml settings` 和 `pnpm exec vitest run src/features/settings/GeneralSettingsPanel.test.tsx src/theme/theme.test.ts src/app/App.test.tsx src/app/WidgetApp.test.tsx`。

窗口可见性核心 `restore_visible_rect` 使用物理坐标处理保存矩形和显示器工作区。工作区数组首项必须为主显示器，160 × 48 主要操作区完整位于任一工作区时保留保存坐标，越界时回退到首个可容纳主要操作区的工作区。P8 属性测试使用 256 组场景随机生成正尺寸窗口、负坐标、超大矩形及 1–5 个工作区，验证恢复后主要操作区可见，并验证已有可见位置保持稳定。定向运行命令为 `cargo test --manifest-path src-tauri/Cargo.toml p8_restored_window_keeps_its_main_action_area_visible`。

日历 DTO、仓储、服务和命令分别位于 `domain/calendar.rs`、`repositories/calendar_repository.rs`、`services/calendar_service.rs` 与 `commands/calendar.rs`。新增日历来源时需要同时维护仓储候选查询、`CalendarDay` 聚合逻辑和前端 `src/features/calendar/types.ts`。完成时间与专注结束时间必须先转换到 `CalendarQuery.timezone` 再确定本地日期，查询范围继续采用 UTC 半开区间以覆盖 DST。

日历定向验证可运行 `cargo test --manifest-path src-tauri/Cargo.toml calendar` 和 `pnpm exec vitest run src/features/calendar/calendarModel.test.ts src/features/calendar/CalendarWorkspace.test.tsx src/app/App.test.tsx`。Rust 用例覆盖周月年自然边界、闰年、零数据日期、重复模板排除、实例聚合、查询时区日期归属、DST、取消轮次排除，以及分类与项目筛选；前端用例覆盖周期位移、Monday-first 月网格、周期切换、筛选、导航和 App 接入。

统计 DTO、纯聚合与服务分别位于 `domain/statistics.rs` 和 `services/statistics_service.rs`，命令位于 `commands/statistics.rs`。统计始终接收完整 `CalendarQuery` 并复用 `CalendarService`，新增指标时应优先从 `CalendarPeriodResult` 聚合，保持筛选、自然周期和时区日界线一致。前端 `statisticsModel.ts` 提供浏览器预览的同口径聚合与年度月趋势转换，`StatisticsOverview.tsx` 只负责摘要、双指标趋势、项目投入和零数据行动入口。Rust 统计服务测试通过临时 SQLite 数据库覆盖专注轮次跨越本地午夜后按结束日期归属、零数据周返回完整七日趋势，以及项目筛选同步限制任务、专注和项目投入聚合。定向验证可运行 `cargo test --manifest-path src-tauri/Cargo.toml statistics` 和 `pnpm exec vitest run src/features/calendar/statisticsModel.test.ts src/features/calendar/StatisticsOverview.test.tsx src/features/calendar/CalendarWorkspace.test.tsx`。

便签与周目标类型、仓储、服务和命令分别位于 `domain/planning.rs`、`repositories/planning_repository.rs`、`services/planning_service.rs` 与 `commands/planning.rs`。周目标进度必须复用 `CalendarService` 和 `StatisticsSummary`，保证自然周、IANA 时区、日界线与有效专注定义一致。前端类型和命令封装位于 `src/features/today/planningTypes.ts` 与 `planningClient.ts`；`TodayWorkspace` 负责 500ms 防抖、`Ctrl+Enter` 保存反馈和目标表单，`App` 负责按选中日期与所在周加载数据。planning 服务测试验证完成任务、专注分钟与活跃天数在关联数据写入后重算，并检查目标封顶和回写值可再次读取。定向验证可运行 `cargo test --manifest-path src-tauri/Cargo.toml planning` 和 `pnpm exec vitest run src/features/today/TodayWorkspace.test.tsx src/app/App.test.tsx`。

## 测试策略

- 领域单元测试：任务状态、日期边界、计时状态机、统计聚合和备份校验。
- React 组件测试：任务操作、计时控制、筛选、主题和键盘可访问性。
- Rust 集成测试：SQLite 事务、迁移、恢复回滚和计时校准。
- 桌面端到端测试：托盘、通知、快捷键、关闭到托盘、重启恢复和安装升级。
- Windows 兼容测试：Windows 10 22H2 与 Windows 11 23H2，100% 和 125% 文本缩放，多显示器切换。

任务测试的当前基线为 119 项 Rust 核心库单元测试、3 项 Windows 适配器集成测试及 68 项 Vitest/jsdom 前端测试。Rust 用例覆盖计划日期边界和格式、筛选值校验、完成/恢复/移除状态、项目关联更新与解除、组合筛选、稳定排序、专注状态机、恢复校准、并发到期完成、日历自然周期与时区日界线、真实跨日专注结束日期归属、统计空周期、项目筛选与项目投入聚合、便签校验与持久化、周目标校验、指标映射、关联数据重算与持久化、通用设置默认值、Patch 合并和 SQLite 往返、小组件配置校验与双表持久化、Shell 附着与失效恢复、窗口置顶和穿透策略、DPI 几何转换、窗口越界恢复、解锁持久化、托盘菜单状态、主窗口关闭策略、通知偏好、到时查询、投递去重、发送失败记录、长时间休眠补偿、系统时间回拨、快捷键校验与冲突回滚、开机启动同步回滚、任务栏进度映射，以及 P1–P8 属性；Windows 适配器场景组合验证 Shell 附着、Explorer 重启、回退提示和解锁路径；前端用例验证编辑器表单语义、可访问错误、检查项操作、五态任务行、今日工作台、便签自动与快捷保存、周目标提交、重复范围编辑、主题属性、专注目标映射、时长边界、状态转换、自然结束、空格键焦点避让、日历周期切换与筛选、统计摘要、年度趋势、项目投入与空状态入口、小组件配置同步、锁定操作、浮窗回退提示、三档任务容量、详细元数据、完成、延后、专注快捷操作、托盘事件导航、通用设置保存错误、系统明暗解析、主窗口与小组件设置事件同步、通知设置失败回滚、开机启动同步和快捷键冲突恢复。阶段检查点同时要求 Clippy 零警告、Rust 格式检查、TypeScript 类型检查和 Vite 生产构建通过。

## 发布门禁

1. 类型检查、前端测试、Rust 测试和端到端核心流程全部通过。
2. 在干净 Windows 虚拟机完成安装、升级、卸载和 WebView2 前置条件验证。
3. 验证 8 小时后台运行、休眠恢复和跨日统计。
4. 验证备份导出、有效恢复、损坏文件拒绝和事务回滚。
5. 使用受保护的签名环境完成可执行文件与安装包签名。
6. 对安装包执行恶意软件扫描并生成 SHA-256 校验值。

## 参考资料

- Tauri 2 官方文档：`https://v2.tauri.app/`
- 参考产品：`https://8bfz9eam.showcase.monkeycode-ai.online/`

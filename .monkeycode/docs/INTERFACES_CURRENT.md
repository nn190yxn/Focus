# 抵达 Focus 当前接口补充

## 抵达 Focus 项目接口

前端类型化边界位于 `src/features/projects/projectClient.ts`，公开以下桌面调用：

```ts
interface ProjectClient {
  list(status: ProjectStatus | null, today: string): Promise<CommandResult<ProjectSummary[]>>;
  get(id: string, today: string): Promise<CommandResult<ProjectDetail>>;
  create(input: ProjectInput): Promise<CommandResult<ProjectRecord>>;
  update(id: string, input: ProjectInput): Promise<CommandResult<ProjectRecord>>;
  setStatus(id: string, status: ProjectStatus): Promise<CommandResult<ProjectRecord>>;
}
```

方法依次调用 `project_list`、`project_get`、`project_create`、`project_update` 和 `project_set_status`。`ProjectStatus` 支持 `active`、`paused`、`completed` 和 `archived`；`ProjectInput` 包含 `name`、`description`、`color`、`icon`、`startedOn` 和可空 `targetOn`。

`ProjectSummary` 组合项目记录、活跃/完成/总任务数、完成百分比、专注秒数、下一任务以及期限状态。期限状态支持 `none`、`overdue`、`atRisk` 和 `onTrack`。`ProjectDetail` 在摘要之外返回项目关联的 `TaskRecord[]`，供完成、恢复、添加任务和开始专注操作使用。

## 抵达 Focus 重复任务调度接口

Rust 调度器公开自动协调入口：

```rust
fn reconcile_active_to_utc_now(
    &self,
    trigger: GenerationTrigger,
    utc_now: DateTime<Utc>,
) -> Result<Vec<GenerationSummary>, DomainError>;
```

自动协调入口接受 `Startup` 或 `DayBoundary`。每条规则使用自身 `timezone` 将 `utc_now` 转换为本地日期；启动触发从 `startsOn` 回填，日界线触发只生成本地当天。未来才生效的规则返回零影响摘要，已暂停或结束的规则不会进入活跃规则集合。

`desktop::recurrence::reconcile_and_emit` 将各规则 `affectedCount` 相加。总数大于零时通过 Tauri Event Bus 广播 `today://changed`，payload 为空；主窗口和 Widget 将该事件视为重新读取权威 Today digest 的信号。

项目、任务与重复计划写 command 由 Tauri 自动注入 `AppHandle`，前端 invoke 参数保持原有协议。`after_today_change` 接受领域 `Result<T, DomainError>` 与事件回调：成功结果执行一次回调并保留原值，失败结果直接保留原错误。以下成功写入会发送空 payload 的 `today://changed`：`project_create`、`project_update`、`project_set_status`、`project_remove`、`task_create`、`task_update`、`task_set_completed`、`task_remove`、`task_set_check_item_completed`、`task_reorder_check_items`、`recurrence_create`、`recurrence_update`、`recurrence_set_status`、`instance_complete`、`instance_skip`、`instance_delay_today` 和 `instance_reschedule_tomorrow`。

## 抵达 Focus 通知重试接口

`NotificationRepository::reserve` 以 `(kind, source_id, scheduled_for)` 为唯一投递身份，并返回 `NotificationReservation`。首次预留插入带 60 秒 lease 的 `pending` 记录并返回 `Acquired`；已有 `failed` 或 lease 已过期的 `pending` 记录通过同一原子语句恢复为 `pending`，刷新 lease 时间与提示音偏好、清除旧错误并返回 `Acquired`；活动 `pending` 返回 `InFlight`；`sent` 返回 `AlreadySent`。接管过程复用原投递记录，因此记录数量保持幂等。

`NotificationService::reconcile_task_reminders` 处理提醒窗口内全部候选并返回成功发送数。发布失败会先把对应记录更新为 `failed`；活动 lease 会生成内部 `NOTIFICATION_DELIVERY_IN_FLIGHT`；批次结束后返回首个 `DomainError`。desktop worker 将 reconciliation 结果传给 `next_reminder_scan_cursor`：成功结果返回当前扫描时间，失败结果返回原扫描时间，下一轮因而覆盖失败或中断投递的计划时间，直到 lease 过期后重新接管。

## 抵达 Focus 专注状态事件接口

专注写 command `focus_start`、`focus_pause`、`focus_resume` 和 `focus_reset` 成功后发送 `focus://state-changed`，payload 为对应 `FocusState` 判别联合。`focus_finish` 成功后先发送保存的 `FocusSession` 作为 `focus://completed` payload，再发送 ready 状态。`focus_reconcile` 与 `focus_get_state` 在协调出自动完成轮次时发送同一组事件。

`FocusState` 的 `state` 支持 `ready`、`running` 和 `paused`。主窗口与 Widget 将 `focus://state-changed` payload 直接用于即时 UI 同步；Widget 的周期 `focus_get_state` 调用承担权威状态校准。事件发送属于已提交写入后的通知，事件总线失败不会改变 command 的领域结果。

`focus_start` 在创建活动专注前校验目标任务与关联项目。普通任务读取当前 `project_id`，重复实例读取 `snapshot_project_id`；关联项目处于 `paused` 状态时返回稳定错误码 `FOCUS_PROJECT_PAUSED`。前端将其映射为“该项目已暂停，请恢复项目后开始专注。”或 “This project is paused. Resume it before starting focus.”。托盘候选列表跳过暂停项目中的待处理任务，并继续选择下一条符合条件的任务。

## 抵达 Focus 国际化接口

国际化公开类型位于 `src/i18n/`：

```ts
type LanguagePreference = "system" | "zhCn" | "en";
type SupportedLocale = "zh-CN" | "en-US";

interface I18nValue {
  locale: SupportedLocale;
  t: (key: MessageKey, params?: Record<string, string | number>) => string;
  formatDate: (value: Date | string, options?: Intl.DateTimeFormatOptions) => string;
  formatTime: (value: Date | string, options?: Intl.DateTimeFormatOptions) => string;
  formatRelativeTime: (value: number, unit: Intl.RelativeTimeFormatUnit) => string;
}
```

`resolveLocale` 对显式偏好执行固定映射；系统偏好读取首个可用系统语言，以 `zh` 开头时解析为 `zh-CN`，其余语言解析为 `en-US`。设置服务通过 `settings://changed` 向主窗口和小组件发送完整 `GeneralPreferences`，两个窗口据此即时刷新 locale。

`MessageKey` 由简体中文资源键推导，英文资源必须提供同一组键。翻译模板使用 `{name}` 形式参数；仅日期字符串 `YYYY-MM-DD` 会按本地中午构造日期，避免 UTC 转换造成跨日。

## 抵达 Focus 无障碍组件接口

`Dialog` 的公开属性保持为 `open`、`title`、`children` 和 `onClose`。组件负责初始焦点、Tab 焦点循环、Escape 关闭、关闭后焦点恢复、`aria-modal` 和唯一标题关联；调用方只需控制打开状态并提供标题。内容中带 `autofocus` 的可操作元素优先获得初始焦点，其余场景聚焦首个可操作元素。

`SegmentedControl<T>` 接收可读组名、只读选项数组、当前值和变更回调：

```ts
interface SegmentedControlProps<T extends string> {
  label: string;
  options: readonly { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
}
```

当前值是唯一进入顺序导航的 radio；方向键循环到相邻选项，Home 和 End 跳到边界选项并同步调用 `onChange`。任务行和小组件的完成、打开、专注、继续、延后、顺延与跳过操作使用包含任务标题的本地化 `aria-label`。

`SemanticThemeTokens` 在基础画布、表面、正文、边框和强调色之外公开 `accentContrast`、`focusRing`、`success`、`warning` 与 `danger`。`themeStyle` 将它们映射为共享 CSS 自定义属性。小组件通过 `--widget-opacity` 仅调整背景颜色与透明背景的混合比例。

## 抵达 Focus 主窗口状态接口

主窗口设备状态由 Rust `MainWindowState` 表达，并通过 `PreferencesRepository` 的 `get_main_window_state` 与 `set_main_window_state` 读写：

```rust
struct MainWindowState {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    maximized: bool,
    monitor_id: Option<String>,
    scale_factor: f64,
}
```

`x`、`y` 使用 Tauri 物理坐标；`width`、`height` 使用逻辑尺寸；`scale_factor` 记录保存时 DPI 比例。所有数值必须有限，宽高和 DPI 比例必须大于零。缺失、字段不完整或值域无效的状态按首次启动处理。持久化 JSON 使用 camelCase，并以 `mainWindowState` 为键写入 `preferences.value_json`。

桌面生命周期公开 Rust 内部接口：

- `restore_main_window(app)`：应用保存尺寸，修正显示器边界，恢复最大化状态并显示主窗口。
- `persist_main_window_state(app)`：保存普通窗口矩形或更新最大化标记；最小化窗口不覆盖既有状态。
- `show_main_window(app)`：供单实例、托盘和快捷键激活现有窗口。
- `persist_before_exit(app)`：依次校准活动专注并保存主窗口、小组件状态；`WIDGET_WINDOW_MISSING` 表示无可保存的 Widget 几何并允许退出继续，其他窗口操作或存储错误继续返回失败。
- `request_exit(app, code)`：持久化成功后请求 Tauri 退出。

`activate_existing_instance_window(target)` 是 `show_main_window` 使用的可测试激活边界，依次执行显示、取消最小化和聚焦，任一步失败即返回错误。`restored_main_window_rect(state, work_areas)` 将持久化状态转换为物理矩形并复用共享可见区域修正算法；桌面适配层再把结果应用到 Tauri 窗口。

## 抵达 Focus Widget Shell 接口

`ShellAttachmentOutcome` 是 Shell monitor 向 Tauri 窗口适配层输出的内部契约。`DesktopAttached` 映射到 `AppliedShellMode::Desktop` 和 `always_on_top = false`；`Floating`、`FloatingFallback` 映射到 `AppliedShellMode::Floating` 和 `always_on_top = true`。仅 `DesktopAttached { recovered: true }` 表示从失效或回退状态完成恢复。

Widget 前端订阅以下 Shell 生命周期事件：

- `widget://mode-fallback`：桌面附着失败时发送，payload 包含 `fromMode: "desktop"`、`toMode: "floating"` 和 `reason`；同一连续失败周期只发送一次。
- `widget://mode-restored`：桌面宿主重新附着且原生层级恢复后发送，payload 为空；前端清除当前回退提示。

## 抵达 Focus 更新接口

- `update_check()`：访问构建时配置的 HTTPS endpoint，返回可选的 `UpdateMetadata`，并在进程内保存待下载更新。
- `update_download()`：下载并验证当前待更新包，返回最终下载进度；实时进度通过 `update://download-progress` 发送。
- `update_install()`：要求已下载并通过签名验证的包，先持久化退出状态，再启动安装和应用重启。

```ts
interface UpdateMetadata {
  currentVersion: string;
  version: string;
  notes: string | null;
  publishedAt: number | null;
}

interface UpdateDownloadProgress {
  downloaded: number;
  contentLength: number | null;
}
```

更新 command 继续使用 `CommandResult<T>`。稳定错误码覆盖 `UPDATE_NOT_CONFIGURED`、`UPDATE_CONFIGURATION_INVALID`、`UPDATE_CHECK_FAILED`、`UPDATE_DOWNLOAD_FAILED`、`UPDATE_INSTALL_FAILED`、`UPDATE_BUSY`、`UPDATE_NOT_AVAILABLE` 和 `UPDATE_NOT_DOWNLOADED`；前端不展示底层 endpoint、路径、签名或插件错误。

## 抵达 Focus Windows 安装发布配置

NSIS 安装配置位于 `src-tauri/tauri.conf.json`：

```json
{
  "bundle": {
    "targets": ["nsis"],
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper",
        "silent": true
      },
      "nsis": {
        "installMode": "currentUser",
        "languages": ["SimpChinese", "English"],
        "displayLanguageSelector": true,
        "startMenuFolder": "抵达 Focus"
      }
    }
  }
}
```

Tauri 标准 NSIS 模板负责安装目录页和桌面快捷方式选择；`startMenuFolder` 声明开始菜单目录。签名覆盖接口位于 `src-tauri/tauri.windows-signing.conf.example.json`，公开字段为 `certificateThumbprint`、`digestAlgorithm` 和 `timestampUrl`。真实 thumbprint 只写入被忽略的 `src-tauri/tauri.windows-signing.conf.json`，签名构建通过 `pnpm tauri:build:windows:signed` 合并该配置。

`pnpm tauri:build:windows` 显式传递 `--features desktop-app` 并合并 `src-tauri/tauri.windows-unsigned.conf.json`；该覆盖配置将 `bundle.createUpdaterArtifacts` 设为 `false`，用于无需 updater 私钥的本地安装包验收。`pnpm tauri:build:windows:signed` 同样显式启用 `desktop-app`，合并本地 Authenticode 配置，并沿用基础配置中的 updater 产物生成设置。

指定 `--target x86_64-pc-windows-msvc` 时，无签名应用输出到 `src-tauri/target/x86_64-pc-windows-msvc/release/arrive-focus.exe`，NSIS 安装包输出到 `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`。release 入口通过 `windows_subsystem = "windows"` 固定为 Windows GUI 子系统，验包时应检查 PE 架构、GUI subsystem、NSIS 文件类型和 SHA-256；Windows 发布机继续检查 Authenticode 状态和安装生命周期。

## 抵达 Focus command 错误接口

所有 Rust command 使用同一判别联合协议，`ok` 在 JSON 中为布尔值：

```ts
type CommandResult<T> =
  | { ok: true; data: T; version: number }
  | { ok: false; error: DomainError };

interface DomainError {
  code: string;
  message: string;
  field?: string;
}
```

`invokeCommand` 将 Tauri 调用层异常归一化为 `COMMAND_INVOCATION_FAILED`，并把失败 command 写入 `field`。包装器随后调用 `diagnostic_command_failure(command, error)`，由 Rust 日志层记录经过单行、长度限制和字符过滤的 command 与 Tauri 拒绝原因；诊断调用失败会被隔离。组件不得直接展示 `DomainError.message`；`domainErrorMessage` 根据 `code` 映射 `MessageKey`，精确覆盖任务日期、项目历史、便签、专注、通知、快捷键和备份确认等高频场景，其中 `FOCUS_PROJECT_PAUSED` 提供恢复项目后再开始专注的双语操作提示，并为存储、备份、重复计划、桌面集成、输入冲突和未知错误提供安全分类文案。

Rust command 失败诊断事件格式为 `event=domain_error context=<context> code=<code> field=<field>`。诊断字段只接受 ASCII 字母、数字、下划线、连字符、点和冒号，单字段最长 120 个字符；事件不包含 `message` 或 command 请求参数。

## 抵达 Focus 备忘录数据库接口

SQLite schema 版本 4 增加 `memos`、`memo_tags`、`memo_tag_links` 和 `memo_reminders`。备忘录标题上限为 200 字符，正文上限为 20000 字符；标签规范名全局唯一，单条备忘录与标签通过复合主键关联；每条备忘录最多关联一条提醒定义。提醒支持 `once` 与 `recurring`，重复频率支持 `daily`、`weekdays`、`weekly` 和 `monthly`，到期扫描索引为 `(status, next_scheduled_for)`。

`notification_deliveries.kind` 增加 `memoReminder`。投递身份继续使用 `(kind, source_id, scheduled_for)` 唯一约束，既有专注完成、任务到时和重复任务到时记录在版本 4 迁移期间完整复制。

Rust 备忘录输入协议由 `MemoInput`、`MemoListQuery`、`MemoReminderInput` 和 `MemoReminderRule` 定义，并通过 serde 输出 camelCase 字段。提醒 `kind` 为 `once` 时使用 `scheduledLocal` 与 `timezone`；为 `recurring` 时使用 `frequency`、`interval`、`weekdays`、`monthlyDay`、`localTime`、`startsOn`、`endsOn` 和 `timezone`。

`MemoReminderService::next_occurrence(schedule, after)` 返回严格晚于 `after` 的下一 UTC 时间；重复规则已越过 `endsOn` 时返回空值。`daily`、`weekly` 和 `monthly` 使用正整数 `interval` 表达自定义间隔，`weekdays` 固定按周一至周五执行且要求 `interval = 1`。每月的 `monthlyDay` 会收敛到目标月份最后一个自然日。

`MemoReminderService::prepare_rule(memo_id, current, schedule, now)` 为创建或更新准备持久化提醒。新规则生成 ID，更新规则保留原 ID 与 `createdAt`，取消提醒返回空值；下一发生时间为空时状态直接设为 `completed`。`MemoRepository::create` 与 `update` 在核心字段和标签事务中同步保存该结果。

`MemoRepository::list_due_reminders(now, untitled_label)` 返回 `status = active` 且 `nextScheduledFor <= now` 的 `DueMemoReminder[]`，结果按发生时间和提醒 ID 升序排列。每项包含完整 `MemoReminderRule`、备忘录 ID 和派生后的本地化显示标题。

`MemoReminderService::reconcile_due(now, untitled_label, deliver)` 逐项调用投递回调并返回成功投递数量。投递失败的提醒保留原发生时间，同批后续提醒继续处理，最终返回首个错误。`advance_after_delivery` 将一次提醒设为 `completed`，将重复提醒推进到严格晚于当前发生时间的下一时刻；没有后续有效日期时设为 `completed`。

`MemoRepository::advance_reminder(id, expected_scheduled_for, next_scheduled_for, status, updated_at)` 仅更新 ID、active 状态和旧发生时间全部匹配的记录，并返回是否实际更新。该比较更新防止陈旧扫描结果覆盖已推进状态。

`NotificationService::reconcile_memo_reminders(now, untitled_label, publisher)` 读取通知偏好并协调全部到期备忘录提醒。每次发生使用 `kind = memoReminder`、`source_id = reminder.id` 和 `scheduled_for = nextScheduledFor` 作为唯一身份；通知标题固定为“备忘录提醒”，正文只使用派生后的显示标题，提示音遵循全局通知偏好。

协调器复用 `NotificationRepository::reserve` 的三态结果：`Acquired` 调用 publisher 并记录 `sent` 或 `failed`，`InFlight` 返回 `NOTIFICATION_DELIVERY_IN_FLIGHT` 并保留提醒状态，`AlreadySent` 跳过 publisher 并继续推进提醒。`failed` 记录和超过 60 秒的 `pending` lease 可在后续周期重新获取，从而覆盖发布失败重试与进程中断恢复。

备忘录通知的 `SystemNotification.activation` 为 `OpenMemo { memo_id }`，专注和任务通知不设置激活数据。Windows 通知主体点击后，桌面层要求 `memo_id` 是规范小写连字符 UUID，随后显示主窗口并广播 `memo://open-requested`；事件 payload 是备忘录 ID 字符串，不包含标题、正文或标签。窗口激活完成前不会发送事件，订阅方收到事件后通过 `memo_get` 读取权威详情。

前端 `memoClient` 公开 `list(query)`、`get(id)`、`create(input)`、`update(id, input)`、`remove(id)` 和 `listTags()`。这些方法分别映射到 `memo_list`、`memo_get`、`memo_create`、`memo_update`、`memo_remove` 与 `memo_tag_list`，返回统一的 `CommandResult<T>`。

六个备忘录 Tauri commands 已注册到桌面 invoke handler。读取命令直接查询 SQLite 权威数据；创建和更新命令先执行领域字段及标签规范化、准备提醒规则，再通过 Repository 事务保存核心字段、标签和提醒；删除命令使用 Repository 事务级联边界。所有领域失败统一经 `CommandResult::from_result` 返回并只记录模块、错误码和字段名。

`memo_create`、`memo_update` 和 `memo_remove` 由 Tauri 自动注入 `AppHandle`，前端 invoke 参数保持现有协议。三个写命令成功提交后发送一次空 payload 的 `memo://changed`；失败命令返回原 `DomainError` 且不发送事件。主窗口订阅方将事件视为重新读取 SQLite 权威数据的失效通知。

主窗口将 `memo://changed` 映射为递增的 `dataRevision`，将 `memo://open-requested` 的字符串 ID 映射为 `{ memoId, sequence }`。`sequence` 每次接收事件时递增，使相同 ID 的连续通知点击仍可被页面逐次处理；两者通过 `MemoWorkspaceProps` 传入备忘录页面，页面不把事件 payload 作为详情数据使用。

`MemoWorkspaceProps` 还允许注入 `runtime`、`MemoClient` 和初始 `MemoListQuery`，用于浏览器预览和组件测试。正式桌面运行时，`dataRevision` 或查询变化触发 `memo_list`；`openRequest.memoId` 触发 `memo_get`。`MEMO_NOT_FOUND` 会被解释为详情失效信号并触发列表权威刷新，其他错误通过稳定错误码映射为安全文案。

`MemoWorkspaceProps.onQueryChange` 在已提交的搜索词或标签 ID 变化时返回完整 `MemoListQuery`。主窗口用该回调保存最近条件，并在备忘录页面重新挂载时通过 `initialQuery` 恢复；搜索输入的 200 毫秒防抖发生在回调之前，标签选择和清除条件会立即回调。

`MemoEditor` 接收可空的权威 `MemoRecord`、保存状态与安全错误，并通过 `onSave(MemoInput)` 提交完整草稿。标题和正文分别限制为 200 与 20000 个 Unicode 字符；保存按钮和 Ctrl/Cmd+Enter 使用同一回调。组件不生成标签 ID；已有记录由 `MemoWorkspace` 调用 `memo_update`，新草稿首次有效保存调用 `memo_create`，成功响应中的 `MemoRecord` 成为后续编辑的权威记录。保存失败时标题和正文草稿继续保留，标签与置顶值按传入的权威记录恢复。

`MemoEditor` 在草稿与最后请求输入不同时启动 500 毫秒自动保存计时，新的编辑会替换待保存快照。`MemoWorkspace` 对 `onSave` 请求执行单通道串行编排：活动请求期间的新输入覆盖 `queuedSave`，活动请求结束后只提交最新排队值。新草稿的首个请求使用 `create(input)`；创建成功且存在排队值时，后续请求使用返回 ID 调用 `update(id, queuedInput)`，避免重复创建。保存错误通过 `saveError` 返回，编辑器展示稳定错误和“重新保存”入口。

`MemoReminderEditorProps` 包含 `open`、可空 `schedule`、`saving`、可注入 `now`、`onClose()` 和 `onSave(MemoReminderSchedule)`。组件直接输出现有 once/recurring 判别联合：星期使用 Rust 协议的 1 至 7；`weekdays` 固定 interval 为 1；daily 与 weekdays 清空 `weekdays` 和 `monthlyDay`；weekly 至少包含一个星期；monthly 包含 1 至 31 的 `monthlyDay`。验证错误使用双语资源并聚焦对应控件。

`MemoReminderPermissionNotice` 在存在活动草稿提醒且处于 Tauri runtime 时调用 `NotificationClient.getSettings()`。权限为 denied 时显示说明，并通过 `NotificationClient.openSystemSettings()` 打开 Windows 通知设置；浏览器预览不请求桌面权限。

备忘录简体中文和英文资源共享完全相同的键集合，覆盖导航、加载、空状态、筛选、表单、提醒摘要与验证、权限说明、删除确认、保存状态和稳定错误。`MemoEditor` 的图标加文字按钮保留可读名称；标签移除图标按钮同时提供 `aria-label` 与同文案 `title`。提醒和删除 Dialog 关闭后由共享组件恢复触发元素焦点。

`LatestMemoSaveQueue<T>` 公开 `enqueue(value): T | null`、`complete(): T | null` 和 `isActive(): boolean`。调用方只执行 `enqueue` 或 `complete` 返回的非空值，因此并发编辑会收敛为当前活动请求之后的一次最新值保存。

`MemoEditor` 的删除边界由 `deleting`、`deleteError` 和 `onDelete()` 构成。新草稿不显示删除入口；权威记录确认删除后，`MemoWorkspace` 使用当前记录 ID 调用 `MemoClient.remove(id)`。成功响应清除 `selectedId`、`selectedMemo` 和草稿状态并切回列表；失败响应保持当前详情，通过 `MEMO_DELETE_FAILED` 的安全映射显示错误，并推进内部刷新版本重新读取权威列表。

command 接线测试在前端逐项固定六个 invoke 名称与参数对象，在 Rust 侧通过内存 SQLite 固定创建、更新、标签替换、取消置顶、删除及缺失记录错误。`after_memo_change` 单元测试独立固定成功一次回调和失败零回调，因此事件行为与数据库编排均可在默认测试配置中验证。

标签规范名是去除首尾空白后的 Unicode 小写字符串。单个输入中的空白和大小写变体合并为一个标签，并保留首次输入的显示名称；数据库中的 `normalized_name` 唯一约束保证跨备忘录复用同一标签实体。

备忘录删除领域错误使用稳定代码：目标不存在时为 `MEMO_NOT_FOUND`，SQLite 删除事务失败时为 `MEMO_DELETE_FAILED`。删除失败消息为固定安全文案，不包含 SQL、标题、正文、标签或底层数据库错误详情。

Repository 保存失败使用 `MEMO_SAVE_FAILED`，持久化提醒行无法恢复为领域联合时使用 `MEMO_REMINDER_DATA_INVALID`。`MemoRecord` 聚合核心字段、按规范名排序的标签、可选提醒、显示标题和审计时间；读取不存在的 ID 返回空值，由 service/command 边界映射为 `MEMO_NOT_FOUND`。

备忘录稳定错误码包括：`MEMO_TITLE_INVALID`、`MEMO_BODY_INVALID`、`MEMO_SEARCH_INVALID`、`MEMO_TAG_ID_INVALID`、`MEMO_TAG_INVALID`、`MEMO_TAG_LIMIT_EXCEEDED`、`MEMO_NOT_FOUND`、`MEMO_SAVE_FAILED`、`MEMO_DELETE_FAILED`、`MEMO_REMINDER_TIME_INVALID`、`MEMO_REMINDER_DATE_INVALID`、`MEMO_REMINDER_INTERVAL_INVALID`、`MEMO_REMINDER_WEEKDAYS_INVALID`、`MEMO_REMINDER_MONTHLY_DAY_INVALID`、`MEMO_REMINDER_TIMEZONE_INVALID`、`MEMO_REMINDER_DATA_INVALID`、`MEMO_NOTIFICATION_ACTIVATION_INVALID` 和 `MEMO_NOTIFICATION_ACTIVATION_FAILED`。前端对标题、正文、标签、记录失效、保存、删除及提醒错误提供精确双语安全文案，搜索与标签 ID 验证沿用通用输入提示。

`MemoRepository::list` 接受 `MemoListQuery { search, tagId }`。`search` 在标题、正文和标签名称中执行不区分 ASCII 大小写的字面量包含匹配，`tagId` 使用 EXISTS 子查询筛选；两者同时提供时采用交集语义。排序键固定为 pinned、`pinnedAt DESC`、`updatedAt DESC`、`id ASC`。

`memo_tag_list` 返回 `MemoTagSummary[]`。每项包含 `id`、`name` 和 `memoCount`，其中 `memoCount` 是调用时关联该标签的备忘录数量；零关联标签不会出现在结果中。

## 抵达 Focus 备份 JSON 接口

`BackupService::export_json()` 从 SQLite 业务表生成稳定字段顺序的 pretty JSON；`BackupService::parse_json(input)` 返回通过预校验的 `ValidatedBackup`。当前导出格式版本为 `2`，解析器接受版本 1 和版本 2，单个 JSON 输入上限为 128 MiB。版本 1 输入会在解析时补齐空的备忘录集合。

```json
{
  "formatVersion": 2,
  "exportedAt": "2026-07-21T09:00:00.000Z",
  "data": {
    "projects": [],
    "tasks": [],
    "checkItems": [],
    "recurrenceRules": [],
    "taskInstances": [],
    "focusSessions": [],
    "activeFocus": null,
    "notes": [],
    "weeklyGoals": [],
    "preferences": [],
    "memos": [],
    "memoTags": [],
    "memoTagLinks": [],
    "memoReminders": []
  }
}
```

成功解析后，`BackupImportSummary` 返回十四类记录数量、总记录数、`earliestDate` 和 `latestDate`。格式错误使用 `BACKUP_FORMAT_INVALID`，未知版本使用 `BACKUP_VERSION_UNSUPPORTED`，超限输入使用 `BACKUP_FILE_TOO_LARGE`；字段和值域错误通过稳定备份错误码和可选字段路径报告。备忘录集合会校验长度、规范标签名、唯一性、标签和提醒数量、跨集合引用、提醒枚举、字段组合、本地日期时间、IANA 时区，以及活动提醒的下一 UTC 发生时间。

主窗口公开三个 Tauri command：

- `backup_export`：打开原生保存对话框，写入 JSON 并返回 `BackupExportResult | null`；用户取消时返回 `null`。
- `backup_inspect`：打开原生文件选择对话框，完成限量读取和全量预校验，返回包含确认令牌、格式版本、导出时间和摘要的 `BackupInspection | null`。
- `backup_restore`：接收 `{ token: string }`，消费已校验数据，在单事务内创建恢复前快照并替换业务数据，返回 `BackupRestoreResult`。

`BackupRestoreResult` 包含 `sourcePath`、`snapshotPath` 和导入摘要。恢复失败使用 `BACKUP_RESTORE_FAILED`，快照创建失败使用 `BACKUP_SNAPSHOT_FAILED`，确认令牌缺失或失效使用 `BACKUP_CONFIRMATION_INVALID`。成功恢复后发送 `backup://restored` 事件，并重新广播当前通用设置。

## 抵达 Focus 核心流程测试边界

`src-tauri/tests/desktop_core_flow.rs` 直接组合 `ProjectService`、`TaskService`、`RecurrenceService`、`RecurrenceScheduler`、`TodayService`、`WidgetService`、`NotificationService`、`FocusService`、`CalendarService`、`StatisticsService` 和 `BackupService`。测试只替换 `NotificationPublisher` 外部系统通知端口，领域输入、SQLite repository、重复实例约束、通知投递记录、日历统计和备份 JSON 接口均使用生产实现。

该测试固定以下跨模块契约：同一规则和日期只生成一个实例；今日摘要同时包含普通任务和重复实例；默认小组件包含今日进度与任务模块；同一到时事件只发布一次通知；已完成任务和有效专注出现在项目周历及统计摘要中；导出的版本 2 备份可重新检查，并包含项目、任务、重复规则、任务实例和专注记录。

## 抵达 Focus Windows 安装烟测接口

`scripts/windows-installer-smoke.ps1` 的必填参数为 `BaselineInstallerPath` 和 `UpgradeInstallerPath`，两者必须指向不同的 NSIS `.exe` 文件。可选参数 `InstallDirectory`、`AppDataDirectory`、`ExecutableName`、`UninstallerName` 和 `LaunchTimeoutSeconds` 用于匹配发布机环境；默认应用数据目录为当前用户漫游配置目录下的 `com.arrive.focus`。

脚本成功时输出包含 `baselineVersion`、`upgradeVersion`、`installDirectory`、`appDataDirectory` 和 `dataPreserved` 的 JSON。任一安装进程返回非零状态、应用未在时限内创建数据库、升级后可执行文件哈希未变化、卸载后程序文件仍存在，或升级与卸载阶段的数据标记不一致时，脚本抛出错误并返回失败状态。

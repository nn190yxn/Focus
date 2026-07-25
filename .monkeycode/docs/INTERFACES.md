# 接口定义

## 接口原则

前端通过类型化命令调用 Rust 业务层，通过事件接收计时和系统状态变化。每个命令返回统一结果结构，错误包含稳定错误码、用户可读消息和可选诊断信息。

```ts
type CommandResult<T> =
  | { ok: true; data: T; version: number }
  | { ok: false; error: { code: string; message: string } };
```

当前 Rust 端已注册 `health`、项目域、任务与检查项、重复规则、任务实例、今日汇总、专注、日历统计、便签与周目标、通用设置、小组件和通知命令。其余表格中的命令是后续业务阶段的稳定接口规划，完成实现后逐项注册和测试。

| 已实现命令 | 输入 | 输出 |
|------------|------|------|
| `health` | 无 | `CommandResult<"ready">` |
| `project_create` | `ProjectInput` | `CommandResult<ProjectRecord>` |
| `project_update` | 项目标识、`ProjectInput` | `CommandResult<ProjectRecord>` |
| `project_set_status` | 项目标识、目标状态 | `CommandResult<ProjectRecord>` |
| `project_remove` | 项目标识、历史处理策略 | `CommandResult<void>` |
| `project_list` | 可选状态、本地日期 | `CommandResult<ProjectSummary[]>` |
| `project_get` | 项目标识、本地日期 | `CommandResult<ProjectDetail>` |
| `task_list` | `TaskListFilter` | `CommandResult<TaskListItem[]>` |
| `task_get` | 任务标识 | `CommandResult<TaskDetail>` |
| `task_create` | `TaskInput`、本地日期 | `CommandResult<TaskDetail>` |
| `task_update` | 任务标识、`TaskInput`、本地日期 | `CommandResult<TaskDetail>` |
| `task_set_completed` | 任务标识、完成状态 | `CommandResult<TaskDetail>` |
| `task_remove` | 任务标识 | `CommandResult<void>` |
| `task_set_check_item_completed` | 任务标识、检查项标识、完成状态 | `CommandResult<TaskDetail>` |
| `task_reorder_check_items` | 任务标识、有序检查项标识 | `CommandResult<TaskDetail>` |
| `recurrence_get` | 规则标识 | `CommandResult<RecurrenceRule>` |
| `recurrence_create` | 规则、生成日期范围 | `CommandResult<GenerationSummary>` |
| `recurrence_update` | 候选规则、变更范围、生成结束日期 | `CommandResult<GenerationSummary>` |
| `recurrence_set_status` | 规则标识、暂停或结束状态 | `CommandResult<RecurrenceRule>` |
| `instance_complete` | 实例标识 | `CommandResult<TaskInstanceRecord>` |
| `instance_skip` | 实例标识 | `CommandResult<TaskInstanceRecord>` |
| `instance_delay_today` | 实例标识、本地时间 | `CommandResult<TaskInstanceRecord>` |
| `instance_reschedule_tomorrow` | 实例标识 | `CommandResult<TaskInstanceRecord>` |
| `today_get_digest` | 本地日期 | `CommandResult<TodayDigest>` |
| `focus_get_state` | 无 | `CommandResult<FocusState>` |
| `focus_reconcile` | 无 | `CommandResult<FocusReconcileResult>` |
| `focus_start` | `FocusTarget`、计划分钟数 | `CommandResult<FocusState>` |
| `focus_pause` | 无 | `CommandResult<FocusState>` |
| `focus_resume` | 无 | `CommandResult<FocusState>` |
| `focus_reset` | 无 | `CommandResult<FocusState>` |
| `focus_finish` | 完成方式 | `CommandResult<FocusSession>` |
| `settings_get` | 无 | `CommandResult<GeneralPreferences>` |
| `settings_update` | `GeneralPreferencesPatch` | `CommandResult<GeneralPreferences>` |
| `desktop_integration_get_settings` | 无 | `CommandResult<DesktopIntegrationSettings>` |
| `desktop_integration_update_shortcuts` | `ShortcutPreferences` | `CommandResult<ShortcutPreferences>` |
| `desktop_integration_set_autostart` | 启用状态 | `CommandResult<boolean>` |
| `notification_get_settings` | 无 | `CommandResult<NotificationSettings>` |
| `notification_update_preferences` | `NotificationPreferences` | `CommandResult<NotificationPreferences>` |
| `notification_open_settings` | 无 | `CommandResult<void>` |
| `note_get` | 本地日期 | `CommandResult<DailyNote | null>` |
| `note_save` | `DailyNoteInput` | `CommandResult<DailyNote>` |
| `weekly_goal_list` | 周起始日期、IANA 时区 | `CommandResult<WeeklyGoal[]>` |
| `weekly_goal_save` | `WeeklyGoalInput`、IANA 时区 | `CommandResult<WeeklyGoal>` |

## 任务命令

| 命令 | 输入 | 输出 |
|------|------|------|
| `task_list` | 可选日期范围、项目、分类、完成状态、搜索词 | `TaskListItem[]` |
| `task_get` | 任务标识 | `TaskDetail` |
| `task_create` | 标题、分类、可选项目、优先级、计划日期时间、检查项、本地日期 | `TaskDetail` |
| `task_update` | 任务标识、完整任务输入、本地日期 | `TaskDetail` |
| `task_set_completed` | 任务标识、完成状态 | `TaskDetail` |
| `task_remove` | 任务标识 | 软移除结果 |
| `task_set_check_item_completed` | 任务标识、检查项标识、完成状态 | `TaskDetail` |
| `task_reorder_check_items` | 任务标识、有序检查项标识 | `TaskDetail` |

`TaskInput` 支持 `work | study | health | life` 分类、0–3 优先级、可选项目、可选计划日期时间和检查项。计划日期使用 `YYYY-MM-DD`，计划时间使用 `HH:MM`；提供计划时间时必须同时提供日期。`TaskDetail` 返回任务记录及按位置稳定排序的检查项。

`TaskListFilter` 的 `startsOn` 与 `endsOn` 使用含边界日期范围；`completion` 支持 `pending | completed`；`search` 对标题执行大小写不敏感子串匹配。结果排除 `removed` 任务，`TaskListItem` 同时返回可选的项目名称、颜色、图标和状态摘要。

前端 `src/features/tasks/taskClient.ts` 为八个已注册任务命令提供类型化调用入口，复用 `TaskInput`、`TaskDetail`、`TaskListFilter` 和 `TaskListItem`。任务编辑器使用与 Rust DTO 一致的 camelCase 字段；打开已有任务时先通过 `task_get` 读取完整检查项，检查项更新时保留已有标识和完成状态。

今日工作台在 Tauri 运行时以选中日期调用 `task_list`，日期范围的起止值相同。普通浏览器运行时使用内存样例数据，以支持无原生后端的界面预览。

## 任务界面组件

| 组件 | 输入 | 行为 |
|------|------|------|
| `TaskEditor` | 当前本地日期、项目摘要、可选初始任务 | 编辑标题、分类、项目、优先级、日期时间和检查项，校验后输出完整 `TaskInput` |
| `TaskRow` | `TaskListItem`、视觉状态、操作回调 | 展示时间、标题、项目或分类、优先级与状态，并分离详情、完成和专注操作 |

`TaskRow` 的视觉状态为 `normal | current | completed | overdue | paused`。每种状态同时提供文字标签和颜色；完成按钮使用 `aria-pressed` 暴露当前完成状态。

## 项目命令

| 命令 | 输入 | 输出 |
|------|------|------|
| `project_list` | 状态筛选、搜索词 | `ProjectSummary[]` |
| `project_get` | 项目标识 | `ProjectDetail` |
| `project_create` | 项目资料 | `Project` |
| `project_update` | 项目标识、变更字段 | `Project` |
| `project_set_status` | 项目标识、目标状态 | `Project` |
| `project_archive` | 项目标识 | 归档结果 |

## 重复任务与今日汇总命令

`RecurrencePattern` 是带 `kind` 判别字段的结构化枚举：`daily` 包含正整数 `interval`，`weekdays` 无附加字段，`weekly` 包含正整数 `interval` 和 ISO 星期数组 `weekdays`，`monthly` 包含正整数 `interval` 和 1–31 的 `dayOfMonth`。`RecurrenceRule` 保存 `taskTemplateId`、规则模式、可选 `localTime`、IANA `timezone`、`startsOn`、可选 `endsOn`、`active | paused | ended` 状态和正整数版本。

Rust 内部已实现 `RecurrenceScheduler::run(trigger, rangeStart, rangeEnd)`。`trigger` 支持 `Startup`、`DayBoundary`、`RuleChanged { ruleId }` 和 `TimezoneChanged { ruleId }`；返回每条规则的 `GenerationSummary`，包含 `ruleId`、计划日期数 `scheduledCount` 和实际插入或更新数 `affectedCount`。前两类触发遍历活动规则并仅插入缺失实例，后两类触发刷新指定规则的待处理实例。面向当前重复计划界面的规则与实例命令均已注册。

`TaskInstanceRecord` 保存 `recurrenceRuleId`、`ruleVersion`、`scheduledDate`、可选 UTC `scheduledAt`、标题与项目快照、状态、完成时间、来源实例和审计时间。无本地执行时间的规则将 `scheduledAt` 保存为空值。

`RecurrenceService` 当前通过 Tauri commands 暴露以下实例与规则操作：

| 内部方法 | 输入 | 行为 |
|----------|------|------|
| `complete_instance` | 实例标识 | 将可操作实例改为 `completed` 并记录 UTC 完成时间 |
| `skip_instance` | 实例标识 | 将可操作实例改为 `skipped` |
| `delay_instance_today` | 实例标识、本地时间 | 在实例原计划日期内设置更晚的执行时间 |
| `reschedule_instance_tomorrow` | 实例标识 | 原实例改为 `rescheduled`，创建或复用唯一明日 pending 实例 |
| `set_rule_status` | 规则标识、`paused / ended` | 递增版本并保留全部历史实例 |
| `apply_schedule_change` | 候选规则、变更范围、生成范围结束日期 | 仅更新本次计划时间，或原子更新规则与未来 pending 实例 |

`RecurrenceChangeScope` 使用 `{ scope: "thisInstance", instanceId }` 或 `{ scope: "future", effectiveOn }`。未来范围要求候选规则保持标识、任务模板和状态，并将版本准确递增一；已完成、已跳过、已顺延和已开始实例保持原状态与快照。

Rust 内部已实现 `TodayService::get_digest(date)`。日期使用 `YYYY-MM-DD`，无效输入返回 `TODAY_DATE_INVALID`。返回结构如下：

```ts
interface TodayDigest {
  date: string;
  items: TodayDigestItem[];
}

interface TodayDigestItem {
  sourceKind: "task" | "recurringInstance";
  sourceId: string;
  itemKind: "ordinaryTask" | "projectTask" | "recurringInstance";
  recurrenceRuleId: string | null;
  title: string;
  category: string;
  priority: number;
  scheduledDate: string;
  scheduledTime: string | null;
  status: "pending" | "completed";
  completedAt: string | null;
  project: TodayProjectSummary | null;
  isOverdue: boolean;
  createdAt: string;
}
```

`scheduledTime` 对普通任务使用已保存的本地 `HH:MM`，对重复实例使用 `scheduledAt` 按规则 IANA 时区转换后的本地时间。`isOverdue` 表示 pending 来源的计划日期早于查询日期。结果按逾期、是否有计划时间、计划时间、优先级、创建时间和稳定来源标识排序。

| 命令 | 输入 | 输出 |
|------|------|------|
| `recurrence_get` | 规则标识 | `RecurrenceRule` |
| `recurrence_create` | 重复规则、生成起止日期 | `GenerationSummary` |
| `recurrence_update` | 候选规则、变更范围、生成结束日期 | `GenerationSummary` |
| `recurrence_set_status` | 规则标识、状态 | `RecurrenceRule` |
| `instance_complete` | 实例标识 | `TaskInstanceRecord` |
| `instance_skip` | 实例标识 | `TaskInstanceRecord` |
| `instance_delay_today` | 实例标识、本地时间 | `TaskInstanceRecord` |
| `instance_reschedule_tomorrow` | 实例标识 | `TaskInstanceRecord` |
| `today_get_digest` | 本地日期 | `TodayDigest` |

前端 `recurrenceClient` 与 `todayClient` 封装上述命令。`RecurrenceEditor` 输出不含持久化身份的 `RecurrenceRuleInput`，创建流程在任务模板保存后补齐规则标识、模板标识、活动状态和首个版本。`RecurrenceScopeEditor` 对 `thisInstance` 保持当前版本，对 `future` 将版本递增一。

## 专注命令

| 命令 | 输入 | 输出 |
|------|------|------|
| `focus_get_state` | 无 | `FocusState` |
| `focus_reconcile` | 无 | `FocusReconcileResult` |
| `focus_start` | `FocusTarget`、1–180 计划分钟数 | `FocusState` |
| `focus_pause` | 无 | `FocusState` |
| `focus_resume` | 无 | `FocusState` |
| `focus_finish` | `early | deadline` 完成方式 | `FocusSession` |
| `focus_reset` | 无 | `FocusState` |

`FocusTarget` 包含可选 `taskId` 与 `taskInstanceId`，调用方必须精确提供一个标识。普通任务和重复实例均需保持 `pending` 状态，系统全局只允许一条活动专注记录。

`FocusState` 使用 `ready | running | paused` 作为 `state` 判别字段。所有状态包含 `serverTime`；运行状态包含 `targetEndsAt`，暂停状态包含 `pausedAt`；两种活动状态都包含计划秒数、剩余秒数、开始时间和中断次数。`focus_finish` 的 `deadline` 仅在剩余时间归零后生效，`early` 写入有效提前完成记录；`focus_reset` 写入 `cancelled` 历史并返回 `ready`。

`FocusReconcileResult` 包含校准后的 `state` 和可选 `completedSession`。运行轮次未到期时返回按目标结束时间计算的剩余秒数；暂停轮次保持持久化剩余值；首次处理到期轮次时返回 `deadline` 会话，后续重复调用返回 `ready` 且省略完成会话。`focus_get_state` 同样先执行校准。Tauri Rust 层在 `RunEvent::Resumed` 中调用相同流程，产生完成记录时发送系统通知并广播 `focus://completed`。

前端 `focusClient` 为七个专注命令提供类型化调用入口。`FocusWorkspace` 接收 `WorkspaceTask[]` 和可选初始任务，普通任务提交 `taskId`，重复实例提交 `taskInstanceId`；运行状态按 `targetEndsAt` 驱动显示，到零时调用 `focus_reconcile`。最近轮次保存当前界面运行期间收到的命令结果与 `focus://completed` 事件，并按会话标识去重保留最近五条。

## 日历与统计命令

| 命令 | 输入 | 输出 |
|------|------|------|
| `calendar_get_period` | `CalendarQuery` | `CalendarPeriodResult` |
| `statistics_get_summary` | `CalendarQuery` | `StatisticsSummary` |

`CalendarQuery` 包含 `period: week | month | year`、`anchorDate: YYYY-MM-DD`、IANA `timezone`，以及可选 `category` 和 `projectId`。周周期固定为周一至周日，月与年使用自然周期；无数据日期同样包含在结果中。

`CalendarPeriodResult` 返回 `period`、`startsOn`、`endsOn`、完整 `days` 和可用于筛选的 `projects`。每个 `CalendarDay` 分别包含 `plannedTasks`、`completedTasks` 与 `focusSessions`；任务条目标识 `task | recurringInstance` 来源、分类、项目、计划时间、状态和完成时间，专注条目包含实际秒数、`deadline | early` 完成方式及起止时间。完成任务按 `completedAt`、专注轮次按 `endedAt` 转换为查询时区后的本地日期归组，`cancelled` 专注记录不进入结果。

`StatisticsSummary` 与日历命令共享 `CalendarQuery`，返回 `period`、`startsOn`、`endsOn`、`plannedTaskCount`、`completedTaskCount`、整数 `completionPercent`、`focusSeconds`、向下取整的 `focusMinutes`、`effectiveSessionCount` 和 `activeDayCount`。`trend` 保留每个本地日期的计划数、完成数、专注秒数和有效轮次；`projectInvestments` 按项目专注秒数降序返回项目摘要、专注秒数、有效轮次和占总专注时长的整数百分比。无项目轮次计入总专注，项目投入列表仅包含具有关联项目的轮次。

## 便签与周目标命令

| 命令 | 输入 | 输出 |
|------|------|------|
| `note_get` | `noteDate: YYYY-MM-DD` | `DailyNote | null` |
| `note_save` | `DailyNoteInput` | `DailyNote` |
| `weekly_goal_list` | `weekStartsOn: YYYY-MM-DD`、`timezone` | `WeeklyGoal[]` |
| `weekly_goal_save` | `WeeklyGoalInput`、`timezone` | `WeeklyGoal` |

`DailyNoteInput` 包含 `body` 与 `noteDate`，正文允许为空且最多 4000 个 Unicode 字符。`note_get` 在当天缺少记录时返回 `null`。前端停止输入 500ms 后调用 `note_save`，`Ctrl+Enter` 会取消待执行的保存计时并立即调用同一命令。

`WeeklyGoalInput` 包含可选 `id`、周一格式的 `weekStartsOn`、`title`、`category` 和正整数 `targetCount`。`category` 支持 `completedTasks | focusMinutes | activeDays`。`WeeklyGoal` 增加 `completedCount`、稳定 `position` 与审计时间；列表和保存命令均按调用方 IANA 时区聚合该自然周数据，并返回限制在目标数量以内的当前进度。

## 设置和系统命令

| 命令 | 输入 | 输出 |
|------|------|------|
| `settings_get` | 无 | `GeneralPreferences` |
| `settings_update` | `GeneralPreferencesPatch` | `GeneralPreferences` |
| `desktop_integration_get_settings` | 无 | 全局快捷键、开机启动和启动期冲突信息 |
| `desktop_integration_update_shortcuts` | 启用状态与四项快捷键 | 当前有效快捷键偏好 |
| `desktop_integration_set_autostart` | 启用状态 | Windows 登录启动项当前状态 |
| `notification_get_settings` | 无 | 通知偏好和权限状态 |
| `notification_update_preferences` | 通知与提示音开关 | 当前通知偏好 |
| `notification_open_settings` | 无 | 打开 Windows 通知设置 |
| `window_show_main` | 无 | 无 |
| `widget_get_config` | 无 | `WidgetConfig` |
| `widget_update_config` | `WidgetConfigInput` | `WidgetConfig` |
| `widget_show` | 无 | `WidgetConfig` |
| `widget_unlock` | 无 | `WidgetConfig` |
| `app_exit` | 无 | 无 |

`ShortcutPreferences` 包含总开关和 `showMainWindow`、`toggleFocus`、`createQuickTask`、`unlockWidget` 四项字符串绑定。所有绑定必须非空、可解析且互不重复。更新命令先注册候选按键组合，再释放候选集合中不再使用的旧组合；注册冲突返回 `SHORTCUT_CONFLICT` 和对应字段，数据库与运行态继续使用原配置。

`DesktopIntegrationSettings` 返回当前快捷键偏好、由系统启动项读取的 `autostartEnabled`，以及应用启动期可能产生的 `shortcutError`。`desktop_integration_set_autostart` 立即调用 Windows 登录启动项适配器；偏好保存失败时恢复调用前的系统状态。

`GeneralPreferences` 包含 `language: system | zhCn | en`、`appearance: system | light | dark`、`theme: mint | noir | office | blush` 和 `backgroundRunning`。`GeneralPreferencesPatch` 允许只提交需要变更的字段；`settings_update` 返回合并并持久化后的完整偏好，再以同一结构广播 `settings://changed`。首次读取使用系统语言、系统外观、薄荷主题和启用后台运行的默认值。

任务栏进度由后台托盘刷新线程直接更新，无前端命令。`running` 状态使用 Normal 和剩余秒数占计划秒数的 0–100 比例，`paused` 使用 Paused，`ready` 清除任务栏进度。

`WidgetConfigInput` 包含 `size: compact | standard | expanded`、`mode: desktop | floating`、`locked`、`opacity`、`modules`、窗口坐标与尺寸、可选 `monitorId` 和 `scaleFactor`。模块值支持 `clock`、`currentFocus`、`todayProgress`、`tasks`、`quickActions`、`projectProgress`、`weeklyGoals` 和 `noteEntry`。`WidgetConfig` 在输入字段基础上增加 `lastVisibleAt` 与 `updatedAt`。

`widget_get_config` 在首次调用时创建标准档默认配置。`widget_update_config` 原子保存布局与窗口状态，应用尺寸、位置、置顶、鼠标穿透和缩放策略，并向小组件窗口广播配置事件。应用保存位置前读取主显示器及全部可用显示器工作区；左上 160 × 48 主要操作区脱离所有工作区时，位置恢复到主显示器中央或安全原点。`widget_show` 更新最近可见时间、应用持久化窗口状态并显示窗口；普通浮窗同时获得焦点。`widget_unlock` 通过 `WidgetService::unlock` 持久化解锁状态，再关闭鼠标穿透、恢复缩放并显示窗口；托盘解锁菜单复用同一流程。

### 系统托盘

托盘菜单由 `desktop::tray` 在 Tauri setup 阶段创建，并每秒从 SQLite 权威专注状态刷新。菜单提供以下操作：

| 菜单项 | 行为 |
|--------|------|
| 当前或下一任务 | 只读展示活动目标或 TodayDigest 第一条待完成事项 |
| 剩余时间 | 只读展示 `MM:SS`，暂停状态附带文字标识 |
| 显示主窗口 | 显示、取消最小化并聚焦 `main` 窗口 |
| 显示小组件 | 更新最近可见时间并应用已保存窗口配置 |
| 开始/暂停/继续 | Ready 时以 TodayDigest 第一条待办启动 25 分钟轮次，Running/Paused 时切换状态；无待办时打开专注空间 |
| 创建快速任务 | 恢复主窗口并广播 `tray://quick-task` |
| 解锁小组件 | 复用 `WidgetService::unlock` 和原生窗口配置 |
| 退出抵达 Focus | 请求应用退出；活动轮次状态已持续保存在 SQLite |

托盘图标左键释放会恢复并聚焦主窗口。主窗口收到关闭请求时执行关闭策略，后台运行默认启用时阻止销毁并隐藏窗口。托盘刷新调用 `FocusService::reconcile`，保证主窗口隐藏期间到期轮次仍被原子完成。

### Windows 通知

`NotificationSettings` 返回 `preferences` 和 `permissionState`。偏好包含默认启用的 `notificationsEnabled` 与 `soundEnabled`；权限状态为 `granted | denied | unknown`。Windows 桌面插件将权限交由系统管理，因此设置页使用 `unknown` 表达系统管理状态，并提供 Windows 通知设置入口。

专注完成通知包含任务名称和实际专注时长；普通定时任务与重复实例到达计划时间时发送到时通知。任务 worker 每 15 秒按连续时间窗口查询 pending 来源，系统恢复时通过 Tauri `RunEvent::Resumed` 立即补跑。`notification_deliveries` 使用 `(kind, source_id, scheduled_for)` 唯一键，保证同一业务事件只保留一条投递记录；提示音开启时使用 Windows 默认通知音。

通知处理先以业务身份预留 `pending` 投递记录，再调用系统发布器并将记录更新为 `sent` 或 `failed`。P7 属性测试通过重复提交三类事件验证该接口的 at-most-once 语义，发布失败记录同样占用唯一身份并停止自动重试。

小组件内容复用业务命令，无独立任务写接口。加载时调用 `today_get_digest` 与 `focus_get_state`；普通任务完成调用 `task_set_completed`，重复实例完成与今日延后分别调用 `instance_complete` 和 `instance_delay_today`。开始专注把来源映射为互斥的 `FocusTarget` 并调用 `focus_start`，默认时长为 25 分钟；活动轮次调用 `focus_pause` 或 `focus_resume`。每次任务写入成功后重新读取当日汇总，专注状态每 2 秒与 Rust 权威状态校准。

## 数据命令

| 命令 | 输入 | 输出 |
|------|------|------|
| `backup_export` | 目标路径 | 备份摘要 |
| `backup_inspect` | 来源路径 | 校验结果和数据摘要 |
| `backup_restore` | 来源路径、校验令牌 | 恢复结果 |
| `backup_list_snapshots` | 无 | 自动快照列表 |

文件路径由原生对话框产生，业务界面只传递用户已选择的路径。恢复确认使用短时校验令牌绑定已检查文件，避免文件内容变化后直接写入。

## 事件

| 事件 | 载荷 | 用途 |
|------|------|------|
| `focus://tick` | 剩余秒数、状态、时间戳 | 同步前端计时显示 |
| `focus://completed` | 专注轮次 | 更新界面并展示完成状态 |
| `focus://state-changed` | `FocusState` | 同步托盘触发的开始、暂停和继续状态 |
| `tray://quick-task` | 无 | 恢复主窗口并打开快速任务编辑器 |
| `tray://open-focus` | 无 | 在无可启动待办时打开专注空间 |
| `settings://changed` | 完整 `GeneralPreferences` | 同步主窗口与小组件的主题、语言偏好和后台行为 |
| `project://changed` | 项目标识、版本 | 刷新项目与关联进度 |
| `today://changed` | 本地日期、版本 | 同步主窗口、小组件和托盘 |
| `widget://config-changed` | `WidgetConfig` | 同步小组件尺寸档位、透明度和锁定状态 |
| `widget://mode-fallback` | 原模式、回退模式、原因码 | 告知桌面贴附回退状态 |
| `backup://restored` | 恢复摘要 | 重新加载业务数据 |

`widget://mode-fallback` 当前载荷为 `{ fromMode: "desktop", toMode: "floating", reason }`。`reason` 支持 `UNSUPPORTED_PLATFORM`、`HOST_NOT_FOUND`、`ATTACHMENT_FAILED` 和 `DETACHMENT_FAILED`。同一次连续故障只广播一次；成功重新附着后，后续独立故障可以再次广播。小组件收到事件后显示普通浮窗状态说明。

## 备份格式

```json
{
  "app": "arrive-focus",
  "formatVersion": 1,
  "exportedAt": "2026-07-18T12:00:00Z",
  "data": {
    "tasks": [],
    "projects": [],
    "taskCheckItems": [],
    "recurrenceRules": [],
    "taskInstances": [],
    "focusSessions": [],
    "notes": [],
    "weeklyGoals": [],
    "preferences": {}
  }
}
```

导入解析器按 `formatVersion` 选择版本化反序列化器，再转换为当前领域模型。导出文件仅包含业务数据和用户设置。

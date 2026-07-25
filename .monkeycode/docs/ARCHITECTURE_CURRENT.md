# 抵达 Focus 当前架构补充

## 抵达 Focus 项目持久化架构

抵达 Focus 项目模块位于 `src/features/projects/`。桌面运行时由 `projectClient.ts` 通过 Tauri commands 读取和修改 SQLite 中的项目，`ProjectWorkspace.tsx` 只维护筛选、选择、对话框和加载状态；浏览器预览使用隔离的示例数据，不参与桌面持久化。

项目列表读取 `ProjectSummary`，详情读取 `ProjectDetail` 及其真实任务集合。创建、编辑和状态变更成功后重新读取当前筛选与详情，失败时保留编辑上下文并通过统一领域错误映射展示安全文案。详情响应必须与当前选中项目 ID 一致，避免异步切换项目时显示旧详情或向错误项目发起任务操作。

主应用启动时读取全量项目摘要，为任务编辑器提供动态项目选项。项目页的“添加任务”进入共享任务编辑器并预选项目；“开始专注”将详情中的真实任务转换为专注工作区输入；任务完成或恢复继续复用 `task_set_completed`，成功后刷新项目详情、今日摘要和周目标。

## 抵达 Focus 重复任务生产调度

`RecurrenceScheduler::reconcile_active_to_utc_now` 负责把所有活跃规则生成到各自 IANA 时区下的本地当天。`Startup` 从规则 `startsOn` 回填完整缺口，用于首次启动和系统恢复后的跨日补偿；`DayBoundary` 只处理规则本地当天，用于常驻进程的增量生成。开放式规则由运行时持续推进，带结束日期的规则继续由领域层裁剪生成范围。

桌面启动在 SQLite 注册为 Tauri state 后立即执行 `Startup`。通知 worker 每 15 秒先执行 `DayBoundary`，再扫描任务提醒，使新生成实例在同一轮后续通知处理中可见；`RunEvent::Resumed` 执行完整回填后再补扫提醒。重复调用依赖 `(recurrence_rule_id, scheduled_date)` 唯一约束和 repository upsert 保持幂等。

`desktop/recurrence.rs` 汇总受影响实例数，仅在实际写入时广播 `today://changed`。主窗口收到事件后刷新当前日期摘要、项目期限信息和周目标；若用户仍停留在跨日前的“今天”，日期会推进到新的本地当天。Widget 收到同一事件后重新读取 Today digest。

项目、普通任务与重复计划的 Tauri 写 command 通过 `desktop/today_events.rs` 共用提交后事件边界。项目创建、更新、状态变更与移除，任务创建、更新、完成恢复、移除、检查项变更与排序，以及重复规则创建、更新、状态变更和实例完成、跳过、当天延后、顺延明天，均在 service 成功写入 SQLite 后广播一次 `today://changed`；日期解析或领域写入失败时保持零广播。事件同时到达来源窗口和其他 WebView，使主窗口的项目摘要、今日摘要与周目标和 Widget 的 Today digest 重新读取同一权威数据。项目暂停、完成、归档或解除关联后，任务的嵌入项目状态与专注候选资格随权威摘要一并刷新。

通知 worker 使用内存扫描游标构造连续提醒窗口。每轮会处理窗口内全部候选；整批成功后将游标推进到当前时间，任一发布失败或活动投递仍在 lease 内时保留原游标，使候选继续落在下一轮窗口内。`notification_deliveries` 的唯一键继续保证每个事件只有一条投递记录：新候选创建带 60 秒 lease 的 `pending` 记录，系统发布成功后转为 `sent`，发布失败后转为 `failed` 并保存稳定错误码；后续扫描可原子地接管 `failed` 或 lease 已过期的 `pending` 记录，并刷新 lease 时间、提示音偏好和错误状态。活动 `pending` 返回 `InFlight`，`sent` 返回 `AlreadySent`。批次会继续尝试同一窗口中的其他候选，再向 worker 返回首个发布错误或 in-flight 状态。

## 抵达 Focus 专注状态同步

`desktop/focus_events.rs` 定义专注状态的提交后事件边界。`focus_start`、`focus_pause`、`focus_resume` 和 `focus_reset` 在领域服务成功写入权威状态后广播 `focus://state-changed`，失败时保持零广播；Tauri 自动注入 `AppHandle`，前端 command 参数不变。手动提前完成在保存专注轮次并清除活动状态后发送 `focus://completed` 与 ready 状态，自动到期协调也发送同一组事件。托盘和全局快捷键继续通过托盘控制路径变更状态，并复用相同状态事件。

主窗口专注空间与 Widget 都直接消费事件 payload，立即呈现开始、暂停、继续、重置和完成状态。Widget 保留每 2 秒读取 `focus_get_state` 的校准轮询，用于休眠、调度延迟和丢失事件后的恢复；UI 倒计时仍只负责展示，SQLite 与 Rust 专注服务继续作为权威来源。

`desktop/memo_events.rs` 定义备忘录提交后事件边界。`memo_create`、`memo_update` 和 `memo_remove` 在 Repository 事务成功提交后各广播一次空 payload 的 `memo://changed`；验证或持久化失败时保留原领域错误并跳过广播。事件只作为重新读取 SQLite 权威列表与详情的信号，事件投递失败不回滚已提交的备忘录数据。

备忘录 command 的纯编排 helper 与 Tauri 参数注入分离，使默认 Rust 测试可在真实内存 SQLite 上验证创建、更新、标签替换、置顶状态、删除和稳定失败错误。桌面特性编译继续覆盖 `AppHandle` 注入、六个 command 宏展开及 invoke handler 注册。

`FocusService::validate_target` 是开始专注的权威资格边界。普通任务通过当前 `project_id` 查询项目，重复实例通过生成时固化的 `snapshot_project_id` 查询项目；项目状态为 `paused` 时返回 `FOCUS_PROJECT_PAUSED`，从而统一覆盖主窗口、Widget、托盘和全局快捷键等入口。托盘在选择下一项时预先过滤暂停项目候选并继续查找下一条待处理任务；服务层校验继续防止旧 UI 快照或直接 command 调用绕过限制。无项目引用或引用项目已不存在时沿用任务自身的可用性结果。

## 抵达 Focus 国际化架构

抵达 Focus 的国际化模块位于 `src/i18n/`。`messages.ts` 以简体中文资源推导 `MessageKey`，英文资源通过 `Record<MessageKey, string>` 约束键完整性；`locale.ts` 将 `system`、`zhCn` 和 `en` 偏好解析为 `zh-CN` 或 `en-US`，并监听浏览器 `languagechange`；`I18nContext.tsx` 提供参数插值、日期、时间和相对时间格式化。

主窗口 `App.tsx` 与小组件 `WidgetApp.tsx` 分别订阅 `settings://changed`，解析相同语言偏好并挂载 `I18nProvider`。两个窗口共享文案资源和 `Intl` 格式器，语言变化会同步更新界面、根节点 `data-locale` 与文档 `lang` 属性。固定界面文案已覆盖导航、今日、项目、专注、日历、统计、任务、重复计划、设置和小组件；`src/lib/domainError.ts` 根据稳定领域错误码及错误类别选择类型化中英文安全文案。

## 抵达 Focus 无障碍与缩放架构

共享 UI 边界位于 `src/components/ui.tsx`。`Dialog` 使用 React 唯一标题 ID 建立可读名称，打开时保存当前触发元素并将焦点移入对话框，Tab 与 Shift+Tab 在可操作元素间循环，Escape 触发关闭，卸载后恢复触发元素焦点。`SegmentedControl` 使用 radiogroup、radio、`aria-checked` 和 roving tabindex，支持方向键、Home 与 End 切换。主导航通过 `aria-current="page"` 表达当前页面，项目卡通过 `aria-pressed` 表达选中状态，任务行与小组件快捷操作的可读名称包含任务标题。

主题模块为主窗口和小组件共享 `accentContrast`、`focusRing`、`success`、`warning` 与 `danger` 语义令牌。明暗模式分别解析状态色和主按钮前景色；组件测试将 OKLCH 转换为相对亮度，逐主题验证正文、辅助文字、强调文字、主按钮和状态文字与对应背景的 WCAG 2.2 AA 对比度。全局焦点环覆盖按钮、表单控件、链接和可编程聚焦元素。

主内容容器允许自然重排，侧栏、对话框和小组件在内容超出可见区域时提供滚动；窄窗口下标题栏、操作区和对话框页脚允许换行。小组件透明度仅作用于背景合成，文字、图标、操作控件和焦点环保持完整不透明。`prefers-reduced-motion: reduce` 会停用装饰性动画与过渡，状态变化仍通过即时颜色、文字和结构反馈呈现。

## 抵达 Focus 桌面生命周期架构

Tauri 启动先从编译期 `Context` 的应用标识解析数据目录、打开 SQLite，并通过 `Builder::manage(database)` 把数据库注册到 AppManager；随后才注册插件和执行 `setup`。该顺序保证 WebView 首次加载并发起 command 时 `State<Database>` 已经可用，启动契约测试固定数据库注册早于 setup。第二个进程启动时，single-instance 插件回调复用托盘和全局快捷键共同使用的 `show_main_window()`，显示、取消最小化并聚焦现有主窗口。主窗口在 `tauri.conf.json` 中初始隐藏；`desktop/main_window.rs` 恢复保存状态，再显示并聚焦窗口，避免默认位置闪现。

`MainWindowState` 保存物理坐标、逻辑宽高、最大化状态、显示器名称和 DPI scale factor。状态以 `mainWindowState` 键写入现有 `preferences` 表，属于设备运行态数据。窗口移动、缩放和 DPI 变化通过 `MainWindowGeometryRuntime` 进行 180 毫秒防抖写入；最大化期间只更新最大化标记并保留最后一个普通窗口矩形，最小化期间保持既有状态。

恢复流程按主显示器优先收集所有工作区，并复用小组件的 `restore_visible_rect` 算法验证主要操作区域。保存矩形仍可见时保留原位置；矩形位于全部显示器之外时居中到主显示器工作区；显示器信息暂时不可用时调用 Tauri `center()`。

`desktop/lifecycle.rs` 统一显式退出：先校准并持久化活动专注，再同步保存主窗口和小组件几何，全部成功后调用 `AppHandle::exit`。托盘退出、关闭主窗口且后台运行关闭、Tauri `ExitRequested` 都经过该边界；持久化失败会记录稳定错误码并阻止退出。关闭到托盘会在隐藏前同步保存主窗口状态，并继续保留后台计时。

Widget 的原生 `CloseRequested` 由应用拦截。运行时先尝试持久化当前几何，再隐藏窗口并保留 WebView、Shell monitor 和前端状态，使 `Alt+F4` 后仍可通过托盘或 command 重新显示。显式退出仍把活动专注、主窗口状态和可访问的 Widget 几何作为持久化边界；Widget 已被外部销毁而无法按标签取得时跳过其几何，实际窗口操作或数据库写入失败继续阻止退出和更新安装。

Widget Shell monitor 每 2 秒检查桌面附着宿主。`ShellAttachmentOutcome` 同时定义父窗口附着结果和应应用的 Tauri 原生层级：`DesktopAttached` 关闭 `always_on_top`，`Floating` 与 `FloatingFallback` 开启 `always_on_top`。Explorer 重启或宿主失效时，桌面模式临时回退为置顶浮窗，并仅在一次连续失败周期内广播一次 `widget://mode-fallback`；重新发现宿主并附着后关闭置顶并广播 `widget://mode-restored`，前端据此清除回退提示。显式切换浮窗模式会停止桌面恢复请求并清除失败周期状态。

任务 13.4 的自动化边界由前端行为测试、CSS 契约测试和 Rust 纯逻辑测试组成。Testing Library 验证 Dialog 的键盘顺序、显式自动焦点和关闭后焦点恢复，以及主导航在 3 秒预算内进入可交互状态；`accessibility.contract.test.ts` 读取实际全局样式，固定焦点环、减少动效、主内容、侧栏、Dialog 和 Widget 的缩放重排契约。Rust 侧将已有实例激活抽象为可替换窗口目标，验证显示、取消最小化、聚焦的调用顺序和失败短路；主窗口恢复测试将持久化状态直接送入共享可见区域修正算法，覆盖屏幕外位置居中。

## 抵达 Focus 更新架构

`src-tauri/src/commands/update.rs` 在 Rust 侧封装 `tauri-plugin-updater`，前端只通过项目 command 协议访问更新能力。`PendingUpdateState` 管理检查、可下载、下载中、已验签和安装中状态，阻止并发操作；更新检查设置 30 秒超时并只接受构建时注入的 HTTPS 发布端点。下载由 updater 插件完成并验证 Minisign 签名，进度通过 `update://download-progress` 事件发送到主窗口，已验证的包保存在进程内等待用户确认。

设置页 `UpdateSettingsPanel` 在桌面运行时自动检查版本，展示版本号、发布日期和更新说明，并将下载确认与安装确认拆为两个显式步骤。安装入口先调用桌面生命周期的 `persist_before_exit`，校准活动专注并保存主窗口与小组件状态；持久化成功后启动更新安装并调用 Tauri restart。检查、下载、验签、持久化或安装失败时应用继续运行，界面只展示稳定安全文案，内部错误通过现有脱敏诊断日志记录。

发布构建通过 `ARRIVE_FOCUS_UPDATE_ENDPOINT` 和 `ARRIVE_FOCUS_UPDATE_PUBLIC_KEY` 注入发布端点与 Minisign 公钥，`bundle.createUpdaterArtifacts` 生成更新产物。签名私钥只属于受保护的发布环境；Windows Authenticode 代码签名由后续安装包发布任务配置，与 updater 包签名组成独立信任边界。

Windows 打包提供无签名验包与正式签名两个入口。无签名入口合并 `src-tauri/tauri.windows-unsigned.conf.json`，关闭 updater 产物并显式启用 `desktop-app` Cargo feature，可在 Windows 构建机或 Linux xwin MSVC 交叉构建环境生成 NSIS 安装包；签名入口合并被 Git 忽略的 Authenticode 配置，同样显式启用 `desktop-app`，并保留 updater 产物与双重签名边界。Linux 交叉构建产物用于静态格式和编译完整性验证，正式 Authenticode 签名及安装、升级、卸载验收位于 Windows 发布边界。

## 抵达 Focus Windows 安装包架构

`src-tauri/tauri.conf.json` 将 Windows bundle 固定为 NSIS，并使用 Tauri 标准安装模板。`currentUser` 安装模式提供安装目录页；标准完成页提供桌面快捷方式复选框；`startMenuFolder` 将开始菜单快捷方式收口到“抵达 Focus”目录。安装程序支持简体中文和英文，并在启动时显示语言选择器。

WebView2 前置条件使用 `downloadBootstrapper`。目标设备缺少 Evergreen WebView2 Runtime 时，安装程序通过 Microsoft bootstrapper 静默安装运行时，因此首次安装需要网络连接。应用和安装包图标使用 `src-tauri/icons/icon.ico`。

Windows Authenticode 使用独立发布覆盖文件 `src-tauri/tauri.windows-signing.conf.json`。仓库只保存无凭据模板 `tauri.windows-signing.conf.example.json`，本地覆盖文件由 `.gitignore` 排除。发布环境填写证书 SHA-1 thumbprint，使用 SHA-256 摘要与 HTTPS 时间戳服务；Tauri 在同一次 bundle 流程中签署 Windows 可执行文件和 NSIS 安装包。该信任边界与 updater 的 Minisign 私钥和公钥配置保持独立。

`scripts/windows-installer-smoke.ps1` 在一次性 Windows 测试用户下接收基线版和升级版 NSIS 安装包，使用 `/S` 与隔离的 `/D` 目录依次完成静默安装和升级。脚本启动两个已安装版本并等待 `arrive-focus.sqlite3` 创建，通过可执行文件 SHA-256 变化确认升级替换，最后静默运行 `uninstall.exe /S`，验证程序文件移除且 `%APPDATA%/com.arrive.focus` 中的 SQLite 数据和测试标记继续保留。脚本在安装目录、应用数据目录或同名进程已存在时立即停止，避免覆盖日常用户环境。

## 抵达 Focus 自动化桌面核心流程

`src-tauri/tests/desktop_core_flow.rs` 以单个内存 SQLite 数据库串联公开领域服务，覆盖 Release Acceptance 的跨模块核心路径：创建项目和当日任务、创建每日重复任务规则、幂等生成今日实例、聚合今日清单、读取并显示默认小组件、发布到时提醒、开始和提前完成专注、完成任务、读取周历复盘与项目统计，最后导出并重新解析版本化 JSON 备份。

测试在通知边界注入实现 `NotificationPublisher` 的内存记录器，以已授权状态验证系统通知标题、正文和重复 reconcile 幂等性。其余步骤均使用正式 service 与 repository，通过真实迁移后的 SQLite schema 验证跨模块引用、持久化和统计结果；窗口位置、Tauri WebView 与 Windows 原生安装行为由独立桌面逻辑测试、前端行为测试和后续安装升级烟测覆盖。

## 抵达 Focus 错误边界与诊断日志

Rust command 统一通过 `CommandResult::from_result` 将领域服务结果转换为前端协议。`CommandResult<T>` 自定义序列化保证 `ok` 为 JSON 布尔值；失败响应保留稳定 `code`、可选 `field` 和内部 `message`，供业务逻辑识别和兼容现有 command 契约。

领域失败通过 `tauri-plugin-log` 写入桌面诊断日志。单条事件只包含 command 模块上下文、错误码和字段名；各字段限制为 120 个 ASCII 安全字符，换行和特殊字符会被替换。日志入口不接收 command 参数，也不写入 `DomainError.message`，因此任务标题、便签正文、文件路径和数据库详情不会进入诊断事件。

前端所有生产界面通过 `domainErrorMessage(error, t)` 展示错误。精确错误码优先映射到可执行文案，其余错误按存储、备份、专注、重复计划、桌面集成、输入和冲突类别映射；未知错误使用通用安全文案，Tauri invoke 异常统一转换为 `COMMAND_INVOCATION_FAILED`。invoke 包装器同时通过独立诊断 command 将经过单行、长度限制和字符过滤的 command 与拒绝原因写入 Rust 应用日志，诊断通道失败不会改变公开错误契约。组件测试使用包含路径和任务标题标记的底层错误验证敏感原文不会进入 DOM。

备忘录错误在同一边界中按标题、正文、标签、记录失效、保存、删除和提醒设置映射为可执行的简体中文与英文文案。前端映射只读取稳定错误码并忽略内部 message；Rust 诊断日志只保留 `commands::memo`、错误码和稳定字段名，标题、正文、标签与搜索词均不会进入日志事件。

今日页的随手便签以 SQLite 日记录为权威来源，输入停止 500 毫秒后自动保存，同时提供明确的“保存记录”按钮和 `Ctrl+Enter` 快捷保存。编辑器分别跟踪当前草稿与最后保存值，较早保存请求的状态回写不会覆盖用户随后输入的新内容。

备忘录中心的数据库基础由迁移 `0004_memo_center.sql` 建立。迁移创建 `memos`、`memo_tags`、`memo_tag_links` 和 `memo_reminders`，并将通知投递约束扩展为支持 `memoReminder`。通知表替换、既有记录复制和新表创建位于同一个迁移事务中，失败时由迁移框架整体回滚。

备忘录领域类型集中在 `src-tauri/src/domain/memo.rs`。`MemoInput` 组合内容、标签、置顶状态和可选提醒，`MemoReminderInput` 使用带 `kind` 判别字段的 once/recurring 枚举，`MemoReminderRule` 表达持久化规则状态。验证层统一处理 Unicode 长度、标签规范化唯一性、频率专属参数、本地日期时间和 IANA 时区。

`src-tauri/src/services/memo_reminder_service.rs` 将有效提醒规则转换为严格晚于给定 UTC 时刻的下一发生时间。一次提醒直接解析本地日期时间；重复提醒复用 `domain::recurrence::next_scheduled_date` 计算每天、工作日、每周和每月日期，再按保存的 IANA 时区转换为 UTC。每月日期在短月份收敛至月末，夏令时缺失时刻顺延至首个有效分钟，重叠时刻固定选择较早实例，结束日期之后返回空结果。

提醒协调由同一服务的 `reconcile_due` 执行。Repository 按发生时间和提醒 ID 稳定读取所有 active 到期提醒，服务逐项调用投递回调，并只在投递成功后推进状态；一次提醒转为 completed，重复提醒从当前发生时间计算严格递增的下一发生时间，越过结束日期后转为 completed。单项投递失败会保留原到期状态并继续处理同批其他提醒，批次结束时返回首个错误，供后续通知租约层决定重试。

状态推进使用提醒 ID、active 状态和旧 `next_scheduled_for` 作为比较条件。重复扫描得到的陈旧候选无法覆盖已经推进的记录，因此服务重入时保持单调状态。

`NotificationService::reconcile_memo_reminders` 将每个到期项映射为 `memoReminder` 投递，使用提醒 ID 作为 `source_id`、当前 UTC 发生时间作为 `scheduled_for`，并复用 `notification_deliveries` 的唯一键、60 秒 lease、失败重试和状态记录。发布成功后推进提醒；活动 lease 返回批次错误并保留原发生时间；失败记录和过期 lease 可由后续轮次接管。若进程在投递标记为 `sent` 后、提醒推进前中断，下一轮的 `AlreadySent` 结果会跳过 publisher 并完成条件推进。

桌面通知 worker 每轮在同一连续扫描周期中依次协调任务提醒和全部到期备忘录提醒，两类协调都会执行。任一来源返回错误时扫描游标保持原值；已成功投递的记录依靠唯一身份在重复窗口中保持幂等。

备忘录系统通知携带 `SystemNotificationActivation::OpenMemo { memo_id }`，激活数据只包含备忘录 ID。Windows 桌面发布路径使用 `tauri-winrt-notification` 注册通知主体点击回调；回调进入 `desktop/memo_notification_activation.rs` 后验证规范 UUID，复用主窗口显示、取消最小化和聚焦流程，再发送字符串 payload 的 `memo://open-requested`。非法 ID 以及窗口激活失败会在事件发送前停止，并以稳定错误码和无内容日志记录失败。React 页面在后续 UI 接线中订阅该事件，并通过 command 重新读取 SQLite 权威详情。

前端备忘录共享契约位于 `src/features/memos/types.ts`，并通过 `memoClient.ts` 访问 Tauri command。页面和组件统一依赖 `MemoRecord`、`MemoSummary`、`MemoListQuery` 与 `MemoReminderSchedule`，避免在 UI 层重复定义后端数据形状。

主窗口导航在“今日”和“项目”之间注册“备忘录”页面。`App` 订阅 `memo://changed` 并递增页面数据修订号，订阅字符串 payload 的 `memo://open-requested` 后切换至备忘录页面并生成带递增序号的 `MemoOpenRequest`；连续两次请求同一备忘录 ID 仍会形成独立打开请求。`MemoWorkspace` 接收数据修订号和打开请求，并编排权威列表、详情、新建草稿与保存流程。备份恢复事件同时推进备忘录数据修订号，所有异步 Tauri 监听器在 App 卸载时释放。

备忘录桌面页面使用固定 360px 列表栏和 `minmax(0, 1fr)` 弹性编辑栏。工作区高度按主窗口内容区收敛并隐藏页面级溢出，`.memo-list-pane` 与 `.memo-editor` 分别使用 `overflow-y: auto` 和 `overscroll-behavior: contain` 建立独立滚动边界；两个区域均复用共享 `Panel` 的主题边框、圆角、背景和阴影。

`.memo-workspace` 使用 inline-size container query 监测自身内容宽度。宽度低于 980px 时页面切换为单列，并依据 `data-mobile-view` 显示列表或编辑区域；隐藏区域保持挂载，因此后续列表搜索、标签筛选和滚动上下文可以跨详情往返保留。通知打开请求自动进入编辑视图，编辑区提供可读的“返回备忘录列表”操作。

`MemoWorkspace` 通过可注入的 `MemoClient` 读取列表和通知指定详情，生产环境默认使用正式 Tauri client。列表加载期间保持固定骨架尺寸；无记录时展示首条创建行动；有查询条件且无匹配时提供清除条件入口；详情返回 `MEMO_NOT_FOUND` 时关闭编辑视图、显示安全失效提示并再次读取权威列表。查询状态和两个面板保持在同一组件生命周期内，数据修订号变化会重新读取列表及当前详情。

列表摘要由 `src/features/memos/MemoListItem.tsx` 独立渲染。每项显示权威 `MemoSummary` 的显示标题、最多两行正文预览、前三个标签与剩余数量、提醒摘要和更新时间；置顶与提醒同时使用图标和可读文本。提醒摘要覆盖一次、每天、工作日、每周、每月、完成和取消状态，星期名称与日期时间遵循当前界面 locale。

备忘录搜索输入在 `MemoWorkspace` 内保留即时文本，停止输入 200 毫秒后才更新 `MemoListQuery` 并触发 `memo_list`。标签筛选通过 `memo_tag_list` 读取权威关联计数，选择后立即与已提交搜索词组成交集查询；标签因失去全部关联从权威列表消失时，页面同步清除失效标签条件。`App` 持有最近提交的查询并在工作区卸载时继续保留，使用户切换主页面再返回时恢复搜索词和标签，主窗口生命周期结束时自然释放。

`src/features/memos/MemoEditor.tsx` 维护完整 `MemoInput` 草稿，并通过统一 `onSave(input)` 边界提交标题、纯文本正文、标签和置顶状态。标题与正文分别按 200 和 20000 个 Unicode 字符受控截断并显示计数，用户可使用保存按钮或 Ctrl/Cmd+Enter 显式保存。标签输入提供 trim、30 字符、大小写重复和 10 个上限反馈，最终规范化与跨备忘录标签复用仍由 Rust 领域服务保证。`MemoWorkspace` 对已有记录调用 `memo_update`，对新建草稿首次调用 `memo_create`；创建成功后草稿切换为 command 返回的权威记录并刷新列表与标签计数。元数据保存失败时编辑器恢复权威标签和置顶值，同时保留用户正在编辑的标题与正文草稿。

提醒编辑由独立的 `MemoReminderEditor` Dialog 完成。分段控件切换一次和重复提醒；重复模式按频率渐进展示星期或每月日期，并统一收集间隔、本地时间、开始日期、可选结束日期和 IANA 时区。频率切换会清理无关字段，前端在提交前验证未来时间、频率形状、日期范围和时区，并把焦点移到首个错误字段；Rust 继续作为最终领域校验边界。当前规则在编辑器内以本地化摘要显示，并提供修改与取消操作；桌面通知权限为 denied 时，`MemoReminderPermissionNotice` 明确说明规则已保存并提供 Windows 通知设置入口。

备忘录交互按自然 DOM 顺序组织为搜索、标签筛选、列表、新建、标题、置顶、标签编辑、正文、提醒、删除和保存；已有记录列表尾部始终提供新建入口。共享 `Dialog` 负责初始焦点、Tab 循环、Escape 关闭和触发点焦点恢复，提醒字段错误会聚焦对应输入。备忘录按钮、筛选项、标签移除、分段控件和星期选择提供至少 40px 操作目标；低于 980px 时提醒字段同步改为单列，125% 文本缩放继续依靠面板独立滚动防止内容截断，减少动态效果偏好沿用全局 motion 契约。

`MemoWorkspace` 在详情 command 成功、记录切换和新草稿开始时同步更新 `selectedMemoRef`。这保证详情刚渲染即执行置顶或保存时使用正确记录 ID，并防止开始新草稿后的同一事件循环误用旧权威记录。

编辑器将最后请求输入与当前草稿分开保存。草稿变化后启动 500 毫秒计时，后续输入会重置计时；显式保存、Ctrl/Cmd+Enter 和标签或置顶操作会立即提交并更新最后请求值。保存期间标题、正文和元数据继续可编辑。`MemoWorkspace` 同一时间只执行一个保存请求，并把期间收到的输入收敛为一份最新排队草稿；当前请求完成后立即提交该草稿。存在排队输入时，中间 command 响应只更新内部权威记录，最终响应才回写编辑器，因此较早响应不会覆盖后续输入，新草稿也只执行一次 `memo_create`。失败时保留标题与正文草稿、恢复权威标签和置顶值，并把显式操作改为“重新保存”。

保存队列的纯状态边界位于 `src/features/memos/memoSaveQueue.ts`。`LatestMemoSaveQueue.enqueue` 在空闲时返回应立即执行的输入，在活动请求期间只覆盖一份最新排队值；`complete` 在存在排队值时返回下一输入并保持活动状态，队列清空后才回到空闲。React 编排和 Property M1 共享该实现。

已保存记录的删除入口位于编辑器固定操作栏。`MemoEditor` 使用共享 `Dialog` 展示权威 `displayTitle`，并明确说明记录、标签关联和未发送提醒的删除影响；取消会关闭对话框并由共享组件恢复触发点焦点。确认后 `MemoWorkspace` 调用 `memo_remove`，成功时清除详情并返回列表，失败时保留当前记录、刷新权威列表并在对话框中显示稳定错误文案，用户可直接重试。

`src-tauri/src/services/memo_service.rs` 提供独立于 SQLite 的备忘录核心领域逻辑。创建与更新统一校验输入、规范化标题、保留正文原始空白并维护审计时间；置顶首次发生时记录时间，连续编辑保持原置顶时间，取消置顶清除该时间。显示标题依次取规范化标题、正文首个非空行前 40 个 Unicode 字符和调用方提供的本地化无标题文案。

标签输入由 `MemoService::normalize_tags` 去除首尾空白并生成 Unicode 小写规范名，相同规范名保留首次输入的显示形式并合并为一个关联。`MemoRepository::replace_tags` 在单个 SQLite 事务中确认备忘录存在、复用全局标签、替换当前关联并清理全局孤立标签；任一步失败会回滚新标签与关联变化。

`MemoRepository::remove` 使用单个领域写事务删除备忘录。外键级联移除标签链接与提醒定义，随后清理失去全部关联的标签；共享标签继续保留。缺失记录返回 `MEMO_NOT_FOUND`，事务执行失败返回安全且稳定的 `MEMO_DELETE_FAILED`，回滚会恢复备忘录及全部依赖记录。

`MemoRepository` 的 create/update 在单个事务中写入核心字段、替换标签关系和替换可选提醒定义，随后通过 get 聚合完整 `MemoRecord`。get 在同一次数据库读取锁内读取核心记录、标签和可选提醒，并把提醒表行恢复为 once/recurring 判别联合；调用方提供当前语言的无标题文案用于派生 `displayTitle`。标签或提醒写入失败会连同核心字段、标签关系和原提醒状态一起回滚。

列表查询使用参数化 SQL 组合标题、正文和标签搜索及单标签筛选。搜索输入中的反斜杠、`%` 和 `_` 会先转义为 LIKE 字面量；所有结果统一按置顶状态、置顶时间倒序、更新时间倒序和 ID 升序排列，再聚合为包含 120 字符正文摘要的 `MemoSummary`。

标签筛选列表通过 `memo_tags` 与 `memo_tag_links` 实时聚合，返回仍有关联的标签和备忘录数量。关联替换与删除事务清理孤立标签，列表查询同时使用内连接和正计数约束，因此筛选项始终反映 SQLite 当前权威状态。

Repository 集成测试使用临时文件 SQLite 数据库跨越创建、搜索、筛选、更新、标签替换、提醒聚合和删除流程，并在关闭后重新打开数据库验证提交结果。失败场景通过 trigger 注入标签写入错误，重开数据库后确认核心字段与原标签关联均保持原值。

## 抵达 Focus 备份架构

抵达 Focus 的备份边界位于 `src-tauri/src/`。`domain/backup.rs` 定义版本 2 的 `BackupEnvelope`、全部可移植记录、导入摘要和预校验规则；`repositories/backup_repository.rs` 在同一次 SQLite 只读连接中按稳定顺序读取快照；`services/backup_service.rs` 负责生成格式化 JSON、识别格式版本、限制输入大小并返回 `ValidatedBackup`。导出固定生成版本 2，解析器继续接受版本 1，并在反序列化前把四个备忘录集合确定性转换为空集合。

备份数据覆盖项目、任务、检查项、重复规则、任务实例、专注轮次、活动专注、便签、周目标、偏好、备忘录、标签、标签关联和备忘录提醒。窗口位置、通知投递记录和备份历史属于设备或运行态数据，不进入可移植数据集。解析阶段会在数据库写入前校验结构、未知字段、时间日期、枚举、字符串长度、数值范围、集合数量、ID 唯一性及跨记录引用，并生成记录数量与最早、最晚业务日期摘要。备忘录预校验还固定规范标签名、每条备忘录最多十个标签和一个提醒，并验证提醒频率字段组合、状态、IANA 时区及活动提醒的下一 UTC 发生时间符合本地规则。

主窗口通过 Rust 侧 `tauri-plugin-dialog` 打开原生 JSON 保存或选择对话框，路径和文件内容始终由后端处理。恢复采用“选择并校验 → 展示摘要并确认 → 消费校验令牌”的三段式流程；待恢复数据保存在进程内，不经前端往返传输。

恢复事务获取 SQLite 写锁后读取当前业务快照，先把版本化 JSON 写入应用数据目录的 `backups/` 并登记 `pre_restore` 历史，再清理通知投递派生记录并按外键拓扑替换全部可移植业务数据。备忘录数据按标签关联、提醒、备忘录、标签顺序清理，再按备忘录、标签、标签关联、提醒顺序写入；任务实例先以空来源引用插入，随后统一恢复自引用字段。任意 SQL 或外键检查失败时事务自动回滚，独立快照文件继续保留，并在数据库可写时补记历史记录；成功后广播 `backup://restored`，主窗口与小组件重新读取权威数据。窗口位置、小组件布局、迁移记录和既有备份历史在恢复期间保持不变。

正确性属性 P9 使用 Rust `proptest` 生成零到四组带合法引用的业务记录，随机覆盖状态、重复模式、可选时间、完成值、跨实例来源和活动专注引用。每个样本依次执行版本化 JSON 序列化、正式解析、SQLite 事务恢复和再次导出，并比较恢复后的规范化业务模型与导入摘要。

正确性属性 M7 使用独立的 `proptest` 关系图生成器构造零到六条备忘录及其共享标签、标签关联和可选提醒。样本覆盖置顶状态、空集合、不同标签组合、一次性与重复提醒、活动/完成/取消状态以及 UTC、Asia/Shanghai、America/New_York 和 Europe/London 时区；每组先恢复到源 SQLite，再执行正式导出、解析、目标 SQLite 恢复和再次导出，最终按稳定标识排序后比较四类备忘录业务集合。

`tests/backup_restore.rs` 通过磁盘临时 SQLite 数据库验证公共备份边界：版本 1 输入确定转换为空备忘录集合，未知版本和版本 2 损坏标签引用在恢复前被拒绝；成功恢复会替换备忘录、标签、关联和提醒，生成可重新解析的旧数据快照，并在数据库重开后保持新数据；备忘录插入阶段的 SQL trigger 故障注入会触发事务回滚，同时保留原项目、原备忘录、恢复前快照及其历史记录。

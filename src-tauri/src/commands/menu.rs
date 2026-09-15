//! 原生菜单（仅 macOS）：系统栏渲染，菜单项动作经 `menu-action` 事件桥回前端
//! （处理函数在 `lib.rs` 的 `on_menu_event`）。
//!
//! Windows / Linux 用无边框窗口，标题栏里的 web 菜单见前端 `MenuBar.vue`——原生
//! 菜单在无边框窗口不渲染，故不在这些平台上构建（模块整体按平台裁剪）。

use tauri::menu::{
    AboutMetadata, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder,
};

/// 构建并安装应用菜单。
pub fn install(app: &tauri::App) -> tauri::Result<()> {
    let action = |id: &str,
                  label: &str,
                  accelerator: Option<&str>|
     -> Result<tauri::menu::MenuItem<tauri::Wry>, tauri::Error> {
        let mut item = MenuItemBuilder::with_id(id, label);
        if let Some(accelerator) = accelerator {
            item = item.accelerator(accelerator);
        }
        item.build(app)
    };

    let version = app.package_info().version.to_string();
    let credits =
        "感谢 OpenFOAM (openfoam.org) 提供 CFD 求解基座\n感谢 moldingFoam 项目提供注塑求解模块";
    let about_metadata = |version: &str| AboutMetadata {
        name: Some("Kairos".into()),
        version: Some(version.into()),
        copyright: Some("Copyright © 2026 Yuki".into()),
        credits: Some(credits.into()),
        ..Default::default()
    };

    // macOS 规范：首个子菜单是应用菜单（关于 / 服务 / 隐藏 / 退出），
    // 没有它 ⌘Q 退出与「关于」入口都会缺席。
    // 系统预定义项默认英文标题（About / Hide / Quit…），统一给中文标签。
    let app_menu = {
        let about =
            PredefinedMenuItem::about(app, Some("关于 Kairos"), Some(about_metadata(&version)))?;
        let services = PredefinedMenuItem::services(app, Some("服务"))?;
        let hide = PredefinedMenuItem::hide(app, Some("隐藏 Kairos"))?;
        let hide_others = PredefinedMenuItem::hide_others(app, Some("隐藏其他"))?;
        let show_all = PredefinedMenuItem::show_all(app, Some("显示全部"))?;
        let quit = PredefinedMenuItem::quit(app, Some("退出 Kairos"))?;
        SubmenuBuilder::new(app, "Kairos")
            .item(&about)
            .separator()
            .item(&services)
            .separator()
            .item(&hide)
            .item(&hide_others)
            .item(&show_all)
            .separator()
            .item(&quit)
            .build()?
    };

    let file = SubmenuBuilder::new(app, "文件")
        .item(&action("file.new", "新建项目", Some("CmdOrCtrl+N"))?)
        .item(&action("file.open", "打开项目…", Some("CmdOrCtrl+O"))?)
        .separator()
        .item(&action("file.save", "保存", Some("CmdOrCtrl+S"))?)
        .item(&action(
            "file.saveAs",
            "另存为…",
            Some("Shift+CmdOrCtrl+S"),
        )?)
        .build()?;

    // 编辑菜单用系统预定义项：没有它 macOS 的 ⌘C/⌘V 在输入框里不生效。
    let edit = {
        let undo = PredefinedMenuItem::undo(app, Some("撤销"))?;
        let redo = PredefinedMenuItem::redo(app, Some("重做"))?;
        let cut = PredefinedMenuItem::cut(app, Some("剪切"))?;
        let copy = PredefinedMenuItem::copy(app, Some("拷贝"))?;
        let paste = PredefinedMenuItem::paste(app, Some("粘贴"))?;
        let select_all = PredefinedMenuItem::select_all(app, Some("全选"))?;
        SubmenuBuilder::new(app, "编辑")
            .item(&undo)
            .item(&redo)
            .separator()
            .item(&cut)
            .item(&copy)
            .item(&paste)
            .separator()
            .item(&select_all)
            .build()?
    };

    let view = SubmenuBuilder::new(app, "视图")
        .item(&action("view.theme", "切换主题", None)?)
        .build()?;

    // 工具聚合校验、依赖与虚拟机管理
    let tools = SubmenuBuilder::new(app, "工具")
        .item(&action("analysis.checkNetwork", "校验模具网络", None)?)
        .item(&action("tools.refreshDeps", "探测运行时依赖", None)?)
        .separator()
        .item(&action("tools.vmPanel", "虚拟机面板", None)?)
        .item(&action("tools.vmStart", "启动虚拟机", None)?)
        .item(&action("tools.vmShell", "进入虚拟机 Shell", None)?)
        .item(&action("tools.vmStop", "关闭虚拟机", None)?)
        .build()?;

    let results = SubmenuBuilder::new(app, "结果")
        .item(&action("results.exportCsv", "导出当前场为 CSV", None)?)
        .build()?;

    let report = SubmenuBuilder::new(app, "报告")
        .item(&action("report.open", "打开报告工作台", None)?)
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&app_menu)
        .item(&file)
        .item(&edit)
        .item(&view)
        .item(&tools)
        .item(&results)
        .item(&report)
        .build()?;
    app.set_menu(menu)?;
    Ok(())
}

use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::{
    CustomMenuItem, Manager, Menu, MenuItem, Submenu,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct SvnResult {
    success: bool,
    files: Vec<String>,
    error: Option<String>,
}

/// 检测系统是否安装了 SVN 命令
#[tauri::command]
fn check_svn(svn_path: Option<String>) -> Result<bool, String> {
    let cmd = svn_path
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "svn".to_string());

    let output = Command::new(cmd).arg("--version").arg("--quiet").output();

    match output {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}

/// 获取 SVN 远程文件列表
#[tauri::command]
fn fetch_svn_files(
    url: String,
    username: Option<String>,
    password: Option<String>,
    svn_path: Option<String>,
) -> Result<Vec<String>, String> {
    let cmd = svn_path
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "svn".to_string());

    // 检测 svn 命令是否存在
    let svn_exists = check_svn(svn_path.clone()).map_err(|e| format!("检测 SVN 失败: {}", e))?;
    if !svn_exists {
        return Err("系统未安装 SVN 命令，请先安装 SVN".to_string());
    }

    // 构建 svn ls 命令
    let mut args = vec![
        "ls".to_string(),
        "-R".to_string(),
        "--non-interactive".to_string(),
        url.clone(),
    ];

    // 添加认证参数（如果有）
    if let Some(user) = &username {
        if !user.is_empty() {
            args.push("--username".to_string());
            args.push(user.clone());
        }
    }

    // 执行命令
    let output = Command::new(&cmd)
        .args(&args)
        .output()
        .map_err(|e| format!("执行 SVN 命令失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // 尝试带密码认证
        if stderr.contains("Authentication failed") || stderr.contains("Could not authenticate") {
            if let (Some(user), Some(pass)) = (&username, &password) {
                if !user.is_empty() && !pass.is_empty() {
                    let mut retry_args = vec![
                        "ls".to_string(),
                        "-R".to_string(),
                        "--non-interactive".to_string(),
                        "--username".to_string(),
                        user.clone(),
                        "--password".to_string(),
                        pass.clone(),
                        url.clone(),
                    ];

                    let retry_output = Command::new(&cmd)
                        .args(&retry_args)
                        .output()
                        .map_err(|e| format!("重试 SVN 命令失败: {}", e))?;

                    if retry_output.status.success() {
                        return parse_svn_output(
                            String::from_utf8_lossy(&retry_output.stdout).to_string(),
                        );
                    } else {
                        let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
                        return Err(format!("认证失败: {}", retry_stderr));
                    }
                }
            }
            return Err(format!("认证失败: {}", stderr));
        }

        return Err(format!("SVN 命令执行失败: {}", stderr));
    }

    parse_svn_output(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 解析 SVN 输出，转换为文件路径列表
fn parse_svn_output(output: String) -> Result<Vec<String>, String> {
    let mut files = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // SVN ls -R 输出格式：每行一个路径
        // 可能包含目录（以 / 结尾）
        if !line.ends_with('/') {
            files.push(line.to_string());
        }
    }

    Ok(files)
}

/// 自动检测 SVN 命令路径
#[tauri::command]
fn detect_svn_path() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("where").arg("svn").output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let path = stdout.lines().next()?.trim().to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("which").arg("svn").output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let path = stdout.lines().next()?.trim().to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }
}

/// 复制文本到剪贴板
#[tauri::command]
fn copy_to_clipboard(text: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("复制到剪贴板失败: {}", e))?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).ok();
        }
        
        child.wait().map_err(|e| format!("复制到剪贴板失败: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        let mut child = Command::new("cmd")
            .args(["/C", "clip"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("复制到剪贴板失败: {}", e))?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).ok();
        }
        
        child.wait().map_err(|e| format!("复制到剪贴板失败: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("复制到剪贴板失败: {}", e))?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).ok();
        }
        
        child.wait().map_err(|e| format!("复制到剪贴板失败: {}", e))?;
    }

    Ok(())
}

fn main() {
    let settings_item = CustomMenuItem::new("open-settings".to_string(), "设置");
    let app_menu = Menu::new()
        .add_submenu(Submenu::new(
            "SVN 文件搜索",
            Menu::new().add_item(settings_item),
        ))
        .add_native_item(MenuItem::Quit);

    tauri::Builder::default()
        .menu(app_menu)
        .on_menu_event(|event| {
            if event.menu_item_id() == "open-settings" {
                let _ = event.window().emit("open-settings", ());
            }
        })
        .setup(|app| {
            // 设置窗口
            let window = app.get_window("main").unwrap();
            window.set_title("SVN 文件搜索").ok();

            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_svn,
            fetch_svn_files,
             detect_svn_path,
            copy_to_clipboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

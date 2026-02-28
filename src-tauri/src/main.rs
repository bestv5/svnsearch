#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::menu::MenuBuilder;
use tauri::{Emitter, Manager};

mod database;
mod search_query;

/// 创建一个在 Windows 下不弹出黑框的命令执行器。
///
/// 说明：本应用在 Windows 上是 GUI 子系统（`windows_subsystem="windows"`），
/// 当它启动 `svn.exe` 这类控制台程序时，系统会默认弹出一个控制台窗口。
/// 通过设置 `CREATE_NO_WINDOW` 可以避免该黑框闪现。
#[allow(unused_mut)]
fn command_no_window<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SvnResult {
    success: bool,
    files: Vec<String>,
    error: Option<String>,
}

/// 名称分段（用于高亮渲染）
#[derive(Debug, Serialize, Deserialize)]
pub struct NameSegment {
    pub text: String,
    pub highlight: bool,
}

/// 搜索返回的条目（url + path + 是否目录 + 名称分段高亮）
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexEntry {
    pub url: String,
    pub path: String,
    pub is_dir: bool,
    pub name_segments: Vec<NameSegment>,
}

/// 检测系统是否安装了 SVN 命令
#[tauri::command]
fn check_svn(svn_path: Option<String>) -> Result<bool, String> {
    let cmd = svn_path
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "svn".to_string());

    let output = command_no_window(&cmd)
        .arg("--version")
        .arg("--quiet")
        .output();

    match output {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}

/// 根据当前平台解码 SVN 命令输出，解决 Windows 下中文乱码问题
fn decode_svn_text(bytes: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        use encoding_rs::GBK;
        let (cow, _, _) = GBK.decode(bytes);
        cow.to_string()
    }

    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).to_string()
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
    let output = command_no_window(&cmd)
        .args(&args)
        .output()
        .map_err(|e| format!("执行 SVN 命令失败: {}", e))?;

    if !output.status.success() {
        let stderr = decode_svn_text(&output.stderr);

        // 尝试带密码认证
        if stderr.contains("Authentication failed") || stderr.contains("Could not authenticate") {
            if let (Some(user), Some(pass)) = (&username, &password) {
                if !user.is_empty() && !pass.is_empty() {
                    let retry_args = vec![
                        "ls".to_string(),
                        "-R".to_string(),
                        "--non-interactive".to_string(),
                        "--username".to_string(),
                        user.clone(),
                        "--password".to_string(),
                        pass.clone(),
                        url.clone(),
                    ];

                    let retry_output = command_no_window(&cmd)
                        .args(&retry_args)
                        .output()
                        .map_err(|e| format!("重试 SVN 命令失败: {}", e))?;

                    if retry_output.status.success() {
                        let stdout = decode_svn_text(&retry_output.stdout);
                        return parse_svn_output(stdout);
                    } else {
                        let retry_stderr = decode_svn_text(&retry_output.stderr);
                        return Err(format!("认证失败: {}", retry_stderr));
                    }
                }
            }
            return Err(format!("认证失败: {}", stderr));
        }

        return Err(format!("SVN 命令执行失败: {}", stderr));
    }

    let stdout = decode_svn_text(&output.stdout);
    parse_svn_output(stdout)
}

/// 解析 SVN 输出，转换为路径列表（含文件与目录；目录以 / 结尾，会一并写入索引）
fn parse_svn_output(output: String) -> Result<Vec<String>, String> {
    let mut list = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        list.push(line.to_string());
    }
    Ok(list)
}

/// 将索引保存到数据库（按仓库 URL）
#[tauri::command]
fn save_index(url: String, files: Vec<String>) -> Result<(), String> {
    database::save_index(&url, &files)
}

/// 从数据库加载某仓库的索引
#[tauri::command]
fn load_index(url: String) -> Result<Vec<String>, String> {
    database::load_index(&url)
}

/// 清空某仓库的索引
#[tauri::command]
fn clear_index(url: String) -> Result<(), String> {
    database::clear_index(&url)
}

/// 搜索所有仓库的索引（按新语法仅匹配名称，返回条目及名称高亮分段）
#[tauri::command]
fn search_index(query: String, limit: Option<u32>) -> Result<Vec<IndexEntry>, String> {
    let limit = limit.unwrap_or(200);
    let rows = database::search_index(&query, limit)?;
    Ok(rows
        .into_iter()
        .map(|(url, path, is_dir, segs)| IndexEntry {
            url,
            path,
            is_dir,
            name_segments: segs
                .into_iter()
                .map(|(text, highlight)| NameSegment { text, highlight })
                .collect(),
        })
        .collect())
}

/// 自动检测 SVN 命令路径
#[tauri::command]
fn detect_svn_path() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let output = command_no_window("where").arg("svn").output().ok()?;
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
        let output = command_no_window("which").arg("svn").output().ok()?;
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
        let mut child = command_no_window("pbcopy")
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
        let mut child = command_no_window("cmd")
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
        let mut child = command_no_window("xclip")
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
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let menu = MenuBuilder::new(&handle)
                .text("open-settings", "设置")
                .quit_with_text("退出")
                .build()
                .map_err(|e| e.to_string())?;
            let _ = app.set_menu(menu);

            app.on_menu_event(move |app, event| {
                if event.id().as_ref() == "open-settings" {
                    let _ = app.emit("open-settings", ());
                }
            });

            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_title("SVN 文件搜索");
                #[cfg(debug_assertions)]
                {
                    let _ = win.open_devtools();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_svn,
            fetch_svn_files,
            detect_svn_path,
            copy_to_clipboard,
            save_index,
            load_index,
            clear_index,
            search_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

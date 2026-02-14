use tauri::Manager;
use std::env;

#[cfg(target_os = "macos")]
pub fn set_autostart(enable: bool) -> Result<(), String> {
    use std::process::Command;
    
    if enable {
        let output = Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to make login item at end of login items with properties {path:\"/Applications/SVN Search.app\", hidden:false}")
            .output()
            .map_err(|e| format!("Failed to set autostart: {}", e))?;
        
        if !output.status.success() {
            return Err("Failed to set autostart".to_string());
        }
    } else {
        let output = Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to delete login item \"SVN Search\"")
            .output()
            .map_err(|e| format!("Failed to disable autostart: {}", e))?;
        
        if !output.status.success() {
            return Err("Failed to disable autostart".to_string());
        }
    }
    
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn set_autostart(enable: bool) -> Result<(), String> {
    use std::path::PathBuf;
    use std::fs;
    
    let mut startup_path = PathBuf::from(env::var("APPDATA").unwrap_or_else(|_| ".".to_string()));
    startup_path.push("Microsoft");
    startup_path.push("Windows");
    startup_path.push("Start Menu");
    startup_path.push("Programs");
    startup_path.push("Startup");
    
    fs::create_dir_all(&startup_path).map_err(|e| format!("Failed to create startup directory: {}", e))?;
    
    let shortcut_path = startup_path.join("SVN Search.lnk");
    
    if enable {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?;
        
        let vbs_script = format!(
            r#"
Set WshShell = CreateObject("WScript.Shell")
Set Shortcut = WshShell.CreateShortcut("{}")
Shortcut.TargetPath = "{}"
Shortcut.Save
"#,
            shortcut_path.display(),
            exe_path.display()
        );
        
        let vbs_path = startup_path.join("create_shortcut.vbs");
        fs::write(&vbs_path, vbs_script)
            .map_err(|e| format!("Failed to write vbs script: {}", e))?;
        
        Command::new("wscript.exe")
            .arg(&vbs_path)
            .output()
            .map_err(|e| format!("Failed to create shortcut: {}", e))?;
        
        fs::remove_file(vbs_path).ok();
    } else {
        if shortcut_path.exists() {
            fs::remove_file(&shortcut_path)
                .map_err(|e| format!("Failed to remove shortcut: {}", e))?;
        }
    }
    
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn set_autostart(enable: bool) -> Result<(), String> {
    use std::path::PathBuf;
    use std::fs;
    
    let mut desktop_path = PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".to_string()));
    desktop_path.push(".config");
    desktop_path.push("autostart");
    fs::create_dir_all(&desktop_path).map_err(|e| format!("Failed to create autostart directory: {}", e))?;
    
    let desktop_file_path = desktop_path.join("svnsearch.desktop");
    
    if enable {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?;
        
        let desktop_content = format!(
            r#"[Desktop Entry]
Type=Application
Name=SVN Search
Exec={}
Icon=svnsearch
Terminal=false
Categories=Utility;
"#,
            exe_path.display()
        );
        
        fs::write(&desktop_file_path, desktop_content)
            .map_err(|e| format!("Failed to write desktop file: {}", e))?;
    } else {
        if desktop_file_path.exists() {
            fs::remove_file(&desktop_file_path)
                .map_err(|e| format!("Failed to remove desktop file: {}", e))?;
        }
    }
    
    Ok(())
}

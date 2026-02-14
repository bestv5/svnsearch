use std::process::Command;
use std::str;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvnFile {
    pub path: String,
    pub filename: String,
    pub is_dir: bool,
    pub size: u64,
    pub revision: u64,
    pub last_modified: Option<String>,
}

pub struct SvnClient {
    svn_path: String,
}

impl SvnClient {
    pub fn new(svn_path: Option<String>) -> Self {
        Self {
            svn_path: svn_path.unwrap_or_else(|| "svn".to_string()),
        }
    }

    pub fn test_connection(&self, url: &str, username: Option<&str>, password: Option<&str>) -> Result<bool, String> {
        let mut cmd = Command::new(&self.svn_path);
        cmd.args(["info", url, "--xml"]);
        cmd.args(["--non-interactive", "--trust-server-cert"]);
        
        if let Some(user) = username {
            cmd.args(["--username", user]);
        }
        if let Some(pass) = password {
            cmd.args(["--password", pass]);
        }

        let output = cmd.output()
            .map_err(|e| format!("Failed to execute svn: {}", e))?;

        Ok(output.status.success())
    }

    pub fn list_directory(&self, url: &str, username: Option<&str>, password: Option<&str>, recursive: bool) -> Result<Vec<SvnFile>, String> {
        let mut cmd = Command::new(&self.svn_path);
        cmd.args(["list", url, "--xml"]);
        
        if recursive {
            cmd.arg("-R");
        }
        
        cmd.args(["--non-interactive", "--trust-server-cert"]);
        
        if let Some(user) = username {
            cmd.args(["--username", user]);
        }
        if let Some(pass) = password {
            cmd.args(["--password", pass]);
        }

        let output = cmd.output()
            .map_err(|e| format!("Failed to execute svn: {}", e))?;

        if !output.status.success() {
            let stderr = str::from_utf8(&output.stderr).unwrap_or("");
            return Err(format!("SVN list failed: {}", stderr));
        }

        let stdout = str::from_utf8(&output.stdout).unwrap_or("");
        self.parse_list_output(stdout, url)
    }

    fn parse_list_output(&self, xml: &str, base_url: &str) -> Result<Vec<SvnFile>, String> {
        let mut files = Vec::new();
        
        let mut current_depth = 0;
        let mut path_stack = Vec::new();

        for line in xml.lines() {
            if line.contains("<entry") {
                if let Some(path_start) = line.find("path=\"") {
                    if let Some(path_end) = line[path_start + 6..].find("\"") {
                        let path = &line[path_start + 6..path_start + 6 + path_end];
                        
                        let depth = path.matches('/').count();
                        while path_stack.len() > depth {
                            path_stack.pop();
                        }
                        
                        let full_path = if path_stack.is_empty() {
                            path.to_string()
                        } else {
                            format!("{}/{}", path_stack.join("/"), path)
                        };
                        
                        let filename = if let Some(last_slash) = full_path.rfind('/') {
                            full_path[last_slash + 1..].to_string()
                        } else {
                            full_path.clone()
                        };
                        
                        let is_dir = line.contains("kind=\"dir\"");
                        let size = self.extract_size(line);
                        let revision = self.extract_revision(line);
                        
                        files.push(SvnFile {
                            path: full_path,
                            filename,
                            is_dir,
                            size,
                            revision,
                            last_modified: None,
                        });
                        
                        path_stack.push(path.to_string());
                    }
                }
            }
        }

        Ok(files)
    }

    fn extract_size(&self, line: &str) -> u64 {
        if let Some(size_start) = line.find("size=\"") {
            if let Some(size_end) = line[size_start + 6..].find("\"") {
                return line[size_start + 6..size_start + 6 + size_end]
                    .parse()
                    .unwrap_or(0);
            }
        }
        0
    }

    fn extract_revision(&self, line: &str) -> u64 {
        if let Some(rev_start) = line.find("revision=\"") {
            if let Some(rev_end) = line[rev_start + 10..].find("\"") {
                return line[rev_start + 10..rev_start + 10 + rev_end]
                    .parse()
                    .unwrap_or(0);
            }
        }
        0
    }
}

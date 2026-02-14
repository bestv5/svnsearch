use serde::{Deserialize, Serialize};
use std::process::Command;
use std::str;

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

    pub fn test_connection(
        &self,
        url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<bool, String> {
        let mut cmd = Command::new(&self.svn_path);
        cmd.args(["info", url, "--xml"]);
        cmd.args(["--non-interactive", "--trust-server-cert"]);

        if let Some(user) = username {
            cmd.args(["--username", user]);
        }
        if let Some(pass) = password {
            cmd.args(["--password", pass]);
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute svn: {}", e))?;

        Ok(output.status.success())
    }

    pub fn get_latest_revision(
        &self,
        url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<u64, String> {
        let mut cmd = Command::new(&self.svn_path);
        cmd.args(["info", url, "--xml"]);
        cmd.args(["--non-interactive", "--trust-server-cert"]);

        if let Some(user) = username {
            cmd.args(["--username", user]);
        }
        if let Some(pass) = password {
            cmd.args(["--password", pass]);
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute svn: {}", e))?;

        if !output.status.success() {
            let stderr = str::from_utf8(&output.stderr).unwrap_or("");
            return Err(format!("SVN info failed: {}", stderr));
        }

        let stdout = str::from_utf8(&output.stdout).unwrap_or("");
        self.parse_revision_from_info(stdout)
    }

    fn parse_revision_from_info(&self, xml: &str) -> Result<u64, String> {
        for line in xml.lines() {
            if let Some(start) = line.find("<commit revision=\"") {
                let start_idx = start + 18;
                if let Some(end) = line[start_idx..].find("\"") {
                    let revision_str = &line[start_idx..start_idx + end];
                    return revision_str.parse::<u64>().map_err(|e| e.to_string());
                }
            }
        }
        Err("Could not find revision in SVN info output".to_string())
    }

    pub fn list_directory(
        &self,
        url: &str,
        username: Option<&str>,
        password: Option<&str>,
        recursive: bool,
    ) -> Result<Vec<SvnFile>, String> {
        self.list_directory_at_revision(url, username, password, 0)
    }

    pub fn list_directory_at_revision(
        &self,
        url: &str,
        username: Option<&str>,
        password: Option<&str>,
        revision: u64,
    ) -> Result<Vec<SvnFile>, String> {
        let mut cmd = Command::new(&self.svn_path);

        if revision > 0 {
            cmd.args(["list", url, "-r", &revision.to_string(), "--xml"]);
        } else {
            cmd.args(["list", url, "--xml"]);
        }

        cmd.arg("-R");
        cmd.args(["--non-interactive", "--trust-server-cert"]);

        if let Some(user) = username {
            cmd.args(["--username", user]);
        }
        if let Some(pass) = password {
            cmd.args(["--password", pass]);
        }

        let output = cmd
            .output()
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

                        if filename.is_empty() {
                            continue;
                        }

                        let is_dir = line.contains("kind=\"dir\"");
                        let size = self.extract_size(line);
                        let revision = self.extract_revision(line);
                        let last_modified = self.extract_date(line);

                        files.push(SvnFile {
                            path: full_path,
                            filename,
                            is_dir,
                            size,
                            revision,
                            last_modified,
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

    fn extract_date(&self, line: &str) -> Option<String> {
        if let Some(date_start) = line.find("<date>") {
            let start_idx = date_start + 6;
            if let Some(date_end) = line[start_idx..].find("</date>") {
                let date_str = &line[start_idx..start_idx + date_end];
                if let Some(iso_end) = date_str.find('T') {
                    return Some(date_str[..iso_end].to_string());
                }
                return Some(date_str.to_string());
            }
        }
        None
    }
}

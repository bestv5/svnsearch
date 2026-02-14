import subprocess
import re
import base64
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Optional, Tuple
from xml.etree import ElementTree
from config import load_config

class SVNClient:
    def __init__(self, svn_path=None):
        self.svn_path = svn_path or load_config().get('svn_path', 'svn')
        self.default_timeout = 1800  # 30分钟
        self.list_timeout = 3600     # 60分钟用于列表操作
        self.content_timeout = 300    # 5分钟用于文件内容
    
    def _run_command(self, args: List[str], username: str = None, password: str = None, 
                     timeout: int = None) -> Tuple[int, str, str]:
        if timeout is None:
            timeout = self.default_timeout
        
        cmd = [self.svn_path] + args
        cmd.extend(['--non-interactive', '--trust-server-cert-failures=unknown-ca'])
        
        if username:
            cmd.extend(['--username', username])
        if password:
            cmd.extend(['--password', password])
        
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
                encoding='utf-8',
                errors='replace'
            )
            return result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            return -1, '', 'Command timeout'
        except FileNotFoundError:
            return -1, '', f'SVN executable not found: {self.svn_path}'
    
    def test_connection(self, url: str, username: str = None, password: str = None) -> Tuple[bool, str]:
        returncode, stdout, stderr = self._run_command(
            ['info', url, '--xml'],
            username, password,
            timeout=30
        )
        
        if returncode == 0:
            return True, 'Connection successful'
        return False, stderr or 'Connection failed'
    
    def list_directory(self, url: str, username: str = None, password: str = None,
                       recursive: bool = True) -> List[Dict]:
        args = ['list', url, '--xml']
        if recursive:
            args.append('-R')
        
        returncode, stdout, stderr = self._run_command(args, username, password, self.list_timeout)
        
        if returncode != 0:
            raise Exception(stderr or 'Failed to list directory')
        
        return self._parse_list_output(stdout, url)
    
    def _parse_list_output(self, xml_output: str, base_url: str) -> List[Dict]:
        files = []
        try:
            root = ElementTree.fromstring(xml_output)
            base_url = base_url.rstrip('/')
            
            for entry in root.findall('.//entry'):
                path = entry.get('path', '')
                kind = entry.get('kind', 'file')
                
                size_elem = entry.find('size')
                size = int(size_elem.text) if size_elem is not None and size_elem.text else 0
                
                commit = entry.find('commit')
                revision = int(commit.get('revision', 0)) if commit is not None else 0
                
                last_modified = None
                if commit is not None:
                    date_elem = commit.find('date')
                    if date_elem is not None and date_elem.text:
                        last_modified = date_elem.text
                
                filename = Path(path).name or path
                if filename.isEmpty() and not path.isEmpty():
                    filename = path
                
                files.append({
                    'path': path,
                    'filename': filename,
                    'is_dir': kind == 'dir',
                    'size': size,
                    'revision': revision,
                    'last_modified': last_modified,
                    'full_url': f'{base_url}/{path}'
                })
        except ElementTree.ParseError as e:
            pass
        
        return files
    
    def get_file_content(self, url: str, username: str = None, password: str = None) -> Tuple[bytes, str]:
        returncode, stdout, stderr = self._run_command(
            ['cat', url],
            username, password,
            self.content_timeout
        )
        
        if returncode != 0:
            raise Exception(stderr or 'Failed to get file content')
        
        return stdout.encode('utf-8', errors='replace'), 'text'
    
    def get_info(self, url: str, username: str = None, password: str = None) -> Optional[Dict]:
        returncode, stdout, stderr = self._run_command(
            ['info', url, '--xml'],
            username, password
        )
        
        if returncode != 0:
            return None
        
        try:
            root = ElementTree.fromstring(stdout)
            entry = root.find('.//entry')
            if entry is not None:
                return {
                    'url': entry.find('url').text if entry.find('url') is not None else None,
                    'revision': entry.get('revision'),
                    'kind': entry.get('kind'),
                    'root': entry.find('root').text if entry.find('root') is not None else None,
                }
        except ElementTree.ParseError:
            pass
        
        return None

svn_client = SVNClient()

import sqlite3
import os
from pathlib import Path
from datetime import datetime
from contextlib import contextmanager
from config import DB_FILE, ensure_data_dir

class Database:
    def __init__(self):
        self.db_path = DB_FILE
        ensure_data_dir()
        self.init_db()
    
    @contextmanager
    def get_connection(self):
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        try:
            yield conn
        finally:
            conn.close()
    
    def init_db(self):
        with self.get_connection() as conn:
            conn.executescript('''
                CREATE TABLE IF NOT EXISTS repositories (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    url TEXT NOT NULL UNIQUE,
                    username TEXT,
                    password TEXT,
                    last_update TEXT,
                    created_at TEXT DEFAULT CURRENT_TIMESTAMP
                );
                
                CREATE TABLE IF NOT EXISTS file_index (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    repo_id INTEGER NOT NULL,
                    path TEXT NOT NULL,
                    filename TEXT NOT NULL,
                    is_dir INTEGER DEFAULT 0,
                    size INTEGER DEFAULT 0,
                    revision INTEGER DEFAULT 0,
                    last_modified TEXT,
                    indexed_at TEXT DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (repo_id) REFERENCES repositories(id),
                    UNIQUE(repo_id, path)
                );
                
                CREATE INDEX IF NOT EXISTS idx_filename ON file_index(filename);
                CREATE INDEX IF NOT EXISTS idx_path ON file_index(path);
                CREATE INDEX IF NOT EXISTS idx_repo_id ON file_index(repo_id);
            ''')
            conn.commit()
    
    def add_repository(self, name, url, username=None, password=None):
        with self.get_connection() as conn:
            cursor = conn.execute(
                'INSERT OR REPLACE INTO repositories (name, url, username, password) VALUES (?, ?, ?, ?)',
                (name, url, username, password)
            )
            conn.commit()
            return cursor.lastrowid
    
    def get_repositories(self):
        with self.get_connection() as conn:
            return [dict(row) for row in conn.execute('SELECT * FROM repositories ORDER BY name')]
    
    def get_repository(self, repo_id):
        with self.get_connection() as conn:
            row = conn.execute('SELECT * FROM repositories WHERE id = ?', (repo_id,)).fetchone()
            return dict(row) if row else None
    
    def get_repository_by_url(self, url):
        with self.get_connection() as conn:
            row = conn.execute('SELECT * FROM repositories WHERE url = ?', (url,)).fetchone()
            return dict(row) if row else None
    
    def delete_repository(self, repo_id):
        with self.get_connection() as conn:
            conn.execute('DELETE FROM file_index WHERE repo_id = ?', (repo_id,))
            conn.execute('DELETE FROM repositories WHERE id = ?', (repo_id,))
            conn.commit()
    
    def update_repository(self, repo_id, name, url, username=None, password=None):
        with self.get_connection() as conn:
            conn.execute('''
                UPDATE repositories 
                SET name = ?, url = ?, username = ?, password = ?
                WHERE id = ?
            ''', (name, url, username, password, repo_id))
            conn.commit()
    
    def update_repo_time(self, repo_id):
        with self.get_connection() as conn:
            conn.execute(
                'UPDATE repositories SET last_update = ? WHERE id = ?',
                (datetime.now().isoformat(), repo_id)
            )
            conn.commit()
    
    def clear_repo_index(self, repo_id):
        with self.get_connection() as conn:
            conn.execute('DELETE FROM file_index WHERE repo_id = ?', (repo_id,))
            conn.commit()
    
    def add_file(self, repo_id, path, filename, is_dir=False, size=0, revision=0, last_modified=None):
        with self.get_connection() as conn:
            conn.execute('''
                INSERT OR REPLACE INTO file_index 
                (repo_id, path, filename, is_dir, size, revision, last_modified)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            ''', (repo_id, path, filename, int(is_dir), size, revision, last_modified))
    
    def bulk_add_files(self, files):
        with self.get_connection() as conn:
            conn.executemany('''
                INSERT OR REPLACE INTO file_index 
                (repo_id, path, filename, is_dir, size, revision, last_modified)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            ''', files)
            conn.commit()
    
    def search_files(self, query, repo_id=None, limit=1000):
        with self.get_connection() as conn:
            sql = '''
                SELECT f.*, r.name as repo_name, r.url as repo_url
                FROM file_index f
                JOIN repositories r ON f.repo_id = r.id
                WHERE f.filename LIKE ?
            '''
            params = [f'%{query}%']
            
            if repo_id:
                sql += ' AND f.repo_id = ?'
                params.append(repo_id)
            
            sql += ' ORDER BY f.filename LIMIT ?'
            params.append(limit)
            
            return [dict(row) for row in conn.execute(sql, params)]
    
    def get_file_count(self, repo_id=None):
        with self.get_connection() as conn:
            if repo_id:
                return conn.execute('SELECT COUNT(*) FROM file_index WHERE repo_id = ?', (repo_id,)).fetchone()[0]
            return conn.execute('SELECT COUNT(*) FROM file_index').fetchone()[0]

db = Database()

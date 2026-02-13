import threading
from datetime import datetime
from typing import Dict, Optional
from models import db
from svn_client import svn_client

class IndexService:
    def __init__(self):
        self._indexing_status: Dict[int, Dict] = {}
        self._lock = threading.Lock()
    
    def get_status(self, repo_id: int = None) -> Dict:
        with self._lock:
            if repo_id:
                return self._indexing_status.get(repo_id, {'status': 'idle'})
            return dict(self._indexing_status)
    
    def index_repository(self, repo_id: int) -> Dict:
        repo = db.get_repository(repo_id)
        if not repo:
            return {'success': False, 'error': 'Repository not found'}
        
        with self._lock:
            if repo_id in self._indexing_status and self._indexing_status[repo_id].get('status') == 'indexing':
                return {'success': False, 'error': 'Already indexing'}
            self._indexing_status[repo_id] = {
                'status': 'indexing',
                'progress': 0,
                'message': 'Starting...',
                'start_time': datetime.now().isoformat()
            }
        
        thread = threading.Thread(target=self._index_repo, args=(repo,))
        thread.start()
        
        return {'success': True, 'message': 'Indexing started'}
    
    def _index_repo(self, repo: Dict):
        repo_id = repo['id']
        try:
            self._update_status(repo_id, 'message', 'Fetching file list...')
            
            files = svn_client.list_directory(
                repo['url'],
                repo.get('username'),
                repo.get('password'),
                recursive=True
            )
            
            total = len(files)
            self._update_status(repo_id, 'total', total)
            
            batch = []
            batch_size = 1000
            processed = 0
            
            for file_info in files:
                batch.append((
                    repo_id,
                    file_info['path'],
                    file_info['filename'],
                    int(file_info['is_dir']),
                    file_info['size'],
                    file_info['revision'],
                    file_info['last_modified']
                ))
                
                if len(batch) >= batch_size:
                    db.bulk_add_files(batch)
                    batch = []
                
                processed += 1
                if processed % 100 == 0:
                    self._update_status(repo_id, 'progress', processed)
                    self._update_status(repo_id, 'message', f'Indexed {processed}/{total} files')
            
            if batch:
                db.bulk_add_files(batch)
            
            db.update_repo_time(repo_id)
            
            with self._lock:
                self._indexing_status[repo_id] = {
                    'status': 'completed',
                    'progress': total,
                    'total': total,
                    'message': f'Completed: {total} files indexed',
                    'end_time': datetime.now().isoformat()
                }
                
        except Exception as e:
            with self._lock:
                self._indexing_status[repo_id] = {
                    'status': 'error',
                    'message': str(e),
                    'end_time': datetime.now().isoformat()
                }
    
    def _update_status(self, repo_id: int, key: str, value):
        with self._lock:
            if repo_id in self._indexing_status:
                self._indexing_status[repo_id][key] = value
    
    def index_all_repositories(self) -> Dict:
        repos = db.get_repositories()
        results = []
        for repo in repos:
            result = self.index_repository(repo['id'])
            results.append({'repo_id': repo['id'], 'name': repo['name'], 'result': result})
        return {'results': results}

index_service = IndexService()

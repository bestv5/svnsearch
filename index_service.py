import threading
import logging
from datetime import datetime
from typing import Dict, Optional, List
from models import db
from svn_client import svn_client

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class IndexService:
    def __init__(self):
        self._indexing_status: Dict[int, Dict] = {}
        self._lock = threading.Lock()
        self.batch_size = 500  # 每批处理 500 个文件
        self.current_batch: Dict[int, List] = {}  # 记录当前批次的文件
    
    def get_status(self, repo_id: int = None) -> Dict:
        with self._lock:
            if repo_id:
                return self._indexing_status.get(repo_id, {'status': 'idle'})
            return dict(self._indexing_status)
    
    def index_repository(self, repo_id: int, start_index: int = 0) -> Dict:
        repo = db.get_repository(repo_id)
        if not repo:
            return {'success': False, 'error': 'Repository not found'}
        
        with self._lock:
            if repo_id in self._indexing_status and self._indexing_status[repo_id].get('status') == 'indexing':
                return {'success': False, 'error': 'Already indexing'}
            
            logger.info(f"开始索引仓库: {repo['name']} (ID: {repo_id})")
            self._indexing_status[repo_id] = {
                'status': 'indexing',
                'progress': 0,
                'message': 'Starting...',
                'start_time': datetime.now().isoformat(),
                'total': 0,
                'processed': 0
            }
        
        thread = threading.Thread(target=self._index_repo, args=(repo, start_index))
        thread.start()
        
        return {'success': True, 'message': 'Indexing started'}
    
    def _index_repo(self, repo: Dict, start_index: int = 0):
        repo_id = repo['id']
        try:
            logger.info(f"[{repo['name']}] 正在获取文件列表...")
            self._update_status(repo_id, 'message', 'Fetching file list...')
            
            files = svn_client.list_directory(
                repo['url'],
                repo.get('username'),
                repo.get('password'),
                recursive=True
            )
            
            total = len(files)
            self._update_status(repo_id, 'total', total)
            logger.info(f"[{repo['name']}] 获取到 {total} 个文件")
            
            # 分批处理文件
            batch_num = 0
            for i in range(start_index, len(files), self.batch_size):
                batch = files[i:i + self.batch_size]
                batch_num += 1
                
                logger.info(f"[{repo['name']}] 处理第 {batch_num} 批 ({len(batch)} 个文件)...")
                db.bulk_add_files(batch)
                
                processed = i + len(batch)
                progress = processed * 100 // total
                self._update_status(repo_id, 'progress', processed)
                logger.info(f"[{repo['name']}] 索引进度: {progress}% ({processed}/{total})")
            
            db.update_repo_time(repo_id)
            logger.info(f"[{repo['name']}] 索引完成！共 {total} 个文件")
            
            with self._lock:
                self._indexing_status[repo_id] = {
                    'status': 'completed',
                    'progress': total,
                    'total': total,
                    'message': f'Completed: {total} files indexed',
                    'end_time': datetime.now().isoformat()
                }
                
        except Exception as e:
            logger.error(f"[{repo['name']}] 索引失败: {str(e)}")
            with self._lock:
                self._indexing_status[repo_id] = {
                    'status': 'error',
                    'message': str(e),
                    'end_time': datetime.now().isoformat()
                }
    
    def continue_indexing(self, repo_id: int) -> Dict:
        """继续索引（从中断的地方开始）"""
        with self._lock:
            if repo_id not in self._indexing_status:
                return {'success': False, 'error': 'Not indexing'}
            
            status = self._indexing_status[repo_id]
            if status.get('status') != 'indexing':
                return {'success': False, 'error': 'Cannot continue - not in indexing state'}
            
            processed = status.get('processed', 0)
            logger.info(f"[仓库 {repo_id}] 继续索引，从第 {processed} 个文件开始...")
            
            # 从上次处理的位置继续
            repo = db.get_repository(repo_id)
            files = svn_client.list_directory(
                repo['url'],
                repo.get('username'),
                repo.get('password'),
                recursive=True
            )
            
            total = len(files)
            self._update_status(repo_id, 'total', total)
            
            batch_num = processed // self.batch_size + 1
            for i in range(processed, len(files), self.batch_size):
                batch = files[i:i + self.batch_size]
                batch_num += 1
                
                logger.info(f"[仓库 {repo_id}] 处理第 {batch_num} 批 ({len(batch)} 个文件)...")
                db.bulk_add_files(batch)
                
                new_processed = i + len(batch)
                progress = new_processed * 100 // total
                self._update_status(repo_id, 'progress', new_processed)
                logger.info(f"[仓库 {repo_id}] 索引进度: {progress}% ({new_processed}/{total})")
            
            db.update_repo_time(repo_id)
            logger.info(f"[仓库 {repo_id}] 索引完成！共 {total} 个文件")
            
            with self._lock:
                self._indexing_status[repo_id] = {
                    'status': 'completed',
                    'progress': total,
                    'total': total,
                    'message': f'Completed: {total} files indexed',
                    'end_time': datetime.now().isoformat()
                }
        
        return {'success': True, 'message': 'Indexing continued'}
    
    def _update_status(self, repo_id: int, key: str, value):
        with self._lock:
            if repo_id in self._indexing_status:
                self._indexing_status[repo_id][key] = value
                logger.debug(f"[仓库 {repo_id}] 更新状态: {key} = {value}")
    
    def index_all_repositories(self) -> Dict:
        repos = db.get_repositories()
        logger.info(f"开始批量索引 {len(repos)} 个仓库...")
        results = []
        for repo in repos:
            result = self.index_repository(repo['id'])
            results.append({'repo_id': repo['id'], 'name': repo['name'], 'result': result})
        return {'results': results}

index_service = IndexService()

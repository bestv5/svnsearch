from flask import Flask, request, jsonify, send_from_directory, Response
from flask_cors import CORS
import os
import base64
from config import load_config, save_config, ensure_data_dir
from models import db
from svn_client import svn_client
from index_service import index_service
from apscheduler.schedulers.background import BackgroundScheduler
from apscheduler.triggers.interval import IntervalTrigger

app = Flask(__name__, static_folder='static')
CORS(app)

scheduler = BackgroundScheduler()
scheduler.start()

def schedule_index_updates():
    config = load_config()
    interval = config.get('update_interval', 60)
    
    scheduler.remove_all_jobs()
    
    if interval > 0:
        scheduler.add_job(
            func=index_service.index_all_repositories,
            trigger=IntervalTrigger(minutes=interval),
            id='auto_index',
            replace_existing=True
        )

@app.route('/')
def index():
    return send_from_directory('static', 'index.html')

@app.route('/api/config', methods=['GET'])
def get_config():
    config = load_config()
    repos = db.get_repositories()
    config['repositories'] = repos
    return jsonify(config)

@app.route('/api/config', methods=['POST'])
def update_config():
    data = request.json
    config = load_config()
    
    if 'data_dir' in data:
        config['data_dir'] = data['data_dir']
        save_config(config)
    
    if 'update_interval' in data:
        config['update_interval'] = data['update_interval']
        save_config(config)
        schedule_index_updates()
    
    if 'svn_path' in data:
        config['svn_path'] = data['svn_path']
        save_config(config)
    
    return jsonify({'success': True, 'config': config})

@app.route('/api/repositories', methods=['GET'])
def get_repositories():
    repos = db.get_repositories()
    for repo in repos:
        repo['file_count'] = db.get_file_count(repo['id'])
    return jsonify(repos)

@app.route('/api/repositories', methods=['POST'])
def add_repository():
    data = request.json
    name = data.get('name')
    url = data.get('url')
    username = data.get('username')
    password = data.get('password')
    
    if not name or not url:
        return jsonify({'success': False, 'error': 'Name and URL are required'}), 400
    
    success, message = svn_client.test_connection(url, username, password)
    if not success:
        return jsonify({'success': False, 'error': f'Connection failed: {message}'}), 400
    
    repo_id = db.add_repository(name, url, username, password)
    
    return jsonify({'success': True, 'repo_id': repo_id})

@app.route('/api/repositories/<int:repo_id>', methods=['DELETE'])
def delete_repository(repo_id):
    db.delete_repository(repo_id)
    return jsonify({'success': True})

@app.route('/api/repositories/<int:repo_id>/index', methods=['POST'])
def index_repository(repo_id):
    result = index_service.index_repository(repo_id)
    return jsonify(result)

@app.route('/api/repositories/<int:repo_id>/status', methods=['GET'])
def get_index_status(repo_id):
    status = index_service.get_status(repo_id)
    return jsonify(status)

@app.route('/api/index/all', methods=['POST'])
def index_all():
    result = index_service.index_all_repositories()
    return jsonify(result)

@app.route('/api/search', methods=['GET'])
def search():
    query = request.args.get('q', '')
    repo_id = request.args.get('repo_id', type=int)
    limit = request.args.get('limit', 1000, type=int)
    
    if not query:
        return jsonify([])
    
    results = db.search_files(query, repo_id, limit)
    
    for r in results:
        r['full_url'] = f"{r['repo_url']}/{r['path']}"
    
    return jsonify(results)

@app.route('/api/file/content')
def get_file_content():
    url = request.args.get('url')
    repo_id = request.args.get('repo_id', type=int)
    
    if not url:
        return jsonify({'error': 'URL is required'}), 400
    
    repo = None
    if repo_id:
        repo = db.get_repository(repo_id)
    
    username = repo.get('username') if repo else None
    password = repo.get('password') if repo else None
    
    try:
        content, content_type = svn_client.get_file_content_binary(url, username, password)
        
        if content_type.startswith('image/'):
            encoded = base64.b64encode(content).decode('utf-8')
            return jsonify({
                'type': 'image',
                'content_type': content_type,
                'data': encoded
            })
        elif content_type.startswith('text/') or content_type == 'text/plain':
            try:
                text = content.decode('utf-8')
                return jsonify({
                    'type': 'text',
                    'content': text
                })
            except UnicodeDecodeError:
                encoded = base64.b64encode(content).decode('utf-8')
                return jsonify({
                    'type': 'binary',
                    'content_type': content_type,
                    'data': encoded
                })
        else:
            encoded = base64.b64encode(content).decode('utf-8')
            return jsonify({
                'type': 'binary',
                'content_type': content_type,
                'data': encoded
            })
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/stats')
def get_stats():
    repos = db.get_repositories()
    total_files = db.get_file_count()
    
    return jsonify({
        'repository_count': len(repos),
        'total_files': total_files,
        'repositories': [{
            'id': r['id'],
            'name': r['name'],
            'last_update': r['last_update'],
            'file_count': db.get_file_count(r['id'])
        } for r in repos]
    })

ensure_data_dir()
schedule_index_updates()

if __name__ == '__main__':
    import webbrowser
    import threading
    import time
    
    def open_browser():
        time.sleep(1.5)
        webbrowser.open('http://localhost:5000')
    
    threading.Thread(target=open_browser, daemon=True).start()
    app.run(host='0.0.0.0', port=5000, debug=True)

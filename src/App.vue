<template>
  <div class="app">
    <div class="search-box">
      <input 
        type="text" 
        class="search-input" 
        v-model="searchQuery"
        @input="handleSearch"
        @focus="showHistory"
        placeholder="搜索文件... (Ctrl+Space)"
        ref="searchInput"
      >
      <div v-if="showHistory && searchHistory.length > 0" class="search-history">
        <div 
          v-for="item in searchHistory" 
          :key="item"
          class="history-item"
          @click="selectHistory(item)"
        >
          {{ item }}
        </div>
      </div>
    </div>
    
    <div class="results">
      <div v-if="loading" class="loading">
        加载中...
      </div>
      
      <div v-else-if="results.length === 0 && searchQuery" class="empty-state">
        未找到匹配的文件
      </div>
      
      <div v-else-if="results.length === 0 && !searchQuery" class="empty-state">
        输入关键词开始搜索
      </div>
      
      <div 
        v-for="file in results" 
        :key="file.id"
        class="result-item"
        @click="openFile(file)"
        @contextmenu="showContextMenu($event, file)"
      >
        <div class="result-icon">
          {{ getFileIcon(file.file_type, file.is_dir) }}
        </div>
        <div class="result-info">
          <div class="result-name">{{ file.filename }}</div>
          <div class="result-path">{{ file.repo_name }}/{{ file.path }}</div>
          <div class="result-meta">
            <span class="meta-item">{{ getFileTypeLabel(file.file_type) }}</span>
            <span class="meta-item">{{ formatSize(file.size) }}</span>
            <span class="meta-item">r{{ file.revision }}</span>
            <span class="meta-item">{{ formatTime(file.last_modified) }}</span>
          </div>
        </div>
      </div>
    </div>
    
    <div class="status-bar">
      <span>{{ indexStatus }}</span>
      <span style="margin-left: 20px;">{{ fileCount }} 文件</span>
      <button @click="refreshAll" class="refresh-btn" title="刷新所有仓库">🔄</button>
      <button @click="showRepoSettings" class="settings-btn">⚙️</button>
    </div>
    
    <div v-if="showSettingsPanel" class="settings-panel" @click.self="closeSettings">
      <div class="settings-content" @click.stop>
        <h3>设置</h3>
        
        <div class="setting-section">
          <h4>数据库设置</h4>
          <div class="form-group">
            <label>数据库路径</label>
            <div class="input-with-button">
              <input type="text" v-model="config.db_path" readonly>
              <button @click="changeDbPath" class="btn btn-small">更改</button>
            </div>
          </div>
        </div>

        <div class="setting-section">
          <h4>自动索引</h4>
          <div class="setting-item">
            <label>启用定时索引</label>
            <input type="checkbox" v-model="config.auto_index_enabled" @change="saveConfig">
          </div>
          <div class="form-group" v-if="config.auto_index_enabled">
            <label>索引间隔（分钟）</label>
            <select v-model="config.auto_index_interval" @change="saveConfig">
              <option :value="15">15分钟</option>
              <option :value="30">30分钟</option>
              <option :value="60">1小时</option>
              <option :value="120">2小时</option>
              <option :value="360">6小时</option>
              <option :value="720">12小时</option>
              <option :value="1440">24小时</option>
            </select>
          </div>
        </div>
        
        <div class="setting-section">
          <h4>SVN 仓库</h4>
          <div class="repo-list">
            <div v-if="repositories.length === 0" class="empty-state">
              暂无仓库，请添加
            </div>
            <div 
              v-for="repo in repositories" 
              :key="repo.id"
              class="repo-item"
            >
              <div class="repo-info">
                <div class="repo-name">{{ repo.name }}</div>
                <div class="repo-url">{{ repo.url }}</div>
                <div class="repo-meta">
                  <span v-if="repo.last_revision">版本: r{{ repo.last_revision }}</span>
                  <span v-if="repo.last_update"> | 更新: {{ repo.last_update }}</span>
                  <span> | 文件: {{ repoFileCounts[repo.id] || 0 }}</span>
                </div>
              </div>
              <div class="repo-actions">
                <button @click="indexRepoIncremental(repo)" class="btn btn-small" title="增量索引">🔄</button>
                <button @click="indexRepoFull(repo)" class="btn btn-small" title="完全索引">📥</button>
                <button @click="editRepo(repo)" class="btn btn-small">✏️</button>
                <button @click="deleteRepo(repo)" class="btn btn-small btn-danger">🗑️</button>
              </div>
            </div>
          </div>
          <button @click="showAddRepo" class="btn btn-primary">添加仓库</button>
        </div>
        
        <div class="setting-item">
          <label>开机自启动</label>
          <input type="checkbox" v-model="settings.autostart" @change="toggleAutostart">
        </div>
        
        <div class="setting-item">
          <label>主题</label>
          <select v-model="settings.theme" @change="changeTheme">
            <option value="dark">深色</option>
            <option value="light">浅色</option>
          </select>
        </div>
        
        <div class="setting-actions">
          <button @click="closeSettings" class="btn btn-secondary">关闭</button>
        </div>
      </div>
    </div>
    
    <div v-if="showAddRepoPanel" class="settings-panel" @click.self="closeAddRepo">
      <div class="settings-content" @click.stop>
        <h3>{{ editingRepo ? '编辑仓库' : '添加仓库' }}</h3>
        
        <div class="form-group">
          <label>名称</label>
          <input type="text" v-model="repoForm.name" placeholder="仓库名称">
        </div>
        
        <div class="form-group">
          <label>URL</label>
          <input type="text" v-model="repoForm.url" placeholder="svn://example.com/repo">
        </div>
        
        <div class="form-group">
          <label>用户名 (可选)</label>
          <input type="text" v-model="repoForm.username" placeholder="用户名">
        </div>
        
        <div class="form-group">
          <label>密码 (可选)</label>
          <input type="password" v-model="repoForm.password" placeholder="密码">
        </div>
        
        <div v-if="indexingStatus" class="indexing-status">
          {{ indexingStatus }}
        </div>
        
        <div class="setting-actions">
          <button @click="closeAddRepo" class="btn btn-secondary">取消</button>
          <button @click="saveRepo" class="btn btn-primary">{{ editingRepo ? '保存' : '添加' }}</button>
        </div>
      </div>
    </div>
    
    <div 
      v-if="contextMenu.show" 
      class="context-menu"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      @click="closeContextMenu"
    >
      <div class="context-item" @click="copyPath">复制路径</div>
      <div class="context-item" @click="copyUrl">复制 SVN URL</div>
      <div class="context-item" @click="copyFullUrl">复制完整 URL</div>
      <div class="context-item" @click="openFolder">在浏览器中打开</div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { writeText, readText } from '@tauri-apps/api/clipboard'

const searchQuery = ref('')
const results = ref([])
const loading = ref(false)
const indexStatus = ref('索引中...')
const fileCount = ref(0)
const searchInput = ref(null)
const showHistory = ref(false)
const searchHistory = ref([])
const showSettingsPanel = ref(false)
const showAddRepoPanel = ref(false)
const repositories = ref([])
const repoFileCounts = ref({})
const editingRepo = ref(null)
const indexingStatus = ref('')
const contextMenu = ref({ show: false, x: 0, y: 0, file: null })
const settings = ref({
  autostart: false,
  theme: 'dark',
  historySize: 10
})
const config = ref({
  db_path: '',
  auto_index_enabled: false,
  auto_index_interval: 60
})
const repoForm = ref({
  name: '',
  url: '',
  username: '',
  password: ''
})

let searchTimeout = null

const getFileIcon = (fileType, isDir) => {
  if (isDir) return '📁'
  const icons = {
    'text': '📝',
    'code': '💻',
    'image': '🖼️',
    'audio': '🎵',
    'video': '🎬',
    'archive': '📦',
    'pdf': '📄',
    'document': '📃'
  }
  return icons[fileType] || '📄'
}

const getFileTypeLabel = (fileType) => {
  const labels = {
    'text': '文本',
    'code': '代码',
    'image': '图片',
    'audio': '音频',
    'video': '视频',
    'archive': '压缩包',
    'pdf': 'PDF',
    'document': '文档',
    'other': '其他'
  }
  return labels[fileType] || '其他'
}

const handleSearch = () => {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(async () => {
    if (searchQuery.value.trim()) {
      loading.value = true
      showHistory.value = false
      try {
        results.value = await invoke('search_files', { query: searchQuery.value })
        addToHistory(searchQuery.value)
      } catch (error) {
        console.error('Search error:', error)
        results.value = []
      }
      loading.value = false
    } else {
      results.value = []
    }
  }, 300)
}

const addToHistory = (query) => {
  const history = searchHistory.value.filter(item => item !== query)
  history.unshift(query)
  searchHistory.value = history.slice(0, settings.value.historySize)
  localStorage.setItem('searchHistory', JSON.stringify(searchHistory.value))
}

const selectHistory = (item) => {
  searchQuery.value = item
  showHistory.value = false
  handleSearch()
}

const showHistoryList = () => {
  showHistory.value = true
}

const openFile = async (file) => {
  const fullUrl = `${file.repo_url}/${file.path}`
  window.open(fullUrl, '_blank')
}

const copyPath = async () => {
  if (contextMenu.value.file) {
    const path = `${contextMenu.value.file.repo_name}/${contextMenu.value.file.path}`
    await writeText(path)
    closeContextMenu()
  }
}

const copyUrl = async () => {
  if (contextMenu.value.file) {
    const url = `svn://${contextMenu.value.file.repo_url}/${contextMenu.value.file.path}`
    await writeText(url)
    closeContextMenu()
  }
}

const copyFullUrl = async () => {
  if (contextMenu.value.file) {
    const url = `${contextMenu.value.file.repo_url}/${contextMenu.value.file.path}`
    await writeText(url)
    closeContextMenu()
  }
}

const openFolder = async () => {
  if (contextMenu.value.file) {
    const path = contextMenu.value.file.path
    const folderPath = path.substring(0, path.lastIndexOf('/'))
    const fullUrl = `${contextMenu.value.file.repo_url}/${folderPath}`
    window.open(fullUrl, '_blank')
    closeContextMenu()
  }
}

const showContextMenu = (event, file) => {
  event.preventDefault()
  contextMenu.value = {
    show: true,
    x: event.clientX,
    y: event.clientY,
    file
  }
}

const closeContextMenu = () => {
  contextMenu.value.show = false
}

const showRepoSettings = async () => {
  showSettingsPanel.value = true
  await loadRepositories()
  await loadConfig()
}

const closeSettings = () => {
  showSettingsPanel.value = false
}

const loadRepositories = async () => {
  try {
    repositories.value = await invoke('get_repositories')
    for (const repo of repositories.value) {
      const count = await invoke('get_index_status_by_repo', { repoId: repo.id })
      repoFileCounts.value[repo.id] = count
    }
  } catch (error) {
    console.error('Load repositories error:', error)
  }
}

const loadConfig = async () => {
  try {
    config.value = await invoke('get_config')
    if (!config.value.db_path) {
      config.value.db_path = await invoke('get_db_path')
    }
  } catch (error) {
    console.error('Load config error:', error)
  }
}

const saveConfig = async () => {
  try {
    await invoke('update_config', {
      dbPath: config.value.db_path || null,
      autoIndexEnabled: config.value.auto_index_enabled,
      autoIndexInterval: config.value.auto_index_interval
    })
  } catch (error) {
    console.error('Save config error:', error)
  }
}

const changeDbPath = async () => {
  alert('请在设置中修改数据库路径，重启后生效')
}

const showAddRepo = () => {
  editingRepo.value = null
  repoForm.value = { name: '', url: '', username: '', password: '' }
  showAddRepoPanel.value = true
}

const closeAddRepo = () => {
  showAddRepoPanel.value = false
  editingRepo.value = null
  indexingStatus.value = ''
}

const editRepo = (repo) => {
  editingRepo.value = repo
  repoForm.value = {
    name: repo.name,
    url: repo.url,
    username: repo.username || '',
    password: repo.password || ''
  }
  showAddRepoPanel.value = true
}

const saveRepo = async () => {
  if (!repoForm.value.name || !repoForm.value.url) {
    return
  }
  
  try {
    if (editingRepo.value) {
      await invoke('update_repository', {
        id: editingRepo.value.id,
        name: repoForm.value.name,
        url: repoForm.value.url,
        username: repoForm.value.username || null,
        password: repoForm.value.password || null
      })
    } else {
      await invoke('add_repository', {
        name: repoForm.value.name,
        url: repoForm.value.url,
        username: repoForm.value.username || null,
        password: repoForm.value.password || null
      })
    }
    await loadRepositories()
    closeAddRepo()
  } catch (error) {
    console.error('Save repository error:', error)
  }
}

const deleteRepo = async (repo) => {
  if (!confirm(`确定要删除仓库 "${repo.name}" 吗？`)) {
    return
  }
  
  try {
    await invoke('delete_repository', { id: repo.id })
    await loadRepositories()
  } catch (error) {
    console.error('Delete repository error:', error)
  }
}

const indexRepoFull = async (repo) => {
  indexingStatus.value = '正在全量索引...'
  try {
    const result = await invoke('index_repository', { repoId: repo.id })
    indexingStatus.value = result
    await updateStatus()
    await loadRepositories()
    setTimeout(() => {
      indexingStatus.value = ''
    }, 3000)
  } catch (error) {
    indexingStatus.value = `索引失败: ${error}`
    console.error('Index repository error:', error)
  }
}

const indexRepoIncremental = async (repo) => {
  indexingStatus.value = '正在增量索引...'
  try {
    const result = await invoke('index_repository_incremental', { repoId: repo.id })
    indexingStatus.value = result
    await updateStatus()
    await loadRepositories()
    setTimeout(() => {
      indexingStatus.value = ''
    }, 3000)
  } catch (error) {
    indexingStatus.value = `索引失败: ${error}`
    console.error('Index repository error:', error)
  }
}

const refreshAll = async () => {
  indexingStatus.value = '正在刷新所有仓库...'
  try {
    const result = await invoke('index_all_repositories')
    indexingStatus.value = result
    await updateStatus()
    await loadRepositories()
    setTimeout(() => {
      indexingStatus.value = ''
    }, 3000)
  } catch (error) {
    indexingStatus.value = `刷新失败: ${error}`
    console.error('Refresh error:', error)
  }
}

const toggleAutostart = async () => {
  try {
    await invoke('toggle_autostart', { enable: settings.value.autostart })
  } catch (error) {
    console.error('Autostart error:', error)
  }
}

const changeTheme = () => {
  document.body.setAttribute('data-theme', settings.value.theme)
  localStorage.setItem('theme', settings.value.theme)
}

const formatSize = (bytes) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}

const formatTime = (time) => {
  if (!time) return '-'
  const date = new Date(time)
  return date.toLocaleDateString('zh-CN')
}

const updateStatus = async () => {
  try {
    fileCount.value = await invoke('get_index_status')
    indexStatus.value = '就绪'
  } catch (error) {
    console.error('Status error:', error)
  }
}

const loadSettings = () => {
  const savedSettings = localStorage.getItem('settings')
  if (savedSettings) {
    Object.assign(settings.value, JSON.parse(savedSettings))
  }
  
  const savedHistory = localStorage.getItem('searchHistory')
  if (savedHistory) {
    searchHistory.value = JSON.parse(savedHistory)
  }
  
  const savedTheme = localStorage.getItem('theme') || 'dark'
  document.body.setAttribute('data-theme', savedTheme)
}

const handleKeyDown = (e) => {
  if (e.ctrlKey && e.code === 'Space') {
    e.preventDefault()
    searchInput.value?.focus()
  }
  if (e.key === 'Escape') {
    closeSettings()
    closeAddRepo()
    closeContextMenu()
  }
}

const handleClickOutside = (e) => {
  if (showSettingsPanel.value && !e.target.closest('.settings-panel')) {
    closeSettings()
  }
  if (showAddRepoPanel.value && !e.target.closest('.settings-panel')) {
    closeAddRepo()
  }
  if (contextMenu.value.show && !e.target.closest('.context-menu')) {
    closeContextMenu()
  }
}

onMounted(() => {
  loadSettings()
  updateStatus()
  window.addEventListener('keydown', handleKeyDown)
  window.addEventListener('click', handleClickOutside)
  searchInput.value?.focus()
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
  window.removeEventListener('click', handleClickOutside)
  clearTimeout(searchTimeout)
})

watch(settings, (newSettings) => {
  localStorage.setItem('settings', JSON.stringify(newSettings))
}, { deep: true })
</script>

<style scoped>
.app {
  max-width: 900px;
  margin: 0 auto;
  padding: 20px;
}

.search-box {
  position: relative;
  margin-bottom: 20px;
}

.search-input {
  width: 100%;
  padding: 15px 20px;
  font-size: 16px;
  border: 2px solid #444;
  border-radius: 10px;
  background: #2a2a2a;
  color: #fff;
  outline: none;
  transition: border-color 0.2s;
}

.search-input:focus {
  border-color: #667eea;
}

.search-input::placeholder {
  color: #888;
}

.search-history {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: #333;
  border-radius: 10px;
  margin-top: 5px;
  max-height: 300px;
  overflow-y: auto;
  z-index: 1000;
}

.history-item {
  padding: 10px 15px;
  cursor: pointer;
  transition: background 0.2s;
}

.history-item:hover {
  background: #444;
}

.results {
  max-height: calc(100vh - 120px);
  overflow-y: auto;
}

.result-item {
  padding: 12px 15px;
  border-bottom: 1px solid #333;
  cursor: pointer;
  transition: background 0.2s;
  display: flex;
  align-items: center;
  gap: 10px;
}

.result-item:hover {
  background: #3a3a3a;
}

.result-icon {
  width: 40px;
  height: 40px;
  background: #667eea;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}

.result-info {
  flex: 1;
  min-width: 0;
}

.result-name {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.result-path {
  font-size: 12px;
  color: #888;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.result-meta {
  font-size: 11px;
  color: #666;
  margin-top: 2px;
  display: flex;
  gap: 10px;
}

.meta-item {
  white-space: nowrap;
}

.empty-state {
  text-align: center;
  padding: 20px;
  color: #666;
}

.loading {
  text-align: center;
  padding: 40px;
  color: #888;
}

.status-bar {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  background: #1a1a1a;
  padding: 8px 20px;
  font-size: 12px;
  color: #666;
  border-top: 1px solid #333;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.refresh-btn, .settings-btn {
  background: none;
  border: none;
  font-size: 16px;
  cursor: pointer;
  padding: 5px;
  margin-left: 10px;
}

.settings-panel {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.settings-content {
  background: #2a2a2a;
  border-radius: 10px;
  padding: 20px;
  min-width: 500px;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
}

.settings-content h3 {
  margin-bottom: 20px;
  color: #fff;
}

.setting-section {
  margin-bottom: 20px;
}

.setting-section h4 {
  color: #fff;
  margin-bottom: 10px;
  font-size: 14px;
}

.repo-list {
  margin-bottom: 15px;
}

.repo-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px;
  background: #333;
  border-radius: 5px;
  margin-bottom: 10px;
}

.repo-info {
  flex: 1;
  min-width: 0;
}

.repo-name {
  font-weight: 500;
  color: #fff;
  margin-bottom: 4px;
}

.repo-url {
  font-size: 12px;
  color: #888;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.repo-meta {
  font-size: 11px;
  color: #666;
  margin-top: 4px;
}

.repo-actions {
  display: flex;
  gap: 5px;
}

.setting-item {
  margin-bottom: 15px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.setting-item label {
  color: #fff;
  font-size: 14px;
}

.setting-item input[type="checkbox"] {
  width: 20px;
  height: 20px;
}

.setting-item input[type="number"] {
  width: 80px;
  padding: 5px;
  border: 1px solid #444;
  border-radius: 5px;
  background: #333;
  color: #fff;
}

.setting-item select {
  width: 120px;
  padding: 5px;
  border: 1px solid #444;
  border-radius: 5px;
  background: #333;
  color: #fff;
}

.setting-actions {
  margin-top: 20px;
  text-align: right;
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 5px;
  cursor: pointer;
  font-size: 14px;
}

.btn-small {
  padding: 4px 8px;
  font-size: 12px;
}

.btn-primary {
  background: #667eea;
  color: #fff;
}

.btn-primary:hover {
  background: #5568d3;
}

.btn-secondary {
  background: #444;
  color: #fff;
}

.btn-secondary:hover {
  background: #555;
}

.btn-danger {
  background: #e74c3c;
  color: #fff;
}

.btn-danger:hover {
  background: #c0392b;
}

.form-group {
  margin-bottom: 15px;
}

.form-group label {
  display: block;
  color: #fff;
  font-size: 14px;
  margin-bottom: 5px;
}

.form-group input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #444;
  border-radius: 5px;
  background: #333;
  color: #fff;
  font-size: 14px;
}

.form-group input:focus {
  outline: none;
  border-color: #667eea;
}

.form-group select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #444;
  border-radius: 5px;
  background: #333;
  color: #fff;
  font-size: 14px;
}

.input-with-button {
  display: flex;
  gap: 10px;
}

.input-with-button input {
  flex: 1;
}

.indexing-status {
  padding: 10px;
  background: #333;
  border-radius: 5px;
  color: #667eea;
  font-size: 14px;
  margin-bottom: 15px;
}

.context-menu {
  position: fixed;
  background: #2a2a2a;
  border: 1px solid #444;
  border-radius: 5px;
  min-width: 180px;
  z-index: 3000;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.3);
}

.context-item {
  padding: 10px 15px;
  cursor: pointer;
  color: #fff;
  transition: background 0.2s;
}

.context-item:hover {
  background: #444;
}

[data-theme="light"] .search-input,
[data-theme="light"] .settings-content {
  background: #fff;
  color: #333;
}

[data-theme="light"] .search-input {
  border-color: #ddd;
}

[data-theme="light"] .search-input:focus {
  border-color: #667eea;
}

[data-theme="light"] .search-input::placeholder {
  color: #999;
}

[data-theme="light"] .result-item:hover {
  background: #f0f0f0;
}

[data-theme="light"] .settings-content {
  background: #fff;
}

[data-theme="light"] .settings-content h3,
[data-theme="light"] .setting-item label,
[data-theme="light"] .setting-section h4,
[data-theme="light"] .repo-name {
  color: #333;
}

[data-theme="light"] .setting-item input,
[data-theme="light"] .setting-item select,
[data-theme="light"] .form-group input,
[data-theme="light"] .form-group select,
[data-theme="light"] .repo-item {
  background: #f5f5f5;
  border-color: #ddd;
  color: #333;
}

[data-theme="light"] .context-menu {
  background: #fff;
  border-color: #ddd;
}

[data-theme="light"] .context-item {
  color: #333;
}

[data-theme="light"] .context-item:hover {
  background: #f0f0f0;
}

[data-theme="light"] .search-history,
[data-theme="light"] .repo-item {
  background: #f5f5f5;
}

[data-theme="light"] .history-item:hover,
[data-theme="light"] .repo-item:hover {
  background: #e0e0e0;
}
</style>

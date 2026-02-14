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
          {{ file.is_dir ? '📁' : '📄' }}
        </div>
        <div class="result-info">
          <div class="result-name">{{ file.filename }}</div>
          <div class="result-path">{{ file.repo_name }}/{{ file.path }}</div>
          <div class="result-meta">
            {{ formatSize(file.size) }} | {{ formatTime(file.last_modified) }}
          </div>
        </div>
      </div>
    </div>
    
    <div class="status-bar">
      <span>{{ indexStatus }}</span>
      <span style="margin-left: 20px;">{{ fileCount }} 文件</span>
      <button @click="showSettings" class="settings-btn">⚙️</button>
    </div>
    
    <div v-if="showSettingsPanel" class="settings-panel" @click.self="closeSettings">
      <div class="settings-content" @click.stop>
        <h3>设置</h3>
        
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
        
        <div class="setting-item">
          <label>搜索历史数量</label>
          <input type="number" v-model="settings.historySize" min="0" max="50">
        </div>
        
        <div class="setting-actions">
          <button @click="closeSettings" class="btn btn-secondary">关闭</button>
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
      <div class="context-item" @click="copyUrl">复制 URL</div>
      <div class="context-item" @click="openFolder">打开文件夹</div>
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
const contextMenu = ref({ show: false, x: 0, y: 0, file: null })
const settings = ref({
  autostart: false,
  theme: 'dark',
  historySize: 10
})

let searchTimeout = null

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

const showSettings = () => {
  showSettingsPanel.value = true
}

const closeSettings = () => {
  showSettingsPanel.value = false
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
  return date.toLocaleString('zh-CN')
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
    closeContextMenu()
  }
}

const handleClickOutside = (e) => {
  if (showSettingsPanel.value && !e.target.closest('.settings-panel')) {
    closeSettings()
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
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
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

.settings-btn {
  background: none;
  border: none;
  font-size: 16px;
  cursor: pointer;
  padding: 5px;
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
  min-width: 300px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
}

.settings-content h3 {
  margin-bottom: 20px;
  color: #fff;
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
}

.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 5px;
  cursor: pointer;
  font-size: 14px;
}

.btn-secondary {
  background: #444;
  color: #fff;
}

.btn-secondary:hover {
  background: #555;
}

.context-menu {
  position: fixed;
  background: #2a2a2a;
  border: 1px solid #444;
  border-radius: 5px;
  min-width: 150px;
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
[data-theme="light"] .setting-item label {
  color: #333;
}

[data-theme="light"] .setting-item input,
[data-theme="light"] .setting-item select {
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
</style>

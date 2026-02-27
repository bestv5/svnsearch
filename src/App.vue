<template>
  <div class="app">
    <!-- 首页：搜索文件 -->
    <div v-if="currentView === 'search'" class="view-search">
      <!-- 搜索区域 -->
      <div class="search-panel">
        <input 
          v-model="searchQuery" 
          type="text" 
          class="search-input"
          :placeholder="profiles.length > 0 ? '输入文件名或路径片段搜索（跨仓库）...' : '请先在“设置”中添加仓库并建立索引，然后回来搜索'"
          :disabled="isLoading || isSearching"
          @input="handleSearch"
        />
        <button class="btn-settings" type="button" title="设置" @click="goToSettings">⚙️</button>
      </div>
      
      <!-- 结果列表 -->
      <div class="results-container">
        <div v-if="filteredFiles.length === 0 && searchQuery" class="empty-state">
          未找到匹配的文件
        </div>
        
        <div 
          v-for="(file, index) in filteredFiles" 
          :key="file.url + '/' + file.path"
          class="result-item"
          @click="copyPath(file)"
          @contextmenu.prevent="openContextMenu($event, file)"
        >
          <div class="file-icon">📄</div>
          <div class="file-info">
            <div class="file-name">{{ getFileName(file.path) }}</div>
            <div class="file-path">{{ file.path }}</div>
            <div class="file-repo">仓库：{{ file.title }}</div>
          </div>
        </div>

        <!-- 右键菜单 -->
        <div
          v-if="contextMenu.visible"
          class="context-menu"
          :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
        >
          <div class="context-menu-item" @click="handleCopyFromMenu">
            复制完整路径和文件名
          </div>
        </div>
      </div>
      
      <!-- 状态栏 -->
      <div class="status-bar">
        <span v-if="errorMessage" class="error">{{ errorMessage }}</span>
        <span v-else-if="isLoading">正在更新索引...（约 {{ progress }}% ）</span>
        <div v-if="isLoading" class="progress-bar">
          <div class="progress-inner" :style="{ width: progress + '%' }"></div>
        </div>
        <span v-else-if="isSearching">正在搜索...</span>
        <span v-else-if="searchQuery">找到 {{ filteredFiles.length }} 个结果</span>
        <span v-else>就绪</span>
      </div>
    </div>

    <!-- 二级页：SVN 地址管理 -->
    <div v-else class="view-settings">
      <div class="settings-header">
        <button class="btn-link" type="button" @click="goBackToSearch">← 返回搜索</button>
        <span class="settings-title">SVN 仓库配置</span>
      </div>

      <!-- SVN 命令路径配置（独立区域） -->
      <div class="svn-path-panel">
        <div class="form-group">
          <label>SVN 命令路径</label>
          <input
            v-model="svnPath"
            type="text"
            :placeholder="svnPathAutoDetected ? `自动检测：${svnPathAutoDetected}` : '例如：/usr/bin/svn 或 C:\\\\Program Files\\\\Subversion\\\\bin\\\\svn.exe'"
            :disabled="isLoading"
          />
          <div class="hint-text" v-if="svnPathAutoDetected && svnPath !== svnPathAutoDetected">
            自动检测到：{{ svnPathAutoDetected }}
          </div>
        </div>
      </div>

      <div class="config-panel">
        <div class="config-main">
          <div class="config-form">
            <div class="form-group">
              <label>仓库标题（可选）</label>
              <input
                v-model="repoTitle"
                type="text"
                placeholder="例如：研发主干 / 组件库"
                :disabled="isLoading"
              />
            </div>

            <div class="form-group">
              <label>SVN 仓库地址</label>
              <input 
                v-model="svnUrl" 
                type="text" 
                placeholder="https://svn.example.com/repo/trunk"
                :disabled="isLoading"
              />
            </div>
            
            <div class="form-row">
              <div class="form-group">
                <label>用户名（可选）</label>
                <input 
                  v-model="username" 
                  type="text" 
                  placeholder="用户名"
                  :disabled="isLoading"
                />
              </div>
              <div class="form-group">
                <label>密码（可选）</label>
                <input 
                  v-model="password" 
                  type="password" 
                  placeholder="密码"
                  :disabled="isLoading"
                />
              </div>
            </div>
            
            <div class="form-actions">
              <button 
                class="btn-primary" 
                @click="indexRepository"
                :disabled="!svnUrl || isLoading"
              >
                {{ isLoading ? '正在获取文件列表...' : '更新索引' }}
              </button>
              
              <button 
                class="btn-secondary"
                type="button"
                @click="clearIndex"
                :disabled="isLoading || fileCount === 0"
              >
                清空索引
              </button>

              <button 
                class="btn-secondary"
                type="button"
                @click="saveCurrentAsProfile"
                :disabled="isLoading"
              >
                保存为配置
              </button>
              
              <span v-if="fileCount > 0" class="file-count">
                已索引 {{ fileCount }} 个文件
              </span>
            </div>
          </div>

          <div class="config-profiles">
            <div class="profiles-header">
              <span class="profiles-title">已保存配置</span>
              <span class="profiles-count" v-if="profiles.length">共 {{ profiles.length }} 个</span>
              <span class="profiles-empty" v-else>暂无配置</span>
            </div>
            <div class="profiles-list" v-if="profiles.length">
              <div 
                v-for="profile in profiles" 
                :key="profile.id" 
                class="profile-item"
                :class="{ active: profile.id === selectedProfileId }"
              >
                <div class="profile-info" @click="useProfile(profile)">
                  <div class="profile-name">{{ profile.name }}</div>
                  <div class="profile-url">{{ profile.url }}</div>
                </div>
                <button 
                  class="profile-delete"
                  type="button"
                  @click.stop="deleteProfile(profile.id)"
                >
                  ✕
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="status-bar">
        <span v-if="errorMessage" class="error">{{ errorMessage }}</span>
        <span v-else-if="isLoading">正在获取文件列表...（约 {{ progress }}% ）</span>
        <div v-if="isLoading" class="progress-bar">
          <div class="progress-inner" :style="{ width: progress + '%' }"></div>
        </div>
        <span v-else-if="fileCount > 0">已索引 {{ fileCount }} 个文件</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'

// 状态
const svnUrl = ref('')
const username = ref('')
const password = ref('')
const isLoading = ref(false)
const isSearching = ref(false)
const errorMessage = ref('')
const fileCount = ref(0)
const searchQuery = ref('')
const progress = ref(0)

// 页面视图：search / settings
const currentView = ref('search')

// 文件列表
const filteredFiles = ref([])

// 配置管理
const STORAGE_KEY = 'svnsearch_svn_profiles'
const STORAGE_SVN_PATH_KEY = 'svnsearch_svn_path'
const profiles = ref([])
const selectedProfileId = ref(null)
let unlistenOpenSettings = null
let progressTimer = null

// SVN 路径配置
const svnPath = ref('')
const svnPathAutoDetected = ref('')

// 仓库标题（保存在 profile 中）
const repoTitle = ref('')

let searchDebounceTimer = null
let latestSearchToken = 0

const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  file: null
})

function loadProfiles() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) {
      profiles.value = []
      return
    }
    const parsed = JSON.parse(raw)
    if (Array.isArray(parsed)) {
      profiles.value = parsed
      if (!selectedProfileId.value && profiles.value.length > 0) {
        selectedProfileId.value = profiles.value[0].id
      }
    } else {
      profiles.value = []
    }
  } catch (e) {
    console.error('加载配置失败', e)
    profiles.value = []
  }
}

function persistProfiles() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(profiles.value))
  } catch (e) {
    console.error('保存配置失败', e)
  }
}

function loadSvnPath() {
  try {
    const raw = localStorage.getItem(STORAGE_SVN_PATH_KEY)
    if (raw) {
      svnPath.value = raw
    }
  } catch (e) {
    console.error('加载 SVN 路径失败', e)
  }
}

function persistSvnPath() {
  try {
    if (svnPath.value) {
      localStorage.setItem(STORAGE_SVN_PATH_KEY, svnPath.value)
    } else {
      localStorage.removeItem(STORAGE_SVN_PATH_KEY)
    }
  } catch (e) {
    console.error('保存 SVN 路径失败', e)
  }
}

onMounted(() => {
  loadProfiles()
  loadSvnPath()
  // 回显选中配置到表单
  if (profiles.value.length > 0) {
    const selected =
      profiles.value.find((p) => p.id === selectedProfileId.value) || profiles.value[0]
    if (selected) {
      useProfile(selected)
    }
  }

  // 自动检测 SVN 路径，用于默认回显
  invoke('detect_svn_path')
    .then((path) => {
      if (typeof path === 'string' && path) {
        svnPathAutoDetected.value = path
        if (!svnPath.value) {
          svnPath.value = path
        }
      }
    })
    .catch(() => {
      // ignore
    })

  listen('open-settings', () => {
    currentView.value = 'settings'
  }).then((unlisten) => {
    unlistenOpenSettings = unlisten
  }).catch(() => {
    // ignore
  })

  window.addEventListener('click', handleGlobalClick)
  window.addEventListener('contextmenu', handleGlobalContextMenu)
})

onUnmounted(() => {
  if (unlistenOpenSettings) {
    unlistenOpenSettings()
  }
  if (progressTimer) {
    clearInterval(progressTimer)
    progressTimer = null
  }

  window.removeEventListener('click', handleGlobalClick)
  window.removeEventListener('contextmenu', handleGlobalContextMenu)
})

watch(
  profiles,
  () => {
    persistProfiles()
  },
  { deep: true }
)

watch(svnPath, () => {
  persistSvnPath()
})

function useProfile(profile) {
  selectedProfileId.value = profile.id
  repoTitle.value = profile.title || ''
  svnUrl.value = profile.url || ''
  username.value = profile.username || ''
  password.value = profile.password || ''
}

function saveCurrentAsProfile() {
  if (!svnUrl.value) {
    errorMessage.value = '请先填写 SVN 仓库地址，再保存为配置'
    return
  }

  // 自动生成配置名称：优先用地址+用户名，避免依赖弹窗
  const autoName = username.value
    ? `${svnUrl.value} (${username.value})`
    : svnUrl.value

  // 先按当前选中的配置 id 查找，不存在则按 url+username 匹配
  let existingIndex = profiles.value.findIndex((p) => p.id === selectedProfileId.value)
  if (existingIndex === -1) {
    existingIndex = profiles.value.findIndex(
      (p) => p.url === svnUrl.value && p.username === username.value
    )
  }

  const profileData = {
    id: existingIndex >= 0 ? profiles.value[existingIndex].id : Date.now().toString(),
    name: autoName,
    title: repoTitle.value || '',
    url: svnUrl.value,
    username: username.value,
    password: password.value
  }

  if (existingIndex >= 0) {
    profiles.value.splice(existingIndex, 1, profileData)
  } else {
    profiles.value.push(profileData)
  }

  selectedProfileId.value = profileData.id
  errorMessage.value = ''
}

function deleteProfile(profileId) {
  const idx = profiles.value.findIndex((p) => p.id === profileId)
  if (idx === -1) return
  profiles.value.splice(idx, 1)
  if (selectedProfileId.value === profileId) {
    selectedProfileId.value = profiles.value[0]?.id || null
    const next = profiles.value[0]
    if (next) {
      useProfile(next)
    }
  }
}

function goBackToSearch() {
  currentView.value = 'search'
}

function goToSettings() {
  currentView.value = 'settings'
}

// 清空索引结果
async function clearIndex() {
  if (!svnUrl.value) return

  isLoading.value = true
  progress.value = 0
  errorMessage.value = ''

  try {
    await invoke('clear_index', { url: svnUrl.value })
    fileCount.value = 0
    // 清完当前仓库索引后，若正在搜索则刷新一次结果（跨仓库）
    if (searchQuery.value.trim()) {
      await performSearch()
    } else {
      filteredFiles.value = []
    }
  } catch (error) {
    errorMessage.value = error.toString()
  } finally {
    isLoading.value = false
    progress.value = 100
  }
}

// 索引仓库
async function indexRepository() {
  if (!svnUrl.value) return
  
  isLoading.value = true
  progress.value = 0
  errorMessage.value = ''
  filteredFiles.value = []
  fileCount.value = 0

  if (progressTimer) {
    clearInterval(progressTimer)
    progressTimer = null
  }

  // 伪进度条：请求过程中缓慢增加到 90%
  progressTimer = setInterval(() => {
    if (!isLoading.value) {
      clearInterval(progressTimer)
      progressTimer = null
      return
    }
    if (progress.value < 90) {
      progress.value += 1
    }
  }, 200)
  
  try {
    const files = await invoke('fetch_svn_files', {
      url: svnUrl.value,
      username: username.value || null,
      password: password.value || null,
      svnPath: svnPath.value || null
    })
    
    await invoke('save_index', { url: svnUrl.value, files })
    fileCount.value = files.length
    // 更新索引后，如果用户当前有关键词，则立刻刷新搜索结果
    if (searchQuery.value.trim()) {
      await performSearch()
    }
  } catch (error) {
    errorMessage.value = error.toString()
  } finally {
    isLoading.value = false
    progress.value = 100
    if (progressTimer) {
      clearInterval(progressTimer)
      progressTimer = null
    }
  }
}

// 搜索处理
function handleSearch() {
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
    searchDebounceTimer = null
  }
  searchDebounceTimer = setTimeout(() => {
    performSearch()
  }, 200)
}

async function performSearch() {
  const query = searchQuery.value.trim()
  if (!query) {
    filteredFiles.value = []
    errorMessage.value = ''
    isSearching.value = false
    return
  }

  const token = ++latestSearchToken
  isSearching.value = true
  errorMessage.value = ''

  try {
    const entries = await invoke('search_index', { query, limit: 200 })
    if (token !== latestSearchToken) return

    const urlToProfile = new Map((profiles.value || []).map((p) => [p.url, p]))
    filteredFiles.value = (entries || []).map((e) => {
      const p = urlToProfile.get(e.url)
      const title = p?.title || p?.name || e.url
      return { url: e.url, path: e.path, title }
    })
  } catch (error) {
    if (token !== latestSearchToken) return
    errorMessage.value = error.toString()
    filteredFiles.value = []
  } finally {
    if (token === latestSearchToken) {
      isSearching.value = false
    }
  }
}

// 获取文件名
function getFileName(filePath) {
  const parts = filePath.split('/')
  return parts[parts.length - 1] || filePath
}

function getFullSvnUrl(entry) {
  const base = (entry.url || '').replace(/\/+$/, '')
  const path = (entry.path || '').replace(/^\/+/, '')
  if (!base) return path
  if (!path) return base
  return `${base}/${path}`
}

// 复制路径
async function copyPath(entry) {
  const text = getFullSvnUrl(entry)
  try {
    await navigator.clipboard.writeText(text)
  } catch (error) {
    try {
      await invoke('copy_to_clipboard', { text })
    } catch (_) {
      // ignore
    }
  }
}

function openContextMenu(event, file) {
  contextMenu.value.visible = true
  contextMenu.value.x = event.clientX
  contextMenu.value.y = event.clientY
  contextMenu.value.file = file
}

function handleCopyFromMenu() {
  if (contextMenu.value.file) {
    copyPath(contextMenu.value.file)
  }
  contextMenu.value.visible = false
}

function handleGlobalClick() {
  if (contextMenu.value.visible) {
    contextMenu.value.visible = false
  }
}

function handleGlobalContextMenu(e) {
  // 如果右键发生在菜单外，则关闭现有菜单，保留浏览器/系统默认菜单
  if (!(e.target instanceof HTMLElement)) return
  if (!e.target.closest('.result-item') && !e.target.closest('.context-menu')) {
    contextMenu.value.visible = false
  }
}
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: #1a1a2e;
  color: #eee;
  overflow: hidden;
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  max-width: 900px;
  margin: 0 auto;
  padding: 20px;
}

.view-search,
.view-settings {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* SVN 路径区域 */
.svn-path-panel {
  background: #111827;
  padding: 16px 20px;
  border-radius: 10px;
  margin-bottom: 16px;
}

/* 配置区域 */
.config-panel {
  background: #16213e;
  padding: 20px;
  border-radius: 12px;
  margin-bottom: 20px;
}

.config-main {
  display: flex;
  gap: 20px;
}

.config-form {
  flex: 2;
}

.config-profiles {
  flex: 1.2;
  background: #0f172a;
  border-radius: 10px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  max-height: 220px;
}

.profiles-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.profiles-title {
  font-size: 13px;
  color: #9ca3af;
}

.profiles-count {
  font-size: 12px;
  color: #6ee7b7;
}

.profiles-empty {
  font-size: 12px;
  color: #4b5563;
}

.profiles-list {
  flex: 1;
  overflow-y: auto;
  margin-top: 4px;
}

.profile-item {
  display: flex;
  align-items: center;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: pointer;
  gap: 6px;
}

.profile-item:hover {
  background: #111827;
}

.profile-item.active {
  background: #1d4ed8;
}

.profile-item.active .profile-name {
  color: #e5e7eb;
}

.profile-item.active .profile-url {
  color: #bfdbfe;
}

.profile-info {
  flex: 1;
  min-width: 0;
}

.profile-name {
  font-size: 13px;
  color: #e5e7eb;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.profile-url {
  font-size: 11px;
  color: #6b7280;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.profile-delete {
  border: none;
  background: transparent;
  color: #6b7280;
  cursor: pointer;
  font-size: 12px;
  padding: 2px 4px;
}

.profile-delete:hover {
  color: #f87171;
}

.form-group {
  margin-bottom: 15px;
}

.form-group label {
  display: block;
  margin-bottom: 6px;
  font-size: 13px;
  color: #888;
}

.form-group input {
  width: 100%;
  padding: 10px 14px;
  font-size: 14px;
  border: 1px solid #333;
  border-radius: 8px;
  background: #0f0f23;
  color: #eee;
  outline: none;
  transition: border-color 0.2s;
}

.form-group input:focus {
  border-color: #667eea;
}

.form-group input:disabled {
  opacity: 0.5;
}

.hint-text {
  margin-top: 4px;
  font-size: 12px;
  color: #6b7280;
}

.form-row {
  display: flex;
  gap: 15px;
}

.form-row .form-group {
  flex: 1;
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 15px;
  margin-top: 20px;
}

.btn-secondary {
  padding: 10px 18px;
  font-size: 13px;
  border-radius: 8px;
  border: 1px solid #374151;
  background: #111827;
  color: #e5e7eb;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.btn-secondary:hover:not(:disabled) {
  background: #1f2937;
  border-color: #4b5563;
}

.btn-secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  padding: 12px 24px;
  font-size: 14px;
  font-weight: 500;
  border: none;
  border-radius: 8px;
  background: #667eea;
  color: #fff;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-primary:hover:not(:disabled) {
  background: #5a6fd6;
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.file-count {
  font-size: 13px;
  color: #4ade80;
}

/* 搜索区域 */
.search-panel {
  margin-bottom: 15px;
  position: relative;
  display: flex;
  align-items: center;
}

.search-input {
  width: 100%;
  padding: 14px 60px 14px 18px;
  font-size: 15px;
  border: 2px solid #333;
  border-radius: 10px;
  background: #16213e;
  color: #eee;
  outline: none;
  transition: border-color 0.2s;
}

.search-input:focus {
  border-color: #667eea;
}

/* 设置按钮 */
.btn-settings {
  position: absolute;
  right: 12px;
  top: 50%;
  transform: translateY(-50%);
  width: 40px;
  height: 40px;
  border: none;
  border-radius: 50%;
  background: #1f2937;
  color: #9ca3af;
  font-size: 18px;
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-settings:hover {
  background: #374151;
  color: #eee;
}

/* 结果列表 */
.results-container {
  flex: 1;
  overflow-y: auto;
  background: #16213e;
  border-radius: 12px;
}

.result-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid #252547;
  cursor: pointer;
  transition: background 0.2s;
}

.result-item:hover {
  background: #1f2b4d;
}

.file-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  background: #667eea;
  border-radius: 8px;
  flex-shrink: 0;
}

.file-info {
  flex: 1;
  min-width: 0;
}

.file-name {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 3px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-path {
  font-size: 12px;
  color: #888;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-repo {
  margin-top: 2px;
  font-size: 12px;
  color: #9ca3af;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.context-menu {
  position: fixed;
  z-index: 1000;
  min-width: 180px;
  background: #111827;
  border-radius: 8px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
  padding: 4px 0;
  border: 1px solid #374151;
}

.context-menu-item {
  padding: 8px 14px;
  font-size: 13px;
  color: #e5e7eb;
  cursor: pointer;
}

.context-menu-item:hover {
  background: #1f2937;
}

.empty-state {
  padding: 40px;
  text-align: center;
  color: #666;
}

/* 状态栏 */
.status-bar {
  margin-top: 15px;
  padding: 10px 16px;
  font-size: 13px;
  color: #666;
  background: #0f0f23;
  border-radius: 8px;
}

.status-bar .error {
  color: #f87171;
}

.progress-bar {
  margin-top: 6px;
  width: 100%;
  height: 4px;
  background: #111827;
  border-radius: 999px;
  overflow: hidden;
}

.progress-inner {
  height: 100%;
  background: linear-gradient(90deg, #4ade80, #22c55e);
  transition: width 0.2s ease-out;
}
</style>

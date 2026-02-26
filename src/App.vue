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
          :placeholder="fileCount > 0 ? '输入文件名搜索...' : '请先在“设置”中索引仓库，然后在这里搜索文件（Ctrl+Space）'"
          :disabled="fileCount === 0 || isLoading"
          @input="handleSearch"
        />
      </div>
      
      <!-- 结果列表 -->
      <div class="results-container">
        <div v-if="filteredFiles.length === 0 && searchQuery" class="empty-state">
          未找到匹配的文件
        </div>
        
        <div 
          v-for="(file, index) in filteredFiles" 
          :key="index"
          class="result-item"
          @click="copyPath(file)"
        >
          <div class="file-icon">📄</div>
          <div class="file-info">
            <div class="file-name">{{ getFileName(file) }}</div>
            <div class="file-path">{{ file }}</div>
          </div>
        </div>
      </div>
      
      <!-- 状态栏 -->
      <div class="status-bar">
        <span v-if="errorMessage" class="error">{{ errorMessage }}</span>
        <span v-else-if="isLoading">正在获取文件列表...（约 {{ progress }}% ）</span>
        <div v-if="isLoading" class="progress-bar">
          <div class="progress-inner" :style="{ width: progress + '%' }"></div>
        </div>
        <span v-else-if="searchQuery">找到 {{ filteredFiles.length }} 个结果</span>
        <span v-else-if="fileCount > 0">就绪</span>
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
const errorMessage = ref('')
const fileCount = ref(0)
const searchQuery = ref('')
const progress = ref(0)

// 页面视图：search / settings
const currentView = ref('search')

// 文件列表
const allFiles = ref([])
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
})

onUnmounted(() => {
  if (unlistenOpenSettings) {
    unlistenOpenSettings()
  }
  if (progressTimer) {
    clearInterval(progressTimer)
    progressTimer = null
  }
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

// 清空索引结果
function clearIndex() {
  allFiles.value = []
  filteredFiles.value = []
  fileCount.value = 0
  searchQuery.value = ''
  errorMessage.value = ''
  progress.value = 0
}

// 索引仓库
async function indexRepository() {
  if (!svnUrl.value) return
  
  isLoading.value = true
  progress.value = 0
  errorMessage.value = ''
  allFiles.value = []
  filteredFiles.value = []
  fileCount.value = 0
  searchQuery.value = ''

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
    
    allFiles.value = files
    fileCount.value = files.length
    filteredFiles.value = files.slice(0, 100) // 初始显示前100个
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
  const query = searchQuery.value.toLowerCase().trim()
  
  if (!query) {
    filteredFiles.value = allFiles.value.slice(0, 100)
    return
  }
  
  // 不区分大小写的子字符串匹配
  filteredFiles.value = allFiles.value
    .filter(file => file.toLowerCase().includes(query))
    .slice(0, 200) // 限制显示数量
}

// 获取文件名
function getFileName(filePath) {
  const parts = filePath.split('/')
  return parts[parts.length - 1] || filePath
}

// 复制路径
async function copyPath(file) {
  try {
    await invoke('copy_to_clipboard', { text: file })
  } catch (error) {
    // 回退到原生方法
    navigator.clipboard.writeText(file)
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
}

.search-input {
  width: 100%;
  padding: 14px 18px;
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

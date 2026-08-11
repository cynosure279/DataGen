<template>
  <n-card title="生成结果" size="small">
    <n-tabs v-if="store.results?.files.length" type="card" v-model:value="activeTab">
      <n-tab-pane v-for="f in store.results.files" :key="f.filename" :name="f.filename" :tab="f.filename">
        <n-input type="textarea" :value="f.content" readonly :autosize="{ minRows: 6, maxRows: 20 }" />
      </n-tab-pane>
    </n-tabs>
    <n-empty v-else description="生成数据后将在此处显示" />
    <n-alert v-if="store.status === 'error'" type="error" title="生成失败">{{ store.errorMessage }}</n-alert>
    <n-space v-if="store.status === 'done'" justify="end" style="margin-top: 8px;">
      <n-button @click="store.reset()">清除</n-button>
      <n-button type="primary" @click="saveFiles">保存文件</n-button>
    </n-space>
  </n-card>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useGenerationStore } from '@/stores/generation'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useMessage, NCard, NTabs, NTabPane, NInput, NEmpty, NAlert, NSpace, NButton } from 'naive-ui'
const store = useGenerationStore()
const message = useMessage()
const activeTab = ref('')
async function saveFiles() {
  if (!store.results?.files.length) return
  try {
    const dir = await open({ directory: true, multiple: false, title: '保存目录' })
    if (!dir) return
    await invoke('save_files', { dir, filesJson: JSON.stringify(store.results.files.map(f => ({filename:f.filename,content:f.content}))) })
    message.success('已保存 ' + store.results.files.length + ' 个文件')
  } catch(e) { message.error('保存失败: ' + String(e)) }
}
</script>
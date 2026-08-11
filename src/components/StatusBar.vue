<template>
  <n-space align="center" size="small">
    <n-tag :type="compilerStore.detecting ? 'warning' : compilerStore.compilers.length > 0 ? 'success' : 'error'" size="small">
      {{ compilerStatusText }}
    </n-tag>
    <n-tag :type="genTagType" size="small">
      {{ genStatusText }}
    </n-tag>
  </n-space>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useCompilerStore } from '@/stores/compilers'
import { useGenerationStore } from '@/stores/generation'
import { NSpace, NTag } from 'naive-ui'

const compilerStore = useCompilerStore()
const generationStore = useGenerationStore()

onMounted(() => { compilerStore.detectCompilers() })

const compilerStatusText = computed(() => {
  if (compilerStore.detecting) return '编译器: 检测中...'
  if (compilerStore.compilers.length > 0) return `编译器: ${compilerStore.compilers.length} 个已检测`
  return `编译器: 未检测 (${compilerStore.missing.length} 缺失)`
})

const genTagType = computed(() => {
  switch (generationStore.status) {
    case 'generating': case 'compiling': case 'running': return 'warning'
    case 'done': return 'success'
    case 'error': return 'error'
    default: return 'default'
  }
})

const genStatusText = computed(() => {
  switch (generationStore.status) {
    case 'idle': return '生成: 空闲'
    case 'generating': return '生成: 进行中...'
    case 'compiling': return '编译: 进行中...'
    case 'running': return '运行: 进行中...'
    case 'done': return `生成: 完成 (${generationStore.results?.files.length || 0} 文件)`
    case 'error': return '生成: 失败'
    default: return '生成: 空闲'
  }
})
</script>
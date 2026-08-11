<template>
  <n-card title="编译器" size="small">
    <n-space vertical size="small">
      <n-form-item label="选择编译器">
        <n-select v-model:value="store.selectedCompiler" :options="compilerOpts" placeholder="请先检测编译器"/>
      </n-form-item>
      <n-form-item label="编译参数"><n-input v-model:value="store.compileArgs" placeholder="-O2 -std=c++17"/></n-form-item>
      <n-space>
        <n-button :loading="store.detecting" @click="store.detectCompilers()">检测编译器</n-button>
        <n-button type="primary" @click="handleCompile">编译并运行</n-button>
      </n-space>
      <n-alert v-if="store.missing.length" type="warning" title="缺失编译器">
        未找到: {{ store.missing.join(', ') }}。请安装后再检测。
      </n-alert>
    </n-space>
  </n-card>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useCompilerStore } from '@/stores/compilers'
import { useGenerationStore } from '@/stores/generation'
import { NCard, NSpace, NFormItem, NSelect, NInput, NButton, NAlert } from 'naive-ui'

const store = useCompilerStore()
const genStore = useGenerationStore()

const compilerOpts = computed(() => store.compilers.map(c => ({ label: `${c.name} (${c.version})`, value: c.name })))

async function handleCompile() {
  if (!store.selectedCompiler) return
  // TODO: wire to custom gen editor source
  genStore.status = 'compiling'
}
</script>
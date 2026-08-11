<template>
  <n-card title="自定义生成器" size="small">
    <n-space vertical size="small">
      <n-form-item label="语言">
        <n-select
          v-model:value="language"
          :options="[
            { label: 'Python', value: 'python' },
            { label: 'C++', value: 'cpp' },
          ]"
        />
      </n-form-item>
      <n-form-item label="代码">
        <n-input
          v-model:value="code"
          type="textarea"
          :autosize="{ minRows: 6, maxRows: 16 }"
          placeholder="在此编写自定义生成器代码..."
        />
      </n-form-item>
      <n-button type="primary" @click="useCustomGen">
        <template #icon>
          <n-icon size="16">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
              <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
            </svg>
          </n-icon>
        </template>
        使用自定义生成器
      </n-button>
    </n-space>
  </n-card>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useMessage } from 'naive-ui'
import {
  NCard,
  NSpace,
  NFormItem,
  NSelect,
  NInput,
  NButton,
  NIcon,
} from 'naive-ui'

const message = useMessage()

const language = ref<'python' | 'cpp'>('python')
const code = ref('')

function useCustomGen() {
  if (!code.value.trim()) {
    message.warning('请先编写生成器代码')
    return
  }
  message.success(`自定义 ${language.value} 生成器已提交`)
}
</script>
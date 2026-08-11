import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { GenResult } from '@/types'
import { invokeGenerate, invokeGenerateAndRun } from '@/api/tauri'
import { useConfigStore } from '@/stores/config'

type GenStatus = 'idle' | 'generating' | 'compiling' | 'running' | 'done' | 'error'

export const useGenerationStore = defineStore('generation', () => {
  const status = ref<GenStatus>('idle')
  const results = ref<GenResult | null>(null)
  const errorMessage = ref('')

  async function generate() {
    const config = useConfigStore().toTestConfig()
    status.value = 'generating'
    errorMessage.value = ''
    try {
      results.value = await invokeGenerate(config)
      status.value = 'done'
    } catch (e) {
      errorMessage.value = String(e)
      status.value = 'error'
    }
  }

  async function generateAndRun(source: string, language: string) {
    const config = useConfigStore().toTestConfig()
    status.value = 'compiling'
    errorMessage.value = ''
    try {
      const r: any = await invokeGenerateAndRun(config, source, language)
      results.value = r.generation
      status.value = 'done'
    } catch (e) {
      errorMessage.value = String(e)
      status.value = 'error'
    }
  }

  function reset() {
    status.value = 'idle'
    results.value = null
    errorMessage.value = ''
  }

  return { status, results, errorMessage, generate, generateAndRun, reset }
})
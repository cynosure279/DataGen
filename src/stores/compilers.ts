import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { CompilerInfo } from '@/types'
import { invokeDetectCompilers } from '@/api/tauri'

export const useCompilerStore = defineStore('compiler', () => {
  const compilers = ref<CompilerInfo[]>([])
  const missing = ref<string[]>([])
  const selectedCompiler = ref<string>('')
  const compileArgs = ref<string>('-O2 -std=c++17')
  const detecting = ref(false)

  async function detectCompilers() {
    detecting.value = true
    try {
      const result = await invokeDetectCompilers()
      compilers.value = result.found
      missing.value = result.missing
      if (compilers.value.length > 0 && !selectedCompiler.value) {
        selectedCompiler.value = compilers.value[0].name
      }
    } catch (e) {
      console.error('Compiler detection failed:', e)
    } finally {
      detecting.value = false
    }
  }

  function selectCompiler(name: string) {
    selectedCompiler.value = name
  }

  return {
    compilers,
    missing,
    selectedCompiler,
    compileArgs,
    detecting,
    detectCompilers,
    selectCompiler,
  }
})
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { FieldDef, TestCaseMode, TestConfig } from '@/types'

export const useConfigStore = defineStore('config', () => {
  const activeConfig = ref<TestConfig>({
    files_count: 10,
    prefix: 'test',
    suffix: '',
    testcase_mode: 'Disabled',
    fields: [],
  })

  function addField(field: FieldDef) {
    activeConfig.value.fields.push(field)
  }
  function removeField(index: number) {
    activeConfig.value.fields.splice(index, 1)
  }
  function updateField(index: number, field: Partial<FieldDef>) {
    const target = activeConfig.value.fields[index]
    if (target) Object.assign(target, field)
  }
  function setTestCaseMode(mode: TestCaseMode) {
    activeConfig.value.testcase_mode = mode
  }
  function resetConfig() {
    activeConfig.value = { files_count: 10, prefix: 'test', suffix: '', testcase_mode: 'Disabled', fields: [] }
  }
  function toTestConfig(): TestConfig {
    return JSON.parse(JSON.stringify(activeConfig.value))
  }

  return { activeConfig, addField, removeField, updateField, setTestCaseMode, resetConfig, toTestConfig }
}, { persist: true })
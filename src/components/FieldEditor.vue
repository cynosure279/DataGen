<template>
  <n-card size="small" :title="localField.name || '(未命名)'">
    <template #header-extra><n-button size="tiny" quaternary circle type="error" @click="$emit('remove')">✕</n-button></template>
    <n-space vertical size="small">
      <n-space>
        <n-input v-model:value="localField.name" placeholder="字段名" style="flex:1"/>
        <n-select v-model:value="localField.data_type" :options="typeOpts" style="flex:1"/>
        <n-select v-model:value="localField.distribution" :options="distOpts" style="flex:1"/>
      </n-space>

      <!-- Dependency mode: only CountFrom (array dependency) -->
      <div class="form-field">
        <label class="form-label">数组依赖（可选）</label>
        <n-select v-model:value="depMode" :options="depOpts" @update:value="onDepChange"/>
      </div>
      <template v-if="depMode === 'CountFrom'">
        <div class="form-field">
          <label class="form-label">个数来源字段</label>
          <n-select v-model:value="countFromField" :options="fieldOpts" @update:value="onCountFromChange" />
        </div>
        <div class="form-field">
          <label class="form-label">元素值表达式</label>
          <ExpressionEditor v-model="localElemValue" :allFields="allFields" :selfIndex="selfIndex" />
        </div>
      </template>

      <!-- Static bounds: min/max expression editors -->
      <template v-if="depMode !== 'CountFrom'">
        <div class="form-field">
          <label class="form-label">最小值表达式</label>
          <ExpressionEditor v-model="localMin" :allFields="allFields" :selfIndex="selfIndex" />
        </div>
        <div class="form-field">
          <label class="form-label">最大值表达式</label>
          <ExpressionEditor v-model="localMax" :allFields="allFields" :selfIndex="selfIndex" />
        </div>
      </template>

      <div class="form-field">
        <label class="form-label">分隔符</label>
        <n-select v-model:value="localField.separator" :options="[{label:'空格 (同行)',value:'Space'},{label:'换行',value:'Newline'}]" @update:value="() => emitUpdate()"/>
      </div>
    </n-space>
  </n-card>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import type { FieldDef, ValueExpr } from '@/types'
import { NCard, NSpace, NInput, NSelect, NButton } from 'naive-ui'
import ExpressionEditor from './ExpressionEditor.vue'

const props = defineProps<{ modelValue: FieldDef; selfIndex?: number; allFields?: FieldDef[] }>()
const emit = defineEmits<{ (e:'update:modelValue',v:FieldDef): void; (e:'remove'): void }>()

const typeOpts = ['Int32','Int64','Float32','Float64','Char','String'].map(v=>({label:v,value:v}))
const distOpts = ['Uniform','Normal','Exponential','Poisson','Binomial','Geometric','LogNormal','Cauchy'].map(v=>({label:v,value:v}))

// Simplified dep mode: only 'none' or 'CountFrom'
const depMode = ref<'none'|'CountFrom'>('none')
const countFromField = ref('')
const localElemValue = ref<ValueExpr>({ type: 'const', value: 1 })

// Min/max expression state
const localMin = ref<ValueExpr>({ type: 'const', value: 0 })
const localMax = ref<ValueExpr>({ type: 'const', value: 100 })

const localField = reactive<FieldDef>({ ...props.modelValue })

// Field options for selectors (all other named fields)
const fieldOpts = computed(() => {
  const all = props.allFields || []
  return all
    .filter((f, j) => j !== (props.selfIndex ?? -1) && f.name)
    .map(f => ({ label: f.name, value: f.name }))
})

// Simplified dep options: only CountFrom
const depOpts = computed(() => {
  const names = fieldOpts.value.map(o => o.value)
  if (!names.length) {
    return [{ label: '无依赖 (需先给其他字段命名)', value: 'none' }]
  }
  return [
    { label: '无依赖', value: 'none' },
    ...names.map(n => ({ label: `CountFrom: 个数 = ${n}`, value: `cf:${n}` })),
  ]
})

// Initialize from modelValue
function initFromField(field: FieldDef) {
  Object.assign(localField, field)
  depMode.value = 'none'
  localMin.value = { type: 'const', value: 0 }
  localMax.value = { type: 'const', value: 100 }
  localElemValue.value = { type: 'const', value: 1 }
  countFromField.value = ''

  if (field.range.type === 'count_from') {
    depMode.value = 'CountFrom'
    countFromField.value = field.range.from_field
    localElemValue.value = field.range.elem_value
  } else {
    // Static range
    localMin.value = field.range.min
    localMax.value = field.range.max
  }
}

function onDepChange(val: string) {
  if (val === 'none') {
    depMode.value = 'none'
    emitUpdate()
  } else if (val.startsWith('cf:')) {
    const parent = val.slice(3)
    depMode.value = 'CountFrom'
    countFromField.value = parent
    localField.range = { type: 'count_from', from_field: parent, elem_value: localElemValue.value }
    emitUpdate()
  }
}

function onCountFromChange() {
  if (depMode.value !== 'CountFrom') return
  localField.range = { type: 'count_from', from_field: countFromField.value, elem_value: localElemValue.value }
  emitUpdate()
}

function emitUpdate() {
  if (depMode.value === 'CountFrom') {
    localField.range = { type: 'count_from', from_field: countFromField.value, elem_value: localElemValue.value }
  } else {
    localField.range = { type: 'static', min: localMin.value, max: localMax.value }
  }
  emit('update:modelValue', { ...localField })
}

// Watch modelValue changes from parent
watch(() => props.modelValue, v => initFromField(v), { immediate: true, deep: true })

// Watch field name changes
watch(() => localField.name, () => emitUpdate())
</script>

<style scoped>
.form-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}
.form-label {
  font-size: 12px;
  color: var(--n-text-color-3, #888);
  font-weight: 500;
}
</style>
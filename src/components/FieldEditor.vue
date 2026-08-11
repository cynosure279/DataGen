<template>
  <n-card size="small" :title="localField.name || '(未命名)'">
    <template #header-extra><n-button size="tiny" quaternary circle type="error" @click="$emit('remove')">✕</n-button></template>
    <n-space vertical size="small">
      <n-space>
        <n-input v-model:value="localField.name" placeholder="字段名" style="flex:1"/>
        <n-select v-model:value="localField.data_type" :options="typeOpts" style="flex:1"/>
        <n-select v-model:value="localField.distribution" :options="distOpts" style="flex:1"/>
      </n-space>
      <n-form-item label="依赖字段（可选）">
        <n-select v-model:value="depMode" :options="depOpts" @update:value="onDepChange"/>
      </n-form-item>
      <n-form-item label="分隔符">
        <n-select v-model:value="localField.separator" :options="[{label:'空格 (同行)',value:'Space'},{label:'换行',value:'Newline'}]" @update:value="() => emitUpdate()"/>
      </n-form-item>
      <template v-if="depMode === 'CountFrom'">
        <n-space>
          <n-input-number v-model:value="depElemMin" placeholder="元素最小值"/>
          <n-input-number v-model:value="depElemMax" placeholder="元素最大值"/>
        </n-space>
      </template>
      <template v-else-if="depMode === 'ValueFrom'">
        <n-input-number v-model:value="depMultiplier" :min="0.1" :step="0.1" placeholder="倍数 (1.0 = 不超过父字段)"/>
      </template>
      <template v-else>
        <n-space><n-input-number v-model:value="flatRange.min" placeholder="Min"/><n-input-number v-model:value="flatRange.max" placeholder="Max"/></n-space>
      </template>
    </n-space>
  </n-card>
</template>

<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import type { FieldDef, RangeValue } from '@/types'
import { NCard, NSpace, NFormItem, NInput, NInputNumber, NSelect, NButton } from 'naive-ui'

const props = defineProps<{ modelValue: FieldDef; selfIndex?: number; allFields?: FieldDef[] }>()
const emit = defineEmits<{ (e:'update:modelValue',v:FieldDef): void; (e:'remove'): void }>()

const typeOpts = ['Int32','Int64','Float32','Float64','Char','String'].map(v=>({label:v,value:v}))
const distOpts = ['Uniform','Normal','Exponential','Poisson'].map(v=>({label:v,value:v}))

const depMode = ref<'none'|'CountFrom'|'ValueFrom'>('none')
const depElemMin = ref(1); const depElemMax = ref(1000); const depMultiplier = ref(1.0)

function extractRange(rv: RangeValue): { min: number; max: number } {
  if ('Int32' in rv) return rv.Int32
  if ('Int64' in rv) return rv.Int64
  if ('Float32' in rv) return rv.Float32
  if ('Float64' in rv) return rv.Float64
  if ('Char' in rv) return rv.Char
  if ('StringLen' in rv) return rv.StringLen
  return { min: 0, max: 100 }
}
function wrapRange(dt: string, min: number, max: number): RangeValue {
  const r = { min, max }
  switch (dt) {
    case 'Int32': return { Int32: r }
    case 'Int64': return { Int64: r }
    case 'Float32': return { Float32: r }
    case 'Float64': return { Float64: r }
    case 'Char': return { Char: r }
    default: return { StringLen: r }
  }
}

const depOpts = ref<{label:string;value:string}[]>([{label:'无依赖 (需先给其他字段命名)',value:'none'}]);

function refreshDepOpts() {
  const all = props.allFields || []
  const parentNames = all
    .filter((f, j) => j !== (props.selfIndex ?? -1) && f.name)
    .map(f => f.name)
  if (!parentNames.length) {
    depOpts.value = [{ label: '无依赖 (需先给其他字段命名)', value: 'none' }]
  } else {
    depOpts.value = [
      { label: '无依赖', value: 'none' },
      ...parentNames.map(n => ({ label: `CountFrom: 个数 = ${n}`, value: `cf:${n}` })),
      ...parentNames.map(n => ({ label: `ValueFrom: max = ${n} × 倍数`, value: `vf:${n}` })),
    ]
  }
}

watch(() => props.allFields?.map(f => f.name).join(','), () => refreshDepOpts(), { immediate: true })

function onDepChange(val: string) {
  if (val === 'none') {
    localField.depends_on = undefined
    depMode.value = 'none'
  } else if (val.startsWith('cf:')) {
    const parent = val.slice(3)
    localField.depends_on = parent
    depMode.value = 'CountFrom'
    localField.range = { CountFrom: { from_field: parent, elem_min: depElemMin.value, elem_max: depElemMax.value } }
  } else if (val.startsWith('vf:')) {
    const parent = val.slice(3)
    localField.depends_on = parent
    depMode.value = 'ValueFrom'
    localField.range = { ValueFrom: { from_field: parent, multiplier: depMultiplier.value } }
  }
  emitUpdate()
}

const localField = reactive<FieldDef>({ ...props.modelValue })
const flatRange = reactive(extractRange(props.modelValue.range))

watch(() => props.modelValue, v => { Object.assign(localField, v); Object.assign(flatRange, extractRange(v.range)) })
watch(() => localField.name, () => emitUpdate())
watch([flatRange, () => localField.data_type], () => {
  if (depMode.value === 'none') {
    localField.range = wrapRange(localField.data_type, flatRange.min, flatRange.max)
  }
  emitUpdate()
}, { deep: true })

watch([depElemMin, depElemMax], () => {
  if (depMode.value === 'CountFrom' && localField.depends_on) {
    localField.range = { CountFrom: { from_field: localField.depends_on, elem_min: depElemMin.value, elem_max: depElemMax.value } }
    emitUpdate()
  }
})

watch(depMultiplier, () => {
  if (depMode.value === 'ValueFrom' && localField.depends_on) {
    localField.range = { ValueFrom: { from_field: localField.depends_on, multiplier: depMultiplier.value } }
    emitUpdate()
  }
})

function emitUpdate() { emit('update:modelValue', { ...localField }) }
</script>
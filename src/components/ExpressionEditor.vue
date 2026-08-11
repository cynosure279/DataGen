<template>
  <n-space vertical size="small" style="width:100%">
    <n-select v-model:value="mode" :options="modeOpts" @update:value="onModeChange" />

    <!-- Const mode -->
    <n-input-number v-if="mode === 'const'" v-model:value="constVal" placeholder="常量值" @update:value="emitUpdate" />

    <!-- FromField mode -->
    <n-select v-else-if="mode === 'from_field'" v-model:value="fromFieldName" :options="fieldOpts" placeholder="选择依赖字段" @update:value="emitUpdate" />

    <!-- Random mode -->
    <template v-else-if="mode === 'random'">
      <n-select v-model:value="randomDist" :options="distOpts" @update:value="emitUpdate" />
      <div class="expr-field">
        <label class="expr-label">下限表达式</label>
        <ExpressionEditor v-model="localLo" :allFields="allFields" :selfIndex="selfIndex" />
      </div>
      <div class="expr-field">
        <label class="expr-label">上限表达式</label>
        <ExpressionEditor v-model="localHi" :allFields="allFields" :selfIndex="selfIndex" />
      </div>
    </template>

    <!-- Op mode -->
    <template v-else-if="mode === 'op'">
      <n-space>
        <n-select v-model:value="opField" :options="fieldOpts" placeholder="选择字段" style="flex:1" @update:value="emitUpdate" />
        <n-select v-model:value="opOperator" :options="opOpts" style="flex:1" @update:value="emitUpdate" />
        <n-input-number v-model:value="opOperand" placeholder="操作数" style="flex:1" @update:value="emitUpdate" />
      </n-space>
    </template>
  </n-space>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { Distribution, FieldDef, Op, ValueExpr } from '@/types'
import { NSpace, NInputNumber, NSelect } from 'naive-ui'

defineOptions({ name: 'ExpressionEditor' })

const props = defineProps<{
  modelValue: ValueExpr
  allFields?: FieldDef[]
  selfIndex?: number
}>()
const emit = defineEmits<{ (e: 'update:modelValue', v: ValueExpr): void }>()

const modeOpts = [
  { label: '常量', value: 'const' },
  { label: '依赖字段', value: 'from_field' },
  { label: '随机', value: 'random' },
  { label: '运算', value: 'op' },
]

const distOpts = ['Uniform', 'Normal', 'Exponential', 'Poisson', 'Binomial', 'Geometric', 'LogNormal', 'Cauchy']
  .map(v => ({ label: v, value: v }))

const opOpts = [
  { label: '× 乘', value: 'Mul' },
  { label: '+ 加', value: 'Add' },
  { label: '- 减', value: 'Sub' },
  { label: 'Min 最小值', value: 'Min' },
  { label: 'Max 最大值', value: 'Max' },
]

const fieldOpts = computed(() => {
  const all = props.allFields || []
  return all
    .filter((f, j) => j !== (props.selfIndex ?? -1) && f.name)
    .map(f => ({ label: f.name, value: f.name }))
})

// Guard flag to prevent infinite update loops
let _isUpdating = false

// Internal state
const mode = ref<'const' | 'from_field' | 'random' | 'op'>('const')
const constVal = ref(0)
const fromFieldName = ref('')
const randomDist = ref<string>('Uniform')
const localLo = ref<ValueExpr>({ type: 'const', value: 0 })
const localHi = ref<ValueExpr>({ type: 'const', value: 100 })
const opField = ref('')
const opOperator = ref<Op>('Mul')
const opOperand = ref(1)

function initFromExpr(expr: ValueExpr) {
  switch (expr.type) {
    case 'const':
      mode.value = 'const'
      constVal.value = expr.value
      break
    case 'from_field':
      mode.value = 'from_field'
      fromFieldName.value = expr.name
      break
    case 'random':
      mode.value = 'random'
      randomDist.value = expr.distribution
      localLo.value = expr.lo
      localHi.value = expr.hi
      break
    case 'op':
      mode.value = 'op'
      opField.value = expr.field
      opOperator.value = expr.operator
      opOperand.value = expr.operand
      break
  }
}

function buildExpr(): ValueExpr {
  switch (mode.value) {
    case 'const':
      return { type: 'const', value: constVal.value }
    case 'from_field':
      return { type: 'from_field', name: fromFieldName.value }
    case 'random':
      return { type: 'random', distribution: randomDist.value as Distribution, lo: localLo.value, hi: localHi.value }
    case 'op':
      return { type: 'op', field: opField.value, operator: opOperator.value, operand: opOperand.value }
  }
}

function onModeChange() {
  // Reset sub-values when mode changes
  emitUpdate()
}

function emitUpdate() {
  _isUpdating = true
  emit('update:modelValue', buildExpr())
  _isUpdating = false
}

watch(() => props.modelValue, v => {
  if (_isUpdating) return
  initFromExpr(v)
}, { immediate: true, deep: true })
</script>

<style scoped>
.expr-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}
.expr-label {
  font-size: 12px;
  color: var(--n-text-color-3, #888);
  font-weight: 500;
}
</style>
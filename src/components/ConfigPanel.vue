<template>
  <n-card title="生成配置" size="small">
    <n-space vertical size="small">
      <n-form-item label="文件数量"><n-input-number v-model:value="config.files_count" :min="1" :max="1000" style="width:100%"/></n-form-item>
      <n-form-item label="文件名前缀"><n-input v-model:value="config.prefix" placeholder="test"/></n-form-item>
      <n-form-item label="文件名后缀"><n-input v-model:value="config.suffix" placeholder="(可选)"/></n-form-item>
      <n-form-item label="多测试用例"><n-switch :value="tcEnabled" @update:value="toggleTC"/></n-form-item>
      <template v-if="tcEnabled">
        <n-select v-model:value="tcMode" :options="[{label:'固定数量',value:'fixed'},{label:'随机范围',value:'random'}]"/>
        <template v-if="tcMode==='fixed'"><n-input-number v-model:value="tcFixed" :min="1" :max="1000"/></template>
        <template v-else><n-space><n-input-number v-model:value="tcMin" :min="1" :max="1000" placeholder="Min"/><n-input-number v-model:value="tcMax" :min="1" :max="1000" placeholder="Max"/></n-space></template>
      </template>
      <n-divider/>
      <n-space justify="space-between"><b>字段定义</b><n-button size="tiny" @click="addField">+ 添加字段</n-button></n-space>
      <FieldEditor v-for="(f,i) in config.fields" :key="i" :model-value="f" :self-index="i" :all-fields="config.fields" @update:model-value="updateField(i,$event)" @remove="removeField(i)"/>
      <n-empty v-if="!config.fields.length" description="暂无字段"/>
      <n-button type="primary" block :disabled="!config.fields.length" :loading="genStore.status==='generating'" @click="genStore.generate()">开始生成</n-button>
    </n-space>
  </n-card>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useConfigStore } from '@/stores/config'
import { useGenerationStore } from '@/stores/generation'
import { NCard,NSpace,NFormItem,NInput,NInputNumber,NSelect,NSwitch,NButton,NDivider,NEmpty } from 'naive-ui'
import FieldEditor from '@/components/FieldEditor.vue'

const configStore = useConfigStore()
const genStore = useGenerationStore()
const config = computed(() => configStore.activeConfig)

const tcMode = ref('fixed'); const tcFixed = ref(1); const tcMin = ref(1); const tcMax = ref(10)
const tcEnabled = computed({
  get: () => config.value.testcase_mode !== 'Disabled',
  set: (v: boolean) => { if (v) configStore.setTestCaseMode({ Fixed: tcFixed.value }); else configStore.setTestCaseMode('Disabled') }
})
function toggleTC(v: boolean) { tcEnabled.value = v }

function addField() {
  configStore.addField({ name:'', data_type:'Int32', distribution:'Uniform', range:{ type:'static', min:{ type:'const', value:1 }, max:{ type:'const', value:100 } } })
}
function removeField(i: number) { configStore.removeField(i) }
function updateField(i: number, f: any) { configStore.updateField(i, f) }
</script>

<template>
  <n-grid :cols="2" :x-gap="12" style="height:100%;">
    <n-grid-item><ConfigPanel /></n-grid-item>
    <n-grid-item style="overflow-y:auto;">
      <n-space vertical size="small">
        <ResultView />
        <n-card title="评测" size="small">
          <n-space vertical size="small">
            <n-form-item label="语言">
              <n-select v-model:value="language" :options="[{label:'C++',value:'cpp'},{label:'Python',value:'python'}]" />
            </n-form-item>
            <n-form-item label="解答代码">
              <n-input v-model:value="solutionCode" type="textarea" :autosize="{minRows:6,maxRows:14}" placeholder="在此粘贴 ans.cpp 或 ans.py..." />
            </n-form-item>
            <n-form-item label="编译器">
              <n-select v-model:value="compilerStore.selectedCompiler" :options="compilerOpts" placeholder="请先检测" />
            </n-form-item>
            <n-form-item label="编译参数">
              <n-input v-model:value="compilerStore.compileArgs" placeholder="-O2 -std=c++17" />
            </n-form-item>
            <n-space>
              <n-button :loading="compilerStore.detecting" @click="compilerStore.detectCompilers()">检测编译器</n-button>
              <n-button type="primary" :loading="judging" :disabled="!canJudge" @click="runJudge">生成数据 & 评测</n-button>
            </n-space>
          </n-space>
        </n-card>
      </n-space>
    </n-grid-item>
  </n-grid>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NGrid, NGridItem, NSpace, NCard, NFormItem, NSelect, NInput, NButton } from 'naive-ui'
import { useCompilerStore } from '@/stores/compilers'
import { useGenerationStore } from '@/stores/generation'
import { useConfigStore } from '@/stores/config'
import { invokeGenerate, invokeCompile, invokeRun } from '@/api/tauri'
import ConfigPanel from '@/components/ConfigPanel.vue'
import ResultView from '@/components/ResultView.vue'

const compilerStore = useCompilerStore()
const genStore = useGenerationStore()
const configStore = useConfigStore()

const language = ref<'cpp'|'python'>('cpp')
const solutionCode = ref('')
const judging = ref(false)

const compilerOpts = computed(() => compilerStore.compilers.map(c => ({label:`${c.name} (${c.version})`,value:c.name})))

const canJudge = computed(() =>
  solutionCode.value.trim() && compilerStore.selectedCompiler && configStore.activeConfig.fields.length > 0
)

async function runJudge() {
  judging.value = true
  genStore.errorMessage = ''
  try {
    // 1. Generate test data
    genStore.status = 'generating'
    const config = configStore.toTestConfig()
    const genResult = await invokeGenerate(config)

    // 2. Compile solution
    genStore.status = 'compiling'
    const compileResult = await invokeCompile(
      solutionCode.value, language.value,
      compilerStore.selectedCompiler,
      compilerStore.compileArgs.split(' ').filter(Boolean)
    )
    if (!compileResult.success || !compileResult.binary_path) {
      genStore.errorMessage = '编译失败: ' + compileResult.stderr
      genStore.status = 'error'
      return
    }

    // 3. Run against all generated inputs
    genStore.status = 'running'
    const outputs: { input: string; output: string; stderr: string }[] = []
    for (const file of genResult.files) {
      const exec = await invokeRun(compileResult.binary_path, file.content, 5)
      outputs.push({ input: file.filename, output: exec.stdout, stderr: exec.stderr })
    }

    // 4. Build result
    genStore.results = {
      files: outputs.map((o, i) => ({
        filename: genResult.files[i].filename.replace('.in', '.out'),
        content: o.output,
      })),
      metadata: genResult.metadata,
    }
    genStore.status = 'done'
  } catch (e: any) {
    genStore.errorMessage = String(e)
    genStore.status = 'error'
  } finally {
    judging.value = false
  }
}
</script>
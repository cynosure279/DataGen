<template>
  <n-modal :show="show" @update:show="(v: boolean) => emit('update:show', v)" preset="card" title="欢迎使用 DataGen" style="width:640px;">
    <n-steps :current="step + 1" style="margin-bottom:24px;">
      <n-step title="编译器" description="检测系统编译器" />
      <n-step title="配置" description="设置生成参数" />
      <n-step title="评测" description="粘贴代码 & 运行" />
    </n-steps>

    <div v-if="step===0">
      <p>DataGen 需要系统编译器来编译和运行你的代码。</p>
      <p>点击页面中 <b>"检测编译器"</b> 按钮，自动检测 gcc/g++/clang/python3。</p>
      <n-alert type="info">Linux 通常已预装。macOS 需 Xcode CLT。Windows 需 MinGW。</n-alert>
    </div>

    <div v-if="step===1">
      <p>在左侧 <b>生成配置</b> 面板中：</p>
      <ol><li>设置 <b>文件数量</b></li><li>可选开启 <b>多测试用例</b></li><li>添加 <b>字段</b>：类型 + 分布 + 范围</li></ol>
      <p>点击 <b>"开始生成"</b> 即可生成测试数据。</p>
    </div>

    <div v-if="step===2">
      <p>在右侧 <b>评测</b> 面板中：</p>
      <ol><li>选择语言（C++ 或 Python）</li><li>粘贴你的解答代码（ans.cpp / ans.py）</li><li>选择编译器 + 编译参数</li><li>点击 <b>"生成数据 & 评测"</b></li></ol>
      <n-alert type="success">DataGen 将：生成输入 → 编译代码 → 运行 → 展示输出！</n-alert>
    </div>

    <template #footer>
      <n-space justify="space-between" style="width:100%;">
        <n-button @click="emit('update:show', false)">跳过</n-button>
        <n-space>
          <n-button v-if="step>0" @click="step--">上一步</n-button>
          <n-button v-if="step<2" type="primary" @click="step++">下一步</n-button>
          <n-button v-else type="primary" @click="emit('update:show', false)">开始使用</n-button>
        </n-space>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NModal, NSteps, NStep, NButton, NSpace, NAlert } from 'naive-ui'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ (e:'update:show',v:boolean): void }>()

const step = ref(0)
watch(() => props.show, (v) => { if (v) step.value = 0 })
</script>
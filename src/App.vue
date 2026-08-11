<template>
  <n-config-provider :locale="zhCN" :theme="isDark ? darkTheme : null">
    <n-message-provider>
      <n-global-style />
      <div class="app-shell" :class="{ dark: isDark }">
        <header class="app-header">
          <span class="app-title">DataGen</span>
          <n-space>
            <n-button quaternary size="small" @click="showOnboarding = true">新手引导</n-button>
            <n-button quaternary circle @click="isDark = !isDark">{{ isDark ? '☀️' : '🌙' }}</n-button>
          </n-space>
        </header>
        <main class="app-main"><router-view /></main>
        <footer class="app-footer"><StatusBar /></footer>
      </div>
      <Onboarding v-model:show="showOnboarding" />
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { zhCN, darkTheme, NGlobalStyle } from 'naive-ui'
import { NConfigProvider, NMessageProvider, NButton, NSpace } from 'naive-ui'
import StatusBar from '@/components/StatusBar.vue'
import Onboarding from '@/components/Onboarding.vue'

const isDark = ref(false)
const showOnboarding = ref(true)
</script>

<style>
html, body, #app { margin: 0; padding: 0; height: 100%; }
.app-shell { display: flex; flex-direction: column; height: 100vh; background: #fff; color: #333; }
.app-shell.dark { background: #1a1a2e; color: #e0e0e0; }
.app-header { display: flex; align-items: center; justify-content: space-between; padding: 0 16px; height: 48px; border-bottom: 1px solid #e8e8e8; flex-shrink: 0; }
.app-shell.dark .app-header { border-bottom-color: #2a2a3e; }
.app-title { font-size: 18px; font-weight: 700; }
.app-main { flex: 1; overflow-y: auto; padding: 16px; }
.app-footer { height: 40px; display: flex; align-items: center; padding: 0 16px; border-top: 1px solid #e8e8e8; flex-shrink: 0; }
.app-shell.dark .app-footer { border-top-color: #2a2a3e; }
</style>
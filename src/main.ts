import { createApp } from 'vue'
import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'
import App from './App.vue'
import router from './router'

// Clear stale persisted state from old type formats (v1→v2 migration)
try {
  const raw = localStorage.getItem('config')
  if (raw) {
    const parsed = JSON.parse(raw)
    if (!parsed.activeConfig?.fields?.every?.((f: any) => f.range?.type)) {
      localStorage.removeItem('config')
    }
  }
} catch { localStorage.removeItem('config') }

const app = createApp(App)

const pinia = createPinia()
pinia.use(piniaPluginPersistedstate)

app.use(pinia)
app.use(router)
app.mount('#app')
import { createRouter, createWebHashHistory } from 'vue-router'
import DataGen from '@/views/DataGen.vue'

const routes = [
  {
    path: '/',
    name: 'DataGen',
    component: DataGen,
  },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

export default router
# DataGen

> ACM 赛题测试数据生成器 & 评测工具  
> ACM Test Data Generator & Judge Tool

[![CI](https://github.com/<owner>/<repo>/actions/workflows/ci.yml/badge.svg)](https://github.com/<owner>/<repo>/actions/workflows/ci.yml)

---

## 技术栈 / Tech Stack

| 层级 / Layer       | 技术 / Technology                          |
| ------------------ | ------------------------------------------ |
| 前端 / Frontend    | Vue 3.5 + Naive UI 2.41 + Pinia 2.3       |
| 路由 / Router      | Vue Router 4.4                             |
| 构建 / Build       | Vite 6 + vue-tsc                           |
| 后端引擎 / Engine  | Rust (5-crate workspace)                   |
| 数据生成 / Gen     | Python (subprocess)                        |
| 配置 / Config      | TOML                                       |
| 包管理 / PM        | pnpm 9                                     |
| 测试 / Test        | Vitest                                     |
| 代码质量 / Lint    | ESLint + Prettier + TypeScript 5.6 (strict)|

---

## 开发命令 / Dev Commands

```bash
# 安装依赖 / Install dependencies
pnpm install

# 启动开发服务器 / Start dev server
pnpm dev

# 构建前端 / Build frontend
pnpm build

# 预览构建产物 / Preview build
pnpm preview

# Tauri CLI 命令 / Tauri commands
pnpm tauri dev
pnpm tauri build

# 代码检查 / Lint
pnpm lint

# 运行测试 / Run tests
pnpm test
```

---

## 项目结构 / Project Structure

```
datagen/
├── src/                  # Vue 3 前端源码
├── src-tauri/            # Tauri Rust 后端 (5-crate workspace)
├── public/               # 静态资源
├── package.json
├── tsconfig.json
├── AGENTS.md             # AI agent 系统拓扑
└── README.md
```


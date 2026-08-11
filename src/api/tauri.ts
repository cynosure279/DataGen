import { invoke } from '@tauri-apps/api/core'
import type {
  TestConfig,
  GenResult,
  CompileResult,
  ExecResult,
  DetectionResult,
} from '@/types'

export async function invokeGenerate(config: TestConfig): Promise<GenResult> {
  const json = await invoke<string>('generate_data', { configJson: JSON.stringify(config) })
  return JSON.parse(json)
}

export async function invokeCompile(
  source: string,
  language: string,
  compiler: string,
  args: string[],
): Promise<CompileResult> {
  const json = await invoke<string>('compile_code', { source, language, compiler, args })
  return JSON.parse(json)
}

export async function invokeRun(
  binaryPath: string,
  input: string,
  timeout: number,
): Promise<ExecResult> {
  const json = await invoke<string>('run_binary', { binaryPath, input, timeout })
  return JSON.parse(json)
}

export async function invokeDetectCompilers(): Promise<DetectionResult> {
  const json = await invoke<string>('detect_compilers')
  return JSON.parse(json)
}

export async function invokeGenerateAndRun(
  config: TestConfig,
  source: string,
  language: string,
): Promise<unknown> {
  const json = await invoke<string>('generate_and_run', {
    configJson: JSON.stringify(config),
    source,
    language,
  })
  return JSON.parse(json)
}
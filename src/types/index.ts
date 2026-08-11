export type DataType = 'Int32' | 'Int64' | 'BigInt' | 'Float32' | 'Float64' | 'Char' | 'String'
export type Distribution = 'Uniform' | 'Normal' | 'Exponential' | 'Poisson'

export interface Range { min: number; max: number }

export type RangeValue =
  | { Int32: Range } | { Int64: Range } | { Float32: Range } | { Float64: Range }
  | { Char: Range } | { StringLen: Range }
  | { CountFrom: { from_field: string; elem_min: number; elem_max: number } }
  | { ValueFrom: { from_field: string; multiplier: number } }

export type FieldSeparator = 'Space' | 'Newline'

export interface FieldDef {
  name: string
  data_type: DataType
  distribution: Distribution
  range: RangeValue
  depends_on?: string
  separator?: FieldSeparator
}

export type TestCaseMode = 'Disabled' | { Fixed: number } | { Random: { distribution: Distribution; range: Range } }

export interface TestConfig {
  files_count: number
  prefix: string
  suffix: string
  testcase_mode: TestCaseMode
  fields: FieldDef[]
  seed?: number
}

export interface GenFile { filename: string; content: string }
export interface GenMetadata { seed: number; generated_at: string; config_hash: string }
export interface GenResult { files: GenFile[]; metadata: GenMetadata }

export interface CompilerInfo { name: string; path: string; version: string }
export interface DetectionResult { found: CompilerInfo[]; missing: string[] }

export interface CompileResult {
  success: boolean
  binary_path: string | null
  stderr: string
  exit_code: number | null
}

export interface ExecResult {
  stdout: string
  stderr: string
  exit_code: number | null
  timed_out: boolean
  killed: boolean
}
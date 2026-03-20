/* tslint:disable */
/* eslint-disable */

export function all_notes_off(id: number): void;

export function apply_params_from_buf(id: number): void;

export function create_engine(id: number, sample_rate: number): void;

/**
 * Diagnostic: return the length of a stored wavetable.
 */
export function debug_wavetable_len(id: number, exc_idx: number, body_idx: number): number;

/**
 * Diagnostic: return the peak value of a stored wavetable.
 */
export function debug_wavetable_peak(id: number, exc_idx: number, body_idx: number): number;

export function destroy_engine(id: number): void;

export function get_active_voice_count(id: number): number;

export function get_output_len(): number;

export function get_output_ptr(id: number): number;

export function get_param_ptr(id: number): number;

/**
 * Returns pointer to engine #id's wavetable upload buffer.
 * JS writes convolved data here, then calls store_wavetable.
 */
export function get_wavetable_ptr(id: number): number;

export function note_off(id: number, note: number): void;

export function note_on(id: number, note: number, velocity: number): void;

export function render(id: number): void;

/**
 * Store the uploaded wavetable into the engine's cache at (exc, body) index.
 * JS calls this after writing data to the wavetable buffer.
 */
export function store_wavetable(id: number, exc_idx: number, body_idx: number, len: number): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly all_notes_off: (a: number) => void;
    readonly apply_params_from_buf: (a: number) => void;
    readonly create_engine: (a: number, b: number) => void;
    readonly debug_wavetable_len: (a: number, b: number, c: number) => number;
    readonly debug_wavetable_peak: (a: number, b: number, c: number) => number;
    readonly destroy_engine: (a: number) => void;
    readonly get_active_voice_count: (a: number) => number;
    readonly get_output_len: () => number;
    readonly get_output_ptr: (a: number) => number;
    readonly get_param_ptr: (a: number) => number;
    readonly get_wavetable_ptr: (a: number) => number;
    readonly note_off: (a: number, b: number) => void;
    readonly note_on: (a: number, b: number, c: number) => void;
    readonly render: (a: number) => void;
    readonly store_wavetable: (a: number, b: number, c: number, d: number) => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

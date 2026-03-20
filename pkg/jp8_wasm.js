/* @ts-self-types="./jp8_wasm.d.ts" */

/**
 * @param {number} id
 */
export function all_notes_off(id) {
    wasm.all_notes_off(id);
}

/**
 * @param {number} id
 */
export function apply_params_from_buf(id) {
    wasm.apply_params_from_buf(id);
}

/**
 * @param {number} id
 * @param {number} sample_rate
 */
export function create_engine(id, sample_rate) {
    wasm.create_engine(id, sample_rate);
}

/**
 * @param {number} id
 */
export function destroy_engine(id) {
    wasm.destroy_engine(id);
}

/**
 * @param {number} id
 * @returns {number}
 */
export function get_active_voice_count(id) {
    const ret = wasm.get_active_voice_count(id);
    return ret >>> 0;
}

/**
 * @returns {number}
 */
export function get_output_len() {
    const ret = wasm.get_output_len();
    return ret >>> 0;
}

/**
 * @param {number} id
 * @returns {number}
 */
export function get_output_ptr(id) {
    const ret = wasm.get_output_ptr(id);
    return ret >>> 0;
}

/**
 * @param {number} id
 * @returns {number}
 */
export function get_param_ptr(id) {
    const ret = wasm.get_param_ptr(id);
    return ret >>> 0;
}

/**
 * Returns pointer to engine #id's wavetable upload buffer.
 * JS writes convolved data here, then calls store_wavetable.
 * @param {number} id
 * @returns {number}
 */
export function get_wavetable_ptr(id) {
    const ret = wasm.get_wavetable_ptr(id);
    return ret >>> 0;
}

/**
 * @param {number} id
 * @param {number} note
 */
export function note_off(id, note) {
    wasm.note_off(id, note);
}

/**
 * @param {number} id
 * @param {number} note
 * @param {number} velocity
 */
export function note_on(id, note, velocity) {
    wasm.note_on(id, note, velocity);
}

/**
 * @param {number} id
 */
export function render(id) {
    wasm.render(id);
}

/**
 * Store the uploaded wavetable into the engine's cache at (exc, body) index.
 * JS calls this after writing data to the wavetable buffer.
 * @param {number} id
 * @param {number} exc_idx
 * @param {number} body_idx
 * @param {number} len
 */
export function store_wavetable(id, exc_idx, body_idx, len) {
    wasm.store_wavetable(id, exc_idx, body_idx, len);
}

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./jp8_wasm_bg.js": import0,
    };
}

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('jp8_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };

-- CSLN LuaJIT Binding
-- This module provides a high-level Lua interface to the CSLN Rust processor.

local ffi = require("ffi")

-- Define the C interface
ffi.cdef[[
    typedef struct Processor Processor;

    Processor* csln_processor_new(const char* style_json, const char* bib_json);
    void csln_processor_free(Processor* processor);
    
    char* csln_render_citation_latex(Processor* processor, const char* cite_json);
    char* csln_render_bibliography_latex(Processor* processor);
    
    void csln_string_free(char* s);
]]

local CSLN = {}
CSLN.__index = CSLN

local function is_windows()
    return jit and jit.os == "Windows"
end

local function is_macos()
    return jit and jit.os == "OSX"
end

local function shared_lib_name()
    if is_windows() then
        return "csln_processor.dll"
    end
    if is_macos() then
        return "libcsln_processor.dylib"
    end
    return "libcsln_processor.so"
end

local function resolve_library()
    local env_path = os.getenv("CSLN_LIB_PATH")
    local lib_name = shared_lib_name()
    local candidates = {}

    if env_path and #env_path > 0 then
        table.insert(candidates, env_path)
    end

    -- Prefer release builds for normal use, with debug fallback.
    table.insert(candidates, "target/release/" .. lib_name)
    table.insert(candidates, "target/debug/" .. lib_name)
    table.insert(candidates, lib_name)

    local required_symbols = {
        "csln_processor_new",
        "csln_processor_free",
        "csln_render_citation_latex",
        "csln_render_bibliography_latex",
        "csln_string_free",
    }
    local load_errors = {}

    for _, candidate in ipairs(candidates) do
        local ok, loaded = pcall(ffi.load, candidate)
        if ok then
            local symbols_ok = true
            local missing = nil
            for _, symbol in ipairs(required_symbols) do
                local has_symbol = pcall(function()
                    return loaded[symbol]
                end)
                if not has_symbol then
                    symbols_ok = false
                    missing = symbol
                    break
                end
            end

            if symbols_ok then
                return loaded, candidate
            end
            table.insert(load_errors, candidate .. " (missing symbol: " .. tostring(missing) .. ")")
        else
            table.insert(load_errors, candidate .. " (" .. tostring(loaded) .. ")")
        end
    end

    return nil, candidates, load_errors
end

local lib, loaded_path, load_errors = resolve_library()
if lib == nil then
    error(
        "Failed to load csln_processor shared library. Tried: "
            .. table.concat(loaded_path, ", ")
            .. ". Details: "
            .. table.concat(load_errors, " | ")
            .. ". Ensure it is built with: cargo build --package csln_processor --release --features ffi"
    )
end

function CSLN.new(style_json, bib_json)
    local self = setmetatable({}, CSLN)
    self.ptr = lib.csln_processor_new(style_json, bib_json)
    if self.ptr == nil then
        return nil, "Failed to initialize CSLN processor"
    end
    self.ptr = ffi.gc(self.ptr, lib.csln_processor_free)
    self.lib_path = loaded_path
    return self
end

function CSLN:free()
    if self.ptr then
        local ptr = ffi.gc(self.ptr, nil)
        lib.csln_processor_free(ptr)
        self.ptr = nil
    end
end

function CSLN:render_citation(cite_json)
    local c_str = lib.csln_render_citation_latex(self.ptr, cite_json)
    if c_str == nil then return nil end
    
    local lua_str = ffi.string(c_str)
    lib.csln_string_free(c_str)
    return lua_str
end

function CSLN:render_bibliography()
    local c_str = lib.csln_render_bibliography_latex(self.ptr)
    if c_str == nil then return nil end
    
    local lua_str = ffi.string(c_str)
    lib.csln_string_free(c_str)
    return lua_str
end

return CSLN

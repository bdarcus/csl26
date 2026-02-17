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

-- Load the library (adjust path as needed)
local lib_path = "target/debug/libcsln_processor.dylib" -- OS-dependent extension
local lib = ffi.load(lib_path)

function CSLN.new(style_json, bib_json)
    local self = setmetatable({}, CSLN)
    self.ptr = lib.csln_processor_new(style_json, bib_json)
    if self.ptr == nil then
        return nil, "Failed to initialize CSLN processor"
    end
    return self
end

function CSLN:free()
    if self.ptr then
        lib.csln_processor_free(self.ptr)
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

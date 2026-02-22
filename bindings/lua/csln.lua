-- CSLN LuaJIT Binding
-- This module provides a high-level Lua interface to the CSLN Rust processor.

local ffi = require("ffi")

-- Define the C interface
ffi.cdef[[
    typedef struct Processor Processor;

    /* Constructors (in-memory JSON data) */
    Processor* csln_processor_new(const char* style_json, const char* bib_json);
    Processor* csln_processor_new_with_locale(const char* style_json,
                                              const char* bib_json,
                                              const char* locale_json);

    /* File-based constructors (preferred for LaTeX integration) */
    Processor* csln_processor_new_from_yaml(const char* style_yaml_path,
                                            const char* bib_yaml_path);
    Processor* csln_processor_new_from_bib(const char* style_yaml_path,
                                           const char* bib_path);

    void csln_processor_free(Processor* processor);

    /* Citation rendering */
    char* csln_render_citation_latex(Processor* processor, const char* cite_json);
    char* csln_render_citation_html(Processor* processor, const char* cite_json);
    char* csln_render_citation_plain(Processor* processor, const char* cite_json);

    /* Bibliography rendering */
    char* csln_render_bibliography_latex(Processor* processor);
    char* csln_render_bibliography_html(Processor* processor);
    char* csln_render_bibliography_plain(Processor* processor);

    void csln_string_free(char* s);
]]

local CSLN = {}
CSLN.__index = CSLN

-- ---------------------------------------------------------------------------
-- Library resolution
-- ---------------------------------------------------------------------------

--- Detect the current OS, compatible with both LuaJIT (jit.os) and
--- LuaTeX standard Lua (os.uname).
local function detect_os()
    -- LuaJIT exposes jit.os
    if jit then
        return jit.os  -- "OSX", "Windows", "Linux", etc.
    end
    -- Standard LuaTeX exposes os.uname()
    if os.uname then
        local ok, info = pcall(os.uname)
        if ok and type(info) == "table" then
            local s = info.sysname or ""
            if s == "Darwin" then return "OSX" end
            if s == "Windows_NT" or s:find("MINGW") or s:find("MSYS") then
                return "Windows"
            end
            return "Linux"
        end
    end
    -- Last resort: path separator
    if package.config:sub(1, 1) == "\\" then return "Windows" end
    return "Linux"
end

local function shared_lib_name()
    local os_name = detect_os()
    if os_name == "Windows" then return "csln_processor.dll" end
    if os_name == "OSX"     then return "libcsln_processor.dylib" end
    return "libcsln_processor.so"
end

local function resolve_library()
    local env_path = os.getenv("CSLN_LIB_PATH")
    local lib_name = shared_lib_name()
    local candidates = {}

    if env_path and #env_path > 0 then
        table.insert(candidates, env_path)
    end
    table.insert(candidates, "target/release/" .. lib_name)
    table.insert(candidates, "target/debug/"   .. lib_name)
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
            for _, sym in ipairs(required_symbols) do
                local has = pcall(function() return loaded[sym] end)
                if not has then
                    symbols_ok = false
                    missing = sym
                    break
                end
            end
            if symbols_ok then return loaded, candidate end
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

-- ---------------------------------------------------------------------------
-- Internal helpers
-- ---------------------------------------------------------------------------

--- Copy a C string returned by Rust, free it, return a Lua string.
local function consume_c_str(c_str)
    if c_str == nil then return nil end
    local s = ffi.string(c_str)
    lib.csln_string_free(c_str)
    return s
end

--- Escape a string for safe embedding inside a JSON string literal.
local function json_escape(s)
    s = tostring(s)
    s = s:gsub('\\', '\\\\')
    s = s:gsub('"',  '\\"')
    s = s:gsub('\n', '\\n')
    s = s:gsub('\r', '\\r')
    s = s:gsub('\t', '\\t')
    return s
end

--- Build a CSLN Citation JSON string from a Lua options table.
--
-- opts fields (all optional except items):
--   mode            = "integral" | "non-integral"   (default: "non-integral")
--   suppress_author = true | false
--   prefix, suffix  = strings (citation-level affix)
--   items           = list of { id, label, locator, prefix, suffix }
--
-- Single-key shorthand: if opts is a plain string it is treated as a bare key.
local function build_citation_json(opts)
    if type(opts) == "string" then
        opts = { items = { { id = opts } } }
    end

    local parts = {}

    if opts.mode and opts.mode ~= "non-integral" then
        table.insert(parts, '"mode":"' .. json_escape(opts.mode) .. '"')
    end
    if opts.suppress_author then
        table.insert(parts, '"suppress-author":true')
    end
    if opts.prefix then
        table.insert(parts, '"prefix":"' .. json_escape(opts.prefix) .. '"')
    end
    if opts.suffix then
        table.insert(parts, '"suffix":"' .. json_escape(opts.suffix) .. '"')
    end

    -- Build items array
    local item_strs = {}
    for _, item in ipairs(opts.items or {}) do
        local ip = {}
        table.insert(ip, '"id":"' .. json_escape(item.id) .. '"')
        if item.label then
            table.insert(ip, '"label":"' .. json_escape(item.label) .. '"')
        end
        if item.locator then
            table.insert(ip, '"locator":"' .. json_escape(item.locator) .. '"')
        end
        if item.prefix then
            table.insert(ip, '"prefix":"' .. json_escape(item.prefix) .. '"')
        end
        if item.suffix then
            table.insert(ip, '"suffix":"' .. json_escape(item.suffix) .. '"')
        end
        table.insert(item_strs, "{" .. table.concat(ip, ",") .. "}")
    end
    table.insert(parts, '"items":[' .. table.concat(item_strs, ",") .. "]")

    return "{" .. table.concat(parts, ",") .. "}"
end

-- ---------------------------------------------------------------------------
-- Processor constructors
-- ---------------------------------------------------------------------------

--- Create a processor from in-memory JSON strings (low-level).
function CSLN.new(style_json, bib_json)
    local self = setmetatable({}, CSLN)
    self.ptr = lib.csln_processor_new(style_json, bib_json)
    if self.ptr == nil then return nil, "Failed to initialise CSLN processor" end
    self.ptr      = ffi.gc(self.ptr, lib.csln_processor_free)
    self.lib_path = loaded_path
    return self
end

--- Create a processor from CSLN YAML files on disk (primary format).
-- @param style_path  path to a CSLN YAML style file
-- @param bib_path    path to a CSLN YAML bibliography file
function CSLN.from_yaml(style_path, bib_path)
    local self = setmetatable({}, CSLN)
    self.ptr = lib.csln_processor_new_from_yaml(style_path, bib_path)
    if self.ptr == nil then
        return nil, "Failed to initialise CSLN processor from YAML files: "
            .. tostring(style_path) .. ", " .. tostring(bib_path)
    end
    self.ptr      = ffi.gc(self.ptr, lib.csln_processor_free)
    self.lib_path = loaded_path
    return self
end

--- Create a processor from a CSLN YAML style and a biblatex .bib file.
-- @param style_path  path to a CSLN YAML style file
-- @param bib_path    path to a biblatex .bib file
function CSLN.from_bib(style_path, bib_path)
    local self = setmetatable({}, CSLN)
    self.ptr = lib.csln_processor_new_from_bib(style_path, bib_path)
    if self.ptr == nil then
        return nil, "Failed to initialise CSLN processor from .bib file: "
            .. tostring(bib_path)
    end
    self.ptr      = ffi.gc(self.ptr, lib.csln_processor_free)
    self.lib_path = loaded_path
    return self
end

-- ---------------------------------------------------------------------------
-- Processor methods
-- ---------------------------------------------------------------------------

function CSLN:free()
    if self.ptr then
        local ptr = ffi.gc(self.ptr, nil)
        lib.csln_processor_free(ptr)
        self.ptr = nil
    end
end

--- Render a citation to a LaTeX string.
-- @param opts  string (bare key) or table — see build_citation_json above.
function CSLN:render_citation(opts)
    local cite_json = build_citation_json(opts)
    local c_str = lib.csln_render_citation_latex(self.ptr, cite_json)
    return consume_c_str(c_str)
end

--- Render a citation to an HTML string.
function CSLN:render_citation_html(opts)
    local cite_json = build_citation_json(opts)
    local c_str = lib.csln_render_citation_html(self.ptr, cite_json)
    return consume_c_str(c_str)
end

--- Render a citation to a plain-text string.
function CSLN:render_citation_plain(opts)
    local cite_json = build_citation_json(opts)
    local c_str = lib.csln_render_citation_plain(self.ptr, cite_json)
    return consume_c_str(c_str)
end

--- Render the full bibliography as a LaTeX string.
function CSLN:render_bibliography()
    local c_str = lib.csln_render_bibliography_latex(self.ptr)
    return consume_c_str(c_str)
end

--- Render the full bibliography as an HTML string.
function CSLN:render_bibliography_html()
    local c_str = lib.csln_render_bibliography_html(self.ptr)
    return consume_c_str(c_str)
end

--- Render the full bibliography as a plain-text string.
function CSLN:render_bibliography_plain()
    local c_str = lib.csln_render_bibliography_plain(self.ptr)
    return consume_c_str(c_str)
end

-- ---------------------------------------------------------------------------
-- Locator helpers (consumed by the LaTeX package)
-- ---------------------------------------------------------------------------

--- Map of biblatex optional-argument prefixes to CSLN LocatorType strings.
CSLN.locator_labels = {
    ["p."]    = "page",    ["pp."]   = "page",
    ["ch."]   = "chapter", ["chap."] = "chapter",
    ["sec."]  = "section", ["S"]     = "section", ["\xc2\xa7"] = "section",
    ["vol."]  = "volume",  ["v."]    = "volume",
    ["no."]   = "number",  ["n."]    = "number",
    ["fig."]  = "figure",  ["f."]    = "figure",
    ["l."]    = "line",
    ["fn."]   = "note",    ["n"]     = "note",
}

--- Infer a CSLN locator label and value from a raw biblatex optional-arg string.
-- E.g. "p. 23"  -> "page",  "23"
--      "ch. 3"  -> "chapter", "3"
--      "23"     -> "page",  "23"  (bare number defaults to page)
-- Returns label, locator  (both nil if input is empty)
function CSLN.parse_locator(s)
    if not s or s == "" then return nil, nil end
    for prefix, label in pairs(CSLN.locator_labels) do
        local esc  = prefix:gsub("[%(%)%.%%%+%-%*%?%[%^%$]", "%%%1")
        local rest = s:match("^" .. esc .. "%s*(.*)")
        if rest then return label, rest end
    end
    if s:match("^%d") then return "page", s end
    return "page", s
end

--- Render a citation and push it into the TeX output stream.
-- proc     : a CSLN processor object
-- cite_opts: string (bare key) or table — see build_citation_json
function CSLN.do_cite(proc, cite_opts)
    if not proc then
        tex.sprint("[csln: processor not initialised]")
        return
    end
    local result = proc:render_citation(cite_opts)
    if result then
        tex.sprint(result)
    else
        local key = type(cite_opts) == "string" and cite_opts
                    or (cite_opts.items and cite_opts.items[1] and cite_opts.items[1].id)
                    or tostring(cite_opts)
        tex.sprint("[csln: render error for " .. key .. "]")
    end
end

return CSLN

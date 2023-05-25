local FNAME = ngx.config.prefix() .. "/conf/nginx.conf"
local LINES = {}

local STRIP_LUA_INDENT = os.getenv("RUSTY_STRIP_LUA_INDENT") == "1"

do
  local fh = assert(io.open(FNAME, "r"))

  for line in fh:lines() do
    table.insert(LINES, line)
  end

  fh:close()
end


local CURRENT = {
  name = "main",
}

local CONF = CURRENT


local function substr(subj, pat)
  return subj
     and subj:find(pat, nil, true) ~= nil
      or false
end

local function in_lua_block()
  return CURRENT and substr(CURRENT.name, "by_lua_block")
end

local function in_nginx_block()
  return CURRENT and not in_lua_block()
end

local function trim(s)
  return (s:gsub("^%s*(.-)%s*$", "%1"))
end

local function rtrim(s)
  return (s:gsub("^(.-)%s*$", "%1"))
end

local function is_nginx_comment(line)
  return in_nginx_block() and ngx.re.find(line, [[^\s*#]], "oj") ~= nil
end


local function is_lua_comment(line)
  return in_lua_block() and ngx.re.find(line, [[^\s*--]], "oj") ~= nil
end


local function is_empty(line)
  return line == "" or trim(line) == ""
end

local function skip(line)
  return is_empty(line) or is_nginx_comment(line)
end

local function section_enter(line)
  local m = ngx.re.match(line, [[
    ^[\s\t]*

    (?<name>(events|stream|http|init_by_lua_block|init_worker_by_lua_block))

    [\s\t]*

    [{]

    .*
  ]], "ojx")

  local name = m and m.name
  if not name then
    return false
  end

  local parent = CURRENT

  local child = {
    name   = name,
    parent = parent,
  }

  parent.children = parent.children or {}
  table.insert(parent.children, child)

  CURRENT = child

  --print("ENTER " .. child.name)

  return true
end

local function section_exit(line)
  local pos = ngx.re.find(line, [[
    ^[\s\t]*

    [}]

    [\s\t]*

    (\#.*)?

    $
  ]], "ojx")


  if not pos then
    return false
  end

  if in_nginx_block() and CURRENT.directives then
    table.sort(CURRENT.directives)
  end

  local parent = CURRENT.parent

  -- clear cyclical reference
  CURRENT.parent = nil

  CURRENT = parent

  return true
end

local function add_line(line)
  local lines

  if in_nginx_block() then
    line = trim(line)

    if is_nginx_comment(line) or is_empty(line) then
      return
    end

    CURRENT.directives = CURRENT.directives or {}
    lines = CURRENT.directives

  else
    assert(in_lua_block())
    if is_lua_comment(line) then
      return
    end

    if STRIP_LUA_INDENT then
      line = trim(line)
      if is_empty(line) then
        return
      end
    end

    CURRENT.lua = CURRENT.lua or {}
    lines = CURRENT.lua

    local last = lines[#lines] or ""

    -- trim repeated line breaks in lua
    if is_empty(line) and is_empty(last) then
      return
    end
  end

  table.insert(lines, line)
end

for _, line in ipairs(LINES) do
  line = rtrim(line)

  if in_lua_block() and not section_exit(line) then
    add_line(line)

  elseif in_nginx_block() then
    if not section_enter(line) and not section_exit(line) then
      add_line(line)
    end
  end
end

print(require("cjson").encode(CONF))

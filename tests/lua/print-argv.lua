local cjson = require "cjson"
cjson.encode_escape_forward_slash(false)

local fname = os.getenv("RUSTY_CLI_TEST_OUTPUT") or "/dev/stdout"
local fh = assert(io.open(fname, "w+"))

local PROC_SELF = "/proc/" .. ngx.worker.pid()

local function exec(cmd)
  local proc = io.popen(cmd, "r")
  local out = proc:read("*a")
  proc:close()

  out = out:gsub("\n$", "")
  return out
end

local function get_cmd()
  local proc = io.open(PROC_SELF .. "/cmdline", "r")
  local data = proc:read("*a")
  proc:close()

  local items = {}

  data:gsub("[^%z]+", function(item)
    table.insert(items, item)
  end)

  return items
end

local function printf(...)
  return fh:write(string.format(...))
end

local json = {
  cwd = exec("readlink " .. PROC_SELF .. "/cwd"),
  exe = exec("readlink " .. PROC_SELF .. "/exe"),
  cmd = {},
  arg = {},
}

for i, elem in ipairs(get_cmd()) do
  json.cmd[i] = elem
end

for k, v in pairs(arg) do
  json.arg[k] = v
end

fh:write(cjson.encode(json))

fh:close()

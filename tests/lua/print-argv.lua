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


printf("CWD = %q\n", exec("readlink " .. PROC_SELF .. "/cwd"))
printf("EXE = %q\n", exec("readlink " .. PROC_SELF .. "/exe"))

do
  printf("CMD = {\n")
  for i, elem in ipairs(get_cmd()) do
    printf("  [%s] = %q\n", i, elem)
  end
  printf("}\n")
end

do
  local keys = {}

  for k in pairs(arg) do
    table.insert(keys, k)
  end

  table.sort(keys)

  printf("ARG = {\n")
  for _, k in ipairs(keys) do
    printf("  [%s] = %q\n", k, arg[k])
  end
  printf("}\n")
end

fh:close()

local fname = os.getenv("RUSTY_CLI_TEST_OUTPUT") or "/dev/stdout"
local fh = assert(io.open(fname, "w+"))

local keys = {}

for k in pairs(arg) do
  table.insert(keys, k)
end

table.sort(keys)

for _, k in ipairs(keys) do
  local key = ("arg[%s]"):format(k)

  fh:write(("%-10s = %q\n"):format(key, arg[k]))
end

fh:close()

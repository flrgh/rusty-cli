local fname = ngx.config.prefix() .. "/conf/nginx.conf"
local fh = assert(io.open(fname, "r"))
print(fh:read("*a"))
fh:close()

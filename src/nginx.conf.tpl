daemon off;
master_process off;
worker_processes 1;
pid logs/nginx.pid;

{% for line in main_conf %}
{{ line }}
{%- endfor %}

events {
    worker_connections {{ worker_connections }};
}

{% if stream_enabled %}
stream {
    access_log off;
    lua_socket_log_errors off;
    lua_regex_cache_max_entries 40960;
    {% for line in stream_conf %}
    {{ line }}
    {%- endfor %}
}
{% endif %}

http {
    access_log off;
    lua_socket_log_errors off;
    lua_regex_cache_max_entries 40960;

    {% for line in http_conf %}
    {{ line }}
    {%- endfor %}

    init_by_lua_block {
        ngx.config.is_console = true

        local stdout = io.stdout
        local ngx_null = ngx.null
        local maxn = table.maxn
        local unpack = unpack
        local concat = table.concat

        local expand_table
        function expand_table(src, inplace)
            local n = maxn(src)
            local dst = inplace and src or {}
            for i = 1, n do
                local arg = src[i]
                local typ = type(arg)
                if arg == nil then
                    dst[i] = "nil"

                elseif typ == "boolean" then
                    if arg then
                        dst[i] = "true"
                    else
                        dst[i] = "false"
                    end

                elseif arg == ngx_null then
                    dst[i] = "null"

                elseif typ == "table" then
                    dst[i] = expand_table(arg, false)

                elseif typ ~= "string" then
                    dst[i] = tostring(arg)

                else
                    dst[i] = arg
                end
            end
            return concat(dst)
        end

        local function output(...)
            local args = {...}

            return stdout:write(expand_table(args, true))
        end

        ngx.orig_print = ngx.print
        ngx.print = output

        ngx.orig_say = ngx.say
        ngx.say = function (...)
                local ok, err = output(...)
                if ok then
                    return output("\n")
                end
                return ok, err
            end
        print = ngx.say

        ngx.flush = function (...) return stdout:flush() end
        -- we cannot close stdout here due to a bug in Lua:
        ngx.eof = function (...) return true end
        ngx.orig_exit = ngx.exit
        ngx.exit = os.exit
    }

    init_worker_by_lua_block {
        local exit = os.exit
        local stderr = io.stderr
        local ffi = require "ffi"

        local function handle_err(err)
            if err then
                err = string.gsub(err, "^init_worker_by_lua:%d+: ", "")
                stderr:write("ERROR: ", err, "\n")
            end
            return exit(1)
        end

        local ok, err = pcall(function ()
            if not ngx.config
               or not ngx.config.ngx_lua_version
               or ngx.config.ngx_lua_version < 10009
            then
                error("at least ngx_lua 0.10.9 is required")
            end

            local signal_graceful_exit =
                require("ngx.process").signal_graceful_exit
            if not signal_graceful_exit then
                error("lua-resty-core library is too old; "
                      .. "missing the signal_graceful_exit() function "
                      .. "in ngx.process")
            end

            {% for line in lua_loader %}
            {{ line }}
            {%- endfor %}

            -- print("calling timer.at...")
            local ok, err = ngx.timer.at(0, function ()
                -- io.stderr:write("timer firing")
                local ok, err = xpcall(gen, function (err)
                    -- level 3: we skip this function and the
                    -- error() call itself in our stacktrace
                    local trace = debug.traceback(err, 3)
                    return handle_err(trace)
                end)
                if not ok then
                    return handle_err(err)
                end
                if ffi.abi("win") then
                    return exit(0)
                end
                signal_graceful_exit()
            end)
            if not ok then
                return handle_err(err)
            end
            -- print("timer created")
        end)

        if not ok then
            return handle_err(err)
        end
    }
}

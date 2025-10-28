local ffi = require "ffi"

ffi.cdef [[
  void* malloc(size_t bytes);
]]

ffi.fill(ffi.C.malloc(0), 999999999999999999999999)

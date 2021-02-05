local api = vim.api
local rofl = {}

-- local binary_path = vim.fn.fnamemodify(api.nvim_get_runtime_file("lua/rofl.lua", false)[1], ":h:h") .. "/target/debug/rofl_nvim"
local binary_path = vim.fn.fnamemodify(api.nvim_get_runtime_file("lua/rofl.lua", false)[1], ":h:h") .. "/target/release/rofl_nvim"

rofl.start = function(bufnr)
  bufnr = bufnr or 0

  if rofl.job_id then
    return
  end

  rofl.job_id = vim.fn.jobstart(
    {binary_path},
    {
      rpc = true
    }
  )
end

rofl.byte_offset = function()
  return vim.fn.line2byte(vim.fn.line('.')) + vim.fn.col('.') - 2
end

rofl.attach = function(bufnr)
  bufnr = bufnr or 0

  vim.cmd [[augroup Rofl]]
  vim.cmd [[au!]]
  vim.cmd [[augroup END]]

  vim.api.nvim_register_filterfunc(function() return 1 end)

  -- vim.cmd [[autocmd! InsertCharPre <buffer> lua require'rofl'.notify("v_char", vim.api.nvim_get_vvar("char"))]]
  vim.cmd [[autocmd Rofl InsertCharPre <buffer> lua require'rofl'.insert_char_pre()]]

  -- vim.cmd [[autocmd! TextChangedP <buffer> lua require'rofl'.notify("complete")]]
  -- vim.cmd [[autocmd! TextChangedI <buffer> lua require'rofl'.notify("complete")]]
  -- vim.cmd [[autocmd! TextChanged <buffer> lua require'rofl'.notify("complete")]]

  vim.cmd [[autocmd Rofl InsertLeave <buffer> lua require'rofl'.notify("insert_leave")]]

  vim.api.nvim_buf_attach(0, false, {
    on_lines = function(_, buf, _, firstline, _, new_lastline)
      local mode = api.nvim_get_mode()["mode"]
      if mode ~= "i" or mode ~= "ic" then return end
      local lines = api.nvim_buf_get_lines(buf, firstline, new_lastline, false)
      if #lines == 0 then
        rofl.notify("complete")
      end
    end
  })
end

rofl.insert_char_pre = function()
  rofl.notify("v_char", vim.api.nvim_get_vvar("char"))
  rofl.notify("complete")
end

rofl.request = function(method, ...)
  rofl.start()
  return vim.rpcrequest(rofl.job_id, method, ...)
end

rofl.notify = function(method, ...)
  rofl.start()
  vim.rpcnotify(rofl.job_id, method, ...)
end

rofl.update_words = function()
  rofl.notify("update_buffer_words")
end

local sources = {

}

rofl.add_source = function(name, fn)
  if sources[name] ~= nil then
    error("There is already a source named " .. name)
    return
  end
  sources[name] = fn
end

rofl.get_source = function(name)
  return sources[name]
end

-- use this to be able to run sources in tokio tasks
return rofl

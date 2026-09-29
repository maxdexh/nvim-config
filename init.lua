local config_dir = vim.fn.stdpath("config") --[[@as string]]

local debug = vim.fn.getenv("NVIM_DEBUG") ~= vim.NIL

---@param root string
---@return string
local function dir_stamp(root)
	local parts = {}

	local function walk(dir)
		local handle = vim.uv.fs_scandir(dir)
		if not handle then
			return
		end

		while true do
			local name, typ = vim.uv.fs_scandir_next(handle)
			if not name then
				break
			end

			local path = vim.fs.joinpath(dir, name)
			local stat = vim.uv.fs_stat(path)

			if stat then
				if typ == "directory" then
					walk(path)
				elseif typ == "file" then
					parts[#parts + 1] = table.concat({
						path,
						stat.size,
						stat.mtime.sec,
						stat.mtime.nsec,
					}, "\0")
				end
			end
		end
	end

	walk(root)

	table.sort(parts)

	return vim.fn.sha256(table.concat(parts, "\n"))
end

---@return string
local function compile_blocking()
	local cmd = {
		"cargo",
		"build",
		"--message-format=json-render-diagnostics",
	}
	if not debug then
		table.insert(cmd, "--release")
	end

	local opts = {
		cwd = config_dir,
		text = true,
	}

	---@param output vim.SystemCompleted
	---@return string
	local function get_artifact(output)
		if output.code ~= 0 then
			local msg = "command `" .. table.concat(cmd, " ") .. "` exited with code " .. output.code .. "\n\n"
			msg = msg .. (output.stderr or "logic error: missing stderr")
			error(msg)
		end

		if output.stdout == nil then
			error("logic error: missing stdout")
		end

		local artifacts = {} ---@type table[]

		local function ensure(cond, subject, msg)
			if not cond then
				error(msg .. ":\n" .. vim.inspect(subject))
			end
			return true
		end

		local lines = vim.split(output.stdout, "\n", { trimempty = true })
		for _, line in ipairs(lines) do
			local msg = vim.json.decode(line) ---@type unknown
			if
				ensure(type(msg) == "table", msg, "msg should be table")
				and msg.reason == "compiler-artifact"
				and ensure(type(msg.target) == "table", msg.target, "target should be table")
				and msg.target.name == "nvim_config"
			then
				table.insert(artifacts, msg)
			end
		end
		if #artifacts ~= 1 then
			error("logic error: expected single compiler-artifact message, got:\n" .. vim.inspect(artifacts))
		end
		local fnames = artifacts[1].filenames
		if type(fnames) ~= "table" or #fnames ~= 1 or type(fnames[1]) ~= "string" then
			error("logic error: expected exactly one filename, got:\n" .. vim.inspect(fnames))
		end
		return fnames[1]
	end

	return get_artifact(vim.system(cmd, opts):wait())
end

local cache_dir = vim.fs.joinpath(config_dir, "target", "config-lib-cache")

---@param stamp string
---@return string
local function get_store_path(stamp)
	return vim.fs.joinpath(cache_dir, "nvim-config-" .. tostring(stamp))
end

---@return string
local function get_newest_lib()
	local newest ---@type string?
	local newest_time = -1 ---@type number

	for name, type in vim.fs.dir(cache_dir) do
		if type == "file" then
			local path = vim.fs.joinpath(cache_dir, name)
			local stat = vim.uv.fs_stat(path)

			if stat then
				local mtime = stat.mtime.sec
				if mtime > newest_time then
					newest = path
					newest_time = mtime
				end
			end
		end
	end

	if newest then
		return newest
	end

	error("Found no candidate libs in store")
end

---@param path string
---@return fun()
local function load_lib(path)
	local lib = package.loadlib(path, "luaopen_nvim_config")
	if lib then
		return lib
	end

	error("Failed to load nvim config lib:\n" .. path)
end

---@param lib fun()
---@return boolean
local function run_lib(lib)
	-- Errors should already be printed by the config
	return (pcall(lib))
end

local lib_cache_path = get_store_path(dir_stamp(vim.fs.joinpath(config_dir, "src")))

if not debug and vim.fn.filereadable(lib_cache_path) == 1 then
	local reload_ok, reload_error = pcall(function()
		run_lib(load_lib(get_newest_lib()))
	end)

	vim.defer_fn(function()
		if not reload_ok then
			vim.notify(reload_error, vim.log.levels.ERROR)
		end
	end, 100)
	return
end

local compile_ok, comp_err_or_lib_fn, lib_path = pcall(function()
	local lib_path = compile_blocking()

	return load_lib(lib_path), lib_path
end)

if compile_ok then
	if not run_lib(comp_err_or_lib_fn) then
		return
	end

	if debug then
		return
	end

	-- Only cache the lib if we successfully make it out of the config
	-- and are not in debug mode
	local store_dir = vim.fs.dirname(lib_cache_path)

	if vim.fn.mkdir(store_dir, "p") == 0 then
		vim.fn.notify("failed to create lib cache dir:\n" .. store_dir, vim.log.levels.ERROR)
		return
	end

	if vim.fn.filecopy(lib_path, lib_cache_path) == 0 then
		vim.fn.notify(
			"failed to copy config lib to cache dir:\n" .. "src: " .. lib_path .. "\ndst: " .. lib_cache_path,
			vim.log.levels.ERROR
		)
		return
	end
	vim.notify("successfully cached new config lib:\n" .. lib_cache_path)

	return
end

local reload_ok, reload_error = pcall(function()
	run_lib(load_lib(get_newest_lib()))
end)

vim.defer_fn(function()
	vim.notify(comp_err_or_lib_fn, vim.log.levels.ERROR)
	if not reload_ok then
		vim.notify(reload_error, vim.log.levels.ERROR)
	end
end, 100)

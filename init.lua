local config_path = vim.fn.stdpath("config") --[[@as string]]

---@return string
local function compile_blocking()
	local cmd = {
		"cargo",
		"build",
		"--message-format=json-render-diagnostics",
	}
	if vim.fn.getenv("NVIM_DEBUG") == vim.NIL then
		table.insert(cmd, "--release")
	end

	local opts = {
		cwd = config_path,
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

local build_dir = vim.fs.joinpath(config_path, "target", "bak")

---@param time integer
---@return string
local function make_store_path(time)
	local output = vim.system({
		"git",
		"rev-parse",
		"--short",
		"HEAD",
	}, {
		cwd = config_path,
	}):wait(100)

	local commit_hash = output.code == 0 and output.stdout or "unknown"
	commit_hash = vim.fn.trim(commit_hash, "\n ")

	return vim.fs.joinpath(build_dir, "nvim-config-" .. commit_hash .. "-" .. tostring(time))
end

---@return string
local function get_store_newest()
	local newest_ts ---@type number?
	local newest_lib ---@type string?

	for name, typ in vim.fs.dir(build_dir) do
		(function()
			if typ ~= "file" and typ ~= "link" then
				return
			end
			local parts = vim.split(name, "-")
			if #parts == 0 then
				return
			end
			local timestamp = tonumber(parts[#parts])
			if timestamp == nil then
				return
			end

			if newest_ts == nil or timestamp > newest_ts then
				newest_ts = timestamp
				newest_lib = name
			end
		end)()
	end

	if newest_lib then
		return vim.fs.joinpath(build_dir, newest_lib)
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

local timestamp = vim.fn.localtime()
local compile_ok, compile_error = pcall(function()
	local lib_path = compile_blocking()

	load_lib(lib_path)()

	vim.api.nvim_create_user_command("StoreConfigLib", function()
		local store_path = make_store_path(timestamp)
		local store_dir = vim.fs.dirname(store_path)
		if vim.fn.mkdir(store_dir, "p") == 0 then
			error("failed to create store directory:\n" .. store_dir)
		end
		if vim.fn.filecopy(lib_path, store_path) == 0 then
			error("failed to copy config lib to store:\n" .. "src: " .. lib_path .. "\ndst: " .. store_path)
		end
		vim.notify("successfully copied config lib to store:\n" .. store_path)
	end, {})
end)
if compile_ok then
	return
end

local reload_ok, reload_error = pcall(function()
	local lib_path = get_store_newest()
	load_lib(lib_path)()
end)

vim.defer_fn(function()
	vim.notify(compile_error, vim.log.levels.ERROR)
	if not reload_ok then
		vim.notify(reload_error, vim.log.levels.ERROR)
	end
end, 100)

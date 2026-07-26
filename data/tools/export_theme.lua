-- export_theme.lua

local function rgb_to_hex(rgb)
    if rgb == nil then
        return nil
    end

    return string.format("#%06X", rgb)
end

local function normalize(hl)
    local out = {}

    for k, v in pairs(hl) do
        if k == "fg" or k == "bg" or k == "sp" then
            out[k] = rgb_to_hex(v)
        else
            out[k] = v
        end
    end

    return out
end

function EXPORT_THEME(filename)

    local export = {
        metadata = {
            name = vim.g.colors_name or "unknown",
            background = vim.o.background,
            termguicolors = vim.o.termguicolors,
            generated = os.date("!%Y-%m-%dT%H:%M:%SZ"),
        },

        terminal = {},

        highlights = {},
    }

    ----------------------------------------------------------
    -- terminal colors
    ----------------------------------------------------------

    for i = 0, 15 do
        local key = "terminal_color_" .. i
        local ok, value = pcall(function()
            return vim.g[key]
        end)

        if ok and value ~= nil then
            export.terminal[key] = value
        end
    end

    ----------------------------------------------------------
    -- highlight groups
    ----------------------------------------------------------

    local groups = vim.fn.getcompletion("", "highlight")

    table.sort(groups)

    for _, name in ipairs(groups) do

        local ok, hl = pcall(vim.api.nvim_get_hl, 0, {
            name = name,
            link = false,      -- resolve links
        })

        if ok then
            export.highlights[name] = normalize(hl)
        end
    end

    ----------------------------------------------------------
    -- write file
    ----------------------------------------------------------

    local file = assert(io.open(filename, "w"))

    file:write(vim.json.encode(export))

    file:close()

    print(string.format(
        "Exported %d highlight groups -> %s",
        #groups,
        filename
    ))
end

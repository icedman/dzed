#!/usr/bin/env python3
import sys
import os
import subprocess
import tempfile
import json
import re

def convert_json_to_toml(json_path, toml_path, repo_url=None):
    with open(json_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    # if os.path.exists(json_path):
    #     os.remove(json_path)

    metadata_in = data.get("metadata", {})
    name = metadata_in.get("name")
    if not name or name == "unknown":
        print("Error: Metadata name is unknown or missing.", file=sys.stderr)
        sys.exit(1)
    bg_type = metadata_in.get("background", "dark")

    highlights = data.get("highlights", {})

    unique_colors = set()
    for style in highlights.values():
        if not isinstance(style, dict):
            continue
        fg = style.get("fg")
        bg = style.get("bg")
        if fg and fg.startswith("#"):
            unique_colors.add(fg.lower())
        if bg and bg.startswith("#"):
            unique_colors.add(bg.lower())

    sorted_unique_colors = sorted(list(unique_colors))

    palette_hex_to_name = {}
    colors_table = {}
    for i, color_hex in enumerate(sorted_unique_colors):
        color_name = f"c_{i}"
        palette_hex_to_name[color_hex] = color_name
        colors_table[color_name] = color_hex

    ui = {}
    syntax = {}

    group_to_ui = {
        "Normal": ("foreground", "background"),
        "Cursor": ("caret", None),
        "Visual": (None, "selection"),
        "LineNr": ("gutter_foreground", None),
        "FoldColumn": (None, "gutter"),
        "Folded": (None, "find_highlight"),
        "Search": ("find_highlight_foreground", "find_highlight"),
        "IncSearch": ("find_highlight_foreground", "find_highlight"),
        "CursorLine": (None, "cursor_line"),
        "CursorLineNr": ("cursor_line_nr", None),
        "StatusLine": ("statusline_foreground", "statusline_background"),
        "StatusLineNC": ("statusline_nc_foreground", "statusline_nc_background"),
        "TabLine": ("tabline_foreground", "tabline_background"),
        "TabLineSel": ("tabline_sel_foreground", "tabline_sel_background"),
        "TabLineFill": (None, "tabline_fill"),
        "Pmenu": ("pmenu_foreground", "pmenu_background"),
        "PmenuSel": ("pmenu_sel_foreground", "pmenu_sel_background"),
        "PmenuSbar": (None, "pmenu_sbar"),
        "PmenuThumb": (None, "pmenu_thumb"),
        "DiffAdd": ("diff_add_foreground", "diff_add_background"),
        "DiffChange": ("diff_change_foreground", "diff_change_background"),
        "DiffDelete": ("diff_delete_foreground", "diff_delete_background"),
        "DiffText": ("diff_text_foreground", "diff_text_background"),
        "DiagnosticError": ("diagnostic_error", None),
        "DiagnosticWarn": ("diagnostic_warn", None),
        "DiagnosticInfo": ("diagnostic_info", None),
        "DiagnosticHint": ("diagnostic_hint", None),
        "DiagnosticOk": ("diagnostic_ok", None),
        "ErrorMsg": ("error", None),
        "WarningMsg": ("warning", None),
        "MatchParen": ("match_paren_foreground", "match_paren_background"),
        "NonText": ("non_text", None),
        "SpecialKey": ("special_key", None),
        "Whitespace": ("whitespace", None),
        "Added": ("diff_add_foreground", None),
        "Changed": ("diff_change_foreground", None),
        "Removed": ("diff_delete_foreground", None),
        "WinSeparator": ("border_foreground", "border_background"),
        "VertSplit": ("border_foreground", "border_background"),
        "FloatBorder": ("float_border_foreground", "float_border_background"),
        "NormalFloat": ("float_foreground", "float_background"),
    }

    group_to_syntax = {
        "Comment": "comment",
        "Keyword": "keyword",
        "Statement": "keyword",
        "String": "string",
        "Constant": "constant",
        "Number": "number",
        "Function": "function",
        "Identifier": "function",
        "Type": "type",
        "Operator": "operator",
        "PreProc": "keyword",
        "Boolean": "boolean",
        "Character": "character",
        "Float": "float",
        "Conditional": "keyword",
        "Repeat": "keyword",
        "Label": "keyword",
        "Exception": "keyword",
        "Include": "keyword",
        "Define": "keyword",
        "Macro": "keyword",
        "PreCondit": "keyword",
        "StorageClass": "type",
        "Structure": "type",
        "Typedef": "type",
        "Special": "special",
        "SpecialChar": "special",
        "Tag": "special",
        "Delimiter": "delimiter",
        "SpecialComment": "comment",
        "Debug": "special",
        "Todo": "todo",
        "Underlined": "underlined",
        "Error": "error",
        "@variable": "variable",
        "@property": "property",
        "@module": "module",
        "@constructor": "constructor",
        "@tag": "tag",
        "@tag.delimiter": "tag_delimiter",
        "@tag.attribute": "tag_attribute",
        "@markup.heading": "heading",
        "@markup.link": "link",
    }

    for group, style in highlights.items():
        if not isinstance(style, dict):
            continue
        fg_hex = style.get("fg")
        bg_hex = style.get("bg")

        fg_name = palette_hex_to_name.get(fg_hex.lower()) if fg_hex else None
        bg_name = palette_hex_to_name.get(bg_hex.lower()) if bg_hex else None

        if group in group_to_ui:
            fg_key, bg_key = group_to_ui[group]
            if fg_key and fg_name:
                ui[fg_key] = fg_name
            if bg_key and bg_name:
                ui[bg_key] = bg_name
        if group in group_to_syntax:
            syntax_key = group_to_syntax[group]
            if fg_name:
                syntax[syntax_key] = fg_name

    if "selection" not in ui and "background" in ui:
        ui["selection"] = "foreground"
    if "caret" not in ui and "foreground" in ui:
        ui["caret"] = "foreground"

    author = ""
    github = ""
    if repo_url:
        github = repo_url
        match = re.search(r'(?:github\.com[:/])([^/]+)', repo_url)
        if match:
            author = f"github/{match.group(1)}"

    with open(toml_path, 'w', encoding='utf-8') as f:
        f.write("[metadata]\n")
        f.write(f'name = "{name}"\n')
        f.write(f'description = ""\n')
        f.write(f'author = "{author}"\n')
        if github:
            f.write(f'github = "{github}"\n')
        f.write(f'type = "{bg_type}"\n\n')

        f.write("[colors]\n")
        for color_name, val in sorted(colors_table.items(), key=lambda item: int(item[0].split("_")[1])):
            f.write(f'{color_name} = "{val}"\n')
        f.write("\n")

        f.write("[ui]\n")
        for ui_name, val in sorted(ui.items()):
            f.write(f'{ui_name} = "{val}"\n')
        f.write("\n")

        f.write("[syntax]\n")
        for syntax_name, val in sorted(syntax.items()):
            f.write(f'{syntax_name} = "{val}"\n')

def export_single_theme(repo_url, scheme_name, lua_script, output_dir):
    toml_path = os.path.join(output_dir, f"{scheme_name}.toml")
    json_path = os.path.join(output_dir, f"{scheme_name}.json")

    if os.path.exists(json_path):
        print(f"\nJSON file already exists for '{scheme_name}': {json_path}. Skipping export, converting directly to TOML...")
        try:
            convert_json_to_toml(json_path, toml_path, repo_url=repo_url)
            print(f"  Successfully generated: {toml_path}")
        except Exception as e:
            print(f"  Error converting JSON to TOML: {e}")
        return

    if os.path.exists(toml_path):
        print(f"\nSkipping colorscheme '{scheme_name}' as output file already exists: {toml_path}")
        return

    print(f"\nProcessing colorscheme '{scheme_name}' from {repo_url}...")
    with tempfile.TemporaryDirectory() as temp_dir:
        print(f"  Cloning repository...")
        try:
            subprocess.run(["git", "clone", "--depth", "1", repo_url, temp_dir], check=True)
        except subprocess.CalledProcessError:
            print(f"  Failed to clone repository: {repo_url}")
            return

        print(f"  Exporting colorscheme to JSON...")
        try:
            subprocess.run([
                "nvim",
                "--headless",
                "--clean",
                "--cmd", f"set runtimepath^={temp_dir}",
                "-c", f"colorscheme {scheme_name}",
                "-c", f"luafile {lua_script}",
                "-c", f"lua EXPORT_THEME('{json_path}')",
                "-c", "qa!"
            ], check=True)
        except subprocess.CalledProcessError:
            print(f"  Failed to export theme '{scheme_name}' using Neovim.")
            return

        print(f"  Converting JSON to TOML...")
        try:
            convert_json_to_toml(json_path, toml_path, repo_url=repo_url)
            print(f"  Successfully generated: {toml_path}")
        except Exception as e:
            print(f"  Error converting JSON to TOML: {e}")

def main():
    if len(sys.argv) < 2:
        print("Usage:")
        print("  Single theme: export_to_toml.py <github_repo_url> <colorscheme_name>")
        print("  Multiple themes: export_to_toml.py <sources_txt_file>")
        sys.exit(1)

    input_path = sys.argv[1]

    script_dir = os.path.dirname(os.path.abspath(__file__))
    lua_script = os.path.join(script_dir, "export_theme.lua")
    output_dir = "data/schemes"

    if not os.path.exists(lua_script):
        print(f"Error: Required exporter script not found at '{lua_script}'")
        sys.exit(1)

    os.makedirs(output_dir, exist_ok=True)

    if os.path.isfile(input_path):
        print(f"Reading sources from file: {input_path}...")
        with open(input_path, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                parts = line.split()
                if len(parts) >= 2:
                    repo_url = parts[0]
                    scheme_name = parts[1]
                    export_single_theme(repo_url, scheme_name, lua_script, output_dir)
                else:
                    print(f"Skipping invalid line: {line}")
    else:
        if len(sys.argv) < 3:
            print("Error: Colorscheme name required when exporting a single repository.")
            print("Usage: export_to_toml.py <github_repo_url> <colorscheme_name>")
            sys.exit(1)
        repo_url = input_path
        scheme_name = sys.argv[2]
        export_single_theme(repo_url, scheme_name, lua_script, output_dir)

if __name__ == "__main__":
    main()

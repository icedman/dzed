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
        
    if os.path.exists(json_path):
        os.remove(json_path)
        
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

def main():
    if len(sys.argv) < 3:
        print("Usage: export_to_toml.py <github_repo_url> <colorscheme_name> [<output_dir>]")
        sys.exit(1)
        
    repo_url = sys.argv[1]
    scheme_name = sys.argv[2]
    output_dir = sys.argv[3] if len(sys.argv) > 3 else "."
    
    script_dir = os.path.dirname(os.path.abspath(__file__))
    lua_script = os.path.join(script_dir, "export_theme.lua")
    
    if not os.path.exists(lua_script):
        print(f"Error: Required exporter script not found at '{lua_script}'")
        sys.exit(1)
        
    os.makedirs(output_dir, exist_ok=True)
    
    print(f"Cloning theme repository: {repo_url}...")
    with tempfile.TemporaryDirectory() as temp_dir:
        subprocess.run(["git", "clone", "--depth", "1", repo_url, temp_dir], check=True)
        
        print(f"Exporting colorscheme '{scheme_name}' to JSON...")
        json_path = os.path.join(output_dir, f"{scheme_name}.json")
        
        subprocess.run([
            "nvim", "--headless", "--clean",
            "--cmd", f"set runtimepath^={temp_dir}",
            "-c", f"colorscheme {scheme_name}",
            "-c", f"luafile {lua_script}",
            "-c", f"lua EXPORT_THEME('{json_path}')",
            "-c", "qa!"
        ], check=True)
        
        toml_path = os.path.join(output_dir, f"{scheme_name}.toml")
        print(f"Converting JSON to TOML color scheme format...")
        convert_json_to_toml(json_path, toml_path, repo_url=repo_url)
            
        print(f"\nSuccessfully unified export completed! Generated theme: {toml_path}")

if __name__ == "__main__":
    main()

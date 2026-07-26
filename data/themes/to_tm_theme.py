#!/usr/bin/env python3
import sys
import os
import json
import plistlib

def convert_vscode_to_tm(json_path, out_path):
    with open(json_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
        
    theme_name = data.get("displayName", data.get("name", "Converted Theme"))
    
    root_settings = {}
    colors = data.get("colors", {})
    
    mapping = {
        "editor.background": "background",
        "editor.foreground": "foreground",
        "editorCursor.foreground": "caret",
        "editor.selectionBackground": "selection",
        "editor.lineHighlightBackground": "lineHighlight",
        "editor.findMatchHighlightBackground": "findHighlight",
    }
    
    for vscode_key, tm_key in mapping.items():
        if vscode_key in colors:
            root_settings[tm_key] = colors[vscode_key]
            
    token_colors = data.get("tokenColors", [])
    for rule in token_colors:
        if "scope" not in rule and "settings" in rule:
            for k, v in rule["settings"].items():
                if k in ["background", "foreground", "caret", "selection"]:
                    root_settings[k] = v
                    
    settings_list = []
    settings_list.append({
        "settings": root_settings
    })
    
    for rule in token_colors:
        scope = rule.get("scope")
        if not scope:
            continue
            
        scope_str = ""
        if isinstance(scope, list):
            scope_str = ", ".join(scope)
        elif isinstance(scope, str):
            scope_str = scope
            
        settings_dict = {
            "settings": rule.get("settings", {})
        }
        if "name" in rule:
            settings_dict["name"] = rule["name"]
        settings_dict["scope"] = scope_str
        
        settings_list.append(settings_dict)
        
    plist_data = {
        "name": theme_name,
        "settings": settings_list
    }
    
    with open(out_path, 'wb') as f:
        plistlib.dump(plist_data, f)
        
    print(f"Successfully converted VS Code theme to tmTheme: {out_path}")

def main():
    if len(sys.argv) < 2:
        print("Usage: to_tm_theme.py <path_to_vscode_json> [<output_tmTheme_path>]")
        sys.exit(1)
        
    json_path = sys.argv[1]
    if len(sys.argv) > 2:
        out_path = sys.argv[2]
    else:
        out_path = json_path.replace(".json", ".tmTheme")
        
    if not os.path.exists(json_path):
        print(f"Error: File '{json_path}' not found.")
        sys.exit(1)
        
    convert_vscode_to_tm(json_path, out_path)

if __name__ == "__main__":
    main()

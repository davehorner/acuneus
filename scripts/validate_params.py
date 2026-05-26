import os
import re

def get_wgsl_params(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    match = re.search(r'struct \w*Params\w*\s*\{([^}]*)\}', content)
    if not match: return []
    fields = []
    for line in match.group(1).split('\n'):
        line = line.split('//')[0].strip()
        if not line: continue
        field_match = re.search(r'([a-zA-Z0-9_]+)\s*:\s*([a-zA-Z0-9_<,>]+)', line)
        if field_match:
            name = field_match.group(1)
            typ = field_match.group(2).replace(',', '')
            if name.startswith('_pad'): continue
            if 'pad' in name.lower() and ('f32' in typ or 'i32' in typ): continue
            fields.append(name)
    return fields

def get_registry_params():
    with open('src/bin_registry.rs', 'r', encoding='utf-8') as f:
        content = f.read()
    
    registry = {}
    matches = re.finditer(r'pub const ([A-Z0-9_]+)_PARAMS: &\[BinParamSpec\] = &\[(.*?)\];', content, re.DOTALL)
    for m in matches:
        bin_name = m.group(1).lower()
        if bin_name == 'neuron2d': bin_name = '2dneuron'
        fields_text = m.group(2)
        params = []
        for line in fields_text.split('\n'):
            param_match = re.search(r'!\("([^"]+)"', line)
            if param_match:
                params.append(param_match.group(1))
        registry[bin_name] = params
    return registry

def main():
    registry = get_registry_params()
    existing = ['roto', 'cuneus', 'spiral', 'voronoi', 'matrix', 'tree', '2dneuron', 'gabor', 'plasma', 'lorenz', 'nebula', 'satan', 'sdvert', 'asahi']
    
    for b in existing:
        wgsl_path = f'shaders/{b}.wgsl'
        wgsl_params = get_wgsl_params(wgsl_path)
        reg_params = registry.get(b, [])
        
        # Preserve order for missing logic
        missing_in_reg = [p for p in wgsl_params if p not in reg_params]
        extra_in_reg = [p for p in reg_params if p not in wgsl_params]
        
        if missing_in_reg or extra_in_reg:
            print(f"--- {b} ---")
            if missing_in_reg:
                print(f"  Missing in registry (in WGSL but not UI): {', '.join(missing_in_reg)}")
            if extra_in_reg:
                print(f"  Extra in registry (in UI but not WGSL): {', '.join(extra_in_reg)}")

if __name__ == '__main__':
    main()

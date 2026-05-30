import os
import re

def to_title_case(name):
    return ' '.join(word.capitalize() for word in name.split('_'))

def parse_wgsl_params(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    match = re.search(r'struct \w*Params\w*\s*\{([^}]*)\}', content)
    if not match:
        return None
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
            
            if typ == 'f32':
                fields.append(f'    f32_param!("{name}", "{to_title_case(name)}", 0.0, 1.0, 0.5),')
            elif typ == 'bool':
                fields.append(f'    bool_param!("{name}", "{to_title_case(name)}", 0.0),')
            elif typ in ('vec3<f32>', 'vec4<f32>'):
                fields.append(f'    color3_param!("{name}", "{to_title_case(name)}", 0.0, 1.0, 0.5),')
    return fields

def main():
    shaders_dir = 'shaders'
    existing = ['roto', 'cuneus', 'spiral', 'voronoi', 'matrix', 'tree', '2dneuron', 'gabor', 'plasma', 'lorenz', 'nebula', 'satan', 'sdvert', 'asahi']
    new_bins = []
    with open('generated.rs', 'w', encoding='utf-8') as out:
        for file in sorted(os.listdir(shaders_dir)):
            if file.endswith('.wgsl'):
                bin_name = file[:-5]
                if bin_name in existing: continue
                fields = parse_wgsl_params(os.path.join(shaders_dir, file))
                if fields:
                    const_name = bin_name.upper() + '_PARAMS'
                    out.write(f'pub const {const_name}: &[BinParamSpec] = &[\n')
                    for f in fields:
                        out.write(f + '\n')
                    out.write('];\n\n')
                    new_bins.append(bin_name)
        if new_bins:
            out.write('pub const BINS: &[&CStr] = &[\n')
            for b in existing + new_bins:
                out.write(f'    cstr!("{b}"),\n')
            out.write('];\n\n')
            
            out.write('pub fn params_for_bin(bin_name: &str) -> Option<&\'static [BinParamSpec]> {\n')
            out.write('    match bin_name {\n')
            for b in existing + new_bins:
                const_name = b.upper() + '_PARAMS'
                if b == '2dneuron':
                    const_name = 'NEURON2D_PARAMS'
                out.write(f'        "{b}" => Some({const_name}),\n')
            out.write('        _ => None,\n')
            out.write('    }\n')
            out.write('}\n')

if __name__ == '__main__':
    main()

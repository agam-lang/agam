import re
import os

filepath = r'c:\Users\ksvik\Projects\Agam-Lang\agam\crates\backends\agam_codegen\src\c_emitter.rs'
out_dir = r'c:\Users\ksvik\Projects\Agam-Lang\agam\crates\backends\agam_codegen\runtime'

os.makedirs(out_dir, exist_ok=True)

content = open(filepath, 'r', encoding='utf-8').read()

def extract_and_replace():
    global content
    
    # We look for blocks like:
    # fn emit_common_prelude(out: &mut String) {
    #     out.push_str(
    #         r#"/* ... */"#,
    #     );
    # }
    
    # Use re to find all prelude emitting functions and their embedded raw string blocks.
    func_pattern = re.compile(r'fn\s+emit_([a-z_]+_prelude)\s*\([^\)]*\)\s*\{[^{]*?out\.push_str\(\s*r#"(.*?)"#,[ \t\r\n]*\);', re.DOTALL)
    
    func_matches = list(func_pattern.finditer(content))
    print(f'Found {len(func_matches)} prelude functions with embedded C code.')
    
    for match in func_matches:
        prelude_name = match.group(1)
        c_code = match.group(2)
        
        filename = f'{prelude_name}.c'
        out_path = os.path.join(out_dir, filename)
        
        with open(out_path, 'w', encoding='utf-8') as f:
            f.write(c_code)
            
        print(f'Wrote {out_path}')
        
        old_str = match.group(0)
        push_str_match = re.search(r'out\.push_str\(\s*r#"(.*?)"#,[ \t\r\n]*\);', old_str, re.DOTALL)
        if push_str_match:
            to_replace = push_str_match.group(0)
            replacement = f'out.push_str(include_str!("../runtime/{filename}"));'
            content = content.replace(to_replace, replacement)

    if len(func_matches) > 0:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print('c_emitter.rs updated.')

extract_and_replace()

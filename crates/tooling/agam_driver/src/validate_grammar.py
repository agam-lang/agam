import sys
import glob
from pathlib import Path
from lark import Lark, exceptions
from lark.indenter import Indenter

class AgamIndenter(Indenter):
    NL_type = '_NEWLINE'
    OPEN_PAREN_types = ['LPAREN', 'LBRACKET', 'LBRACE']
    CLOSE_PAREN_types = ['RPAREN', 'RBRACKET', 'RBRACE']
    INDENT_type = '_INDENT'
    DEDENT_type = '_DEDENT'
    tab_len = 4

def main():
    grammar_path = Path("docs/specification/grammar.ebnf")
    if not grammar_path.exists():
        print(f"Error: {grammar_path} not found.")
        sys.exit(1)

    with open(grammar_path, 'r', encoding='utf-8') as f:
        grammar_content = f.read()

    try:
        parser = Lark(grammar_content, start='start', parser='lalr', postlex=AgamIndenter())
    except Exception as e:
        print(f"Failed to compile grammar: {e}")
        sys.exit(1)

    files = []
    files.extend(glob.glob("agam/examples/**/*.agam", recursive=True))
    files.extend(glob.glob("benchmarks/suites/**/*.agam", recursive=True))

    if not files:
        print("No .agam files found to validate.")
        sys.exit(0)

    success = True
    for filepath in files:
        with open(filepath, 'r', encoding='utf-8') as f:
            code = f.read()
        
        # skip lines like @lang.advance and other pragmas just for standard parsing, or handle in grammar
        try:
            parser.parse(code + "\n")
            print(f"OK: {filepath}")
        except exceptions.LarkError as e:
            print(f"FAIL: {filepath}")
            print(e)
            success = False
            break

    if not success:
        sys.exit(1)
    
    print(f"All {len(files)} files parsed successfully.")

if __name__ == "__main__":
    main()

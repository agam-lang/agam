import os
import subprocess

BASE_DIR = r"c:\Users\ksvik\IdeaProjects\Agam-Lang"
repos = ['agamlab', 'std', 'registry-index', 'sdk-packs', 'rfcs', 'governance',
         'agam-vscode', 'agam-intellij', 'playground', 'examples', 'benchmarks', 'agam-lang.github.io']

for repo in repos:
    rpath = os.path.join(BASE_DIR, repo)
    if not os.path.exists(rpath): continue
    try:
        subprocess.run(["git", "add", "."], cwd=rpath, check=True)
        subprocess.run(["git", "commit", "-m", "feat: add comprehensive multi-agent system, enriched docs, rules, phases, and skills"], cwd=rpath, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run(["git", "push"], cwd=rpath, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        print(f"Pushed {repo}")
    except Exception as e:
        print(f"Error with {repo}: {e}")

print("All repositories updated and pushed!")

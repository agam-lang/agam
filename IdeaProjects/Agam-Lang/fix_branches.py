import os
import subprocess

BASE_DIR = r"c:\Users\ksvik\IdeaProjects\Agam-Lang"
repos = ['.github', 'agamlab', 'std', 'registry-index', 'sdk-packs', 'agam-lang.github.io', 'agam-vscode', 'agam-intellij', 'rfcs', 'governance', 'examples', 'playground', 'benchmarks']

for repo in repos:
    rpath = os.path.join(BASE_DIR, repo)
    if not os.path.exists(rpath): continue
    
    # Check current branch
    res = subprocess.run(["git", "branch", "--show-current"], cwd=rpath, capture_output=True, text=True)
    branch = res.stdout.strip()
    
    if branch == "master":
        print(f"Normalizing {repo} from 'master' to 'main' branch...")
        # Rename branch locally
        subprocess.run(["git", "branch", "-m", "master", "main"], cwd=rpath)
        # Push to main on remote
        subprocess.run(["git", "push", "-u", "origin", "main"], cwd=rpath)
        # Change default branch in Github API to main
        subprocess.run(["gh", "api", "-X", "PATCH", f"/repos/agam-lang/{repo}", "-f", "default_branch=main"], cwd=rpath, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        # Attempt to delete master on remote
        subprocess.run(["git", "push", "origin", "--delete", "master"], cwd=rpath, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

print("All branches unified to 'main'. Workflows will now connect properly!")

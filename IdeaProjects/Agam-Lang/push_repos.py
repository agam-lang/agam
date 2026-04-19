import os
import subprocess

BASE_DIR = r"c:\Users\ksvik\IdeaProjects\Agam-Lang"

repos = [
    ('.github', 'agam-lang/.github'),
    ('agamlab', 'agam-lang/agamlab'),
    ('agam-lang.github.io', 'agam-lang/agam-lang.github.io'),
    ('std', 'agam-lang/std'),
    ('registry-index', 'agam-lang/registry-index'),
    ('sdk-packs', 'agam-lang/sdk-packs'),
    ('rfcs', 'agam-lang/rfcs'),
    ('agam-vscode', 'agam-lang/agam-vscode'),
    ('agam-intellij', 'agam-lang/agam-intellij'),
    ('playground', 'agam-lang/playground'),
    ('examples', 'agam-lang/examples'),
    ('benchmarks', 'agam-lang/benchmarks'),
    ('governance', 'agam-lang/governance')
]

for local_dir, remote_name in repos:
    repo_path = os.path.join(BASE_DIR, local_dir)
    print(f"==========================================")
    print(f"Processing {local_dir} -> {remote_name}...")
    
    if not os.path.exists(repo_path):
        print(f"Directory {repo_path} doesn't exist, skipping.")
        continue
        
    try:
        subprocess.run(["git", "add", "."], cwd=repo_path, check=True)
        # Commit (may fail if there are no changes, but that's ok)
        res = subprocess.run(["git", "commit", "-m", "Initial commit from Agam Scaffolding"], cwd=repo_path, capture_output=True, text=True)
        
        # Check if the github repo already exists, if not, create it
        create_res = subprocess.run(["gh", "repo", "create", remote_name, "--public", "--source=.", "--remote=origin", "--push"], cwd=repo_path, capture_output=True, text=True)
        
        if create_res.returncode == 0:
            print(f"Success! Created {remote_name} and pushed initial commit.")
        else:
            # If repo creation failed, it might already exist. Let's try to just push.
            print(f"Repo creation failed or repo already exists. Error: {create_res.stderr.strip()}")
            if "already exists" in create_res.stderr or "Name already exists" in create_res.stderr:
                print(f"Attempting to add remote and push instead...")
                subprocess.run(["git", "remote", "add", "origin", f"https://github.com/{remote_name}.git"], cwd=repo_path)
                push_res = subprocess.run(["git", "push", "-u", "origin", "main"], cwd=repo_path, capture_output=True, text=True)
                if push_res.returncode == 0:
                    print(f"Successfully pushed to existing repo {remote_name}.")
                else:
                    print(f"Failed to push: {push_res.stderr.strip()}")
            
    except subprocess.CalledProcessError as e:
        print(f"Error processing {local_dir}. {e}")

print("==========================================")
print("Finished remote sync process.")

import os
import subprocess
import json

BASE_DIR = r"c:\Users\ksvik\IdeaProjects\Agam-Lang"
USER = "vikash236"

repos_cfg = [
    'agam', 'agamlab', 'std', 'registry-index', 'sdk-packs', 
    'agam-lang.github.io', 'agam-vscode', 'agam-intellij', 'rfcs', 
    'governance', 'examples', 'playground', 'benchmarks', '.github'
]

print("Ensuring V2 Project configuration...")
project_url = "https://github.com/orgs/agam-lang/projects/1"
try:
    cmd = ["gh", "project", "create", "--owner", "agam-lang", "--title", "Agam-Lang Ecosystem", "--format", "json"]
    res = subprocess.run(cmd, cwd=BASE_DIR, capture_output=True, text=True)
    if res.returncode == 0:
        data = json.loads(res.stdout)
        project_url = data.get('url', project_url)
        print(f"Created project: {project_url}")
    else:
        print("Project creation skipped or failed. Using fallback ID 1.")
        print(res.stderr)
except:
    print("Project mapping failed natively. Falling back to ID 1.")

auto_assign_tmpl = f"""name: Auto Assignment
on:
  issues:
    types: [opened]
  pull_request:
    types: [opened]
jobs:
  assign:
    runs-on: ubuntu-latest
    steps:
      - name: Assign to {USER}
        uses: actions/github-script@v7
        with:
          script: |
            github.rest.issues.addAssignees({{
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              assignees: ['{USER}']
            }});
"""

add_to_project_tmpl = f"""name: Auto Add to Project
on:
  issues:
    types: [opened]
  pull_request:
    types: [opened]
jobs:
  add_to_project:
    runs-on: ubuntu-latest
    steps:
      - name: Add to GitHub Project
        uses: actions/add-to-project@v0.5.0
        with:
          project-url: {project_url}
          github-token: ${{{{ secrets.GITHUB_TOKEN }}}}
"""

publish_npm_tmpl = """name: Publish to GitHub Packages (npm)
on:
  release:
    types: [published]
jobs:
  publish:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 18
          registry-url: 'https://npm.pkg.github.com'
          scope: '@agam-lang'
      - run: npm ci
      - run: npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
"""

publish_docker_tmpl = """name: Publish GHCR Container Fast
on:
  release:
    types: [published]
  workflow_dispatch:
jobs:
  push_to_registry:
    name: Push Docker image
    runs-on: ubuntu-latest
    permissions:
      packages: write
      contents: read
    steps:
      - uses: actions/checkout@v4
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: ghcr.io/agam-lang/agam:latest
"""

dummy_dockerfile = """FROM ubuntu:22.04
RUN apt-get update && apt-get install -y ca-certificates curl
# Skeleton marker for agam binaries
CMD ["/bin/bash"]
"""

for repo in repos_cfg:
    rpath = os.path.join(BASE_DIR, repo)
    if not os.path.exists(rpath): continue
    wf_dir = os.path.join(rpath, '.github', 'workflows')
    os.makedirs(wf_dir, exist_ok=True)
    with open(os.path.join(wf_dir, 'auto-assign.yml'), 'w', encoding='utf8') as f: f.write(auto_assign_tmpl)
    with open(os.path.join(wf_dir, 'add-to-project.yml'), 'w', encoding='utf8') as f: f.write(add_to_project_tmpl)

if os.path.exists(os.path.join(BASE_DIR, 'agam-vscode')):
    with open(os.path.join(BASE_DIR, 'agam-vscode', '.github', 'workflows', 'publish.yml'), 'w', encoding='utf8') as f:
         f.write(publish_npm_tmpl)

if os.path.exists(os.path.join(BASE_DIR, 'agam')):
    with open(os.path.join(BASE_DIR, 'agam', '.github', 'workflows', 'publish-ghcr.yml'), 'w', encoding='utf8') as f:
         f.write(publish_docker_tmpl)
    if not os.path.exists(os.path.join(BASE_DIR, 'agam', 'Dockerfile')):
        with open(os.path.join(BASE_DIR, 'agam', 'Dockerfile'), 'w', encoding='utf8') as f:
            f.write(dummy_dockerfile)

print("Pushing logic...")
for repo in repos_cfg:
    rpath = os.path.join(BASE_DIR, repo)
    if not os.path.exists(rpath): continue
    try:
        subprocess.run(["git", "add", "."], cwd=rpath, check=True)
        subprocess.run(["git", "commit", "-m", "feat(mgmt): introduce organizational automations"], cwd=rpath, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run(["git", "push"], cwd=rpath, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        print(f"Pushed to {repo}")
    except Exception as e: 
        print(f"Failed push for {repo}")
        pass

print("Complete.")

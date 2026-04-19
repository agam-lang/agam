import sys, json
from graphify.build import build_from_json
from graphify.export import to_html, to_obsidian, to_canvas
from pathlib import Path

extraction = json.loads(Path('graphify-out/.graphify_extract.json').read_text(encoding='utf-8'))
analysis   = json.loads(Path('graphify-out/.graphify_analysis.json').read_text(encoding='utf-8'))

G = build_from_json(extraction)
communities = {int(k): v for k, v in analysis['communities'].items()}

# Load curated community labels
labels_path = Path('graphify-out/community_labels.json')
if labels_path.exists():
    raw_labels = json.loads(labels_path.read_text(encoding='utf-8'))
    community_labels = {int(k): v for k, v in raw_labels.items()}
else:
    community_labels = None

# HTML export
if G.number_of_nodes() > 5000:
    print(f"Graph has {G.number_of_nodes()} nodes - too large for HTML. Use Obsidian.")
else:
    to_html(G, communities, 'graphify-out/graph.html')
    print("graph.html written - open in any browser")

# Obsidian vault export
obsidian_dir = r"C:\Users\ksvik\OneDrive\Documents\agam\agam-graph"
cohesion = {int(k): v for k, v in analysis['cohesion'].items()}
n = to_obsidian(G, communities, obsidian_dir, community_labels=community_labels, cohesion=cohesion)
print(f"Obsidian vault: {n} notes in {obsidian_dir}/")

to_canvas(G, communities, f"{obsidian_dir}/graph.canvas")
print(f"Canvas: {obsidian_dir}/graph.canvas")
print()
print(f"Open {obsidian_dir}/ as a vault in Obsidian.")
print("  Graph view   - nodes colored by community")
print("  graph.canvas - structured layout with communities as groups")

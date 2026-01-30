#!/usr/bin/env python3
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
LABELS_MD = REPO / "docs/project/labels.md"
MILESTONES_MD = REPO / "docs/project/milestones.md"
BACKLOG_MD = REPO / "docs/project/issue-backlog.md"

def run(cmd: list[str]) -> str:
    r = subprocess.run(cmd, cwd=REPO, text=True, capture_output=True)
    if r.returncode != 0:
        print("CMD FAILED:", " ".join(cmd))
        print(r.stdout)
        print(r.stderr)
        raise SystemExit(r.returncode)
    return r.stdout.strip()

def gh(*args: str) -> str:
    return run(["gh", *args])

def ensure_auth():
    try:
        gh("auth", "status")
    except SystemExit:
        print("ERROR: gh not authenticated. Run: gh auth login")
        raise

def label_color(name: str) -> str:
    # simple stable palette (purple-ish leaning)
    if name.startswith("tier/"):
        return "6F42C1"  # purple
    if name.startswith("prio/"):
        return "B07219"  # orange/brown
    if name.startswith("type/"):
        return "0E8A16"  # green
    if name.startswith("area/"):
        return "5319E7"  # deep purple
    return "D4C5F9"      # light purple

def parse_labels(md: Path) -> list[str]:
    text = md.read_text(encoding="utf-8")
    labels = re.findall(r"`([^`]+)`", text)
    # Keep only actual labels, not milestone names from the bottom section
    # (milestones in that file are also in backticks; we filter by known prefixes)
    keep = []
    for x in labels:
        if any(x.startswith(p) for p in ("type/", "tier/", "prio/", "area/")):
            keep.append(x)
    return sorted(set(keep))

def create_labels():
    labels = parse_labels(LABELS_MD)
    existing = gh("label", "list").splitlines()
    existing_names = set()
    for line in existing:
        # gh label list output begins with name
        existing_names.add(line.split("\t")[0].strip())

    for name in labels:
        if name in existing_names:
            continue
        color = label_color(name)
        # Description kept short
        desc = "Auto-generated label"
        gh("label", "create", name, "--color", color, "--description", desc)
        print(f"Created label: {name}")

def parse_milestones(md: Path) -> list[tuple[str, str]]:
    text = md.read_text(encoding="utf-8").splitlines()
    milestones = []
    cur_name = None
    cur_desc = []
    for line in text:
        m = re.match(r"^##\s+(.+?)\s+—\s+(.+)$", line)
        if m:
            if cur_name:
                milestones.append((cur_name, "\n".join(cur_desc).strip()))
            cur_name = m.group(1).strip()
            cur_desc = [m.group(2).strip()]
            continue
        if cur_name:
            cur_desc.append(line)
    if cur_name:
        milestones.append((cur_name, "\n".join(cur_desc).strip()))
    return milestones

def create_milestones():
    existing = gh("api", "repos/{owner}/{repo}/milestones?state=all")
    # cheap check: milestone titles in json
    existing_titles = set(re.findall(r'"title"\s*:\s*"([^"]+)"', existing))

    for title, desc in parse_milestones(MILESTONES_MD):
        if title in existing_titles:
            continue
        gh("api", "-X", "POST", "repos/{owner}/{repo}/milestones",
           "-f", f"title={title}",
           "-f", f"description={desc}")
        print(f"Created milestone: {title}")

def parse_issues(md: Path) -> list[dict]:
    text = md.read_text(encoding="utf-8")
    # Each issue starts with a "### Title"
    # We capture labels and milestone lines beneath it.
    issues = []
    pattern = re.compile(
        r"###\s+(?P<title>.+?)\n"
        r"\*\*Labels:\*\*\s+(?P<labels>.+?)\n"
        r"\*\*Milestone:\*\*\s+(?P<milestone>.+?)\n\n"
        r"(?P<body>.*?)(?=\n---\n|\Z)",
        re.S
    )
    for m in pattern.finditer(text):
        title = m.group("title").strip()
        labels = [x.strip(" `") for x in m.group("labels").split(",")]
        milestone = m.group("milestone").strip()
        body = m.group("body").strip()
        issues.append({
            "title": title,
            "labels": labels,
            "milestone": milestone,
            "body": body,
        })
    return issues

def create_issues(dry_run: bool = False):
    # Avoid duplicates by checking existing issue titles
    existing = gh("issue", "list", "--state", "all", "--limit", "500", "--json", "title")
    existing_titles = set(re.findall(r'"title"\s*:\s*"([^"]+)"', existing))

    for it in parse_issues(BACKLOG_MD):
        if it["title"] in existing_titles:
            print(f"Skip (exists): {it['title']}")
            continue

        cmd = [
            "issue", "create",
            "--title", it["title"],
            "--body", it["body"],
            "--milestone", it["milestone"],
        ]
        for lab in it["labels"]:
            cmd += ["--label", lab]

        if dry_run:
            print("DRY RUN:", "gh " + " ".join(cmd))
        else:
            gh(*cmd)
            print(f"Created issue: {it['title']}")

def main():
    ensure_auth()

    mode = sys.argv[1] if len(sys.argv) > 1 else "all"
    dry = "--dry-run" in sys.argv

    if mode in ("labels", "all"):
        create_labels()
    if mode in ("milestones", "all"):
        create_milestones()
    if mode in ("issues", "all"):
        create_issues(dry_run=dry)

    print("✅ Done.")

if __name__ == "__main__":
    main()

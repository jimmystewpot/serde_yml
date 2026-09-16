# Implementation Plan: Harden GitHub Actions Workflow Permissions (Single PR)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve all 10 GitHub CodeQL code-scanning security alerts (`actions/missing-workflow-permissions`, CWE-275) by establishing explicit, least-privilege `permissions` blocks across all 8 repository workflow files, validated and delivered in a single Pull Request.

**Architecture:** Apply top-level `permissions: contents: read` as the baseline default across all 8 workflow files. Explicitly grant elevated permissions at the job level only where operations require it (`document.yml` job `all` requires `contents: write` for GitHub Pages deployment; `release.yml` job `release` requires `contents: write` for creating GitHub releases and `actions: read` for downloading artifacts via GitHub API). Validate YAML parsing and permission schemas programmatically before creating the PR.

**Tech Stack:** GitHub Actions, YAML, Python 3 / PyYAML (for validation), GitHub CLI (`gh`).

**Spec:** [Analysis & Security Recommendations](https://github.com/jimmystewpot/serde_yml/security/code-scanning) (from the brainstorming phase).

---

## Global Constraints

- **Least Privilege:** Every workflow must declare top-level `permissions: contents: read` to restrict the default `GITHUB_TOKEN` scope.
- **Job-Level Scoping:** Elevate permissions only on jobs that write to GitHub repository resources (`document.yml` job `all`, `release.yml` job `release`).
- **Zero Breakage:** Ensure `peaceiris/actions-gh-pages` and `actions/create-release` retain the necessary write access to publish docs and releases when triggered on `master`.
- **Clean Deliverable:** All changes must be cleanly packaged on a dedicated branch (`fix/ci-workflow-permissions`) and submitted via a single Pull Request targeting `master`.
- **YAML Validation:** Every workflow file must pass strict YAML syntax validation and schema consistency checks.

---

## User Review Required

> [!NOTE]
> All 10 alerts in GitHub Code Scanning are instances of `actions/missing-workflow-permissions`. Fixing these 8 workflow files will completely eliminate all active security alerts on the repository.

> [!IMPORTANT]
> The single PR will branch from `origin/master`, implement the updates, validate the syntax, and open a single Pull Request to `master`.

---

## Proposed Changes

### CI / CD Workflows

#### [MODIFY] [audit.yml](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/audit.yml)
- Add top-level `permissions: contents: read`.

#### [MODIFY] [check.yml](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/check.yml)
- Add top-level `permissions: contents: read`.

#### [MODIFY] [coverage.yml](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/coverage.yml)
- Add top-level `permissions: contents: read`.

#### [MODIFY] [document.yml](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/document.yml)
- Add top-level `permissions: contents: read`.
- Add job-level `permissions: contents: write` to job `all` (for `gh-pages` branch push).

#### [MODIFY] [format.yml](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/format.yml)
- Add top-level `permissions: contents: read`.

#### [MODIFY] [lint.yml](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/lint.yml)
- Add top-level `permissions: contents: read`.

#### [MODIFY] [release.yml](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/release.yml)
- Add top-level `permissions: contents: read`.
- Add job-level `permissions: contents: read` to job `build`.
- Add job-level `permissions: contents: write` and `actions: read` to job `release` (for artifact download & release creation).
- Add job-level `permissions: contents: read` to job `crate`.

#### [MODIFY] [test.yml](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/test.yml)
- Add top-level `permissions: contents: read`.

---

## Tasks

### Task 1: Branch Setup & Baseline Verification

**Files:**
- None (git branch management)

**Interfaces:**
- Consumes: `origin/master` (commit `e021082`)
- Produces: Clean working branch `fix/ci-workflow-permissions` tracked against `origin/master`

- [ ] **Step 1: Check out a new branch `fix/ci-workflow-permissions` from `origin/master`**
```bash
git checkout -B fix/ci-workflow-permissions origin/master
```

- [ ] **Step 2: Verify git status is clean and points to origin/master**
```bash
git status
```

---

### Task 2: Harden Read-Only Workflows

**Files:**
- Modify: [`.github/workflows/audit.yml:1-18`](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/audit.yml)
- Modify: [`.github/workflows/check.yml:1-18`](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/check.yml)
- Modify: [`.github/workflows/coverage.yml:1-15`](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/coverage.yml)
- Modify: [`.github/workflows/format.yml:1-18`](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/format.yml)
- Modify: [`.github/workflows/lint.yml:1-18`](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/lint.yml)
- Modify: [`.github/workflows/test.yml:1-7`](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/test.yml)

**Interfaces:**
- Consumes: Existing read-only workflows
- Produces: Workflows declaring explicit top-level `permissions: contents: read`

- [ ] **Step 1: Add `permissions: contents: read` to `.github/workflows/audit.yml`**
- [ ] **Step 2: Add `permissions: contents: read` to `.github/workflows/check.yml`**
- [ ] **Step 3: Add `permissions: contents: read` to `.github/workflows/coverage.yml`**
- [ ] **Step 4: Add `permissions: contents: read` to `.github/workflows/format.yml`**
- [ ] **Step 5: Add `permissions: contents: read` to `.github/workflows/lint.yml`**
- [ ] **Step 6: Add `permissions: contents: read` to `.github/workflows/test.yml`**
- [ ] **Step 7: Verify YAML validity with Python parser**
```bash
python3 -c "
import yaml, glob
for path in ['audit.yml', 'check.yml', 'coverage.yml', 'format.yml', 'lint.yml', 'test.yml']:
    data = yaml.safe_load(open(f'.github/workflows/{path}'))
    assert data.get('permissions') == {'contents': 'read'}, f'Failed on {path}: {data.get(\"permissions\")}'
print('All 6 read-only workflows validated successfully')
"
```

---

### Task 3: Harden `document.yml` with Deployment Scoping

**Files:**
- Modify: [`.github/workflows/document.yml:1-25`](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/document.yml)

**Interfaces:**
- Consumes: Existing `document.yml`
- Produces: Top-level `contents: read` and job `all` `contents: write` for `gh-pages` deploy

- [ ] **Step 1: Add top-level `permissions: contents: read`**
- [ ] **Step 2: Add job-level `permissions: contents: write` under `jobs.all`**
- [ ] **Step 3: Verify YAML syntax and permissions structure**
```bash
python3 -c "
import yaml
data = yaml.safe_load(open('.github/workflows/document.yml'))
assert data.get('permissions') == {'contents': 'read'}, 'Top-level permission mismatch'
assert data['jobs']['all']['permissions'] == {'contents': 'write'}, 'Job permission mismatch'
print('document.yml validated successfully')
"
```

---

### Task 4: Harden `release.yml` with Multi-Job Scoping

**Files:**
- Modify: [`.github/workflows/release.yml`](file:///home/jalamb/go/src/github.com/jimmystewpot/serde_yml/.github/workflows/release.yml)

**Interfaces:**
- Consumes: Existing `release.yml`
- Produces: Top-level `contents: read`, job `build` `contents: read`, job `release` `contents: write` + `actions: read`, job `crate` `contents: read`

- [ ] **Step 1: Add top-level `permissions: contents: read`**
- [ ] **Step 2: Add `permissions: contents: read` to job `build`**
- [ ] **Step 3: Add `permissions: contents: write` and `actions: read` to job `release`**
- [ ] **Step 4: Add `permissions: contents: read` to job `crate`**
- [ ] **Step 5: Verify YAML syntax and permissions structure**
```bash
python3 -c "
import yaml
data = yaml.safe_load(open('.github/workflows/release.yml'))
assert data.get('permissions') == {'contents': 'read'}, 'Top-level permission mismatch'
assert data['jobs']['build']['permissions'] == {'contents': 'read'}, 'Build job permission mismatch'
assert data['jobs']['release']['permissions'] == {'contents': 'write', 'actions': 'read'}, 'Release job permission mismatch'
assert data['jobs']['crate']['permissions'] == {'contents': 'read'}, 'Crate job permission mismatch'
print('release.yml validated successfully')
"
```

---

### Task 5: Automated Verification, Commit & PR Creation

**Files:**
- All 8 workflows in `.github/workflows/`

**Interfaces:**
- Consumes: All updated workflow files
- Produces: Committed changes, pushed branch `fix/ci-workflow-permissions`, and open Pull Request targeting `master`

- [ ] **Step 1: Run comprehensive validation across all 8 workflows**
```bash
python3 -c "
import yaml, glob
workflows = glob.glob('.github/workflows/*.yml')
assert len(workflows) == 8, f'Expected 8 workflows, found {len(workflows)}'
for w in workflows:
    with open(w) as f:
        doc = yaml.safe_load(f)
    assert 'permissions' in doc or any('permissions' in j for j in doc.get('jobs', {}).values()), f'{w} missing permissions'
    print(f'✓ {w}: valid')
print('All workflows pass verification.')
"
```

- [ ] **Step 2: Review `git diff` for indentation, styling, and accuracy**
```bash
git diff
```

- [ ] **Step 3: Commit the changes with conventional commit message**
```bash
git add .github/workflows/*.yml docs/superpowers/plans/2026-09-17-ci-workflow-permissions.md
git commit -m "fix(ci): define explicit least-privilege permissions across all workflows

Fixes all 10 GitHub CodeQL code scanning alerts (actions/missing-workflow-permissions, CWE-275).
- Adds top-level 'contents: read' to all 8 workflow files.
- Grants 'contents: write' to document.yml for GitHub Pages deployment.
- Grants 'contents: write' and 'actions: read' to release.yml release job for release creation and artifact download.
- Grants 'contents: read' to release build and crate jobs."
```

- [ ] **Step 4: Push branch to origin**
```bash
git push -u origin fix/ci-workflow-permissions
```

- [ ] **Step 5: Create a single Pull Request via `gh pr create`**
```bash
gh pr create --base master --head fix/ci-workflow-permissions \
  --title "fix(ci): define explicit least-privilege permissions across all workflows" \
  --body "## Summary
Resolves all 10 GitHub Code Scanning alerts (\`actions/missing-workflow-permissions\`, CWE-275).

### Changes
- Configured default top-level \`permissions: contents: read\` across all 8 workflow files in \`.github/workflows/\`.
- Explicitly elevated permissions on jobs requiring write capabilities:
  - \`.github/workflows/document.yml\` (\`all\` job): \`contents: write\` (required by \`peaceiris/actions-gh-pages\` for pushing to \`gh-pages\`).
  - \`.github/workflows/release.yml\` (\`release\` job): \`contents: write\` and \`actions: read\` (required by \`actions/create-release\` and artifact download via GitHub API).
  - \`.github/workflows/release.yml\` (\`build\` & \`crate\` jobs): \`contents: read\`.

### Verification
- Validated YAML syntax and AST structure across all 8 workflows via PyYAML.
- Verified least-privilege scoping against CodeQL \`actions/missing-workflow-permissions\` criteria."
```

---

## Verification Plan

### Automated Verification
1. **PyYAML Validation**: Parse all 8 `.github/workflows/*.yml` files to ensure zero syntax errors and verify correct `permissions` keys.
2. **CodeQL Alignment**: Ensure all 10 alert locations now have explicit `permissions` defined at either the workflow or job level.
3. **Git Status & Cleanliness**: Ensure only the intended workflow files (and plan doc) are modified.

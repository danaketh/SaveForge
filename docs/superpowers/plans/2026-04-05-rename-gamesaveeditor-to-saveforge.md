# Rename SaveForge → SaveForge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename all `SaveForge` occurrences to `SaveForge` across the entire solution — namespaces, project names, folder names, and UI strings.

**Architecture:** Three-phase approach: (1) content replacement in all source/config files while folders stay put, (2) rename .csproj files in place, (3) rename the project directories and update the solution file accordingly. Build artifacts (bin/, obj/) are skipped — they regenerate on next build.

**Tech Stack:** .NET 8, Avalonia UI, C#, MSBuild solution/project files

---

## File Map

### Files modified by content (text replacement `SaveForge` → `SaveForge`):
- `src/SaveForge.App/**/*.cs` (21 files)
- `src/SaveForge.App/**/*.axaml` (8 files — includes UI text strings)
- `src/SaveForge.Core/**/*.cs` (12 files)
- `src/SaveForge.Forms/**/*.cs` + `.axaml` (multiple files)
- `src/SaveForge.Infrastructure/**/*.cs`
- `src/SaveForge.PluginHost/**/*.cs`
- `src/SaveForge.SDK/**/*.cs`
- `src/SaveForge.SamplePlugin.StardewLike/**/*.cs` + `manifest.json`
- `src/SaveForge.Security/**/*.cs`
- `src/SaveForge.Store/**/*.cs`
- `tests/SaveForge.Tests/**/*.cs`
- All 10 `.csproj` files (RootNamespace, ProjectReference paths)
- `SaveForge.sln` (project names and relative paths)
- `docs/ARCHITECTURE.md`, `docs/PLUGIN_AUTHORING.md`, `README.md`
- `docs/superpowers/plans/2026-04-04-gamesaveeditor.md`
- `docs/superpowers/specs/2026-04-04-gamesaveeditor-design.md`

### Files renamed:
- `src/SaveForge.App/SaveForge.App.csproj` → `SaveForge.App.csproj`
- `src/SaveForge.Core/SaveForge.Core.csproj` → `SaveForge.Core.csproj`
- `src/SaveForge.Forms/SaveForge.Forms.csproj` → `SaveForge.Forms.csproj`
- `src/SaveForge.Infrastructure/SaveForge.Infrastructure.csproj` → `SaveForge.Infrastructure.csproj`
- `src/SaveForge.PluginHost/SaveForge.PluginHost.csproj` → `SaveForge.PluginHost.csproj`
- `src/SaveForge.SDK/SaveForge.SDK.csproj` → `SaveForge.SDK.csproj`
- `src/SaveForge.SamplePlugin.StardewLike/SaveForge.SamplePlugin.StardewLike.csproj` → `SaveForge.SamplePlugin.StardewLike.csproj`
- `src/SaveForge.Security/SaveForge.Security.csproj` → `SaveForge.Security.csproj`
- `src/SaveForge.Store/SaveForge.Store.csproj` → `SaveForge.Store.csproj`
- `tests/SaveForge.Tests/SaveForge.Tests.csproj` → `SaveForge.Tests.csproj`

### Directories renamed:
- `src/SaveForge.App/` → `src/SaveForge.App/`
- `src/SaveForge.Core/` → `src/SaveForge.Core/`
- `src/SaveForge.Forms/` → `src/SaveForge.Forms/`
- `src/SaveForge.Infrastructure/` → `src/SaveForge.Infrastructure/`
- `src/SaveForge.PluginHost/` → `src/SaveForge.PluginHost/`
- `src/SaveForge.SDK/` → `src/SaveForge.SDK/`
- `src/SaveForge.SamplePlugin.StardewLike/` → `src/SaveForge.SamplePlugin.StardewLike/`
- `src/SaveForge.Security/` → `src/SaveForge.Security/`
- `src/SaveForge.Store/` → `src/SaveForge.Store/`
- `tests/SaveForge.Tests/` → `tests/SaveForge.Tests/`

---

## Task 1: Replace all SaveForge references in source files

**Files:** All `.cs` and `.axaml` files under `src/` and `tests/`, excluding `bin/` and `obj/` directories.

- [ ] **Step 1: Run global content replacement in .cs files**

```bash
find /I/Projects/danaketh/SaveForge/src /I/Projects/danaketh/SaveForge/tests \
  -name "*.cs" \
  -not -path "*/bin/*" \
  -not -path "*/obj/*" \
  -exec sed -i 's/SaveForge/SaveForge/g' {} +
```

Expected: No errors. Command exits silently (or with file count).

- [ ] **Step 2: Run global content replacement in .axaml files**

```bash
find /I/Projects/danaketh/SaveForge/src /I/Projects/danaketh/SaveForge/tests \
  -name "*.axaml" \
  -not -path "*/bin/*" \
  -not -path "*/obj/*" \
  -exec sed -i 's/SaveForge/SaveForge/g' {} +
```

Expected: No errors. This also renames the UI text strings `Text="SaveForge"` → `Text="SaveForge"` and `Title="SaveForge"` → `Title="SaveForge"`, which is correct.

- [ ] **Step 3: Verify replacements in a sample file**

```bash
grep -n "SaveForge" /I/Projects/danaketh/SaveForge/src/SaveForge.App/App.axaml.cs
```

Expected: No output (no remaining matches).

```bash
grep -n "SaveForge" /I/Projects/danaketh/SaveForge/src/SaveForge.App/App.axaml.cs | head -5
```

Expected: Lines showing `namespace SaveForge.App` and `using SaveForge.*`.

- [ ] **Step 4: Commit progress checkpoint**

```bash
cd /I/Projects/danaketh/SaveForge
git add -u
git commit -m "refactor: replace SaveForge namespace/references with SaveForge in all source files"
```

---

## Task 2: Replace SaveForge references in project and solution files

**Files:** All `.csproj` files and `SaveForge.sln`.

- [ ] **Step 1: Replace in all .csproj files**

```bash
find /I/Projects/danaketh/SaveForge/src /I/Projects/danaketh/SaveForge/tests \
  -name "*.csproj" \
  -exec sed -i 's/SaveForge/SaveForge/g' {} +
```

Expected: No errors. This updates `<RootNamespace>`, `<AssemblyName>` (if present), and `<ProjectReference Include=...>` paths within each .csproj.

- [ ] **Step 2: Replace in the solution file**

```bash
sed -i 's/SaveForge/SaveForge/g' /I/Projects/danaketh/SaveForge/SaveForge.sln
```

Expected: No errors. This updates project names, paths (e.g., `src\SaveForge.SDK\SaveForge.SDK.csproj` → `src\SaveForge.SDK\SaveForge.SDK.csproj`), and display names in the solution.

- [ ] **Step 3: Replace in manifest.json**

```bash
sed -i 's/SaveForge/SaveForge/g' /I/Projects/danaketh/SaveForge/src/SaveForge.SamplePlugin.StardewLike/manifest.json
```

Expected: `"pluginAssembly"` becomes `"SaveForge.SamplePlugin.StardewLike.dll"` and `"author"` becomes `"SaveForge Team"`.

- [ ] **Step 4: Replace in documentation files**

```bash
sed -i 's/SaveForge/SaveForge/g' \
  /I/Projects/danaketh/SaveForge/README.md \
  /I/Projects/danaketh/SaveForge/docs/ARCHITECTURE.md \
  /I/Projects/danaketh/SaveForge/docs/PLUGIN_AUTHORING.md \
  /I/Projects/danaketh/SaveForge/docs/superpowers/plans/2026-04-04-gamesaveeditor.md \
  /I/Projects/danaketh/SaveForge/docs/superpowers/specs/2026-04-04-gamesaveeditor-design.md
```

Expected: No errors.

- [ ] **Step 5: Verify no remaining SaveForge references in tracked source**

```bash
grep -r "SaveForge" /I/Projects/danaketh/SaveForge \
  --include="*.cs" --include="*.axaml" --include="*.csproj" --include="*.sln" \
  --include="*.json" --include="*.md" \
  --exclude-dir=bin --exclude-dir=obj \
  --exclude-dir=.git \
  -l
```

Expected: No output (empty result). If any files appear, inspect and fix them manually before continuing.

- [ ] **Step 6: Commit**

```bash
cd /I/Projects/danaketh/SaveForge
git add -u
git commit -m "refactor: replace SaveForge with SaveForge in project/solution/manifest/docs files"
```

---

## Task 3: Rename .csproj files to match new project names

**Files:** The 10 `.csproj` files that still have `SaveForge` in their filename.

- [ ] **Step 1: Rename all .csproj files**

```bash
cd /I/Projects/danaketh/SaveForge

mv src/SaveForge.App/SaveForge.App.csproj                                         src/SaveForge.App/SaveForge.App.csproj
mv src/SaveForge.Core/SaveForge.Core.csproj                                       src/SaveForge.Core/SaveForge.Core.csproj
mv src/SaveForge.Forms/SaveForge.Forms.csproj                                     src/SaveForge.Forms/SaveForge.Forms.csproj
mv src/SaveForge.Infrastructure/SaveForge.Infrastructure.csproj                   src/SaveForge.Infrastructure/SaveForge.Infrastructure.csproj
mv src/SaveForge.PluginHost/SaveForge.PluginHost.csproj                           src/SaveForge.PluginHost/SaveForge.PluginHost.csproj
mv src/SaveForge.SDK/SaveForge.SDK.csproj                                         src/SaveForge.SDK/SaveForge.SDK.csproj
mv "src/SaveForge.SamplePlugin.StardewLike/SaveForge.SamplePlugin.StardewLike.csproj" "src/SaveForge.SamplePlugin.StardewLike/SaveForge.SamplePlugin.StardewLike.csproj"
mv src/SaveForge.Security/SaveForge.Security.csproj                               src/SaveForge.Security/SaveForge.Security.csproj
mv src/SaveForge.Store/SaveForge.Store.csproj                                     src/SaveForge.Store/SaveForge.Store.csproj
mv tests/SaveForge.Tests/SaveForge.Tests.csproj                                   tests/SaveForge.Tests/SaveForge.Tests.csproj
```

Expected: Each mv completes silently. No "No such file" errors.

- [ ] **Step 2: Verify new filenames exist**

```bash
find /I/Projects/danaketh/SaveForge/src /I/Projects/danaketh/SaveForge/tests -name "SaveForge.*.csproj" | sort
```

Expected: 10 lines, one per project:
```
.../src/SaveForge.App/SaveForge.App.csproj
.../src/SaveForge.Core/SaveForge.Core.csproj
.../src/SaveForge.Forms/SaveForge.Forms.csproj
.../src/SaveForge.Infrastructure/SaveForge.Infrastructure.csproj
.../src/SaveForge.PluginHost/SaveForge.PluginHost.csproj
.../src/SaveForge.SDK/SaveForge.SDK.csproj
.../src/SaveForge.SamplePlugin.StardewLike/SaveForge.SamplePlugin.StardewLike.csproj
.../src/SaveForge.Security/SaveForge.Security.csproj
.../src/SaveForge.Store/SaveForge.Store.csproj
.../tests/SaveForge.Tests/SaveForge.Tests.csproj
```

- [ ] **Step 3: Verify no old .csproj files remain**

```bash
find /I/Projects/danaketh/SaveForge/src /I/Projects/danaketh/SaveForge/tests -name "SaveForge.*.csproj"
```

Expected: No output.

- [ ] **Step 4: Stage renames and commit**

```bash
cd /I/Projects/danaketh/SaveForge
git add -A
git commit -m "refactor: rename .csproj files from SaveForge.* to SaveForge.*"
```

---

## Task 4: Rename project directories

**Directories:** The 10 project folders under `src/` and `tests/` that still carry the `SaveForge` prefix.

> **Important:** Do these one at a time. The solution file already has the new paths from Task 2 (e.g., `src\SaveForge.SDK\SaveForge.SDK.csproj`), so once directories are renamed the solution will correctly resolve.

- [ ] **Step 1: Rename all project directories**

```bash
cd /I/Projects/danaketh/SaveForge

mv src/SaveForge.App                       src/SaveForge.App
mv src/SaveForge.Core                      src/SaveForge.Core
mv src/SaveForge.Forms                     src/SaveForge.Forms
mv src/SaveForge.Infrastructure            src/SaveForge.Infrastructure
mv src/SaveForge.PluginHost                src/SaveForge.PluginHost
mv src/SaveForge.SDK                       src/SaveForge.SDK
mv "src/SaveForge.SamplePlugin.StardewLike" "src/SaveForge.SamplePlugin.StardewLike"
mv src/SaveForge.Security                  src/SaveForge.Security
mv src/SaveForge.Store                     src/SaveForge.Store
mv tests/SaveForge.Tests                   tests/SaveForge.Tests
```

Expected: Each mv completes silently.

- [ ] **Step 2: Verify directory structure**

```bash
ls /I/Projects/danaketh/SaveForge/src/ && ls /I/Projects/danaketh/SaveForge/tests/
```

Expected:
```
SaveForge.App
SaveForge.Core
SaveForge.Forms
SaveForge.Infrastructure
SaveForge.PluginHost
SaveForge.SDK
SaveForge.SamplePlugin.StardewLike
SaveForge.Security
SaveForge.Store

SaveForge.Tests
```

- [ ] **Step 3: Verify solution file paths match new directory names**

```bash
grep "csproj" /I/Projects/danaketh/SaveForge/SaveForge.sln
```

Expected: All paths use `SaveForge.*` (no `SaveForge` anywhere).

- [ ] **Step 4: Stage and commit**

```bash
cd /I/Projects/danaketh/SaveForge
git add -A
git commit -m "refactor: rename project directories from SaveForge.* to SaveForge.*"
```

---

## Task 5: Verify the build and do a final cleanup check

- [ ] **Step 1: Restore and build the solution**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet restore SaveForge.sln 2>&1 | tail -5
```

Expected: `Restore complete.` or similar success message. No errors about missing projects or invalid paths.

- [ ] **Step 2: Build**

```bash
dotnet build SaveForge.sln --no-restore 2>&1 | tail -10
```

Expected: `Build succeeded.` with 0 errors. Warnings are acceptable.

- [ ] **Step 3: Run tests**

```bash
dotnet test SaveForge.sln --no-build 2>&1 | tail -10
```

Expected: All tests pass. No test failures.

- [ ] **Step 4: Final SaveForge scan across entire repo (excluding bin/obj/git)**

```bash
grep -r "SaveForge" /I/Projects/danaketh/SaveForge \
  --exclude-dir=bin --exclude-dir=obj --exclude-dir=.git \
  -l
```

Expected: No output. If any file appears (e.g., a leftover in an IDE config), inspect and fix it.

- [ ] **Step 5: Final commit**

```bash
cd /I/Projects/danaketh/SaveForge
git add -A
git commit -m "refactor: complete rename from SaveForge to SaveForge — build verified"
```

---

## Rollback procedure

If anything goes wrong before Task 4 is committed:

```bash
cd /I/Projects/danaketh/SaveForge
git reset --hard HEAD
```

After Task 4 is committed, revert all 4 commits:

```bash
git revert HEAD~3..HEAD
```

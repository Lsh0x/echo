# Initialize flowmates workflow and rename rbase to echo

**Type:** enhancement  
**Status:** done  
**Branch:** chore/init-flowmates-workflow-rename-rbase  
**Linked roadmap section:** setup

---

## 🧠 Context
The repository is currently using a template structure with the name "rbase" (from the Rust template). The project is actually called "echo" and needs to be properly initialized with the flowmates workflow system. The flowmates workflow has been partially set up (directories and templates copied), but the codebase still contains references to "rbase" that need to be changed to "echo".

Current state:
- Flowmates workflow structure has been created (issues/echo/* directories, templates, scripts)
- Git hooks have been installed
- `.gitignore` has been updated
- However, the codebase still references "rbase" in multiple places:
  - `Cargo.toml`: package name is "rbase"
  - `README.md`: title and badges reference "rbase"
  - `CHANGELOG.md`: references to "rbase" repository URLs
  - Various other references throughout the codebase

## 🎯 Goal
1. Complete the initialization of the flowmates workflow system in the repository
2. Rename all references from "rbase" to "echo" throughout the codebase to match the actual project name
3. Ensure the repository is properly configured for the flowmates workflow

## 📏 Success Metrics
- [ ] All "rbase" references changed to "echo" in codebase
- [ ] Cargo.toml package name is "echo"
- [ ] README.md updated with correct project name and badges
- [ ] CHANGELOG.md updated with correct repository references
- [ ] Flowmates workflow structure validated and complete
- [ ] Repository ready for flowmates workflow usage

## 🧩 Acceptance Criteria
- [ ] No "rbase" references remain in the codebase (verified by grep)
- [ ] `Cargo.toml` package name is "echo"
- [ ] `README.md` title and all badges reference "echo" instead of "rbase"
- [ ] `CHANGELOG.md` repository URLs updated to reference "echo"
- [ ] All code compiles successfully with new package name
- [ ] Tests pass with new package name
- [ ] Documentation is consistent with "echo" naming
- [ ] Flowmates workflow directories and files are properly set up
- [ ] Git hooks are functional

## 🛠️ Implementation Outline
1. Create/switch to branch `chore/init-flowmates-workflow-rename-rbase`
2. Search for all occurrences of "rbase" in the codebase (case-insensitive)
3. Update `Cargo.toml`:
   - Change package name from "rbase" to "echo"
   - Update description if needed
4. Update `README.md`:
   - Change title from "rbase" to "echo"
   - Update all badge URLs to reference "echo" repository
   - Update any other references to "rbase"
5. Update `CHANGELOG.md`:
   - Update repository URLs from "rbase" to "echo"
6. Search for any other files that might reference "rbase":
   - Check for any configuration files
   - Check for any documentation files
   - Check for any test files
7. Verify flowmates workflow setup:
   - Confirm all directories exist (issues/echo/*, issues/shared/templates)
   - Confirm templates are present
   - Confirm scripts are present
   - Confirm git hooks are installed
8. Run tests to ensure everything still works
9. Build the project to verify compilation
10. Move this file to `in_progress/` then `done/`
11. Create PR referencing this issue

## 🔍 Alternatives Considered
- Keep "rbase" name → Not appropriate, project is called "echo"
- Only rename package name → Incomplete, would leave inconsistencies in documentation
- Skip flowmates workflow initialization → Would leave repository without proper workflow structure

## ⚠️ Risks / Mitigations
- Risk: Breaking changes if package name changes affect dependencies → Mitigation: This is a new project, no external dependencies yet
- Risk: Git history references "rbase" → Mitigation: Acceptable, only affects new commits going forward
- Risk: CI/CD might reference old name → Mitigation: Check and update any CI/CD configuration files
- Risk: Documentation inconsistencies → Mitigation: Comprehensive search and replace, verify with grep

## 🔗 Discussion Notes
- Flowmates workflow initialization was already started (directories created, templates copied, hooks installed)
- This issue completes the setup and ensures consistency with the "echo" project name
- The rename should be comprehensive to avoid confusion


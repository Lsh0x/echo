# Initialize Repository

**Type:** feature  
**Status:** in_progress  
**Branch:** feat/init-repo  
**Linked roadmap section:** setup

---

## 🧠 Context
The repository was created from the rbase template and still contains template references. The project needs to be properly initialized with:
- Correct project name (echo) in all configuration files
- Flowmates artifact storage initialized
- Repository identity aligned with the echo project

## 🎯 Goal
Initialize the repository by updating all rbase references to echo, initializing flowmates artifact storage, and ensuring the project is ready for development.

## 📏 Success Metrics
- [ ] All rbase references changed to echo in Cargo.toml and README.md
- [ ] Flowmates artifact storage directory structure created
- [ ] Repository identity properly configured

## 🧩 Acceptance Criteria
- [ ] Cargo.toml package name updated to "echo"
- [ ] Cargo.toml description updated for echo project
- [ ] README.md updated with echo-specific content
- [ ] Flowmates artifact directory created at `/Users/lsh/projects/flowmates/projects/e50a71e05c4182de/`
- [ ] Basic artifact directory structure initialized (artifacts/ subdirectory)
- [ ] Project compiles successfully after changes
- [ ] All references to rbase removed

## 🧪 Implementation Steps
1. Create/switch to branch `feat/init-repo`
2. Update Cargo.toml: change package name from "rbase" to "echo"
3. Update Cargo.toml: update description to reflect echo project
4. Update README.md: replace rbase references with echo
5. Initialize flowmates artifact storage directory structure
6. Verify project builds: `cargo build`
7. Move this file to `in_progress/` then `done/`
8. Create PR referencing this issue

## 🚫 Out of Scope
- Setting up CI/CD workflows (separate task)
- Creating production environment variables (separate task)
- Writing echo-specific functionality (separate task)

## 🔬 Risks / Mitigations
- Risk: Breaking existing build → Mitigation: Test build after each change
- Risk: Missing some rbase references → Mitigation: Search codebase for "rbase" before completing

## 🔗 Discussion Notes
Initial repository setup task to align project identity with echo branding.


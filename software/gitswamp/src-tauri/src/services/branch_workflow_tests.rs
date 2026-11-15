use std::fs;

use tempfile::TempDir;

use super::*;

struct Fixture(TempDir);

impl Fixture {
    fn new(branch: &str, commit: bool) -> Self {
        let fixture = Self(tempfile::tempdir().unwrap());
        fixture.git(&["init", "--initial-branch", branch]);
        fixture.git(&["config", "user.name", "Workflow Test"]);
        fixture.git(&["config", "user.email", "workflow@example.invalid"]);
        fixture.git(&["config", "core.autocrlf", "false"]);
        if commit {
            fixture.commit("shared.txt", "base\n", "Base");
        }
        fixture
    }
    fn path(&self) -> &str {
        self.0.path().to_str().unwrap()
    }
    fn git(&self, command: &[&str]) -> String {
        Git::git_cli(self.path(), command).unwrap()
    }
    fn commit(&self, path: &str, content: &str, message: &str) {
        let file = self.0.path().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, content).unwrap();
        self.git(&["add", "--", path]);
        self.git(&["commit", "-m", message]);
    }
    fn run(&self, config: &Config, request: &Request) -> Outcome {
        let plan = preview(self.path(), config, request).unwrap();
        execute(self.path(), config, request, &plan.token).unwrap()
    }
}

fn config(strategy: Strategy) -> Config {
    Config {
        strategy,
        primary: "main".into(),
        development: "dev".into(),
        feature: "feature/".into(),
        fix: "bugfix/".into(),
        release: "release/".into(),
        hotfix: "hotfix/".into(),
        support: "support/".into(),
        tag: "v".into(),
        short_branches: true,
        release_branches: false,
    }
}

fn request(action: Action, source: &str, name: &str) -> Request {
    Request {
        action,
        source: source.into(),
        name: name.into(),
        cleanup: false,
        remote: None,
    }
}

#[test]
fn detects_local_defaults_and_keeps_unborn_repositories_unmodified() {
    let blank = Fixture::new("production", false);
    let inspection = inspect(blank.path()).unwrap();
    assert!(inspection.unborn && !inspection.minimal);
    assert_eq!(inspection.primary, "production");
    assert!(inspection.local.is_empty());
    let mut setup = config(Strategy::Gitflow);
    setup.primary = "production".into();
    assert!(validate(&setup).is_ok());
    assert!(preview(
        blank.path(),
        &setup,
        &request(Action::PrepareDevelopment, "", "")
    )
    .is_err());
    let f = Fixture::new("master", true);
    assert!(inspect(f.path()).unwrap().minimal);
    f.git(&["branch", "develop"]);
    assert_eq!(
        inspect(f.path()).unwrap().development.as_deref(),
        Some("develop")
    );
    assert_eq!(
        inspect(f.path()).unwrap().suggestion,
        Some(Strategy::Gitflow)
    );
    let trunk = Fixture::new("trunk", true);
    assert_eq!(
        inspect(trunk.path()).unwrap().suggestion,
        Some(Strategy::Trunk)
    );
    assert_ne!(inspect(trunk.path()).unwrap().key, inspection.key);
}

#[test]
fn suggests_from_fetched_refs_but_never_calls_shallow_history_minimal() {
    let source = Fixture::new("main", true);
    source.commit("second.txt", "second\n", "Second commit");
    source.git(&["branch", "dev"]);
    let clone = tempfile::tempdir().unwrap();
    let url = format!(
        "file://{}{}",
        if cfg!(windows) { "/" } else { "" },
        source.path().replace('\\', "/")
    );
    Git::git_cli(
        clone.path().to_str().unwrap(),
        &[
            "clone",
            "--depth",
            "1",
            "--no-single-branch",
            &url,
            "checkout",
        ],
    )
    .unwrap();
    let path = clone.path().join("checkout");
    let inspection = inspect(path.to_str().unwrap()).unwrap();
    assert!(!inspection.minimal);
    assert_eq!(inspection.suggestion, Some(Strategy::Gitflow));
    assert!(inspection.remote.contains(&"dev".to_owned()));
    assert!(!inspection.local.contains(&"dev".to_owned()));
}

#[test]
fn gitflow_creates_feature_finishes_release_and_tags_the_production_result() {
    let f = Fixture::new("main", true);
    let cfg = config(Strategy::Gitflow);
    assert_eq!(
        f.run(&cfg, &request(Action::PrepareDevelopment, "", ""))
            .status,
        "applied"
    );
    assert_eq!(f.git(&["branch", "--show-current"]), "main");
    assert_eq!(
        inspect(f.path()).unwrap().development.as_deref(),
        Some("dev")
    );
    assert_eq!(
        f.run(&cfg, &request(Action::Feature, "", "payments"))
            .status,
        "applied"
    );
    assert_eq!(f.git(&["branch", "--show-current"]), "feature/payments");
    f.commit("payments.txt", "payments\n", "Payments");
    let feature = f.git(&["rev-parse", "HEAD"]);
    let mut finishing = request(Action::Finish, "feature/payments", "");
    finishing.cleanup = true;
    assert_eq!(f.run(&cfg, &finishing).status, "applied");
    f.git(&["merge-base", "--is-ancestor", &feature, "dev"]);
    assert_eq!(f.git(&["branch", "--list", "feature/payments"]), "");
    assert_eq!(
        f.run(&cfg, &request(Action::Release, "", "1.2.0")).status,
        "applied"
    );
    f.commit("release.txt", "release\n", "Prepare release");
    let release = f.git(&["rev-parse", "HEAD"]);
    let mut finishing = request(Action::Finish, "release/1.2.0", "1.2.0");
    finishing.cleanup = true;
    let result = f.run(&cfg, &finishing);
    assert_eq!(result.status, "applied", "{:?}", result.error);
    f.git(&["merge-base", "--is-ancestor", &release, "main"]);
    f.git(&["merge-base", "--is-ancestor", &release, "dev"]);
    assert_eq!(
        f.git(&["rev-parse", "refs/tags/v1.2.0"]),
        f.git(&["rev-parse", "main"])
    );
    assert_eq!(f.git(&["branch", "--list", "release/1.2.0"]), "");
    assert_eq!(
        f.run(&cfg, &request(Action::Support, "v1.2.0", "1.x"))
            .status,
        "applied"
    );
    assert_eq!(
        f.git(&["rev-parse", "support/1.x"]),
        f.git(&["rev-parse", "refs/tags/v1.2.0"])
    );
}

#[test]
fn hotfix_preserves_completed_steps_and_never_tags_or_deletes_during_conflict() {
    let f = Fixture::new("main", true);
    let cfg = config(Strategy::Gitflow);
    f.git(&["switch", "-c", "dev"]);
    f.commit("shared.txt", "development\n", "Development change");
    assert_eq!(
        f.run(&cfg, &request(Action::Hotfix, "", "1.0.1")).status,
        "applied"
    );
    f.commit("shared.txt", "production fix\n", "Production change");
    let mut finishing = request(Action::Finish, "hotfix/1.0.1", "1.0.1");
    finishing.cleanup = true;
    let plan = preview(f.path(), &cfg, &finishing).unwrap();
    assert!(plan
        .comparisons
        .iter()
        .any(|c| c.target == "dev" && c.conflicts == Some(true)));
    let result = execute(f.path(), &cfg, &finishing, &plan.token).unwrap();
    assert_eq!(result.status, "conflict", "{:?}", result.error);
    assert!(result
        .completed
        .contains(&"Merge hotfix/1.0.1 into main".to_owned()));
    assert!(result
        .remaining
        .iter()
        .any(|line| line.contains("into dev")));
    let open = Git::open(f.path()).unwrap();
    assert_eq!(open.state(), RepositoryState::Merge);
    assert!(open.index().unwrap().has_conflicts());
    assert_eq!(f.git(&["tag", "--list", "v1.0.1"]), "");
    assert!(!f.git(&["branch", "--list", "hotfix/1.0.1"]).is_empty());
    assert!(preview(f.path(), &cfg, &finishing).is_err());
    f.commit("shared.txt", "resolved\n", "Complete hotfix merge");
    let resumed = f.run(&cfg, &finishing);
    assert_eq!(resumed.status, "applied", "{:?}", resumed.error);
    assert_eq!(
        f.git(&["rev-parse", "refs/tags/v1.0.1"]),
        f.git(&["rev-parse", "main"])
    );
    assert_eq!(f.git(&["branch", "--list", "hotfix/1.0.1"]), "");
}

#[test]
fn clean_checks_stale_preview_active_worktree_and_fast_forward_rules() {
    let f = Fixture::new("main", true);
    let cfg = config(Strategy::Trunk);
    let start = request(Action::Feature, "", "search");
    let old = preview(f.path(), &cfg, &start).unwrap();
    f.commit("main.txt", "updated\n", "Change main");
    assert!(matches!(execute(f.path(), &cfg, &start, &old.token), Err(error) if error == STALE));
    fs::write(f.0.path().join("dirty.txt"), "keep").unwrap();
    assert!(preview(f.path(), &cfg, &start).is_err());
    assert!(f.0.path().join("dirty.txt").exists());
    fs::remove_file(f.0.path().join("dirty.txt")).unwrap();
    assert_eq!(f.run(&cfg, &start).status, "applied");
    assert!(preview(f.path(), &cfg, &start).is_err());
    f.commit("search.txt", "search\n", "Add search");
    let finish = request(Action::Finish, "feature/search", "");
    let other = tempfile::tempdir().unwrap();
    let workspace = other.path().join("checkout");
    f.git(&["worktree", "add", workspace.to_str().unwrap(), "main"]);
    assert!(preview(f.path(), &cfg, &finish).is_err());
    f.git(&["worktree", "remove", workspace.to_str().unwrap()]);
    assert_eq!(f.run(&cfg, &finish).status, "applied");
    assert_eq!(
        f.git(&["rev-parse", "main"]),
        f.git(&["rev-parse", "feature/search"])
    );
    f.git(&["switch", "--detach", "main"]);
    assert!(preview(f.path(), &cfg, &request(Action::Tag, "", "1.0.0")).is_err());
}

#[test]
fn handoff_uses_the_selected_remote_and_never_performs_local_integration() {
    let f = Fixture::new("main", true);
    let mut cfg = config(Strategy::Swamp);
    cfg.feature = "change/".into();
    f.git(&["branch", "change/search"]);
    f.git(&["switch", "change/search"]);
    f.commit("tests/search.test.ts", "test search\n", "Cover search");
    let main = f.git(&["rev-parse", "main"]);
    let branch = f.git(&["rev-parse", "change/search"]);
    f.git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/example/project",
    ]);
    f.git(&["update-ref", "refs/remotes/origin/main", &main]);
    let mut review = request(Action::Handoff, "change/search", "");
    review.remote = Some("origin".into());
    assert!(
        !preview(f.path(), &cfg, &review)
            .unwrap()
            .handoff
            .unwrap()
            .published
    );
    f.git(&["update-ref", "refs/remotes/origin/change/search", &branch]);
    let plan = preview(f.path(), &cfg, &review).unwrap();
    let handoff = plan.handoff.unwrap();
    assert!(handoff.published && plan.steps.is_empty());
    assert_eq!(handoff.checkpoint.unwrap().test_paths, 1);
    assert!(execute(f.path(), &cfg, &review, &plan.token).is_err());
    assert!(preview(
        f.path(),
        &cfg,
        &request(Action::Finish, "change/search", "")
    )
    .is_err());
    assert!(preview(
        f.path(),
        &cfg,
        &request(Action::Cleanup, "change/search", "")
    )
    .is_err());
    assert_eq!(f.git(&["rev-parse", "main"]), main);
}

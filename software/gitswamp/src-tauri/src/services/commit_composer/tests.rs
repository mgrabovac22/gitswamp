use std::fs;

use tempfile::TempDir;

use super::super::secret_service::GITHUB_MAX_FILE_BYTES;
use super::*;

struct Fixture(TempDir);
impl Fixture {
    fn new(initial: bool) -> Self {
        let f = Self(tempfile::tempdir().unwrap());
        f.git(&["init", "--initial-branch=main"]);
        f.git(&["config", "user.name", "Composer Test"]);
        f.git(&["config", "user.email", "composer@example.invalid"]);
        f.git(&["config", "core.autocrlf", "false"]);
        if initial {
            f.git(&["commit", "--allow-empty", "-m", "Base"]);
        }
        f
    }
    fn path(&self) -> &str {
        self.0.path().to_str().unwrap()
    }
    fn git(&self, args: &[&str]) -> String {
        Git::git_cli(self.path(), args).unwrap()
    }
    fn write(&self, path: &str, bytes: &str) {
        let path = self.0.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn drafts(plan: &Composition) -> Vec<Draft> {
        plan.groups
            .iter()
            .map(|g| Draft {
                subject: g.subject.clone(),
                description: g.description.clone(),
                units: g.units.clone(),
            })
            .collect()
    }
    fn apply(&self, plan: &Composition) -> Vec<String> {
        apply(
            self.path(),
            &plan.snapshot,
            &Self::drafts(plan),
            false,
            false,
        )
        .unwrap()
    }
}

fn together(plan: &Composition, a: &str, b: &str) -> bool {
    plan.units.iter().any(|unit| {
        [a, b]
            .iter()
            .all(|path| unit.changes.iter().any(|i| plan.changes[*i].path == *path))
    })
}

fn suggested_together(plan: &Composition, a: &str, b: &str) -> bool {
    std::ptr::eq(draft_with(plan, a), draft_with(plan, b))
}

fn draft_with<'a>(plan: &'a Composition, path: &str) -> &'a Group {
    plan.groups
        .iter()
        .find(|group| {
            group
                .units
                .iter()
                .flat_map(|unit| &plan.units[*unit].changes)
                .any(|index| plan.changes[*index].path == path)
        })
        .unwrap_or_else(|| panic!("No draft contains {path}"))
}

#[test]
fn composes_exact_snapshots_without_consuming_partial_unstaged_edits() {
    let f = Fixture::new(true);
    f.write("src/auth/token.ts", "export const token = 'base';\n");
    f.git(&["add", "--all"]);
    f.git(&["commit", "-m", "Add original file"]);
    f.write("src/auth/token.ts", "export const token = 'staged';\n");
    f.git(&["add", "--", "src/auth/token.ts"]);
    f.write("src/auth/token.ts", "export const token = 'working';\n");
    f.write(
        "tests/token.test.ts",
        "import { token } from '../src/auth/token';\n",
    );
    f.write("package.json", "{\"name\":\"fixture\"}\n");
    f.write(
        "package-lock.json",
        "{\"name\":\"fixture\",\"lockfileVersion\":3}\n",
    );
    f.write("docs/usage.md", "# Use the fixture\n");
    f.write("src/routes/[route].ts", "export function route() {}\n");
    let head_before = f.git(&["rev-parse", "HEAD"]);
    let index_before = Git::index_bytes(&f.0.path().join(".git/index"), MAX_INDEX).unwrap();
    let objects_before = f.git(&["count-objects"]);
    let plan = plan(f.path(), false).unwrap();
    assert_eq!(f.git(&["count-objects"]), objects_before);
    assert_eq!(f.git(&["rev-parse", "HEAD"]), head_before);
    assert_eq!(
        Git::index_bytes(&f.0.path().join(".git/index"), MAX_INDEX).unwrap(),
        index_before
    );
    assert!(suggested_together(
        &plan,
        "src/auth/token.ts",
        "tests/token.test.ts"
    ));
    assert!(suggested_together(
        &plan,
        "package.json",
        "package-lock.json"
    ));
    assert!(plan
        .changes
        .iter()
        .any(|file| file.path.ends_with("token.ts") && file.partial));
    let change = plan
        .changes
        .iter()
        .position(|file| file.path == "src/auth/token.ts")
        .unwrap();
    let diff = preview_change(f.path(), &plan.snapshot, false, change).unwrap();
    assert!(diff
        .lines
        .iter()
        .any(|line| line.line_type == "addition" && line.content.contains("'staged'")));
    assert!(diff
        .lines
        .iter()
        .any(|line| line.line_type == "deletion" && line.content.contains("'base'")));
    assert!(!diff
        .lines
        .iter()
        .any(|line| line.content.contains("'working'")));
    let commits = f.apply(&plan);
    assert!(commits.len() >= 3);
    assert_eq!(
        f.git(&["rev-list", "--count", &format!("{head_before}..HEAD")]),
        commits.len().to_string()
    );
    assert_eq!(
        f.git(&["show", "HEAD:src/auth/token.ts"]),
        "export const token = 'staged';"
    );
    assert_eq!(f.git(&["status", "--short"]), "M src/auth/token.ts");
    assert_eq!(
        f.git(&["show", "HEAD:src/routes/[route].ts"]),
        "export function route() {}"
    );
    f.git(&["fsck", "--no-dangling"]);
}

#[test]
fn creates_initial_commit_and_rejects_stale_changes_and_invalid_group_assignment() {
    let f = Fixture::new(false);
    f.write("readme.md", "First draft");
    let first = plan(f.path(), false).unwrap();
    f.write("readme.md", "New contents");
    assert_eq!(
        apply(
            f.path(),
            &first.snapshot,
            &Fixture::drafts(&first),
            false,
            false
        )
        .unwrap_err(),
        STALE
    );
    assert!(!f.0.path().join(".git/index").exists());
    let second = plan(f.path(), false).unwrap();
    let mut drafts = Fixture::drafts(&second);
    let duplicate = drafts[0].units[0];
    drafts[0].units.push(duplicate);
    assert!(apply(f.path(), &second.snapshot, &drafts, false, false).is_err());
    assert_eq!(f.apply(&second).len(), 1);
    assert_eq!(f.git(&["show", "HEAD:readme.md"]), "New contents");
    assert_eq!(f.git(&["symbolic-ref", "--short", "HEAD"]), "main");
}

#[test]
fn groups_renames_and_path_replacements_and_preserves_index_on_cancel() {
    let f = Fixture::new(true);
    f.write("old.rs", "pub fn demo() {}\n");
    f.write("asset", "asset");
    f.git(&["add", "--all"]);
    f.git(&["commit", "-m", "Add original paths"]);
    f.git(&["mv", "old.rs", "new.rs"]);
    fs::remove_file(f.0.path().join("asset")).unwrap();
    f.write("asset/index.js", "export const path = 1;\n");
    let plan = plan(f.path(), false).unwrap();
    assert!(plan
        .changes
        .iter()
        .any(|file| file.path == "new.rs" && file.old_path.as_deref() == Some("old.rs")));
    assert!(together(&plan, "asset", "asset/index.js"));
    f.apply(&plan);
    assert_eq!(f.git(&["status", "--porcelain"]), "");
    f.git(&["fsck", "--no-dangling"]);
}

#[test]
fn enforces_secret_review_and_leaves_custom_filters_to_manual_staging() {
    let f = Fixture::new(true);
    f.write("settings.js", "const api_key = 'AKIAIOSFODNN7EXAMPLE';\n");
    let p = plan(f.path(), true).unwrap();
    assert!(!p.review.findings.is_empty());
    assert!(apply(f.path(), &p.snapshot, &Fixture::drafts(&p), true, false).is_err());
    assert_eq!(f.git(&["rev-list", "--count", "HEAD"]), "1");
    apply(f.path(), &p.snapshot, &Fixture::drafts(&p), true, true).unwrap();
    f.write(".gitattributes", "*.txt filter=custom\n");
    f.write("input.txt", "Custom filter contents");
    assert!(plan(f.path(), false)
        .err()
        .unwrap()
        .contains("configured Git filter"));
}

#[test]
fn groups_matching_dependency_imports_and_schema_migrations() {
    let f = Fixture::new(true);
    f.write(
        "package.json",
        "{\n  \"dependencies\": {\n    \"super-validator\": \"1.0.0\"\n  }\n}\n",
    );
    f.write("src/validation.ts", "import { check } from 'super-validator';\nexport function validate() { return check(); }\n");
    f.write("prisma/schema.prisma", "model User { id Int @id }\n");
    f.write(
        "prisma/migrations/001/migration.sql",
        "CREATE TABLE User (id INTEGER PRIMARY KEY);\n",
    );
    let plan = plan(f.path(), false).unwrap();
    assert!(suggested_together(
        &plan,
        "src/validation.ts",
        "package.json"
    ));
    assert!(suggested_together(
        &plan,
        "prisma/schema.prisma",
        "prisma/migrations/001/migration.sql"
    ));
    assert!(!suggested_together(
        &plan,
        "src/validation.ts",
        "prisma/schema.prisma"
    ));
}

#[test]
fn keeps_cross_module_imports_together_with_a_scope_and_specific_message() {
    let f = Fixture::new(true);
    f.write(
        "packages/cli/src/main.ts",
        "import { buildCommitSeries } from '../../core/src/index';\nbuildCommitSeries();\n",
    );
    f.write(
        "packages/core/src/index.ts",
        "export function buildCommitSeries() {}\n",
    );
    let plan = plan(f.path(), false).unwrap();
    assert!(suggested_together(
        &plan,
        "packages/cli/src/main.ts",
        "packages/core/src/index.ts"
    ));
    assert_eq!(plan.groups.len(), 1);
    assert_eq!(
        plan.groups[0].subject,
        "Feat(packages): Add build commit series"
    );
    assert!(plan.groups[0]
        .description
        .contains("Adds the `buildCommitSeries` declaration."));
}

#[test]
fn pairs_only_the_corresponding_manifest_and_lockfile() {
    let f = Fixture::new(true);
    f.write("pixi.toml", "[workspace]\nname = \"demo\"\n");
    f.write("pixi.lock", "version: 7\n");
    f.write("package.json", "{\"name\":\"demo\"}\n");
    f.write("yarn.lock", "# yarn lockfile\n");
    let plan = plan(f.path(), false).unwrap();
    assert!(suggested_together(&plan, "pixi.toml", "pixi.lock"));
    assert!(suggested_together(&plan, "package.json", "yarn.lock"));
    assert!(!suggested_together(&plan, "pixi.toml", "yarn.lock"));
    assert!(plan.groups.iter().all(|group| group.subject.contains('(')));
}

#[test]
fn moves_one_file_out_of_a_suggested_group_without_changing_the_final_tree() {
    let f = Fixture::new(true);
    let provider = "src/session.ts";
    let test = "tests/session.test.ts";
    f.write(provider, "export function restoreSession() {}\n");
    f.write(
        test,
        "import { restoreSession } from '../src/session';\nrestoreSession();\n",
    );
    f.git(&["add", "."]);
    let final_tree = f.git(&["write-tree"]);
    let preview = plan(f.path(), false).unwrap();
    assert!(suggested_together(&preview, provider, test));
    assert!(!together(&preview, provider, test));
    let unit = |path| {
        preview
            .units
            .iter()
            .find(|unit| {
                unit.changes
                    .iter()
                    .any(|id| preview.changes[*id].path == path)
            })
            .unwrap()
            .id
    };
    let draft = |path, subject: &str| Draft {
        subject: subject.into(),
        description: String::new(),
        units: vec![unit(path)],
    };
    let split = [
        draft(provider, "Feat(session): Add session restoration"),
        draft(
            test,
            "Test(session): Cover the session restoration entry point",
        ),
    ];
    assert!(apply(
        f.path(),
        &preview.snapshot,
        &[
            draft(test, "Test(session): Call session restoration"),
            draft(provider, "Feat(session): Add session restoration")
        ],
        false,
        false
    )
    .is_err());
    let result = apply(f.path(), &preview.snapshot, &split, false, false).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(
        f.git(&[
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            &result[0]
        ]),
        provider
    );
    assert_eq!(
        f.git(&[
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            &result[1]
        ]),
        test
    );
    assert_eq!(f.git(&["rev-parse", "HEAD^{tree}"]), final_tree);
}

#[test]
fn drafts_a_feature_across_layers_and_keeps_ambiguous_names_independent() {
    let f = Fixture::new(true);
    let controller = "src/orders/controllers/CheckoutController.cs";
    let service = "src/orders/services/CheckoutService.cs";
    let test = "tests/orders/CheckoutServiceTests.cs";
    let docs = "docs/orders/checkout-guide.md";
    let other = "src/admin/controllers/CheckoutController.cs";
    let ambiguous = "tests/CheckoutTests.cs";
    let separate_a = "src/orders/services/TaxRules.cs";
    let separate_b = "src/orders/services/DiscountRules.cs";
    for path in [
        controller, service, test, other, ambiguous, separate_a, separate_b,
    ] {
        f.write(path, "// Fixture\n");
    }
    f.write(docs, "# Checkout examples\n");
    let before = f.git(&["rev-parse", "HEAD"]);
    let preview = plan(f.path(), false).unwrap();
    let checkout = draft_with(&preview, controller);
    assert_eq!(checkout.units, draft_with(&preview, service).units);
    assert_eq!(checkout.units, draft_with(&preview, test).units);
    assert_eq!(checkout.units, draft_with(&preview, docs).units);
    assert_ne!(checkout.units, draft_with(&preview, other).units);
    assert_ne!(checkout.units, draft_with(&preview, ambiguous).units);
    assert_ne!(
        draft_with(&preview, separate_a).units,
        draft_with(&preview, separate_b).units
    );
    assert!(
        !together(&preview, controller, test),
        "review groups must remain editable"
    );
    assert!(checkout
        .description
        .contains("Keep the matching implementation"));
    assert!(checkout.subject.contains("checkout implementation"));
    assert_eq!(f.git(&["rev-parse", "HEAD"]), before);
    f.apply(&preview);
    assert_eq!(f.git(&["status", "--porcelain"]), "");
}

#[test]
fn groups_ten_files_into_three_editable_feature_commits() {
    let f = Fixture::new(true);
    let cart = [
        "apps/shop/src/features/cart.ts",
        "apps/shop/src/features/cart/hooks/useCoupons.ts",
        "apps/shop/src/features/cart/styles/coupons.css",
        "apps/shop/src/features/cart/tests/cartTotals.spec.ts",
        "apps/shop/src/features/cart/README.md",
    ];
    let history = [
        "apps/shop/src/features/repository/history/CommitFilter.vue",
        "apps/shop/src/features/repository/history/hooks/useTimeline.ts",
        "apps/shop/src/features/repository/history/tests/restore.spec.ts",
    ];
    let admin = [
        "apps/admin/src/features/cart/CartDialog.tsx",
        "apps/admin/src/features/cart/GroupPermissions.ts",
    ];
    for path in cart.iter().chain(&history).chain(&admin) {
        f.write(path, "// Draft fixture\n");
    }
    let head = f.git(&["rev-parse", "HEAD"]);
    let draft = plan(f.path(), false).unwrap();
    assert_eq!(draft.changes.len(), 10);
    assert_eq!(draft.groups.len(), 3);
    for module in [&cart[..], &history, &admin] {
        let first = draft_with(&draft, module[0]);
        assert!(first.description.contains("same feature module"));
        for file in &module[1..] {
            assert_eq!(first.units, draft_with(&draft, file).units);
            assert!(!together(&draft, module[0], file));
        }
    }
    assert_ne!(draft_with(&draft, cart[0]).units, draft_with(&draft, history[0]).units);
    assert_ne!(draft_with(&draft, cart[0]).units, draft_with(&draft, admin[0]).units);
    assert_eq!(f.git(&["rev-parse", "HEAD"]), head);
    let commits = f.apply(&draft);
    assert_eq!(commits.len(), 3);
    assert!(f.git(&["status", "--porcelain"]).is_empty());
}

#[test]
fn diff_intent_groups_matching_changes_without_merging_two_behaviors_in_one_module() {
    let f = Fixture::new(true);
    let redeem = "src/features/checkout/PanelOne.ts";
    let limits = "src/features/checkout/PanelTwo.ts";
    let shipping = "src/features/checkout/PanelThree.ts";
    let delivery = "src/features/checkout/PanelFour.ts";
    let tests = "src/features/checkout/tests/coupon.spec.ts";
    f.write(redeem, "export function redeemCoupon() { return true; }\n");
    f.write(limits, "export function validateCouponLimit() { return true; }\n");
    f.write(shipping, "export function calculateShipping() { return 4; }\n");
    f.write(delivery, "export function estimateShippingWindow() { return 5; }\n");
    f.write(tests, "// Test fixture\n");

    let draft = plan(f.path(), false).unwrap();
    let coupon = draft_with(&draft, redeem);
    assert_eq!(draft.groups.len(), 2);
    assert_eq!(coupon.units, draft_with(&draft, limits).units);
    assert_eq!(coupon.units, draft_with(&draft, tests).units);
    assert_ne!(coupon.units, draft_with(&draft, shipping).units);
    assert_eq!(draft_with(&draft, shipping).units, draft_with(&draft, delivery).units);
    assert!(coupon.description.contains("matching diff evidence"));
    assert_eq!(f.apply(&draft).len(), 2);
    assert!(f.git(&["status", "--porcelain"]).is_empty());
}

#[test]
fn drafts_ui_with_named_stories_and_locales_without_crossing_workspaces() {
    let f = Fixture::new(true);
    let screen = "apps/store/src/features/cart/CartScreen.tsx";
    let story = "apps/store/src/features/cart/CartScreen.stories.tsx";
    let hook = "apps/store/src/features/cart/hooks/useCart.ts";
    let style = "apps/store/src/features/cart/CartScreen.module.css";
    let locale = "apps/store/locales/hr/cart.json";
    let common = "apps/store/locales/hr/common.json";
    let admin = "apps/admin/locales/hr/cart.json";
    f.write(screen, "export function CartScreen() {}\n");
    f.write(story, "export const EmptyCart = {};\n");
    f.write(hook, "export function useCart() {}\n");
    f.write(style, ".empty { display: none; }\n");
    for path in [locale, common, admin] {
        f.write(path, "{\"title\":\"Cart\"}\n");
    }
    let preview = plan(f.path(), false).unwrap();
    let ui = draft_with(&preview, screen);
    assert_eq!(ui.units, draft_with(&preview, story).units);
    assert_eq!(ui.units, draft_with(&preview, hook).units);
    assert_eq!(ui.units, draft_with(&preview, style).units);
    assert_eq!(ui.units, draft_with(&preview, locale).units);
    assert_ne!(ui.units, draft_with(&preview, common).units);
    assert_ne!(ui.units, draft_with(&preview, admin).units);
}

#[test]
fn drafts_a_migration_with_the_source_that_uses_its_exact_table() {
    let f = Fixture::new(true);
    let write = "src/data/InvoiceRepository.cs";
    let unrelated = "src/data/PaymentRepository.cs";
    let migration = "data/migrations/001.sql";
    f.write(
        write,
        "var sql = \"INSERT INTO billing.invoices (id) VALUES (@id)\";\n",
    );
    f.write(
        unrelated,
        "var sql = \"SELECT id FROM reporting.invoices WHERE id = @id\";\n",
    );
    f.write(
        migration,
        "ALTER TABLE billing.invoices ADD COLUMN number TEXT;\n",
    );
    let preview = plan(f.path(), false).unwrap();
    let invoice = draft_with(&preview, write);
    assert_eq!(invoice.units, draft_with(&preview, migration).units);
    assert_ne!(invoice.units, draft_with(&preview, unrelated).units);
    assert!(invoice
        .description
        .contains("Keep the migration with the implementation"));
    assert!(!together(&preview, write, migration));

    f.write(
        "src/reporting/ExportRepository.cs",
        "var sql = \"SELECT id FROM billing.invoices WHERE id = @id\";\n",
    );
    let preview = plan(f.path(), false).unwrap();
    assert_ne!(
        draft_with(&preview, write).units,
        draft_with(&preview, migration).units
    );
}

#[test]
fn explains_actual_changes_without_copying_values_into_messages() {
    let f = Fixture::new(true);
    f.write("app/settings.env", "retry_count=old-value\n");
    f.write(
        "src/app.py",
        "def calculate_total(value):\n    return value\n",
    );
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Add baseline"]);
    f.write("app/settings.env", "retry_count=new-value\n");
    f.write(
        "src/app.py",
        "def calculate_total(value, fee):\n    return value + fee\n",
    );
    f.write(
        "db/migrations/001.sql",
        "ALTER TABLE accounts ADD COLUMN status TEXT;\n",
    );
    f.write("docs/guide.md", "## Configure branch protection\n");
    f.git(&["add", "--", "app/settings.env", "src/app.py"]);
    let plan = plan(f.path(), false).unwrap();
    let messages = plan
        .groups
        .iter()
        .map(|draft| format!("{}\n{}", draft.subject, draft.description))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(messages.contains("Update table accounts"), "{messages}");
    assert!(
        messages.contains("Updates the `calculate_total` declaration."),
        "{messages}"
    );
    assert!(
        messages.contains("Updates the `retry_count` configuration key."),
        "{messages}"
    );
    assert!(
        messages.contains("Configure branch protection"),
        "{messages}"
    );
    assert!(!messages.contains("old-value") && !messages.contains("new-value"));
    f.apply(&plan);
    assert_eq!(f.git(&["status", "--porcelain"]), "");
}

#[test]
fn describes_each_file_in_a_multifile_feature_without_leaking_values() {
    let f = Fixture::new(true);
    let screen = "apps/store/src/cart/CartScreen.tsx";
    f.write(screen, "export function CartScreen() {}\n");
    let locales: Vec<_> = [
        "en", "hr", "fr", "es", "de", "tr", "pl", "pt", "it", "ja", "ko", "cs",
    ]
    .into_iter()
    .map(|locale| format!("apps/store/locales/{locale}/cart.json"))
    .collect();
    for path in &locales {
        f.write(
            path,
            "{\n  \"checkout_title\": \"private-fixture-content\"\n}\n",
        );
    }
    let preview = plan(f.path(), false).unwrap();
    let draft = draft_with(&preview, screen);
    assert!(draft.description.contains("Changes by file:"));
    assert!(draft
        .description
        .contains("Adds the `CartScreen` declaration."));
    let prose = draft
        .description
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(prose.contains("Adds the `checkout_title` configuration key."));
    assert!(locales.iter().all(|path| draft.description.contains(path)));
    assert!(!draft.description.contains("private-fixture-content"));
    assert!(draft.description.lines().all(|line| line.len() <= 72));
    assert!(draft.subject.encode_utf16().count() <= 72);
}

#[test]
fn drafts_contract_and_function_body_messages_from_staged_evidence() {
    let f = Fixture::new(true);
    let source = "core/rank.rs";
    f.write(
        source,
        "fn rank_candidate(left: i32, right: i32) -> i32 {\n    left + right\n}\n",
    );
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Add baseline"]);
    f.write(
        source,
        "fn rank_candidate(left: i32, right: i32) -> i32 {\n    left - right\n}\n",
    );
    f.write(
        "rpc/catalog.proto",
        "rpc ListCatalog(Request) returns (Response);\n",
    );
    f.write(
        "data/migrations/001.sql",
        "ALTER TABLE catalog ADD COLUMN alias VARCHAR(32) DEFAULT 'private-test-value';\n",
    );
    f.git(&["add", "."]);
    let proposed = plan(f.path(), false).unwrap();
    let group = |path| {
        proposed
            .groups
            .iter()
            .find(|group| {
                group.units.iter().any(|id| {
                    proposed.units[*id]
                        .changes
                        .iter()
                        .any(|id| proposed.changes[*id].path == path)
                })
            })
            .unwrap()
    };
    assert!(group(source).subject.contains("Revise rank candidate"));
    assert!(group(source)
        .description
        .contains("Changes the implementation in `rank_candidate`."));
    assert!(group("rpc/catalog.proto")
        .subject
        .contains("Expose endpoint ListCatalog"));
    let sql = &group("data/migrations/001.sql").description;
    assert!(sql.contains("Adds the column `alias`."), "{sql}");
    assert!(!sql.contains("private-test-value"));
    assert!(proposed
        .groups
        .iter()
        .all(|group| group.subject.encode_utf16().count() <= 72));
    f.apply(&proposed);
    assert_eq!(f.git(&["status", "--porcelain"]), "");
}

#[test]
fn composes_behavior_from_hunks_and_splits_unrelated_features_in_one_folder() {
    let f = Fixture::new(true);
    f.write("src/ui/account.ts", "export function selectAccount(account) {\n  const initial = 1;\n  const next = initial;\n  const saved = next;\n  const value = saved;\n  return account.id + value;\n}\n");
    f.write(
        "src/ui/dashboard.js",
        "function bindDashboard(el) {\n  return el;\n}\n",
    );
    f.write(
        "client/request.py",
        "async def request_status():\n    return await wait_for(fetch(), 10)\n",
    );
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Add baseline"]);
    f.write("src/ui/account.ts", "export function selectAccount(account) {\n  const initial = 1;\n  const next = initial;\n  const saved = next;\n  const value = saved;\n  if (account == null) return;\n  return account.id + value;\n}\n");
    f.write(
        "src/ui/dashboard.js",
        "function bindDashboard(el) {\n  el.addEventListener('click', refresh);\n  return el;\n}\n",
    );
    f.write(
        "client/request.py",
        "async def request_status():\n    return await wait_for(fetch(), 30)\n",
    );
    f.git(&["add", "."]);
    let plan = plan(f.path(), false).unwrap();
    let group_for = |path: &str| {
        plan.groups
            .iter()
            .find(|group| {
                group.units.iter().any(|id| {
                    plan.units[*id]
                        .changes
                        .iter()
                        .any(|file| plan.changes[*file].path == path)
                })
            })
            .unwrap()
    };
    let account = group_for("src/ui/account.ts");
    let dashboard = group_for("src/ui/dashboard.js");
    let request = group_for("client/request.py");
    assert!(
        account
            .subject
            .contains("Add null checks in select account"),
        "{}",
        account.subject
    );
    assert!(
        account
            .description
            .starts_with("Adds null checks in `selectAccount`."),
        "{}",
        account.description
    );
    assert!(
        dashboard
            .subject
            .contains("Add event handlers in bind dashboard"),
        "{}",
        dashboard.subject
    );
    assert_ne!(account.units, dashboard.units);
    assert!(
        request
            .description
            .starts_with("Updates timeouts in `request_status`."),
        "{}",
        request.description
    );
    f.apply(&plan);
    assert_eq!(f.git(&["status", "--porcelain"]), "");
}

#[test]
fn stages_independent_hunks_of_one_file_without_consuming_unstaged_bytes() {
    let f = Fixture::new(true);
    let baseline = "export function normalizeInput(value) {\n    return value.trim();\n}\n\n// context one\n// context two\n// context three\n// context four\n// context five\n// context six\n// context seven\n\nexport function configureTimer() {\n    return 1;\n}\n";
    f.write("src/features.ts", baseline);
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Baseline"]);
    let after = baseline
        .replace(
            "    return value.trim();",
            "    if (value == null) return '';\n    return value.trim();",
        )
        .replace(
            "    return 1;",
            "    setTimeout(runTask, 20);\n    return 1;",
        );
    f.write("src/features.ts", &after);
    f.git(&["add", "--", "src/features.ts"]);
    f.write(
        "src/features.ts",
        &(after.clone() + "// Keep this unstaged\n"),
    );
    let head = f.git(&["rev-parse", "HEAD"]);
    let plan = plan(f.path(), false).unwrap();
    assert_eq!(plan.changes.len(), 2);
    assert!(plan
        .changes
        .iter()
        .all(|file| file.chunk.is_some() && file.partial));
    assert_eq!(plan.groups.len(), 2);
    let change = plan
        .changes
        .iter()
        .position(|file| {
            file.chunk
                .as_ref()
                .is_some_and(|chunk| chunk.scope == "configureTimer")
        })
        .unwrap();
    let diff = preview_change(f.path(), &plan.snapshot, false, change).unwrap();
    assert!(diff
        .lines
        .iter()
        .any(|line| line.line_type == "addition" && line.content.contains("setTimeout")));
    assert!(!diff
        .lines
        .iter()
        .any(|line| line.content.contains("value == null")
            || line.content.contains("Keep this unstaged")));
    let commits = f.apply(&plan);
    let intermediate = f.git(&["show", &format!("{}:src/features.ts", commits[0])]);
    assert_ne!(
        intermediate.contains("setTimeout"),
        intermediate.contains("value == null")
    );
    assert_eq!(f.git(&["show", "HEAD:src/features.ts"]), after.trim_end());
    assert_eq!(f.git(&["diff", "--cached", "--name-only"]), "");
    assert!(f
        .git(&["diff", "--", "src/features.ts"])
        .contains("+// Keep this unstaged"));
    assert_eq!(f.git(&["rev-parse", "HEAD~2"]), head);
}

#[test]
fn analyzes_large_diffs_without_a_shared_file_budget() {
    let f = Fixture::new(true);
    for n in 0..6 {
        let text = format!(
            "// {}\nexport function buildSearchIndex{n}() {{ return {n}; }}\n",
            "x".repeat(1_500_000)
        );
        f.write(&format!("modules/item{n}/search.ts"), &text);
    }
    f.write("docs/guide.md", "# Search configuration\n");
    f.git(&["add", "."]);
    let expected = f.git(&["write-tree"]);
    let preview = plan(f.path(), false).unwrap();
    assert!(!preview.limited && !preview.requires_build_verification);
    assert!(!suggested_together(
        &preview,
        "modules/item5/search.ts",
        "docs/guide.md"
    ));
    for n in 0..6 {
        let path = format!("modules/item{n}/search.ts");
        assert!(preview
            .changes
            .iter()
            .any(|change| change.path == path && change.added == 2));
        let draft = draft_with(&preview, &path);
        assert!(
            draft.description.contains(&format!("buildSearchIndex{n}")),
            "{}",
            draft.description
        );
    }
    assert!(f.apply(&preview).len() > 1);
    assert_eq!(f.git(&["rev-parse", "HEAD^{tree}"]), expected);
}

#[test]
fn checks_unchanged_callers_past_the_old_reverse_scan_and_import_limits() {
    let f = Fixture::new(true);
    for n in 0..310 {
        f.write(
            &format!("src/helpers/file{n:03}.ts"),
            "export const value = 1;\n",
        );
    }
    let names: Vec<_> = (0..24).map(|n| format!("helper{n:02}")).collect();
    let mut provider = names
        .iter()
        .map(|name| format!("export const {name} = 1;\n"))
        .collect::<String>();
    provider.push_str("export const requiredLast = 1;\n");
    f.write("src/provider.ts", &provider);
    f.write(
        "src/z-caller.ts",
        &format!(
            "import {{ {}, requiredLast }} from './provider';\n",
            names.join(", ")
        ),
    );
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Baseline"]);
    f.write(
        "src/provider.ts",
        provider
            .strip_suffix("export const requiredLast = 1;\n")
            .unwrap(),
    );
    f.write("docs/guide.md", "# Explain setup\n");
    let error = plan(f.path(), false).err().unwrap();
    assert!(
        error.contains("z-caller.ts imports requiredLast"),
        "{error}"
    );

    f.write(
        "src/z-caller.ts",
        &format!("import {{ {} }} from './provider';\n", names.join(", ")),
    );
    let fixed = plan(f.path(), false).unwrap();
    assert!(!fixed.requires_build_verification);
    assert!(!suggested_together(
        &fixed,
        "src/provider.ts",
        "docs/guide.md"
    ));
    assert!(f.apply(&fixed).len() > 1);
}

#[test]
fn dependency_review_continues_after_the_import_cache_fills() {
    let f = Fixture::new(true);
    let names: Vec<_> = (0..1450).map(|n| format!("exportedHelper{n:04}")).collect();
    let provider: String = names
        .iter()
        .map(|name| format!("export const {name} = 1;\n"))
        .collect();
    f.write("src/provider.ts", &provider);
    let callers = format!("import {{ {} }} from './provider';\n", names.join(", "));
    for n in 0..46 {
        f.write(&format!("src/caller{n:02}.ts"), &callers);
    }
    f.write(
        "src/caller46.ts",
        "import { missingFromProvider } from './provider';\n",
    );
    f.git(&["add", "."]);
    let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
    let tree = snapshot.repo.find_tree(snapshot.tree).unwrap();
    let mut review = dependencies::Check::new(
        &snapshot.repo,
        snapshot.head,
        snapshot.tree,
        &snapshot.changes,
    )
    .unwrap();
    for _ in 0..2 {
        let error = review.verify(&tree).unwrap().to_string();
        assert!(
            error.contains("caller46.ts imports missingFromProvider"),
            "{error}"
        );
        assert!(!review.limited);
    }
}

#[test]
fn retains_independent_groups_when_one_source_is_too_large_for_static_review() {
    let f = Fixture::new(true);
    f.write(
        "src/large.ts",
        &format!("// {}\n", "x".repeat(PATCH_FILE_LIMIT)),
    );
    f.write("docs/guide.md", "# Explain setup\n");
    let before = f.git(&["rev-parse", "HEAD"]);
    let draft = plan(f.path(), false).unwrap();
    assert!(draft.limited && draft.requires_build_verification);
    let groups = Fixture::drafts(&draft);
    assert_eq!(groups.len(), 2);
    let error = apply(f.path(), &draft.snapshot, &groups, false, false).unwrap_err();
    assert!(error.contains("Enable Verify every commit"), "{error}");
    assert_eq!(f.git(&["rev-parse", "HEAD"]), before);
    let checked = apply_with_verification(
        f.path(),
        &draft.snapshot,
        &groups,
        false,
        false,
        Some("git --version"),
    )
    .unwrap();
    assert_eq!(checked.len(), 2);
}

#[test]
fn refuses_missing_local_dependency_and_unsafe_manual_export_order() {
    let f = Fixture::new(true);
    f.write("src/core.ts", "export function existing() {}\n\n// one\n// two\n// three\n// four\n// five\n// six\n// seven\n\nexport function keep() {\n  return 1;\n}\n");
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Baseline"]);
    f.write("src/core.ts", "export function existing() {}\nexport function newFeature() {}\n\n// one\n// two\n// three\n// four\n// five\n// six\n// seven\n\nexport function keep() {\n  return 2;\n}\n");
    f.write(
        "src/use.ts",
        "import { newFeature } from './core';\nnewFeature();\n",
    );
    f.write("docs/README.md", "# Document baseline\n");
    f.git(&["add", "."]);
    let proposed = plan(f.path(), false).unwrap();
    let consumer = proposed
        .units
        .iter()
        .find(|unit| {
            unit.changes
                .iter()
                .any(|id| proposed.changes[*id].path == "src/use.ts")
        })
        .unwrap();
    assert!(consumer
        .changes
        .iter()
        .any(|id| proposed.changes[*id].path == "src/core.ts"
            || proposed.changes[*id].path == "src/use.ts"));
    f.apply(&proposed);
    let original = f.git(&["rev-parse", "HEAD"]);
    f.write("src/missing.ts", "import './missing-dependency';\n");
    assert!(plan(f.path(), false)
        .err()
        .is_some_and(|error| error.contains("missing-dependency")));
    assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
}

#[test]
fn blocks_reordering_a_caller_before_a_new_export() {
    let f = Fixture::new(true);
    f.write("src/feature.ts", "export const keep = 1;\n");
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Baseline"]);
    f.write(
        "src/feature.ts",
        "export const keep = 1;\nexport const enabled = true;\n",
    );
    // Deliberately bypass the grouping heuristic so the commit-tree guard is tested directly.
    f.write("src/caller.ts", "import { enabled } from './feature';\n");
    f.git(&["add", "."]);
    let original = f.git(&["rev-parse", "HEAD"]);
    let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
    let units: Vec<_> = (0..snapshot.changes.len())
        .map(|id| Unit {
            id,
            changes: vec![id],
        })
        .collect();
    let caller = snapshot
        .changes
        .iter()
        .position(|file| file.path == "src/caller.ts")
        .unwrap();
    let provider = snapshot
        .changes
        .iter()
        .position(|file| file.path == "src/feature.ts")
        .unwrap();
    let make = |id| Draft {
        subject: "Refactor(core): Review dependency".into(),
        description: String::new(),
        units: vec![id],
    };
    let signature = Git::open(f.path()).unwrap().signature().unwrap();
    let error = create_series(
        &snapshot,
        &signature,
        &units,
        &[make(caller), make(provider)],
        None,
        false,
    )
    .unwrap_err();
    assert!(error.contains("imports enabled before"), "{error}");
    assert_eq!(
        create_series(
            &snapshot,
            &signature,
            &units,
            &[make(provider), make(caller)],
            None,
            false,
        )
        .unwrap()
        .len(),
        2
    );
    assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
}

#[test]
fn verifies_every_commit_before_moving_the_branch() {
    let f = Fixture::new(true);
    f.write("src/first.rs", "const VALUE: i32 = 7;\n");
    f.write("src/second.rs", "const FALLBACK: i32 = 9;\n");
    f.write("verify.rs", "include!(\"src/first.rs\");\nfn main() {}\n");
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Baseline"]);
    f.write("src/first.rs", "const VALUE: i32 = invalid_name;\n");
    f.write("src/second.rs", "const invalid_name: i32 = 7;\n");
    f.write("docs/guide.md", "# Guide\n");
    f.git(&["add", "."]);
    let original = f.git(&["rev-parse", "HEAD"]);
    let index = f.git(&["write-tree"]);
    let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
    let units = rules::movable_units(&snapshot.changes);
    let unit = |path: &str| {
        units
            .iter()
            .position(|unit| {
                unit.changes
                    .iter()
                    .any(|id| snapshot.changes[*id].path == path)
            })
            .unwrap()
    };
    let consumer = unit("src/first.rs");
    let provider = unit("src/second.rs");
    let docs = unit("docs/guide.md");
    assert_ne!(consumer, provider);
    let make = |units| Draft {
        subject: "Test(core): Check each staged snapshot".into(),
        description: String::new(),
        units,
    };
    #[cfg(windows)]
    let command = "copy /b src\\second.rs+verify.rs combined.rs >nul && rustc --crate-name verify_sample combined.rs";
    #[cfg(not(windows))]
    let command =
        "cat src/second.rs verify.rs > combined.rs && rustc --crate-name verify_sample combined.rs";
    let risky = [
        make(vec![consumer]),
        make(vec![provider]),
        Draft {
            subject: "Docs(guide): Add setup instructions".into(),
            description: "Describe how to run the project.\n\nCloses #123".into(),
            units: vec![docs],
        },
    ];
    let result = apply_with_verification(
        f.path(),
        &snapshot.fingerprint(false),
        &risky,
        false,
        false,
        Some(command),
    );
    let error = result.expect_err("an intermediate commit should not compile");
    assert!(
        error.contains("Commit 1: Build verification failed") && error.contains("invalid_name"),
        "{error}"
    );
    assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
    assert_eq!(f.git(&["write-tree"]), index);
    assert!(!f.0.path().join("combined.rs").exists());
    let failed = find_buildable_groups(
        f.path(),
        &snapshot.fingerprint(false),
        &risky,
        false,
        "exit 1",
    );
    assert!(
        matches!(failed, Err(message) if message.contains("Regrouping cannot fix the final tree"))
    );
    let repaired = find_buildable_groups(
        f.path(),
        &snapshot.fingerprint(false),
        &risky,
        false,
        command,
    )
    .unwrap();
    assert_eq!(repaired.groups.len(), 2);
    assert_eq!(repaired.groups[0].units, [consumer, provider]);
    assert!(repaired.groups[0]
        .description
        .contains("Keep these changes together"));
    assert!(repaired.groups[0]
        .description
        .lines()
        .all(|line| line.len() <= 72));
    assert_eq!(repaired.groups[1].subject, risky[2].subject);
    assert_eq!(repaired.groups[1].description, risky[2].description);
    assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
    assert_eq!(f.git(&["write-tree"]), index);
    let safe = repaired
        .groups
        .into_iter()
        .map(|group| Draft {
            subject: group.subject,
            description: group.description,
            units: group.units,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        apply_with_verification(
            f.path(),
            &snapshot.fingerprint(false),
            &safe,
            false,
            false,
            Some(command)
        )
        .unwrap()
        .len(),
        2
    );
    assert_eq!(f.git(&["rev-parse", "HEAD~2"]), original);
    assert_eq!(f.git(&["status", "--porcelain"]), "");
    assert!(!f.0.path().join("combined.rs").exists());
}

#[test]
fn detects_local_asset_includes_and_unchanged_callers() {
    for (caller, program, dependency) in [
        (
            "src/main.rs",
            "const DEFAULT: &str = include_str!(\"default.toml\");\n",
            "src/default.toml",
        ),
        (
            "lib/Bootstrap.php",
            "<?php\nrequire_once(__DIR__ . '/Config.php');\n",
            "lib/Config.php",
        ),
        ("objc/View.m", "#import \"View.h\"\n", "objc/View.h"),
    ] {
        let f = Fixture::new(true);
        f.write(caller, "");
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "Add baseline"]);
        f.write(caller, program);
        f.write(dependency, "text\n");
        f.git(&["add", "."]);
        let proposed = plan(f.path(), false).unwrap();
        let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
        let caller_unit = proposed
            .units
            .iter()
            .find(|unit| {
                unit.changes
                    .iter()
                    .any(|id| proposed.changes[*id].path == caller)
            })
            .unwrap();
        if !caller_unit
            .changes
            .iter()
            .any(|id| proposed.changes[*id].path == dependency)
        {
            let mut series = Series::new(&snapshot).unwrap();
            let (id, _) = series.advance(&proposed.units, &[caller_unit.id]).unwrap();
            let mut check = dependencies::Check::new(
                &snapshot.repo,
                snapshot.head,
                snapshot.tree,
                &snapshot.changes,
            )
            .unwrap();
            assert!(
                check
                    .verify(&snapshot.repo.find_tree(id).unwrap())
                    .is_some(),
                "{caller}"
            );
        }
        f.apply(&proposed);
        f.git(&["rm", "--", dependency]);
        assert!(matches!(
            plan(f.path(), false),
            Err(message) if message.contains("missing a local dependency") && message.contains(caller)
        ));
    }
}

#[test]
fn detects_missing_python_and_c_dependencies_between_git_trees() {
    let f = Fixture::new(true);
    f.write("tools/build.py", "from .settings import limit\n");
    f.write("tools/settings.py", "limit = 1\n");
    f.write("native/main.c", "#include \"features.h\"\n");
    f.write("native/features.h", "int select_feature(void);\n");
    f.git(&["add", "."]);
    let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
    let units: Vec<_> = (0..snapshot.changes.len())
        .map(|id| Unit {
            id,
            changes: vec![id],
        })
        .collect();
    let select = |path: &str| {
        vec![snapshot
            .changes
            .iter()
            .position(|file| file.path == path)
            .unwrap()]
    };
    let mut check = dependencies::Check::new(
        &snapshot.repo,
        snapshot.head,
        snapshot.tree,
        &snapshot.changes,
    )
    .unwrap();
    for (consumer, provider) in [
        ("tools/build.py", "tools/settings.py"),
        ("native/main.c", "native/features.h"),
    ] {
        let mut series = Series::new(&snapshot).unwrap();
        let first = series.advance(&units, &select(consumer)).unwrap().0;
        let problem = check
            .verify(&snapshot.repo.find_tree(first).unwrap())
            .unwrap()
            .to_string();
        assert!(problem.contains(consumer), "{problem}");
        let second = series.advance(&units, &select(provider)).unwrap().0;
        assert!(check
            .verify(&snapshot.repo.find_tree(second).unwrap())
            .is_none());
    }
}

#[test]
fn checks_unchanged_callers_when_a_local_module_is_deleted() {
    let f = Fixture::new(true);
    f.write("src/caller.ts", "import { value } from './feature';\n");
    f.write("src/feature.ts", "export const value = 1;\n");
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Baseline"]);
    f.git(&["rm", "--", "src/feature.ts"]);
    let error = plan(f.path(), false)
        .err()
        .expect("an unchanged caller still imports this module");
    assert!(
        error.contains("src/caller.ts") && error.contains("src/feature"),
        "{error}"
    );
}

#[test]
fn combines_the_provider_with_its_caller_without_absorbing_unrelated_steps() {
    let f = Fixture::new(true);
    f.write("src/feature.ts", "export const keep = 1;\n");
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "Baseline"]);
    f.write(
        "src/feature.ts",
        "export const keep = 1;\nexport const enabled = true;\n",
    );
    f.write("src/caller.ts", "import { enabled } from './feature';\n");
    f.write("docs/guide.md", "# Describe a separate workflow\n");
    f.git(&["add", "."]);
    let snapshot = Snapshot::build(&Git::open(f.path()).unwrap(), false).unwrap();
    let units: Vec<_> = (0..snapshot.changes.len())
        .map(|id| Unit {
            id,
            changes: vec![id],
        })
        .collect();
    let index = |path: &str| {
        snapshot
            .changes
            .iter()
            .position(|file| file.path == path)
            .unwrap()
    };
    let (caller, docs, provider) = (
        index("src/caller.ts"),
        index("docs/guide.md"),
        index("src/feature.ts"),
    );
    let mut groups: Vec<_> = [caller, docs, provider]
        .into_iter()
        .map(|id| rules::combined(&snapshot.changes, &units, vec![id]))
        .collect();
    let mut check = dependencies::Check::new(
        &snapshot.repo,
        snapshot.head,
        snapshot.tree,
        &snapshot.changes,
    )
    .unwrap();
    let (notice, _) = arrange_groups(&snapshot, &units, &mut groups, &mut check, false).unwrap();
    assert!(notice.is_some());
    assert_eq!(
        groups
            .iter()
            .map(|group| group.units.clone())
            .collect::<Vec<_>>(),
        [vec![caller, provider], vec![docs]]
    );
}

#[test]
fn pairs_python_relative_imports_and_only_matching_schema_outputs() {
    let f = Fixture::new(true);
    f.write(
        "pylib/api/routes.py",
        "from ..domain.account import Account\n",
    );
    f.write("pylib/domain/account.py", "class Account: pass\n");
    f.write("proto/account.proto", "message Account {}\n");
    f.write("generated/account_pb2.py", "# generated\n");
    f.write("generated/report_pb2.py", "# generated\n");
    f.write(
        "stores/main/migrations/001.sql",
        "CREATE TABLE ledger (id INT);\n",
    );
    f.write(
        "stores/reporting/migrations/001.sql",
        "CREATE TABLE jobs (id INT);\n",
    );
    let plan = plan(f.path(), false).unwrap();
    assert!(suggested_together(
        &plan,
        "pylib/api/routes.py",
        "pylib/domain/account.py"
    ));
    assert!(suggested_together(
        &plan,
        "proto/account.proto",
        "generated/account_pb2.py"
    ));
    assert!(!suggested_together(
        &plan,
        "proto/account.proto",
        "generated/report_pb2.py"
    ));
    assert!(!suggested_together(
        &plan,
        "stores/main/migrations/001.sql",
        "stores/reporting/migrations/001.sql"
    ));
}

#[test]
fn refuses_oversize_without_writing_an_object_or_a_commit() {
    let f = Fixture::new(true);
    fs::File::create(f.0.path().join("large.bin"))
        .unwrap()
        .set_len(GITHUB_MAX_FILE_BYTES + 1)
        .unwrap();
    let before = f.git(&["count-objects"]);
    assert!(plan(f.path(), false).err().unwrap().contains("100 MiB"));
    assert_eq!(f.git(&["count-objects"]), before);
    assert_eq!(f.git(&["rev-list", "--count", "HEAD"]), "1");
}

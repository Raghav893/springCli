//! Snapshot tests for rendered templates, using `insta`.
//!
//! Snapshots live in `tests/snapshots/` and are auto-managed by `cargo-insta`:
//! - Run `cargo insta accept` after intentionally changing a template to update snapshots.
//! - CI fails if a snapshot diff is detected.

use springup_core::plan::{
    ArchitectureKind, BuildTool, ExtraFeature, Language, Packaging, ProjectPlan,
};
use springup_core::template::{TemplateContext, TemplateRenderer};
use std::path::PathBuf;

fn sample_plan(arch: Option<ArchitectureKind>, extras: Vec<ExtraFeature>) -> ProjectPlan {
    ProjectPlan {
        group_id: "com.example".into(),
        artifact_id: "demo".into(),
        name: "demo".into(),
        description: "A demo service".into(),
        package_name: "com.example.demo".into(),
        spring_boot_version: "3.5.0".into(),
        build_tool: BuildTool::Maven,
        language: Language::Java,
        java_version: "21".into(),
        packaging: Packaging::Jar,
        dependencies: vec!["web".into(), "data-jpa".into(), "postgresql".into()],
        architecture: arch,
        extras,
        output_dir: PathBuf::from("./demo"),
        git_init: false,
        initial_commit: false,
    }
}

fn render_template(plan: &ProjectPlan, template_name: &str) -> String {
    let renderer = TemplateRenderer::new();
    let ctx = TemplateContext::from_plan(plan);
    renderer
        .render(template_name, &ctx)
        .expect("template should render")
}

#[test]
fn snapshot_dockerfile_maven() {
    let plan = sample_plan(None, vec![ExtraFeature::Dockerfile]);
    let rendered = render_template(&plan, "docker/Dockerfile.maven.j2");
    insta::assert_snapshot!("dockerfile_maven", rendered);
}

#[test]
fn snapshot_dockerfile_gradle() {
    let mut plan = sample_plan(None, vec![ExtraFeature::Dockerfile]);
    plan.build_tool = BuildTool::GradleGroovy;
    let rendered = render_template(&plan, "docker/Dockerfile.gradle.j2");
    insta::assert_snapshot!("dockerfile_gradle", rendered);
}

#[test]
fn snapshot_docker_compose_with_postgres() {
    let plan = sample_plan(None, vec![ExtraFeature::DockerCompose]);
    let rendered = render_template(&plan, "docker/docker-compose.yml.j2");
    insta::assert_snapshot!("docker_compose_with_postgres", rendered);
}

#[test]
fn snapshot_docker_compose_no_db() {
    let mut plan = sample_plan(None, vec![ExtraFeature::DockerCompose]);
    plan.dependencies = vec!["web".into()];
    let rendered = render_template(&plan, "docker/docker-compose.yml.j2");
    insta::assert_snapshot!("docker_compose_no_db", rendered);
}

#[test]
fn snapshot_ci_maven() {
    let plan = sample_plan(None, vec![ExtraFeature::GithubActionsCi]);
    let rendered = render_template(&plan, "ci/github-actions/ci.maven.yml.j2");
    insta::assert_snapshot!("ci_maven", rendered);
}

#[test]
fn snapshot_ci_gradle() {
    let mut plan = sample_plan(None, vec![ExtraFeature::GithubActionsCi]);
    plan.build_tool = BuildTool::GradleKotlin;
    let rendered = render_template(&plan, "ci/github-actions/ci.gradle.yml.j2");
    insta::assert_snapshot!("ci_gradle", rendered);
}

#[test]
fn snapshot_application_dev_yml() {
    let plan = sample_plan(None, vec![ExtraFeature::ConfigProfiles]);
    let rendered = render_template(&plan, "config-profiles/application-dev.yml.j2");
    insta::assert_snapshot!("application_dev_yml", rendered);
}

#[test]
fn snapshot_application_prod_yml() {
    let plan = sample_plan(None, vec![ExtraFeature::ConfigProfiles]);
    let rendered = render_template(&plan, "config-profiles/application-prod.yml.j2");
    insta::assert_snapshot!("application_prod_yml", rendered);
}

#[test]
fn snapshot_readme() {
    let plan = sample_plan(Some(ArchitectureKind::Layered), vec![ExtraFeature::Readme]);
    let rendered = render_template(&plan, "readme/README.md.j2");
    insta::assert_snapshot!("readme", rendered);
}

#[test]
fn snapshot_layered_global_exception_handler() {
    let plan = sample_plan(Some(ArchitectureKind::Layered), vec![]);
    let rendered = render_template(
        &plan,
        "architectures/layered/exception/GlobalExceptionHandler.j2",
    );
    insta::assert_snapshot!("layered_global_exception_handler", rendered);
}

#[test]
fn snapshot_layered_api_response_dto() {
    let plan = sample_plan(Some(ArchitectureKind::Layered), vec![]);
    let rendered = render_template(&plan, "architectures/layered/dto/ApiResponse.j2");
    insta::assert_snapshot!("layered_api_response_dto", rendered);
}

#[test]
fn snapshot_layered_sample_controller() {
    let mut plan = sample_plan(Some(ArchitectureKind::Layered), vec![]);
    plan.dependencies = vec!["web".into(), "data-jpa".into(), "validation".into()];
    let rendered = render_template(
        &plan,
        "architectures/layered/controller/SampleController.j2",
    );
    insta::assert_snapshot!("layered_sample_controller", rendered);
}

#[test]
fn snapshot_hexagonal_sample_controller() {
    let plan = sample_plan(Some(ArchitectureKind::Hexagonal), vec![]);
    let rendered = render_template(
        &plan,
        "architectures/hexagonal/adapter/in/web/SampleController.j2",
    );
    insta::assert_snapshot!("hexagonal_sample_controller", rendered);
}

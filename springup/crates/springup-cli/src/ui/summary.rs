//! Pretty end-of-run summary: file tree, generated files, next-step commands.

use std::path::Path;

use console::style;
use indicatif::ProgressFinish;

use springup_core::plan::{BuildTool, ProjectPlan};
use springup_core::template::AppliedFiles;

use super::theme;

/// Print a spinner with the given message; returns a `ProgressBar` whose `.finish_with_message`
/// the caller invokes.
pub fn spinner(message: impl Into<String>) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(
        indicatif::ProgressStyle::with_template("{spinner:.dim} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb.set_message(message.into());
    pb.with_finish(ProgressFinish::AndLeave)
}

/// Print the final "what was generated" summary.
pub fn print_summary(plan: &ProjectPlan, applied: &AppliedFiles, output_dir: &Path) {
    println!();
    println!(
        "{}",
        theme::success().apply_to("✓ Project generated successfully.")
    );
    println!();

    // Output location
    println!(
        "{} {}",
        theme::heading().apply_to("Location:"),
        style(output_dir.display()).cyan()
    );

    // Project metadata
    println!("{}", theme::section().apply_to("Project"));
    println!("  name:           {}", plan.name);
    println!("  group:artifact: {}:{}", plan.group_id, plan.artifact_id);
    println!("  package:        {}", plan.package_name);
    println!("  boot version:   {}", plan.spring_boot_version);
    println!(
        "  build / lang:   {} / {} (Java {})",
        plan.build_tool,
        match plan.language {
            springup_core::plan::Language::Java => "java",
            springup_core::plan::Language::Kotlin => "kotlin",
        },
        plan.java_version
    );
    println!("  packaging:      {:?}", plan.packaging);
    println!();

    // Dependencies
    if !plan.dependencies.is_empty() {
        println!("{}", theme::section().apply_to("Dependencies"));
        for d in &plan.dependencies {
            println!("  • {d}");
        }
        println!();
    }

    // Custom layer
    if plan.architecture.is_some() || !plan.extras.is_empty() {
        println!("{}", theme::section().apply_to("Custom layer"));
        if let Some(a) = plan.architecture {
            println!("  architecture: {a:?}");
        }
        if !plan.extras.is_empty() {
            let extras_str: Vec<_> = plan.extras.iter().map(|e| e.slug()).collect();
            println!("  extras:       {}", extras_str.join(", "));
        }
        println!();
    }

    // Files written by the custom layer (relative paths only)
    if !applied.files.is_empty() {
        println!("{}", theme::section().apply_to("Custom-layer files"));
        for f in &applied.files {
            println!("  {}", f.display());
        }
        println!();
    }

    // Next steps
    println!("{}", theme::section().apply_to("Next steps"));
    println!("  cd {}", output_dir.display());
    match plan.build_tool {
        BuildTool::Maven => {
            println!("  ./mvnw spring-boot:run");
        }
        BuildTool::GradleGroovy | BuildTool::GradleKotlin => {
            println!("  ./gradlew bootRun");
        }
    }
    println!();
}

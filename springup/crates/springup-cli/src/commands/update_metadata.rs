//! `springup update-metadata` — refresh the cached Initializr metadata.

use color_eyre::eyre;

use springup_core::initializr::{InitializrClient, InitializrConfig, MetadataCache};

use crate::ui::{messages, summary, theme};

pub fn run() -> color_eyre::Result<i32> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let result: eyre::Result<()> = rt.block_on(async move {
        let cache = MetadataCache::new()?;
        let cfg = InitializrConfig {
            refresh: true,
            ..Default::default()
        };
        let client = InitializrClient::new(cfg, cache)?;
        let pb = summary::spinner(messages::FETCHING_METADATA);
        let m = client.fetch_metadata().await?;
        pb.finish_with_message("Metadata refreshed.");
        println!(
            "{} latest Spring Boot version: {}",
            theme::success().apply_to("✓"),
            m.latest_stable_boot_version().unwrap_or("(unknown)")
        );
        let dep_count = m.all_dependency_ids().len();
        println!("  dependencies known: {dep_count}");
        Ok(())
    });
    result.map(|_| 0)
}

use crate::cli::ImportArgs;
use anyhow::{Context, Result};
use slate_core::model::MediaReference;
use slate_core::project::ProjectBundle;
use slate_media::{MediaProbe, ProxyGenerator, WaveformGenerator};

pub fn execute(args: ImportArgs, json: bool) -> Result<()> {
    slate_media::init().context("Failed to initialize FFmpeg subsystem")?;

    let mut project = ProjectBundle::load_from_bundle(&args.project)
        .with_context(|| format!("Failed to open project bundle at {:?}", args.project))?;

    let meta = MediaProbe::probe(&args.file)
        .with_context(|| format!("Failed to probe media file {:?}", args.file))?;

    let media_id = slate_core::MediaId::new();
    let file_stem = args
        .file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("media");

    let mut proxy_path = None;
    if args.proxy {
        let out_proxy = args
            .project
            .join(ProjectBundle::PROXIES_DIR)
            .join(format!("{}_{}.mp4", file_stem, media_id));
        ProxyGenerator::generate_proxy(&args.file, &out_proxy, None)
            .with_context(|| format!("Failed to generate proxy at {:?}", out_proxy))?;
        proxy_path = Some(out_proxy);
    }

    let mut waveform_path = None;
    if args.waveform && meta.audio_codec.is_some() {
        let out_wf = args
            .project
            .join(ProjectBundle::WAVEFORMS_DIR)
            .join(format!("{}_{}.slwf", file_stem, media_id));
        let buckets = WaveformGenerator::generate_waveform(&args.file, meta.duration, 50)
            .with_context(|| format!("Failed to generate waveform for {:?}", args.file))?;
        WaveformGenerator::save_waveform_file(&buckets, &out_wf)
            .with_context(|| format!("Failed to save waveform to {:?}", out_wf))?;
        waveform_path = Some(out_wf);
    }

    let media_ref = MediaReference {
        id: media_id,
        original_path: args.file.clone(),
        relative_path: args.file.clone(),
        metadata: meta,
        proxy_path,
        waveform_path,
    };

    project.media_pool.push(media_ref.clone());

    ProjectBundle::save_to_bundle(&args.project, &project)
        .with_context(|| format!("Failed to save project bundle at {:?}", args.project))?;

    if json {
        let out = serde_json::json!({
            "status": "success",
            "media_id": media_id,
            "media_ref": media_ref
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!(
            "✓ Imported '{}' into project (ID: {})",
            args.file.display(),
            media_id
        );
        if args.proxy {
            println!("  Proxy generated");
        }
        if args.waveform {
            println!("  Waveform generated");
        }
    }

    Ok(())
}

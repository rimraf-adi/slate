use anyhow::Result;
use slate_render::context::GpuContext;

pub fn execute(json: bool) -> Result<()> {
    let slate_version = env!("CARGO_PKG_VERSION");
    let target_os = std::env::consts::OS;
    let target_arch = std::env::consts::ARCH;

    let gpu_info = match GpuContext::new_headless() {
        Ok(ctx) => {
            let info = ctx.adapter.get_info();
            Some(serde_json::json!({
                "name": info.name,
                "backend": format!("{:?}", info.backend),
                "device_type": format!("{:?}", info.device_type),
                "driver": info.driver,
            }))
        }
        Err(e) => Some(serde_json::json!({
            "error": format!("{}", e)
        })),
    };

    if json {
        let out = serde_json::json!({
            "version": slate_version,
            "os": target_os,
            "arch": target_arch,
            "gpu": gpu_info
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Slate Video Engine v{}", slate_version);
        println!("Platform: {} ({})", target_os, target_arch);
        if let Some(gpu) = gpu_info {
            if let Some(err) = gpu.get("error") {
                println!("GPU: Not available ({})", err);
            } else {
                println!(
                    "GPU: {} [{}] ({})",
                    gpu["name"].as_str().unwrap_or(""),
                    gpu["backend"].as_str().unwrap_or(""),
                    gpu["device_type"].as_str().unwrap_or("")
                );
            }
        }
    }

    Ok(())
}

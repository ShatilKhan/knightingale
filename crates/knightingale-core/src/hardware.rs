//! Hardware detection for spec-aware model recommendation.

use serde::Serialize;
use sysinfo::System;

use crate::model::Model;

#[derive(Debug, Clone, Serialize)]
pub struct Hardware {
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub ram_total_mb: u64,
    pub gpu: Option<GpuInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub vendor: String,
    pub name: String,
    pub vram_mb: u64,
}

pub fn detect() -> Hardware {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();
    let cpus = sys.cpus();
    let cpu_brand = cpus
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "unknown".into());
    let ram_total_mb = sys.total_memory() / 1024 / 1024;
    let gpu = detect_gpu();
    Hardware {
        cpu_brand,
        cpu_cores: cpus.len(),
        ram_total_mb,
        gpu,
    }
}

fn detect_gpu() -> Option<GpuInfo> {
    // NVIDIA first via nvidia-smi if available; cheap and reliable.
    if let Some(g) = detect_nvidia() {
        return Some(g);
    }
    // Linux: parse /sys for AMD VRAM.
    #[cfg(target_os = "linux")]
    if let Some(g) = detect_amd_linux() {
        return Some(g);
    }
    None
}

fn detect_nvidia() -> Option<GpuInfo> {
    let out = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if line.is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() < 2 {
        return None;
    }
    let name = parts[0].to_string();
    let vram_mb: u64 = parts[1].parse().ok()?;
    Some(GpuInfo {
        vendor: "NVIDIA".into(),
        name,
        vram_mb,
    })
}

#[cfg(target_os = "linux")]
fn detect_amd_linux() -> Option<GpuInfo> {
    use std::fs;
    for entry in fs::read_dir("/sys/class/drm").ok()? {
        let entry = entry.ok()?;
        let p = entry.path().join("device/mem_info_vram_total");
        if let Ok(s) = fs::read_to_string(&p) {
            let bytes: u64 = s.trim().parse().ok()?;
            let vram_mb = bytes / 1024 / 1024;
            if vram_mb > 0 {
                return Some(GpuInfo {
                    vendor: "AMD".into(),
                    name: "AMD GPU".into(),
                    vram_mb,
                });
            }
        }
    }
    None
}

/// Pick the highest-accuracy model that fits comfortably in the detected
/// hardware. Headroom rule: model VRAM ≤ 50% of detected VRAM (GPU mode), or
/// model RAM ≤ 25% of detected RAM (CPU mode).
pub fn recommend<'a>(hw: &Hardware, catalog: &'a [Model], english_only: bool) -> Option<&'a Model> {
    let candidates: Vec<&Model> = catalog
        .iter()
        .filter(|m| {
            if english_only && m.language == "multi" {
                return false;
            }
            if let Some(g) = &hw.gpu {
                m.vram_mb as u64 * 2 <= g.vram_mb
            } else {
                (m.size_mb as u64) * 4 <= hw.ram_total_mb
            }
        })
        .collect();
    // Prefer larger (more accurate) models among those that fit.
    candidates.into_iter().max_by_key(|m| m.size_mb)
}

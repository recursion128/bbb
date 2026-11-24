//! Pixel diff for client replay screenshots. Reads a JSON array of jobs
//! `{"left", "right", "diff", "masks": [[x0, y0, x1, y1], ...]}` on stdin,
//! prints one JSON line per job and writes a diff image (changed pixels red
//! over a dimmed copy of `left`) when anything outside the masks changed.

use std::io::Read as _;

use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use serde::Deserialize;

const THRESHOLDS: [u8; 4] = [0, 2, 8, 32];

#[derive(Deserialize)]
struct Job {
    left: String,
    right: String,
    diff: String,
    #[serde(default)]
    masks: Vec<[u32; 4]>,
}

fn main() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let jobs: Vec<Job> = serde_json::from_str(&input).context("parse jobs")?;
    for job in jobs {
        let left = open(&job.left)?;
        let right = open(&job.right)?;
        if left.dimensions() != right.dimensions() {
            println!(
                "{}",
                serde_json::json!({
                    "left": job.left, "size_mismatch": [left.dimensions(), right.dimensions()],
                })
            );
            continue;
        }
        let masked = |x: u32, y: u32| {
            job.masks
                .iter()
                .any(|[x0, y0, x1, y1]| (*x0..*x1).contains(&x) && (*y0..*y1).contains(&y))
        };
        let mut over = [0u64; THRESHOLDS.len()];
        let mut max_delta = 0u8;
        let mut diff = RgbaImage::new(left.width(), left.height());
        for (x, y, out) in diff.enumerate_pixels_mut() {
            let (a, b) = (left.get_pixel(x, y), right.get_pixel(x, y));
            let delta = if masked(x, y) {
                0
            } else {
                (0..4).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0)
            };
            max_delta = max_delta.max(delta);
            for (count, threshold) in over.iter_mut().zip(THRESHOLDS) {
                *count += u64::from(delta > threshold);
            }
            *out = if delta > 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([a[0] / 3, a[1] / 3, a[2] / 3, 255])
            };
        }
        if over[0] > 0 {
            diff.save(&job.diff)
                .with_context(|| format!("write {}", job.diff))?;
        }
        let over: serde_json::Map<String, serde_json::Value> = THRESHOLDS
            .iter()
            .zip(over)
            .map(|(threshold, count)| (threshold.to_string(), count.into()))
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "left": job.left,
                "pixels": u64::from(left.width()) * u64::from(left.height()),
                "max_delta": max_delta,
                "over": over,
            })
        );
    }
    Ok(())
}

fn open(path: &str) -> Result<RgbaImage> {
    Ok(image::open(path)
        .with_context(|| format!("read {path}"))?
        .to_rgba8())
}

//! Reads an image and reduces it to weighted colour clusters in OKLab.
//!
//! Quantising in OKLab rather than RGB keeps clusters perceptually even. The
//! whole process is deterministic: the same image always yields the same theme.

use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageReader};

use crate::color::{Color, Oklab, Oklch};

const SAMPLE_SIZE: u32 = 220;
const CLUSTERS: usize = 16;
const ITERATIONS: usize = 24;
/// Clusters closer than this are the same colour to the eye.
const MERGE_DELTA_E: f64 = 0.025;

#[derive(Clone, Copy, Debug)]
pub struct Cluster {
    pub lab: Oklab,
    /// Fraction of the image covered, summing to 1 across clusters.
    pub weight: f64,
}

impl Cluster {
    pub fn lch(&self) -> Oklch {
        self.lab.to_lch()
    }
}

pub fn load(path: &Path) -> Result<DynamicImage> {
    ImageReader::open(path)
        .with_context(|| format!("no se pudo abrir {}", path.display()))?
        .with_guessed_format()?
        .decode()
        .with_context(|| format!("no se pudo decodificar {}", path.display()))
}

/// Small JPEG used to embed the source image in the HTML preview.
pub fn thumbnail_jpeg(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    img.thumbnail(720, 720)
        .to_rgb8()
        .write_with_encoder(JpegEncoder::new_with_quality(&mut out, 82))?;
    Ok(out.into_inner())
}

pub fn clusters(img: &DynamicImage) -> Vec<Cluster> {
    let points = histogram(img);
    if points.is_empty() {
        return Vec::new();
    }
    merge(kmeans(&points))
}

struct Point {
    lab: Oklab,
    weight: f64,
}

/// Collapses pixels into a 32³ histogram, averaging OKLab inside each bin, so
/// k-means cost no longer depends on the image size.
///
/// Samples original pixels on a grid rather than resizing: resampling blends
/// neighbours, inventing colours the image lacks and leaking the hidden RGB of
/// fully transparent areas into the result.
fn histogram(img: &DynamicImage) -> Vec<Point> {
    let rgba = img.to_rgba8();
    let stride = (rgba.width().max(rgba.height()) / SAMPLE_SIZE).max(1);
    let mut bins = vec![(0.0, 0.0, 0.0, 0.0); 1 << 15];
    for (_, _, p) in rgba
        .enumerate_pixels()
        .filter(|(x, y, _)| x % stride == 0 && y % stride == 0)
    {
        let [r, g, b, a] = p.0;
        if a < 128 {
            continue;
        }
        let i = ((r as usize >> 3) << 10) | ((g as usize >> 3) << 5) | (b as usize >> 3);
        let lab = Color::new(r, g, b).to_lab();
        let bin = &mut bins[i];
        bin.0 += lab.l;
        bin.1 += lab.a;
        bin.2 += lab.b;
        bin.3 += 1.0;
    }
    bins.into_iter()
        .filter(|b| b.3 > 0.0)
        .map(|(l, a, b, n)| Point {
            lab: Oklab {
                l: l / n,
                a: a / n,
                b: b / n,
            },
            weight: n,
        })
        .collect()
}

/// Weighted k-means with deterministic seeding: each new centre maximises
/// weight × squared distance to the existing centres. This picks up small but
/// distinct accents that uniform sampling would miss.
fn kmeans(points: &[Point]) -> Vec<Cluster> {
    let heaviest = points
        .iter()
        .max_by(|a, b| a.weight.total_cmp(&b.weight))
        .expect("points is non-empty");
    let mut centers = vec![heaviest.lab];
    while centers.len() < CLUSTERS.min(points.len()) {
        let next = points
            .iter()
            .max_by(|a, b| seed_score(a, &centers).total_cmp(&seed_score(b, &centers)))
            .expect("points is non-empty");
        centers.push(next.lab);
    }

    let total: f64 = points.iter().map(|p| p.weight).sum();
    let mut weights = vec![0.0; centers.len()];
    for _ in 0..ITERATIONS {
        let mut sums = vec![(0.0, 0.0, 0.0, 0.0); centers.len()];
        for p in points {
            let s = &mut sums[nearest(p.lab, &centers)];
            s.0 += p.lab.l * p.weight;
            s.1 += p.lab.a * p.weight;
            s.2 += p.lab.b * p.weight;
            s.3 += p.weight;
        }
        for (i, (l, a, b, w)) in sums.into_iter().enumerate() {
            weights[i] = w;
            if w > 0.0 {
                centers[i] = Oklab {
                    l: l / w,
                    a: a / w,
                    b: b / w,
                };
            }
        }
    }

    centers
        .into_iter()
        .zip(weights)
        .filter(|(_, w)| *w > 0.0)
        .map(|(lab, w)| Cluster {
            lab,
            weight: w / total,
        })
        .collect()
}

fn seed_score(p: &Point, centers: &[Oklab]) -> f64 {
    let d = centers
        .iter()
        .map(|c| p.lab.delta_e(*c))
        .fold(f64::INFINITY, f64::min);
    p.weight * d * d
}

fn nearest(lab: Oklab, centers: &[Oklab]) -> usize {
    let mut best = (0, f64::INFINITY);
    for (i, c) in centers.iter().enumerate() {
        let d = lab.delta_e(*c);
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0
}

/// Repeatedly fuses the closest pair until no two clusters are distinguishable.
fn merge(mut cs: Vec<Cluster>) -> Vec<Cluster> {
    loop {
        let mut closest: Option<(usize, usize, f64)> = None;
        for i in 0..cs.len() {
            for j in i + 1..cs.len() {
                let d = cs[i].lab.delta_e(cs[j].lab);
                if d < MERGE_DELTA_E && closest.is_none_or(|c| d < c.2) {
                    closest = Some((i, j, d));
                }
            }
        }
        let Some((i, j, _)) = closest else { break };
        let (a, b) = (cs[i], cs.swap_remove(j));
        let w = a.weight + b.weight;
        cs[i] = Cluster {
            lab: Oklab {
                l: (a.lab.l * a.weight + b.lab.l * b.weight) / w,
                a: (a.lab.a * a.weight + b.lab.a * b.weight) / w,
                b: (a.lab.b * a.weight + b.lab.b * b.weight) / w,
            },
            weight: w,
        };
    }
    cs.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    cs
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn two_colour_image_gives_two_weighted_clusters() {
        let img = RgbaImage::from_fn(100, 100, |x, _| {
            if x < 75 {
                Rgba([20, 40, 90, 255])
            } else {
                Rgba([220, 120, 40, 255])
            }
        });
        let cs = clusters(&DynamicImage::ImageRgba8(img));
        assert_eq!(cs.len(), 2);
        assert!((cs[0].weight - 0.75).abs() < 0.03);
        assert!((cs[0].weight + cs[1].weight - 1.0).abs() < 1e-9);
    }

    #[test]
    fn transparent_pixels_are_ignored() {
        let img = RgbaImage::from_fn(50, 50, |x, _| {
            if x < 25 {
                Rgba([255, 0, 0, 0])
            } else {
                Rgba([10, 80, 120, 255])
            }
        });
        let cs = clusters(&DynamicImage::ImageRgba8(img));
        assert_eq!(cs.len(), 1);
        assert!(cs[0].lch().h > 200.0 && cs[0].lch().h < 260.0);
    }

    #[test]
    fn extraction_is_deterministic() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_fn(64, 64, |x, y| {
            Rgba([(x * 4) as u8, (y * 4) as u8, 128, 255])
        }));
        let (a, b) = (clusters(&img), clusters(&img));
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.lab, y.lab);
        }
    }
}

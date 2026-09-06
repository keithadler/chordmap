//! Key from Krumhansl-Kessler profile correlation.

const MAJOR: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];
const MINOR: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyCandidate {
    pub tonic: usize,
    pub minor: bool,
    pub correlation: f32,
}

fn pearson(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len() as f32;
    let ma = a.iter().sum::<f32>() / n;
    let mb = b.iter().sum::<f32>() / n;
    let mut num = 0.0;
    let mut da = 0.0;
    let mut db = 0.0;
    for i in 0..a.len() {
        num += (a[i] - ma) * (b[i] - mb);
        da += (a[i] - ma).powi(2);
        db += (b[i] - mb).powi(2);
    }
    num / (da * db).sqrt().max(1e-9)
}

/// All 24 keys, best first.
pub fn rank(global_chroma: &[f32; 12]) -> Vec<KeyCandidate> {
    let mut out = Vec::with_capacity(24);
    for minor in [false, true] {
        let profile = if minor { MINOR } else { MAJOR };
        for tonic in 0..12 {
            let rotated: Vec<f32> = (0..12).map(|i| profile[(i + 12 - tonic) % 12]).collect();
            out.push(KeyCandidate {
                tonic,
                minor,
                correlation: pearson(global_chroma, &rotated),
            });
        }
    }
    out.sort_by(|a, b| b.correlation.partial_cmp(&a.correlation).unwrap());
    out
}
